use anyhow::{Context, Result, anyhow, bail};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

// -----------------------------------------------------------------------------
// FILES
// -----------------------------------------------------------------------------

pub const BUTTONS_V1_FILE: &str = "./data_buttons.json";
pub const BUTTONS_V3_FILE: &str = "./data_buttons.v3.json";
pub const TAGS_FILE: &str = "./data_tags.json";
pub const SYSTEM_PREFERENCES_FILE: &str = "./system_preferences.json";
pub const HISTORY_DIR: &str = "./dashboard_history";
pub const MAX_HISTORY_SNAPSHOTS: usize = 100;

// -----------------------------------------------------------------------------
// ADMIN CREDENTIALS
// -----------------------------------------------------------------------------

pub const ADMIN_USERNAME: &str = "admin";
pub const ADMIN_PASSWORD: &str = "admin";

// Runtime admin credentials.
//
// Docker Compose can override these with:
//
//     ADMINUSERNAME=someuser
//     ADMINPASSWORD=somepassword
//
// If either variable is missing or empty, the simple development default
// remains admin/admin.  We intentionally are not hashing credentials yet.
#[cfg(not(target_arch = "wasm32"))]
pub fn configured_admin_username() -> String {
    std::env::var("adminusername")
        .or_else(|_| std::env::var("ADMINUSERNAME"))
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ADMIN_USERNAME.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn configured_admin_username() -> String {
    ADMIN_USERNAME.to_string()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn configured_admin_password() -> String {
    std::env::var("adminpassword")
        .or_else(|_| std::env::var("ADMINPASSWORD"))
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ADMIN_PASSWORD.to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn configured_admin_password() -> String {
    ADMIN_PASSWORD.to_string()
}

// -----------------------------------------------------------------------------
// ADMIN CREDENTIAL STRUCTURE
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AdminCredentials {
    pub username: String,
    pub password: String,
}

impl AdminCredentials {
    pub fn is_admin(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.username == configured_admin_username()
                && self.password == configured_admin_password()
        }

        // The browser cannot read Docker's runtime environment.  Successful
        // browser credentials have already been validated by the server before
        // ADMIN_AUTH is populated, so on WASM this simply means "authenticated
        // credentials are present" for rendering the admin UI.
        #[cfg(target_arch = "wasm32")]
        {
            !self.username.is_empty() && !self.password.is_empty()
        }
    }
}

// -----------------------------------------------------------------------------
// DASHBOARD PAYLOAD
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DashboardPayload {
    pub buttons: Vec<ButtonGroupView>,
    pub is_admin: bool,
    pub available_tags: Vec<String>,
    pub autosort: bool,
}

// -----------------------------------------------------------------------------
// BUTTON GROUP VIEW
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ButtonGroupView {
    pub name: String,
    pub rows: Vec<ButtonRowView>,
}

// -----------------------------------------------------------------------------
// BUTTON ROW VIEW
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ButtonRowView {
    Divider {
        name: String,
    },

    Link {
        name: String,
        url: String,

        // Optional free-form notes about this URL row.
        // The full string is sent to the browser and displayed in the URL list
        // as a single truncated line.
        comments: String,

        // Original hashtag string.
        //
        // Example:
        //
        //     "#rust #python #programming"
        //
        hashtags: String,

        // Hashtags split into individual tags.
        //
        // Example:
        //
        //     ["#rust", "#python", "#programming"]
        //
        hashtag_tokens: Vec<String>,

        // Secret is only sent to the browser when the user is admin.
        secret_text: Option<String>,

        // True when a secret exists, even when it is hidden.
        has_secret: bool,
    },
}

// -----------------------------------------------------------------------------
// CREATE BUTTON INPUT
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ButtonCreateInput {
    pub name: String,
    pub first_row: ButtonRowInput,
}

// -----------------------------------------------------------------------------
// RENAME BUTTON INPUT
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ButtonRenameInput {
    pub original_name: String,
    pub name: String,
}

// -----------------------------------------------------------------------------
// ROW INPUT
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ButtonRowInput {
    pub original_name: Option<String>,
    pub name: String,
    pub kind: RowKind,
    pub url: String,
    pub hashtags: String,
    pub comments: String,
    pub secrets: String,
}

// -----------------------------------------------------------------------------
// ROW KIND
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RowKind {
    Link,
    Divider,
}

// -----------------------------------------------------------------------------
// INTERNAL DASHBOARD DOCUMENT
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct DashboardDocument {
    pub buttons: Vec<ButtonGroup>,
}

// -----------------------------------------------------------------------------
// INTERNAL BUTTON GROUP
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub struct ButtonGroup {
    pub name: String,
    pub rows: Vec<ButtonRow>,
}

// -----------------------------------------------------------------------------
// INTERNAL BUTTON ROW
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum ButtonRow {
    Divider {
        name: String,
    },

    Link {
        name: String,
        url: String,
        hashtags: String,
        comments: String,
        secrets: String,
    },
}

// -----------------------------------------------------------------------------
// STORED JSON ROW
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum StoredRow {
    Divider(String),
    Link(StoredLink),
}

// -----------------------------------------------------------------------------
// STORED JSON LINK
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredLink {
    url: String,
    hashtags: String,
    comments: String,
    secrets: String,
}

// -----------------------------------------------------------------------------
// STORED JSON TYPES
// -----------------------------------------------------------------------------

type StoredDashboard = IndexMap<String, IndexMap<String, StoredRow>>;

type LegacyDashboard = IndexMap<String, IndexMap<String, String>>;

// -----------------------------------------------------------------------------
// DASHBOARD -> PAYLOAD
// -----------------------------------------------------------------------------

impl DashboardDocument {
    pub fn to_payload(
        &self,
        is_admin: bool,
        available_tags: Vec<String>,
        autosort: bool,
    ) -> DashboardPayload {
        DashboardPayload {
            is_admin,

            available_tags,

            autosort,

            buttons: self
                .buttons
                .iter()
                .map(|button| button.to_view(is_admin))
                .collect(),
        }
    }
}

// -----------------------------------------------------------------------------
// BUTTON GROUP -> VIEW
// -----------------------------------------------------------------------------

impl ButtonGroup {
    fn to_view(&self, is_admin: bool) -> ButtonGroupView {
        ButtonGroupView {
            name: self.name.clone(),

            rows: self.rows.iter().map(|row| row.to_view(is_admin)).collect(),
        }
    }
}

// -----------------------------------------------------------------------------
// BUTTON ROW -> VIEW
// -----------------------------------------------------------------------------

impl ButtonRow {
    fn to_view(&self, is_admin: bool) -> ButtonRowView {
        match self {
            // -------------------------------------------------------------
            // DIVIDER
            // -------------------------------------------------------------
            Self::Divider { name } => ButtonRowView::Divider { name: name.clone() },

            // -------------------------------------------------------------
            // LINK
            // -------------------------------------------------------------
            Self::Link {
                name,
                url,
                hashtags,
                comments,
                secrets,
            } => {
                ButtonRowView::Link {
                    name: name.clone(),

                    url: url.clone(),

                    comments: comments.clone(),

                    // Always expose the normalized hashtag string.
                    hashtags: normalize_hashtags(hashtags),

                    // Split the normalized hashtags for searching/rendering.
                    hashtag_tokens: parse_hashtags(hashtags),

                    // Only expose the actual secret to admins.
                    secret_text: if is_admin && !secrets.is_empty() {
                        Some(secrets.clone())
                    } else {
                        None
                    },

                    has_secret: !secrets.is_empty(),
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------
// LOAD DASHBOARD PAYLOAD
// -----------------------------------------------------------------------------

pub fn load_dashboard_payload_from_path(
    path: &Path,
    credentials: &AdminCredentials,
) -> Result<DashboardPayload> {
    let document = read_dashboard_document(path)?;

    let available_tags = load_managed_tags(&document)?;

    let is_admin = credentials.is_admin();
    // Auto-sort is a system-wide dashboard setting.
    // It is intentionally NOT tied to the logged-in user.
    let autosort = load_system_preferences()?.autosort;

    Ok(document.to_payload(is_admin, available_tags, autosort))
}

// -----------------------------------------------------------------------------
// READ DASHBOARD DOCUMENT
// -----------------------------------------------------------------------------

pub fn read_dashboard_document(path: &Path) -> Result<DashboardDocument> {
    let file_text = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read {}", path.display()))?;

    parse_dashboard_document(path, &file_text)
}

// -----------------------------------------------------------------------------
// CHECK DASHBOARD JSON
// -----------------------------------------------------------------------------
//
// This is deliberately the same parser used by normal dashboard loading.
// Therefore a manual edit made with vi receives the same useful path, line,
// column, and source-line information that the dashboard itself would show.
// -----------------------------------------------------------------------------

pub fn check_dashboard_json(path: &Path) -> Result<()> {
    read_dashboard_document(path).map(|_| ())
}

// -----------------------------------------------------------------------------
// PARSE DASHBOARD DOCUMENT
// -----------------------------------------------------------------------------

pub fn parse_dashboard_document(path: &Path, file_text: &str) -> Result<DashboardDocument> {
    let mut deserializer = serde_json::Deserializer::from_str(file_text);

    let stored: StoredDashboard = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|err| anyhow!(format_deserialize_error(path, file_text, err)))?;

    stored_dashboard_to_document(stored)
}

// -----------------------------------------------------------------------------
// HISTORY BACKUP
// -----------------------------------------------------------------------------
//
// Keep the backup helper in this data module instead of the UI history module.
// That is important because data_button_v2_migrator.rs also compiles this file
// as its own standalone module.
// -----------------------------------------------------------------------------

pub fn backup_dashboard_file(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }

    std::fs::create_dir_all(HISTORY_DIR)?;

    // History filenames use UTC so they are easy to sort alphabetically:
    //
    //     dashboard-20260921_061530_123.json
    //
    // That means the filename itself tells us when the snapshot was created.
    let filename = make_history_filename()?;
    let destination = Path::new(HISTORY_DIR).join(&filename);

    std::fs::copy(path, &destination)?;

    // Keep only the newest 100 snapshots.
    //
    // The timestamp is at the start of every new filename, so normal
    // string sorting gives us oldest -> newest ordering.
    let mut snapshots: Vec<(String, std::path::PathBuf)> = Vec::new();

    for entry in std::fs::read_dir(HISTORY_DIR)? {
        let entry = entry?;
        let entry_path = entry.path();

        if !entry_path.is_file() {
            continue;
        }

        let Some(name) = entry_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if !is_history_snapshot_filename(name) {
            continue;
        }

        snapshots.push((name.to_string(), entry_path));
    }

    snapshots.sort_by(|left, right| left.0.cmp(&right.0));

    for (_, old_path) in snapshots.into_iter().rev().skip(MAX_HISTORY_SNAPSHOTS) {
        std::fs::remove_file(old_path)?;
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// HISTORY FILENAME
// -----------------------------------------------------------------------------
//
// We intentionally keep this dependency-free rather than adding another crate
// just to format a timestamp.
//
// The filename is UTC and has this shape:
//
//     dashboard-YYYYMMDD_HHMMSS_milliseconds.json
//
// If several dashboard writes happen during the same second, we add _01, _02,
// etc. so an existing snapshot can never be accidentally overwritten.
// -----------------------------------------------------------------------------

fn make_history_filename() -> Result<String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| anyhow!("Could not determine current time: {error}"))?;

    let total_milliseconds = duration.as_millis();
    let total_seconds = total_milliseconds / 1_000;
    let milliseconds = total_milliseconds % 1_000;

    let seconds_in_day = 24 * 60 * 60;
    let days = total_seconds / seconds_in_day;
    let seconds_today = total_seconds % seconds_in_day;

    let hour = seconds_today / 3600;
    let minute = (seconds_today % 3600) / 60;
    let second = seconds_today % 60;

    let (year, month, day) = civil_date_from_days(days as i64);

    // New format:
    //
    //     dashboard-YYYYMMDD_HHMMSS_milliseconds.json
    //
    // The milliseconds are always exactly three digits. For example:
    //
    //     dashboard-20260921_061530_123.json
    //
    // Because the timestamp is UTC and sortable, alphabetical filename order
    // is also chronological order.
    let base = format!(
        "dashboard-{year:04}{month:02}{day:02}_{hour:02}{minute:02}{second:02}_{milliseconds:03}"
    );

    let plain = format!("{base}.json");
    if !Path::new(HISTORY_DIR).join(&plain).exists() {
        return Ok(plain);
    }

    // Several changes can theoretically happen inside the same millisecond.
    // Keep the timestamp intact and add a tiny collision suffix so we never
    // overwrite an existing snapshot.
    for number in 1..=9999 {
        let filename = format!("{base}_{number:02}.json");

        if !Path::new(HISTORY_DIR).join(&filename).exists() {
            return Ok(filename);
        }
    }

    bail!("Could not create a unique history snapshot filename.")
}

// Convert days since 1970-01-01 into a Gregorian calendar date.
// This is the standard civil-date calculation and needs no external crate.
fn civil_date_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };

    (year, month as u32, day as u32)
}

fn is_history_snapshot_filename(filename: &str) -> bool {
    let Some(stem) = filename
        .strip_prefix("dashboard-")
        .and_then(|value| value.strip_suffix(".json"))
    else {
        return false;
    };

    let bytes = stem.as_bytes();

    // New format:
    // YYYYMMDD_HHMMSS_milliseconds
    //
    // Example:
    // 20260921_061530_123
    if bytes.len() == 19
        && bytes[8] == b'_'
        && bytes[15] == b'_'
        && bytes[..8].iter().all(u8::is_ascii_digit)
        && bytes[9..15].iter().all(u8::is_ascii_digit)
        && bytes[16..].iter().all(u8::is_ascii_digit)
    {
        return true;
    }

    // Also accept the optional collision suffix:
    // YYYYMMDD_HHMMSS_milliseconds_01
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

    // Keep the old nanosecond-format snapshots valid so an existing history
    // directory can still be displayed and cleaned up safely.
    stem.chars().all(|character| character.is_ascii_digit())
}

// -----------------------------------------------------------------------------
// WRITE DASHBOARD DOCUMENT
// -----------------------------------------------------------------------------

pub fn write_dashboard_document(path: &Path, document: &DashboardDocument) -> Result<()> {
    validate_document(document)?;

    // Keep a restore point BEFORE replacing the live JSON.
    // This lives in dashboard_data.rs so the standalone migration binary
    // can also compile dashboard_data.rs without needing the UI history module.
    backup_dashboard_file(path)?;

    let json = serde_json::to_string_pretty(&document_to_stored_dashboard(document))
        .with_context(|| format!("Could not serialize {}", path.display()))?;

    std::fs::write(path, format!("{json}\n"))
        .with_context(|| format!("Could not write {}", path.display()))?;

    Ok(())
}

// -----------------------------------------------------------------------------
// CREATE BUTTON
// -----------------------------------------------------------------------------

pub fn create_button(
    path: &Path,
    credentials: &AdminCredentials,
    input: ButtonCreateInput,
) -> Result<()> {
    ensure_admin(credentials)?;

    let name = clean_required("button name", &input.name)?;

    let mut document = read_dashboard_document(path)?;

    if document.buttons.iter().any(|button| button.name == name) {
        bail!("A button named '{name}' already exists.");
    }

    let first_row = row_from_input(&name, input.first_row)?;

    document.buttons.push(ButtonGroup {
        name,
        rows: vec![first_row],
    });

    write_dashboard_document(path, &document)
}

// -----------------------------------------------------------------------------
// RENAME BUTTON
// -----------------------------------------------------------------------------

pub fn rename_button(
    path: &Path,
    credentials: &AdminCredentials,
    input: ButtonRenameInput,
) -> Result<()> {
    ensure_admin(credentials)?;

    let new_name = clean_required("button name", &input.name)?;

    let mut document = read_dashboard_document(path)?;

    let Some(index) = document
        .buttons
        .iter()
        .position(|button| button.name == input.original_name)
    else {
        bail!("Could not find button '{}'.", input.original_name);
    };

    if input.original_name != new_name
        && document
            .buttons
            .iter()
            .any(|button| button.name == new_name)
    {
        bail!("A button named '{new_name}' already exists.");
    }

    document.buttons[index].name = new_name;

    write_dashboard_document(path, &document)
}

// -----------------------------------------------------------------------------
// CLONE BUTTON
// -----------------------------------------------------------------------------
//
// Clone the complete button group and insert it immediately after the original.
//
// The new name is generated on the server so the JSON mutation is safe even
// when names such as `Servers-1`, `Servers-2`, etc. already exist.
//
// Example:
//
//     Servers
//     Servers-1
//     Servers-2
//
// If `Servers-1` is cloned, the next available numbered name is `Servers-2`.
// The suffix is always chosen on the server so no existing name is overwritten.
// -----------------------------------------------------------------------------

pub fn clone_button(
    path: &Path,
    credentials: &AdminCredentials,
    button_name: &str,
) -> Result<String> {
    ensure_admin(credentials)?;

    let mut document = read_dashboard_document(path)?;

    let Some(index) = document
        .buttons
        .iter()
        .position(|button| button.name == button_name)
    else {
        bail!("Could not find button '{button_name}'.");
    };

    let existing_names = document.buttons.iter().map(|button| button.name.as_str());

    let cloned_name = unique_clone_name(button_name, existing_names);

    let mut cloned_button = document.buttons[index].clone();
    cloned_button.name = cloned_name.clone();

    // `insert(index + 1, ...)` is what makes the clone appear immediately
    // after the source in the stored JSON order.
    document.buttons.insert(index + 1, cloned_button);

    write_dashboard_document(path, &document)?;

    Ok(cloned_name)
}

// -----------------------------------------------------------------------------
// DELETE BUTTON
// -----------------------------------------------------------------------------

pub fn delete_button(path: &Path, credentials: &AdminCredentials, button_name: &str) -> Result<()> {
    ensure_admin(credentials)?;

    let mut document = read_dashboard_document(path)?;

    let Some(index) = document
        .buttons
        .iter()
        .position(|button| button.name == button_name)
    else {
        bail!("Could not find button '{button_name}'.");
    };

    document.buttons.remove(index);

    write_dashboard_document(path, &document)
}

// -----------------------------------------------------------------------------
// SAVE ROW
// -----------------------------------------------------------------------------

pub fn save_row(
    path: &Path,
    credentials: &AdminCredentials,
    button_name: &str,
    input: ButtonRowInput,
) -> Result<()> {
    ensure_admin(credentials)?;

    let mut document = read_dashboard_document(path)?;

    let Some(button) = document
        .buttons
        .iter_mut()
        .find(|button| button.name == button_name)
    else {
        bail!("Could not find button '{button_name}'.");
    };

    let new_row = row_from_input(button_name, input.clone())?;

    if let Some(original_name) = &input.original_name {
        let Some(index) = button
            .rows
            .iter()
            .position(|row| row.name() == original_name)
        else {
            bail!("Could not find row '{original_name}' in button '{button_name}'.");
        };

        if original_name != new_row.name()
            && button.rows.iter().any(|row| row.name() == new_row.name())
        {
            bail!(
                "A row named '{}' already exists in button '{}'.",
                new_row.name(),
                button_name
            );
        }

        button.rows[index] = new_row;
    } else {
        if button.rows.iter().any(|row| row.name() == new_row.name()) {
            bail!(
                "A row named '{}' already exists in button '{}'.",
                new_row.name(),
                button_name
            );
        }

        button.rows.push(new_row);
    }

    write_dashboard_document(path, &document)
}

// -----------------------------------------------------------------------------
// DELETE ROW
// -----------------------------------------------------------------------------

// -----------------------------------------------------------------------------
// CLONE ROW
// -----------------------------------------------------------------------------
//
// Clone one row and insert it immediately after the original row.
//
// The clone gets a unique suffix generated from the original row name.  The
// entire row is cloned, including URL, hashtags, comments, secrets, and whether
// it is a divider or link.
//
// Example:
//
//     Proxmox
//     Proxmox-1
// -----------------------------------------------------------------------------

pub fn clone_row(
    path: &Path,
    credentials: &AdminCredentials,
    button_name: &str,
    row_name: &str,
) -> Result<String> {
    ensure_admin(credentials)?;

    let mut document = read_dashboard_document(path)?;

    let Some(button) = document
        .buttons
        .iter_mut()
        .find(|button| button.name == button_name)
    else {
        bail!("Could not find button '{button_name}'.");
    };

    let Some(index) = button.rows.iter().position(|row| row.name() == row_name) else {
        bail!("Could not find row '{row_name}' in button '{button_name}'.");
    };

    let existing_names = button.rows.iter().map(|row| row.name());

    let cloned_name = unique_clone_name(row_name, existing_names);

    let mut cloned_row = button.rows[index].clone();

    match &mut cloned_row {
        ButtonRow::Divider { name } | ButtonRow::Link { name, .. } => {
            *name = cloned_name.clone();
        }
    }

    // Insert directly after the source row so the clone is truly in-place.
    button.rows.insert(index + 1, cloned_row);

    write_dashboard_document(path, &document)?;

    Ok(cloned_name)
}

pub fn delete_row(
    path: &Path,
    credentials: &AdminCredentials,
    button_name: &str,
    row_name: &str,
) -> Result<()> {
    ensure_admin(credentials)?;

    let mut document = read_dashboard_document(path)?;

    let Some(button) = document
        .buttons
        .iter_mut()
        .find(|button| button.name == button_name)
    else {
        bail!("Could not find button '{button_name}'.");
    };

    if button.rows.len() <= 1 {
        bail!("Button '{button_name}' must keep at least one row.");
    }

    let Some(index) = button.rows.iter().position(|row| row.name() == row_name) else {
        bail!("Could not find row '{row_name}' in button '{button_name}'.");
    };

    button.rows.remove(index);

    write_dashboard_document(path, &document)
}

// -----------------------------------------------------------------------------
// SYSTEM PREFERENCES
// -----------------------------------------------------------------------------
//
// These settings belong to the dashboard itself rather than to one user.
// That means the setting survives F5 and is the same for every visitor.
// Only an administrator may change it.
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SystemPreferences {
    #[serde(default)]
    pub autosort: bool,
}

pub fn load_system_preferences() -> Result<SystemPreferences> {
    let path = Path::new(SYSTEM_PREFERENCES_FILE);

    if !path.exists() {
        return Ok(SystemPreferences::default());
    }

    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read {}", path.display()))?;

    serde_json::from_str(&text).with_context(|| format!("Could not parse {}", path.display()))
}

pub fn set_system_autosort(credentials: &AdminCredentials, enabled: bool) -> Result<()> {
    ensure_admin(credentials)?;

    let preferences = SystemPreferences { autosort: enabled };

    let json = serde_json::to_string_pretty(&preferences)?;

    std::fs::write(SYSTEM_PREFERENCES_FILE, format!("{json}\n"))
        .with_context(|| format!("Could not write {}", SYSTEM_PREFERENCES_FILE))?;

    Ok(())
}

// -----------------------------------------------------------------------------
// LOAD MANAGED TAGS
// -----------------------------------------------------------------------------
//
// Tags live in a small separate JSON file so the tag manager can contain tags
// that are not attached to a URL yet.
//
// If the file does not exist, we derive the initial list from the dashboard.
// Used tags are always included, even if the tag file was edited by hand.
// -----------------------------------------------------------------------------

pub fn load_managed_tags(document: &DashboardDocument) -> Result<Vec<String>> {
    let mut tags = if Path::new(TAGS_FILE).exists() {
        let text = std::fs::read_to_string(TAGS_FILE)
            .with_context(|| format!("Could not read {}", TAGS_FILE))?;

        serde_json::from_str::<Vec<String>>(&text)
            .with_context(|| format!("Could not parse {}", TAGS_FILE))?
    } else {
        Vec::new()
    };

    for button in &document.buttons {
        for row in &button.rows {
            if let ButtonRow::Link { hashtags, .. } = row {
                tags.extend(parse_hashtags(hashtags));
            }
        }
    }

    normalize_tag_list(&tags)
}

// -----------------------------------------------------------------------------
// SAVE MANAGED TAGS
// -----------------------------------------------------------------------------

pub fn save_managed_tags(credentials: &AdminCredentials, tags: Vec<String>) -> Result<()> {
    ensure_admin(credentials)?;

    let document = read_dashboard_document(Path::new(BUTTONS_V3_FILE))?;
    let normalized = normalize_tag_list(&tags)?;

    // A tag currently used by a URL may not be removed from the managed list.
    let used_tags: std::collections::BTreeSet<String> = document
        .buttons
        .iter()
        .flat_map(|button| button.rows.iter())
        .filter_map(|row| match row {
            ButtonRow::Link { hashtags, .. } => Some(hashtags.as_str()),
            ButtonRow::Divider { .. } => None,
        })
        .flat_map(parse_hashtags)
        .collect();

    let saved_tags: std::collections::BTreeSet<String> = normalized.iter().cloned().collect();

    for used in used_tags {
        if !saved_tags.contains(&used) {
            let mut usages = Vec::new();

            for button in &document.buttons {
                for row in &button.rows {
                    if let ButtonRow::Link { name, hashtags, .. } = row {
                        if parse_hashtags(hashtags)
                            .iter()
                            .any(|tag| tag.eq_ignore_ascii_case(&used))
                        {
                            usages.push(format!("'{}' / '{}'", button.name, name));
                        }
                    }
                }
            }

            if usages.is_empty() {
                bail!("Cannot remove tag '{used}' because it is still used by a URL.");
            }

            bail!(
                "Cannot remove tag '{used}'. It is still used by these button/URL rows: {}. Remove the tag from those rows first.",
                usages.join(", ")
            );
        }
    }

    let json =
        serde_json::to_string_pretty(&normalized).context("Could not serialize managed tags")?;

    std::fs::write(TAGS_FILE, format!("{json}\n"))
        .with_context(|| format!("Could not write {}", TAGS_FILE))?;

    Ok(())
}

// -----------------------------------------------------------------------------
// REORDER BUTTONS
// -----------------------------------------------------------------------------
//
// `from_index` is the button being dragged.
// `to_index` is the button it was dropped onto.
//
// Auto-sort is a presentation setting. Manual button dragging is therefore
// intended to be used while auto-sort is OFF. The server still accepts the
// request only from an administrator.
// -----------------------------------------------------------------------------

pub fn reorder_buttons(
    path: &Path,
    credentials: &AdminCredentials,
    from_index: usize,
    to_index: usize,
) -> Result<()> {
    ensure_admin(credentials)?;

    let mut document = read_dashboard_document(path)?;

    if from_index >= document.buttons.len() || to_index >= document.buttons.len() {
        bail!("Invalid button position.");
    }

    if from_index == to_index {
        return Ok(());
    }

    let button = document.buttons.remove(from_index);
    document.buttons.insert(to_index, button);

    write_dashboard_document(path, &document)
}

// -----------------------------------------------------------------------------
// REORDER ROWS
// -----------------------------------------------------------------------------
//
// `from_index` is the row being dragged.
// `to_index` is the row it was dropped onto.
// This works for both links and dividers because both are ButtonRow values.
// -----------------------------------------------------------------------------

pub fn reorder_rows(
    path: &Path,
    credentials: &AdminCredentials,
    button_name: &str,
    from_index: usize,
    to_index: usize,
) -> Result<()> {
    ensure_admin(credentials)?;

    let mut document = read_dashboard_document(path)?;

    let Some(button) = document
        .buttons
        .iter_mut()
        .find(|button| button.name == button_name)
    else {
        bail!("Could not find button '{button_name}'.");
    };

    if from_index >= button.rows.len() || to_index >= button.rows.len() {
        bail!("Invalid row position for button '{button_name}'.");
    }

    if from_index == to_index {
        return Ok(());
    }

    let row = button.rows.remove(from_index);
    button.rows.insert(to_index, row);

    write_dashboard_document(path, &document)
}

// -----------------------------------------------------------------------------
// MIGRATE V1 FILE
// -----------------------------------------------------------------------------

pub fn migrate_v1_file_to_v2_string(path: &Path) -> Result<String> {
    let file_text = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read {}", path.display()))?;

    let mut deserializer = serde_json::Deserializer::from_str(&file_text);

    let legacy: LegacyDashboard = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|err| anyhow!(format_deserialize_error(path, &file_text, err)))?;

    let document = legacy_dashboard_to_document(legacy)?;

    let json = serde_json::to_string_pretty(&document_to_stored_dashboard(&document))
        .with_context(|| format!("Could not serialize migrated data for {}", path.display()))?;

    Ok(format!("{json}\n"))
}

// -----------------------------------------------------------------------------
// V2 STORED JSON TYPES (MIGRATION INPUT)
// -----------------------------------------------------------------------------
//
// Version 2 stored URL rows did not have a `comments` field. Keep a separate
// input type so the normal v3 parser can remain strict with deny_unknown_fields
// and the migration can explicitly add the new field.
//
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum StoredRowV2 {
    Divider(String),
    Link(StoredLinkV2),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredLinkV2 {
    url: String,
    hashtags: String,
    secrets: String,
}

type StoredDashboardV2 = IndexMap<String, IndexMap<String, StoredRowV2>>;

// -----------------------------------------------------------------------------
// MIGRATE V2 FILE TO V3
// -----------------------------------------------------------------------------

pub fn migrate_v2_file_to_v3_string(path: &Path) -> Result<String> {
    let file_text = std::fs::read_to_string(path)
        .with_context(|| format!("Could not read {}", path.display()))?;

    let mut deserializer = serde_json::Deserializer::from_str(&file_text);

    let stored_v2: StoredDashboardV2 = serde_path_to_error::deserialize(&mut deserializer)
        .map_err(|err| anyhow!(format_deserialize_error(path, &file_text, err)))?;

    let mut buttons = Vec::new();

    for (button_name, rows) in stored_v2 {
        if rows.is_empty() {
            bail!("Button '{button_name}' must contain at least one row.");
        }

        let mut parsed_rows = Vec::new();

        for (row_name, row_value) in rows {
            match row_value {
                StoredRowV2::Divider(value) => {
                    if !row_name.contains("DIVIDER") {
                        bail!(
                            "Row '{button_name}.{row_name}' uses a string value, but only DIVIDER rows may do that."
                        );
                    }

                    if !value.is_empty() {
                        bail!(
                            "Divider row '{button_name}.{row_name}' must use an empty string value."
                        );
                    }

                    parsed_rows.push(ButtonRow::Divider { name: row_name });
                }

                StoredRowV2::Link(link) => {
                    if row_name.contains("DIVIDER") {
                        bail!(
                            "Row '{button_name}.{row_name}' looks like a divider but contains link fields."
                        );
                    }

                    parsed_rows.push(ButtonRow::Link {
                        name: clean_required("row name", &row_name)?,
                        url: clean_required("url", &link.url)?,
                        hashtags: normalize_hashtags(&link.hashtags),
                        comments: String::new(),
                        secrets: link.secrets,
                    });
                }
            }
        }

        buttons.push(ButtonGroup {
            name: clean_required("button name", &button_name)?,
            rows: parsed_rows,
        });
    }

    let document = DashboardDocument { buttons };
    validate_document(&document)?;

    let json = serde_json::to_string_pretty(&document_to_stored_dashboard(&document))
        .with_context(|| format!("Could not serialize migrated data for {}", path.display()))?;

    Ok(format!("{json}\\n"))
}

// -----------------------------------------------------------------------------
// STORED JSON -> INTERNAL DOCUMENT
// -----------------------------------------------------------------------------

fn stored_dashboard_to_document(stored: StoredDashboard) -> Result<DashboardDocument> {
    let mut buttons = Vec::new();

    for (button_name, rows) in stored {
        if rows.is_empty() {
            bail!("Button '{button_name}' must contain at least one row.");
        }

        let mut parsed_rows = Vec::new();

        for (row_name, row_value) in rows {
            match row_value {
                // -------------------------------------------------------------
                // DIVIDER
                // -------------------------------------------------------------
                StoredRow::Divider(value) => {
                    if !row_name.contains("DIVIDER") {
                        bail!(
                            "Row '{button_name}.{row_name}' uses a string value, but only DIVIDER rows may do that."
                        );
                    }

                    if !value.is_empty() {
                        bail!(
                            "Divider row '{button_name}.{row_name}' must use an empty string value."
                        );
                    }

                    parsed_rows.push(ButtonRow::Divider { name: row_name });
                }

                // -------------------------------------------------------------
                // LINK
                // -------------------------------------------------------------
                StoredRow::Link(link) => {
                    if row_name.contains("DIVIDER") {
                        bail!(
                            "Row '{button_name}.{row_name}' looks like a divider but contains link fields."
                        );
                    }

                    let url = clean_required("url", &link.url)?;

                    parsed_rows.push(ButtonRow::Link {
                        name: clean_required("row name", &row_name)?,

                        url,

                        comments: link.comments,

                        // Normalize tags while loading.
                        //
                        // This means even old JSON such as:
                        //
                        //     "rust python"
                        //
                        // becomes internally:
                        //
                        //     "#rust #python"
                        //
                        hashtags: normalize_hashtags(&link.hashtags),

                        secrets: link.secrets,
                    });
                }
            }
        }

        buttons.push(ButtonGroup {
            name: clean_required("button name", &button_name)?,

            rows: parsed_rows,
        });
    }

    let document = DashboardDocument { buttons };

    validate_document(&document)?;

    Ok(document)
}

// -----------------------------------------------------------------------------
// LEGACY DASHBOARD -> INTERNAL DOCUMENT
// -----------------------------------------------------------------------------

fn legacy_dashboard_to_document(legacy: LegacyDashboard) -> Result<DashboardDocument> {
    let mut buttons = Vec::new();

    for (button_name, rows) in legacy {
        if rows.is_empty() {
            bail!("Button '{button_name}' must contain at least one row.");
        }

        let mut parsed_rows = Vec::new();

        for (row_name, value) in rows {
            if row_name.contains("DIVIDER") {
                if !value.is_empty() {
                    bail!(
                        "Divider row '{button_name}.{row_name}' must use an empty string in the legacy file."
                    );
                }

                parsed_rows.push(ButtonRow::Divider { name: row_name });
            } else {
                parsed_rows.push(ButtonRow::Link {
                    name: clean_required("row name", &row_name)?,

                    url: clean_required("url", &value)?,

                    hashtags: String::new(),

                    comments: String::new(),

                    secrets: String::new(),
                });
            }
        }

        buttons.push(ButtonGroup {
            name: clean_required("button name", &button_name)?,

            rows: parsed_rows,
        });
    }

    let document = DashboardDocument { buttons };

    validate_document(&document)?;

    Ok(document)
}

// -----------------------------------------------------------------------------
// INTERNAL DOCUMENT -> STORED JSON
// -----------------------------------------------------------------------------

fn document_to_stored_dashboard(document: &DashboardDocument) -> StoredDashboard {
    let mut stored = IndexMap::new();

    for button in &document.buttons {
        let mut rows = IndexMap::new();

        for row in &button.rows {
            match row {
                // -------------------------------------------------------------
                // DIVIDER
                // -------------------------------------------------------------
                ButtonRow::Divider { name } => {
                    rows.insert(name.clone(), StoredRow::Divider(String::new()));
                }

                // -------------------------------------------------------------
                // LINK
                // -------------------------------------------------------------
                ButtonRow::Link {
                    name,
                    url,
                    hashtags,
                    comments,
                    secrets,
                } => {
                    rows.insert(
                        name.clone(),
                        StoredRow::Link(StoredLink {
                            url: url.clone(),

                            comments: comments.clone(),

                            // Normalize AGAIN when writing.
                            //
                            // This guarantees that JSON on disk always
                            // contains exactly one '#' at the beginning
                            // of every tag.
                            hashtags: normalize_hashtags(hashtags),

                            secrets: secrets.clone(),
                        }),
                    );
                }
            }
        }

        stored.insert(button.name.clone(), rows);
    }

    stored
}

// -----------------------------------------------------------------------------
// VALIDATE DOCUMENT
// -----------------------------------------------------------------------------

fn validate_document(document: &DashboardDocument) -> Result<()> {
    if document.buttons.is_empty() {
        bail!("The dashboard file must contain at least one button.");
    }

    for button in &document.buttons {
        clean_required("button name", &button.name)?;

        if button.rows.is_empty() {
            bail!("Button '{}' must contain at least one row.", button.name);
        }

        let mut seen_row_names = std::collections::BTreeSet::new();

        for row in &button.rows {
            let row_name = clean_required("row name", row.name())?;

            if !seen_row_names.insert(row_name.clone()) {
                bail!(
                    "Button '{}' contains duplicate row '{}'.",
                    button.name,
                    row_name
                );
            }

            match row {
                // -------------------------------------------------------------
                // DIVIDER
                // -------------------------------------------------------------
                ButtonRow::Divider { .. } => {
                    if !row_name.contains("DIVIDER") {
                        bail!(
                            "Divider row '{}.{}' must include 'DIVIDER' in the row name.",
                            button.name,
                            row_name
                        );
                    }
                }

                // -------------------------------------------------------------
                // LINK
                // -------------------------------------------------------------
                ButtonRow::Link { url, hashtags, .. } => {
                    if row_name.contains("DIVIDER") {
                        bail!(
                            "Link row '{}.{}' cannot include 'DIVIDER' in the row name.",
                            button.name,
                            row_name
                        );
                    }

                    clean_required("url", url)?;

                    validate_hashtags(hashtags)?;
                }
            }
        }
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// ROW FROM INPUT
// -----------------------------------------------------------------------------

fn row_from_input(button_name: &str, input: ButtonRowInput) -> Result<ButtonRow> {
    let row_name = clean_required("row name", &input.name)?;

    match input.kind {
        // ---------------------------------------------------------------------
        // DIVIDER
        // ---------------------------------------------------------------------
        RowKind::Divider => {
            if !row_name.contains("DIVIDER") {
                bail!(
                    "Divider row '{}.{}' must include 'DIVIDER' in the row name.",
                    button_name,
                    row_name
                );
            }

            Ok(ButtonRow::Divider { name: row_name })
        }

        // ---------------------------------------------------------------------
        // LINK
        // ---------------------------------------------------------------------
        RowKind::Link => {
            if row_name.contains("DIVIDER") {
                bail!(
                    "Link row '{}.{}' cannot include 'DIVIDER' in the row name.",
                    button_name,
                    row_name
                );
            }

            validate_hashtags(&input.hashtags)?;

            Ok(ButtonRow::Link {
                name: row_name,

                url: clean_required("url", &input.url)?,

                comments: input.comments,

                // ---------------------------------------------------------
                // NORMALIZE TAGS HERE
                // ---------------------------------------------------------
                //
                // This is the important part for the editor.
                //
                // Whatever the user types:
                //
                //     rust python
                //     #rust #python
                //     ##rust ###python
                //
                // JSON will receive:
                //
                //     "#rust #python"
                // ---------------------------------------------------------
                hashtags: normalize_hashtags(&input.hashtags),

                secrets: input.secrets,
            })
        }
    }
}

// -----------------------------------------------------------------------------
// UNIQUE CLONE NAME
// -----------------------------------------------------------------------------
//
// Generate:
//
//     original
//     original-1
//     original-2
//     original-3
//
// The function accepts an iterator so it can be used for both button names and
// row names without duplicating the collision logic.
//
// The first available suffix wins.  We never replace or rename an existing
// object while cloning.
// -----------------------------------------------------------------------------

fn unique_clone_name<'a, I>(original_name: &str, existing_names: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    let existing_names: std::collections::BTreeSet<&str> = existing_names.into_iter().collect();

    // If the source already ends in "-<number>", continue that number rather
    // than producing names such as "Ubuntu-1-1".
    //
    //     Ubuntu   -> Ubuntu-1
    //     Ubuntu   -> Ubuntu-2
    //     Ubuntu-1 -> Ubuntu-2
    //     Ubuntu-2 -> Ubuntu-3
    //
    // This also means cloning an older clone naturally continues the same
    // numbering sequence.
    let (base_name, starting_number) = match original_name.rsplit_once('-') {
        Some((base, suffix))
            if !base.is_empty()
                && !suffix.is_empty()
                && suffix.chars().all(|c| c.is_ascii_digit()) =>
        {
            let number = suffix.parse::<u64>().unwrap_or(0);
            (base, number.saturating_add(1))
        }
        _ => (original_name, 1),
    };

    for number in starting_number.. {
        let candidate = format!("{base_name}-{number}");

        if !existing_names.contains(candidate.as_str()) {
            return candidate;
        }
    }

    unreachable!("The clone-name counter cannot overflow in practical use.")
}

// -----------------------------------------------------------------------------
// CLEAN REQUIRED VALUE
// -----------------------------------------------------------------------------

fn clean_required(label: &str, value: &str) -> Result<String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        bail!("{label} cannot be empty.");
    }

    Ok(trimmed.to_string())
}

// -----------------------------------------------------------------------------
// ENSURE ADMIN
// -----------------------------------------------------------------------------

fn ensure_admin(credentials: &AdminCredentials) -> Result<()> {
    if credentials.is_admin() {
        Ok(())
    } else {
        bail!("Admin login is required for this action.");
    }
}

// -----------------------------------------------------------------------------
// NORMALIZE HASHTAGS
// -----------------------------------------------------------------------------
//
// JSON is strict and always receives exactly one '#' per tag.
// The user can still type forgiving input such as:
//     rust #python ###programming
// -----------------------------------------------------------------------------

fn normalize_hashtags(value: &str) -> String {
    let mut seen = std::collections::BTreeSet::new();

    for raw_tag in value.split_whitespace() {
        let clean = raw_tag.trim_start_matches('#');

        if !clean.is_empty() {
            seen.insert(format!("#{clean}"));
        }
    }

    seen.into_iter().collect::<Vec<_>>().join(" ")
}

// -----------------------------------------------------------------------------
// NORMALIZE TAG LIST
// -----------------------------------------------------------------------------

fn normalize_tag_list(tags: &[String]) -> Result<Vec<String>> {
    let mut normalized = std::collections::BTreeSet::new();

    for raw_tag in tags {
        let clean = raw_tag.trim().trim_start_matches('#');

        if clean.is_empty() {
            continue;
        }

        if clean.chars().count() < 4 {
            bail!("Tag '{raw_tag}' must contain at least 4 characters.");
        }

        normalized.insert(format!("#{clean}"));
    }

    Ok(normalized.into_iter().collect())
}

// -----------------------------------------------------------------------------
// VALIDATE HASHTAGS
// -----------------------------------------------------------------------------

fn validate_hashtags(value: &str) -> Result<()> {
    for tag in parse_hashtags(value) {
        let clean = tag.trim_start_matches('#');

        if clean.chars().count() < 4 {
            bail!("Tag '{tag}' must contain at least 4 characters.");
        }
    }

    Ok(())
}

// -----------------------------------------------------------------------------
// PARSE HASHTAGS
// -----------------------------------------------------------------------------

fn parse_hashtags(value: &str) -> Vec<String> {
    normalize_hashtags(value)
        .split_whitespace()
        .map(ToString::to_string)
        .collect()
}

// -----------------------------------------------------------------------------
// FORMAT DESERIALIZATION ERROR
// -----------------------------------------------------------------------------

fn format_deserialize_error(
    path: &Path,
    file_text: &str,
    error: serde_path_to_error::Error<serde_json::Error>,
) -> String {
    let error_path = error.path().to_string();

    let json_error = error.into_inner();

    let line_number = json_error.line();

    let column_number = json_error.column();

    let mut message = format!("Dashboard data error in {}: {}", path.display(), json_error);

    if !error_path.is_empty() {
        message.push_str(&format!(" at .{error_path}"));
    }

    if line_number > 0 {
        message.push_str(&format!(" (line {line_number}, column {column_number})"));
    }

    if let Some(line_text) = file_text.lines().nth(line_number.saturating_sub(1)) {
        message.push_str(&format!("\n\n{line_number:>4} | {line_text}"));

        if column_number > 0 {
            let padding = " ".repeat(column_number.saturating_sub(1));

            message.push_str(&format!("\n     | {padding}^"));
        }
    }

    message
}

// -----------------------------------------------------------------------------
// BUTTON ROW NAME
// -----------------------------------------------------------------------------

impl ButtonRow {
    pub fn name(&self) -> &str {
        match self {
            Self::Divider { name } | Self::Link { name, .. } => name,
        }
    }
}
