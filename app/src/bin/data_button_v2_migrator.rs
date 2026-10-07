use anyhow::{Context, Result};
use std::path::PathBuf;

#[allow(dead_code)]
#[path = "../dashboard_data.rs"]
mod dashboard_data;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);

    let input_path = args.next().context(
        "Usage: cargo run --bin data_button_v2_migrator -- <input data_buttons.json> [output data_buttons.v2.json]",
    )?;

    let output_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| default_output_path(&input_path));

    let migrated = dashboard_data::migrate_v1_file_to_v2_string(PathBuf::from(&input_path).as_path())?;

    std::fs::write(&output_path, migrated)
        .with_context(|| format!("Could not write {}", output_path.display()))?;

    println!(
        "Migrated {} -> {}",
        PathBuf::from(&input_path).display(),
        output_path.display()
    );

    Ok(())
}

fn default_output_path(input_path: &str) -> PathBuf {
    let input = PathBuf::from(input_path);
    let parent = input.parent().map(PathBuf::from).unwrap_or_default();
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("data_buttons");

    parent.join(format!("{stem}.v2.json"))
}
