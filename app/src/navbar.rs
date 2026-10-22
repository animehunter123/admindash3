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
// NAVBAR TREE HELPERS
// -----------------------------------------------------------------------------
// Paths are sequences of child indexes (for example [1, 0, 2]). Keeping paths
// rather than references lets us safely edit a cloned tree and then set the
// signal once. This is the same basic pattern used by tree editors elsewhere.
fn nav_item_at_path<'a>(items: &'a [NavItem], path: &[usize]) -> Option<&'a NavItem> {
    let (first, rest) = path.split_first()?;
    let item = items.get(*first)?;
    if rest.is_empty() {
        Some(item)
    } else {
        nav_item_at_path(&item.children, rest)
    }
}

fn nav_item_at_path_mut<'a>(items: &'a mut [NavItem], path: &[usize]) -> Option<&'a mut NavItem> {
    let (first, rest) = path.split_first()?;
    let item = items.get_mut(*first)?;
    if rest.is_empty() {
        Some(item)
    } else {
        nav_item_at_path_mut(&mut item.children, rest)
    }
}

fn nav_siblings_mut<'a>(
    items: &'a mut Vec<NavItem>,
    path: &[usize],
) -> Option<&'a mut Vec<NavItem>> {
    if path.is_empty() {
        return None;
    }
    if path.len() == 1 {
        return Some(items);
    }
    nav_item_at_path_mut(items, &path[..path.len() - 1]).map(|item| &mut item.children)
}

fn nav_remove_at_path(items: &mut Vec<NavItem>, path: &[usize]) -> Option<NavItem> {
    let index = *path.last()?;
    let siblings = nav_siblings_mut(items, path)?;
    if index < siblings.len() {
        Some(siblings.remove(index))
    } else {
        None
    }
}

fn nav_insert_under_path(items: &mut Vec<NavItem>, parent: &[usize], item: NavItem) -> bool {
    if parent.is_empty() {
        items.push(item);
        return true;
    }

    if let Some(parent_item) = nav_item_at_path_mut(items, parent) {
        // If + Child is used on a single direct URL, convert it into a real
        // submenu without losing the existing destination. The old URL becomes
        // the first child; the newly-created child is appended after it.
        // Example: "asingleurl" -> menu "asingleurl" containing "url01" and
        // the new child "url02".
        if parent_item.children.is_empty() {
            if let Some(existing_url) = parent_item.url.take() {
                let child_name = existing_url
                    .trim_end_matches('/')
                    .rsplit('/')
                    .find(|part| !part.is_empty())
                    .unwrap_or(parent_item.name.as_str())
                    .to_string();
                parent_item.children.push(NavItem {
                    name: child_name,
                    url: Some(existing_url),
                    open_in_new_window: false,
                    children: Vec::new(),
                });
            }
        }
        parent_item.children.push(item);
        true
    } else {
        false
    }
}

fn nav_find_path(
    items: &[NavItem],
    name: &str,
    url: &str,
    parent: &mut Vec<usize>,
) -> Option<Vec<usize>> {
    for (index, item) in items.iter().enumerate() {
        parent.push(index);
        if item.name == name && item.url.as_deref().unwrap_or("") == url {
            return Some(parent.clone());
        }
        if let Some(found) = nav_find_path(&item.children, name, url, parent) {
            return Some(found);
        }
        parent.pop();
    }
    None
}

fn nav_collect_move_targets(
    items: &[NavItem],
    prefix: Vec<usize>,
    output: &mut Vec<(Vec<usize>, String)>,
) {
    for (index, item) in items.iter().enumerate() {
        // Root index 0 is the fixed Admindash3 brand and is not a move target.
        if prefix.is_empty() && index == 0 {
            continue;
        }
        let mut path = prefix.clone();
        path.push(index);
        // A parent at level four can receive a child at level five.
        if path.len() <= 4 {
            let indent = "— ".repeat(path.len().saturating_sub(1));
            output.push((path.clone(), format!("{indent}{}", item.name)));
            nav_collect_move_targets(&item.children, path, output);
        }
    }
}

fn nav_path_after_removal(target: &[usize], source: &[usize]) -> Vec<usize> {
    // Removing a sibling shifts the target's index down by one. This works at
    // any depth, including when source and target have different parents.
    let mut adjusted = target.to_vec();
    if source.is_empty() || target.len() < source.len() {
        return adjusted;
    }

    let source_parent = &source[..source.len() - 1];
    if target.starts_with(source_parent) {
        let sibling_index_position = source_parent.len();
        if adjusted.len() > sibling_index_position
            && adjusted[sibling_index_position] > source[source.len() - 1]
        {
            adjusted[sibling_index_position] -= 1;
        }
    }
    adjusted
}

// Return the number of levels occupied by this item and its deepest child.
// A leaf has height 1; an item with one child that is a leaf has height 2.
fn nav_subtree_height(item: &NavItem) -> usize {
    1 + item
        .children
        .iter()
        .map(nav_subtree_height)
        .max()
        .unwrap_or(0)
}

// Move an item beneath a destination item without ever silently discarding
// either item's URL or children.
//
// Rules:
// 1. A destination URL becomes the FIRST child link, named after the destination.
// 2. The moved item (with all of its descendants and URL intact) is appended LAST.
// 3. Existing destination children remain in their original order after the
//    preserved destination URL.
// 4. Invalid moves (self/descendant targets, missing paths, or depth > 5) fail.
fn nav_move_item_under_target(
    items: &mut Vec<NavItem>,
    source_path: &[usize],
    target_path: &[usize],
) -> Result<(), String> {
    if source_path.is_empty() || target_path.is_empty() {
        return Err("Choose a navbar item and a destination.".to_string());
    }

    // Moving an item beneath itself or one of its descendants would create a
    // cycle. The target path is from the same pre-move tree as source_path.
    if target_path.starts_with(source_path) {
        return Err("An item cannot be nested under itself or one of its children.".to_string());
    }

    let source_item = nav_item_at_path(items, source_path)
        .cloned()
        .ok_or_else(|| "The item being moved no longer exists.".to_string())?;
    let destination = nav_item_at_path(items, target_path)
        .ok_or_else(|| "That destination no longer exists.".to_string())?;

    // The destination is at target_path.len() (top level = 1). The moved
    // subtree is placed one level below it, so its deepest level is:
    // destination depth + subtree height. Check before modifying the tree.
    if target_path.len() + nav_subtree_height(&source_item) > 5 {
        return Err("That move would exceed the five-level navbar nesting limit.".to_string());
    }
    let _destination_name = destination.name.clone();

    let moved_item = nav_remove_at_path(items, source_path)
        .ok_or_else(|| "The item being moved could not be removed.".to_string())?;
    let adjusted_target = nav_path_after_removal(target_path, source_path);
    let parent = nav_item_at_path_mut(items, &adjusted_target)
        .ok_or_else(|| "That destination no longer exists after moving the item.".to_string())?;

    // A NavItem may represent either a direct URL or a dropdown. If the
    // destination has a URL, preserve it as a child BEFORE existing children;
    // then append the moved item at the end as requested.
    if let Some(existing_url) = parent.url.take() {
        parent.children.insert(
            0,
            NavItem {
                name: parent.name.clone(),
                url: Some(existing_url),
                open_in_new_window: false,
                children: Vec::new(),
            },
        );
    }
    parent.children.push(moved_item);

    crate::data_navbar::validate_nav_items(items)
}

#[cfg(test)]
mod navbar_move_tests {
    use super::{nav_item_at_path, nav_move_item_under_target};
    use crate::data_navbar::NavItem;

    fn item(name: &str, url: Option<&str>, children: Vec<NavItem>) -> NavItem {
        NavItem {
            name: name.to_string(),
            url: url.map(str::to_string),
            open_in_new_window: false,
            children,
        }
    }

    fn base_tree(top01: NavItem, top02: NavItem) -> Vec<NavItem> {
        // Index zero represents the fixed brand, which is never a move target.
        vec![item("Admindash3", None, vec![]), top01, top02]
    }

    #[test]
    fn moving_url_item_under_url_item_preserves_both_urls_and_appends_moved_item() {
        let mut tree = base_tree(
            item("top01", Some("url01"), vec![]),
            item("top02", Some("url02"), vec![]),
        );

        nav_move_item_under_target(&mut tree, &[1], &[2]).unwrap();

        let destination = nav_item_at_path(&tree, &[1]).unwrap();
        assert_eq!(destination.name, "top02");
        assert_eq!(destination.url, None);
        assert_eq!(destination.children.len(), 2);
        assert_eq!(destination.children[0].name, "top02");
        assert_eq!(destination.children[0].url.as_deref(), Some("url02"));
        assert_eq!(destination.children[1].name, "top01");
        assert_eq!(destination.children[1].url.as_deref(), Some("url01"));
    }

