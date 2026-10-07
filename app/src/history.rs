#![allow(non_snake_case)]

use crate::dashboard_data::{self, AdminCredentials};
use crate::{ADMIN_AUTH, DASHBOARD_REFRESH_KEY, HISTORY_PAGE};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const HISTORY_DIR: &str = dashboard_data::HISTORY_DIR;
pub const MAX_SNAPSHOTS: usize = dashboard_data::MAX_HISTORY_SNAPSHOTS;
pub const SNAPSHOTS_PER_PAGE: usize = 10;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistorySnapshot {
    pub filename: String,
    pub created_label: String,
    pub sort_key: String,
    pub preview: Vec<HistoryDiffLine>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HistoryDiffLine {
    pub kind: HistoryDiffKind,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum HistoryDiffKind {
    Context,
    Added,
    Removed,
}

// -----------------------------------------------------------------------------
// HISTORY LIST / REVERT / CLEAR
// -----------------------------------------------------------------------------
//
// The actual backup is created centrally by
// dashboard_data::write_dashboard_document().
// Keeping it there also lets the standalone migration binary compile cleanly.
// -----------------------------------------------------------------------------

// -----------------------------------------------------------------------------
// LIST SNAPSHOTS
// -----------------------------------------------------------------------------

pub fn list_history() -> anyhow::Result<Vec<HistorySnapshot>> {
    if !Path::new(HISTORY_DIR).exists() {
        return Ok(Vec::new());
    }

    let mut snapshots = Vec::new();

    // Compare snapshots with the current live dashboard.  The UI only shows a
    // small preview, giving the admin a quick "what will change?" view before
    // choosing Revert.
    let current_document = if Path::new(dashboard_data::BUTTONS_V3_FILE).exists() {
        Some(std::fs::read_to_string(dashboard_data::BUTTONS_V3_FILE)?)
    } else {
        None
    };

    for entry in std::fs::read_dir(HISTORY_DIR)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let Some(filename) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if !is_safe_snapshot_filename(filename) {
            continue;
        }

        let sort_key = filename.to_string();
        let created_label = timestamp_label_from_filename(filename);
        let preview = current_document
            .as_deref()
            .and_then(|current| build_history_preview(&path, current).ok())
            .unwrap_or_default();

        snapshots.push(HistorySnapshot {
            filename: filename.to_string(),
            created_label,
            sort_key,
            preview,
        });
    }

    // New filenames are YYYYMMDD_HHMMSS_milliseconds, so reverse filename ordering puts
    // the newest snapshot first.
    snapshots.sort_by(|left, right| {
        right
            .sort_key
            .cmp(&left.sort_key)
            .then_with(|| right.filename.cmp(&left.filename))
    });

    Ok(snapshots)
}

fn is_safe_snapshot_filename(filename: &str) -> bool {
    let Some(stem) = filename
        .strip_prefix("dashboard-")
        .and_then(|value| value.strip_suffix(".json"))
    else {
        return false;
    };

    let bytes = stem.as_bytes();

    // New format:
    // dashboard-YYYYMMDD_HHMMSS_milliseconds.json
    //
    // Example:
    // dashboard-20260921_061530_123.json
    if bytes.len() == 19
        && bytes[8] == b'_'
        && bytes[15] == b'_'
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[16..].iter().all(u8::is_ascii_digit)
    {
        return true;
    }

    // Same format with a collision suffix:
    // dashboard-YYYYMMDD_HHMMSS_milliseconds_01.json
    if bytes.len() >= 22
        && bytes[8] == b'_'
        && bytes[15] == b'_'
        && bytes[19] == b'_'
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[16..19].iter().all(u8::is_ascii_digit)
        && bytes[20..].iter().all(u8::is_ascii_digit)
    {
        return true;
    }

    // Accept the old nanosecond filename format too, so upgrading does not
    // make existing snapshots disappear from the History page.
    stem.chars().all(|character| character.is_ascii_digit())
}

fn timestamp_label_from_filename(filename: &str) -> String {
    let Some(stem) = filename
        .strip_prefix("dashboard-")
        .and_then(|value| value.strip_suffix(".json"))
    else {
        return "Unknown timestamp".to_string();
    };

    // New snapshots contain the full timestamp directly in the filename.
    // Example:
    // dashboard-20260921_061530_123.json
    if stem.len() >= 19
        && stem.as_bytes().get(8) == Some(&b'_')
        && stem.as_bytes().get(15) == Some(&b'_')
    {
        let timestamp = &stem[..19];
        return format!(
            "Created: {}-{}-{} {}:{}:{}.{} UTC",
            &timestamp[0..4],
            &timestamp[4..6],
            &timestamp[6..8],
            &timestamp[9..11],
            &timestamp[11..13],
            &timestamp[13..15],
            &timestamp[16..19],
        );
    }

    // Old snapshots used nanoseconds since Unix epoch. Keep them readable
    // enough to identify them while they are still in the history directory.
    format!("Created: {stem} ns since Unix epoch")
}

// -----------------------------------------------------------------------------
// MINI DIFF PREVIEW
// -----------------------------------------------------------------------------
//
// This is deliberately a small, dependency-free preview rather than a full
// diff engine.  It shows the first changed area with a couple of context lines.
// Red lines are from the current dashboard; green lines are from the snapshot
// that Revert would restore.
// -----------------------------------------------------------------------------

const HISTORY_DIFF_CONTEXT: usize = 2;
const HISTORY_DIFF_MAX_LINES: usize = 10;

fn build_history_preview(
    snapshot_path: &Path,
    current_json: &str,
) -> anyhow::Result<Vec<HistoryDiffLine>> {
    let snapshot_json = std::fs::read_to_string(snapshot_path)?;

    let current_lines: Vec<&str> = current_json.lines().collect();
    let snapshot_lines: Vec<&str> = snapshot_json.lines().collect();

    if current_lines == snapshot_lines {
        return Ok(Vec::new());
    }

    let first_difference = current_lines
        .iter()
        .zip(snapshot_lines.iter())
        .position(|(current_line, snapshot_line)| current_line != snapshot_line)
        .unwrap_or(current_lines.len().min(snapshot_lines.len()));

    let start = first_difference.saturating_sub(HISTORY_DIFF_CONTEXT);
    let mut preview = Vec::new();

    // A little context before the first changed line.
    for line in current_lines.iter().skip(start).take(HISTORY_DIFF_CONTEXT) {
        preview.push(HistoryDiffLine {
            kind: HistoryDiffKind::Context,
            text: format!("  {line}"),
        });
    }

    let mut current_index = first_difference;
    let mut snapshot_index = first_difference;

    while preview.len() < HISTORY_DIFF_MAX_LINES
        && (current_index < current_lines.len() || snapshot_index < snapshot_lines.len())
    {
        match (
            current_lines.get(current_index),
            snapshot_lines.get(snapshot_index),
        ) {
            (Some(current_line), Some(snapshot_line)) if current_line == snapshot_line => {
                if preview.len() >= HISTORY_DIFF_CONTEXT + 2 {
                    break;
                }

                preview.push(HistoryDiffLine {
                    kind: HistoryDiffKind::Context,
                    text: format!("  {current_line}"),
                });
                current_index += 1;
                snapshot_index += 1;
            }
            (Some(current_line), Some(snapshot_line)) => {
                preview.push(HistoryDiffLine {
                    kind: HistoryDiffKind::Removed,
                    text: format!("- {current_line}"),
                });
                current_index += 1;

                if preview.len() < HISTORY_DIFF_MAX_LINES {
                    preview.push(HistoryDiffLine {
                        kind: HistoryDiffKind::Added,
                        text: format!("+ {snapshot_line}"),
                    });
                    snapshot_index += 1;
                }
            }
            (Some(current_line), None) => {
                preview.push(HistoryDiffLine {
                    kind: HistoryDiffKind::Removed,
                    text: format!("- {current_line}"),
                });
                current_index += 1;
            }
            (None, Some(snapshot_line)) => {
                preview.push(HistoryDiffLine {
                    kind: HistoryDiffKind::Added,
                    text: format!("+ {snapshot_line}"),
                });
                snapshot_index += 1;
            }
            (None, None) => break,
        }
    }

    Ok(preview)
}

// -----------------------------------------------------------------------------
// REVERT A SNAPSHOT
// -----------------------------------------------------------------------------
//
// We validate the snapshot through the normal dashboard parser first.
//
// Then write_dashboard_document() is used to restore it. That automatically
// creates a backup of the CURRENT dashboard before the revert happens, which
// means the revert itself is also recoverable.
// -----------------------------------------------------------------------------

pub fn revert_history_snapshot(
    credentials: &AdminCredentials,
    filename: &str,
) -> anyhow::Result<()> {
    if !credentials.is_admin() {
        anyhow::bail!("Admin login is required for this action.");
    }

    if !is_safe_snapshot_filename(filename) {
        anyhow::bail!("Invalid history snapshot name.");
    }

    let snapshot_path = Path::new(HISTORY_DIR).join(filename);

    if !snapshot_path.exists() {
        anyhow::bail!("History snapshot '{filename}' no longer exists.");
    }

    let document = dashboard_data::read_dashboard_document(&snapshot_path)?;

    dashboard_data::write_dashboard_document(
        Path::new(dashboard_data::BUTTONS_V3_FILE),
        &document,
    )?;

    Ok(())
}

// -----------------------------------------------------------------------------
// DELETE ONE SNAPSHOT
// -----------------------------------------------------------------------------

pub fn delete_history_snapshot(
    credentials: &AdminCredentials,
    filename: &str,
) -> anyhow::Result<()> {
    if !credentials.is_admin() {
        anyhow::bail!("Admin login is required for this action.");
    }

    if !is_safe_snapshot_filename(filename) {
        anyhow::bail!("Invalid history snapshot name.");
    }

    let snapshot_path = Path::new(HISTORY_DIR).join(filename);

    if !snapshot_path.exists() {
        anyhow::bail!("History snapshot '{filename}' no longer exists.");
    }

    std::fs::remove_file(snapshot_path)?;

    Ok(())
}

// -----------------------------------------------------------------------------
// CLEAR HISTORY
// -----------------------------------------------------------------------------

pub fn clear_history(credentials: &AdminCredentials) -> anyhow::Result<usize> {
    if !credentials.is_admin() {
        anyhow::bail!("Admin login is required for this action.");
    }

    if !Path::new(HISTORY_DIR).exists() {
        return Ok(0);
    }

    let snapshots = list_history()?;
    let mut removed = 0usize;

    for snapshot in snapshots {
        let path = Path::new(HISTORY_DIR).join(&snapshot.filename);

        if path.exists() {
            std::fs::remove_file(path)?;
            removed += 1;
        }
    }

    Ok(removed)
}

// -----------------------------------------------------------------------------
// HISTORY PAGE
// -----------------------------------------------------------------------------

#[component]
pub fn HistoryPage() -> Element {
    let mut busy = use_signal(|| false);
    let mut error_message = use_signal(|| None::<String>);
    let mut status_message = use_signal(|| None::<String>);
    let mut current_page = use_signal(|| 0usize);
    let mut show_clear_modal = use_signal(|| false);
    let mut revert_target = use_signal(|| None::<String>);
    let mut delete_target = use_signal(|| None::<String>);

    let credentials = ADMIN_AUTH();
    let mut history = use_server_future(move || {
        let credentials = credentials.clone();
        async move { load_history_server(credentials).await }
    })?; //.unwrap();

    let is_admin = ADMIN_AUTH().is_admin();

    if !is_admin {
        return rsx! {
            div { class: "container mx-auto px-4 mt-6",
                div { class: "rounded-lg border border-red-200 bg-red-50 p-6 text-red-800 shadow-sm",
                    h1 { class: "text-xl font-semibold", "History" }
                    p { class: "mt-2", "Admin login is required to view dashboard history." }
                    button {
                        class: "mt-4 rounded-md bg-gray-700 px-4 py-2 text-sm font-medium text-white hover:bg-gray-800",
                        onclick: move |_| {
                            *HISTORY_PAGE.write() = false;
                        },
                        "Back to Dashboard"
                    }
                }
            }
        };
    }

    rsx! {
        div { class: "container mx-auto px-4 mt-6 pb-8",
            div { class: "rounded-lg border border-blue-200 bg-blue-50 p-4 shadow-sm",
                div { class: "flex flex-wrap items-center justify-between gap-3",
                    div {
                        h1 { class: "text-xl font-semibold text-blue-950", "Dashboard History" }
                        p { class: "mt-1 text-sm text-blue-800",
                            "The 100 most recent dashboard JSON snapshots are kept here, with 10 snapshots shown per page. "
                            "Every dashboard edit creates a backup before the new JSON is written."
                        }
                    }

                    div { class: "flex flex-wrap gap-2",
                        button {
                            class: "rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50",
                            disabled: busy(),
                            title: "Permanently delete all dashboard history snapshots.",
                            onclick: move |_| {
                                error_message.set(None);
                                status_message.set(None);
                                show_clear_modal.set(true);
                            },
                            "Clear History"
                        }

                        button {
                            class: "rounded-md bg-gray-700 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-gray-800",
                            onclick: move |_| {
                                *HISTORY_PAGE.write() = false;
                            },
                            "Back to Dashboard"
                        }
                    }
                }
            }

            if let Some(message) = status_message() {
                div { class: "mt-4 rounded-md border border-green-200 bg-green-50 px-3 py-2 text-sm text-green-800",
                    "{message}"
                }
            }

            if let Some(message) = error_message() {
                div { class: "mt-4 rounded-md border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700 whitespace-pre-wrap",
                    "{message}"
                }
            }

            div { class: "mt-4 space-y-3",
                match &*history.read_unchecked() {
                    Some(Ok(snapshots)) if snapshots.is_empty() => rsx! {
                        div { class: "rounded-lg border border-gray-200 bg-white p-6 text-gray-600 shadow-sm",
                            "No dashboard history snapshots exist yet."
                        }
                    },
                    Some(Ok(snapshots)) => {
                        let total_pages = snapshots.len().div_ceil(SNAPSHOTS_PER_PAGE);
                        let page = current_page().min(total_pages.saturating_sub(1));
                        let start = page * SNAPSHOTS_PER_PAGE;
                        let end = (start + SNAPSHOTS_PER_PAGE).min(snapshots.len());

                        rsx! {
                            div { class: "flex flex-wrap items-center justify-between gap-2 rounded-lg border border-gray-200 bg-white px-4 py-3 shadow-sm",
                                div { class: "text-sm text-gray-600",
                                    "Showing snapshots {start + 1}-{end} of {snapshots.len()} "
                                    "(100 maximum, 10 per page)"
                                }

                                if total_pages > 1 {
                                    div { class: "flex items-center gap-2",
                                        button {
                                            class: "rounded-md border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-40",
                                            disabled: page == 0,
                                            onclick: move |_| {
                                                if current_page() > 0 {
                                                    current_page.set(current_page() - 1);
                                                }
                                            },
                                            "Previous"
                                        }

                                        span { class: "text-sm text-gray-600",
                                            "Page {page + 1} of {total_pages}"
                                        }

                                        button {
                                            class: "rounded-md border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-700 hover:bg-gray-50 disabled:cursor-not-allowed disabled:opacity-40",
                                            disabled: page + 1 >= total_pages,
                                            onclick: move |_| {
                                                if current_page() + 1 < total_pages {
                                                    current_page.set(current_page() + 1);
                                                }
                                            },
                                            "Next"
                                        }
                                    }
                                }
                            }

                            for (display_index, snapshot) in snapshots[start..end].iter().enumerate() {
                                {
                                    let filename = snapshot.filename.clone();
                                    let revert_filename = filename.clone();
                                    let delete_filename = filename.clone();
                                    let snapshot_number = start + display_index + 1;

                                    rsx! {
                                        // Use a two-column layout so a long JSON line can
                                        // scroll inside the diff instead of forcing the whole
                                        // snapshot card to become wider.
                                        div { class: "grid grid-cols-1 items-start gap-4 rounded-lg border border-gray-200 bg-white p-4 shadow-sm lg:grid-cols-[minmax(0,1fr)_auto]",
                                            div { class: "min-w-0 w-full",
                                                div { class: "font-medium text-gray-900",
                                                    "Snapshot {snapshot_number}"
                                                }
                                                div { class: "mt-1 font-mono text-xs text-gray-500 break-all",
                                                    "{snapshot.filename}"
                                                }
                                                div { class: "mt-1 text-xs text-gray-500",
                                                    "{snapshot.created_label}"
                                                }

                                                if !snapshot.preview.is_empty() {
                                                    // The diff always fills the snapshot's content
                                                    // column. `min-w-0` above plus `overflow-auto`
                                                    // below keeps very long JSON lines contained.
                                                    div { class: "mt-3 w-full min-w-0 overflow-hidden rounded-md border border-gray-200 bg-gray-950",
                                                        div { class: "border-b border-gray-700 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide text-gray-400",
                                                            "Mini diff vs current dashboard"
                                                        }
                                                        pre { class: "max-h-40 w-full max-w-full overflow-auto px-3 py-2 font-mono text-[11px] leading-5",
                                                            for line in snapshot.preview.iter() {
                                                                div {
                                                                    class: match &line.kind {
                                                                        HistoryDiffKind::Context => "text-gray-300",
                                                                        HistoryDiffKind::Added => "bg-green-950 text-green-300",
                                                                        HistoryDiffKind::Removed => "bg-red-950 text-red-300",
                                                                    },
                                                                    "{line.text}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    div { class: "mt-3 rounded-md bg-gray-50 px-3 py-2 text-xs text-gray-500",
                                                        "No visible JSON difference from the current dashboard."
                                                    }
                                                }
                                            }

                                            div { class: "flex flex-wrap justify-end gap-2",
                                                button {
                                                    class: "rounded-md bg-amber-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-amber-700 disabled:cursor-not-allowed disabled:opacity-50",
                                                    disabled: busy(),
                                                    title: "Replace the current dashboard JSON with this snapshot. The current JSON is backed up first.",
                                                    onclick: move |_| {
                                                        error_message.set(None);
                                                        status_message.set(None);
                                                        revert_target.set(Some(revert_filename.clone()));
                                                    },
                                                    "Revert"
                                                }

                                                button {
                                                    class: "rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white shadow-sm transition hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50",
                                                    disabled: busy(),
                                                    title: "Permanently delete this history snapshot.",
                                                    onclick: move |_| {
                                                        error_message.set(None);
                                                        status_message.set(None);
                                                        delete_target.set(Some(delete_filename.clone()));
                                                    },
                                                    "Delete"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(error)) => rsx! {
                        div { class: "rounded-lg border border-red-200 bg-red-50 p-6 text-red-800 shadow-sm",
                            "Could not load dashboard history: {error}"
                        }
                    },
                    None => rsx! {
                        div { class: "rounded-lg border border-gray-200 bg-white p-6 text-gray-600 shadow-sm",
                            "Loading dashboard history..."
                        }
                    },
                }
            }

            // -----------------------------------------------------------------
            // REVERT CONFIRMATION MODAL
            // -----------------------------------------------------------------
            if let Some(filename) = revert_target() {
                div {
                    class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4",
                    onclick: move |_| {
                        if !busy() {
                            revert_target.set(None);
                        }
                    },

                    div {
                        class: "w-full max-w-lg rounded-xl bg-white p-6 shadow-2xl",
                        onclick: move |event| event.stop_propagation(),

                        h2 { class: "text-lg font-semibold text-amber-700", "Revert Dashboard Snapshot?" }
                        p { class: "mt-2 text-sm text-gray-700",
                            "Are you sure you want to revert to this snapshot?"
                        }
                        div { class: "mt-3 rounded-md border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-900",
                            strong { "A snapshot will be saved first. " }
                            "The current dashboard JSON will be backed up before the selected snapshot replaces it, so this revert can itself be undone."
                        }
                        div { class: "mt-3 rounded-md bg-gray-50 px-3 py-2 font-mono text-xs text-gray-700 break-all",
                            "{filename}"
                        }

                        div { class: "mt-6 flex justify-end gap-3",
                            button {
                                class: "rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50",
                                disabled: busy(),
                                onclick: move |_| revert_target.set(None),
                                "Cancel"
                            }

                            button {
                                class: "rounded-md bg-amber-600 px-4 py-2 text-sm font-medium text-white hover:bg-amber-700 disabled:cursor-not-allowed disabled:opacity-50",
                                disabled: busy(),
                                onclick: {
                                    let filename = filename.clone();
                                    move |_| {
                                        let filename = filename.clone();
                                        spawn(async move {
                                            busy.set(true);
                                            error_message.set(None);
                                            status_message.set(None);

                                            match revert_history_server(ADMIN_AUTH(), filename.clone()).await {
                                                Ok(()) => {
                                                    busy.set(false);
                                                    revert_target.set(None);
                                                    status_message.set(Some(
                                                        format!("Restored {filename}. Returning to the dashboard..."),
                                                    ));
                                                    *DASHBOARD_REFRESH_KEY.write() += 1;
                                                    *HISTORY_PAGE.write() = false;
                                                }
                                                Err(error) => {
                                                    busy.set(false);
                                                    error_message.set(Some(error.to_string()));
                                                }
                                            }
                                        });
                                    }
                                },
                                if busy() { "Reverting..." } else { "Yes, Revert Snapshot" }
                            }
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // DELETE ONE SNAPSHOT CONFIRMATION MODAL
            // -----------------------------------------------------------------
            if let Some(filename) = delete_target() {
                div {
                    class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4",
                    onclick: move |_| {
                        if !busy() {
                            delete_target.set(None);
                        }
                    },

                    div {
                        class: "w-full max-w-lg rounded-xl bg-white p-6 shadow-2xl",
                        onclick: move |event| event.stop_propagation(),

                        h2 { class: "text-lg font-semibold text-red-700", "Delete History Snapshot?" }
                        p { class: "mt-2 text-sm text-gray-700",
                            "Are you sure you want to permanently delete this snapshot?"
                        }
                        div { class: "mt-3 rounded-md bg-gray-50 px-3 py-2 font-mono text-xs text-gray-700 break-all",
                            "{filename}"
                        }
                        p { class: "mt-3 text-xs text-gray-500",
                            "This removes only this history snapshot. It does not change the current dashboard."
                        }

                        div { class: "mt-6 flex justify-end gap-3",
                            button {
                                class: "rounded-md border border-gray-300 px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50",
                                disabled: busy(),
                                onclick: move |_| delete_target.set(None),
                                "Cancel"
                            }

                            button {
                                class: "rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50",
                                disabled: busy(),
                                onclick: {
                                    let filename = filename.clone();
                                    move |_| {
                                        let filename = filename.clone();
                                        spawn(async move {
                                            busy.set(true);
                                            error_message.set(None);
                                            status_message.set(None);

                                            match delete_history_server(ADMIN_AUTH(), filename.clone()).await {
                                                Ok(()) => {
                                                    busy.set(false);
                                                    delete_target.set(None);
                                                    status_message.set(Some(
                                                        format!("Deleted history snapshot {filename}."),
                                                    ));
                                                    history.restart();
                                                }
                                                Err(error) => {
                                                    busy.set(false);
                                                    error_message.set(Some(error.to_string()));
                                                }
                                            }
                                        });
                                    }
                                },
                                if busy() { "Deleting..." } else { "Yes, Delete Snapshot" }
                            }
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // CLEAR HISTORY CONFIRMATION MODAL
            // -----------------------------------------------------------------
            //
            // We intentionally list every snapshot here so the administrator
            // can see exactly what will be deleted before confirming.
            // -----------------------------------------------------------------
            if show_clear_modal() {
                div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4",
                    div { class: "flex max-h-[85vh] w-full max-w-2xl flex-col rounded-xl bg-white shadow-2xl",
                        div { class: "border-b border-gray-200 p-5",
                            div { class: "flex items-start justify-between gap-4",
                                div {
                                    h2 { class: "text-lg font-semibold text-red-700", "Clear Dashboard History?" }
                                    p { class: "mt-1 text-sm text-gray-600",
                                        "Are you really sure? This permanently deletes every history snapshot listed below."
                                    }
                                }

                                button {
                                    class: "rounded-md px-2 py-1 text-xl text-gray-500 hover:bg-gray-100 hover:text-gray-800",
                                    title: "Cancel",
                                    onclick: move |_| {
                                        show_clear_modal.set(false);
                                    },
                                    "×"
                                }
                            }
                        }

                        div { class: "min-h-0 flex-1 overflow-y-auto p-5",
                            match &*history.read_unchecked() {
                                Some(Ok(snapshots)) if snapshots.is_empty() => rsx! {
                                    p { class: "text-sm text-gray-500", "There are no snapshots to delete." }
                                },
                                Some(Ok(snapshots)) => rsx! {
                                    div { class: "space-y-2",
                                        p { class: "mb-3 text-sm font-medium text-gray-700",
                                            "The following {snapshots.len()} snapshot(s) will be deleted:"
                                        }

                                        for snapshot in snapshots.iter() {
                                            div { class: "rounded border border-gray-200 bg-gray-50 px-3 py-2 font-mono text-xs text-gray-700 break-all",
                                                "{snapshot.filename}"
                                            }
                                        }
                                    }
                                },
                                Some(Err(error)) => rsx! {
                                    p { class: "text-sm text-red-600", "Could not load the history list: {error}" }
                                },
                                None => rsx! {
                                    p { class: "text-sm text-gray-500", "Loading history list..." }
                                },
                            }
                        }

                        div { class: "flex flex-wrap justify-end gap-2 border-t border-gray-200 p-5",
                            button {
                                class: "rounded-md border border-gray-300 bg-white px-4 py-2 text-sm font-medium text-gray-700 hover:bg-gray-50 disabled:opacity-50",
                                disabled: busy(),
                                onclick: move |_| {
                                    show_clear_modal.set(false);
                                },
                                "Cancel"
                            }

                            button {
                                class: "rounded-md bg-red-600 px-4 py-2 text-sm font-medium text-white hover:bg-red-700 disabled:cursor-not-allowed disabled:opacity-50",
                                disabled: busy(),
                                onclick: move |_| {
                                    async move {
                                        busy.set(true);
                                        error_message.set(None);
                                        status_message.set(None);

                                        match clear_history_server(ADMIN_AUTH()).await {
                                            Ok(count) => {
                                                busy.set(false);
                                                show_clear_modal.set(false);
                                                current_page.set(0);
                                                status_message.set(Some(
                                                    format!("Cleared {count} dashboard history snapshot(s).")
                                                ));

                                                // Reload the server future so the
                                                // page immediately shows an empty
                                                // history list.
                                                history.restart();
                                            }
                                            Err(error) => {
                                                busy.set(false);
                                                error_message.set(Some(error.to_string()));
                                            }
                                        }
                                    }
                                },
                                "Yes, Clear Everything"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[server]
async fn load_history_server(
    credentials: AdminCredentials,
) -> Result<Vec<HistorySnapshot>, ServerFnError> {
    if !credentials.is_admin() {
        return Err(ServerFnError::ServerError {
            message: "Admin login is required for this action.".to_string(),
            code: 403,
            details: None,
        });
    }

    list_history().map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn revert_history_server(
    credentials: AdminCredentials,
    filename: String,
) -> Result<(), ServerFnError> {
    revert_history_snapshot(&credentials, &filename).map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn delete_history_server(
    credentials: AdminCredentials,
    filename: String,
) -> Result<(), ServerFnError> {
    delete_history_snapshot(&credentials, &filename).map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}

#[server]
async fn clear_history_server(credentials: AdminCredentials) -> Result<usize, ServerFnError> {
    clear_history(&credentials).map_err(|error| ServerFnError::ServerError {
        message: error.to_string(),
        code: 500,
        details: None,
    })
}
