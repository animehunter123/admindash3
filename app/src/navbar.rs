use crate::dashboard_data::{ADMIN_PASSWORD, ADMIN_USERNAME, ButtonRowView, DashboardPayload};
use crate::data_navbar::NavItem;
use crate::{ADMIN_AUTH, DASHBOARD_REFRESH_KEY, EDIT_ROW_REQUEST, IFRAME_URL};

use dioxus::prelude::*;
use dioxus_web::WebEventExt;

use wasm_bindgen::{JsCast, JsValue};
use web_sys::window;

const SEARCH_STORAGE_KEY: &str = "admindash3_search";
const ADMIN_STORAGE_KEY: &str = "admindash3_admin";
const ADMIN_USERNAME_STORAGE_KEY: &str = "admindash3_admin_username";
const ADMIN_PASSWORD_STORAGE_KEY: &str = "admindash3_admin_password";
const ADMIN_SESSION_EXPIRES_STORAGE_KEY: &str = "admindash3_admin_session_expires";
const ADMIN_SESSION_DURATION_MS: f64 = 60.0 * 60.0 * 1000.0;

fn browser_storage_get(key: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let key = serde_json::to_string(key).ok()?;
        js_sys::eval(&format!("localStorage.getItem({key})"))
            .ok()
            .and_then(|value| value.as_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
        None
    }
}

fn browser_storage_set(key: &str, value: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let (Ok(key), Ok(value)) = (serde_json::to_string(key), serde_json::to_string(value)) {
            let _ = js_sys::eval(&format!("localStorage.setItem({key}, {value})"));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (key, value);
    }
}

fn browser_storage_remove(key: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Ok(key) = serde_json::to_string(key) {
            let _ = js_sys::eval(&format!("localStorage.removeItem({key})"));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = key;
    }
}

fn expire_admin_session_and_reload() {
    *ADMIN_AUTH.write() = Default::default();
    browser_storage_remove(ADMIN_STORAGE_KEY);
    browser_storage_remove(ADMIN_USERNAME_STORAGE_KEY);
    browser_storage_remove(ADMIN_PASSWORD_STORAGE_KEY);
    browser_storage_remove(ADMIN_SESSION_EXPIRES_STORAGE_KEY);
    *DASHBOARD_REFRESH_KEY.write() += 1;
    if let Some(win) = window() {
        let _ = win.location().reload();
    }
}

// Restore an existing admin session synchronously during the first client
// render. This is intentionally separate from the normal `use_effect` used
// for search restoration.
//
// If this happened inside an effect, `use_server_future()` would first see
// anonymous credentials, render/load that dashboard, and only afterwards see
// the restored admin credentials. That second request is what caused the
// small admin flicker after F5.
pub fn restore_admin_from_browser_storage() {
    if ADMIN_AUTH.peek().is_admin() {
        return;
    }

    if browser_storage_get(ADMIN_STORAGE_KEY).as_deref() != Some("1") {
        return;
    }

    let Some(expires_at) = browser_storage_get(ADMIN_SESSION_EXPIRES_STORAGE_KEY)
        .and_then(|value| value.parse::<f64>().ok())
    else {
        expire_admin_session_and_reload();
        return;
    };

    if expires_at <= js_sys::Date::now() {
        expire_admin_session_and_reload();
        return;
    }

    let username = browser_storage_get(ADMIN_USERNAME_STORAGE_KEY)
        .unwrap_or_else(|| ADMIN_USERNAME.to_string());
    let password = browser_storage_get(ADMIN_PASSWORD_STORAGE_KEY)
        .unwrap_or_else(|| ADMIN_PASSWORD.to_string());

    *ADMIN_AUTH.write() = crate::dashboard_data::AdminCredentials { username, password };
}

// -----------------------------------------------------------------------------
// SEARCH RESULT
// -----------------------------------------------------------------------------

#[derive(Clone, PartialEq)]
struct SearchResult {
    button_name: String,

    row_name: String,

    url: String,

    // Optional free-form notes attached to the URL.
    // These are displayed in the search result when present.
    comments: String,

    // All tags belonging to this result.
    all_tags: Vec<String>,

    // Only tags which matched the search.
    matched_tags: Vec<String>,

    has_secret: bool,

    secret_text: Option<String>,
}

// -----------------------------------------------------------------------------
// NAVBAR
// -----------------------------------------------------------------------------