    #[test]
    fn moving_url_item_under_empty_menu_adds_it_as_the_first_child() {
        let mut tree = base_tree(
            item("top01", Some("url01"), vec![]),
            item("top02", None, vec![]),
        );

        nav_move_item_under_target(&mut tree, &[1], &[2]).unwrap();

        let destination = nav_item_at_path(&tree, &[1]).unwrap();
        assert_eq!(destination.url, None);
        assert_eq!(destination.children.len(), 1);
        assert_eq!(destination.children[0].name, "top01");
        assert_eq!(destination.children[0].url.as_deref(), Some("url01"));
    }

    #[test]
    fn moving_nested_item_preserves_its_entire_subtree_and_destination_url() {
        let nested = item(
            "top01",
            Some("top01-url"),
            vec![item("mid01", Some("url01"), vec![])],
        );
        let mut tree = base_tree(nested, item("top02", Some("url02"), vec![]));

        nav_move_item_under_target(&mut tree, &[1], &[2]).unwrap();

        let destination = nav_item_at_path(&tree, &[1]).unwrap();
        assert_eq!(destination.children.len(), 2);
        assert_eq!(destination.children[0].url.as_deref(), Some("url02"));
        assert_eq!(destination.children[1].name, "top01");
        assert_eq!(destination.children[1].url.as_deref(), Some("top01-url"));
        assert_eq!(destination.children[1].children[0].name, "mid01");
        assert_eq!(
            destination.children[1].children[0].url.as_deref(),
            Some("url01")
        );
    }

    #[test]
    fn destination_url_precedes_existing_children_and_moved_item_is_appended_last() {
        let destination = item(
            "top02",
            Some("url02"),
            vec![item("existing", Some("existing-url"), vec![])],
        );
        let mut tree = base_tree(item("top01", Some("url01"), vec![]), destination);

        nav_move_item_under_target(&mut tree, &[1], &[2]).unwrap();

        let children = &nav_item_at_path(&tree, &[1]).unwrap().children;
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].url.as_deref(), Some("url02"));
        assert_eq!(children[1].name, "existing");
        assert_eq!(children[2].name, "top01");
    }

    #[test]
    fn moving_a_nested_item_under_a_nested_destination_preserves_both_urls() {
        let mut tree = vec![
            item("Admindash3", None, vec![]),
            item(
                "top01",
                None,
                vec![item(
                    "mid01",
                    None,
                    vec![item("nested-source", Some("nested-url"), vec![])],
                )],
            ),
            item(
                "top02",
                None,
                vec![item("mid02", Some("mid02-url"), vec![])],
            ),
        ];

        nav_move_item_under_target(&mut tree, &[1, 0, 0], &[2, 0]).unwrap();

        let destination = nav_item_at_path(&tree, &[2, 0]).unwrap();
        assert_eq!(destination.name, "mid02");
        assert_eq!(destination.children.len(), 2);
        assert_eq!(destination.children[0].url.as_deref(), Some("mid02-url"));
        assert_eq!(destination.children[1].name, "nested-source");
        assert_eq!(destination.children[1].url.as_deref(), Some("nested-url"));
        assert!(nav_item_at_path(&tree, &[1, 0, 0]).is_none());
    }

    #[test]
    fn moving_a_later_sibling_to_an_earlier_target_uses_the_adjusted_path() {
        let mut tree = vec![
            item("Admindash3", None, vec![]),
            item("first", Some("first-url"), vec![]),
            item("source", Some("source-url"), vec![]),
            item("target", Some("target-url"), vec![]),
        ];

        nav_move_item_under_target(&mut tree, &[2], &[3]).unwrap();

        let target = nav_item_at_path(&tree, &[2]).unwrap();
        assert_eq!(target.name, "target");
        assert_eq!(target.children.len(), 2);
        assert_eq!(target.children[0].url.as_deref(), Some("target-url"));
        assert_eq!(target.children[1].name, "source");
    }

    #[test]
    fn refuses_self_descendant_and_too_deep_moves() {
        let mut tree = base_tree(
            item("top01", None, vec![item("mid01", None, vec![])]),
            item("top02", None, vec![]),
        );
        assert!(nav_move_item_under_target(&mut tree, &[1], &[1, 0]).is_err());

        let source_with_child = item(
            "source",
            None,
            vec![item("source-child", Some("child-url"), vec![])],
        );
        let deep_target = item(
            "deep1",
            None,
            vec![item(
                "deep2",
                None,
                vec![item("deep3", None, vec![item("deep4", None, vec![])])],
            )],
        );
        let mut too_deep = base_tree(source_with_child, deep_target);
        assert!(nav_move_item_under_target(&mut too_deep, &[1], &[2, 0, 0, 0]).is_err());
    }
}

