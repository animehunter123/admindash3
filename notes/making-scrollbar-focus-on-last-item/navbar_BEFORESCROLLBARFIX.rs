use crate::IFRAME_URL;
use crate::data_navbar::NavItem;
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
    // let items = navbar_items(); // call this method from navbar.rs!

    let mut search = use_signal(String::new);
    let mut search_open = use_signal(|| false);

    // Index of the currently highlighted search result.
    let mut selected_index = use_signal(|| 0usize);

    // -------------------------------------------------------------------------
    // MOBILE NAVIGATION MENU
    // -------------------------------------------------------------------------
    // At <= 430px the normal navbar links are hidden and replaced with a
    // hamburger button.
    //
    // This signal controls whether the mobile navigation dropdown is open.
    //
    // IMPORTANT:
    // We are NOT making a second hardcoded navbar.
    //
    // The mobile menu below uses the SAME `items` returned by navbar_items().
    // So adding/removing navbar entries automatically affects both versions.
    // -------------------------------------------------------------------------
    let mut mobile_menu_open = use_signal(|| false);

    // -------------------------------------------------------------------------
    // ARE WE ON THE MAIN PAGE?
    // -------------------------------------------------------------------------
    let is_main_page = {
        let iframe_url = IFRAME_URL.read();

        iframe_url
            .as_deref()
            .map(|url| url.is_empty())
            .unwrap_or(true)
    };

    // -------------------------------------------------------------------------
    // DYNAMIC NAVBAR CLICK QUEUE
    // -------------------------------------------------------------------------
    //
    // During SSR -> WASM hydration there can be a short period where:
    //
    //     HTML exists
    //          |
    //          v
    //     user clicks navbar
    //          |
    //          v
    //     WASM hasn't attached onclick yet
    //
    // A normal Dioxus onclick would therefore be lost.
    //
    // The browser DOES understand normal href links before WASM exists.
    //
    // Therefore our navbar links use a temporary URL fragment:
    //
    //     #__pending_nav?url=<encoded URL>
    //
    // Once WASM mounts, this effect checks the fragment and recovers the
    // click.
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

            // -----------------------------------------------------------------
            // We deliberately avoid UrlSearchParams here.
            //
            // This keeps the recovery code simple and avoids introducing
            // another browser API into the SSR rendering path.
            //
            // Our format is:
            //
            //     #__pending_nav?url=<encoded URL>
            // -----------------------------------------------------------------
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

            // -----------------------------------------------------------------
            // Remove the temporary fragment.
            //
            // replace_state() changes the URL without causing another page
            // load.
            // -----------------------------------------------------------------
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
    // Press:
    //
    //     /
    //
    // OR:
    //
    //     Ctrl + K
    //
    // and Quick Search receives focus.
    //
    // This listener is attached to the document rather than the input itself,
    // because the user may currently have focus somewhere else on the page.
    //
    // We deliberately ignore other editable elements so:
    //
    //     /
    //     Ctrl + K
    //
    // don't hijack normal typing inside another input or textarea.
    // -------------------------------------------------------------------------
    use_effect(move || {
        let Some(win) = window() else {
            return;
        };

        // We need the document here so we can register the keyboard listener.
        let Some(listener_document) = win.document() else {
            return;
        };

        let callback =
            wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                // ---------------------------------------------------------
                // Only activate Quick Search on the main page.
                // ---------------------------------------------------------
                if !is_main_page {
                    return;
                }

                // ---------------------------------------------------------
                // Get the current document.
                //
                // We intentionally do this INSIDE the callback instead of
                // moving a Document into the callback.
                //
                // This keeps the Rust ownership much simpler.
                // ---------------------------------------------------------
                let Some(win) = window() else {
                    return;
                };

                let Some(document) = win.document() else {
                    return;
                };

                // ---------------------------------------------------------
                // Don't steal keyboard input from another editable element.
                // ---------------------------------------------------------
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

                // ---------------------------------------------------------
                // Check for:
                //
                //     /
                //
                // OR:
                //
                //     Ctrl + K
                // ---------------------------------------------------------
                let open_search = event.key() == "/"
                    || (event.key().eq_ignore_ascii_case("k") && event.ctrl_key());

                if !open_search {
                    return;
                }

                // Don't let the browser perform its normal action.
                event.prevent_default();

                // ---------------------------------------------------------
                // Find our Quick Search input.
                //
                // This is the input that has:
                //
                //     id: "autocomplete_field"
                // ---------------------------------------------------------
                let Some(element) = document.get_element_by_id("autocomplete_field") else {
                    return;
                };

                // ---------------------------------------------------------
                // Convert the DOM element into an HtmlInputElement.
                // ---------------------------------------------------------
                if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
                    // Give Quick Search keyboard focus.
                    let _ = input.focus();

                    // Select any existing text.
                    //
                    // Example:
                    //
                    //     current search = "printer"
                    //
                    // Press "/" and type "server":
                    //
                    //     new search = "server"
                    //
                    // rather than:
                    //
                    //     "printerserver"
                    input.select();
                }

                // Open the search results.
                search_open.set(true);

                // Start with the first result selected.
                selected_index.set(0);
            })
                as Box<dyn FnMut(web_sys::KeyboardEvent)>);

        let callback_ref = callback.as_ref().unchecked_ref();

        // ---------------------------------------------------------------------
        // Register the keyboard listener on the whole document.
        // ---------------------------------------------------------------------
        let _ = listener_document.add_event_listener_with_callback("keydown", callback_ref);

        // ---------------------------------------------------------------------
        // Keep the Closure alive.
        //
        // JavaScript now has a reference to this callback.
        // ---------------------------------------------------------------------
        callback.forget();
    });

    let query = search().to_lowercase();

    // Build the filtered search-result list once per render.
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

    // The keyboard handler needs its own clone of the current matches.
    let matches_for_keyboard = matches.clone();

    rsx! {
                                nav {
                                    class: "relative bg-gray-900 p-4 flex items-center gap-2",

                                    // -----------------------------------------------------------------
                                    // SEARCH OUTSIDE-CLICK LAYER
                                    // -----------------------------------------------------------------
                                    //
                                    // When the search results are open, this invisible layer covers
                                    // the rest of the page.
                                    //
                                    // Clicking it closes the search.
                                    //
                                    // The actual search UI has z-50, while this layer has z-40.
                                    // Therefore clicks INSIDE the search still reach the search.
                                    // -----------------------------------------------------------------
                                    if is_main_page && search_open() && !matches.is_empty() {
                                        div {
                                            class: "fixed inset-0 z-40",

                                            onclick: move |_| {
                                                search_open.set(false);
                                            },
                                        }
                                    }

                                    // -----------------------------------------------------------------
                                    // MOBILE HAMBURGER BUTTON
                                    // -----------------------------------------------------------------
                                    //
                                    // Tailwind's:
                                    //
                                    //     max-[430px]:flex
                                    //
                                    // means:
                                    //
                                    //     "apply flex when the viewport is 430px or smaller"
                                    //
                                    // The normal `hidden` means it is invisible by default.
                                    //
                                    // Therefore:
                                    //
                                    //     > 430px  -> hidden
                                    //     <= 430px -> flex
                                    //
                                    // This is the responsive breakpoint we're using for this navbar.
                                    // -----------------------------------------------------------------
                                    button {
                                        class: "hidden max-[430px]:flex items-center justify-center
                        w-10 h-10 rounded-md text-white
                        hover:bg-gray-800 focus:outline-none
                        focus:ring-2 focus:ring-blue-500",

                        //                aria-label: "Open navigation menu",

                                        onclick: move |_| {
                                            mobile_menu_open.set(!mobile_menu_open());
                                        },

                                        // -------------------------------------------------------------
                                        // Simple hamburger icon. These are just three little CSS bars.
                                        // No Font Awesome. No Google Fonts. No external icon library.
                                        // -------------------------------------------------------------
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
                                    // LEFT SIDE - NORMAL NAVBAR
                                    // -----------------------------------------------------------------
                                    //
                                    // The first item is the brand and remains visible at every size.
                                    //
                                    // The other navbar entries are hidden at <= 430px.
                                    //
                                    // They are NOT deleted from the DOM logic; they are simply hidden
                                    // with Tailwind and appear in the hamburger menu below instead.
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
                                    // This uses the EXACT SAME `items` collection as the desktop menu.
                                    //
                                    // So if data_navbar.json contains:
                                    //
                                    //     Admindash3
                                    //     Wiki
                                    //     CopyPasta
                                    //
                                    // the mobile menu automatically gets:
                                    //
                                    //     Wiki
                                    //     CopyPasta
                                    //
                                    // The brand is skipped because it is already permanently visible.
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

                                                        // -------------------------------------------------
                                                        // Close the hamburger menu when a menu item is
                                                        // clicked.
                                                        // -------------------------------------------------
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
                                    // RIGHT SIDE - QUICK SEARCH
                                    // -----------------------------------------------------------------
                                    //
                                    // Desktop:
                                    //
                                    //     w-64 = 16rem = 256px
                                    //
                                    // Mobile:
                                    //
                                    //     max-[430px]:w-40 = 10rem = 160px
                                    //
                                    // This gives the hamburger + brand + search room to coexist on
                                    // a small screen.
                                    //
                                    // IMPORTANT:
                                    //
                                    // The search remains the SAME reactive Dioxus input.
                                    //
                                    // We are only changing its CSS width here.
                                    //
                                    // So all of your existing:
                                    //
                                    //     oninput
                                    //     onkeydown
                                    //     /
                                    //     Ctrl+K
                                    //     ArrowUp
                                    //     ArrowDown
                                    //     Enter
                                    //     Escape
                                    //
                                    // behavior continues to work.
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

                                                    // TEMPORARY TEST TO CONSOLE LOG
            //                                        web_sys::console::log_1(
            //                                            &format!(
            //                                                "ONMOUNTED VALUE = {:?}",
            //                                                existing_value
            //                                            )
            //                                            .into(),
            //                                        );

                                                    if !existing_value.is_empty() {
                                                        spawn_local(async move {
                                                            web_sys::console::log_1(
                                                                &format!(
                                                                    "SETTING SEARCH = {:?}",
                                                                    existing_value
                                                                )
                                                                .into(),
                                                            );

                                                            search.set(existing_value);
                                                            search_open.set(true);
                                                            selected_index.set(0);
                                                        });
                                                    }
                                                }
                                            },

                                            placeholder: "Quick Search... (Press '/')",

                                            // ---------------------------------------------------------
                                            // RESPONSIVE SEARCH WIDTH
                                            //
                                            // Desktop:
                                            //     w-64
                                            //
                                            // Mobile:
                                            //     max-[430px]:w-40
                                            //
                                            // The input is still the same reactive signal.
                                            // ---------------------------------------------------------
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

                                                // Always highlight the first result when opening.
                                                selected_index.set(0);
                                            },

                                            oninput: move |event| {
                                                search.set(event.value());
                                                search_open.set(true);

                                                // A new query means the first match is selected.
                                                selected_index.set(0);
                                            },







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
                                        (selected_index() + 1) % matches_for_keyboard.len();

                                    selected_index.set(next_index);
                                    event.prevent_default();
                                }
                            }

                            Key::Tab if !is_shift => {
                                if !matches_for_keyboard.is_empty() {
                                    let next_index =
                                        (selected_index() + 1) % matches_for_keyboard.len();

                                    selected_index.set(next_index);
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
                                    event.prevent_default();
                                }
                            }


                                /*

                            Key::Enter => {
                                if let Some((_name, url)) =
                                    matches_for_keyboard.get(selected_index()).cloned()
                                {
                                    if let Some(win) = window() {
                                        let _ = win.open_with_url_and_target(&url, "_blank");
                                    }

                                    search.set(String::new());
                                    search_open.set(false);
                                    selected_index.set(0);

                                    event.prevent_default();
                                }
                            }
                */

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









                // -------------------------------------------------------------
                // KEEP THE SEARCH OPEN
                //
                // We deliberately do NOT do:
                //
                //     search.set(String::new());
                //     search_open.set(false);
                //     selected_index.set(0);
                //
                // The user should be able to return to this tab and remember
                // what they searched for.
                // -------------------------------------------------------------

                // -------------------------------------------------------------
                // FIND THE SEARCH INPUT.
                // -------------------------------------------------------------
                if let Some(win) = window() {
                    if let Some(document) = win.document() {

                        if let Some(element) =
                            document.get_element_by_id("autocomplete_field")
                        {
                            // -------------------------------------------------
                            // Convert the DOM element into an input.
                            // -------------------------------------------------
                            if let Some(input) =
                                element.dyn_ref::<web_sys::HtmlInputElement>()
                            {
                                // -------------------------------------------------
                                // Put keyboard focus back into the search box.
                                // -------------------------------------------------
                                let _ = input.focus();

                                // -------------------------------------------------
                                // Select ALL of the existing text.
                                //
                                // This is essentially the programmatic
                                // equivalent of Ctrl+A inside the input.
                                // -------------------------------------------------
                                input.select();
                            }
                        }
                    }
                }











                        // -------------------------------------------------
                        // IMPORTANT:
                        //
                        // DON'T clear the search.
                        // DON'T close the dropdown.
                        // DON'T reset selected_index.
                        //
                        // This lets the user return to this tab and see
                        // exactly what they searched for and which result
                        // they selected.
                        // -------------------------------------------------

                        event.prevent_default();
                    }
                }


                            _ => {}
                        }
                    },







                    /*


                                            onkeydown: move |event| {
                                                match event.key() {
                                                    Key::Escape => {
                                                        search_open.set(false);
                                                        search.set(String::new());
                                                        selected_index.set(0);
                                                    }

                                                    // -------------------------------------------------
                                                    // ARROW DOWN
                                                    //
                                                    // Wrap:
                                                    //
                                                    //     0 -> 1 -> 2 -> 3 -> 0
                                                    // -------------------------------------------------
                                                    Key::ArrowDown => {
                                                        if !matches_for_keyboard.is_empty() {
                                                            let next_index =
                                                                (selected_index() + 1)
                                                                    % matches_for_keyboard.len();

                                                            selected_index.set(next_index);

                                                            event.prevent_default();
                                                        }
                                                    }

                                                    // -------------------------------------------------
                                                    // ARROW UP
                                                    //
                                                    // Wrap:
                                                    //
                                                    //     0 -> last -> last-1 -> ...
                                                    // -------------------------------------------------
                                                    Key::ArrowUp => {
                                                        if !matches_for_keyboard.is_empty() {
                                                            let current = selected_index();

                                                            let previous_index =
                                                                if current == 0 {
                                                                    matches_for_keyboard.len() - 1
                                                                } else {
                                                                    current - 1
                                                                };

                                                            selected_index.set(previous_index);

                                                            event.prevent_default();
                                                        }
                                                    }

                                                    // -------------------------------------------------
                                                    // ENTER
                                                    //
                                                    // Open the currently highlighted result in a new
                                                    // browser tab.
                                                    // -------------------------------------------------
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

                                                            search.set(String::new());
                                                            search_open.set(false);
                                                            selected_index.set(0);

                                                            event.prevent_default();
                                                        }
                                                    }

                                                    _ => {}
                                                }
                                            },




                    */





                                        }

                                        // -----------------------------------------------------------------
                                        // SEARCH RESULTS
                                        // -----------------------------------------------------------------
                                        //
                                        // Desktop:
                                        //
                                        //     absolute right-0 w-96
                                        //
                                        // Mobile:
                                        //
                                        //     fixed left-2 right-2
                                        //
                                        // This is important because a 384px dropdown doesn't really
                                        // fit nicely inside a 430px navbar once padding is included.
                                        //
                                        // On mobile we therefore let the results span almost the
                                        // entire viewport.
                                        // -----------------------------------------------------------------




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







        /*
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
overflow-y-auto
[scrollbar-gutter:stable]
z-50",

        // Switched the last 5 lines above from this in order to get the scrollbar behavior to fix if the
        // list is too long from the searchbox popup!!!
        //                                bg-white rounded-md shadow-xl
        //                                border border-gray-200
        //                                overflow-hidden z-50",

                                            for (index, (name, url)) in matches.iter().enumerate() {
                                                {
                                                    let name = name.clone();
                                                    let url = url.clone();

                                                    let is_selected =
                                                        index == selected_index();

                                                    rsx! {
                                                        button {
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




    */




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
    // PREPARE THE HYDRATION-SAFE URL
    // -------------------------------------------------------------------------
    //
    // IMPORTANT:
    //
    // This code is OUTSIDE rsx!.
    //
    // That is why Rust is allowed to use normal `let` statements here.
    //
    // The resulting String is simply handed to RSX later.
    // -------------------------------------------------------------------------
    let pending_href = item.url.clone().map(|url| {
        // ---------------------------------------------------------------------
        // WASM:
        //
        // Encode the destination URL so it can safely live inside:
        //
        //     #__pending_nav?url=...
        // ---------------------------------------------------------------------
        #[cfg(target_arch = "wasm32")]
        {
            // SO BASICALLY ........
            //
            // js_sys::encode_uri_component() only on WASM,
            // and provide a normal server-side fallback!!!
            //
            // That keeps SSR from trying to execute a browser-only function,
            // so we don't have to deal with it inside the RSX!!!!!!!!!
            let encoded = js_sys::encode_uri_component(&url);

            format!("#__pending_nav?url={}", encoded)
        }

        // ---------------------------------------------------------------------
        // SERVER / SSR:
        //
        // We cannot call JavaScript APIs while rendering on the server.
        //
        // The SSR HTML therefore uses the URL directly as the fallback.
        //
        // Once WASM takes over, the WASM version above is used.
        // ---------------------------------------------------------------------
        #[cfg(not(target_arch = "wasm32"))]
        {
            // For normal URLs this is enough for the SSR fallback.
            //
            // The important part is that SSR rendering does not attempt
            // to call wasm-bindgen functions.
            format!("#__pending_nav?url={}", url)
        }
    });

    rsx! {
        div {
            class: "relative",

            if is_brand {
                a {
                    href: item.url.as_deref().unwrap_or("/"),

                    class: "relative text-white hover:text-blue-400",

                    onclick: move |_| {
                        *IFRAME_URL.write() = Some(String::new());
                    },

                    // -----------------------------------------------------------------
                    // FAVICON
                    //
                    // Keeping the favicon as the brand gives us a nice compact
                    // navbar on mobile too.
                    // -----------------------------------------------------------------
                    img {
                        src: asset!("/assets/favicon.ico"),
                        class: "w-6 h-6",
                        alt: "Admindash3",
                    }

                    // TODO:
                    // For now disabled this but idk if i want this
                    // (no googlefont/external atm so no need atm)
                    //
                    // "{item.name}"
                }
            } else if item.children.is_empty() {
                if let Some(url) = item.url.clone() {
                    a {
                        href: pending_href.unwrap_or_else(|| "#".to_string()),

                        class: "text-white hover:text-blue-400",

                        onclick: move |event| {
                            // WASM is ready, so don't follow the temporary
                            // browser fallback URL.
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
            } else {
                // -----------------------------------------------------------------
                // NAVBAR ITEM WITH CHILDREN
                // -----------------------------------------------------------------
                //
                // This still works inside the mobile hamburger menu.
                //
                // The parent is a button and its children appear underneath it.
                // -----------------------------------------------------------------
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