#[component]
pub fn Navbar(items: Vec<NavItem>, dashboard: Option<DashboardPayload>) -> Element {
    let mut search = use_signal(String::new);

    let mut search_open = use_signal(|| false);

    let mut selected_index = use_signal(|| 0usize);

    let mut mobile_menu_open = use_signal(|| false);

    let mut login_open = use_signal(|| false);

    let mut logout_open = use_signal(|| false);

    let mut login_username = use_signal(String::new);

    let mut login_password = use_signal(String::new);

    let mut login_error = use_signal(|| None::<String>);

    // -------------------------------------------------------------------------
    // PAGE STATE
    // -------------------------------------------------------------------------

    let is_main_page = {
        let iframe_url = IFRAME_URL.read();

        iframe_url
            .as_deref()
            .map(|url| url.is_empty())
            .unwrap_or(true)
    };

    let auth = ADMIN_AUTH();

    // -------------------------------------------------------------------------
    // PERSIST SEARCH + ADMIN LOGIN ACROSS F5
    // -------------------------------------------------------------------------
    //
    // Dioxus signals live only for the current page instance.  localStorage
    // gives us a tiny browser-side persistence layer so F5 can restore the
    // useful UI state.
    //
    // We store only an "admin logged in" flag, not the password.  The server
    // still receives the normal AdminCredentials when a dashboard request is
    // made.
    // -------------------------------------------------------------------------

    // Restore the search text and admin state after hydration.
    use_effect(move || {
        let Some(win) = window() else {
            return;
        };

        // -------------------------------------------------------------
        // Restore search text.
        // -------------------------------------------------------------
        if let Some(saved_search) = browser_storage_get(SEARCH_STORAGE_KEY) {
            if !saved_search.is_empty() {
                search.set(saved_search.clone());
                search_open.set(true);

                // The input is rendered after this effect.  Selecting on the
                // next browser tick makes F5 feel like the user clicked it.
                let callback = wasm_bindgen::closure::Closure::once(Box::new(move || {
                    if let Some(win) = window() {
                        if let Some(document) = win.document() {
                            if let Some(element) = document.get_element_by_id("autocomplete_field")
                            {
                                if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>()
                                {
                                    let _ = input.focus();
                                    input.select();
                                }
                            }
                        }
                    }
                })
                    as Box<dyn FnOnce()>);

                let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                    callback.as_ref().unchecked_ref(),
                    0,
                );
                callback.forget();
            }
        }

        // Admin authentication is restored synchronously by App before its
        // dashboard `use_server_future()` is created. Doing it there avoids
        // the anonymous-first-render -> admin-second-render F5 flicker.
    });

    // -------------------------------------------------------------------------
    // 60-MINUTE ADMIN SESSION TIMEOUT + CROSS-TAB LOGOUT
    // -------------------------------------------------------------------------
    // The expiry timestamp is shared through localStorage, so every browser
    // tab uses the same 60-minute login window. A timeout handles the current
    // tab; the storage listener handles logout/expiry from another tab.
    use_effect(move || {
        // Read ADMIN_AUTH reactively so the timeout is installed immediately
        // after a successful login, not only on the initial page render.
        if !ADMIN_AUTH().is_admin() {
            return;
        }

        let Some(win) = window() else {
            return;
        };
        let Some(value) = browser_storage_get(ADMIN_SESSION_EXPIRES_STORAGE_KEY) else {
            return;
        };
        let Ok(expires_at) = value.parse::<f64>() else {
            return;
        };

        let remaining_ms = (expires_at - js_sys::Date::now()).max(0.0);
        let timeout_ms = remaining_ms.min(ADMIN_SESSION_DURATION_MS) as i32;

        let timeout_callback = wasm_bindgen::closure::Closure::once(Box::new(move || {
            if ADMIN_AUTH.peek().is_admin() {
                expire_admin_session_and_reload();
            }
        })
            as Box<dyn FnOnce()>);

        let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
            timeout_callback.as_ref().unchecked_ref(),
            timeout_ms,
        );
        timeout_callback.forget();

        let storage_closure =
            wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::Event| {
                // Some web-sys builds do not expose StorageEvent as a generated
                // Rust type.  The browser still gives us the normal Event, and
                // the StorageEvent `key` property can be read safely through JS.
                let key = js_sys::Reflect::get(&event, &JsValue::from_str("key"))
                    .ok()
                    .and_then(|value| value.as_string());

                if key.as_deref() == Some(ADMIN_STORAGE_KEY)
                    || key.as_deref() == Some(ADMIN_SESSION_EXPIRES_STORAGE_KEY)
                {
                    let expired = browser_storage_get(ADMIN_SESSION_EXPIRES_STORAGE_KEY)
                        .and_then(|value| value.parse::<f64>().ok())
                        .map(|expires_at| expires_at <= js_sys::Date::now())
                        .unwrap_or(true);

                    if expired && ADMIN_AUTH.peek().is_admin() {
                        expire_admin_session_and_reload();
                    }
                }
            }) as Box<dyn FnMut(web_sys::Event)>);

        let _ = win
            .add_event_listener_with_callback("storage", storage_closure.as_ref().unchecked_ref());

        let win_for_cleanup = win.clone();
        use dioxus::prelude::use_drop;
        use_drop(move || {
            let _ = win_for_cleanup.remove_event_listener_with_callback(
                "storage",
                storage_closure.as_ref().unchecked_ref(),
            );
        });
    });

    // When the admin login modal opens, put the cursor straight into the
    // password field.  The small timeout lets the conditional modal finish
    // rendering before we ask the browser to focus it.
    use_effect(move || {
        if !login_open() {
            return;
        }

        let callback = wasm_bindgen::closure::Closure::once(Box::new(move || {
            if let Some(win) = window() {
                if let Some(document) = win.document() {
                    if let Some(element) = document.get_element_by_id("admin_password_field") {
                        if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
                            let _ = input.focus();
                            input.select();
                        }
                    }
                }
            }
        }) as Box<dyn FnOnce()>);

        if let Some(win) = window() {
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                0,
            );
        }
        callback.forget();
    });

    // Focus the logout confirmation button whenever the modal opens.
    use_effect(move || {
        if !logout_open() {
            return;
        }

        let callback = wasm_bindgen::closure::Closure::once(Box::new(move || {
            if let Some(win) = window() {
                if let Some(document) = win.document() {
                    if let Some(element) = document.get_element_by_id("admin_logout_confirm_button")
                    {
                        if let Some(button) = element.dyn_ref::<web_sys::HtmlElement>() {
                            let _ = button.focus();
                        }
                    }
                }
            }
        }) as Box<dyn FnOnce()>);

        if let Some(win) = window() {
            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                0,
            );
        }
        callback.forget();
    });

    // -------------------------------------------------------------------------
    // PENDING NAVIGATION
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
            if let Some(encoded_url) = query_string.strip_prefix("url=") {
                #[cfg(target_arch = "wasm32")]
                {
                    let decoded = js_sys::decode_uri_component(encoded_url)
                        .ok()
                        .and_then(|value| value.as_string());

                    if let Some(url) = decoded {
                        *IFRAME_URL.write() = Some(url);
                    }
                }
            }

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
    // Ctrl+K or "/" opens the search box.
    // -----------------------------------------------------------------------------

    {
        use wasm_bindgen::closure::Closure;

        use_effect(move || {
            let Some(win) = window() else {
                return;
            };

            let Some(document) = win.document() else {
                return;
            };

            // The event closure needs its own Document.
            let document_for_handler = document.clone();

            let mut search_open_for_handler = search_open;

            let mut selected_index_for_handler = selected_index;

            let mut login_open_for_handler = login_open;
            let mut logout_open_for_handler = logout_open;
            let mut login_username_for_handler = login_username;
            let mut login_password_for_handler = login_password;
            let mut login_error_for_handler = login_error;

            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                // -------------------------------------------------
                // Admin/search toggle shortcuts.
                //
                // Handle these before the form-field check so the
                // shortcuts work while the password box/search box
                // has focus.
                // -------------------------------------------------

                let key = event.key();

                // Escape closes the Admin Login modal.
                //
                // This is handled before the normal form-field check
                // so it works even while the password box has focus.
                if key == "Escape" && *login_open_for_handler.peek() {
                    event.prevent_default();
                    login_open_for_handler.set(false);
                    login_error_for_handler.set(None);
                    login_password_for_handler.set(String::new());
                    return;
                }

                // Escape also closes the logout confirmation modal.
                if key == "Escape" && *logout_open_for_handler.peek() {
                    event.prevent_default();
                    logout_open_for_handler.set(false);
                    return;
                }

                if key == "\\" {
                    event.prevent_default();

                    if ADMIN_AUTH.peek().is_admin() {
                        search_open_for_handler.set(false);
                        selected_index_for_handler.set(0);
                        logout_open_for_handler.set(true);
                        login_error_for_handler.set(None);
                    } else if !*login_open_for_handler.peek() {
                        search_open_for_handler.set(false);
                        selected_index_for_handler.set(0);
                        login_error_for_handler.set(None);
                        login_username_for_handler.set(ADMIN_USERNAME.to_string());
                        login_password_for_handler.set(ADMIN_PASSWORD.to_string());
                        login_open_for_handler.set(true);
                    }

                    return;
                }

                // -------------------------------------------------
                // Don't activate the normal search shortcut inside
                // form fields.
                // -------------------------------------------------

                let Some(active) = document_for_handler.active_element() else {
                    return;
                };

                let tag = active.tag_name().to_lowercase();

                if tag == "input"
                    || tag == "textarea"
                    || tag == "select"
                    || active.has_attribute("contenteditable")
                {
                    return;
                }

                // -------------------------------------------------
                // Check normal search shortcut.
                // -------------------------------------------------

                let open_search = key == "/" || (key.eq_ignore_ascii_case("k") && event.ctrl_key());

                if !open_search {
                    return;
                }

                event.prevent_default();

                // -------------------------------------------------
                // Find search field.
                // -------------------------------------------------

                let Some(element) = document_for_handler.get_element_by_id("autocomplete_field")
                else {
                    return;
                };

                if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
                    let _ = input.focus();

                    input.select();
                }

                search_open_for_handler.set(true);

                selected_index_for_handler.set(0);
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            let callback_ref = closure.as_ref().unchecked_ref();

            let _ = document.add_event_listener_with_callback("keydown", callback_ref);

            // Keep the callback alive.
            closure.forget();
        });
    }

    // -------------------------------------------------------------------------
    // SEARCH QUERY
    // -------------------------------------------------------------------------
    //
    // We remove ALL leading '#' characters.
    //
    // Therefore:
    //
    //     rust
    //     #rust
    //     ##rust
    //
    // all search for:
    //
    //     rust
    // -----------------------------------------------------------------------------

    let query = search().trim().to_lowercase();

    let normalized_query = query.trim_start_matches('#').to_string();

    // -------------------------------------------------------------------------
    // SEARCH RESULTS
    // -------------------------------------------------------------------------

    let matches: Vec<SearchResult> = if normalized_query.is_empty() {
        Vec::new()
    } else if let Some(dashboard) = dashboard.as_ref() {
        dashboard
            .buttons
            .iter()
            .flat_map(|button| {
                button.rows.iter().filter_map(|row| {
                    match row {
                        // -------------------------------------------------
                        // DIVIDERS ARE NOT SEARCH RESULTS
                        // -------------------------------------------------
                        ButtonRowView::Divider { .. } => None,

                        // -------------------------------------------------
                        // LINK
                        // -------------------------------------------------
                        ButtonRowView::Link {
                            name,
                            url,
                            hashtags,
                            hashtag_tokens,
                            has_secret,
                            secret_text,
                            comments,
                            ..
                        } => {
                            // Search URL/name.
                            let name_matches = name.to_lowercase().contains(&normalized_query);

                            // -------------------------------------------------
                            // Search tags.
                            //
                            // We remove '#' before comparing.
                            //
                            // So:
                            //
                            //     #rust
                            //
                            // becomes:
                            //
                            //     rust
                            // -------------------------------------------------

                            let matched_tags: Vec<String> = hashtag_tokens
                                .iter()
                                .filter(|tag| {
                                    tag.to_lowercase()
                                        .trim_start_matches('#')
                                        .contains(&normalized_query)
                                })
                                .cloned()
                                .collect();

                            // -------------------------------------------------
                            // No name match AND no tag match?
                            //
                            // Then don't return this result.
                            // -------------------------------------------------

                            if !name_matches && matched_tags.is_empty() {
                                return None;
                            }

                            Some(SearchResult {
                                button_name: button.name.clone(),

                                row_name: name.clone(),

                                url: url.clone(),

                                comments: comments.clone(),

                                all_tags: if hashtags.is_empty() {
                                    Vec::new()
                                } else {
                                    hashtag_tokens.clone()
                                },

                                matched_tags,

                                has_secret: *has_secret,

                                secret_text: secret_text.clone(),
                            })
                        }
                    }
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    // -------------------------------------------------------------------------
    // KEEP SELECTED INDEX VALID
    // -------------------------------------------------------------------------

    if !matches.is_empty() {
        let current_index = selected_index();

        if current_index >= matches.len() {
            selected_index.set(matches.len() - 1);
        }
    } else if selected_index() != 0 {
        selected_index.set(0);
    }

    // -------------------------------------------------------------------------
    // SCROLL SELECTED RESULT INTO VIEW
    // -------------------------------------------------------------------------

    let scroll_selected_into_view = move |index: usize| {
        if let Some(win) = window() {
            let callback = wasm_bindgen::closure::Closure::once(Box::new(move || {
                if let Some(win) = window() {
                    if let Some(document) = win.document() {
                        if let Some(element) =
                            document.get_element_by_id(&format!("search-result-{index}"))
                        {
                            element.scroll_into_view_with_bool(false);
                        }
                    }
                }
            })
                as Box<dyn FnOnce()>);

            let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                0,
            );

            callback.forget();
        }
    };

    // -------------------------------------------------------------------------
    // RENDER
    // -------------------------------------------------------------------------

    rsx! {

        nav {
            class: "relative bg-gray-900 p-4 flex items-center gap-2",


            // -----------------------------------------------------------------
            // SEARCH BACKDROP
            // -----------------------------------------------------------------

            if is_main_page
                && search_open()
                && !matches.is_empty()
            {

                div {
                    class: "fixed inset-0 z-40",

                    onclick: move |_| {
                        search_open.set(false);
                    },
                }
            }


            // -----------------------------------------------------------------
            // MOBILE MENU BUTTON
            // -----------------------------------------------------------------

            button {
                class: "hidden max-[430px]:flex
                        items-center justify-center
                        w-10 h-10 rounded-md text-white
                        hover:bg-gray-800
                        focus:outline-none
                        focus:ring-2
                        focus:ring-blue-500",

                onclick: move |_| {
                    mobile_menu_open.set(
                        !mobile_menu_open()
                    );
                },

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
            // NAVIGATION
            // -----------------------------------------------------------------

            ul {
                class: "flex gap-6 items-center
                        relative z-50",

                for (index, item)
                    in items.iter().enumerate()
                {

                    li {
                        class:
                            if index == 0 {
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
            // MOBILE MENU
            // -----------------------------------------------------------------

            if mobile_menu_open() {

                div {
                    class: "absolute left-2 top-16 z-50
                            max-[430px]:block
                            min-[431px]:hidden
                            w-56 bg-gray-900
                            border border-gray-700
                            rounded-md shadow-xl p-2",

                    for (index, item)
                        in items.iter().enumerate()
                    {

                        if index != 0 {

                            div {
                                class: "px-2 py-2
                                        hover:bg-gray-800
                                        rounded-md",

                                onclick: move |_| {
                                    mobile_menu_open
                                        .set(false);
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
            // RIGHT SIDE
            // -----------------------------------------------------------------

            div {
                class: "ml-auto flex items-center
                        gap-3 relative z-50",


                // -------------------------------------------------------------
                // SEARCH
                // -------------------------------------------------------------

                div {
                    class: "relative w-64
                            max-[430px]:w-40",

                    input {
                        id: "autocomplete_field",

                        autofocus: true,

                        value: "{search}",

                        placeholder: "Quick Search... (Press '/')",

                        class:
                            if is_main_page {

                                "w-64 max-[430px]:w-40
                                 px-3 py-2 rounded-md
                                 bg-gray-800 text-white
                                 placeholder-gray-400
                                 border border-gray-700
                                 focus:outline-none
                                 focus:ring-2
                                 focus:ring-blue-500"

                            } else {

                                "w-64 max-[430px]:w-40
                                 px-3 py-2 rounded-md
                                 bg-gray-800 text-white
                                 placeholder-gray-400
                                 border border-gray-700
                                 focus:outline-none
                                 focus:ring-2
                                 focus:ring-blue-500
                                 opacity-0
                                 pointer-events-none"
                            },


                        // -----------------------------------------------------
                        // FOCUS
                        // -----------------------------------------------------

                        onfocus: move |event| {

                            search_open.set(true);

                            selected_index.set(0);


                            let element =
                                event
                                    .data()
                                    .as_web_event();


                            if let Some(input) =
                                element
                                    .dyn_ref::<
                                        web_sys::HtmlInputElement
                                    >()
                            {

                                input.select();
                            }
                        },


                        // -----------------------------------------------------
                        // INPUT
                        // -----------------------------------------------------

                        oninput: move |event| {
                            let value = event.value();

                            search.set(value.clone());
                            search_open.set(true);
                            selected_index.set(0);

                            if value.is_empty() {
                                browser_storage_remove(SEARCH_STORAGE_KEY);
                            } else {
                                browser_storage_set(SEARCH_STORAGE_KEY, &value);
                            }
                        },


                        // -----------------------------------------------------
                        // KEYBOARD NAVIGATION
                        // -----------------------------------------------------

                        onkeydown:
                            move |event: KeyboardEvent| {

                                let is_shift =
                                    event
                                        .modifiers()
                                        .shift();


                                match event.key() {

                                    // -------------------------------------------------
                                    // ESCAPE
                                    // -------------------------------------------------

                                    Key::Escape => {

                                        search_open
                                            .set(false);

                                        search.set(
                                            String::new()
                                        );

                                        browser_storage_remove(SEARCH_STORAGE_KEY);

                                        selected_index
                                            .set(0);

                                        event
                                            .prevent_default();
                                    }


                                    // -------------------------------------------------
                                    // DOWN
                                    // -------------------------------------------------

                                    Key::ArrowDown => {

                                        if !matches
                                            .is_empty()
                                        {

                                            let current =
                                                selected_index();

                                            let next_index =
                                                (
                                                    current + 1
                                                )
                                                %
                                                matches.len();

                                            selected_index
                                                .set(
                                                    next_index
                                                );

                                            scroll_selected_into_view(
                                                next_index
                                            );

                                            event
                                                .prevent_default();
                                        }
                                    }


                                    // -------------------------------------------------
                                    // TAB
                                    // -------------------------------------------------

                                    Key::Tab
                                        if !is_shift =>
                                    {

                                        if !matches
                                            .is_empty()
                                        {

                                            let current =
                                                selected_index();

                                            let next_index =
                                                (
                                                    current + 1
                                                )
                                                %
                                                matches.len();

                                            selected_index
                                                .set(
                                                    next_index
                                                );

                                            scroll_selected_into_view(
                                                next_index
                                            );

                                            event
                                                .prevent_default();
                                        }
                                    }


                                    // -------------------------------------------------
                                    // UP
                                    // -------------------------------------------------

                                    Key::ArrowUp => {

                                        if !matches
                                            .is_empty()
                                        {

                                            let current =
                                                selected_index();

                                            let previous_index =
                                                if current == 0 {

                                                    matches.len()
                                                        - 1

                                                } else {

                                                    current - 1
                                                };

                                            selected_index
                                                .set(
                                                    previous_index
                                                );

                                            scroll_selected_into_view(
                                                previous_index
                                            );

                                            event
                                                .prevent_default();
                                        }
                                    }


                                    // -------------------------------------------------
                                    // SHIFT + TAB
                                    // -------------------------------------------------

                                    Key::Tab
                                        if is_shift =>
                                    {

                                        if !matches
                                            .is_empty()
                                        {

                                            let current =
                                                selected_index();

                                            let previous_index =
                                                if current == 0 {

                                                    matches.len()
                                                        - 1

                                                } else {

                                                    current - 1
                                                };

                                            selected_index
                                                .set(
                                                    previous_index
                                                );

                                            scroll_selected_into_view(
                                                previous_index
                                            );

                                            event
                                                .prevent_default();
                                        }
                                    }


                                    // -------------------------------------------------
                                    // ENTER
                                    // -------------------------------------------------

                                    Key::Enter => {

                                        if let Some(result) =
                                            matches
                                                .get(
                                                    selected_index()
                                                )
                                                .cloned()
                                        {

                                            if let Some(win) =
                                                window()
                                            {

                                                let _ =
                                                    win
                                                        .open_with_url_and_target(
                                                            &result.url,
                                                            "_blank",
                                                        );
                                            }

                                            // Enter navigates in a new tab, but the
                                            // current search UI stays exactly where
                                            // it is: open, highlighted, and focused.
                                            search_open.set(true);


                                            // Keep search field focused.
                                            if let Some(win) =
                                                window()
                                            {

                                                if let Some(document) =
                                                    win.document()
                                                {

                                                    if let Some(element) =
                                                        document
                                                            .get_element_by_id(
                                                                "autocomplete_field"
                                                            )
                                                    {

                                                        if let Some(input) =
                                                            element
                                                                .dyn_ref::<
                                                                    web_sys::HtmlInputElement
                                                                >()
                                                        {

                                                            let _ =
                                                                input.focus();

                                                            input.select();
                                                        }
                                                    }
                                                }
                                            }

                                            event
                                                .prevent_default();
                                        }
                                    }


                                    _ => {}
                                }
                            },
                    }


                    // ---------------------------------------------------------
                    // SEARCH RESULTS
                    // ---------------------------------------------------------

                    if is_main_page
                        && search_open()
                        && !matches.is_empty()
                    {

                        div {
                            class: "absolute right-0 mt-2
                                    w-[28rem]
                                    max-[430px]:fixed
                                    max-[430px]:left-2
                                    max-[430px]:right-2
                                    max-[430px]:top-16
                                    max-[430px]:mt-0
                                    max-[430px]:w-auto
                                    bg-white rounded-md
                                    shadow-xl
                                    border border-gray-200
                                    max-h-[calc(100dvh-100px)]
                                    overflow-y-auto
                                    [scrollbar-gutter:stable]
                                    z-50",


                            for (index, result)
                                in matches.iter().enumerate()
                            {

                                {
                                    let result =
                                        result.clone();

                                    let is_selected =
                                        index
                                            == selected_index();


                                    // -------------------------------------------------
                                    // Which tags should we display?
                                    //
                                    // If a tag matched, only show matching tags.
                                    //
                                    // Otherwise show all tags.
                                    // -------------------------------------------------

                                    let visible_tags =
                                        if result
                                            .matched_tags
                                            .is_empty()
                                        {

                                            result
                                                .all_tags
                                                .clone()

                                        } else {

                                            result
                                                .matched_tags
                                                .clone()
                                        };


                                    rsx! {

                                        button {
                                            id:
                                                "search-result-{index}",

                                            class:
                                                if is_selected {

                                                    "group block w-full
                                                     border-b
                                                     border-gray-100
                                                     bg-blue-50
                                                     px-4 py-3
                                                     text-left
                                                     text-blue-800"

                                                } else {

                                                    "group block w-full
                                                     border-b
                                                     border-gray-100
                                                     px-4 py-3
                                                     text-left
                                                     text-gray-800
                                                     hover:bg-blue-50
                                                     hover:text-blue-800"
                                                },


                                            onclick: move |_| {

                                                selected_index
                                                    .set(index);


                                                if let Some(win) =
                                                    window()
                                                {

                                                    let _ =
                                                        win
                                                            .open_with_url_and_target(
                                                                &result.url,
                                                                "_blank",
                                                            );
                                                }


                                                search.set(
                                                    String::new()
                                                );

                                                browser_storage_remove(SEARCH_STORAGE_KEY);

                                                search_open
                                                    .set(false);

                                                selected_index
                                                    .set(0);
                                            },


                                            // -------------------------------------------------
                                            // URL NAME
                                            // -------------------------------------------------

                                            div {
                                                class:
                                                    "font-medium
                                                     break-words",

                                                "{result.row_name}"
                                            }


                                            // -------------------------------------------------
                                            // BUTTON NAME
                                            // -------------------------------------------------

                                            div {
                                                class:
                                                    "mt-1
                                                     text-xs
                                                     text-gray-500",

                                                "{result.button_name}"
                                            }


                                            // -------------------------------------------------
                                            // COMMENTS
                                            // -------------------------------------------------
                                            //
                                            // Keep comments to one line so a long note cannot
                                            // make the search dropdown unexpectedly tall.
                                            // `title` still lets the full comment be seen on hover.

                                            if !result.comments.trim().is_empty() {

                                                div {
                                                    class:
                                                        "mt-1
                                                         truncate
                                                         text-xs
                                                         text-gray-500",

                                                    title: result.comments.clone(),

                                                    "{result.comments}"
                                                }
                                            }


                                            // -------------------------------------------------
                                            // DIRECT EDIT PENCIL
                                            // -------------------------------------------------
                                            //
                                            // This is intentionally a small span instead of a
                                            // nested <button>. The whole search result is already
                                            // a button, and nested buttons would produce invalid
                                            // HTML. It becomes visible only while the result is
                                            // hovered, and only administrators can see it.

                                            if ADMIN_AUTH().is_admin() {
                                                span {
                                                    class: "float-right ml-2 cursor-pointer rounded px-1.5 py-0.5 text-gray-400 opacity-0 transition-opacity hover:bg-gray-200 hover:text-gray-700 group-hover:opacity-100",
                                                    title: "Edit this URL row",
                                                    onclick: move |event| {
                                                        event.stop_propagation();
                                                        event.prevent_default();

                                                        *EDIT_ROW_REQUEST.write() = Some((
                                                            result.button_name.clone(),
                                                            result.row_name.clone(),
                                                        ));

                                                        search.set(String::new());
                                                        browser_storage_remove(SEARCH_STORAGE_KEY);
                                                        search_open.set(false);
                                                        selected_index.set(0);
                                                    },
                                                    "✎"
                                                }
                                            }


                                            // -------------------------------------------------
                                            // TAGS
                                            // -------------------------------------------------

                                            if !visible_tags.is_empty() {

                                                div {
                                                    class:
                                                        "mt-2
                                                         flex
                                                         flex-wrap
                                                         gap-2",


                                                    for tag
                                                        in visible_tags.iter()
                                                    {

                                                        {

                                                            // -------------------------------------------------
                                                            // Did this specific tag match?
                                                            // -------------------------------------------------

                                                            let tag_matched =
                                                                result
                                                                    .matched_tags
                                                                    .iter()
                                                                    .any(
                                                                        |matched|
                                                                            matched == tag
                                                                    );


                                                            rsx! {

                                                                if tag_matched {

                                                                    // -------------------------------------------------
                                                                    // HIGHLIGHT MATCHED TAG
                                                                    // -------------------------------------------------

                                                                    HighlightedTag {
                                                                        tag:
                                                                            tag.clone(),

                                                                        query:
                                                                            normalized_query.clone(),
                                                                    }

                                                                } else {

                                                                    // -------------------------------------------------
                                                                    // NORMAL TAG
                                                                    // -------------------------------------------------

                                                                    span {
                                                                        class:
                                                                            "rounded-full
                                                                             bg-blue-100
                                                                             px-2 py-1
                                                                             text-[11px]
                                                                             font-medium
                                                                             text-blue-700",

                                                                        "{tag}"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }


                                            // -------------------------------------------------
                                            // SECRET
                                            // -------------------------------------------------

                                            if result.has_secret {

                                                div {
                                                    class:
                                                        "mt-2
                                                         text-xs
                                                         text-gray-700",

                                                    if let Some(secret_text) =
                                                        &result.secret_text
                                                    {

                                                        span {
                                                            class:
                                                                "rounded-md
                                                                 bg-amber-100
                                                                 px-2 py-1
                                                                 font-medium
                                                                 text-amber-900
                                                                 break-all",

                                                            "{secret_text}"
                                                        }

                                                    } else {

                                                        span {
                                                            class:
                                                                "inline-flex
                                                                 items-center
                                                                 gap-2
                                                                 rounded-md
                                                                 bg-amber-50
                                                                 px-2 py-1
                                                                 text-amber-700",

                                                            span {
                                                                "🔑"
                                                            }

                                                            span {
                                                                "Secret hidden"
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
                    }
                }


                // -----------------------------------------------------------------
                // ADMIN STATUS
                // -----------------------------------------------------------------

                if auth.is_admin() {

                    div {
                        class:
                            "flex items-center gap-2",

                        span {
                            class:
                                "rounded-full
                                 bg-green-600/20
                                 px-3 py-1
                                 text-xs font-semibold
                                 text-green-200
                                 border
                                 border-green-500/30",

                            "ADMIN"
                        }


                        button {
                            class:
                                "rounded-md
                                 border border-gray-700
                                 px-3 py-2
                                 text-sm text-white
                                 transition
                                 hover:bg-gray-800",

                            onclick: move |_| {
                                logout_open.set(true);
                            },

                            "Logout"
                        }
                    }

                } else {

                    button {
                        class:
                            "rounded-md
                             border border-gray-700
                             px-3 py-2
                             text-sm text-white
                             transition
                             hover:bg-gray-800",

                        onclick: move |_| {

                            search_open.set(false);
                            login_open.set(true);

                            login_error.set(None);
                            login_username.set(ADMIN_USERNAME.to_string());
                            login_password.set(ADMIN_PASSWORD.to_string());


                        },

                        title: "Admin Login (Press '\\')",
                        "Login"
                    }
                }
            }
        }


        // ---------------------------------------------------------------------
        // LOGOUT CONFIRMATION MODAL
        // ---------------------------------------------------------------------

        if logout_open() {
            div {
                class: "fixed inset-0 z-[60] flex items-center justify-center bg-black/50 p-4",
                onclick: move |_| logout_open.set(false),

                div {
                    class: "w-full max-w-sm rounded-lg bg-white p-6 shadow-xl",
                    onclick: move |event| event.stop_propagation(),

                    h2 {
                        class: "text-xl font-semibold text-gray-900",
                        "Logout Admin?"
                    }

                    p {
                        class: "mt-2 text-sm text-gray-600",
                        "Are you sure you want to leave admin edit mode?"
                    }

                    div {
                        class: "mt-3 rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-xs text-amber-800",
                        "Press Escape to cancel, or press Space when the Logout button is focused."
                    }

                    div {
                        class: "mt-6 flex justify-end gap-3",

                        button {
                            class: "rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 transition hover:bg-gray-50",
                            onclick: move |_| logout_open.set(false),
                            "Cancel"
                        }

                        button {
                            id: "admin_logout_confirm_button",
                            autofocus: true,
                            class: "rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-red-700",
                            onclick: move |_| {
                                *ADMIN_AUTH.write() = Default::default();
                                browser_storage_remove(ADMIN_STORAGE_KEY);
                                browser_storage_remove(ADMIN_USERNAME_STORAGE_KEY);
                                browser_storage_remove(ADMIN_PASSWORD_STORAGE_KEY);
                                browser_storage_remove(ADMIN_SESSION_EXPIRES_STORAGE_KEY);
                                *DASHBOARD_REFRESH_KEY.write() += 1;
                                logout_open.set(false);
                                login_error.set(None);
                                search_open.set(true);
                                selected_index.set(0);

                                if let Some(win) = window() {
                                    let callback = wasm_bindgen::closure::Closure::once(Box::new(move || {
                                        if let Some(win) = window() {
                                            if let Some(document) = win.document() {
                                                if let Some(element) = document.get_element_by_id("autocomplete_field") {
                                                    if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
                                                        let _ = input.focus();
                                                        input.select();
                                                    }
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
                            },
                            "Logout"
                        }
                    }
                }
            }
        }

        // ---------------------------------------------------------------------
        // LOGIN MODAL
        // ---------------------------------------------------------------------

        if login_open() {

            div {
                class:
                    "fixed inset-0 z-50
                     flex items-center
                     justify-center
                     bg-black/50 p-4",

                onclick: move |_| {

                    login_open.set(false);

                    login_error.set(None);
                },


                div {
                    class:
                        "w-full max-w-sm
                         rounded-lg
                         bg-white p-6
                         shadow-xl",

                    onclick: move |event| {
                        event.stop_propagation();
                    },


                    h2 {
                        class:
                            "text-xl font-semibold
                             text-gray-900",

                        "Admin Login"
                    }


                    p {
                        class:
                            "mt-2 text-sm
                             text-gray-600",

                        "Enter your administrator credentials to enable editing."
                    }

                    div {
                        class: "mt-3 rounded-md border border-blue-200 bg-blue-50 px-3 py-2 text-xs text-blue-800",
                        "Keyboard shortcuts: press \\ to open Admin Login, or press Escape to close this window."
                    }


                    div {
                        class:
                            "mt-4 space-y-4",


                        label {
                            class: "block",

                            span {
                                class:
                                    "mb-1 block
                                     text-sm
                                     font-medium
                                     text-gray-700",

                                "Username"
                            }


                            input {
                                class:
                                    "w-full
                                     rounded-md
                                     border
                                     border-gray-300
                                     px-3 py-2
                                     focus:border-blue-500
                                     focus:outline-none
                                     focus:ring-2
                                     focus:ring-blue-500/20",

                                value:
                                    login_username(),

                                oninput:
                                    move |event| {

                                        login_username.set(
                                            event.value()
                                        );
                                    },
                            }
                        }


                        label {
                            class: "block",

                            span {
                                class:
                                    "mb-1 block
                                     text-sm
                                     font-medium
                                     text-gray-700",

                                "Password"
                            }


                            input {
                                id: "admin_password_field",
                                r#type: "password",

                                class:
                                    "w-full
                                     rounded-md
                                     border
                                     border-gray-300
                                     px-3 py-2
                                     focus:border-blue-500
                                     focus:outline-none
                                     focus:ring-2
                                     focus:ring-blue-500/20",

                                value:
                                    login_password(),

                                oninput:
                                    move |event| {

                                        login_password.set(
                                            event.value()
                                        );
                                    },

                                onkeydown: move |event: KeyboardEvent| {
                                    if event.key() == Key::Enter {
                                        event.prevent_default();

                                        let credentials =
                                            crate::dashboard_data::AdminCredentials {
                                                username: login_username(),
                                                password: login_password(),
                                            };

                                        spawn(async move {
                                            match authenticate_admin_server(credentials.clone()).await {
                                                Ok(()) => {
                                                    *ADMIN_AUTH.write() = credentials.clone();
                                                    browser_storage_set(ADMIN_STORAGE_KEY, "1");
                                                    browser_storage_set(ADMIN_USERNAME_STORAGE_KEY, &credentials.username);
                                                    browser_storage_set(ADMIN_PASSWORD_STORAGE_KEY, &credentials.password);
                                                    browser_storage_set(
                                                        ADMIN_SESSION_EXPIRES_STORAGE_KEY,
                                                        &(js_sys::Date::now() + ADMIN_SESSION_DURATION_MS).to_string(),
                                                    );
                                                    *DASHBOARD_REFRESH_KEY.write() += 1;
                                                    login_error.set(None);
                                                    login_open.set(false);
                                                }
                                                Err(_) => {
                                                    login_error.set(Some(
                                                        "Invalid login credentials.".to_string()
                                                    ));
                                                }
                                            }
                                        });
                                    }
                                },
                            }
                        }
                    }


                    if let Some(message) =
                        login_error()
                    {

                        div {
                            class:
                                "mt-4
                                 rounded-md
                                 border border-red-200
                                 bg-red-50
                                 px-3 py-2
                                 text-sm
                                 text-red-700",

                            "{message}"
                        }
                    }


                    div {
                        class:
                            "mt-6
                             flex justify-end
                             gap-3",


                        button {
                            class:
                                "rounded-md
                                 border
                                 border-gray-300
                                 px-4 py-2
                                 text-sm
                                 font-medium
                                 text-gray-700
                                 transition
                                 hover:bg-gray-50",

                            onclick: move |_| {

                                login_open
                                    .set(false);

                                login_error
                                    .set(None);
                            },

                            "Cancel"
                        }


                        button {
                            class:
                                "rounded-md
                                 bg-blue-600
                                 px-4 py-2
                                 text-sm
                                 font-medium
                                 text-white
                                 transition
                                 hover:bg-blue-700",

                            onclick: move |_| {

                                let credentials =
                                    crate::dashboard_data
                                        ::AdminCredentials {
                                            username:
                                                login_username(),

                                            password:
                                                login_password(),
                                        };


                                spawn(async move {
                                    match authenticate_admin_server(credentials.clone()).await {
                                        Ok(()) => {
                                            *ADMIN_AUTH.write() = credentials.clone();
                                            browser_storage_set(ADMIN_STORAGE_KEY, "1");
                                            browser_storage_set(ADMIN_USERNAME_STORAGE_KEY, &credentials.username);
                                            browser_storage_set(ADMIN_PASSWORD_STORAGE_KEY, &credentials.password);
                                                    browser_storage_set(
                                                        ADMIN_SESSION_EXPIRES_STORAGE_KEY,
                                                        &(js_sys::Date::now() + ADMIN_SESSION_DURATION_MS).to_string(),
                                                    );
                                            *DASHBOARD_REFRESH_KEY.write() += 1;
                                            login_error.set(None);
                                            login_open.set(false);
                                        }
                                        Err(_) => {
                                            login_error.set(Some(
                                                "Invalid login credentials.".to_string()
                                            ));
                                        }
                                    }
                                });
                            },

                            "Login"
                        }
                    }
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// HIGHLIGHTED TAG
// -----------------------------------------------------------------------------
//
// Example:
//
//     tag   = "#rust"
//     query = "rus"
//
// Produces:
//
//     "#rus" + "t"
//
// The matching part receives a stronger highlight.
//
// Search is case-insensitive, but the original tag capitalization is preserved.
// -----------------------------------------------------------------------------

#[component]
fn HighlightedTag(tag: String, query: String) -> Element {
    // Remove the # only for matching.  The # is rendered separately below.
    let tag_without_hash = tag.trim_start_matches('#');
    let query = query.trim_start_matches('#').to_lowercase();
    let lower_tag = tag_without_hash.to_lowercase();

    let Some(match_start) = lower_tag.find(&query) else {
        return rsx! {
            span {
                class: "rounded-full bg-blue-100 px-2 py-1 text-[11px] font-medium text-blue-700",
                "{tag}"
            }
        };
    };

    let match_end = match_start + query.len();

    // Managed tags are normal ASCII-style hashtags, so these byte offsets are
    // normally identical in the original string.  Keep a safe fallback for
    // future non-ASCII tags rather than slicing through a UTF-8 character.
    if !tag_without_hash.is_char_boundary(match_start)
        || !tag_without_hash.is_char_boundary(match_end)
    {
        return rsx! {
            span {
                class: "rounded-full bg-blue-100 px-2 py-1 text-[11px] font-medium text-blue-700",
                "{tag}"
            }
        };
    }

    let before = &tag_without_hash[..match_start];
    let matched = &tag_without_hash[match_start..match_end];
    let after = &tag_without_hash[match_end..];

    rsx! {
        span {
            class: "rounded-full bg-blue-100 px-2 py-1 text-[11px] font-medium text-blue-700",
            "#"
            "{before}"
            span {
                class: "rounded-sm bg-yellow-300 px-0.5 text-gray-900",
                "{matched}"
            }
            "{after}"
        }
    }
}

// -----------------------------------------------------------------------------
// NAV ENTRY
// -----------------------------------------------------------------------------

#[component]
fn NavEntry(item: NavItem, is_brand: bool) -> Element {
    let mut open = use_signal(|| false);

    let pending_href = item.url.clone().map(|url| {
        #[cfg(target_arch = "wasm32")]
        {
            let encoded = js_sys::encode_uri_component(&url);

            format!("#__pending_nav?url={encoded}")
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            format!("#__pending_nav?url={url}")
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
                    href:
                        item
                            .url
                            .as_deref()
                            .unwrap_or("/"),

                    class:
                        "relative
                         text-white
                         hover:text-blue-400",

                    onclick: move |_| {

                        *IFRAME_URL.write() =
                            Some(String::new());
                    },

                    img {
                        src:
                            asset!(
                                "/assets/favicon.ico"
                            ),

                        class:
                            "w-6 h-6",

                        alt:
                            "Admindash3",
                    }
                }


            // -----------------------------------------------------------------
            // NORMAL LINK
            // -----------------------------------------------------------------

            } else if item.children.is_empty() {

                if let Some(url) =
                    item.url.clone()
                {

                    a {
                        href:
                            pending_href
                                .unwrap_or_else(
                                    || "#".to_string()
                                ),

                        class:
                            "text-white
                             hover:text-blue-400",

                        onclick:
                            move |event| {

                                event
                                    .prevent_default();

                                *IFRAME_URL.write() =
                                    Some(
                                        url.clone()
                                    );
                            },

                        "{item.name}"
                    }

                } else {

                    a {
                        href: "#",

                        class:
                            "text-white
                             hover:text-blue-400",

                        "{item.name}"
                    }
                }


            // -----------------------------------------------------------------
            // DROPDOWN
            // -----------------------------------------------------------------

            } else {

                button {
                    class:
                        "text-white
                         hover:text-blue-400",

                    onclick: move |_| {

                        open.set(
                            !open()
                        );
                    },

                    "{item.name}"

                    span {
                        class:
                            "ml-2",

                        "..."
                    }
                }


                if open() {

                    ul {
                        class:
                            "absolute
                             left-0 mt-2
                             w-48
                             rounded
                             bg-white
                             p-2
                             shadow-lg
                             z-50",

                        for child
                            in item.children.clone()
                        {

                            li {
                                class:
                                    "p-2
                                     hover:bg-gray-100",

                                a {
                                    href:
                                        child
                                            .url
                                            .as_deref()
                                            .unwrap_or("#"),

                                    class:
                                        "text-gray-800",

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

// -----------------------------------------------------------------------------
// SERVER-SIDE ADMIN AUTHENTICATION
// -----------------------------------------------------------------------------

#[server]
async fn authenticate_admin_server(
    credentials: crate::dashboard_data::AdminCredentials,
) -> Result<(), ServerFnError> {
    if credentials.is_admin() {
        Ok(())
    } else {
        Err(ServerFnError::ServerError {
            message: "Invalid login credentials.".to_string(),
            code: 401,
            details: None,
        })
    }
}
