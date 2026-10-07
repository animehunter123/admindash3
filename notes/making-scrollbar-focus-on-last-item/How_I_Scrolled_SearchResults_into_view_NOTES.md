The important pieces are:

scroll_selected_into_view() helper.
Every search result gets id: "search-result-{index}".
Arrow Up/Down and Tab/Shift+Tab call the helper.
scrollIntoView() is delayed one browser tick so Dioxus has time to render the newly selected result.
Kept your 100dvh + scrollbar-gutter: stable changes.
Removed the giant old commented-out onkeydown and old search-results implementation.

One small improvement I made: scrollIntoView uses { block: "nearest" } rather than forcing the selected item to the center/top. That means the dropdown only scrolls when necessary, which feels much nicer.








so AI/TAILWIND helped me get a fn() for this:













Absolutely. I cleaned up the giant comment blocks but kept the **useful "why is this here?" comments** so the file is still study-friendly.

I also made the scrolling fix **without adding a dependency**. The important pieces are:

1. `scroll_selected_into_view()` helper.
2. Every search result gets `id: "search-result-{index}"`.
3. Arrow Up/Down and Tab/Shift+Tab call the helper.
4. `scrollIntoView()` is delayed one browser tick so Dioxus has time to render the newly selected result.
5. Kept your `100dvh` + `scrollbar-gutter: stable` changes.
6. Removed the giant old commented-out `onkeydown` and old search-results implementation.

One small improvement I made: `scrollIntoView` uses `{ block: "nearest" }` rather than forcing the selected item to the center/top. That means the dropdown only scrolls **when necessary**, which feels much nicer.

Here's the complete `src/navbar.rs`:

