use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// NAVBAR DATA STRUCTURE (v2)
// -----------------------------------------------------------------------------
//
// Live file: data_navbar.v2.json
//
// This represents ONE item from data_navbar.v2.json.
//
// Example JSON:
//
// {
//     "name": "CopyPasta",
//     "url": "http://cp.lm.local",
//     "open_in_new_window": false,
//     "children": []
// }
//
// `open_in_new_window` is new in v2. When true, clicking the item opens the URL
// in a new browser tab/window instead of loading it into the dashboard iframe.
// `#[serde(default)]` keeps older history snapshots (without the field)
// deserializable as `false`.
// -----------------------------------------------------------------------------

/// Live navbar JSON path used by load/save and history revert.
pub const NAVBAR_FILE: &str = "./data_navbar.v2.json";

/// Legacy v1 path (no `open_in_new_window`). Used only by the migrator binary.
pub const NAVBAR_V1_FILE: &str = "./data_navbar.json";

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct NavItem {
    pub name: String,
    pub url: Option<String>,
    /// When true, the live navbar opens this URL in a new browser window/tab.
    /// Default is false (iframe), matching v1 behaviour.
    #[serde(default)]
    pub open_in_new_window: bool,
    pub children: Vec<NavItem>,
}

// -----------------------------------------------------------------------------
// NAVBAR TREE VALIDATION
// -----------------------------------------------------------------------------
// The navbar supports up to five levels: top-level items are level 1.
// This is used by both the browser editor and the authenticated save endpoint.
pub fn validate_nav_items(items: &[NavItem]) -> Result<(), String> {
    fn validate_level(items: &[NavItem], depth: usize) -> Result<(), String> {
        if depth > 5 && !items.is_empty() {
            return Err("Navbar nesting is limited to five levels.".to_string());
        }
        for item in items {
            if item.name.trim().is_empty() {
                return Err("Every navbar item needs a name.".to_string());
            }
            if item.url.as_deref().is_some_and(|url| url.trim().is_empty()) {
                return Err(format!(
                    "Navbar item '{}' has a blank URL. Use null or remove the URL instead.",
                    item.name
                ));
            }
            // open_in_new_window only applies when there is a URL; allow the flag
            // on menu headings without error (it is simply ignored at click time).
            validate_level(&item.children, depth + 1)?;
        }
        Ok(())
    }
    validate_level(items, 1)
}

/// Walk a tree after deserializing v1 JSON (field missing → serde default false).
/// Exists so the migrator can document an explicit “ensure v2 shape” step.
pub fn ensure_v2_defaults(items: &mut [NavItem]) {
    for item in items {
        ensure_v2_defaults(&mut item.children);
    }
}
