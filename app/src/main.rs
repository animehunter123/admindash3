#![allow(non_snake_case)]

use anyhow::Result;
use dioxus::prelude::*;

// My modules
mod buttons;
mod dashboard_data;
mod data_navbar;
mod footer;
mod history;
mod navbar;

use crate::buttons::*;
use crate::dashboard_data::{AdminCredentials, DashboardPayload};
use crate::data_navbar::NavItem;
use crate::footer::Footer;
use crate::history::HistoryPage;
use crate::navbar::Navbar;

// -----------------------------------------------------------------------------
// GLOBAL IFRAME STATE
// -----------------------------------------------------------------------------
pub static IFRAME_URL: GlobalSignal<Option<String>> = Signal::global(|| None);

// -----------------------------------------------------------------------------
// GLOBAL ADMIN AUTHENTICATION STATE
// -----------------------------------------------------------------------------
pub static ADMIN_AUTH: GlobalSignal<AdminCredentials> = Signal::global(AdminCredentials::default);

// -----------------------------------------------------------------------------
// DASHBOARD REFRESH KEY
// -----------------------------------------------------------------------------
//
// Incrementing this value forces the dashboard use_server_future() to run
// again.
//
// Login:
//
//     ADMIN_AUTH = admin/admin
//     DASHBOARD_REFRESH_KEY += 1
//
// Logout:
//
//     ADMIN_AUTH = default credentials
//     DASHBOARD_REFRESH_KEY += 1
//
// The dashboard will then be reloaded from the server using the new
// credentials.
// -----------------------------------------------------------------------------
pub static DASHBOARD_REFRESH_KEY: GlobalSignal<u64> = Signal::global(|| 0);

// -----------------------------------------------------------------------------
// ADMIN HISTORY PAGE STATE
// -----------------------------------------------------------------------------
//
// This is a tiny page switch rather than a full router. Only the admin UI can
// turn it on, and leaving the history page turns it off again.
// -----------------------------------------------------------------------------
pub static HISTORY_PAGE: GlobalSignal<bool> = Signal::global(|| false);

// -----------------------------------------------------------------------------
// SEARCH -> DIRECT ROW EDIT REQUEST
// -----------------------------------------------------------------------------
//
// The navbar owns the search dropdown, while Buttons owns the actual row editor.
// This tiny global request lets the search pencil tell Buttons exactly which
// button/row should be opened without duplicating the row editor UI in navbar.rs.
// The Buttons component consumes the request and immediately clears it.
// -----------------------------------------------------------------------------
pub static EDIT_ROW_REQUEST: GlobalSignal<Option<(String, String)>> = Signal::global(|| None);

// -----------------------------------------------------------------------------
// HELPER: UPDATE A MODAL'S INLINE CSS
// -----------------------------------------------------------------------------
//
// WHY IS THIS HELPER NEEDED?
//
// Normally, web_sys lets us write:
//
//     modal.style().set_property("left", "100px");
//
// However, HtmlElement::style() is only available when the optional
// web-sys CssStyleDeclaration feature is enabled in Cargo.toml.
//
// Your current build does not expose that method, which caused:
//
//     error[E0599]: no method named `style` found for struct `HtmlElement`
//
// Instead of changing Cargo.toml, this helper uses js_sys::Reflect to access
// the browser's existing JavaScript `style` object and its `setProperty()`
// method.
//
// CSS properties such as "max-width" and "left" are passed using their normal
// CSS names. The browser handles those names through setProperty().
//
// This function only runs in the WebAssembly/browser build.
// -----------------------------------------------------------------------------
#[cfg(feature = "web")]
fn set_modal_style(modal: &web_sys::HtmlElement, property: &str, value: &str) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;

    // Get the modal's JavaScript `style` object.
    //
    // Reflect::get returns a Result because JavaScript property access can fail.
    // If it does, return instead of crashing the application.
    let modal_value: &JsValue = modal.as_ref();

    let Ok(style) = js_sys::Reflect::get(modal_value, &JsValue::from_str("style")) else {
        return;
    };

    // Get the style object's setProperty() function.
    let Ok(set_property) = js_sys::Reflect::get(&style, &JsValue::from_str("setProperty")) else {
        return;
    };

    // Convert the JavaScript value into a callable JavaScript function.
    let Ok(set_property) = set_property.dyn_into::<js_sys::Function>() else {
        return;
    };

    // Call style.setProperty(property, value).
    //
    // For example:
    //
    //     set_modal_style(&modal, "left", "100px");
    //
    // has the same CSS effect as:
    //
    //     modal.style.setProperty("left", "100px");
    //
    // The first argument to call2() is the JavaScript `this` value.
    // Here, that must be the style object itself.
    let _ = set_property.call2(
        &style,
        &JsValue::from_str(property),
        &JsValue::from_str(value),
    );
}