```rust
use crate::data_navbar::NavItem;
use crate::IFRAME_URL;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::window;

// -----------------------------------------------------------------------------
// NAVBAR
// -----------------------------------------------------------------------------

#[component]
pub fn Navbar(items: Vec<NavItem>) -> Element {
    let mut search = use_signal(String::new);
    let mut search_open = use_signal(|| false);

    // Tracks which search result is currently highlighted.
    let mut selected_index = use_signal(|| 0usize);

    // Controls the mobile hamburger menu.
    let mut mobile_menu_open = use_signal(|| false);

    // Quick Search is only active when the main page is displayed.
    let is_main_page = {
        let iframe_url = IFRAME_URL.read();

        iframe_url
            .as_deref()
            .map(|url| url.is_empty())
            .unwrap_or(true)
    };

    // -------------------------------------------------------------------------
    // HYDRATION-SAFE NAVBAR CLICK RECOVERY
    // -------------------------------------------------------------------------
    //
    // Before WASM hydrates, normal href links can still be clicked.
    // We temporarily store those clicks in the URL hash and recover them here.
    //
    // Format:
    // #__pending_nav?url=<encoded URL>
    // -------------------------------------------------------------------------

    use_effect(move || {
        let Some(win) = window() else {
            return;
        };

        let location = win.location();

        let Ok(hash) = location.hash() else {
            return;
        };

        if let Some(query_string) = hash.strip_prefix("#__pending_nav?") {
            web_sys::console::log_1(
                &format!(
                    "WASM mounted - found pending navbar click: {:?}",
                    query_string
                )
                .into(),
            );

            if let Some(encoded_url) = query_string.strip_prefix("url=") {
                #[cfg(target_arch = "wasm32")]
                {
                    let decoded = js_sys::decode_uri_component(encoded_url)
                        .ok()
                        .and_then(|value| value.as_string());

                    if let Some(url) = decoded {
                        web_sys::console::log_1(
                            &format!("Executing queued navbar URL: {:?}", url).into(),
                        );

                        *IFRAME_URL.write() = Some(url);
                    }
                }
            }

            // Remove the temporary hash without causing a page reload.
            if let Ok(history) = win.history() {
                let _ = history.replace_state_with_url(
                    &wasm_bindgen::JsValue::NULL,
                    "",
                    Some(&location.pathname().unwrap_or_else(|_| "/".to_string())),
                );
            }
        }
    });

    // -------------------------------------------------------------------------
    // GLOBAL SEARCH SHORTCUT
    // -------------------------------------------------------------------------
    //
    // "/" or Ctrl+K focuses Quick Search.
    // We ignore other inputs so normal typing is not interrupted.
    // -------------------------------------------------------------------------

    use_effect(move || {
        let Some(win) = window() else {
            return;
        };

        let Some(listener_document) = win.document() else {
            return;
        };

        let callback =
            wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if !is_main_page {
                    return;
                }

                let Some(win) = window() else {
                    return;
                };

                let Some(document) = win.document() else {
                    return;
                };

                // Do not hijack keyboard shortcuts while another editable
                // element already has focus.
                if let Some(active) = document.active_element() {
                    let tag = active.tag_name().to_lowercase();

                    if tag == "input"
                        || tag == "textarea"
                        || tag == "select"
                        || active.has_attribute("contenteditable")
                    {
                        return;
                    }
                }

                let open_search = event.key() == "/"
                    || (event.key().eq_ignore_ascii_case("k") && event.ctrl_key());

                if !open_search {
                    return;
                }

                event.prevent_default();

                let Some(element) = document.get_element_by_id("autocomplete_field") else {
                    return;
                };

                if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
                    let _ = input.focus();
                    input.select();
                }

                search_open.set(true);
                selected_index.set(0);
            })
                as Box<dyn FnMut(web_sys::KeyboardEvent)>);

        let callback_ref = callback.as_ref().unchecked_ref();

        let _ =
            listener_document.add_event_listener_with_callback("keydown", callback_ref);

        // JavaScript needs to keep this Closure alive.
        callback.forget();
    });

    // -------------------------------------------------------------------------
    // BUILD SEARCH RESULTS
    // -------------------------------------------------------------------------

    let query = search().to_lowercase();

    let matches: Vec<(String, String)> = {
        let buttons = crate::BUTTONS.read();

        buttons
            .values()
            .flat_map(|links| links.iter())
            .filter(|(name, _url)| {
                !name.starts_with("DIVIDER")
                    && !query.is_empty()
                    && name.to_lowercase().contains(&query)
            })
            .map(|(name, url)| (name.clone(), url.clone()))
            .collect()
    };

    // The keyboard handler owns its own copy of the current results.
    let matches_for_keyboard = matches.clone();

    // -------------------------------------------------------------------------
    // SEARCH RESULT SCROLLING
    // -------------------------------------------------------------------------
    //
    // selected_index changes immediately, but the DOM needs one render cycle
    // before the newly selected result exists/updates in the browser.
    //
    // We therefore wait one browser tick before calling scrollIntoView().
    //
    // No extra Rust crate is required.
    // -------------------------------------------------------------------------

    let scroll_selected_into_view = move |index: usize| {
        if let Some(win) = window() {
            let callback = wasm_bindgen::closure::Closure::once(Box::new(move || {
                if let Some(win) = window() {
                    if let Some(document) = win.document() {
                        if let Some(element) =
                            document.get_element_by_id(&format!("search-result-{}", index))
                        {
                            // false means use the browser's nearest scrolling
                            // behavior instead of forcing the item to the top.
                            element.scroll_into_view_with_bool(false);
                        }
                    }
                }
            }) as Box<dyn FnOnce()>);

            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                0,
            );

            callback.forget();
        }
    };

    // -------------------------------------------------------------------------
    // RENDER NAVBAR
    // -------------------------------------------------------------------------

    rsx! {
        nav {
            class: "relative bg-gray-900 p-4 flex items-center gap-2",

            // Invisible layer that closes Search when clicking outside it.
            if is_main_page && search_open() && !matches.is_empty() {
                div {
                    class: "fixed inset-0 z-40",

                    onclick: move |_| {
                        search_open.set(false);
                    },
                }
            }

            // -----------------------------------------------------------------
            // MOBILE HAMBURGER
            // -----------------------------------------------------------------
            //
            // Normal navbar links disappear at <= 430px.
            // The hamburger opens the same `items` collection.
            // -----------------------------------------------------------------

            button {
                class: "hidden max-[430px]:flex items-center justify-center
                        w-10 h-10 rounded-md text-white
                        hover:bg-gray-800 focus:outline-none
                        focus:ring-2 focus:ring-blue-500",

                onclick: move |_| {
                    mobile_menu_open.set(!mobile_menu_open());
                },

                // Three simple CSS bars make the hamburger icon.
                span {
                    class: "flex flex-col gap-1.5",

                    span {
                        class: "block w-5 h-0.5 bg-white"
                    }

                    span {
                        class: "block w-5 h-0.5 bg-white"
                    }

                    span {
                        class: "block w-5 h-0.5 bg-white"
                    }
                }
            }

            // -----------------------------------------------------------------
            // DESKTOP NAVIGATION
            // -----------------------------------------------------------------

            ul {
                class: "flex gap-6 items-center relative z-50",

                for (index, item) in items.iter().enumerate() {
                    li {
                        class: if index == 0 {
                            ""
                        } else {
                            "max-[430px]:hidden"
                        },

                        NavEntry {
                            item: item.clone(),
                            is_brand: index == 0
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // MOBILE NAVIGATION DROPDOWN
            // -----------------------------------------------------------------
            //
            // Uses the same `items` as the desktop navbar.
            // The first item is the brand, so it is skipped here.
            // -----------------------------------------------------------------

            if mobile_menu_open() {
                div {
                    class: "absolute left-2 top-16 z-50
                            max-[430px]:block min-[431px]:hidden
                            w-56 bg-gray-900 border border-gray-700
                            rounded-md shadow-xl p-2",

                    for (index, item) in items.iter().enumerate() {
                        if index != 0 {
                            div {
                                class: "px-2 py-2 hover:bg-gray-800 rounded-md",

                                onclick: move |_| {
                                    mobile_menu_open.set(false);
                                },

                                NavEntry {
                                    item: item.clone(),
                                    is_brand: false
                                }
                            }
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // QUICK SEARCH
            // -----------------------------------------------------------------
            //
            // Desktop: 256px wide.
            // Mobile: 160px wide.
            // -----------------------------------------------------------------

            div {
                class: "ml-auto relative w-64 max-[430px]:w-40 z-50",

                input {
                    id: "autocomplete_field",

                    autofocus: true,

                    value: "{search}",

                    onmounted: move |event| {
                        let element = event.data().as_web_event();

                        if let Some(input) =
                            element.dyn_ref::<web_sys::HtmlInputElement>()
                        {
                            let existing_value = input.value();

                            // Restore the search text when returning to a tab.
                            if !existing_value.is_empty() {
                                spawn_local(async move {
                                    search.set(existing_value);
                                    search_open.set(true);
                                    selected_index.set(0);
                                });
                            }
                        }
                    },

                    placeholder: "Quick Search... (Press '/')",

                    class: if is_main_page {
                        "w-64 max-[430px]:w-40
                         px-3 py-2 rounded-md
                         bg-gray-800
                         text-white placeholder-gray-400
                         border border-gray-700
                         focus:outline-none
                         focus:ring-2 focus:ring-blue-500"
                    } else {
                        "w-64 max-[430px]:w-40
                         px-3 py-2 rounded-md
                         bg-gray-800
                         text-white placeholder-gray-400
                         border border-gray-700
                         focus:outline-none
                         focus:ring-2 focus:ring-blue-500
                         opacity-0 pointer-events-none"
                    },

                    onfocus: move |_| {
                        search_open.set(true);
                        selected_index.set(0);
                    },

                    oninput: move |event| {
                        search.set(event.value());
                        search_open.set(true);
                        selected_index.set(0);
                    },

                    // ---------------------------------------------------------
                    // SEARCH KEYBOARD NAVIGATION
                    // ---------------------------------------------------------
                    //
                    // Arrow keys and Tab wrap around the results.
                    // The selected result is automatically scrolled into view.
                    // ---------------------------------------------------------

                    onkeydown: move |event| {
                        let is_shift = event.modifiers().shift();

                        match event.key() {
                            Key::Escape => {
                                search_open.set(false);
                                search.set(String::new());
                                selected_index.set(0);

                                event.prevent_default();
                            }

                            Key::ArrowDown => {
                                if !matches_for_keyboard.is_empty() {
                                    let next_index =
                                        (selected_index() + 1)
                                            % matches_for_keyboard.len();

                                    selected_index.set(next_index);
                                    scroll_selected_into_view(next_index);

                                    event.prevent_default();
                                }
                            }

                            Key::Tab if !is_shift => {
                                if !matches_for_keyboard.is_empty() {
                                    let next_index =
                                        (selected_index() + 1)
                                            % matches_for_keyboard.len();

                                    selected_index.set(next_index);
                                    scroll_selected_into_view(next_index);

                                    event.prevent_default();
                                }
                            }

                            Key::ArrowUp => {
                                if !matches_for_keyboard.is_empty() {
                                    let current = selected_index();

                                    let previous_index = if current == 0 {
                                        matches_for_keyboard.len() - 1
                                    } else {
                                        current - 1
                                    };

                                    selected_index.set(previous_index);
                                    scroll_selected_into_view(previous_index);

                                    event.prevent_default();
                                }
                            }

                            Key::Tab if is_shift => {
                                if !matches_for_keyboard.is_empty() {
                                    let current = selected_index();

                                    let previous_index = if current == 0 {
                                        matches_for_keyboard.len() - 1
                                    } else {
                                        current - 1
                                    };

                                    selected_index.set(previous_index);
                                    scroll_selected_into_view(previous_index);

                                    event.prevent_default();
                                }
                            }

                            // Enter opens the selected result in a new tab.
                            Key::Enter => {
                                if let Some((_name, url)) =
                                    matches_for_keyboard
                                        .get(selected_index())
                                        .cloned()
                                {
                                    if let Some(win) = window() {
                                        let _ =
                                            win.open_with_url_and_target(
                                                &url,
                                                "_blank",
                                            );
                                    }

                                    // Keep the search text and dropdown open.
                                    // This lets the user return to this tab later.

                                    if let Some(win) = window() {
                                        if let Some(document) = win.document() {
                                            if let Some(element) =
                                                document
                                                    .get_element_by_id(
                                                        "autocomplete_field",
                                                    )
                                            {
                                                if let Some(input) =
                                                    element.dyn_ref::<
                                                        web_sys::HtmlInputElement,
                                                    >()
                                                {
                                                    let _ = input.focus();
                                                    input.select();
                                                }
                                            }
                                        }
                                    }

                                    event.prevent_default();
                                }
                            }

                            _ => {}
                        }
                    },
                }

                // -------------------------------------------------------------
                // SEARCH RESULTS
                // -------------------------------------------------------------
                //
                // `max-h` uses the viewport height instead of a fixed pixel
                // height. `scrollbar-gutter` prevents navbar layout shifting.
                // -------------------------------------------------------------

                if is_main_page && search_open() && !matches.is_empty() {
                    div {
                        class: "absolute right-0 mt-2 w-96
                                max-[430px]:fixed
                                max-[430px]:left-2
                                max-[430px]:right-2
                                max-[430px]:top-16
                                max-[430px]:mt-0
                                max-[430px]:w-auto
                                bg-white rounded-md shadow-xl
                                border border-gray-200
                                max-h-[calc(100dvh-100px)]
                                overflow-y-auto
                                [scrollbar-gutter:stable]
                                z-50",

                        for (index, (name, url)) in matches.iter().enumerate() {
                            {
                                let name = name.clone();
                                let url = url.clone();
                                let is_selected = index == selected_index();

                                rsx! {
                                    button {
                                        // Give every result a predictable DOM id.
                                        // Keyboard navigation uses this to find the
                                        // selected result and scroll it into view.
                                        id: "search-result-{index}",

                                        class: if is_selected {
                                            "block w-full text-left px-4 py-2
                                             bg-blue-100 text-blue-700"
                                        } else {
                                            "block w-full text-left px-4 py-2
                                             text-gray-800
                                             hover:bg-blue-100
                                             hover:text-blue-700"
                                        },

                                        onclick: move |_| {
                                            if let Some(win) = window() {
                                                let _ =
                                                    win.open_with_url_and_target(
                                                        &url,
                                                        "_blank",
                                                    );
                                            }

                                            search.set(String::new());
                                            search_open.set(false);
                                            selected_index.set(0);
                                        },

                                        "{name}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// NORMAL NAVBAR ENTRY
// -----------------------------------------------------------------------------

#[component]
fn NavEntry(item: NavItem, is_brand: bool) -> Element {
    let mut open = use_signal(|| false);

    // -------------------------------------------------------------------------
    // HYDRATION-SAFE URL
    // -------------------------------------------------------------------------
    //
    // Before WASM loads, the href uses a temporary hash.
    // After hydration, the onclick handler takes over normally.
    // -------------------------------------------------------------------------

    let pending_href = item.url.clone().map(|url| {
        #[cfg(target_arch = "wasm32")]
        {
            let encoded = js_sys::encode_uri_component(&url);
            format!("#__pending_nav?url={}", encoded)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            // SSR cannot call browser APIs, so use a normal string here.
            format!("#__pending_nav?url={}", url)
        }
    });

    rsx! {
        div {
            class: "relative",

            // -----------------------------------------------------------------
            // BRAND
            // -----------------------------------------------------------------

            if is_brand {
                a {
                    href: item.url.as_deref().unwrap_or("/"),

                    class: "relative text-white hover:text-blue-400",

                    onclick: move |_| {
                        *IFRAME_URL.write() = Some(String::new());
                    },

                    img {
                        src: asset!("/assets/favicon.ico"),
                        class: "w-6 h-6",
                        alt: "Admindash3",
                    }
                }
            }

            // -----------------------------------------------------------------
            // NORMAL NAVBAR LINK
            // -----------------------------------------------------------------

            else if item.children.is_empty() {
                if let Some(url) = item.url.clone() {
                    a {
                        href: pending_href.unwrap_or_else(|| "#".to_string()),

                        class: "text-white hover:text-blue-400",

                        onclick: move |event| {
                            // WASM is active, so prevent the temporary href.
                            event.prevent_default();

                            web_sys::console::log_1(
                                &format!(
                                    "Navbar clicked through Dioxus WASM handler: {}",
                                    url
                                )
                                .into(),
                            );

                            *IFRAME_URL.write() = Some(url.clone());
                        },

                        "{item.name}"
                    }
                } else {
                    a {
                        href: "#",
                        class: "text-white hover:text-blue-400",
                        "{item.name}"
                    }
                }
            }

            // -----------------------------------------------------------------
            // NAVBAR ITEM WITH CHILDREN
            // -----------------------------------------------------------------

            else {
                button {
                    class: "text-white hover:text-blue-400",

                    onclick: move |_| {
                        open.set(!open());
                    },

                    "{item.name}"

                    span {
                        class: "ml-2",
                        "???"
                    }
                }

                if open() {
                    ul {
                        class: "absolute left-0 mt-2 w-48
                                bg-white rounded shadow-lg p-2 z-50",

                        for child in item.children.clone() {
                            li {
                                class: "p-2 hover:bg-gray-100",

                                a {
                                    href: child.url.as_deref().unwrap_or("#"),

                                    class: "text-gray-800",

                                    "{child.name}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
```

### The important part to study

The new behavior is really just these three pieces working together.

**1. Every result gets an ID:**

```rust
id: "search-result-{index}",
```

**2. When selection changes, we request scrolling:**

```rust
selected_index.set(previous_index);
scroll_selected_into_view(previous_index);
```

**3. The helper finds that DOM element after Dioxus renders it:**

```rust
let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
    callback.as_ref().unchecked_ref(),
    0,
);
```

Then:

```rust
element.scroll_into_view_with_bool(false);
```

The `false` is the interesting bit: it means **"scroll the minimum amount necessary to make this element visible."**

So if you're halfway down the list and press ↓, it doesn't obnoxiously jump the selected item to the top every time. It just nudges the dropdown when the selection reaches the edge.

And when you do your wrap:

```text
0
↓ ArrowUp
79
```

the browser notices that `search-result-79` isn't visible and automatically scrolls the dropdown down to it.

That's the piece your existing wrap-around logic was missing.

