use anyhow::{Context, Result};
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "../data_navbar.rs"]
mod data_navbar;

use data_navbar::{NAVBAR_FILE, NAVBAR_V1_FILE, NavItem};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);

    let input_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(NAVBAR_V1_FILE));

    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(NAVBAR_FILE));

    let raw = std::fs::read_to_string(&input_path)
        .with_context(|| format!("Could not read {}", input_path.display()))?;

    // Deserialize with the v2 struct: missing `open_in_new_window` becomes false
    // thanks to #[serde(default)].
    let mut items: Vec<NavItem> = serde_json::from_str(&raw)
        .with_context(|| format!("Could not parse {} as navbar JSON", input_path.display()))?;

    data_navbar::ensure_v2_defaults(&mut items);
    data_navbar::validate_nav_items(&items).map_err(anyhow::Error::msg)?;

    let pretty =
        serde_json::to_string_pretty(&items).context("Could not serialize v2 navbar JSON")?;
    std::fs::write(&output_path, format!("{pretty}\n"))
        .with_context(|| format!("Could not write {}", output_path.display()))?;

    println!(
        "Migrated {} -> {} ({} top-level items; open_in_new_window defaults to false)",
        input_path.display(),
        output_path.display(),
        items.len()
    );

    Ok(())
}