// -----------------------------------------------------------------------------
// NAVBAR MODAL AUTOFOCUS HELPER
// -----------------------------------------------------------------------------
// Run after the conditional modal has reached the DOM. Prefer its first visible
// form field; for field-less confirmation dialogs, use the first useful button.
// The close X is deliberately excluded from initial focus and the first Tab stop.
fn focus_first_nav_modal_control(modal_id: &'static str) {
    use wasm_bindgen::closure::Closure;

    let Some(window) = web_sys::window() else {
        return;
    };
    let callback = Closure::once(Box::new(move || {
        let Some(window) = web_sys::window() else {
            return;
        };
        let Some(document) = window.document() else {
            return;
        };
        let Some(modal) = document.get_element_by_id(modal_id) else {
            return;
        };

        if let Some(active) = document.active_element() {
            if modal.contains(Some(&active)) {
                let tag = active.tag_name().to_ascii_lowercase();
                let text = active.text_content().unwrap_or_default();
                let is_close = matches!(text.trim(), "×" | "✕" | "✖" | "X" | "x")
                    || active.get_attribute("aria-label").as_deref() == Some("Close")
                    || active.get_attribute("title").as_deref() == Some("Close");
                if !is_close && matches!(tag.as_str(), "input" | "textarea" | "select" | "button") {
                    return;
                }
            }
        }

        if let Ok(fields) = modal.query_selector_all(
            "input:not([type='hidden']):not([disabled]), textarea:not([disabled]), select:not([disabled])"
        ) {
            for index in 0..fields.length() {
                let Some(node) = fields.item(index) else { continue; };
                let Ok(field) = node.dyn_into::<web_sys::HtmlElement>() else { continue; };
                if field.offset_width() > 0 || field.offset_height() > 0 {
                    let _ = field.focus();
                    return;
                }
            }
        }

        if let Ok(buttons) = modal.query_selector_all("button:not([disabled])") {
            for index in 0..buttons.length() {
                let Some(node) = buttons.item(index) else {
                    continue;
                };
                let Ok(button) = node.dyn_into::<web_sys::HtmlElement>() else {
                    continue;
                };
                let text = button.text_content().unwrap_or_default();
                let is_close = matches!(text.trim(), "×" | "✕" | "✖" | "X" | "x")
                    || button.get_attribute("aria-label").as_deref() == Some("Close")
                    || button.get_attribute("title").as_deref() == Some("Close");
                if !is_close && (button.offset_width() > 0 || button.offset_height() > 0) {
                    let _ = button.focus();
                    return;
                }
            }
        }
    }) as Box<dyn FnOnce()>);
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        callback.as_ref().unchecked_ref(),
        0,
    );
    callback.forget();
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

    // True when this result came from data_navbar.v2.json rather than Buttonboard.
    is_nav_item: bool,
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

    // The manager edits ordinary form fields instead of exposing raw JSON.
    // The tree remains a Vec<NavItem>, so saving preserves data_navbar.v2.json's schema.
    let mut nav_manager_open = use_signal(|| false);
    let mut nav_tree_draft = use_signal(|| items.clone());
    let mut nav_manager_error = use_signal(|| None::<String>);
    let mut nav_manager_status = use_signal(|| None::<String>);
    let mut nav_form_open = use_signal(|| false);
    let mut nav_form_path = use_signal(|| None::<Vec<usize>>);
    let mut nav_form_parent = use_signal(|| Vec::<usize>::new());
    let mut nav_form_name = use_signal(String::new);
    let mut nav_form_url = use_signal(String::new);
    let mut nav_form_open_in_new_window = use_signal(|| false);
    let mut nav_move_path = use_signal(|| None::<Vec<usize>>);
    let mut nav_delete_path = use_signal(|| None::<Vec<usize>>);
    // One shared switch controls the default collapsed state of every manager row.
    let mut nav_tree_collapsed = use_signal(|| true);
    let mut nav_move_tree_collapsed = use_signal(|| false);

    // New**: Shared tick. When it increments, every NavEntry closes its menu.
    // Used so a single outside-click can close all pinned dropdowns at once.
    let mut nav_close_tick = use_signal(|| 0u64);

    // Autofocus whichever navbar dialog is currently topmost. The helper only
    // changes focus when it is outside the modal or sitting on the close X, so
    // ordinary rerenders while typing do not steal the cursor.
    use_effect(move || {
        if nav_delete_path().is_some() {
            focus_first_nav_modal_control("nav_delete_modal");
        } else if nav_move_path().is_some() {
            focus_first_nav_modal_control("nav_move_modal");
        } else if nav_form_open() {
            focus_first_nav_modal_control("nav_editor_modal");
        } else if nav_manager_open() {
            focus_first_nav_modal_control("nav_manager_modal");
        }
    });

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

    // New**: Close every pinned / open navbar dropdown when the user clicks
    // outside any `.nav-entry-root` region (the trigger + its flyout).
    use_effect(move || {
        let Some(win) = window() else {
            return;
        };
        let Some(document) = win.document() else {
            return;
        };

        let mut nav_close_tick_for_handler = nav_close_tick;

        let click_closure =
            wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::Event| {
                let Some(target) = event.target() else {
                    return;
                };
                let Ok(element) = target.dyn_into::<web_sys::Element>() else {
                    return;
                };

                // Click landed inside a navbar entry (trigger or submenu)? Keep menus open.
                if element.closest(".nav-entry-root").ok().flatten().is_some() {
                    return;
                }

                // Otherwise bump the shared tick so every NavEntry resets.
                *nav_close_tick_for_handler.write() += 1;
            }) as Box<dyn FnMut(web_sys::Event)>);

        let _ = document
            .add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref());
        // Keep the callback alive for the lifetime of the page (same pattern as keydown).
        click_closure.forget();
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
            // Navbar editor dialogs are also closed by Escape, just like the
            // existing Buttonboard editors. Capture their signals in this
            // document-level handler so Escape works even when an input has focus.
            let mut nav_manager_open_for_handler = nav_manager_open;
            let mut nav_form_open_for_handler = nav_form_open;
            let mut nav_move_path_for_handler = nav_move_path;
            let mut nav_delete_path_for_handler = nav_delete_path;

            let closure = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                // -------------------------------------------------
                // Admin/search toggle shortcuts.
                //
                // Handle these before the form-field check so the
                // shortcuts work while the password box/search box
                // has focus.
                // -------------------------------------------------

                let key = event.key();

                // -----------------------------------------------------------------
                // MODAL KEYBOARD FOCUS TRAP
                // -----------------------------------------------------------------
                // All dashboard, login/logout, and history dialogs use the
                // `modal-window` class. The last matching element in document
                // order is the topmost modal (for example, Edit Row above the
                // URL-list dialog). Keep Tab and Shift+Tab inside that dialog.
                //
                // This lives in Navbar because Navbar stays mounted while the
                // dashboard switches between its main view and History.
                // Admin login/logout have their own deliberately tuned focus behavior.
                // Leave their Tab handling to the existing browser/modal behavior.
                if key == "Tab"
                    && !*login_open_for_handler.peek()
                    && !*logout_open_for_handler.peek()
                {
                    if let Ok(modals) = document_for_handler.query_selector_all(".modal-window") {
                        let modal = (0..modals.length())
                            .rev()
                            .find_map(|index| modals.item(index));

                        if let Some(modal_node) = modal {
                            // `NodeList::item()` returns a generic `Node`.
                            // `query_selector_all()` is defined on `Element`,
                            // so cast the modal node before searching inside it.
                            if let Ok(modal) = modal_node.dyn_into::<web_sys::Element>() {
                                if let Ok(focusable_nodes) = modal.query_selector_all(
                                    "a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])"
                                ) {
                                let mut focusable =
                                    Vec::<web_sys::HtmlElement>::new();

                                for index in 0..focusable_nodes.length() {
                                    if let Some(node) = focusable_nodes.item(index) {
                                        if let Ok(element) =
                                            node.dyn_into::<web_sys::HtmlElement>()
                                        {
                                            // The red X is a close control, not the first
                                            // keyboard destination. Exclude it from the focus
                                            // trap so a missed autofocus cannot send the first
                                            // Tab to the top-right corner of the modal.
                                            let tag = element.tag_name().to_ascii_lowercase();
                                            let text = element.text_content().unwrap_or_default();
                                            let is_close = tag == "button"
                                                && (matches!(text.trim(), "×" | "✕" | "✖" | "X" | "x")
                                                    || element.get_attribute("aria-label").as_deref() == Some("Close")
                                                    || element.get_attribute("title").as_deref() == Some("Close"));
                                            if !is_close {
                                                focusable.push(element);
                                            }
                                        }
                                    }
                                }

                                if !focusable.is_empty() {
                                    let active = document_for_handler.active_element();
                                    let current_index = active.as_ref().and_then(|active| {
                                        if !modal.contains(Some(active)) {
                                            return None;
                                        }

                                        focusable.iter().position(|element| {
                                            element.is_same_node(Some(active))
                                        })
                                    });

                                    // Let the browser perform ordinary Tab/Shift+Tab navigation.
                                    // Only intervene at the two ends to keep focus inside the
                                    // topmost dialog. This preserves true DOM order (for example,
                                    // Display name -> URL -> Cancel -> Save) instead of manually
                                    // stepping through controls that may be conditionally hidden.
                                    let wrap_to = match (current_index, event.shift_key()) {
                                        (Some(0), true) => Some(focusable.len() - 1),
                                        (Some(index), false) if index + 1 == focusable.len() => Some(0),
                                        (None, true) => Some(focusable.len() - 1),
                                        (None, false) => Some(0),
                                        _ => None,
                                    };

                                    if let Some(next_index) = wrap_to {
                                        event.prevent_default();
                                        let _ = focusable[next_index].focus();
                                        return;
                                    }
                                }
                                }
                            }
                        }
                    }
                }

                // Escape closes the topmost navbar dialog first. This check is
                // deliberately before form-field handling so it works while the
                // name or URL input has focus. Closing the item editor leaves the
                // manager draft intact; it does not discard the manager itself.
                if key == "Escape" && *nav_form_open_for_handler.peek() {
                    event.prevent_default();
                    nav_form_open_for_handler.set(false);
                    return;
                }

                if key == "Escape" && nav_delete_path_for_handler.peek().is_some() {
                    event.prevent_default();
                    nav_delete_path_for_handler.set(None);
                    return;
                }

                if key == "Escape" && nav_move_path_for_handler.peek().is_some() {
                    event.prevent_default();
                    nav_move_path_for_handler.set(None);
                    return;
                }

                if key == "Escape" && *nav_manager_open_for_handler.peek() {
                    event.prevent_default();
                    nav_manager_open_for_handler.set(false);
                    return;
                }

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

                // Do not let the Admin Login/Logout shortcut steal a
                // backslash typed into URL names, secrets, or other edit fields.
                // These IDs exist only while the corresponding edit modal is open.
                // Japanese Mac keyboards may report the physical backslash key as
                // either "\\" or "¥". Treat both spellings identically.
                let is_backslash_shortcut =
                    key == "\\" || key == "¥" || key == "￥" || event.code() == "Backslash";

                // While any editor modal is mounted, let the character pass through
                // normally. Clone and Move URL controls live inside these same editor
                // modals, and the Create Button modal is covered by its own ID.
                if is_backslash_shortcut
                    && (document_for_handler
                        .get_element_by_id("row_editor_modal")
                        .is_some()
                        || document_for_handler
                            .get_element_by_id("button_editor_modal")
                            .is_some()
                        || document_for_handler
                            .get_element_by_id("tag_manager_modal")
                            .is_some()
                        || document_for_handler
                            .get_element_by_id("nav_manager_modal")
                            .is_some()
                        || document_for_handler
                            .get_element_by_id("nav_editor_modal")
                            .is_some()
                        || document_for_handler
                            .get_element_by_id("nav_delete_modal")
                            .is_some()
                        || document_for_handler
                            .get_element_by_id("nav_move_modal")
                            .is_some())
                {
                    return;
                }

                if is_backslash_shortcut {
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
    //
    // Matching order (any of these makes the row a hit):
    //   1. button name
    //   2. hashtags
    //   3. URL / row name
    //
    // After collecting hits we sort the list by:
    //   1. button name  (primary)
    //   2. hashtags     (secondary – joined tag string)
    //   3. URL name     (tertiary)
    // -------------------------------------------------------------------------

    let mut matches: Vec<SearchResult> = if normalized_query.is_empty() {
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
                            // 1. Button name match
                            let button_matches =
                                button.name.to_lowercase().contains(&normalized_query);

                            // 2. URL / row name match
                            let name_matches = name.to_lowercase().contains(&normalized_query);

                            // 3. Hashtag match (strip leading '#' before comparing)
                            let matched_tags: Vec<String> = hashtag_tokens
                                .iter()
                                .filter(|tag| {
                                    tag.to_lowercase()
                                        .trim_start_matches('#')
                                        .contains(&normalized_query)
                                })
                                .cloned()
                                .collect();

                            // Keep the row only when at least one of the three
                            // criteria matches.
                            if !button_matches && !name_matches && matched_tags.is_empty() {
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

                                is_nav_item: false,
                            })
                        }
                    }
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    // Search navbar items too. Flattening is recursive, so items at any
    // supported nesting level can be found by name or URL.
    if !normalized_query.is_empty() {
        fn collect_nav_hits(items: &[NavItem], query: &str, hits: &mut Vec<SearchResult>) {
            for item in items {
                let url = item.url.clone().unwrap_or_default();
                if item.name.to_lowercase().contains(query) || url.to_lowercase().contains(query) {
                    hits.push(SearchResult {
                        button_name: "Navbar".to_string(),
                        row_name: item.name.clone(),
                        url: url.clone(),
                        comments: if url.is_empty() {
                            "Navbar menu item".to_string()
                        } else {
                            url
                        },
                        all_tags: Vec::new(),
                        matched_tags: Vec::new(),
                        has_secret: false,
                        secret_text: None,
                        is_nav_item: true,
                    });
                }
                collect_nav_hits(&item.children, query, hits);
            }
        }
        collect_nav_hits(&items, &normalized_query, &mut matches);
    }

    // Sort: button name → hashtags → URL / row name
    matches.sort_by(|a, b| {
        a.button_name
            .to_lowercase()
            .cmp(&b.button_name.to_lowercase())
            .then_with(|| {
                let a_tags = a.all_tags.join(" ").to_lowercase();
                let b_tags = b.all_tags.join(" ").to_lowercase();
                a_tags.cmp(&b_tags)
            })
            .then_with(|| a.row_name.to_lowercase().cmp(&b.row_name.to_lowercase()))
    });

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
    let items_for_manager_button = items.clone();
    let items_for_desktop_edit = items.clone();
    let items_for_mobile_edit = items.clone();

    // URLs represented in both Buttonboard and the navbar are marked for admins.
    let mut duplicate_nav_urls: Vec<String> = Vec::new();
    if let Some(dashboard) = dashboard.as_ref() {
        for button in &dashboard.buttons {
            for row in &button.rows {
                if let ButtonRowView::Link { url, .. } = row {
                    if !url.is_empty() && !duplicate_nav_urls.contains(url) {
                        duplicate_nav_urls.push(url.clone());
                    }
                }
            }
        }
    }

    rsx! {

        nav {
            class: if auth.is_admin() {
                "relative bg-gray-950 ring-2 ring-inset ring-red-500/70 shadow-[inset_0_-3px_0_rgba(239,68,68,0.35)] p-4 flex items-center gap-2"
            } else {
                "relative bg-gray-900 p-4 flex items-center gap-2"
            },


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

                for (index, item) in items.iter().enumerate() {
                    {
                        let items_for_this_edit = items_for_desktop_edit.clone();
                        rsx! {
                            li {
                                class: if index == 0 { "" } else { "max-[430px]:hidden" },
                                NavEntry {
                                    item: item.clone(),
                                    is_brand: index == 0,
                                    is_admin: auth.is_admin(),
                                    duplicate_urls: duplicate_nav_urls.clone(),
                                    // New**: pass shared close tick so outside-click can close this menu
                                    close_tick: nav_close_tick,
                                    on_edit: move |item: NavItem| {
                                        let tree = items_for_this_edit.clone();
                                        let url = item.url.clone().unwrap_or_default();
                                        if let Some(path) = nav_find_path(&tree, &item.name, &url, &mut Vec::new()) {
                                            nav_tree_draft.set(tree.clone());
                                            nav_form_path.set(Some(path));
                                            nav_form_parent.set(Vec::new());
                                            nav_form_name.set(item.name.clone());
                                            nav_form_url.set(url);
                                            nav_form_open_in_new_window.set(item.open_in_new_window);
                                            nav_manager_error.set(None);
                                            nav_manager_open.set(true);
                                            nav_form_open.set(true);
                                        }
                                    },
                                    depth: 0,
                                    parent_pin_hold: None,
                                }
                            }
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

                    for (index, item) in items.iter().enumerate() {
                        if index != 0 {
                            {
                                let items_for_this_edit = items_for_mobile_edit.clone();
                                rsx! {
                                    div {
                                        class: "px-2 py-2 hover:bg-gray-800 rounded-md",
                                        onclick: move |_| mobile_menu_open.set(false),
                                        NavEntry {
                                            item: item.clone(),
                                            is_brand: false,
                                            is_admin: auth.is_admin(),
                                            duplicate_urls: duplicate_nav_urls.clone(),
                                            // New**: pass shared close tick
                                            close_tick: nav_close_tick,
                                            on_edit: move |item: NavItem| {
                                                let tree = items_for_this_edit.clone();
                                                let url = item.url.clone().unwrap_or_default();
                                                if let Some(path) = nav_find_path(&tree, &item.name, &url, &mut Vec::new()) {
                                                    nav_tree_draft.set(tree.clone());
                                                    nav_form_path.set(Some(path));
                                                    nav_form_parent.set(Vec::new());
                                                    nav_form_name.set(item.name.clone());
                                                    nav_form_url.set(url);
                                                    nav_form_open_in_new_window.set(item.open_in_new_window);
                                                    nav_manager_error.set(None);
                                                    nav_manager_open.set(true);
                                                    nav_form_open.set(true);
                                                }
                                            },
                                            depth: 0,
                                            parent_pin_hold: None,
                                        }
                                    }
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

                        // autofocus can be lost when the server-rendered page is
                        // hydrated by WASM, so request focus again on client mount.
                        autofocus: true,
                        onmounted: move |event| {
                            let _ = event.data().set_focus(true);
                        },

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

                            // Only reset the highlighted result when the search
                            // box is being opened for a new search interaction.
                            //
                            // IMPORTANT: Enter and middle-click deliberately
                            // return focus to this input while keeping the
                            // dropdown open.  Resetting here unconditionally
                            // would move the highlight back to row 0.
                            if !search_open() {
                                selected_index.set(0);
                            }

                            search_open.set(true);


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


                                    // Each `move` event handler needs its own owned copy of
                                    // the URL. `String` is not `Copy`, so the middle-click
                                    // handler cannot move `result.url` and leave it for the
                                    // normal click handler.
                                    // Use a real link rather than `window.open`.
                                    // Native anchor middle-click behavior lets the browser
                                    // open a background tab without switching away from this tab.
                                    let result_url = if result.url.is_empty() { "#".to_string() } else { result.url.clone() };
                                    let items_for_result_edit = items.clone();

                                    rsx! {

                                        a {
                                            id:
                                                "search-result-{index}",

                                            href: result_url.clone(),
                                            target: "_blank",
                                            rel: "noopener noreferrer",

                                            class:
                                                if is_selected {

                                                    "group relative block w-full
                                                     border-b
                                                     border-gray-100
                                                     bg-blue-50
                                                     px-4 py-3 pr-12
                                                     text-left
                                                     text-blue-800"

                                                } else {

                                                    "group relative block w-full
                                                     border-b
                                                     border-gray-100
                                                     px-4 py-3 pr-12
                                                     text-left
                                                     text-gray-800
                                                     hover:bg-blue-50
                                                     hover:text-blue-800"
                                                },


                                            // A native anchor provides the browser's normal
                                            // link behavior: middle-click opens a background tab,
                                            // while Ctrl/Cmd-click and other link gestures also work.
                                            // Do not call `window.open` here, because that usually
                                            // switches focus to the newly opened tab.
                                            onclick: move |_| {
                                                selected_index.set(index);

                                                // Clear the query after an ordinary click. The
                                                // browser follows the anchor's href in a new tab.
                                                search.set(String::new());
                                                browser_storage_remove(SEARCH_STORAGE_KEY);
                                                search_open.set(false);
                                                selected_index.set(0);
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
                                                    class: "pointer-events-auto absolute right-3 top-1/2 z-20 -translate-y-1/2 cursor-pointer rounded px-1.5 py-0.5 text-gray-400 opacity-0 transition-opacity hover:bg-gray-200 hover:text-gray-700 group-hover:opacity-100",
                                                    title: if result.is_nav_item { "Edit this navbar item" } else { "Edit this URL row" },
                                                    onclick: move |event| {
                                                        event.stop_propagation();
                                                        event.prevent_default();

                                                        if result.is_nav_item {
                                                            nav_tree_draft.set(items_for_result_edit.clone());
                                                            nav_manager_error.set(None);
                                                            nav_manager_status.set(None);
                                                            nav_manager_open.set(true);
                                                            if let Some(path) = nav_find_path(&items_for_result_edit, &result.row_name, &result.url, &mut Vec::new()) {
                                                                if let Some(item) = nav_item_at_path(&items_for_result_edit, &path) {
                                                                    nav_form_path.set(Some(path));
                                                                    nav_form_parent.set(Vec::new());
                                                                    nav_form_name.set(item.name.clone());
                                                                    nav_form_url.set(item.url.clone().unwrap_or_default());
                                                                    nav_form_open_in_new_window.set(item.open_in_new_window);
                                                                    nav_form_open.set(true);
                                                                }
                                                            }
                                                        } else {
                                                            *EDIT_ROW_REQUEST.write() = Some((
                                                                result.button_name.clone(),
                                                                result.row_name.clone(),
                                                            ));
                                                        }

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
                            class: "rounded-md border border-indigo-500/70 px-3 py-2 text-sm text-indigo-100 transition hover:bg-indigo-900",
                            title: "Add, edit, remove, or nest navbar items (up to five levels).",
                            onclick: move |_| {
                                nav_tree_draft.set(items_for_manager_button.clone());
                                nav_manager_error.set(None);
                                nav_manager_status.set(None);
                                nav_form_open.set(false);
                                nav_move_path.set(None);
                                nav_manager_open.set(true);
                            },
                            "Manage Navbar"
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
        // NAVBAR CRUD / TREE EDITOR
        // ---------------------------------------------------------------------
        // Every item is edited with normal form fields. The recursive tree list
        // offers the same familiar pencil/clone/move/delete actions as Buttonboard.
        if nav_manager_open() {
            div {
                id: "nav_manager_modal",
                class: "fixed inset-0 z-[70] flex items-center justify-center bg-black/50 p-4",
                // Match the Buttonboard editors: clicking the backdrop must not
                // silently discard a draft. Use Close or Cancel intentionally.
                onclick: move |_| {},
                div {
                    class: "modal-window flex max-h-[90vh] w-full max-w-4xl flex-col rounded-lg bg-white p-5 shadow-xl [&_label]:block [&_label]:w-full [&_label]:min-w-0 [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:box-border [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:max-w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:min-w-0 [&_textarea]:box-border [&_textarea]:block [&_textarea]:w-full [&_textarea]:max-w-full [&_textarea]:min-w-0 [&_select]:box-border [&_select]:w-full [&_select]:max-w-full [&_select]:min-w-0",
                    style: "resize: both; overflow: auto; min-width: min(20rem, 92vw); min-height: 18rem;",
                    onclick: move |event| event.stop_propagation(),
                    div { class: "flex items-start justify-between gap-3",
                        div {
                            h2 { class: "text-lg font-semibold text-gray-900", "Manage Navbar" }
                            p { class: "mt-1 text-xs text-gray-500", "▸ indicates nested menu items; ↗ indicates a direct URL. Expand a row to manage its children." }
                            p { class: "mt-1 text-sm text-gray-600", "Edit links and dropdowns with forms. Use Add child to create nested entries (up to five levels)." }
                        }
                        button { class: "rounded-md px-2 py-1 text-xl text-gray-500 hover:bg-gray-100", title: "Close", onclick: move |_| nav_manager_open.set(false), "×" }
                    }
                    if let Some(message) = nav_manager_error() {
                        p { class: "mt-2 rounded-md bg-red-50 px-3 py-2 text-sm text-red-700", "{message}" }
                    }
                    if let Some(message) = nav_manager_status() {
                        p { class: "mt-2 rounded-md bg-green-50 px-3 py-2 text-sm text-green-700", "{message}" }
                    }
                    div { class: "mt-4 flex flex-wrap items-center justify-between gap-2 border-b border-gray-200 pb-3",
                        p { class: "text-xs text-gray-600", "▸ Menu with children · ↗ Direct URL · A menu can contain both." }
                        button { class: "rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-semibold text-gray-800 hover:bg-gray-100", onclick: move |_| nav_tree_collapsed.set(!nav_tree_collapsed()), if nav_tree_collapsed() { "Expand all ▾" } else { "Collapse all ▸" } }
                        button {
                            class: "rounded-md bg-blue-600 px-3 py-2 text-sm font-semibold text-white hover:bg-blue-700",
                            onclick: move |_| {
                                nav_form_path.set(None);
                                nav_form_parent.set(Vec::new());
                                nav_form_name.set(String::new());
                                nav_form_url.set(String::new());
                                nav_form_open_in_new_window.set(false);
                                nav_form_open.set(true);
                                nav_manager_error.set(None);
                            },
                            "+ Add top-level link"
                        }
                    }
                    div { class: "mt-3 min-h-0 flex-1 overflow-auto rounded-lg border border-gray-200 bg-gray-50 p-2",
                        if nav_tree_draft().is_empty() {
                            div { class: "p-6 text-center text-sm text-gray-500", "No navbar entries yet. Add your first top-level link." }
                        } else {
                            for (index, item) in nav_tree_draft().iter().enumerate() {
                                // The first root item is the fixed Admindash3 brand.
                                // It remains in data_navbar.v2.json, but is intentionally
                                // omitted from this editor so it cannot be moved/deleted.
                                if index > 0 {
                                NavManageEntry {
                                    key: "nav-manager-{index}-{nav_tree_collapsed()}",
                                    item: item.clone(),
                                    path: vec![index],
                                    depth: 0,
                                    collapse_all: nav_tree_collapsed(),
                                    on_edit: move |path: Vec<usize>| {
                                        let tree = nav_tree_draft();
                                        if let Some(item) = nav_item_at_path(&tree, &path) {
                                            nav_form_path.set(Some(path));
                                            nav_form_parent.set(Vec::new());
                                            nav_form_name.set(item.name.clone());
                                            nav_form_url.set(item.url.clone().unwrap_or_default());
                                            nav_form_open_in_new_window.set(item.open_in_new_window);
                                            nav_form_open.set(true);
                                            nav_manager_error.set(None);
                                        }
                                    },
                                    on_add_child: move |path: Vec<usize>| {
                                        if path.len() >= 5 {
                                            nav_manager_error.set(Some("Navbar nesting is limited to five levels.".to_string()));
                                        } else {
                                            nav_form_path.set(None);
                                            nav_form_parent.set(path);
                                            nav_form_name.set(String::new());
                                            nav_form_url.set(String::new());
                                            nav_form_open_in_new_window.set(false);
                                            nav_form_open.set(true);
                                            nav_manager_error.set(None);
                                        }
                                    },
                                    on_clone: move |path: Vec<usize>| {
                                        let mut tree = nav_tree_draft();
                                        if let Some(mut cloned) = nav_item_at_path(&tree, &path).cloned() {
                                            cloned.name.push_str(" (copy)");
                                            if let Some(siblings) = nav_siblings_mut(&mut tree, &path) {
                                                let index = *path.last().unwrap_or(&0);
                                                let insert_at = (index + 1).min(siblings.len());
                                                siblings.insert(insert_at, cloned);
                                                nav_tree_draft.set(tree);
                                                nav_manager_status.set(Some("Navbar item cloned. Remember to save your changes.".to_string()));
                                            }
                                        }
                                    },
                                    on_delete: move |path: Vec<usize>| {
                                        nav_delete_path.set(Some(path));
                                        nav_manager_error.set(None);
                                    },
                                    on_move: move |path: Vec<usize>| {
                                        nav_move_path.set(Some(path));
                                        nav_manager_error.set(None);
                                    },
                                    on_move_up: move |path: Vec<usize>| {
                                        let mut tree = nav_tree_draft();
                                        let index = *path.last().unwrap_or(&0);
                                        if index > 0 {
                                            if let Some(siblings) = nav_siblings_mut(&mut tree, &path) {
                                                siblings.swap(index, index - 1);
                                                nav_tree_draft.set(tree);
                                            }
                                        }
                                    },
                                    on_move_down: move |path: Vec<usize>| {
                                        let mut tree = nav_tree_draft();
                                        let index = *path.last().unwrap_or(&0);
                                        if let Some(siblings) = nav_siblings_mut(&mut tree, &path) {
                                            if index + 1 < siblings.len() {
                                                siblings.swap(index, index + 1);
                                                nav_tree_draft.set(tree);
                                            }
                                        }
                                    },
                                }
                                }
                            }
                        }
                    }
                    div { class: "mt-4 flex flex-wrap justify-end gap-2 border-t border-gray-200 pt-3",
                        button { class: "rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50", onclick: move |_| nav_manager_open.set(false), "Cancel" }
                        button {
                            class: "rounded-md bg-blue-600 px-4 py-2 text-sm font-semibold text-white hover:bg-blue-700",
                            onclick: move |_| async move {
                                nav_manager_error.set(None);
                                nav_manager_status.set(None);
                                let draft = nav_tree_draft();
                                if let Err(error) = crate::data_navbar::validate_nav_items(&draft) {
                                    nav_manager_error.set(Some(error));
                                } else {
                                    match crate::save_navbar(draft, ADMIN_AUTH()).await {
                                        Ok(()) => {
                                            nav_manager_status.set(Some("Navbar saved. Refreshing the navbar…".to_string()));
                                            *DASHBOARD_REFRESH_KEY.write() += 1;
                                            #[cfg(target_arch = "wasm32")]
                                            if let Some(win) = window() { let _ = win.location().reload(); }
                                        }
                                        Err(error) => nav_manager_error.set(Some(error.to_string())),
                                    }
                                }
                            },
                            "Save Navbar"
                        }
                    }
                }
            }
        }

        // Standard Add/Edit modal: no raw JSON required.
        if nav_form_open() {
            div {
                id: "nav_editor_modal",
                class: "fixed inset-0 z-[80] flex items-center justify-center bg-black/50 p-4",
                // Don't lose typed text from an accidental outside click.
                onclick: move |_| {},
                div {
                    class: "modal-window relative w-full max-w-xl rounded-lg bg-white p-6 shadow-xl [&_label]:block [&_label]:w-full [&_label]:min-w-0 [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:box-border [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:max-w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:min-w-0 [&_textarea]:box-border [&_textarea]:block [&_textarea]:w-full [&_textarea]:max-w-full [&_textarea]:min-w-0 [&_select]:box-border [&_select]:w-full [&_select]:max-w-full [&_select]:min-w-0",
                    style: "resize: both; overflow: auto; min-width: min(18rem, 92vw); max-height: 90vh;",
                    onclick: move |event| event.stop_propagation(),
                    div { class: "flex items-center justify-between gap-3",
                        h2 { class: "text-lg font-semibold text-gray-900", if nav_form_path().is_some() { "Edit Navbar Item" } else { "Add Navbar Item" } }
                        button { class: "rounded px-2 py-1 text-xl text-gray-500 hover:bg-gray-100", onclick: move |_| nav_form_open.set(false), "×" }
                    }
                    p { class: "mt-1 text-sm text-gray-600", "A link can have a URL, child items, or both. Leave URL blank for a dropdown-only heading." }
                    label { class: "mt-4 block text-sm font-medium text-gray-700", "Display name" }
                    input {
                        autofocus: true,
                        class: if nav_form_name().trim().is_empty() { "mt-1 w-full rounded-md border-2 border-red-500 bg-red-50 px-3 py-2 text-sm text-red-700 focus:border-red-600 focus:outline-none focus:ring-2 focus:ring-red-500/20" } else { "mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20" },
                        required: true,
                        value: nav_form_name(),
                        placeholder: "e.g. Internal Wiki",
                        oninput: move |event| nav_form_name.set(event.value()),
                    }
                    label { class: "mt-3 block text-sm font-medium text-gray-700", "URL (optional)" }
                    input {
                        class: "mt-1 w-full rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-900 focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500/20",
                        value: nav_form_url(),
                        placeholder: "https://wiki.example.local/ or /",
                        oninput: move |event| nav_form_url.set(event.value()),
                    }
                    label {
                        class: "mt-4 flex items-start gap-2 text-sm text-gray-700",
                        input {
                            r#type: "checkbox",
                            class: "mt-0.5 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500",
                            checked: nav_form_open_in_new_window(),
                            // Only meaningful when a URL is set; still allow toggling
                            // so the preference is ready if the user adds a URL next.
                            oninput: move |event| nav_form_open_in_new_window.set(event.checked()),
                        }
                        span {
                            class: "leading-5",
                            "Open in new window"
                            span {
                                class: "mt-0.5 block text-xs font-normal text-gray-500",
                                "When checked, clicking this link opens a new browser tab instead of the dashboard iframe. Ignored if URL is blank."
                            }
                        }
                    }
                    div { class: "mt-5 flex flex-wrap justify-end gap-2",
                        button { class: "rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50", onclick: move |_| nav_form_open.set(false), "Cancel" }
                        button {
                            class: if nav_form_name().trim().is_empty() { "rounded-md bg-gray-400 px-4 py-2 text-sm font-semibold text-white cursor-not-allowed" } else { "rounded-md bg-blue-600 px-4 py-2 text-sm font-semibold text-white hover:bg-blue-700" },
                            disabled: nav_form_name().trim().is_empty(),
                            onclick: move |_| {
                                let name = nav_form_name().trim().to_string();
                                let url = nav_form_url().trim().to_string();
                                if name.is_empty() {
                                    nav_manager_error.set(Some("Please enter a display name.".to_string()));
                                    return;
                                }
                                let mut tree = nav_tree_draft();
                                let open_in_new_window = nav_form_open_in_new_window() && !url.is_empty();
                                let new_item = NavItem {
                                    name,
                                    url: if url.is_empty() { None } else { Some(url) },
                                    open_in_new_window,
                                    children: Vec::new(),
                                };
                                if let Some(path) = nav_form_path() {
                                    if let Some(item) = nav_item_at_path_mut(&mut tree, &path) {
                                        item.name = new_item.name;
                                        item.url = new_item.url;
                                        item.open_in_new_window = new_item.open_in_new_window;
                                    } else {
                                        nav_manager_error.set(Some("That navbar item no longer exists. Please reopen the editor.".to_string()));
                                        return;
                                    }
                                } else if !nav_insert_under_path(&mut tree, &nav_form_parent(), new_item) {
                                    nav_manager_error.set(Some("The selected parent no longer exists.".to_string()));
                                    return;
                                }
                                if let Err(error) = crate::data_navbar::validate_nav_items(&tree) {
                                    nav_manager_error.set(Some(error));
                                    return;
                                }
                                nav_tree_draft.set(tree);
                                nav_form_open.set(false);
                                nav_manager_error.set(None);
                                nav_manager_status.set(Some("Draft updated. Click Save Navbar to apply changes.".to_string()));
                            },
                            "Save Item"
                        }
                    }
                }
            }
        }

        // Confirm before deleting: a parent may contain several nested links.
        if let Some(delete_path) = nav_delete_path() {
            div {
                id: "nav_delete_modal",
                class: "fixed inset-0 z-[90] flex items-center justify-center bg-black/50 p-4",
                onclick: move |_| nav_delete_path.set(None),
                div {
                    class: "modal-window w-full max-w-md rounded-lg bg-white p-5 shadow-xl [&_label]:block [&_label]:w-full [&_label]:min-w-0 [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:box-border [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:max-w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:min-w-0 [&_textarea]:box-border [&_textarea]:block [&_textarea]:w-full [&_textarea]:max-w-full [&_textarea]:min-w-0 [&_select]:box-border [&_select]:w-full [&_select]:max-w-full [&_select]:min-w-0",
                    onclick: move |event| event.stop_propagation(),
                    h2 { class: "text-lg font-semibold text-gray-900", "Delete Navbar Item?" }
                    p { class: "mt-2 text-sm text-gray-600", "This removes the selected item and all of its nested children from the draft. Nothing is written to disk until you click Save Navbar." }
                    div { class: "mt-5 flex justify-end gap-2",
                        button { class: "rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50", onclick: move |_| nav_delete_path.set(None), "Cancel" }
                        button {
                            class: "rounded-md bg-red-600 px-4 py-2 text-sm font-semibold text-white hover:bg-red-700",
                            onclick: move |_| {
                                let mut tree = nav_tree_draft();
                                if nav_remove_at_path(&mut tree, &delete_path).is_some() {
                                    nav_tree_draft.set(tree);
                                    nav_manager_status.set(Some("Navbar item removed from the draft. Save to apply it.".to_string()));
                                }
                                nav_delete_path.set(None);
                            },
                            "Delete Item"
                        }
                    }
                }
            }
        }

        // Move modal: choose a destination parent, or move back to the top level.
        if let Some(source_path) = nav_move_path() {
            div {
                id: "nav_move_modal",
                class: "fixed inset-0 z-[90] flex items-center justify-center bg-black/50 p-4",
                onclick: move |_| nav_move_path.set(None),
                div {
                    class: "modal-window w-full max-w-lg rounded-lg bg-white p-5 shadow-xl [&_label]:block [&_label]:w-full [&_label]:min-w-0 [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:box-border [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:max-w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:min-w-0 [&_textarea]:box-border [&_textarea]:block [&_textarea]:w-full [&_textarea]:max-w-full [&_textarea]:min-w-0 [&_select]:box-border [&_select]:w-full [&_select]:max-w-full [&_select]:min-w-0",
                    onclick: move |event| event.stop_propagation(),
                    h2 { class: "text-lg font-semibold text-gray-900", "Move Navbar Item" }
                    p { class: "mt-1 text-sm text-gray-600", "Choose a destination. Top level keeps the item in the main navbar; selecting a menu nests it underneath that menu. The item and all its children move together." }
                    button { class: "mt-3 rounded-md border border-gray-300 bg-white px-3 py-2 text-sm font-semibold text-gray-800 hover:bg-gray-100", onclick: move |_| nav_move_tree_collapsed.set(!nav_move_tree_collapsed()), if nav_move_tree_collapsed() { "Show destination menus ▾" } else { "Hide destination menus ▴" } }
                    if !nav_move_tree_collapsed() {
                    div { class: "mt-4 max-h-64 space-y-1 overflow-auto",
                        {
                            let source_path_top = source_path.clone();
                            rsx! {
                                button {
                                    class: "block w-full rounded-md border border-gray-200 px-3 py-2 text-left text-sm hover:bg-blue-50",
                                    onclick: move |_| {
                                        let mut tree = nav_tree_draft();
                                        if let Some(item) = nav_remove_at_path(&mut tree, &source_path_top) {
                                            tree.push(item);
                                            nav_tree_draft.set(tree);
                                            nav_move_path.set(None);
                                            nav_manager_status.set(Some("Item moved to top level. Save to apply changes.".to_string()));
                                        }
                                    },
                                    "Top level"
                                }
                            }
                        }
                        {
                            let tree_for_targets = nav_tree_draft();
                            let mut targets = Vec::new();
                            nav_collect_move_targets(&tree_for_targets, Vec::new(), &mut targets);
                            rsx! {
                                for (target_path, target_name) in targets.iter() {
                                    // Try the exact operation on a throwaway copy. This means
                                    // the menu only offers destinations that really work for
                                    // this item's subtree depth, not just destinations that
                                    // look valid from the target's depth alone.
                                    {
                                        let mut preview_tree = tree_for_targets.clone();
                                        let can_move_here = nav_move_item_under_target(
                                            &mut preview_tree,
                                            &source_path,
                                            target_path,
                                        ).is_ok();
                                        if can_move_here {
                                            let source_path_for_move = source_path.clone();
                                            let target_path_for_move = target_path.clone();
                                            let target_name_for_move = target_name.clone();
                                            rsx! {
                                                button {
                                                    class: "block w-full rounded-md border border-gray-200 px-3 py-2 text-left text-sm hover:bg-blue-50",
                                                    onclick: move |_| {
                                                        let mut tree = nav_tree_draft();
                                                        match nav_move_item_under_target(
                                                            &mut tree,
                                                            &source_path_for_move,
                                                            &target_path_for_move,
                                                        ) {
                                                            Ok(()) => {
                                                                nav_tree_draft.set(tree);
                                                                nav_move_path.set(None);
                                                                nav_manager_error.set(None);
                                                                nav_manager_status.set(Some("Item moved. Destination URL and existing children were preserved; the moved item was appended last. Save to apply changes.".to_string()));
                                                            }
                                                            Err(error) => nav_manager_error.set(Some(error)),
                                                        }
                                                    },
                                                    "Nest under {target_name_for_move}"
                                                }
                                            }
                                        } else {
                                            // Dioxus expects both branches of this expression
                                            // to produce a render node. This hidden span renders
                                            // nothing visible for destinations that are invalid.
                                            rsx! { span { class: "hidden", "" } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    }
                    div { class: "mt-4 flex justify-end",
                        button { class: "rounded-md border border-gray-300 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50", onclick: move |_| nav_move_path.set(None), "Cancel" }
                    }
                }
            }
        }

        // ---------------------------------------------------------------------
        // LOGOUT CONFIRMATION MODAL
        // ---------------------------------------------------------------------

        if logout_open() {
            div {
                class: "fixed inset-0 z-[60] flex items-center justify-center overflow-y-auto bg-black/50 p-2 sm:p-4",
                onclick: move |_| logout_open.set(false),

                div {
                    class: "modal-window w-full max-w-sm rounded-lg bg-white p-6 shadow-xl [&_label]:block [&_label]:w-full [&_label]:min-w-0 [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:box-border [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:max-w-full [&_input:not([type=checkbox]):not([type=radio]):not([type=hidden])]:min-w-0 [&_textarea]:box-border [&_textarea]:block [&_textarea]:w-full [&_textarea]:max-w-full [&_textarea]:min-w-0 [&_select]:box-border [&_select]:w-full [&_select]:max-w-full [&_select]:min-w-0",
                    style: "resize: both; overflow: auto; min-width: min(18rem, 90vw); min-height: 10rem; max-height: 90vh;",
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
                        "modal-window w-full max-w-sm
                         rounded-lg
                         bg-white p-6
                         shadow-xl",
                    style: "resize: both; overflow: auto; min-width: min(18rem, 90vw); min-height: 10rem; max-height: 90vh;",

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
// RECURSIVE NAVBAR MANAGER ROW
// -----------------------------------------------------------------------------
#[component]
fn NavManageEntry(
    item: NavItem,
    path: Vec<usize>,
    depth: usize,
    collapse_all: bool,
    on_edit: EventHandler<Vec<usize>>,
    on_add_child: EventHandler<Vec<usize>>,
    on_clone: EventHandler<Vec<usize>>,
    on_delete: EventHandler<Vec<usize>>,
    on_move: EventHandler<Vec<usize>>,
    on_move_up: EventHandler<Vec<usize>>,
    on_move_down: EventHandler<Vec<usize>>,
) -> Element {
    // The parent includes `collapse_all` in each row key. When the global
    // toggle changes, rows remount and every nesting level starts in the same
    // requested state. This avoids effects that only updated the top row.
    let mut collapsed = use_signal(move || collapse_all);
    let edit_path = path.clone();
    let child_path = path.clone();
    let clone_path = path.clone();
    let delete_path = path.clone();
    let move_path = path.clone();
    let up_path = path.clone();
    let down_path = path.clone();
    rsx! {
        div { class: "mb-2 rounded-md border border-gray-200 bg-white shadow-sm",
            div { class: "flex flex-wrap items-center gap-2 p-3", style: "margin-left: {depth * 10}px;",
                div { class: "min-w-0 flex-1",
                    div { class: "truncate text-sm font-semibold text-gray-800", "{item.name}" }
                    div { class: "truncate text-xs text-gray-600", if let Some(url) = item.url.as_deref() { if item.open_in_new_window { "↗ {url} · new window" } else { "↗ {url}" } } else if !item.children.is_empty() { "▸ Menu · no direct URL" } else { "No URL" } }
                    if !item.children.is_empty() {
                        div { class: "mt-1 text-[11px] text-indigo-600", "{item.children.len()} child item(s)" }
                    }
                }
                div { class: "flex flex-wrap items-center gap-1",
                    button { class: "rounded border border-gray-200 px-2 py-1 text-xs text-gray-600 hover:bg-gray-100", title: "Move earlier", onclick: move |_| on_move_up.call(up_path.clone()), "↑" }
                    button { class: "rounded border border-gray-200 px-2 py-1 text-xs text-gray-600 hover:bg-gray-100", title: "Move later", onclick: move |_| on_move_down.call(down_path.clone()), "↓" }
                    button { class: "rounded border border-blue-200 px-2 py-1 text-xs text-blue-700 hover:bg-blue-50", title: "Add a nested item", onclick: move |_| on_add_child.call(child_path.clone()), "+ Child" }
                    button { class: "rounded border border-gray-300 bg-white px-2 py-1 text-xs font-medium text-gray-900 hover:bg-gray-100", title: "Edit navbar item", onclick: move |_| on_edit.call(edit_path.clone()), "✎ Edit" }
                    button { class: "rounded border border-gray-200 px-2 py-1 text-xs text-gray-600 hover:bg-gray-100", title: "Clone item and its children", onclick: move |_| on_clone.call(clone_path.clone()), "Clone" }
                    button { class: "rounded border border-indigo-200 px-2 py-1 text-xs text-indigo-700 hover:bg-indigo-50", title: "Move under another menu", onclick: move |_| on_move.call(move_path.clone()), "Move" }
                    button { class: "rounded border border-red-200 px-2 py-1 text-xs text-red-700 hover:bg-red-50", title: "Delete item and all its children", onclick: move |_| on_delete.call(delete_path.clone()), "Delete" }
                }
            }
            if !item.children.is_empty() {
                div { class: "border-t border-gray-100 bg-gray-50 p-2",
                    button { class: "mb-2 rounded border border-gray-300 bg-white px-2 py-1 text-xs font-medium text-gray-800 hover:bg-gray-100", onclick: move |_| collapsed.set(!collapsed()), if collapsed() { "Show {item.children.len()} child item(s) ▾" } else { "Hide child items ▴" } }
                    if !collapsed() {
                    for (index, child) in item.children.iter().enumerate() {
                        {
                            let mut child_path = path.clone();
                            child_path.push(index);
                            rsx! {
                                NavManageEntry {
                                    key: "nav-child-{depth}-{index}-{child.name}-{collapse_all}",
                                    item: child.clone(),
                                    path: child_path,
                                    depth: depth + 1,
                                    collapse_all: collapse_all,
                                    on_edit: on_edit,
                                    on_add_child: on_add_child,
                                    on_clone: on_clone,
                                    on_delete: on_delete,
                                    on_move: on_move,
                                    on_move_up: on_move_up,
                                    on_move_down: on_move_down,
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
// NAV ENTRY
// -----------------------------------------------------------------------------

#[component]
fn NavEntry(
    item: NavItem,
    is_brand: bool,
    is_admin: bool,
    duplicate_urls: Vec<String>,
    // Shared outside-click tick from Navbar — when it changes, menus reset.
    close_tick: Signal<u64>,
    on_edit: EventHandler<NavItem>,
    depth: usize,
    // Parent's `child_pin_hold` signal. Nested entries increment this while
    // pinned (or while they themselves hold a pinned descendant) so a
    // hover-only ancestor is not unmounted underneath them.
    parent_pin_hold: Option<Signal<usize>>,
) -> Element {
    // -----------------------------------------------------------------------
    // show_menu = pinned || hovering || child_pin_hold > 0
    //
    // pinned   – click toggle; survives mouse leave
    // hovering – mouseenter / delayed mouseleave
    // child_pin_hold – pinned descendants keep THIS row mounted
    //
    // The double-click / "click does nothing" bug: parent open only via hover,
    // user pins nested item, parent mouseleave/unmount tears the child down
    // and the pin is lost. child_pin_hold + parent_pin_hold fix that chain.
    // -----------------------------------------------------------------------
    let mut pinned = use_signal(|| false);
    let mut hovering = use_signal(|| false);
    let mut child_pin_hold = use_signal(|| 0usize);
    let mut leave_timer = use_signal(|| None::<i32>);

    let show_menu = pinned() || hovering() || child_pin_hold() > 0;
    let has_children = !item.children.is_empty();

    // Bubble "I have pinned descendants" to the parent once, not on every count.
    let mut prev_child_hold_nonzero = use_signal(|| false);
    use_effect(move || {
        let nonzero = child_pin_hold() > 0;
        let was = *prev_child_hold_nonzero.peek();
        if nonzero != was {
            prev_child_hold_nonzero.set(nonzero);
            if let Some(mut hold) = parent_pin_hold {
                if nonzero {
                    let next = (*hold.peek()).saturating_add(1);
                    hold.set(next);
                } else {
                    let next = (*hold.peek()).saturating_sub(1);
                    hold.set(next);
                }
            }
        }
    });

    // Outside click: hard reset.
    let mut last_close_tick = use_signal(|| 0u64);
    use_effect(move || {
        let tick = close_tick();
        if tick != *last_close_tick.peek() {
            last_close_tick.set(tick);
            if let Some(id) = leave_timer() {
                if let Some(win) = window() {
                    win.clear_timeout_with_handle(id);
                }
                leave_timer.set(None);
            }
            // If we were pinned, release the parent hold.
            if *pinned.peek() {
                if let Some(mut hold) = parent_pin_hold {
                    let next = (*hold.peek()).saturating_sub(1);
                                    hold.set(next);
                }
            }
            pinned.set(false);
            hovering.set(false);
            child_pin_hold.set(0);
        }
    });

    let item_for_brand_edit = item.clone();
    let item_for_leaf_edit = item.clone();
    let item_for_dropdown_edit = item.clone();
    let edit_handler_brand = on_edit.clone();
    let edit_handler_leaf = on_edit.clone();
    let edit_handler_dropdown = on_edit.clone();
    let open_in_new_window = item.open_in_new_window;

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
            class: if depth == 0 {
                "nav-entry-root group relative pr-5"
            } else {
                "nav-entry-root group relative w-full pr-5"
            },
            onmouseenter: move |_| {
                if let Some(id) = leave_timer() {
                    if let Some(win) = window() {
                        win.clear_timeout_with_handle(id);
                    }
                    leave_timer.set(None);
                }
                if has_children {
                    hovering.set(true);
                }
            },
            onmouseleave: move |_| {
                // Always clear the hover flag; pin / child holds keep the menu up.
                if let Some(id) = leave_timer() {
                    if let Some(win) = window() {
                        win.clear_timeout_with_handle(id);
                    }
                }
                // Short delay so the pointer can cross into a nested flyout
                // without dropping hovering before the child mouseenter runs.
                let mut hovering = hovering;
                let mut leave_timer = leave_timer;
                if let Some(win) = window() {
                    let callback = wasm_bindgen::closure::Closure::once(Box::new(move || {
                        hovering.set(false);
                        leave_timer.set(None);
                    }) as Box<dyn FnOnce()>);
                    if let Ok(id) = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                        callback.as_ref().unchecked_ref(),
                        200,
                    ) {
                        leave_timer.set(Some(id));
                    }
                    callback.forget();
                } else {
                    hovering.set(false);
                }
            },

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
                if is_admin {
                    span {
                        class: "absolute right-0 top-1/2 z-10 -translate-y-1/2 cursor-pointer rounded px-1 text-xs text-gray-300 opacity-0 transition hover:bg-gray-700 hover:text-white group-hover:opacity-100",
                        title: "Edit this navbar item",
                        onclick: move |event| {
                            event.stop_propagation();
                            event.prevent_default();
                            edit_handler_brand.call(item_for_brand_edit.clone());
                        },
                        "✎"
                    }
                }

            // -----------------------------------------------------------------
            // NORMAL LINK
            // -----------------------------------------------------------------
            } else if item.children.is_empty() {
                if let Some(url) = item.url.clone() {
                    a {
                        href: pending_href.unwrap_or_else(|| "#".to_string()),
                        class: if depth == 0 {
                            "text-white hover:text-blue-400"
                        } else {
                            "block w-full text-white hover:text-blue-400"
                        },
                        onclick: move |event| {
                            event.prevent_default();
                            if open_in_new_window {
                                if let Some(win) = window() {
                                    let _ = win.open_with_url_and_target(&url, "_blank");
                                }
                            } else {
                                *IFRAME_URL.write() = Some(url.clone());
                            }
                        },
                        "{item.name}"
                        if is_admin && item.url.as_deref().is_some_and(|url| duplicate_urls.iter().any(|candidate| candidate == url)) {
                            span {
                                class: "ml-1 text-[10px] font-semibold text-amber-300",
                                title: "This URL also appears on Buttonboard.",
                                "↔"
                            }
                        }
                    }
                } else {
                    a {
                        href: "#",
                        class: if depth == 0 {
                            "text-white hover:text-blue-400"
                        } else {
                            "block w-full text-white hover:text-blue-400"
                        },
                        "{item.name}"
                    }
                }
                if is_admin {
                    span {
                        class: "absolute right-0 top-1/2 z-10 -translate-y-1/2 cursor-pointer rounded px-1 text-xs text-gray-300 opacity-0 transition hover:bg-gray-700 hover:text-white group-hover:opacity-100",
                        title: "Edit this navbar item",
                        onclick: move |event| {
                            event.stop_propagation();
                            event.prevent_default();
                            edit_handler_leaf.call(item_for_leaf_edit.clone());
                        },
                        "✎"
                    }
                }

            // -----------------------------------------------------------------
            // DROPDOWN
            // -----------------------------------------------------------------
            } else {
                div {
                    class: if depth == 0 {
                        "inline-flex items-center gap-1 rounded-sm px-1 py-0.5 transition-colors hover:bg-gray-800"
                    } else {
                        "flex w-full items-center rounded-sm transition-colors"
                    },
                    button {
                        class: if depth == 0 {
                            "inline-flex items-center gap-2 rounded-sm text-left text-white hover:text-blue-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"
                        } else {
                            "flex w-full items-center justify-between gap-2 rounded-sm text-left text-white hover:text-blue-300 focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-400"
                        },
                        title: "Open or close submenu",
                        aria_expanded: "{show_menu}",
                        onclick: move |event| {
                            event.stop_propagation();
                            // Native listener on document also sees this click;
                            // stop it so close_tick does not clear the pin.
                            event.data().as_web_event().stop_propagation();
                            if let Some(id) = leave_timer() {
                                if let Some(win) = window() {
                                    win.clear_timeout_with_handle(id);
                                }
                                leave_timer.set(None);
                            }
                            if pinned() {
                                pinned.set(false);
                                if let Some(mut hold) = parent_pin_hold {
                                    let next = (*hold.peek()).saturating_sub(1);
                                    hold.set(next);
                                }
                            } else {
                                pinned.set(true);
                                hovering.set(true);
                                if let Some(mut hold) = parent_pin_hold {
                                    let next = (*hold.peek()).saturating_add(1);
                                    hold.set(next);
                                }
                            }
                        },
                        span { class: "min-w-0 truncate", "{item.name}" }
                        span {
                            class: "ml-1 inline-flex min-w-4 shrink-0 items-center justify-center text-sm font-bold text-blue-300",
                            title: "Opens a submenu",
                            if show_menu { "▾" } else { "›" }
                        }
                        if is_admin && item.url.as_deref().is_some_and(|url| duplicate_urls.iter().any(|candidate| candidate == url)) {
                            span {
                                class: "ml-1 text-[10px] font-semibold text-amber-300",
                                title: "This URL also appears on Buttonboard.",
                                "↔"
                            }
                        }
                    }
                }
                if is_admin {
                    span {
                        class: "absolute right-0 top-1/2 z-10 -translate-y-1/2 cursor-pointer rounded px-1 text-xs text-gray-300 opacity-0 transition hover:bg-gray-700 hover:text-white group-hover:opacity-100",
                        title: "Edit this navbar item",
                        onclick: move |event| {
                            event.stop_propagation();
                            event.prevent_default();
                            edit_handler_dropdown.call(item_for_dropdown_edit.clone());
                        },
                        "✎"
                    }
                }

                if show_menu {
                    ul {
                        class: if depth == 0 {
                            "absolute left-0 top-full w-56 rounded bg-gray-900 p-2 shadow-lg z-50 border border-gray-700"
                        } else {
                            "absolute left-full top-0 w-56 rounded bg-gray-900 p-2 shadow-lg z-50 border border-gray-700"
                        },
                        for (child_index, child) in item.children.clone().into_iter().enumerate() {
                            li {
                                key: "{depth}-{child_index}-{child.name}",
                                class: "relative rounded-md p-2 transition-colors hover:bg-gray-700 focus-within:bg-gray-700",
                                NavEntry {
                                    item: child,
                                    is_brand: false,
                                    is_admin,
                                    duplicate_urls: duplicate_urls.clone(),
                                    close_tick,
                                    on_edit,
                                    depth: depth + 1,
                                    // Children report pins into OUR hold counter.
                                    parent_pin_hold: Some(child_pin_hold),
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


