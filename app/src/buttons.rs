use crate::dashboard_data::{
    AdminCredentials, ButtonCreateInput, ButtonRenameInput, ButtonRowInput, ButtonRowView,
    DashboardPayload, RowKind,
};
use crate::{
    ADMIN_AUTH, DASHBOARD_REFRESH_KEY, EDIT_ROW_REQUEST, HISTORY_PAGE,
};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
struct ButtonDraft {
    name: String,
    first_row_name: String,
    first_row_url: String,
    first_row_hashtags: String,
    first_row_secrets: String,
}

#[derive(Clone, PartialEq)]
struct ButtonEditorState {
    mode: ButtonEditorMode,
    draft: ButtonDraft,
}

#[derive(Clone, PartialEq)]
enum ButtonEditorMode {
    Create,
    Edit { original_name: String },
}

#[derive(Clone, PartialEq)]
struct RowDraft {
    kind: RowKind,
    name: String,
    url: String,
    hashtags: String,
    comments: String,
    secrets: String,
}

#[derive(Clone, PartialEq)]
struct RowEditorState {
    button_name: String,
    original_name: Option<String>,
    draft: RowDraft,
}

// Pending drag/drop moves are stored in Dioxus state so both button and
// URL-row reordering can use the same custom confirmation modal.
#[derive(Clone, PartialEq)]
enum ReorderConfirmation {
    Button {
        from_index: usize,
        to_index: usize,
        source_name: String,
        target_name: String,
    },
    Row {
        button_name: String,
        from_index: usize,
        to_index: usize,
        source_name: String,
        target_name: String,
    },
}

impl Default for ButtonDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            first_row_name: String::new(),
            first_row_url: String::new(),
            first_row_hashtags: String::new(),
            first_row_secrets: String::new(),
        }
    }
}

impl RowDraft {
    fn empty_link() -> Self {
        Self {
            kind: RowKind::Link,
            name: String::new(),
            url: String::new(),
            hashtags: String::new(),
            comments: String::new(),
            secrets: String::new(),
        }
    }

    fn from_row(row: &ButtonRowView) -> Self {
        match row {
            ButtonRowView::Divider { name } => Self {
                kind: RowKind::Divider,
                name: name.clone(),
                url: String::new(),
                hashtags: String::new(),
                comments: String::new(),
                secrets: String::new(),
            },
            ButtonRowView::Link {
                name,
                url,
                hashtags,
                comments,
                secret_text,
                ..
            } => Self {
                kind: RowKind::Link,
                name: name.clone(),
                url: url.clone(),
                hashtags: hashtags.clone(),
                comments: comments.clone(),
                secrets: secret_text.clone().unwrap_or_default(),
            },
        }
    }
}