// -----------------------------------------------------------------------------
// MAIN
// -----------------------------------------------------------------------------
fn main() -> Result<()> {
    let version = concat!(env!("CARGO_PKG_VERSION"), "-", env!("GIT_HASH"));

    println!(
        "Starting Admindash3 web server version: [{}] on [0.0.0.0:80]...",
        version,
    );

    // Start Dioxus first. Browser localStorage is restored from inside the
    // App runtime below, because GlobalSignal can only be accessed while a
    // Dioxus runtime exists.
    dioxus::launch(App);

    Ok(())
}

// -----------------------------------------------------------------------------
// APP
// -----------------------------------------------------------------------------
#[component]
fn App() -> Element {
    // -------------------------------------------------------------------------
    // RESTORE BROWSER ADMIN SESSION
    // -------------------------------------------------------------------------
    //
    // IMPORTANT: this must happen inside the Dioxus runtime.
    //
    // `ADMIN_AUTH` is a GlobalSignal. Calling `.write()` on that signal from
    // `main()` is too early because `dioxus::launch(App)` has not created the
    // runtime yet. That was the source of:
    //
    //     Must be called from inside a Dioxus runtime.
    //
    // use_effect is also the right place for browser-only localStorage work.
    // On the server there is no browser localStorage, so the helper simply
    // does nothing.
    use_effect(|| {
        crate::navbar::restore_admin_from_browser_storage();
    });

    // -------------------------------------------------------------------------
    // MOVABLE MODALS
    // -------------------------------------------------------------------------
    //
    // Every modal's white panel has the `modal-window` class. This delegated
    // pointer handler lets the user drag a panel by its top strip without
    // adding separate drag state to every individual modal component.
    //
    // Native CSS `resize: both` is applied to those panels too, so the browser
    // provides bottom/right corner resizing while max-width/max-height keep
    // the panel within the viewport on smaller screens.
    #[cfg(feature = "web")]
    {
        use std::cell::RefCell;
        use std::rc::Rc;
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        use_effect(move || {
            let Some(window) = web_sys::window() else {
                return;
            };

            let Some(document) = window.document() else {
                return;
            };

            // Current dragged modal and the pointer offset from its top-left.
            //
            // Rc allows the mouse-down, mouse-move, and mouse-up callbacks
            // to share the same drag state.
            //
            // RefCell allows that shared state to be updated through Rc.
            let dragging: Rc<RefCell<Option<(web_sys::HtmlElement, f64, f64)>>> =
                Rc::new(RefCell::new(None));

            // -----------------------------------------------------------------
            // MOUSE DOWN: decide whether the user started dragging a modal.
            // -----------------------------------------------------------------
            let dragging_for_down = dragging.clone();

            let down = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                let Some(target) = event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                else {
                    return;
                };

                // Walk up from the clicked element until we find a modal
                // panel. This also works when the clicked element is a
                // child inside the modal's title/header area.
                let mut current = Some(target);
                let mut modal: Option<web_sys::HtmlElement> = None;

                while let Some(element) = current {
                    let classes = element.get_attribute("class").unwrap_or_default();

                    if classes
                        .split_whitespace()
                        .any(|class| class == "modal-window")
                    {
                        modal = element.dyn_into::<web_sys::HtmlElement>().ok();
                        break;
                    }

                    current = element.parent_element();
                }

                let Some(modal) = modal else {
                    return;
                };

                // Only the top strip is a drag handle.
                //
                // Do not start dragging when the user clicks a button,
                // input, dropdown, link, or other interactive control.
                let Some(target) = event
                    .target()
                    .and_then(|target| target.dyn_into::<web_sys::Element>().ok())
                else {
                    return;
                };

                let tag = target.tag_name().to_ascii_lowercase();

                if ["button", "input", "select", "textarea", "a"].contains(&tag.as_str()) {
                    return;
                }

                let rect = modal.get_bounding_client_rect();

                // If the user grabs the native bottom-right resize handle,
                // remove the default CSS size limits before the browser starts
                // resizing. Otherwise Tailwind's max-w-* class can stop the
                // modal at its original width even though resize: both is set.
                let near_resize_corner = event.client_x() as f64 >= rect.right() - 24.0
                    && event.client_y() as f64 >= rect.bottom() - 24.0;

                if near_resize_corner {
                    set_modal_style(&modal, "max-width", "none");
                    set_modal_style(&modal, "max-height", "none");
                    set_modal_style(&modal, "overflow", "auto");
                    return;
                }

                // The first 48 pixels of the panel act as the drag area.
                // Clicks farther down should interact with the modal's
                // contents normally.
                if event.client_y() as f64 - rect.top() > 48.0 {
                    return;
                }

                // Convert the centered flex child into a viewport-
                // positioned panel.
                //
                // Save its current screen position and dimensions first,
                // so the panel doesn't jump when dragging begins.
                //
                // IMPORTANT:
                // Use set_modal_style() instead of modal.style().
                // This avoids the missing CssStyleDeclaration feature.
                set_modal_style(&modal, "position", "fixed");
                set_modal_style(&modal, "left", &format!("{}px", rect.left()));
                set_modal_style(&modal, "top", &format!("{}px", rect.top()));
                set_modal_style(&modal, "width", &format!("{}px", rect.width()));
                set_modal_style(&modal, "height", &format!("{}px", rect.height()));
                // The CSS max-width/max-height classes provide sensible defaults
                // when a modal first opens. Once the user starts dragging it,
                // remove those limits so later manual resizing is freeform.
                set_modal_style(&modal, "max-width", "none");
                set_modal_style(&modal, "max-height", "none");
                set_modal_style(&modal, "overflow", "auto");
                set_modal_style(&modal, "margin", "0");
                set_modal_style(&modal, "transform", "none");

                // Store the modal and where inside it the user clicked.
                //
                // Keeping the pointer offset prevents the top-left corner
                // from jumping directly underneath the mouse pointer.
                *dragging_for_down.borrow_mut() = Some((
                    modal,
                    event.client_x() as f64 - rect.left(),
                    event.client_y() as f64 - rect.top(),
                ));

                event.prevent_default();
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);

            // -----------------------------------------------------------------
            // MOUSE MOVE: move the active modal with the pointer.
            // -----------------------------------------------------------------
            let dragging_for_move = dragging.clone();
            let window_for_move = window.clone();

            let moved = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                let active = dragging_for_move.borrow();

                let Some((modal, offset_x, offset_y)) = active.as_ref() else {
                    return;
                };

                // Read the current browser viewport dimensions.
                //
                // The fallback values are only used if the browser does
                // not provide a numeric viewport size.
                let viewport_width = window_for_move
                    .inner_width()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .unwrap_or(1200.0);

                let viewport_height = window_for_move
                    .inner_height()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .unwrap_or(800.0);

                let rect = modal.get_bounding_client_rect();
                let width = rect.width();
                let height = rect.height();

                // Keep the modal inside the visible browser viewport.
                //
                // clamp(min, max) prevents the modal's top-left corner
                // from being dragged outside the available screen area.
                let left = (event.client_x() as f64 - offset_x)
                    .clamp(0.0, (viewport_width - width).max(0.0));

                let top = (event.client_y() as f64 - offset_y)
                    .clamp(0.0, (viewport_height - height).max(0.0));

                // Update the modal's position.
                //
                // These calls replace the old modal.style() code that
                // caused the E0599 compiler error.
                set_modal_style(modal, "left", &format!("{left}px"));
                set_modal_style(modal, "top", &format!("{top}px"));
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);

            // -----------------------------------------------------------------
            // MOUSE UP: stop dragging.
            // -----------------------------------------------------------------
            let dragging_for_up = dragging.clone();

            let up = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                *dragging_for_up.borrow_mut() = None;
            }) as Box<dyn FnMut(web_sys::MouseEvent)>);

            // Register the callbacks on the document so dragging continues
            // even when the pointer moves over another element.
            let _ = document
                .add_event_listener_with_callback("mousedown", down.as_ref().unchecked_ref());

            let _ = document
                .add_event_listener_with_callback("mousemove", moved.as_ref().unchecked_ref());

            let _ =
                document.add_event_listener_with_callback("mouseup", up.as_ref().unchecked_ref());

            let _ = document
                .add_event_listener_with_callback("mouseleave", up.as_ref().unchecked_ref());

            // Clean up browser event listeners when this effect is dropped.
            //
            // Removing listeners matters because the callbacks hold references
            // to Rust closures. Keeping them registered after cleanup could
            // cause stale handlers or memory leaks.
            let document_for_cleanup = document.clone();

            use_drop(move || {
                let _ = document_for_cleanup.remove_event_listener_with_callback(
                    "mousedown",
                    down.as_ref().unchecked_ref(),
                );

                let _ = document_for_cleanup.remove_event_listener_with_callback(
                    "mousemove",
                    moved.as_ref().unchecked_ref(),
                );

                let _ = document_for_cleanup
                    .remove_event_listener_with_callback("mouseup", up.as_ref().unchecked_ref());

                let _ = document_for_cleanup
                    .remove_event_listener_with_callback("mouseleave", up.as_ref().unchecked_ref());
            });
        });
    }

    // -------------------------------------------------------------------------
    // DASHBOARD
    // -------------------------------------------------------------------------
    //
    // Keep this as use_server_future().
    //
    // This gives us the fast SSR/hydration behavior that fixed the white
    // flicker we saw when using use_resource().
    //
    // IMPORTANT:
    //
    // We intentionally DO NOT use .unwrap() here.
    //
    // When DASHBOARD_REFRESH_KEY changes, use_server_future() can temporarily
    // return:
    //
    //     Err(Suspended(...))
    //
    // while the new server request is running.
    //
    // Calling .unwrap() during that period causes the application to panic.
    //
    // Instead, we keep the last successful dashboard in cached_dashboard.
    // -------------------------------------------------------------------------

    let dashboard = use_server_future(move || {
        // ---------------------------------------------------------------------
        // Read the current credentials OUTSIDE the async block.
        // ---------------------------------------------------------------------
        //
        // This makes ADMIN_AUTH a reactive dependency of this future.
        //
        // When ADMIN_AUTH changes, the dashboard can be reloaded with the
        // new credentials.
        // ---------------------------------------------------------------------
        let credentials = ADMIN_AUTH();

        // ---------------------------------------------------------------------
        // Read the refresh key OUTSIDE the async block.
        // ---------------------------------------------------------------------
        //
        // Login/logout changes this value to force a fresh dashboard request.
        // ---------------------------------------------------------------------
        let _refresh_key = DASHBOARD_REFRESH_KEY();

        // ---------------------------------------------------------------------
        // Server request
        // ---------------------------------------------------------------------
        async move { load_dashboard(credentials).await }
    });

    // -------------------------------------------------------------------------
    // CACHED DASHBOARD
    // -------------------------------------------------------------------------
    //
    // This stores the last successfully loaded DashboardPayload.
    //
    // Example:
    //
    //     Anonymous dashboard loaded
    //             ↓
    //     cached_dashboard = anonymous dashboard
    //
    //     Admin logs in
    //             ↓
    //     refresh begins
    //             ↓
    //     future temporarily Suspended
    //             ↓
    //     cached anonymous dashboard remains visible
    //             ↓
    //     admin dashboard arrives
    //             ↓
    //     cached_dashboard = admin dashboard
    //
    // This prevents the dashboard from disappearing during the refresh.
    // -------------------------------------------------------------------------

    let mut cached_dashboard = use_signal(|| None::<DashboardPayload>);

    // -------------------------------------------------------------------------
    // COPY SUCCESSFUL SERVER RESULTS INTO THE CACHE
    // -------------------------------------------------------------------------
    //
    // IMPORTANT RUST OWNERSHIP DETAIL:
    //
    // `dashboard` is also needed later by the RSX rendering code.
    //
    // A `move` closure would otherwise take ownership of it.
    //
    // Therefore we clone the Result<Resource<...>> first and move the clone
    // into this effect.
    // -------------------------------------------------------------------------

    let dashboard_for_effect = dashboard.clone();

    use_effect(move || {
        // ---------------------------------------------------------------------
        // Get the dashboard future.
        // ---------------------------------------------------------------------
        //
        // If use_server_future() is currently suspended, this can be Err(...).
        //
        // In that case we deliberately do NOTHING.
        //
        // The previous cached dashboard remains visible.
        // ---------------------------------------------------------------------
        let Ok(dashboard_resource) = &dashboard_for_effect else {
            return;
        };

        // ---------------------------------------------------------------------
        // Check the actual server-future result.
        // ---------------------------------------------------------------------
        //
        // We only update the cache when the server successfully gives us a
        // DashboardPayload.
        //
        // If the result is None or Some(Err(...)), we leave the existing
        // cache alone.
        // ---------------------------------------------------------------------
        if let Some(Ok(payload)) = dashboard_resource() {
            cached_dashboard.set(Some(payload.clone()));
        }
    });

    // -------------------------------------------------------------------------
    // NAVBAR
    // -------------------------------------------------------------------------
    //
    // Navbar loading is independent of the dashboard refresh mechanism.
    //
    // Therefore the existing .unwrap() behavior is retained here.
    // -------------------------------------------------------------------------

    let navbar = use_server_future(|| async { load_navbar().await }).unwrap();

    // -------------------------------------------------------------------------
    // RENDER
    // -------------------------------------------------------------------------

    rsx! {
        // ---------------------------------------------------------------------
        // FAVICON
        // ---------------------------------------------------------------------
        document::Link {
            rel: "icon",
            href: asset!("/assets/favicon.ico")
        }

        // ---------------------------------------------------------------------
        // TAILWIND CSS
        // ---------------------------------------------------------------------
        document::Stylesheet {
            href: asset!("/assets/tailwind.css")
        }

        // ---------------------------------------------------------------------
        // WHOLE PAGE
        // ---------------------------------------------------------------------
        div {
            // Give the page a fixed viewport-sized flex column.
            //
            // `min-h-screen` only guarantees a minimum height, so the outer
            // page can still grow when the dashboard contains many buttons.
            // With `h-screen overflow-hidden`, the main area is the part that
            // grows and scrolls, while the footer remains in the viewport.
            class: "h-screen flex flex-col overflow-hidden",

            // -----------------------------------------------------------------
            // NAVBAR
            // -----------------------------------------------------------------
            if let Some(Ok(items)) = navbar() {
                Navbar {
                    items: items.clone(),
                    dashboard: cached_dashboard(),
                }
            }

            // -----------------------------------------------------------------
            // MAIN CONTENT
            // -----------------------------------------------------------------
            main {
                // Keep the footer visible at the bottom of the viewport even
                // when the admin dashboard becomes taller after login.
                //
                // The dashboard itself becomes the scrollable region rather
                // than pushing the footer below the viewport.
                class: "min-h-0 flex-1 overflow-y-auto",

                // -------------------------------------------------------------
                // IFRAME
                // -------------------------------------------------------------
                //
                // If an iframe URL is selected, display the iframe instead of
                // the dashboard buttons.
                // -------------------------------------------------------------
                if let Some(url) = IFRAME_URL() {
                    div {
                        class: "w-full",

                        iframe {
                            src: "{url}",
                            class: "w-full h-screen border-0",
                            title: "Embedded application",
                        }
                    }
                } else {
                    // ---------------------------------------------------------
                    // DASHBOARD
                    // ---------------------------------------------------------
                    //
                    // First try the cached dashboard.
                    //
                    // This is the important part of the no-flicker behavior.
                    //
                    // If a refresh is happening, the old dashboard stays on
                    // screen until the new server result arrives.
                    // ---------------------------------------------------------
                    if HISTORY_PAGE() {
                        HistoryPage {}
                    } else if let Some(payload) = cached_dashboard() {
                        Buttons {
                            payload: payload.clone()
                        }
                    } else {
                        // -----------------------------------------------------
                        // INITIAL DASHBOARD LOAD
                        // -----------------------------------------------------
                        //
                        // There is no cached result yet, so we need to look
                        // directly at the server future.
                        // -----------------------------------------------------
                        match &dashboard {
                            // -------------------------------------------------
                            // Dashboard future exists.
                            // -------------------------------------------------
                            Ok(dashboard_resource) => {
                                match &*dashboard_resource.read_unchecked() {
                                    // -----------------------------------------
                                    // Successful dashboard load.
                                    // -----------------------------------------
                                    Some(Ok(payload)) => rsx! {
                                        Buttons {
                                            payload: payload.clone()
                                        }
                                    },

                                    // -----------------------------------------
                                    // Server returned an actual error.
                                    // -----------------------------------------
                                    Some(Err(error)) => rsx! {
                                        ErrorPanel {
                                            message: error.to_string()
                                        }
                                    },

                                    // -----------------------------------------
                                    // Still waiting for the initial result.
                                    // -----------------------------------------
                                    None => rsx! {
                                        LoadingPanel {}
                                    },
                                }
                            }

                            // -------------------------------------------------
                            // Future is suspended.
                            // -------------------------------------------------
                            //
                            // This can happen during a refresh.
                            //
                            // Normally cached_dashboard will already contain
                            // the previous dashboard, so this branch is mainly
                            // a safe fallback for the very first load.
                            // -------------------------------------------------
                            Err(_) => rsx! {
                                LoadingPanel {}
                            },
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // FOOTER
            // -----------------------------------------------------------------
            Footer {}
        }
    }
}

// -----------------------------------------------------------------------------
// LOADING PANEL
// -----------------------------------------------------------------------------
#[component]
fn LoadingPanel() -> Element {
    rsx! {
        div {
            class: "container mx-auto px-4 mt-10",

            div {
                class: "rounded-lg border border-gray-200 bg-white p-6 shadow-sm text-gray-700",

                "Loading dashboard data..."
            }
        }
    }
}

// -----------------------------------------------------------------------------
// ERROR PANEL
// -----------------------------------------------------------------------------
#[component]
fn ErrorPanel(message: String) -> Element {
    rsx! {
        div {
            class: "container mx-auto px-4 mt-10",

            div {
                class: "rounded-lg border border-red-200 bg-red-50 p-6 shadow-sm",

                h2 {
                    class: "text-lg font-semibold text-red-700",

                    "Dashboard Data Error"
                }

                pre {
                    class: "mt-3 whitespace-pre-wrap break-words text-sm text-red-900",

                    "{message}"
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// LOAD NAVBAR JSON
// -----------------------------------------------------------------------------
#[server]
async fn load_navbar() -> Result<Vec<NavItem>, ServerFnError> {
    let source_json_file = "./data_navbar.json";

    // -------------------------------------------------------------------------
    // Read JSON from disk on the server.
    // -------------------------------------------------------------------------
    let fsdata =
        std::fs::read_to_string(source_json_file).expect("couldn't read file data_navbar.json");

    // -------------------------------------------------------------------------
    // Convert JSON into our navbar structs.
    // -------------------------------------------------------------------------
    let items: Vec<NavItem> =
        serde_json::from_str(&fsdata).expect("couldn't deserialize data_navbar.json");

    Ok(items)
}

// -----------------------------------------------------------------------------
// LOAD DASHBOARD
// -----------------------------------------------------------------------------
#[server]
async fn load_dashboard(credentials: AdminCredentials) -> Result<DashboardPayload, ServerFnError> {
    dashboard_data::load_dashboard_payload_from_path(
        std::path::Path::new(dashboard_data::BUTTONS_V3_FILE),
        &credentials,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}
