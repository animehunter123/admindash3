use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// NAVBAR DATA STRUCTURE
// -----------------------------------------------------------------------------
//
// This represents ONE item from data_navbar.json.
//
// Example JSON:
//
// {
//     "name": "CopyPasta",
//     "url": "http://cp.lm.local",
//     "children": []
// }
// -----------------------------------------------------------------------------
#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct NavItem {
    pub name: String,
    pub url: Option<String>,
    pub children: Vec<NavItem>,
}