#[component]
pub fn Buttons(payload: DashboardPayload) -> Element {
    // Read the current authentication state once for this render.  The rest
    // of this component uses `is_admin` to decide which editing controls,
    // drag/drop behavior, and admin actions should be available.
    let is_admin = ADMIN_AUTH.read().is_admin();

    let mut selected_button_name = use_signal(|| None::<String>);
    let mut button_editor = use_signal(|| None::<ButtonEditorState>);
    let mut row_editor = use_signal(|| None::<RowEditorState>);
    let mut mutation_error = use_signal(|| None::<String>);
    let mut mutation_busy = use_signal(|| false);
    let mut tag_manager_open = use_signal(|| false);
    let mut tag_manager_draft = use_signal(Vec::<String>::new);
    let mut new_tag = use_signal(String::new);
    let mut dragging_index = use_signal(|| None::<usize>);
    let mut dragging_row_name = use_signal(|| None::<String>);
    let mut dragging_button_index = use_signal(|| None::<usize>);
    let mut dragging_button_name = use_signal(|| None::<String>);
    let mut reorder_confirmation = use_signal(|| None::<ReorderConfirmation>);
    let mut status_message = use_signal(|| None::<String>);

    // Auto-sort is a system-wide server setting. The DashboardPayload already
    // contains it, so SSR can render the correct order immediately on F5.
    let payload_autosort = payload.autosort;
    let mut autosort_enabled = use_signal(|| payload_autosort);

    // Keep the local UI switch synchronized with the server payload when
    // login/logout or another dashboard refresh changes the system setting.
    use_effect(move || {
        autosort_enabled.set(payload_autosort);
    });

    // Make a sorted copy for rendering. We never change the actual server
    // order here; turning autosort off immediately returns to that order.
    let mut buttons_to_display = payload.buttons.clone();
    if autosort_enabled() {
        buttons_to_display.sort_by_key(|button| button.name.to_lowercase());
    }

    // Clone payload for this effect. The component still needs the original
    // payload later when rendering the selected button.
    let payload_for_pending_button = payload.clone();

    use_effect(move || {
        let Some(win) = web_sys::window() else {
            return;
        };

        let location = win.location();
        let Ok(hash) = location.hash() else {
            return;
        };

        if let Some(query_string) = hash.strip_prefix("#__pending_button?") {
            if let Some(_encoded_name) = query_string.strip_prefix("name=") {
                #[cfg(target_arch = "wasm32")]
                {
                    let decoded = js_sys::decode_uri_component(_encoded_name)
                        .ok()
                        .and_then(|value| value.as_string());

                    if let Some(button_name) = decoded {
                        if payload_for_pending_button
                            .buttons
                            .iter()
                            .any(|button| button.name == button_name)
                        {
                            selected_button_name.set(Some(button_name));
                        }
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

    #[cfg(feature = "web")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        use_effect(move || {
            let Some(window) = web_sys::window() else {
                return;
            };

            let mut selected_button_name_for_handler = selected_button_name;
            let mut button_editor_for_handler = button_editor;
            let mut row_editor_for_handler = row_editor;
            let mut tag_manager_for_handler = tag_manager_open;
            let mut reorder_confirmation_for_handler = reorder_confirmation;

            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() == "Escape" {
                    event.prevent_default();

                    if reorder_confirmation_for_handler.peek().is_some() {
                        reorder_confirmation_for_handler.set(None);
                    } else if *tag_manager_for_handler.peek() {
                        tag_manager_for_handler.set(false);
                    } else if row_editor_for_handler.peek().is_some() {
                        row_editor_for_handler.set(None);
                    } else if button_editor_for_handler.peek().is_some() {
                        button_editor_for_handler.set(None);
                    } else if selected_button_name_for_handler.peek().is_some() {
                        selected_button_name_for_handler.set(None);
                    }
                }
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            let _ = window
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());

            let window_for_cleanup = window.clone();

            use dioxus::prelude::use_drop;

            use_drop(move || {
                let _ = window_for_cleanup.remove_event_listener_with_callback(
                    "keydown",
                    closure.as_ref().unchecked_ref(),
                );
            });
        });
    }

    // -------------------------------------------------------------------------
    // DIRECT ROW EDIT REQUESTS FROM SEARCH
    // -------------------------------------------------------------------------
    //
    // navbar.rs does not own the row editor, so it places a (button, row) pair
    // into this global request signal.  When that request changes, the dashboard
    // finds the matching row and opens the same RowEditorModal used by the
    // normal pencil button.
    // -------------------------------------------------------------------------
    let payload_for_edit_request = payload.clone();

    use_effect(move || {
        let Some((requested_button_name, requested_row_name)) = EDIT_ROW_REQUEST() else {
            return;
        };

        let Some(button) = payload_for_edit_request
            .buttons
            .iter()
            .find(|button| button.name == requested_button_name)
        else {
            *EDIT_ROW_REQUEST.write() = None;
            return;
        };

        let Some(row) = button
            .rows
            .iter()
            .find(|row| row.name() == requested_row_name)
        else {
            *EDIT_ROW_REQUEST.write() = None;
            return;
        };

        mutation_error.set(None);
        row_editor.set(Some(RowEditorState {
            button_name: requested_button_name,
            original_name: Some(requested_row_name),
            draft: RowDraft::from_row(row),
        }));

        // Consume the request so the editor does not reopen on every unrelated
        // signal change.
        *EDIT_ROW_REQUEST.write() = None;
    });

    let selected_button = selected_button_name().and_then(|name| {
        payload
            .buttons
            .iter()
            .find(|button| button.name == name)
            .cloned()
    });

    // Clone the tag list for the Tags button closure.
    // The `move` closure owns this clone instead of moving `payload`.
    let available_tags_for_manager = payload.available_tags.clone();

    // Count how many URL rows currently use each managed tag.  This is derived
    // from the same payload already displayed by the dashboard, so the count
    // stays simple and immediately reflects the latest refresh.
    let tag_usage_counts = calculate_tag_usage_counts(&payload);

    rsx! {
        div { class: "container mx-auto px-4 mt-6",
            if is_admin {
                // -----------------------------------------------------------------
                // ADMIN USAGE
                // -----------------------------------------------------------------
                // Each control gets its own small card.  The button and its
                // explanation stay together, which makes the admin panel much
                // easier to scan than a row of unexplained buttons.
                //
                // The grid changes from five columns on wide screens to fewer
                // columns on smaller screens, so the same information remains
                // readable on a phone-sized browser.
                div {
                    class: "mb-4 overflow-hidden rounded-xl border border-blue-200 bg-gradient-to-br from-blue-50 via-white to-indigo-50 shadow-sm",

                    div {
                        class: "border-b border-blue-100 bg-white/70 px-4 py-4 sm:px-5",

                        div {
                            class: "flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between",

                            div {
                                class: "min-w-0",
                                h2 {
                                    class: "text-lg font-semibold tracking-tight text-blue-950",
                                    "Admin Usage"
                                }
                                p {
                                    class: "mt-1 max-w-3xl text-sm leading-6 text-blue-800",
                                    "You are in edit mode. Use the five controls below to manage the dashboard. Changes are saved immediately. Export ZIP creates a server-side backup of the JSON files and dashboard history."
                                }
                            }

                            button {
                                class: "shrink-0 rounded-md border border-blue-300 bg-white px-3 py-2 text-xs font-semibold text-blue-800 shadow-sm transition hover:bg-blue-50 disabled:cursor-not-allowed disabled:opacity-60",
                                title: "Download a ZIP containing the dashboard JSON files and dashboard_history folder.",
                                disabled: mutation_busy(),
                                onclick: move |_| async move {
                                    mutation_error.set(None);
                                    status_message.set(None);
                                    mutation_busy.set(true);

                                    match export_dashboard_zip_server(ADMIN_AUTH()).await {
                                        Ok((bytes, filename)) => {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                download_zip_in_browser(bytes, filename);
                                                status_message.set(Some(
                                                    "Dashboard export ZIP downloaded.".to_string(),
                                                ));
                                            }

                                            #[cfg(not(target_arch = "wasm32"))]
                                            {
                                                let _ = (bytes, filename);
                                                status_message.set(Some(
                                                    "Dashboard export created.".to_string(),
                                                ));
                                            }
                                        }
                                        Err(error) => mutation_error.set(Some(error.to_string())),
                                    }

                                    mutation_busy.set(false);
                                },
                                "⇩ Export ZIP"
                            }
                        }
                    }

                    div {
                        class: "grid grid-cols-1 gap-3 p-3 sm:grid-cols-2 lg:grid-cols-5",

                        // ---------------------------------------------------------
                        // +Button
                        // ---------------------------------------------------------
                        div {
                            class: "flex h-full flex-col rounded-lg border border-blue-100 bg-white p-3 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md",
                            button {
                                class: "w-full rounded-md bg-blue-600 px-3 py-2 text-sm font-semibold text-white shadow-sm transition hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-60",
                                title: "Create a new main button and its first URL row.",
                                disabled: mutation_busy(),
                                onclick: move |_| {
                                    mutation_error.set(None);
                                    button_editor.set(Some(ButtonEditorState {
                                        mode: ButtonEditorMode::Create,
                                        draft: ButtonDraft::default(),
                                    }));
                                },
                                "+Button"
                            }
                            p {
                                class: "mt-2 flex-1 text-xs leading-5 text-gray-600",
                                "Create a new main dashboard button. It starts with its first URL row."
                            }
                        }

                        // ---------------------------------------------------------
                        // +Tags
                        // ---------------------------------------------------------
                        div {
                            class: "flex h-full flex-col rounded-lg border border-indigo-100 bg-white p-3 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md",
                            button {
                                class: "w-full rounded-md bg-indigo-600 px-3 py-2 text-sm font-semibold text-white shadow-sm transition hover:bg-indigo-700 disabled:cursor-not-allowed disabled:opacity-60",
                                title: "Open the global tag manager to add, remove, and save managed tags.",
                                disabled: mutation_busy(),
                                onclick: move |_| {
                                    mutation_error.set(None);
                                    tag_manager_draft.set(available_tags_for_manager.clone());
                                    new_tag.set(String::new());
                                    tag_manager_open.set(true);
                                },
                                "+Tags"
                            }
                            p {
                                class: "mt-2 flex-1 text-xs leading-5 text-gray-600",
                                "Manage the global tags used by URL rows. Add, remove, and save tags here."
                            }
                        }

                        // ---------------------------------------------------------
                        // +JSON Check
                        // ---------------------------------------------------------
                        div {
                            class: "flex h-full flex-col rounded-lg border border-slate-200 bg-white p-3 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md",
                            button {
                                class: "w-full rounded-md bg-slate-600 px-3 py-2 text-sm font-semibold text-white shadow-sm transition hover:bg-slate-700 disabled:cursor-not-allowed disabled:opacity-60",
                                title: "Validate data_buttons.v3.json with the dashboard parser.",
                                disabled: mutation_busy(),
                                onclick: move |_| async move {
                                    mutation_error.set(None);
                                    status_message.set(None);
                                    mutation_busy.set(true);

                                    match check_json_server(ADMIN_AUTH()).await {
                                        Ok(()) => {
                                            status_message.set(Some(
                                                "Dashboard JSON is valid.".to_string()
                                            ));
                                        }
                                        Err(error) => {
                                            mutation_error.set(Some(error.to_string()));
                                        }
                                    }

                                    mutation_busy.set(false);
                                },
                                "+JSON Check"
                            }
                            p {
                                class: "mt-2 flex-1 text-xs leading-5 text-gray-600",
                                "Check the dashboard JSON using the same parser used when the dashboard loads."
                            }
                        }

                        // ---------------------------------------------------------
                        // +History
                        // ---------------------------------------------------------
                        div {
                            class: "flex h-full flex-col rounded-lg border border-purple-100 bg-white p-3 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md",
                            button {
                                class: "w-full rounded-md bg-purple-600 px-3 py-2 text-sm font-semibold text-white shadow-sm transition hover:bg-purple-700 disabled:cursor-not-allowed disabled:opacity-60",
                                title: "Open the dashboard history page.",
                                disabled: mutation_busy(),
                                onclick: move |_| {
                                    mutation_error.set(None);
                                    status_message.set(None);
                                    *HISTORY_PAGE.write() = true;
                                },
                                "+History"
                            }
                            p {
                                class: "mt-2 flex-1 text-xs leading-5 text-gray-600",
                                "View saved JSON snapshots and use the history tools to inspect or restore earlier versions."
                            }
                        }

                        // ---------------------------------------------------------
                        // +Autosort
                        // ---------------------------------------------------------
                        div {
                            class: "flex h-full flex-col rounded-lg border border-emerald-100 bg-white p-3 shadow-sm transition hover:-translate-y-0.5 hover:shadow-md",
                            button {
                                class: if autosort_enabled() {
                                    "w-full rounded-md bg-emerald-600 px-3 py-2 text-sm font-semibold text-white shadow-sm transition hover:bg-emerald-700 disabled:cursor-not-allowed disabled:opacity-60"
                                } else {
                                    "w-full rounded-md bg-gray-600 px-3 py-2 text-sm font-semibold text-white shadow-sm transition hover:bg-gray-700 disabled:cursor-not-allowed disabled:opacity-60"
                                },
                                title: "Toggle system-wide alphabetical sorting of the main button cards.",
                                disabled: mutation_busy(),
                                onclick: move |_| async move {
                                    let enabled = !autosort_enabled();
                                    autosort_enabled.set(enabled);

                                    match set_autosort_server(ADMIN_AUTH(), enabled).await {
                                        Ok(()) => {
                                            status_message.set(Some(format!(
                                                "Autosort is now {}.",
                                                if enabled { "ON" } else { "OFF" }
                                            )));
                                            *DASHBOARD_REFRESH_KEY.write() += 1;
                                        }
                                        Err(error) => {
                                            autosort_enabled.set(!enabled);
                                            mutation_error.set(Some(error.to_string()));
                                        }
                                    }
                                },
                                if autosort_enabled() { "+Autosort ON" } else { "+Autosort OFF" }
                            }
                            p {
                                class: "mt-2 flex-1 text-xs leading-5 text-gray-600",
                                "Turn alphabetical sorting of the main button cards on or off. This is system-wide and survives F5."
                            }
                        }
                    }
                }
            }

            if let Some(message) = status_message() {
                div {
                    class: "mb-4 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-800",
                    "{message}"
                }
            }

            if let Some(message) = mutation_error() {
                div {
                    class: "mb-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700",
                    "{message}"
                }
            }

            div { class: "grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-5",
                for (index, button) in buttons_to_display.iter().enumerate() {
                    {
                        let button_name = button.name.clone();
                        // Each `move` closure owns its captured values.
                        // Keep separate clones for drag-start and drop so the
                        // original `button_name` remains available for the
                        // link below.
                        let drag_button_name = button_name.clone();
                        let drop_button_name = button_name.clone();
                        let button_for_editor = button.clone();

                        rsx! {
                            div {
                                class: if is_admin && !autosort_enabled() {
                                    if dragging_button_index() == Some(index) {
                                        "group relative cursor-grabbing opacity-70 transition"
                                    } else {
                                        "group relative cursor-grab transition"
                                    }
                                } else {
                                    "group relative"
                                },
                                draggable: is_admin && !autosort_enabled(),
                                ondragstart: move |_| {
                                    if is_admin && !autosort_enabled() {
                                        dragging_button_index.set(Some(index));
                                        dragging_button_name.set(Some(drag_button_name.clone()));
                                    }
                                },
                                ondragend: move |_| {
                                    dragging_button_index.set(None);
                                    dragging_button_name.set(None);
                                },
                                ondragover: move |event| {
                                    if is_admin && !autosort_enabled() {
                                        event.prevent_default();
                                    }
                                },
                                ondrop: move |event| {
                                        if !is_admin || autosort_enabled() {
                                            return;
                                        }

                                        event.prevent_default();

                                        let Some(from_index) = dragging_button_index() else {
                                            return;
                                        };

                                        dragging_button_index.set(None);

                                        let source_name = dragging_button_name()
                                            .unwrap_or_else(|| "this button".to_string());
                                        dragging_button_name.set(None);

                                        if from_index == index || mutation_busy() {
                                            return;
                                        }

                                        // Save the requested move and show the custom
                                        // Dioxus confirmation modal. The server is not
                                        // called until the administrator clicks Move.
                                        reorder_confirmation.set(Some(ReorderConfirmation::Button {
                                            from_index,
                                            to_index: index,
                                            source_name,
                                            target_name: drop_button_name.clone(),
                                        }));
                            },

                                if is_admin {
                                    button {
                                        class: "absolute right-2 top-2 z-10 rounded-full bg-white/90 px-2 py-1 text-xs text-gray-700 shadow transition hover:bg-white",
                                        onclick: move |event| {
                                            event.stop_propagation();
                                            event.prevent_default();
                                            mutation_error.set(None);
                                            button_editor.set(Some(ButtonEditorState {
                                                mode: ButtonEditorMode::Edit {
                                                    original_name: button_for_editor.name.clone(),
                                                },
                                                draft: ButtonDraft {
                                                    name: button_for_editor.name.clone(),
                                                    ..ButtonDraft::default()
                                                },
                                            }));
                                        },
                                        "✎"
                                    }
                                }

                                if is_admin && !autosort_enabled() {
                                    div {
                                        class: "absolute right-2 top-10 z-10 rounded-full bg-blue-100 px-2 py-1 text-sm font-medium text-blue-800 shadow",
                                        title: "Drag this main button to another position.",
                                        "⠿"
                                    }
                                }

                                a {
                                    href: {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let encoded = js_sys::encode_uri_component(&button_name);
                                            format!("#__pending_button?name={}", encoded)
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        {
                                            format!("#__pending_button?name={button_name}")
                                        }
                                    },
                                    class: if is_admin {
                                        "block w-full rounded-md bg-gray-300 px-4 py-6 pr-14 text-center font-medium text-gray-800 shadow-sm transition hover:scale-105 hover:bg-gray-400 hover:shadow-md active:scale-95 truncate overflow-hidden whitespace-nowrap"
                                    } else {
                                        "block w-full rounded-md bg-gray-300 px-4 py-3 text-center font-medium text-gray-800 shadow-sm transition hover:scale-105 hover:bg-gray-400 hover:shadow-md active:scale-95 truncate overflow-hidden whitespace-nowrap"
                                    },
                                    onclick: {
                                        let button_name = button_name.clone();
                                        move |event| {
                                            event.prevent_default();
                                            selected_button_name.set(Some(button_name.clone()));
                                        }
                                    },
                                    "{button_name}"
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(button) = selected_button {
            {
                let button_name = button.name.clone();
                rsx! {
                    div {
                        class: "fixed inset-0 z-40 flex items-center justify-center bg-black/50 p-4",
                        onclick: move |_| {
                            selected_button_name.set(None);
                        },

                        div {
                            class: "relative max-h-[80vh] w-full max-w-2xl overflow-y-auto rounded-lg bg-white p-6 shadow-xl",
                            onclick: move |event| {
                                event.stop_propagation();
                            },

                            button {
                                class: "absolute right-3 top-2 text-2xl text-red-500 transition hover:text-red-700",
                                onclick: move |_| {
                                    selected_button_name.set(None);
                                },
                                "✕"
                            }

                            div { class: "mb-4 flex items-center gap-3 pr-8",
                                h1 { class: "truncate text-2xl font-bold text-gray-900", "{button.name}" }

                                if is_admin {
                                    // The selected-button modal already knows
                                    // exactly which button is open.  Reuse that
                                    // button data to launch the same
                                    // ButtonEditorModal used by the pencil on
                                    // the main dashboard card.
                                    button {
                                        class: "rounded-md border border-blue-300 bg-blue-50 px-3 py-1 text-sm font-medium text-blue-700 transition hover:bg-blue-100",
                                        onclick: {
                                            let button = button.clone();
                                            move |_| {
                                                mutation_error.set(None);
                                                button_editor.set(Some(ButtonEditorState {
                                                    mode: ButtonEditorMode::Edit {
                                                        original_name: button.name.clone(),
                                                    },
                                                    draft: ButtonDraft {
                                                        name: button.name.clone(),
                                                        ..ButtonDraft::default()
                                                    },
                                                }));
                                            }
                                        },
                                        "Edit Button"
                                    }

                                    button {
                                        class: "rounded-md border border-gray-300 px-3 py-1 text-sm text-gray-700 transition hover:bg-gray-100",
                                        onclick: {
                                            let button = button.clone();
                                            move |_| {
                                                mutation_error.set(None);
                                                row_editor.set(Some(RowEditorState {
                                                    button_name: button.name.clone(),
                                                    original_name: None,
                                                    draft: RowDraft::empty_link(),
                                                }));
                                            }
                                        },
                                        "+ Row"
                                    }
                                }
                            }

                            if is_admin {
                                div {
                                    class: "mb-4 rounded-md border border-dashed border-blue-300 bg-blue-50 px-3 py-2 text-xs text-blue-900",
                                    "Admin edit mode: use ✎ to edit a row. Grab the ⠿ handle or the row itself and drag it to reorder. The row will highlight while dragging."
                                }
                            }

                            div { class: "space-y-3",
                                for (index, row) in button.rows.iter().enumerate() {
                                    {
                                        let row_for_editor = row.clone();
                                        let button_name = button_name.clone();

                                        match row {
                                            ButtonRowView::Divider { .. } => {
                                                // These small owned strings let the drag/drop
                                                // closures move their data without consuming the
                                                // `row_for_editor` value also used by the pencil.
                                                let row_editor_button_name = button_name.clone();
                                                let drag_row_name = row_for_editor.name().to_string();
                                                let target_row_name = drag_row_name.clone();

                                                rsx! {
                                                div {
                                                    class: if is_admin {
                                                        if dragging_index() == Some(index) {
                                                            "group relative rounded-md border-2 border-blue-400 bg-blue-50 px-4 py-3 shadow-md opacity-70 cursor-grabbing transition"
                                                        } else {
                                                            "group relative rounded-md border border-dashed border-blue-300 bg-blue-50/30 px-4 py-3 cursor-grab transition hover:border-blue-400 hover:bg-blue-50"
                                                        }
                                                    } else {
                                                        "group relative rounded-md border border-gray-200 px-4 py-3 transition"
                                                    },
                                                    draggable: is_admin,
                                                    ondragstart: move |_| {
                                                        dragging_index.set(Some(index));
                                                        dragging_row_name.set(Some(drag_row_name.clone()));
                                                    },
                                                    ondragend: move |_| {
                                                        dragging_index.set(None);
                                                        dragging_row_name.set(None);
                                                    },
                                                    ondragover: move |event| {
                                                        event.prevent_default();
                                                    },
                                                    ondrop: {
                                                        let button_name = button_name.clone();
                                                        move |event| {
                                                            event.prevent_default();
                                                            let Some(from_index) = dragging_index() else { return; };
                                                            dragging_index.set(None);

                                                            let source_name = dragging_row_name()
                                                                .unwrap_or_else(|| "this row".to_string());
                                                            dragging_row_name.set(None);

                                                            if from_index == index || mutation_busy() { return; }

                                                            // Save the requested row move and let the shared
                                                            // custom confirmation modal handle it.
                                                            reorder_confirmation.set(Some(ReorderConfirmation::Row {
                                                                button_name: button_name.clone(),
                                                                from_index,
                                                                to_index: index,
                                                                source_name,
                                                                target_name: target_row_name.clone(),
                                                            }));
                                                        }
                                                    },
                                                    div {
                                                        class: "my-1 flex items-center gap-3 pr-12",
                                                        span {
                                                            class: "h-1 flex-1 rounded-full bg-gray-300",
                                                        }
                                                        span {
                                                            class: "text-xs font-semibold uppercase tracking-wider text-gray-500",
                                                            "{row_for_editor.name()}"
                                                        }
                                                        span {
                                                            class: "h-1 flex-1 rounded-full bg-gray-300",
                                                        }
                                                    }

                                                    if is_admin {
                                                        div {
                                                            class: "absolute bottom-2 right-3 rounded-md bg-blue-100 px-2 py-1 text-sm font-medium text-blue-800",
                                                            title: "Drag divider to reorder",
                                                            "⠿"
                                                        }

                                                        button {
                                                            class: "absolute right-3 top-2 rounded-full bg-white px-2 py-1 text-xs text-gray-700 shadow transition hover:bg-gray-50",
                                                            onclick: move |_| {
                                                                mutation_error.set(None);
                                                                row_editor.set(Some(RowEditorState {
                                                                    button_name: row_editor_button_name.clone(),
                                                                    original_name: Some(row_for_editor.name().to_string()),
                                                                    draft: RowDraft::from_row(&row_for_editor),
                                                                }));
                                                            },
                                                            "✎"
                                                        }
                                                    }
                                                }
                                                }
                                            },
                                            ButtonRowView::Link {
                                                name,
                                                url,
                                                hashtags,
                                                comments,
                                                hashtag_tokens,
                                                secret_text,
                                                has_secret,
                                            } => {
                                                // These small owned strings let the drag/drop
                                                // closures move their data without consuming the
                                                // `row_for_editor` value also used by the pencil.
                                                let row_editor_button_name = button_name.clone();
                                                let drag_row_name = row_for_editor.name().to_string();
                                                let target_row_name = drag_row_name.clone();

                                                rsx! {
                                                div {
                                                    class: if is_admin {
                                                        if dragging_index() == Some(index) {
                                                            "group relative rounded-md border-2 border-blue-400 bg-blue-50 p-4 shadow-md opacity-70 cursor-grabbing transition"
                                                        } else {
                                                            "group relative rounded-md border border-dashed border-blue-300 bg-blue-50/30 p-4 cursor-grab transition hover:border-blue-400 hover:bg-blue-50"
                                                        }
                                                    } else {
                                                        "group relative rounded-md border border-gray-200 p-4 transition hover:border-gray-300 hover:bg-gray-50"
                                                    },
                                                    draggable: is_admin,
                                                    ondragstart: move |_| {
                                                        dragging_index.set(Some(index));
                                                        dragging_row_name.set(Some(drag_row_name.clone()));
                                                    },
                                                    ondragend: move |_| {
                                                        dragging_index.set(None);
                                                        dragging_row_name.set(None);
                                                    },
                                                    ondragover: move |event| {
                                                        event.prevent_default();
                                                    },
                                                    ondrop: {
                                                        let button_name = button_name.clone();
                                                        move |event| {
                                                            event.prevent_default();
                                                            let Some(from_index) = dragging_index() else { return; };
                                                            dragging_index.set(None);

                                                            let source_name = dragging_row_name()
                                                                .unwrap_or_else(|| "this row".to_string());
                                                            dragging_row_name.set(None);

                                                            if from_index == index || mutation_busy() { return; }

                                                            // Save the requested row move and let the shared
                                                            // custom confirmation modal handle it.
                                                            reorder_confirmation.set(Some(ReorderConfirmation::Row {
                                                                button_name: button_name.clone(),
                                                                from_index,
                                                                to_index: index,
                                                                source_name,
                                                                target_name: target_row_name.clone(),
                                                            }));
                                                        }
                                                    },

                                                    if is_admin {
                                                        div {
                                                            class: "absolute bottom-3 right-3 flex items-center gap-1 rounded-md bg-blue-100 px-2 py-1 text-[11px] font-medium text-blue-800",
                                                            span { class: "text-base leading-none", "⠿" }
                                                        }

                                                        button {
                                                            class: "absolute right-3 top-3 rounded-full bg-white px-2 py-1 text-xs text-gray-700 shadow transition hover:bg-gray-50",
                                                            onclick: move |_| {
                                                                mutation_error.set(None);
                                                                row_editor.set(Some(RowEditorState {
                                                                    button_name: row_editor_button_name.clone(),
                                                                    original_name: Some(row_for_editor.name().to_string()),
                                                                    draft: RowDraft::from_row(&row_for_editor),
                                                                }));
                                                            },
                                                            "✎"
                                                        }
                                                    }

                                                    a {
                                                        class: "block rounded-md pr-10 hover:text-blue-700",
                                                        href: url.clone(),
                                                        target: "_blank",

                                                        div { class: "font-medium text-gray-900 break-words", "{name}" }
                                                        div { class: "mt-1 text-sm text-blue-700 break-all", "{url}" }

                                                         if !comments.trim().is_empty() {
                                                             div {
                                                                 class: "mt-1 truncate text-xs text-gray-500",
                                                                 title: comments.clone(),
                                                                 "{comments}"
                                                             }
                                                         }

                                                        if !hashtags.is_empty() {
                                                            div { class: "mt-3 flex flex-wrap gap-2",
                                                                for tag in hashtag_tokens.iter() {
                                                                    span { class: "rounded-full bg-blue-100 px-2 py-1 text-xs font-medium text-blue-700", "{tag}" }
                                                                }
                                                            }
                                                        }

                                                        if *has_secret {
                                                            div { class: "mt-3 text-sm text-gray-700",
                                                                if let Some(secret_text) = secret_text {
                                                                    span { class: "rounded-md bg-amber-100 px-2 py-1 font-medium text-amber-900 break-all", "{secret_text}" }
                                                                } else {
                                                                    span { class: "inline-flex items-center gap-2 rounded-md bg-amber-50 px-2 py-1 text-amber-700",
                                                                        span { "🔑" }
                                                                        span { "Secret hidden" }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if tag_manager_open() {
            TagManagerModal {
                tags: tag_manager_draft(),
                tag_usage_counts: tag_usage_counts.clone(),
                new_tag: new_tag(),
                busy: mutation_busy(),
                error_message: mutation_error(),
                on_close: move |_| {
                    tag_manager_open.set(false);
                    mutation_error.set(None);
                },
                on_new_tag_input: move |value: String| new_tag.set(value),
                on_add: move |_| {
                    let clean = normalize_tag_for_ui(&new_tag());
                    if clean.chars().count() >= 4 {
                        let mut tags = tag_manager_draft();
                        if !tags.iter().any(|tag| tag.eq_ignore_ascii_case(&clean)) {
                            tags.push(clean);
                            tags.sort_by_key(|tag| tag.to_lowercase());
                            tag_manager_draft.set(tags);
                            new_tag.set(String::new());
                            mutation_error.set(None);
                        } else {
                            mutation_error.set(Some("That tag is already in the list.".to_string()));
                        }
                    } else {
                        mutation_error.set(Some("Tags must contain at least 4 characters.".to_string()));
                    }
                },
                on_remove: move |tag: String| {
                    let mut tags = tag_manager_draft();
                    tags.retain(|existing| existing != &tag);
                    tag_manager_draft.set(tags);
                },
                on_save: move |_| async move {
                    mutation_busy.set(true);
                    mutation_error.set(None);
                    let result = save_tags_server(ADMIN_AUTH(), tag_manager_draft()).await;
                    mutation_busy.set(false);
                    match result {
                        Ok(()) => {
                            tag_manager_open.set(false);
                            *DASHBOARD_REFRESH_KEY.write() += 1;
                        }
                        Err(error) => mutation_error.set(Some(error.to_string())),
                    }
                },
            }
        }

        // ---------------------------------------------------------------------
        // DRAG/DROP CONFIRMATION MODAL
        // ---------------------------------------------------------------------
        // This replaces the browser-native confirm popup. Escape is also
        // handled by the existing global keydown handler above.
        if let Some(confirmation) = reorder_confirmation() {
            ReorderConfirmationModal {
                confirmation: confirmation.clone(),
                busy: mutation_busy(),
                on_cancel: move |_| {
                    reorder_confirmation.set(None);
                },
                on_confirm: move |_| {
                    let Some(confirmation) = reorder_confirmation() else {
                        return;
                    };

                    reorder_confirmation.set(None);
                    mutation_busy.set(true);
                    mutation_error.set(None);
                    status_message.set(None);

                    spawn(async move {
                        let result = match confirmation {
                            ReorderConfirmation::Button {
                                from_index,
                                to_index,
                                ..
                            } => reorder_buttons_server(
                                ADMIN_AUTH(),
                                from_index,
                                to_index,
                            )
                            .await,
                            ReorderConfirmation::Row {
                                button_name,
                                from_index,
                                to_index,
                                ..
                            } => reorder_rows_server(
                                ADMIN_AUTH(),
                                button_name,
                                from_index,
                                to_index,
                            )
                            .await,
                        };

                        mutation_busy.set(false);

                        match result {
                            Ok(()) => *DASHBOARD_REFRESH_KEY.write() += 1,
                            Err(error) => mutation_error.set(Some(error.to_string())),
                        }
                    });
                },
            }
        }

        if let Some(editor) = button_editor() {
            ButtonEditorModal {
                editor: editor.clone(),
                busy: mutation_busy(),
                error_message: mutation_error(),
                on_close: move |_| {
                    button_editor.set(None);
                    mutation_error.set(None);
                },
                on_name_input: move |value: String| {
                    if let Some(mut current) = button_editor() {
                        current.draft.name = value;
                        button_editor.set(Some(current));
                    }
                },
                on_first_row_name_input: move |value: String| {
                    if let Some(mut current) = button_editor() {
                        current.draft.first_row_name = value;
                        button_editor.set(Some(current));
                    }
                },
                on_first_row_url_input: move |value: String| {
                    if let Some(mut current) = button_editor() {
                        current.draft.first_row_url = value;
                        button_editor.set(Some(current));
                    }
                },
                available_tags: payload.available_tags.clone(),
                on_add_first_row_tag: move |tag: String| {
                    if let Some(mut current) = button_editor() {
                        current.draft.first_row_hashtags = add_tag_to_text(&current.draft.first_row_hashtags, &tag);
                        button_editor.set(Some(current));
                    }
                },
                on_first_row_hashtags_input: move |value: String| {
                    if let Some(mut current) = button_editor() {
                        current.draft.first_row_hashtags = value;
                        button_editor.set(Some(current));
                    }
                },
                on_first_row_secrets_input: move |value: String| {
                    if let Some(mut current) = button_editor() {
                        current.draft.first_row_secrets = value;
                        button_editor.set(Some(current));
                    }
                },
                on_save: move |_| async move {
                    let Some(editor) = button_editor() else {
                        return;
                    };

                    mutation_busy.set(true);
                    mutation_error.set(None);

                    let credentials = ADMIN_AUTH();
                    let result = match editor.mode {
                        ButtonEditorMode::Create => {
                            create_button_server(
                                credentials,
                                ButtonCreateInput {
                                    name: editor.draft.name.clone(),
                                    first_row: ButtonRowInput {
                                        original_name: None,
                                        name: editor.draft.first_row_name.clone(),
                                        kind: RowKind::Link,
                                        url: editor.draft.first_row_url.clone(),
                                        hashtags: editor.draft.first_row_hashtags.clone(),
                                        comments: String::new(),
                                        secrets: editor.draft.first_row_secrets.clone(),
                                    },
                                },
                            )
                            .await
                        }
                        ButtonEditorMode::Edit { original_name } => {
                            rename_button_server(
                                credentials,
                                ButtonRenameInput {
                                    original_name,
                                    name: editor.draft.name.clone(),
                                },
                            )
                            .await
                        }
                    };

                    mutation_busy.set(false);

                    match result {
                        Ok(()) => {
                            // After renaming a button, open its URL-list modal
                            // using the new name. The refresh below supplies the
                            // updated button and all of its rows.
                            let saved_button_name = editor.draft.name.clone();

                            button_editor.set(None);
                            mutation_error.set(None);
                            selected_button_name.set(Some(saved_button_name));
                            *DASHBOARD_REFRESH_KEY.write() += 1;
                        }
                        Err(error) => {
                            mutation_error.set(Some(error.to_string()));
                        }
                    }
                },
                on_clone: move |_| async move {
                    let Some(editor) = button_editor() else {
                        return;
                    };

                    let ButtonEditorMode::Edit { original_name } = editor.mode else {
                        return;
                    };

                    mutation_busy.set(true);
                    mutation_error.set(None);

                    let result = clone_button_server(ADMIN_AUTH(), original_name).await;

                    mutation_busy.set(false);

                    match result {
                        Ok(cloned_name) => {
                            // The cloned button is inserted immediately after the
                            // source. Select that new name so the URL-list modal
                            // opens on the clone after the dashboard refresh.
                            button_editor.set(None);
                            mutation_error.set(None);
                            status_message.set(Some(format!(
                                "Cloned button as '{cloned_name}'."
                            )));
                            selected_button_name.set(Some(cloned_name));
                            *DASHBOARD_REFRESH_KEY.write() += 1;
                        }
                        Err(error) => {
                            mutation_error.set(Some(error.to_string()));
                        }
                    }
                },
                on_delete: move |_| async move {
                    let Some(editor) = button_editor() else {
                        return;
                    };

                    let ButtonEditorMode::Edit { original_name } = editor.mode else {
                        return;
                    };

                    mutation_busy.set(true);
                    mutation_error.set(None);

                    let result = delete_button_server(ADMIN_AUTH(), original_name.clone()).await;

                    mutation_busy.set(false);

                    match result {
                        Ok(()) => {
                            if selected_button_name() == Some(original_name.clone()) {
                                selected_button_name.set(None);
                            }
                            button_editor.set(None);
                            *DASHBOARD_REFRESH_KEY.write() += 1;
                        }
                        Err(error) => {
                            mutation_error.set(Some(error.to_string()));
                        }
                    }
                },
            }
        }

        if let Some(editor) = row_editor() {
            RowEditorModal {
                editor: editor.clone(),
                busy: mutation_busy(),
                error_message: mutation_error(),
                on_close: move |_| {
                    row_editor.set(None);
                    mutation_error.set(None);
                },
                on_kind_change: move |kind: RowKind| {
                    if let Some(mut current) = row_editor() {
                        current.draft.kind = kind;
                        row_editor.set(Some(current));
                    }
                },
                on_name_input: move |value: String| {
                    if let Some(mut current) = row_editor() {
                        current.draft.name = value;
                        row_editor.set(Some(current));
                    }
                },
                on_url_input: move |value: String| {
                    if let Some(mut current) = row_editor() {
                        current.draft.url = value;
                        row_editor.set(Some(current));
                    }
                },
                available_tags: payload.available_tags.clone(),
                on_add_tag: move |tag: String| {
                    if let Some(mut current) = row_editor() {
                        current.draft.hashtags = add_tag_to_text(&current.draft.hashtags, &tag);
                        row_editor.set(Some(current));
                    }
                },
                on_hashtags_input: move |value: String| {
                    if let Some(mut current) = row_editor() {
                        current.draft.hashtags = value;
                        row_editor.set(Some(current));
                    }
                },
                on_comments_input: move |value: String| {
                    if let Some(mut current) = row_editor() {
                        current.draft.comments = value;
                        row_editor.set(Some(current));
                    }
                },
                on_secrets_input: move |value: String| {
                    if let Some(mut current) = row_editor() {
                        current.draft.secrets = value;
                        row_editor.set(Some(current));
                    }
                },
                on_save: move |_| async move {
                    let Some(editor) = row_editor() else {
                        return;
                    };

                    mutation_busy.set(true);
                    mutation_error.set(None);

                    let result = save_row_server(
                        ADMIN_AUTH(),
                        editor.button_name.clone(),
                        ButtonRowInput {
                            original_name: editor.original_name.clone(),
                            name: editor.draft.name.clone(),
                            kind: editor.draft.kind.clone(),
                            url: editor.draft.url.clone(),
                            hashtags: editor.draft.hashtags.clone(),
                            comments: editor.draft.comments.clone(),
                            secrets: editor.draft.secrets.clone(),
                        },
                    )
                    .await;

                    mutation_busy.set(false);

                    match result {
                        Ok(()) => {
                            // Keep the button selected after saving a row from the
                            // search pencil.  The dashboard refresh below will load
                            // the new JSON, so the URL-list modal can show the
                            // freshly saved row name immediately after the refresh.
                            let edited_button_name = editor.button_name.clone();

                            row_editor.set(None);
                            mutation_error.set(None);
                            selected_button_name.set(Some(edited_button_name));
                            *DASHBOARD_REFRESH_KEY.write() += 1;
                        }
                        Err(error) => {
                            mutation_error.set(Some(error.to_string()));
                        }
                    }
                },
                on_clone: move |_| async move {
                    let Some(editor) = row_editor() else {
                        return;
                    };

                    let Some(original_name) = editor.original_name.clone() else {
                        return;
                    };

                    mutation_busy.set(true);
                    mutation_error.set(None);

                    let result = clone_row_server(
                        ADMIN_AUTH(),
                        editor.button_name.clone(),
                        original_name,
                    )
                    .await;

                    mutation_busy.set(false);

                    match result {
                        Ok(cloned_name) => {
                            // Return to the button's URL-list modal after cloning.
                            // The refresh loads the newly inserted -1/-2/... row,
                            // making the result immediately visible to the admin.
                            let cloned_button_name = editor.button_name.clone();

                            row_editor.set(None);
                            mutation_error.set(None);
                            status_message.set(Some(format!(
                                "Cloned row as '{cloned_name}'."
                            )));
                            selected_button_name.set(Some(cloned_button_name));
                            *DASHBOARD_REFRESH_KEY.write() += 1;
                        }
                        Err(error) => {
                            mutation_error.set(Some(error.to_string()));
                        }
                    }
                },
                on_delete: move |_| async move {
                    let Some(editor) = row_editor() else {
                        return;
                    };

                    let Some(original_name) = editor.original_name.clone() else {
                        return;
                    };

                    mutation_busy.set(true);
                    mutation_error.set(None);

                    let result =
                        delete_row_server(ADMIN_AUTH(), editor.button_name.clone(), original_name)
                            .await;

                    mutation_busy.set(false);

                    match result {
                        Ok(()) => {
                            row_editor.set(None);
                            *DASHBOARD_REFRESH_KEY.write() += 1;
                        }
                        Err(error) => {
                            mutation_error.set(Some(error.to_string()));
                        }
                    }
                },
            }
        }
    }
}

// -----------------------------------------------------------------------------
// TAG HELPERS
// -----------------------------------------------------------------------------

fn normalize_tag_for_ui(value: &str) -> String {
    let clean = value.trim().trim_start_matches('#');
    if clean.is_empty() {
        String::new()
    } else {
        format!("#{clean}")
    }
}

fn add_tag_to_text(current: &str, tag: &str) -> String {
    let tag = normalize_tag_for_ui(tag);
    if tag.is_empty() {
        return current.trim().to_string();
    }

    let exists = current
        .split_whitespace()
        .any(|existing| normalize_tag_for_ui(existing).eq_ignore_ascii_case(&tag));

    if exists {
        current.trim().to_string()
    } else if current.trim().is_empty() {
        tag
    } else {
        format!("{} {}", current.trim(), tag)
    }
}

fn calculate_tag_usage_counts(payload: &DashboardPayload) -> Vec<(String, usize)> {
    payload
        .available_tags
        .iter()
        .map(|managed_tag| {
            let count = payload
                .buttons
                .iter()
                .flat_map(|button| button.rows.iter())
                .filter_map(|row| match row {
                    ButtonRowView::Link { hashtag_tokens, .. } => Some(hashtag_tokens),
                    ButtonRowView::Divider { .. } => None,
                })
                .flat_map(|tags| tags.iter())
                .filter(|tag| tag.eq_ignore_ascii_case(managed_tag))
                .count();

            (managed_tag.clone(), count)
        })
        .collect()
}

fn tag_usage_count(counts: &[(String, usize)], tag: &str) -> usize {
    counts
        .iter()
        .find(|(managed_tag, _)| managed_tag.eq_ignore_ascii_case(tag))
        .map(|(_, count)| *count)
        .unwrap_or(0)
}

// -----------------------------------------------------------------------------
// TAG MANAGER MODAL
// -----------------------------------------------------------------------------

#[component]
fn TagManagerModal(
    tags: Vec<String>,
    tag_usage_counts: Vec<(String, usize)>,
    new_tag: String,
    busy: bool,
    error_message: Option<String>,
    on_close: EventHandler<()>,
    on_new_tag_input: EventHandler<String>,
    on_add: EventHandler<()>,
    on_remove: EventHandler<String>,
    on_save: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4",
            // Edit modals must not close from a backdrop click. Escape is the
            // intentional cancel path so typed data is not lost accidentally.
            div {
                class: "w-full max-w-xl rounded-lg bg-white p-6 shadow-xl",
                onclick: move |event| event.stop_propagation(),
                h2 { class: "text-xl font-semibold text-gray-900", "Manage Tags" }
                p { class: "mt-2 text-sm text-gray-600", "Tags are stored in data_tags.json. The number beside each tag shows how many URL rows currently use it; used tags cannot be removed." }

                div { class: "mt-4 flex gap-2",
                    input {
                        class: "min-w-0 flex-1 rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                        placeholder: "New tag (minimum 4 characters)",
                        value: new_tag,
                        oninput: move |event| on_new_tag_input.call(event.value()),
                        onkeydown: move |event| {
                            if event.key() == Key::Enter { on_add.call(()); }
                        },
                    }
                    button {
                        r#type: "button",
                        class: "rounded-md bg-indigo-600 px-4 py-2 text-sm font-medium text-white hover:bg-indigo-700 disabled:opacity-60",
                        disabled: busy,
                        onclick: move |_| on_add.call(()),
                        "Add"
                    }
                }

                div { class: "mt-4 max-h-72 overflow-y-auto rounded-md border border-gray-200 p-3",
                    if tags.is_empty() {
                        p { class: "text-sm text-gray-500", "No tags yet." }
                    } else {
                        div { class: "flex flex-wrap gap-2",
                            for tag in tags.iter() {
                                {
                                    let count = tag_usage_count(&tag_usage_counts, tag);

                                    rsx! {
                                        div {
                                            class: if count > 0 {
                                                "flex items-center gap-1 rounded-full bg-blue-100 px-2 py-1 text-xs font-medium text-blue-700"
                                            } else {
                                                "flex items-center gap-1 rounded-full bg-gray-100 px-2 py-1 text-xs font-medium text-gray-600"
                                            },
                                            span { "{tag}" }
                                            span {
                                                class: if count > 0 {
                                                    "rounded-full bg-emerald-100 px-1.5 py-0.5 text-[10px] font-semibold text-emerald-700"
                                                } else {
                                                    "rounded-full bg-gray-200 px-1.5 py-0.5 text-[10px] font-semibold text-gray-500"
                                                },
                                                "{count}"
                                            }
                                            button {
                                                r#type: "button",
                                                class: "ml-1 text-blue-700 hover:text-red-600",
                                                disabled: busy,
                                                title: if count > 0 { "This tag is currently used by URL rows and cannot be removed." } else { "Remove this unused managed tag." },
                                                onclick: { let tag = tag.clone(); move |_| on_remove.call(tag.clone()) },
                                                "✕"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(message) = error_message {
                    div { class: "mt-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700", "{message}" }
                }

                div { class: "mt-6 flex justify-end gap-3",
                    button {
                        class: "rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50",
                        disabled: busy,
                        onclick: move |_| on_close.call(()),
                        "Cancel"
                    }
                    button {
                        class: "rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-60",
                        disabled: busy,
                        onclick: move |_| on_save.call(()),
                        if busy { "Saving..." } else { "Save Tags" }
                    }
                }
            }
        }
    }
}

#[component]
fn ReorderConfirmationModal(
    confirmation: ReorderConfirmation,
    busy: bool,
    on_cancel: EventHandler<()>,
    on_confirm: EventHandler<()>,
) -> Element {
    let (title, message) = match &confirmation {
        ReorderConfirmation::Button {
            source_name,
            target_name,
            ..
        } => (
            "Confirm Button Move",
            format!(
                "Are you sure you want to move the button '{}' before/onto '{}'?",
                source_name, target_name
            ),
        ),
        ReorderConfirmation::Row {
            button_name,
            source_name,
            target_name,
            ..
        } => (
            "Confirm URL Row Move",
            format!(
                "Are you sure you want to move the URL row '{}' in button '{}' before/onto '{}'?",
                source_name, button_name, target_name
            ),
        ),
    };

    rsx! {
        div {
            class: "fixed inset-0 z-[70] flex items-center justify-center bg-black/50 p-4",
            div {
                class: "w-full max-w-lg rounded-xl bg-white p-6 shadow-2xl",
                onclick: move |event| event.stop_propagation(),

                div {
                    class: "flex items-start gap-4",
                    div {
                        class: "flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-amber-100 text-xl",
                        "↕"
                    }
                    // `min-w-0` is important here because this is a flex child.
                    // Without it, a very long button/URL-row name can force the
                    // text column wider than the modal itself.
                    div {
                        class: "min-w-0 flex-1 overflow-hidden",
                        h2 {
                            class: "break-words text-lg font-semibold text-gray-900",
                            "{title}"
                        }
                        p {
                            // `break-all` handles names made from one enormous
                            // unbroken string, such as `Rust111111111111...`.
                            class: "mt-2 break-all text-sm leading-6 text-gray-600",
                            "{message}"
                        }
                        p {
                            class: "mt-2 break-words text-xs text-gray-500",
                            "Click Move to apply the new order, or Cancel. You can also press Escape."
                        }
                    }
                }

                div {
                    class: "mt-6 flex justify-end gap-3",
                    button {
                        class: "rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 shadow-sm transition hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-50",
                        disabled: busy,
                        onclick: move |_| on_cancel.call(()),
                        "Cancel"
                    }
                    button {
                        class: "rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-50",
                        disabled: busy,
                        autofocus: true,
                        onclick: move |_| on_confirm.call(()),
                        if busy { "Moving..." } else { "Move" }
                    }
                }
            }
        }
    }
}

#[component]
fn ButtonEditorModal(
    editor: ButtonEditorState,
    busy: bool,
    error_message: Option<String>,
    on_close: EventHandler<()>,
    on_name_input: EventHandler<String>,
    on_first_row_name_input: EventHandler<String>,
    on_first_row_url_input: EventHandler<String>,
    on_first_row_hashtags_input: EventHandler<String>,
    on_add_first_row_tag: EventHandler<String>,
    available_tags: Vec<String>,
    on_first_row_secrets_input: EventHandler<String>,
    on_save: EventHandler<()>,
    on_clone: EventHandler<()>,
    on_delete: EventHandler<()>,
) -> Element {
    let is_create = matches!(editor.mode, ButtonEditorMode::Create);

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4",
            // Do not close create/edit forms from a backdrop click. Escape is
            // the safe cancel path.

            div {
                class: "w-full max-w-xl rounded-lg bg-white p-6 shadow-xl",
                onclick: move |event| event.stop_propagation(),

                h2 { class: "text-xl font-semibold text-gray-900",
                    if is_create { "Add Button" } else { "Edit Button" }
                }

                p { class: "mt-2 text-sm text-gray-600",
                    if is_create {
                        "New buttons start with one required row so the saved JSON always stays valid."
                    } else {
                        "Rename this button or delete it entirely. To manage its URL list, open the button and use the row controls."
                    }
                }

                div { class: "mt-4 space-y-4",
                    label { class: "block",
                        span { class: "mb-1 block text-sm font-medium text-gray-700", "Button name" }
                        input {
                            class: "w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                            value: editor.draft.name.clone(),
                            oninput: move |event| on_name_input.call(event.value()),
                        }
                    }

                    if is_create {
                        div { class: "rounded-md border border-gray-200 bg-gray-50 p-4",
                            h3 { class: "text-sm font-semibold text-gray-800", "First Row" }

                            div { class: "mt-3 grid gap-3 md:grid-cols-2",
                                label { class: "block",
                                    span { class: "mb-1 block text-sm font-medium text-gray-700", "Row name" }
                                    input {
                                        class: "w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                                        value: editor.draft.first_row_name.clone(),
                                        oninput: move |event| on_first_row_name_input.call(event.value()),
                                    }
                                }

                                label { class: "block md:col-span-2",
                                    span { class: "mb-1 block text-sm font-medium text-gray-700", "URL" }
                                    input {
                                        class: "w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                                        value: editor.draft.first_row_url.clone(),
                                        oninput: move |event| on_first_row_url_input.call(event.value()),
                                    }
                                }

                                label { class: "block",
                                    span { class: "mb-1 block text-sm font-medium text-gray-700", "Hashtags" }
                                    input {
                                        class: "w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                                        list: "available-tags-first-row",
                                        value: editor.draft.first_row_hashtags.clone(),
                                        oninput: move |event| on_first_row_hashtags_input.call(event.value()),
                                    }
                                    datalist { id: "available-tags-first-row",
                                        for tag in available_tags.iter() {
                                            option { value: tag.clone() }
                                        }
                                    }
                                }

                                if !available_tags.is_empty() {
                                    div { class: "md:col-span-2 flex flex-wrap gap-2",
                                        span { class: "w-full text-xs text-gray-500", "Available tags — click to add:" }
                                        for tag in available_tags.iter() {
                                            button {
                                                r#type: "button",
                                                class: "rounded-full bg-gray-100 px-2 py-1 text-[11px] text-gray-700 hover:bg-blue-100 hover:text-blue-700",
                                                disabled: busy,
                                                onclick: { let tag = tag.clone(); move |_| on_add_first_row_tag.call(tag.clone()) },
                                                "+ {tag}"
                                            }
                                        }
                                    }
                                }

                                label { class: "block",
                                    span { class: "mb-1 block text-sm font-medium text-gray-700", "Secrets" }
                                    input {
                                        class: "w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                                        value: editor.draft.first_row_secrets.clone(),
                                        oninput: move |event| on_first_row_secrets_input.call(event.value()),
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(message) = error_message {
                    div { class: "mt-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700", "{message}" }
                }

                div { class: "mt-6 flex flex-wrap items-center justify-between gap-3",
                    div { class: "flex flex-wrap gap-2",
                        if !is_create {
                            button {
                                class: "rounded-md border border-blue-200 px-4 py-2 text-sm font-medium text-blue-700 transition hover:bg-blue-50 disabled:cursor-not-allowed disabled:opacity-60",
                                disabled: busy,
                                title: "Clone this button immediately after the original with a unique -1, -2, -3... name.",
                                onclick: move |_| on_clone.call(()),
                                "Clone Button"
                            }

                            button {
                                class: "rounded-md border border-red-200 px-4 py-2 text-sm font-medium text-red-700 transition hover:bg-red-50 disabled:cursor-not-allowed disabled:opacity-60",
                                disabled: busy,
                                onclick: move |_| on_delete.call(()),
                                "Delete Button"
                            }
                        }
                    }

                    div { class: "flex gap-3",
                        button {
                            class: "rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 transition hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60",
                            disabled: busy,
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }

                        button {
                            class: "rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-60",
                            disabled: busy,
                            onclick: move |_| on_save.call(()),
                            if busy { "Saving..." } else if is_create { "Create Button" } else { "Save Changes" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn RowEditorModal(
    editor: RowEditorState,
    busy: bool,
    error_message: Option<String>,
    on_close: EventHandler<()>,
    on_kind_change: EventHandler<RowKind>,
    on_name_input: EventHandler<String>,
    on_url_input: EventHandler<String>,
    on_hashtags_input: EventHandler<String>,
    on_add_tag: EventHandler<String>,
    available_tags: Vec<String>,
    on_comments_input: EventHandler<String>,
    on_secrets_input: EventHandler<String>,
    on_save: EventHandler<()>,
    on_clone: EventHandler<()>,
    on_delete: EventHandler<()>,
) -> Element {
    let is_create = editor.original_name.is_none();
    let is_link = editor.draft.kind == RowKind::Link;

    // The row editor is a second modal sitting above the URL-list modal.
    // Keep keyboard focus inside this top-most modal so Tab cannot jump into
    // controls belonging to the modal behind it.
    #[cfg(feature = "web")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;

        use_effect(move || {
            let Some(window) = web_sys::window() else {
                return;
            };
            let Some(document) = window.document() else {
                return;
            };
            let Some(modal) = document.get_element_by_id("row_editor_modal") else {
                return;
            };

            // Focus the first useful field when the editor appears.
            if let Some(element) = document.get_element_by_id("row_editor_name") {
                if let Some(input) = element.dyn_ref::<web_sys::HtmlInputElement>() {
                    let _ = input.focus();
                    input.select();
                }
            }

            let document_for_handler = document.clone();
            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                if event.key() != "Tab" {
                    return;
                }

                let Some(modal) = document_for_handler.get_element_by_id("row_editor_modal") else {
                    return;
                };

                let Some(active) = document_for_handler.active_element() else {
                    return;
                };

                if !modal.contains(Some(&active)) {
                    return;
                }

                let Ok(elements) = modal.query_selector_all(
                    "button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])"
                ) else {
                    return;
                };

                let mut focusable = Vec::<web_sys::HtmlElement>::new();
                for index in 0..elements.length() {
                    if let Some(node) = elements.item(index) {
                        if let Ok(element) = node.dyn_into::<web_sys::HtmlElement>() {
                            focusable.push(element);
                        }
                    }
                }

                if focusable.is_empty() {
                    return;
                }

                let current_index = focusable
                    .iter()
                    .position(|element| element.is_same_node(Some(&active)))
                    .unwrap_or(0);

                event.prevent_default();

                let next_index = if event.shift_key() {
                    if current_index == 0 {
                        focusable.len() - 1
                    } else {
                        current_index - 1
                    }
                } else {
                    (current_index + 1) % focusable.len()
                };

                let _ = focusable[next_index].focus();
            }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

            let _ = document
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());

            let window_for_cleanup = window.clone();
            use dioxus::prelude::use_drop;
            use_drop(move || {
                if let Some(document) = window_for_cleanup.document() {
                    let _ = document.remove_event_listener_with_callback(
                        "keydown",
                        closure.as_ref().unchecked_ref(),
                    );
                }
            });
        });
    }

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4",
            // Do not discard partially entered row data when the backdrop is
            // clicked. Escape is the intentional cancel path.

            div {
                id: "row_editor_modal",
                class: "w-full max-w-xl rounded-lg bg-white p-6 shadow-xl",
                onclick: move |event| event.stop_propagation(),

                h2 { class: "text-xl font-semibold text-gray-900",
                    if is_create { "Add Row" } else { "Edit Row" }
                }

                p { class: "mt-2 text-sm text-gray-600",
                    "Button: {editor.button_name} • Use this editor to add, edit, delete, or reorder URL rows."
                }

                div { class: "mt-4 space-y-4",
                    div {
                        span { class: "mb-2 block text-sm font-medium text-gray-700", "Row type" }
                        div { class: "flex gap-3",
                            button {
                                class: if is_link {
                                    "rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white"
                                } else {
                                    "rounded-md border border-gray-300 px-3 py-2 text-sm font-medium text-gray-700"
                                },
                                disabled: busy,
                                onclick: move |_| on_kind_change.call(RowKind::Link),
                                "Link"
                            }

                            button {
                                class: if !is_link {
                                    "rounded-md bg-blue-600 px-3 py-2 text-sm font-medium text-white"
                                } else {
                                    "rounded-md border border-gray-300 px-3 py-2 text-sm font-medium text-gray-700"
                                },
                                disabled: busy,
                                onclick: move |_| on_kind_change.call(RowKind::Divider),
                                "Divider"
                            }
                        }
                    }

                    label { class: "block",
                        span { class: "mb-1 block text-sm font-medium text-gray-700", "Row name" }
                        input {
                            id: "row_editor_name",
                            class: "w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                            value: editor.draft.name.clone(),
                            oninput: move |event| on_name_input.call(event.value()),
                        }
                    }

                    if is_link {
                        div { class: "grid gap-3 md:grid-cols-2",
                            label { class: "block md:col-span-2",
                                span { class: "mb-1 block text-sm font-medium text-gray-700", "URL" }
                                input {
                                    class: "w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                                    value: editor.draft.url.clone(),
                                    oninput: move |event| on_url_input.call(event.value()),
                                }
                            }

                            label { class: "block",
                                span { class: "mb-1 block text-sm font-medium text-gray-700", "Hashtags" }
                                input {
                                    class: "w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                                    list: "available-tags-row",
                                    value: editor.draft.hashtags.clone(),
                                    oninput: move |event| on_hashtags_input.call(event.value()),
                                }
                                datalist { id: "available-tags-row",
                                    for tag in available_tags.iter() {
                                        option { value: tag.clone() }
                                    }
                                }
                            }

                            label { class: "block md:col-span-2",
                                span { class: "mb-1 block text-sm font-medium text-gray-700", "Comments" }
                                textarea {
                                    class: "w-full min-h-28 resize-y rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                                    rows: "5",
                                    placeholder: "Optional notes about this URL...",
                                    value: editor.draft.comments.clone(),
                                    oninput: move |event| on_comments_input.call(event.value()),
                                }
                            }

                            if !available_tags.is_empty() {
                                div { class: "md:col-span-2 flex flex-wrap gap-2",
                                    span { class: "w-full text-xs text-gray-500", "Available tags — click to add:" }
                                    for tag in available_tags.iter() {
                                        button {
                                            r#type: "button",
                                            class: "rounded-full bg-gray-100 px-2 py-1 text-[11px] text-gray-700 hover:bg-blue-100 hover:text-blue-700",
                                            disabled: busy,
                                            onclick: { let tag = tag.clone(); move |_| on_add_tag.call(tag.clone()) },
                                            "+ {tag}"
                                        }
                                    }
                                }
                            }

                            label { class: "block",
                                span { class: "mb-1 block text-sm font-medium text-gray-700", "Secrets" }
                                input {
                                    class: "w-full rounded-md border border-gray-300 px-3 py-2 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                                    value: editor.draft.secrets.clone(),
                                    oninput: move |event| on_secrets_input.call(event.value()),
                                }
                            }
                        }
                    } else {
                        p { class: "rounded-md border border-gray-200 bg-gray-50 px-3 py-2 text-sm text-gray-600",
                            "Divider rows use an empty string value in the JSON file. Only the row name is editable here."
                        }
                    }
                }

                if let Some(message) = error_message {
                    div { class: "mt-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700", "{message}" }
                }

                div { class: "mt-6 flex flex-wrap items-center justify-between gap-3",
                    div { class: "flex flex-wrap gap-2",
                        if !is_create {
                            button {
                                class: "rounded-md border border-blue-200 px-4 py-2 text-sm font-medium text-blue-700 transition hover:bg-blue-50 disabled:cursor-not-allowed disabled:opacity-60",
                                disabled: busy,
                                title: "Clone this row immediately after the original with a unique -1, -2, -3... name.",
                                onclick: move |_| on_clone.call(()),
                                "Clone Row"
                            }

                            button {
                                class: "rounded-md border border-red-200 px-4 py-2 text-sm font-medium text-red-700 transition hover:bg-red-50 disabled:cursor-not-allowed disabled:opacity-60",
                                onclick: move |_| on_delete.call(()),
                                "Delete Row"
                            }
                        }
                    }

                    div { class: "flex gap-3",
                        button {
                            class: "rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 transition hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-60",
                            disabled: busy,
                            onclick: move |_| on_close.call(()),
                            "Cancel"
                        }

                        button {
                            class: "rounded-md bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-60",
                            disabled: busy,
                            onclick: move |_| on_save.call(()),
                            if busy { "Saving..." } else if is_create { "Create Row" } else { "Save Changes" }
                        }
                    }
                }
            }
        }
    }
}

impl ButtonRowView {
    fn name(&self) -> &str {
        match self {
            Self::Divider { name } | Self::Link { name, .. } => name,
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn download_zip_in_browser(bytes: Vec<u8>, filename: String) {
    use js_sys::{Array, Uint8Array};
    use wasm_bindgen::JsCast;
    use web_sys::{Blob, HtmlElement, Url};

    // The ZIP is already generated on the server.  This tiny browser-side
    // section only turns the returned bytes into a temporary Blob URL and
    // clicks a hidden anchor.  There is no polling or filesystem scanning.
    let array = Uint8Array::from(bytes.as_slice());
    let parts = Array::new();
    parts.push(&array);

    let Ok(blob) = Blob::new_with_u8_array_sequence(&parts) else {
        return;
    };
    let Ok(url) = Url::create_object_url_with_blob(&blob) else {
        return;
    };

    let Some(window) = web_sys::window() else {
        let _ = Url::revoke_object_url(&url);
        return;
    };
    let Some(document) = window.document() else {
        let _ = Url::revoke_object_url(&url);
        return;
    };
    let Ok(element) = document.create_element("a") else {
        let _ = Url::revoke_object_url(&url);
        return;
    };
    let Ok(anchor) = element.dyn_into::<HtmlElement>() else {
        let _ = Url::revoke_object_url(&url);
        return;
    };

    let _ = anchor.set_attribute("href", &url);
    let _ = anchor.set_attribute("download", &filename);
    anchor.click();

    // Release the temporary browser object shortly after the click so the ZIP
    // does not remain pinned in browser memory indefinitely.
    let callback = wasm_bindgen::closure::Closure::once(Box::new(move || {
        let _ = Url::revoke_object_url(&url);
    }) as Box<dyn FnOnce()>);

    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        1_000,
    );
    callback.forget();
}

#[server]
async fn export_dashboard_zip_server(
    credentials: AdminCredentials,
) -> Result<(Vec<u8>, String), ServerFnError> {
    if !credentials.is_admin() {
        return Err(ServerFnError::ServerError {
            message: "Admin login is required for this action.".to_string(),
            code: 403,
            details: None,
        });
    }

    crate::dashboard_data::export_dashboard_zip(&credentials).map_err(|error| {
        ServerFnError::ServerError {
            message: error.to_string(),
            code: 500,
            details: None,
        }
    })
}

#[server]
async fn create_button_server(
    credentials: AdminCredentials,
    input: ButtonCreateInput,
) -> Result<(), ServerFnError> {
    crate::dashboard_data::create_button(
        std::path::Path::new(crate::dashboard_data::BUTTONS_V3_FILE),
        &credentials,
        input,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn rename_button_server(
    credentials: AdminCredentials,
    input: ButtonRenameInput,
) -> Result<(), ServerFnError> {
    crate::dashboard_data::rename_button(
        std::path::Path::new(crate::dashboard_data::BUTTONS_V3_FILE),
        &credentials,
        input,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn clone_button_server(
    credentials: AdminCredentials,
    button_name: String,
) -> Result<String, ServerFnError> {
    crate::dashboard_data::clone_button(
        std::path::Path::new(crate::dashboard_data::BUTTONS_V3_FILE),
        &credentials,
        &button_name,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn delete_button_server(
    credentials: AdminCredentials,
    button_name: String,
) -> Result<(), ServerFnError> {
    crate::dashboard_data::delete_button(
        std::path::Path::new(crate::dashboard_data::BUTTONS_V3_FILE),
        &credentials,
        &button_name,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn save_row_server(
    credentials: AdminCredentials,
    button_name: String,
    input: ButtonRowInput,
) -> Result<(), ServerFnError> {
    crate::dashboard_data::save_row(
        std::path::Path::new(crate::dashboard_data::BUTTONS_V3_FILE),
        &credentials,
        &button_name,
        input,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn clone_row_server(
    credentials: AdminCredentials,
    button_name: String,
    row_name: String,
) -> Result<String, ServerFnError> {
    crate::dashboard_data::clone_row(
        std::path::Path::new(crate::dashboard_data::BUTTONS_V3_FILE),
        &credentials,
        &button_name,
        &row_name,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn delete_row_server(
    credentials: AdminCredentials,
    button_name: String,
    row_name: String,
) -> Result<(), ServerFnError> {
    crate::dashboard_data::delete_row(
        std::path::Path::new(crate::dashboard_data::BUTTONS_V3_FILE),
        &credentials,
        &button_name,
        &row_name,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn save_tags_server(
    credentials: AdminCredentials,
    tags: Vec<String>,
) -> Result<(), ServerFnError> {
    crate::dashboard_data::save_managed_tags(&credentials, tags).map_err(|error| {
        ServerFnError::ServerError {
            message: error.to_string(),
            code: 500,
            details: None,
        }
    })
}

#[server]
async fn reorder_buttons_server(
    credentials: AdminCredentials,
    from_index: usize,
    to_index: usize,
) -> Result<(), ServerFnError> {
    crate::dashboard_data::reorder_buttons(
        std::path::Path::new(crate::dashboard_data::BUTTONS_V3_FILE),
        &credentials,
        from_index,
        to_index,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn check_json_server(credentials: AdminCredentials) -> Result<(), ServerFnError> {
    if !credentials.is_admin() {
        return Err(ServerFnError::ServerError {
            message: "Admin login is required for this action.".to_string(),
            code: 403,
            details: None,
        });
    }

    crate::dashboard_data::check_dashboard_json(std::path::Path::new(
        crate::dashboard_data::BUTTONS_V3_FILE,
    ))
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn reorder_rows_server(
    credentials: AdminCredentials,
    button_name: String,
    from_index: usize,
    to_index: usize,
) -> Result<(), ServerFnError> {
    crate::dashboard_data::reorder_rows(
        std::path::Path::new(crate::dashboard_data::BUTTONS_V3_FILE),
        &credentials,
        &button_name,
        from_index,
        to_index,
    )
    .map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn set_autosort_server(
    credentials: AdminCredentials,
    enabled: bool,
) -> Result<(), ServerFnError> {
    crate::dashboard_data::set_system_autosort(&credentials, enabled).map_err(|error| {
        ServerFnError::ServerError {
            message: error.to_string(),
            code: 500,
            details: None,
        }
    })
}

