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
        // If the result is:
        //
        //     None
        //
        // or:
        //
        //     Some(Err(...))
        //
        // we leave the existing cache alone.
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
            class: "min-h-screen flex flex-col",


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
                class: "flex-1",


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
