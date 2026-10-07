#![allow(non_snake_case)]

use dioxus::{logger::tracing, prelude::*};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};


// ============================================================================
// MAIN
// ============================================================================

fn main() {
    init_db().expect("couldnt init the db... ugh");
    init_dummy_data().expect("couldnt put dummy data in");
    dioxus::launch(App);
}


// ============================================================================
// APP
// ============================================================================

#[component]
fn App() -> Element {
    rsx! {
        DatabaseView {}
    }
}


// ============================================================================
// DATA STRUCTURES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ButtonGroup {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ButtonUrl {
    pub id: i64,
    pub group_id: i64,
    pub name: String,
    pub url: String,
    pub hidden_password: Option<String>,
    pub hashtag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DashboardData {
    pub groups: Vec<ButtonGroup>,
    pub urls: Vec<ButtonUrl>,
}


// ============================================================================
// FRONTEND
// ============================================================================

#[component]
pub fn DatabaseView() -> Element {

    // ------------------------------------------------------------------------
    // Database data
    // ------------------------------------------------------------------------
    println!("ok....        ");

    let mut dashboard = use_signal(|| DashboardData {
        groups: vec![],
        urls: vec![],
    });

    // ------------------------------------------------------------------------
    // Modal state
    // ------------------------------------------------------------------------

    let mut modal = use_signal(|| Modal::None);

    // ------------------------------------------------------------------------
    // Load database when the component starts
    // ------------------------------------------------------------------------

    let dashboard_future = use_server_future(|| async {
        get_dashboard().await
    })?;

    // Put the initial database contents into our signal.
    if let Some(result) = dashboard_future.read().as_ref() {
        match result {
            Ok(data) => {
                if dashboard.read().groups.is_empty()
                    && dashboard.read().urls.is_empty()
                {
                    dashboard.set(data.clone());
                }
            }

            Err(err) => {
                tracing::error!("Could not load database: {:?}", err);
            }
        }
    }

    // ------------------------------------------------------------------------
    // Current data
    // ------------------------------------------------------------------------

    let data = dashboard.read();

    rsx! {
        div { style: "padding: 30px; font-family: sans-serif;",

            h1 { "SQLite Dashboard Test" }

            p { "This is talking to SQLite through Dioxus server functions." }

            button {
                onclick: move |_| {
                    modal.set(Modal::AddGroup);
                },

                "+ Add Button"
            }

            hr {}

            // ================================================================
            // BUTTON GROUPS
            // ================================================================
            for group in data.groups.iter() {

                div { style: "
                        border: 1px solid #aaa;
                        margin: 15px 0;
                        padding: 15px;
                        border-radius: 8px;
                    ",

                    div { style: "display: flex; justify-content: space-between;",

                        h2 { "{group.name}" }

                        div {

                            button {
                                onclick: {
                                    let id = group.id;
                                    let name = group.name.clone();

                                    move |_| {
                                        modal
                                            .set(Modal::EditGroup {
                                                id,
                                                name: name.clone(),
                                            });
                                    }
                                },

                                "✏️"
                            }

                            button {
                                onclick: {
                                    let id = group.id;

                                    move |_| async move {

                                        match delete_button(id).await {

                                            Ok(_) => {
                                                match get_dashboard().await {
                                                    Ok(new_data) => {
                                                        dashboard.set(new_data);
                                                    }

                                                    Err(err) => {
                                                        tracing::error!("Refresh failed: {:?}", err);
                                                    }
                                                }
                                            }
                                            Err(err) => {
                                                tracing::error!("Delete failed: {:?}", err);
                                            }
                                        }
                                    }
                                },

                                "🗑️"
                            }
                        }
                    }

                    // ========================================================
                    // URLS BELONGING TO THIS GROUP
                    // ========================================================
                    for url in data.urls.iter().filter(|u| u.group_id == group.id) {

                        div { style: "
                                padding: 10px;
                                margin: 5px 0;
                                background: #f5f5f5;
                                border-radius: 5px;
                            ",

                            div { style: "display: flex; justify-content: space-between;",

                                div {

                                    strong { "{url.name}" }

                                    br {}

                                    a { href: "{url.url}", target: "_blank", "{url.url}" }

                                    if let Some(hashtag) = &url.hashtag {
                                        span { "  {hashtag}" }
                                    }
                                }

                                div {

                                    button {
                                        onclick: {
                                            let url = url.clone();

                                            move |_| {
                                                modal
                                                    .set(Modal::EditUrl {
                                                        id: url.id,
                                                        group_id: url.group_id,
                                                        name: url.name.clone(),
                                                        url: url.url.clone(),
                                                        hidden_password: url.hidden_password.clone().unwrap_or_default(),
                                                        hashtag: url.hashtag.clone().unwrap_or_default(),
                                                    });
                                            }
                                        },

                                        "✏️"
                                    }

                                    button {
                                        onclick: {
                                            let id = url.id;

                                            move |_| async move {

                                                match delete_button_url(id).await {

                                                    Ok(_) => {
                                                        match get_dashboard().await {
                                                            Ok(new_data) => {
                                                                dashboard.set(new_data);
                                                            }

                                                            Err(err) => {
                                                                tracing::error!("Refresh failed: {:?}", err);
                                                            }
                                                        }
                                                    }
                                                    Err(err) => {
                                                        tracing::error!("Delete URL failed: {:?}", err);
                                                    }
                                                }
                                            }
                                        },

                                        "🗑️"
                                    }
                                }
                            }
                        }
                    }

                    button {
                        onclick: {
                            let group_id = group.id;

                            move |_| {
                                modal.set(Modal::AddUrl { group_id });
                            }
                        },

                        "+ Add URL"
                    }
                }
            }

            // =================================================================
            // MODALS
            // =================================================================
            match &*modal.read() {

                Modal::None => rsx! {},

                // =============================================================
                // ADD BUTTON
                // =============================================================
                Modal::AddGroup => rsx! {
                    ModalAddGroup {
                        on_close: move |_| {
                            modal.set(Modal::None);
                        },

                        on_saved: move |_| async move {
                            match get_dashboard().await {
                                Ok(new_data) => {
                                    dashboard.set(new_data);
                                    modal.set(Modal::None);
                                }

                                Err(err) => {
                                    tracing::error!("Refresh failed: {:?}", err);
                                }
                            }
                        },
                    }
                },
                Modal::EditGroup { id, name } => rsx! {
                    ModalEditGroup {
                        id: *id,
                        original_name: name.clone(),

                        on_close: move |_| {
                            modal.set(Modal::None);
                        },

                        on_saved: move |_| async move {
                            match get_dashboard().await {
                                Ok(new_data) => {
                                    dashboard.set(new_data);
                                    modal.set(Modal::None);
                                }

                                Err(err) => {
                                    tracing::error!("Refresh failed: {:?}", err);
                                }
                            }
                        },
                    }
                },
                Modal::AddUrl { group_id } => rsx! {
                    ModalAddUrl {
                        group_id: *group_id,

                        on_close: move |_| {
                            modal.set(Modal::None);
                        },

                        on_saved: move |_| async move {
                            match get_dashboard().await {
                                Ok(new_data) => {
                                    dashboard.set(new_data);
                                    modal.set(Modal::None);
                                }

                                Err(err) => {
                                    tracing::error!("Refresh failed: {:?}", err);
                                }
                            }
                        },
                    }
                },
                Modal::EditUrl { id, group_id: _, name, url, hidden_password, hashtag } => {
                    rsx! {
                        ModalEditUrl {
                            id: *id,
                            original_name: name.clone(),
                            original_url: url.clone(),
                            original_password: hidden_password.clone(),
                            original_hashtag: hashtag.clone(),

                            on_close: move |_| {
                                modal.set(Modal::None);
                            },

                            on_saved: move |_| async move {
                                match get_dashboard().await {
                                    Ok(new_data) => {
                                        dashboard.set(new_data);
                                        modal.set(Modal::None);
                                    }

                                    Err(err) => {
                                        tracing::error!("Refresh failed: {:?}", err);
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


// ============================================================================
// MODAL ENUM
// ============================================================================

#[derive(Clone, PartialEq)]
enum Modal {

    None,

    AddGroup,

    EditGroup {
        id: i64,
        name: String,
    },

    AddUrl {
        group_id: i64,
    },

    EditUrl {
        id: i64,
        group_id: i64,
        name: String,
        url: String,
        hidden_password: String,
        hashtag: String,
    },
}


// ============================================================================
// ADD GROUP MODAL
// ============================================================================

#[component]
fn ModalAddGroup(
    on_close: EventHandler<MouseEvent>,
    on_saved: EventHandler<()>,
) -> Element {

    let mut name = use_signal(String::new);

    rsx! {
        div { style: "position: fixed; inset: 0; background: rgba(0,0,0,.5); display:flex; align-items:center; justify-content:center;",

            div { style: "background:white; padding:30px; border-radius:10px; min-width:350px;",

                h2 { "Add Button" }

                input {
                    value: "{name}",

                    oninput: move |e| {
                        name.set(e.value());
                    },

                    placeholder: "Button name",
                }

                br {}
                br {}

                button {
                    onclick: move |_| async move {

                        if name.read().trim().is_empty() {
                            return;
                        }

                        match add_button(name.read().clone()).await {

                            Ok(_) => {
                                on_saved.call(());
                            }

                            Err(err) => {
                                tracing::error!("Add button failed: {:?}", err);
                            }
                        }
                    },

                    "Create"
                }

                button {
                    onclick: move |e| {
                        on_close.call(e);
                    },

                    "Cancel"
                }
            }
        }
    }
}


// ============================================================================
// EDIT GROUP MODAL
// ============================================================================

#[component]
fn ModalEditGroup(
    id: i64,
    original_name: String,
    on_close: EventHandler<MouseEvent>,
    on_saved: EventHandler<()>,
) -> Element {

    let mut name = use_signal(|| original_name.clone());

    rsx! {
        div { style: "position: fixed; inset: 0; background: rgba(0,0,0,.5); display:flex; align-items:center; justify-content:center;",

            div { style: "background:white; padding:30px; border-radius:10px; min-width:350px;",

                h2 { "Edit Button" }

                input {
                    value: "{name}",

                    oninput: move |e| {
                        name.set(e.value());
                    },
                }

                br {}
                br {}

                button {
                    onclick: move |_| async move {

                        match update_button(id, name.read().clone()).await {

                            Ok(_) => {
                                on_saved.call(());
                            }

                            Err(err) => {
                                tracing::error!("Update button failed: {:?}", err);
                            }
                        }
                    },

                    "Save"
                }

                button {
                    onclick: move |e| {
                        on_close.call(e);
                    },

                    "Cancel"
                }
            }
        }
    }
}


// ============================================================================
// ADD URL MODAL
// ============================================================================

#[component]
fn ModalAddUrl(
    group_id: i64,
    on_close: EventHandler<MouseEvent>,
    on_saved: EventHandler<()>,
) -> Element {

    let mut name = use_signal(String::new);
    let mut url = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut hashtag = use_signal(String::new);

    rsx! {
        div { style: "position: fixed; inset: 0; background: rgba(0,0,0,.5); display:flex; align-items:center; justify-content:center;",

            div { style: "background:white; padding:30px; border-radius:10px; min-width:450px;",

                h2 { "Add URL" }

                input {
                    placeholder: "URL name",

                    oninput: move |e| {
                        name.set(e.value());
                    },
                }

                br {}
                br {}

                input {
                    placeholder: "https://example.com",

                    oninput: move |e| {
                        url.set(e.value());
                    },
                }

                br {}
                br {}

                input {
                    placeholder: "Hidden password (optional)",
                    r#type: "password",

                    oninput: move |e| {
                        password.set(e.value());
                    },
                }

                br {}
                br {}

                input {
                    placeholder: "#hashtag (optional)",

                    oninput: move |e| {
                        hashtag.set(e.value());
                    },
                }

                br {}
                br {}

                button {
                    onclick: move |_| async move {

                        match add_button_url(
                                group_id,
                                name.read().clone(),
                                url.read().clone(),
                                optional_string(&password.read()),
                                optional_string(&hashtag.read()),
                            )
                            .await
                        {
                            Ok(_) => {
                                on_saved.call(());
                            }
                            Err(err) => {
                                tracing::error!("Add URL failed: {:?}", err);
                            }
                        }
                    },

                    "Create"
                }

                button {
                    onclick: move |e| {
                        on_close.call(e);
                    },

                    "Cancel"
                }
            }
        }
    }
}


// ============================================================================
// EDIT URL MODAL
// ============================================================================

#[component]
fn ModalEditUrl(
    id: i64,
    original_name: String,
    original_url: String,
    original_password: String,
    original_hashtag: String,
    on_close: EventHandler<MouseEvent>,
    on_saved: EventHandler<()>,
) -> Element {

    let mut name = use_signal(|| original_name.clone());
    let mut url = use_signal(|| original_url.clone());
    let mut password = use_signal(|| original_password.clone());
    let mut hashtag = use_signal(|| original_hashtag.clone());

    rsx! {
        div { style: "position: fixed; inset: 0; background: rgba(0,0,0,.5); display:flex; align-items:center; justify-content:center;",

            div { style: "background:white; padding:30px; border-radius:10px; min-width:450px;",

                h2 { "Edit URL" }

                input {
                    value: "{name}",

                    oninput: move |e| {
                        name.set(e.value());
                    },
                }

                br {}
                br {}

                input {
                    value: "{url}",

                    oninput: move |e| {
                        url.set(e.value());
                    },
                }

                br {}
                br {}

                input {
                    value: "{password}",
                    r#type: "password",

                    oninput: move |e| {
                        password.set(e.value());
                    },
                }

                br {}
                br {}

                input {
                    value: "{hashtag}",

                    oninput: move |e| {
                        hashtag.set(e.value());
                    },
                }

                br {}
                br {}

                button {
                    onclick: move |_| async move {

                        match update_button_url(
                                id,
                                name.read().clone(),
                                url.read().clone(),
                                optional_string(&password.read()),
                                optional_string(&hashtag.read()),
                            )
                            .await
                        {
                            Ok(_) => {
                                on_saved.call(());
                            }
                            Err(err) => {
                                tracing::error!("Update URL failed: {:?}", err);
                            }
                        }
                    },

                    "Save"
                }

                button {
                    onclick: move |e| {
                        on_close.call(e);
                    },

                    "Cancel"
                }
            }
        }
    }
}


// ============================================================================
// HELPER
// ============================================================================

fn optional_string(value: &str) -> Option<String> {

    let value = value.trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}


// ============================================================================
// DATABASE
// ============================================================================

fn init_db() -> rusqlite::Result<()> {

    let conn = Connection::open("database.db3")?;

    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS button_groups (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS button_urls (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            group_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            hidden_password TEXT,
            hashtag TEXT,

            FOREIGN KEY (group_id)
                REFERENCES button_groups(id)
        );
        ",
    )?;

    Ok(())
}

fn init_dummy_data() -> rusqlite::Result<()> {
    let conn = Connection::open("database.db3")?;

    // Add a couple of button groups
    conn.execute(
        "INSERT INTO button_groups (name) VALUES (?1)",
        ["Rust Programming"],
    )?;

    conn.execute(
        "INSERT INTO button_groups (name) VALUES (?1)",
        ["Dioxus Framework"],
    )?;

    // Get the IDs we just created
    let rust_id = conn.last_insert_rowid() - 1;
    let dioxus_id = conn.last_insert_rowid();

    // Add URLs to Rust Programming
    conn.execute(
        "INSERT INTO button_urls
            (group_id, name, url, hidden_password, hashtag)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            rust_id,
            "The Rust Book",
            "https://doc.rust-lang.org/book/",
            Option::<String>::None,
            Some("rust"),
        ),
    )?;

    conn.execute(
        "INSERT INTO button_urls
            (group_id, name, url, hidden_password, hashtag)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            rust_id,
            "Query.rs",
            "https://query.rs/",
            None::<String>,
            Some("search"),
        ),
    )?;

    // Add URLs to Dioxus Framework
    conn.execute(
        "INSERT INTO button_urls
            (group_id, name, url, hidden_password, hashtag)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            dioxus_id,
            "Dioxus Docs",
            "https://dioxuslabs.com/",
            None::<String>,
            Some("dioxus"),
        ),
    )?;

    Ok(())
}

// ============================================================================
// GET EVERYTHING
// ============================================================================

#[server]
async fn get_dashboard() -> Result<DashboardData, ServerFnError> {

    init_db()
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let conn = Connection::open("database.db3")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    // ------------------------------------------------------------------------
    // Groups
    // ------------------------------------------------------------------------

    let mut stmt = conn
        .prepare(
            "
            SELECT id, name
            FROM button_groups
            ORDER BY id
            "
        )
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let groups = stmt
        .query_map([], |row| {
            Ok(ButtonGroup {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ServerFnError::new(e.to_string()))?;


    // ------------------------------------------------------------------------
    // URLs
    // ------------------------------------------------------------------------

    let mut stmt = conn
        .prepare(
            "
            SELECT
                id,
                group_id,
                name,
                url,
                hidden_password,
                hashtag
            FROM button_urls
            ORDER BY id
            "
        )
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let urls = stmt
        .query_map([], |row| {
            Ok(ButtonUrl {
                id: row.get(0)?,
                group_id: row.get(1)?,
                name: row.get(2)?,
                url: row.get(3)?,
                hidden_password: row.get(4)?,
                hashtag: row.get(5)?,
            })
        })
        .map_err(|e| ServerFnError::new(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ServerFnError::new(e.to_string()))?;


    Ok(DashboardData {
        groups,
        urls,
    })
}


// ============================================================================
// CREATE BUTTON
// ============================================================================

#[server]
async fn add_button(name: String) -> Result<(), ServerFnError> {

    init_db()
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let conn = Connection::open("database.db3")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "
        INSERT INTO button_groups (name)
        VALUES (?1)
        ",
        [&name],
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}


// ============================================================================
// UPDATE BUTTON
// ============================================================================

#[server]
async fn update_button(
    id: i64,
    name: String,
) -> Result<(), ServerFnError> {

    let conn = Connection::open("database.db3")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "
        UPDATE button_groups
        SET name = ?1
        WHERE id = ?2
        ",
        rusqlite::params![name, id],
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}


// ============================================================================
// DELETE BUTTON
// ============================================================================

#[server]
async fn delete_button(id: i64) -> Result<(), ServerFnError> {

    let conn = Connection::open("database.db3")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "
        DELETE FROM button_groups
        WHERE id = ?1
        ",
        [id],
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}


// ============================================================================
// CREATE URL
// ============================================================================

#[server]
async fn add_button_url(
    group_id: i64,
    name: String,
    url: String,
    hidden_password: Option<String>,
    hashtag: Option<String>,
) -> Result<(), ServerFnError> {

    let conn = Connection::open("database.db3")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "
        INSERT INTO button_urls
            (
                group_id,
                name,
                url,
                hidden_password,
                hashtag
            )
        VALUES
            (?1, ?2, ?3, ?4, ?5)
        ",
        rusqlite::params![
            group_id,
            name,
            url,
            hidden_password,
            hashtag
        ],
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}


// ============================================================================
// UPDATE URL
// ============================================================================

#[server]
async fn update_button_url(
    id: i64,
    name: String,
    url: String,
    hidden_password: Option<String>,
    hashtag: Option<String>,
) -> Result<(), ServerFnError> {

    let conn = Connection::open("database.db3")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "
        UPDATE button_urls

        SET
            name = ?1,
            url = ?2,
            hidden_password = ?3,
            hashtag = ?4

        WHERE id = ?5
        ",
        rusqlite::params![
            name,
            url,
            hidden_password,
            hashtag,
            id
        ],
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}


// ============================================================================
// DELETE URL
// ============================================================================

#[server]
async fn delete_button_url(id: i64) -> Result<(), ServerFnError> {

    let conn = Connection::open("database.db3")
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    conn.execute(
        "
        DELETE FROM button_urls
        WHERE id = ?1
        ",
        [id],
    )
    .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

