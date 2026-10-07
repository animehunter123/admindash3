# Admindash3 Developer Guide

> **A beginner-friendly tour of the Admindash3 Rust + Dioxus codebase**
>
> This document is meant to be read **from top to bottom**. You do not need to already be a Rust expert or a Dioxus expert. The goal is to explain what the project is doing, why it is structured this way, and which Rust/Dioxus ideas are hiding inside the code.
>
> Think of this file as the project's **guided museum tour**. We are going to walk past every room, open the interesting drawers, and occasionally poke the machinery just to see what happens. 😄

---

## 1. What is Admindash3?

Admindash3 is a web dashboard built with:

- **Rust** — the programming language.
- **Dioxus** — the UI framework.
- **Dioxus Fullstack / server functions** — lets the browser UI call Rust functions that execute on the server.
- **Serde / serde_json** — converts Rust structs to and from JSON.
- **WebAssembly (WASM)** — runs the interactive Rust UI in the browser.
- **web-sys / wasm-bindgen / js-sys** — lets Rust interact with browser APIs when necessary.
- **Anyhow** — convenient application-level error handling.
- **Tailwind-style CSS classes** — used directly in the RSX markup.

At a high level, the application looks like this:

```text
                         ┌──────────────────────┐
                         │       Browser        │
                         │                      │
                         │  Dioxus UI / WASM    │
                         │  Navbar              │
                         │  Search              │
                         │  Buttons             │
                         │  Modals              │
                         └──────────┬───────────┘
                                    │
                         Dioxus server functions
                                    │
                                    ▼
                         ┌──────────────────────┐
                         │       Rust Server    │
                         │                      │
                         │ dashboard_data.rs    │
                         │ history.rs           │
                         │ filesystem / JSON    │
                         └──────────┬───────────┘
                                    │
                    ┌───────────────┼────────────────┐
                    ▼               ▼                ▼
             data_buttons.v2   data_tags.json   system_preferences.json
                    │
                    ▼
             dashboard_history/
```

There is an important architectural idea here:

> **The UI should not directly manipulate the dashboard JSON file.**

Instead:

```text
Button click
    ↓
Dioxus event handler
    ↓
server function
    ↓
dashboard_data.rs
    ↓
validate / modify / backup
    ↓
write JSON
    ↓
refresh dashboard
    ↓
UI displays new data
```

That separation is one of the most useful things to learn from this project.

---

# 2. The project source map

The supplied project contains these Rust files:

```text
./build.rs

./src/
├── main.rs
├── buttons.rs
├── dashboard_data.rs
├── data_navbar.rs
├── footer.rs
├── history.rs
├── navbar.rs
└── bin/
    └── data_button_v2_migrator.rs
```

Here is the one-line version of what each file does:

| File | Main job |
|---|---|
| `build.rs` | Gets the Git commit hash during compilation |
| `src/main.rs` | Starts the application, owns global state, loads the dashboard |
| `src/buttons.rs` | Displays dashboard buttons and contains the admin editing UI |
| `src/dashboard_data.rs` | The data/model layer: JSON, validation, CRUD, ordering, tags, preferences |
| `src/data_navbar.rs` | Defines the JSON structure used by the navigation bar |
| `src/footer.rs` | Displays the application version |
| `src/history.rs` | Displays and manages dashboard JSON snapshots |
| `src/navbar.rs` | Navigation bar, search, login/logout, browser-side UI behavior |
| `src/bin/data_button_v2_migrator.rs` | Command-line migration tool for old dashboard JSON |

The easiest way to understand the application is:

```text
main.rs
  │
  ├── navbar.rs
  │
  ├── buttons.rs
  │       │
  │       └── dashboard_data.rs
  │
  ├── history.rs
  │       │
  │       └── dashboard_data.rs
  │
  └── footer.rs

data_navbar.rs
  └── provides NavItem used by main.rs / navbar.rs

build.rs
  └── provides GIT_HASH at compile time

bin/data_button_v2_migrator.rs
  └── reuses dashboard_data.rs migration code
```

---

# 3. Before reading the code: the Rust ideas you will see everywhere

If you are new to Rust, several things in this project may look strange at first.

That is normal.

The same handful of concepts appear again and again.

---

## 3.1 `let`

```rust
let name = "Admindash3";
```

`let` creates a variable.

Rust normally infers the type:

```rust
let name = "Admindash3";
```

is understood as a string slice:

```rust
let name: &str = "Admindash3";
```

You can also explicitly specify a type:

```rust
let count: usize = 10;
```

---

## 3.2 `let mut`

Rust variables are immutable by default.

```rust
let name = String::from("hello");
```

You cannot later do:

```rust
name = String::from("goodbye");
```

If you want to change the variable:

```rust
let mut name = String::from("hello");

name = String::from("goodbye");
```

The `mut` means:

> "This variable is allowed to change."

You will see `mut` constantly in Dioxus because signals and temporary collections often need to be changed.

---

# 4. References: `&T`

One of the most important Rust concepts in this project is borrowing.

Suppose:

```rust
fn print_name(name: &String) {
    println!("{name}");
}
```

The `&String` means:

> "Give me access to the String, but do not give ownership of it to me."

This matters because Rust carefully controls who owns data.

You will frequently see:

```rust
&str
```

which means a borrowed string slice.

For example:

```rust
fn clean_required(label: &str, value: &str)
```

The function does not need to own the strings. It only needs to inspect them.

---

# 5. `.clone()`

You will see a lot of this:

```rust
let copy = original.clone();
```

A beginner often asks:

> "Why are we cloning everything?!"

Because Rust ownership rules matter.

If a closure needs to own some data while the original code also needs that data, cloning can give the closure its own copy.

For example:

```rust
let button_name = button.name.clone();

move |_| {
    println!("{button_name}");
}
```

The closure owns its copy of `button_name`.

This is often especially useful with Dioxus event handlers.

### Important learning point

Do not treat `.clone()` as automatically bad.

First understand ownership.

Then, once the application works, you can study whether a particular clone is actually necessary.

---

# 6. `Option<T>`

`Option` means:

> "There may or may not be a value."

It has two possibilities:

```rust
Some(value)
```

or:

```rust
None
```

For example:

```rust
let selected_button_name: Option<String> = None;
```

means:

> "There currently isn't a selected button."

Later:

```rust
selected_button_name.set(Some("Servers".to_string()));
```

means:

> "There is now a selected button, and its name is `Servers`."

You will see patterns like:

```rust
if let Some(button) = selected_button {
    // We have a button.
}
```

This is a very convenient way of saying:

> "If there is a value, give it to me and run this code."

---

# 7. `Result<T, E>`

`Result` means:

> "This operation can succeed or fail."

For example:

```rust
fn load_file() -> Result<String, Error>
```

could return:

```rust
Ok(file_contents)
```

or:

```rust
Err(error)
```

Admindash3 uses this heavily because filesystem and JSON operations can fail.

You will see:

```rust
match result {
    Ok(value) => {
        // Success
    }

    Err(error) => {
        // Failure
    }
}
```

This is one of Rust's superpowers:

> Errors are values that the program is encouraged to handle explicitly.

---

# 8. `?` — the "if this fails, return the error" operator

Consider:

```rust
let text = std::fs::read_to_string(path)?;
```

The `?` means roughly:

```text
Try the operation.

If it succeeds:
    continue.

If it fails:
    return the error from this function.
```

It saves a huge amount of repetitive error-handling code.

You will see `?` throughout `dashboard_data.rs` and `history.rs`.

---

# 9. `match`

Rust's `match` is similar to a powerful `switch`.

Example:

```rust
match row {
    ButtonRowView::Divider { name } => {
        // Divider
    }

    ButtonRowView::Link { name, url, .. } => {
        // Link
    }
}
```

This is especially useful because `ButtonRowView` is an enum.

Rust can make sure you handle all of the enum's variants.

---

# 10. Structs

A struct groups related data together.

Example:

```rust
struct SearchResult {
    button_name: String,
    row_name: String,
    url: String,
}
```

You can then create one:

```rust
let result = SearchResult {
    button_name: "Servers".to_string(),
    row_name: "Proxmox".to_string(),
    url: "https://example.com".to_string(),
};
```

Think of a struct as:

> "Here is one thing, and these are its properties."

---

# 11. Enums

Enums are one of Rust's nicest features.

An enum represents one of several possible forms.

Admindash3 has:

```rust
enum RowKind {
    Link,
    Divider,
}
```

A row can therefore be:

```text
Link
```

or:

```text
Divider
```

It cannot secretly be something else.

Another example is:

```rust
enum ButtonEditorMode {
    Create,
    Edit { original_name: String },
}
```

The editor can therefore explicitly know whether it is creating a new button or editing an existing one.

---

# 12. `impl`

`impl` adds behavior to a type.

For example:

```rust
impl Default for ButtonDraft {
    fn default() -> Self {
        ...
    }
}
```

This says:

> "Here is how Rust should create a default `ButtonDraft`."

You will also see methods such as:

```rust
impl ButtonRow {
    pub fn name(&self) -> &str {
        ...
    }
}
```

Now a row can do:

```rust
row.name()
```

instead of repeating the `match` logic everywhere.

---

# 13. Traits and `derive`

You will see things such as:

```rust
#[derive(Clone, PartialEq)]
```

and:

```rust
#[derive(Clone, PartialEq, Deserialize, Serialize)]
```

`derive` asks Rust to automatically implement useful behavior.

### `Clone`

Allows:

```rust
value.clone()
```

### `PartialEq`

Allows comparisons such as:

```rust
a == b
```

### `Serialize`

Allows a Rust value to become JSON.

### `Deserialize`

Allows JSON to become a Rust value.

This is a major part of how the dashboard data layer works.

---

# 14. Closures

A closure is a small function stored inline.

For example:

```rust
move |_| {
    println!("Clicked!");
}
```

Dioxus uses closures everywhere for events:

```rust
onclick: move |_| {
    ...
}
```

The `_` means:

> "There is an argument, but I don't care about its value."

For example, a click event gives the closure an event.

If you do not need the event:

```rust
move |_| { ... }
```

is enough.

---

# 15. Why `move` appears everywhere

You will constantly see:

```rust
move |_| {
    ...
}
```

`move` means:

> "Move the variables this closure uses into the closure."

This is particularly important because event handlers may execute later.

For example:

```rust
let name = button.name.clone();

button {
    onclick: move |_| {
        println!("{name}");
    }
}
```

The event handler owns `name`.

This is one of the areas where Rust's ownership system becomes very visible.

---

# 16. Async Rust

Some functions are:

```rust
async fn ...
```

An async function can wait for something without blocking the entire application thread.

For example:

```rust
let result = some_server_function().await;
```

`.await` means:

> "Pause this async operation until the result is available."

The important rule is:

> `.await` can only be used inside an `async` context.

That distinction matters a lot when writing Dioxus event handlers.

Correct:

```rust
onclick: move |_| async move {
    let result = some_server_function().await;
}
```

A common beginner mistake is trying:

```rust
onclick: move |_| {
    let result = some_server_function().await;
}
```

That does not work because the closure itself is not async.

---

# 17. Dioxus fundamentals

Dioxus is a component-based UI framework.

A component looks like:

```rust
#[component]
fn MyComponent() -> Element {
    rsx! {
        div {
            "Hello!"
        }
    }
}
```

The important pieces are:

```rust
#[component]
```

marks a Dioxus component.

```rust
Element
```

is what the component returns.

```rust
rsx!
```

is Dioxus's markup syntax.

---

# 18. RSX

RSX looks a little like HTML mixed with Rust.

Example:

```rust
button {
    onclick: move |_| {
        ...
    },

    "Click me"
}
```

This corresponds conceptually to:

```html
<button>
    Click me
</button>
```

But RSX lets you embed Rust directly.

For example:

```rust
div {
    "{button_name}"
}
```

The `{button_name}` is Rust data rendered into the UI.

---

# 19. Signals

Dioxus signals are reactive state.

For example:

```rust
let mut search = use_signal(String::new);
```

This creates state.

When the state changes:

```rust
search.set("proxmox".to_string());
```

Dioxus can re-render the parts of the UI that depend on it.

This is one of the central ideas in modern UI programming:

```text
state changes
      ↓
UI reacts
```

Instead of manually telling the browser:

> "Change this div, then update that text, then hide this element..."

you change the state and let the framework update the UI.

---

# 20. `GlobalSignal`

Admindash3 also uses global signals.

From `main.rs`:

```rust
pub static ADMIN_AUTH: GlobalSignal<AdminCredentials> =
    Signal::global(AdminCredentials::default);
```

This means the authentication state is accessible across components.

The project uses four important global signals:

```text
IFRAME_URL
ADMIN_AUTH
DASHBOARD_REFRESH_KEY
HISTORY_PAGE
```

Think of these as a small shared state board:

```text
┌──────────────────────────────┐
│        Global State          │
├──────────────────────────────┤
│ IFRAME_URL                   │
│ ADMIN_AUTH                   │
│ DASHBOARD_REFRESH_KEY        │
│ HISTORY_PAGE                 │
└──────────────────────────────┘
```

---

# 21. Reading and writing signals

You will see:

```rust
ADMIN_AUTH()
```

to read the current value.

You will see:

```rust
*ADMIN_AUTH.write() = credentials;
```

to replace the value.

For a refresh counter:

```rust
*DASHBOARD_REFRESH_KEY.write() += 1;
```

That is a clever little pattern.

The number itself is not important.

The change is.

The dashboard is watching that state, so incrementing it tells Dioxus:

> "Something happened. Fetch the dashboard again."

---

# 22. `use_signal`

Example:

```rust
let mut selected_button_name =
    use_signal(|| None::<String>);
```

This creates component-local state.

The initial value is:

```rust
None
```

and the type is:

```rust
Option<String>
```

Later:

```rust
selected_button_name.set(Some(button_name));
```

---

# 23. `use_effect`

You will see:

```rust
use_effect(move || {
    ...
});
```

An effect is used for work that should happen because the component is rendered or because values it depends on change.

Examples in this project include:

- synchronizing autosort state,
- reading pending URL/hash state,
- adding browser keyboard listeners,
- managing focus.

A useful mental model is:

```text
Normal component code
    ↓
describes UI/state

use_effect
    ↓
does something as a side effect
```

---

# 24. `use_drop`

When the project installs a browser event listener, it also needs to remove it.

This pattern:

```rust
use_drop(move || {
    ...
});
```

provides cleanup.

That is extremely important for browser event listeners.

Without cleanup, repeatedly mounting/unmounting a component could leave old event handlers hanging around.

---

# 25. Server functions

Admindash3 uses Dioxus server functions such as:

```rust
#[server]
async fn load_dashboard(...) -> Result<DashboardPayload, ServerFnError>
```

This is a really useful concept.

The UI can call:

```rust
load_dashboard(credentials).await
```

while the implementation runs on the server side.

Conceptually:

```text
Browser
   │
   │ load_dashboard(...)
   ▼
Dioxus server-function boundary
   │
   ▼
Rust server
   │
   ▼
dashboard_data.rs
   │
   ▼
JSON file
```

This is much cleaner than putting filesystem access directly into browser code.

---

# 26. The most important architecture rule

A useful way to mentally divide this project is:

### UI layer

```text
main.rs
buttons.rs
navbar.rs
footer.rs
history.rs UI
```

### Data/model layer

```text
dashboard_data.rs
data_navbar.rs
```

### Build tooling

```text
build.rs
src/bin/data_button_v2_migrator.rs
```

The UI asks the data layer to do things.

The UI should not need to understand the physical JSON layout every time it wants to add a row.

---

# 27. `build.rs`

## Purpose

`build.rs` is a Cargo build script.

The supplied file:

```text
./build.rs
```

runs during the build process.

It uses:

```rust
use std::process::Command;
```

and executes:

```text
git rev-parse --short HEAD
```

That obtains the current Git commit's short hash.

For example:

```text
a83f91c
```

The build script then tells Cargo:

```rust
println!("cargo:rustc-env=GIT_HASH={}", git_hash.trim());
```

This creates a compile-time environment variable named:

```text
GIT_HASH
```

The application can later use:

```rust
env!("GIT_HASH")
```

---

## Why this is cool

The application can display:

```text
Admindash3 v1.0.0-a83f91c
```

The version comes from Cargo:

```rust
env!("CARGO_PKG_VERSION")
```

and the Git hash comes from `build.rs`:

```rust
env!("GIT_HASH")
```

So the binary knows exactly which source revision produced it.

### Rust concept to learn

`build.rs` is not normal runtime code.

It is:

```text
build time
   ↓
generate configuration/environment
   ↓
compile application
   ↓
runtime application can read it
```

---

# 28. `src/main.rs`

`main.rs` is the application entry point and the place where the major pieces are assembled.

It:

1. imports the modules,
2. defines global state,
3. starts Dioxus,
4. loads the navbar,
5. loads the dashboard,
6. displays loading/error states,
7. renders the main application layout,
8. exposes server functions for data loading.

---

## 28.1 Module declarations

The file declares:

```rust
mod buttons;
mod data_navbar;
mod dashboard_data;
mod footer;
mod history;
mod navbar;
```

This tells Rust:

> "These other source files are modules belonging to this application."

Then it imports the things it needs:

```rust
use crate::buttons::*;
use crate::data_navbar::NavItem;
use crate::dashboard_data::{AdminCredentials, DashboardPayload};
use crate::footer::Footer;
use crate::history::HistoryPage;
use crate::navbar::Navbar;
```

---

# 29. The four global signals in `main.rs`

## `IFRAME_URL`

```rust
pub static IFRAME_URL: GlobalSignal<Option<String>> =
    Signal::global(|| None);
```

This stores the URL currently selected for the iframe/page content.

It starts as:

```text
None
```

which means no URL is selected.

---

## `ADMIN_AUTH`

```rust
pub static ADMIN_AUTH: GlobalSignal<AdminCredentials> =
    Signal::global(AdminCredentials::default);
```

This stores the current administrator credentials/state.

It is shared by:

- navbar,
- button editor,
- row editor,
- server function calls,
- history controls.

---

## `DASHBOARD_REFRESH_KEY`

```rust
pub static DASHBOARD_REFRESH_KEY: GlobalSignal<u64> =
    Signal::global(|| 0);
```

This is a refresh trigger.

It is not really "data".

It is more like a doorbell.

When something changes:

```rust
*DASHBOARD_REFRESH_KEY.write() += 1;
```

the dashboard loader sees that change and runs again.

---

## `HISTORY_PAGE`

```rust
pub static HISTORY_PAGE: GlobalSignal<bool> =
    Signal::global(|| false);
```

This acts as a very small page switch.

```text
false → normal dashboard
true  → history page
```

The project does not need a full router just to switch between these two views.

---

# 30. `main()`

The entry point:

```rust
fn main() -> Result<()> {
```

builds the version string:

```rust
let version =
    concat!(env!("CARGO_PKG_VERSION"), "-", env!("GIT_HASH"));
```

and starts Dioxus:

```rust
dioxus::launch(App);
```

The important chain is:

```text
main()
  ↓
dioxus::launch(App)
  ↓
App()
  ↓
Navbar / Buttons / Footer / HistoryPage
```

---

# 31. `App()`

`App()` is the main Dioxus component.

Its job is orchestration.

It does not contain every little button-editing rule.

Instead it connects the major pieces:

```text
App
├── Navbar
├── Dashboard
│   └── Buttons
├── HistoryPage
└── Footer
```

This is a good example of component composition.

---

# 32. Dashboard loading in `App()`

The project deliberately uses:

```rust
use_server_future(...)
```

for dashboard loading.

This is important because the application previously used `use_resource()` and experienced a visual loading/flicker problem.

The dashboard loader gets the current credentials and requests:

```rust
load_dashboard(...)
```

from the server.

The returned object is:

```rust
DashboardPayload
```

That payload becomes the data passed into the UI.

---

# 33. Cached dashboard data

The application also keeps the last successful dashboard available while a refresh is happening.

This is important because a server future can temporarily be suspended while new data is loading.

The conceptual flow is:

```text
Existing dashboard
       │
       │ refresh requested
       ▼
keep showing existing dashboard
       │
       │ server request
       ▼
new dashboard arrives
       │
       ▼
replace cached dashboard
```

This gives the UI a smoother experience.

---

# 34. `LoadingPanel()`

`LoadingPanel()` is a small presentation component.

Its job is simply:

> "Tell the user the application is currently loading."

This is a good example of why small components are useful.

Instead of putting a giant block of markup into `App()`, the loading UI gets its own named component.

---

# 35. `ErrorPanel(message)`

`ErrorPanel()` displays a dashboard loading error.

Its parameter:

```rust
message: String
```

is the error text to show.

This teaches a useful Dioxus pattern:

```text
data/state
   ↓
component receives value
   ↓
component renders it
```

---

# 36. `load_navbar()`

```rust
async fn load_navbar()
    -> Result<Vec<NavItem>, ServerFnError>
```

This server function loads the navbar JSON and converts it into:

```rust
Vec<NavItem>
```

A `Vec<T>` is Rust's growable array/list.

So:

```rust
Vec<NavItem>
```

means:

> "A list of navigation items."

---

# 37. `load_dashboard()`

```rust
async fn load_dashboard(
    credentials: AdminCredentials,
) -> Result<DashboardPayload, ServerFnError>
```

This is the server boundary for loading dashboard data.

It delegates the actual data work to `dashboard_data.rs`.

This is an important separation:

```text
main.rs
    "I need the dashboard."

dashboard_data.rs
    "Here is how the dashboard is actually loaded."
```

---

# 38. `src/data_navbar.rs`

This file is tiny — and that is good.

It defines:

```rust
pub struct NavItem {
    pub name: String,
    pub url: Option<String>,
    pub children: Vec<NavItem>,
}
```

The important part is that `NavItem` is recursive.

A navigation item can contain more `NavItem`s:

```text
NavItem
 ├── name
 ├── url
 └── children
       ├── NavItem
       ├── NavItem
       └── NavItem
```

That makes nested menus possible.

---

# 39. Why `url` is an `Option<String>`

A navbar item might have a URL:

```json
{
    "name": "Proxmox",
    "url": "https://proxmox.example",
    "children": []
}
```

But a dropdown heading might not have its own URL:

```json
{
    "name": "Servers",
    "url": null,
    "children": [...]
}
```

Therefore:

```rust
Option<String>
```

is the correct model.

---

# 40. `Deserialize` and `Serialize` on `NavItem`

The type derives:

```rust
#[derive(Clone, PartialEq, Deserialize, Serialize)]
```

This allows it to:

- be cloned,
- be compared,
- be loaded from JSON,
- be written to JSON.

This is the basic bridge between:

```text
JSON
  ↕
Rust struct
```

---

# 41. `src/footer.rs`

The footer is intentionally simple.

Its main component is:

```rust
pub fn Footer() -> Element
```

It builds the version string:

```rust
concat!(
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("GIT_HASH")
)
```

and renders:

```text
Admindash3 v<version>-<git hash>
```

This is a nice example of a component with:

- no local state,
- no server function,
- no inputs,
- no complicated logic.

Sometimes a component should just render something.

---

# 42. `src/dashboard_data.rs`

This is the **heart of the application's data layer**.

If `buttons.rs` is the dashboard's hands and `navbar.rs` is its face, `dashboard_data.rs` is its filing cabinet. 📁

It is responsible for:

- dashboard data structures,
- JSON parsing,
- JSON validation,
- admin credentials,
- CRUD operations,
- tag management,
- ordering,
- autosort preferences,
- history backups,
- migration from the old data format,
- input validation.

This is the file worth studying carefully if you want to become comfortable with Rust.

---

# 43. Dashboard file constants

The file defines:

```rust
pub const BUTTONS_V1_FILE: &str = "./data_buttons.json";
pub const BUTTONS_V2_FILE: &str = "./data_buttons.v2.json";
pub const TAGS_FILE: &str = "./data_tags.json";
pub const SYSTEM_PREFERENCES_FILE: &str = "./system_preferences.json";
pub const HISTORY_DIR: &str = "./dashboard_history";
pub const MAX_HISTORY_SNAPSHOTS: usize = 100;
```

These are centralized configuration values.

Instead of writing:

```rust
"./data_buttons.v2.json"
```

in twenty different places, the application has one constant.

That is much easier to maintain.

---

# 44. Default administrator credentials

The supplied code defines:

```rust
pub const ADMIN_USERNAME: &str = "admin";
pub const ADMIN_PASSWORD: &str = "admin";
```

The actual configured values can also come from environment variables through:

```rust
configured_admin_username()
configured_admin_password()
```

The important learning idea is:

> Configuration should be separated from application logic.

---

# 45. `AdminCredentials`

This struct represents the credentials supplied to administrative operations.

The rest of the application can pass one value:

```rust
AdminCredentials
```

instead of passing username and password separately everywhere.

It also has:

```rust
is_admin()
```

which centralizes the check:

```text
Are these credentials valid?
```

---

# 46. `DashboardPayload`

`DashboardPayload` is the data sent to the UI.

It contains the information the dashboard needs to render:

- buttons,
- available/managed tags,
- autosort state.

This is a very important architectural boundary.

The UI does not need to know every internal JSON representation.

It gets a prepared view model:

```text
JSON/filesystem
      ↓
dashboard_data.rs
      ↓
DashboardPayload
      ↓
Dioxus UI
```

---

# 47. `ButtonGroupView`

A `ButtonGroupView` represents a main dashboard button/card and its rows in a form suitable for rendering.

The word `View` is useful here.

It indicates:

> "This is shaped for the UI."

That is subtly different from the raw stored representation.

---

# 48. `ButtonRowView`

The UI row is represented by an enum:

```rust
pub enum ButtonRowView {
    Divider { ... },
    Link { ... },
}
```

This is an excellent Rust example.

Instead of having one giant struct with a bunch of fields that may or may not apply:

```text
name
url?
divider?
secret?
...
```

the enum says:

```text
This is either a Divider
OR
This is a Link
```

The compiler helps keep those cases separate.

---

# 49. `RowKind`

The editable form uses:

```rust
pub enum RowKind {
    Link,
    Divider,
}
```

This is the editor's representation of the same basic concept.

When the user clicks:

```text
Link
Divider
```

the state changes between these enum variants.

---

# 50. Input structs

The data layer defines:

```rust
ButtonCreateInput
ButtonRenameInput
ButtonRowInput
```

These are command objects.

Instead of a function like:

```rust
create_button(
    username,
    password,
    name,
    first_row_name,
    first_row_url,
    hashtags,
    secrets,
)
```

the function can receive:

```rust
ButtonCreateInput
```

This is easier to extend later.

---

# 51. `DashboardDocument`

This represents the validated internal dashboard document.

It is useful to think of the application as having multiple layers of data:

```text
JSON on disk
    ↓
deserialize
    ↓
DashboardDocument
    ↓
validated / transformed
    ↓
DashboardPayload
    ↓
Dioxus UI
```

---

# 52. `ButtonGroup` and `ButtonRow`

These are the stored model types.

`ButtonRow` is an enum:

```text
Divider
Link
```

The internal stored representation also has supporting types such as:

```text
StoredRow
StoredLink
```

This allows the file format and UI representation to remain somewhat independent.

---

# 53. `SystemPreferences`

This stores system-level settings.

The important current setting is autosort.

The application can therefore remember:

```text
Autosort ON
```

or:

```text
Autosort OFF
```

instead of losing the setting after a refresh.

---

# 54. `configured_admin_username()`

This function determines which administrator username is configured.

The important idea is:

```text
environment configuration
       ↓
Rust function
       ↓
application configuration
```

This lets deployment configuration override code defaults.

---

# 55. `configured_admin_password()`

This does the same thing for the administrator password.

The current project uses the configured password directly rather than a password-hashing system.

That is worth remembering as a learning/development limitation.

---

# 56. `load_dashboard_payload_from_path()`

```rust
pub fn load_dashboard_payload_from_path(
    path: &Path,
    credentials: &AdminCredentials,
) -> Result<DashboardPayload>
```

This is a major entry point into the data layer.

It:

1. checks/uses the supplied credentials,
2. reads the dashboard,
3. builds the UI-friendly payload,
4. returns the result.

A `Path` is Rust's filesystem path type.

Using:

```rust
&Path
```

means the function borrows the path instead of taking ownership.

---

# 57. `read_dashboard_document()`

```rust
pub fn read_dashboard_document(
    path: &Path,
) -> Result<DashboardDocument>
```

This reads the dashboard file and converts it into the internal document representation.

Conceptually:

```text
file
 ↓
read text
 ↓
parse JSON
 ↓
deserialize
 ↓
DashboardDocument
```

---

# 58. `check_dashboard_json()`

```rust
pub fn check_dashboard_json(path: &Path) -> Result<()>
```

This validates the JSON without needing to modify it.

The UI exposes this as:

```text
+JSON Check
```

The project goes beyond simply saying:

```text
JSON is broken
```

and builds useful error information including:

- line,
- column,
- JSON path,
- source line,
- caret position.

That is a great example of turning a technical error into a useful user-facing error.

---

# 59. `parse_dashboard_document()`

```rust
pub fn parse_dashboard_document(
    path: &Path,
    file_text: &str,
) -> Result<DashboardDocument>
```

This performs the actual parsing.

Notice the two inputs:

```text
path
file_text
```

Why both?

Because if JSON parsing fails, the program can report:

```text
which file
which line
which column
what JSON path
what source text
```

instead of only reporting a generic parser error.

---

# 60. `backup_dashboard_file()`

Before modifying the dashboard, the application creates a history snapshot.

Conceptually:

```text
existing dashboard
        ↓
make backup
        ↓
modify dashboard
        ↓
write new dashboard
```

This is the foundation of the history/revert system.

---

# 61. `make_history_filename()`

This creates timestamp-based snapshot names such as:

```text
dashboard-20260921_061530_123.json
```

The timestamp is sortable.

That means filenames naturally sort chronologically.

If two changes occur in the same millisecond, the code adds a collision suffix rather than overwriting an existing snapshot.

This is a nice example of defensive programming.

---

# 62. `civil_date_from_days()`

This helper converts a count of days since the Unix epoch into:

```text
year
month
day
```

It avoids requiring an external date crate for this specific calculation.

For a beginner, the important lesson is not memorizing the algorithm.

The lesson is:

> A small helper can isolate ugly/technical calculations so the rest of the application remains readable.

---

# 63. `is_history_snapshot_filename()`

This validates whether a filename looks like a valid dashboard history snapshot.

That is important for safety.

The history directory may contain files, but the application should not blindly treat every file as a valid dashboard snapshot.

---

# 64. `write_dashboard_document()`

This is the central write operation.

The conceptual job is:

```text
DashboardDocument
      ↓
serialize
      ↓
backup old dashboard
      ↓
write new JSON
```

Centralizing writes makes it much easier to enforce consistent behavior.

---

# 65. CRUD functions

The data layer provides:

```text
create_button()
rename_button()
delete_button()

save_row()
delete_row()
```

CRUD stands for:

```text
C = Create
R = Read
U = Update
D = Delete
```

The dashboard therefore has a complete CRUD workflow.

---

# 66. `create_button()`

Creates a new main dashboard button.

The editor intentionally creates a button with its first row.

This protects the data model from creating an empty/invalid button structure.

---

# 67. `rename_button()`

Changes an existing main button's name.

It receives:

```rust
ButtonRenameInput
```

which contains the old/original name and new name.

The original name matters because the application needs to find the existing object before changing it.

---

# 68. `delete_button()`

Removes an entire main dashboard button.

The UI only exposes this operation while editing an existing button.

After deletion, the UI can refresh the dashboard.

---

# 69. `save_row()`

Creates or updates a row within a button.

The row can be:

```text
Link
```

or:

```text
Divider
```

The function receives:

```rust
ButtonRowInput
```

which contains the editor's current values.

---

# 70. `delete_row()`

Deletes a row from a button.

The function receives:

```text
button_name
row_name
```

so it knows where the row lives.

---

# 71. `load_system_preferences()`

Reads:

```text
system_preferences.json
```

and returns:

```rust
SystemPreferences
```

This is where persistent system-wide UI settings come from.

---

# 72. `set_system_autosort()`

This saves the autosort setting.

The UI can say:

```text
+Autosort ON
```

or:

```text
+Autosort OFF
```

and this function stores that choice.

The setting therefore survives F5 and applies system-wide.

---

# 73. `load_managed_tags()`

Loads the tag configuration.

Tags are kept separately from the main dashboard data.

That means the project has a distinction between:

```text
dashboard content
```

and:

```text
managed tag definitions
```

---

# 74. `save_managed_tags()`

Writes the managed tags.

It also participates in the admin authorization flow.

This prevents an ordinary visitor from modifying the global tag list.

---

# 75. `reorder_buttons()`

This changes the order of the main dashboard buttons.

It receives:

```text
from_index
to_index
```

For example:

```text
Before:

0  Servers
1  Storage
2  Network
3  Monitoring

Move index 3 → index 0

After:

0  Monitoring
1  Servers
2  Storage
3  Network
```

This is a good place to study:

- `Vec`,
- indexes,
- mutation,
- bounds checking,
- cloning,
- persistence.

---

# 76. `reorder_rows()`

This performs the same idea inside a specific button.

The difference is that it also receives:

```text
button_name
```

because rows belong to a button.

---

# 77. `migrate_v1_file_to_v2_string()`

This is the migration engine used by the standalone migration binary.

Its purpose is:

```text
old dashboard format
        ↓
migration
        ↓
new dashboard format
```

It returns a `String` containing the migrated JSON.

This is a good design because the migration logic can be reused from both:

- the application code,
- the command-line migration tool.

---

# 78. Conversion helpers

The data layer contains helpers such as:

```text
stored_dashboard_to_document()
legacy_dashboard_to_document()
document_to_stored_dashboard()
```

These are adapters between representations.

Think:

```text
Old format
   ↓
Legacy representation
   ↓
Current document
   ↓
Stored representation
```

This is a very useful real-world programming lesson:

> Data formats evolve. Your application needs a controlled way to translate between versions.

---

# 79. `validate_document()`

This checks whether a dashboard document satisfies the application's rules.

Validation is different from parsing.

Parsing asks:

> "Is this valid JSON?"

Validation asks:

> "Is this JSON a valid Admindash3 dashboard?"

Those are not the same question.

For example:

```json
{}
```

can be valid JSON but still be an invalid dashboard.

---

# 80. `row_from_input()`

This converts editor input into a real stored row.

It is the bridge between:

```text
form fields
```

and:

```text
ButtonRow
```

It also gives one centralized place to validate and normalize row input.

---

# 81. `clean_required()`

This helper handles required text fields.

Conceptually:

```text
input
 ↓
trim whitespace
 ↓
make sure it is not empty
 ↓
return clean String
```

This prevents things such as:

```text
"        "
```

from being accepted as a meaningful name.

---

# 82. Hashtag helpers

The data layer contains:

```text
normalize_hashtags()
normalize_tag_list()
validate_hashtags()
parse_hashtags()
```

Together they establish consistent tag handling.

That is important because users may type:

```text
#Proxmox
```

or:

```text
Proxmox
```

and the application needs a consistent internal representation.

---

# 83. `format_deserialize_error()`

This converts a low-level JSON deserialization error into a much more useful human-readable error.

This is one of the best examples in the project of:

```text
technical information
        ↓
useful developer/admin information
```

It is particularly useful when manually editing JSON.

---

# 84. `src/buttons.rs`

This is the main dashboard UI.

It handles:

- displaying buttons,
- selecting a button,
- displaying URL rows,
- admin edit controls,
- button creation,
- button editing,
- button deletion,
- row creation,
- row editing,
- row deletion,
- drag/drop,
- tag management,
- JSON validation,
- history navigation,
- autosort,
- modal state.

This is the largest UI file.

---

# 85. `ButtonDraft`

```rust
struct ButtonDraft {
    name: String,
    first_row_name: String,
    first_row_url: String,
    first_row_hashtags: String,
    first_row_secrets: String,
}
```

This represents the temporary values inside the "Add Button" form.

The important idea:

> A draft is not yet saved data.

It is temporary UI state.

---

# 86. `ButtonEditorState`

```rust
struct ButtonEditorState {
    mode: ButtonEditorMode,
    draft: ButtonDraft,
}
```

This combines:

```text
what are we doing?
+
what has the user typed?
```

That makes the modal easier to reason about.

---

# 87. `ButtonEditorMode`

```rust
enum ButtonEditorMode {
    Create,
    Edit { original_name: String },
}
```

This is a great enum example.

The editor itself can be reused for two jobs:

```text
Create
```

or:

```text
Edit
```

The edit variant additionally remembers the original name.

---

# 88. `RowDraft`

This is the temporary editor state for one row:

```text
kind
name
url
hashtags
secrets
```

Again:

```text
RowDraft
```

is temporary form state.

It becomes persistent data only when the save operation succeeds.

---

# 89. `RowEditorState`

This adds context around the row:

```text
button_name
original_name
draft
```

The editor therefore knows:

```text
Which button?
Which existing row?
What are the current edited values?
```

---

# 90. `ButtonDraft::default()`

The `Default` implementation creates an empty form.

This allows:

```rust
ButtonDraft::default()
```

instead of manually writing:

```rust
ButtonDraft {
    name: String::new(),
    ...
}
```

every time.

---

# 91. `RowDraft::empty_link()`

This creates a blank new link row.

The initial kind is:

```rust
RowKind::Link
```

That means when the user clicks:

```text
+ Row
```

the editor starts with a link form.

---

# 92. `RowDraft::from_row()`

This converts an existing:

```rust
ButtonRowView
```

into an editable draft.

It uses `match`:

```text
Divider → divider draft
Link    → link draft
```

This is another excellent place to study Rust enums.

---

# 93. `Buttons(payload)`

The main component is:

```rust
#[component]
pub fn Buttons(payload: DashboardPayload) -> Element
```

The important input is:

```rust
payload
```

The component does not load the JSON itself.

It receives prepared dashboard data.

This is clean component design.

---

# 94. Local UI signals in `Buttons`

The component creates state for:

```text
selected_button_name
button_editor
row_editor
mutation_error
mutation_busy
tag_manager_open
tag_manager_draft
new_tag
dragging_index
dragging_button_index
status_message
```

These signals describe the current UI situation.

For example:

```text
selected_button_name
```

answers:

> "Which dashboard button's modal is open?"

while:

```text
mutation_busy
```

answers:

> "Are we currently saving something?"

---

# 95. Admin state

The component reads:

```rust
let is_admin = ADMIN_AUTH().is_admin();
```

This lets the UI immediately know whether to show:

- pencil buttons,
- drag handles,
- editing controls,
- admin information.

The comment in the supplied code specifically notes that this avoids waiting for another dashboard refresh just to make admin controls appear.

---

# 96. Autosort

The payload contains:

```rust
payload.autosort
```

The UI stores a local signal:

```rust
let mut autosort_enabled =
    use_signal(|| payload_autosort);
```

The component then creates a sorted copy:

```rust
let mut buttons_to_display = payload.buttons.clone();

if autosort_enabled() {
    buttons_to_display.sort_by_key(
        |button| button.name.to_lowercase()
    );
}
```

The important design detail is:

> Sorting for display does not mutate the actual stored server order.

So when autosort is turned off, the original persisted order comes back.

---

# 97. Why `sort_by_key`

This:

```rust
sort_by_key(|button| button.name.to_lowercase())
```

means:

> "Sort each button using its lowercase name as the comparison key."

So:

```text
zebra
Apple
server
```

sorts as:

```text
Apple
server
zebra
```

rather than treating uppercase and lowercase as completely separate ordering rules.

---

# 98. Pending button hashes

The component watches browser location hashes such as:

```text
#__pending_button?name=...
```

This lets navigation state survive a browser navigation action.

The flow is approximately:

```text
Navbar/search
   ↓
select button
   ↓
hash contains button name
   ↓
Buttons reads hash
   ↓
selected_button_name = Some(...)
   ↓
modal opens
```

Then the code removes the temporary hash using browser history.

---

# 99. Browser keyboard events

The dashboard installs a browser-level Escape handler.

It closes things in priority order:

```text
Tags modal
    ↓
Row editor
    ↓
Button editor
    ↓
Selected button
```

This is an example of global keyboard behavior layered on top of component state.

---

# 100. Rendering the dashboard grid

The main buttons are rendered in:

```text
2 columns on small screens
3 on medium screens
5 on large screens
```

through the CSS classes:

```text
grid-cols-2
md:grid-cols-3
lg:grid-cols-5
```

This is responsive UI.

---

# 101. Main-button drag and drop

Admin users can drag main buttons when autosort is disabled.

The important state is:

```rust
dragging_button_index
```

The basic lifecycle is:

```text
dragstart
   ↓
remember source index

dragover
   ↓
allow dropping

drop
   ↓
get source index
   ↓
call reorder_buttons_server()
   ↓
refresh dashboard
```

This is a nice real-world example of event-driven programming.

---

# 102. Main-button editing

The pencil button creates:

```rust
ButtonEditorState {
    mode: ButtonEditorMode::Edit { ... },
    draft: ...
}
```

The existing button name is copied into the draft.

The UI then opens:

```text
ButtonEditorModal
```

---

# 103. Opening a button

Clicking the dashboard button sets:

```rust
selected_button_name.set(Some(button_name.clone()));
```

The component then finds the matching button:

```rust
payload.buttons
    .iter()
    .find(...)
```

This is a useful Rust iterator pattern.

---

# 104. The selected-button modal

When:

```rust
selected_button_name()
```

contains a value, the component renders a modal.

Inside it are:

- button title,
- close button,
- admin `+ Row`,
- existing rows.

The modal is rendered conditionally.

This is one of the simplest and most useful Dioxus patterns:

```rust
if let Some(button) = selected_button {
    // render modal
}
```

---

# 105. Divider rows

The code matches:

```rust
ButtonRowView::Divider { ... }
```

and renders a visual separator.

Admins can:

- drag it,
- edit its name,
- reorder it.

This is another example of an enum controlling UI behavior.

---

# 106. Link rows

A link row contains:

```text
name
url
hashtags
secret information
```

The UI renders the URL as:

```rust
target: "_blank"
```

so it opens in another browser tab.

---

# 107. Secret display

The code distinguishes:

```text
secret exists
```

from:

```text
secret text is available to display
```

If the secret exists but text is not available, the UI shows:

```text
🔑 Secret hidden
```

This is an important example of not assuming that "secret exists" means "secret should be displayed."

---

# 108. Row drag and drop

Rows use:

```rust
dragging_index
```

and call:

```rust
reorder_rows_server(...)
```

The same fundamental pattern as button dragging is reused:

```text
remember source
→ drag
→ drop
→ server reorder
→ refresh
```

This is a great place to practice extracting reusable code later.

---

# 109. `TagManagerModal`

This component receives data and callbacks from its parent.

It does not directly manipulate global state.

Its props include:

```text
tags
tag_usage_counts
new_tag
busy
error_message
```

and event handlers such as:

```text
on_close
on_new_tag_input
on_add
on_remove
on_save
```

This is a very important Dioxus concept:

> A child component can receive data from its parent and send events back to the parent.

Think:

```text
Parent
  │
  │ props
  ▼
Child
  │
  │ EventHandler
  ▼
Parent
```

---

# 110. Tag manager behavior

Tags must contain at least four characters.

The UI also prevents duplicate tags case-insensitively.

For example:

```text
#Proxmox
```

and:

```text
#proxmox
```

are considered the same tag.

Used tags cannot be removed.

That usage count is calculated from the current dashboard payload.

---

# 111. `ButtonEditorModal`

This component handles:

```text
Add Button
```

and:

```text
Edit Button
```

The same component handles both because it receives:

```rust
ButtonEditorMode
```

This is a good example of avoiding two nearly identical components.

---

# 112. Creating a button

When creating:

```text
Button name
First row name
URL
Hashtags
Secrets
```

are collected.

The save callback eventually calls:

```rust
create_button_server(...)
```

The server function then delegates to:

```rust
dashboard_data::create_button(...)
```

---

# 113. Editing a button

When editing, the modal primarily changes the button name.

The URL rows are managed separately by the row editor.

That is a good separation of concerns:

```text
Button editor
    → button identity

Row editor
    → button contents
```

---

# 114. `RowEditorModal`

This is the second-level editor.

It can:

- create a row,
- edit a row,
- delete a row,
- change Link/Divider,
- edit name,
- edit URL,
- edit tags,
- edit secrets.

It is also a good example of a modal stacked above another modal.

---

# 115. Keyboard focus trapping

The row editor contains browser code that:

1. finds the modal,
2. focuses the name input,
3. listens for Tab,
4. finds focusable elements,
5. moves focus forward,
6. moves focus backward for Shift+Tab,
7. wraps around.

So:

```text
Tab at last field
      ↓
first field
```

and:

```text
Shift+Tab at first field
      ↓
last field
```

This is called a **focus trap**.

It is a useful accessibility/UI concept.

---

# 116. The `#[cfg(feature = "web")]` blocks

Some code only makes sense in a browser.

For example:

```rust
web_sys::window()
```

and:

```rust
wasm_bindgen
```

are browser/WASM-oriented APIs.

The project therefore conditionally compiles certain sections.

This is an important Fullstack Rust idea:

```text
same Rust project
      ↓
server code
+
browser/WASM code
```

Not every line belongs in both environments.

---

# 117. Button server functions in `buttons.rs`

The UI calls these server functions:

```text
create_button_server()
rename_button_server()
delete_button_server()
save_row_server()
delete_row_server()
save_tags_server()
reorder_buttons_server()
check_json_server()
reorder_rows_server()
set_autosort_server()
```

They form the bridge between the UI and the data layer.

---

# 118. `create_button_server()`

Receives:

```text
AdminCredentials
ButtonCreateInput
```

It checks admin authorization and delegates the actual creation to the data layer.

The UI should not need to know how the JSON file is structured.

---

# 119. `rename_button_server()`

Receives:

```text
AdminCredentials
ButtonRenameInput
```

and delegates the rename.

---

# 120. `delete_button_server()`

Receives:

```text
AdminCredentials
button_name
```

and delegates deletion.

---

# 121. `save_row_server()`

Receives:

```text
AdminCredentials
button_name
ButtonRowInput
```

and delegates row creation/update.

---

# 122. `delete_row_server()`

Receives:

```text
AdminCredentials
button_name
row_name
```

and deletes the requested row.

---

# 123. `save_tags_server()`

Receives the complete managed tag list and persists it.

This is a good example of replacing a whole small configuration collection in one operation.

---

# 124. `reorder_buttons_server()`

Receives:

```text
from_index
to_index
```

and delegates to the data layer.

---

# 125. `check_json_server()`

Runs the JSON validation through the same parser used by the dashboard.

This is important because the validation is not a separate "fake" validator.

The project is using the same understanding of the JSON format for both:

```text
loading
```

and:

```text
checking
```

---

# 126. `reorder_rows_server()`

Receives:

```text
button_name
from_index
to_index
```

and delegates the reorder.

---

# 127. `set_autosort_server()`

Persists the system-wide autosort setting.

The UI performs an optimistic local change, then asks the server to save it.

If the server rejects it, the UI restores the old state.

That is an example of **optimistic UI**.

---

# 128. Tag helper functions

## `normalize_tag_for_ui()`

Takes a tag and makes it consistent.

For example:

```text
"  #Proxmox  "
```

becomes:

```text
"#Proxmox"
```

---

## `add_tag_to_text()`

Adds a tag to a space-separated text field while avoiding duplicates.

This lets the editor support convenient tag buttons without creating duplicate text.

---

## `calculate_tag_usage_counts()`

Counts how many rows use each managed tag.

The function walks:

```text
payload
  ↓
buttons
  ↓
rows
  ↓
link rows
  ↓
hashtag tokens
```

This is a very useful iterator exercise.

---

## `tag_usage_count()`

Looks up the count for one particular tag.

---

# 129. `src/navbar.rs`

The navbar is responsible for much more than navigation.

It handles:

- navigation links,
- nested menu items,
- search,
- tag searching,
- keyboard navigation,
- admin login,
- admin logout,
- browser local storage,
- mobile menu,
- iframe navigation,
- login/logout focus behavior.

It is effectively the application's command center.

---

# 130. `SearchResult`

The navbar defines:

```rust
struct SearchResult
```

with:

```text
button_name
row_name
url
all_tags
matched_tags
has_secret
secret_text
```

This is a search-specific view model.

It exists because the search UI needs more information than the raw row alone.

---

# 131. Search state

The navbar maintains signals for:

```text
search
search_open
selected_index
mobile_menu_open
login_open
logout_open
login_username
login_password
login_error
```

This is a good example of a component with several independent pieces of UI state.

---

# 132. Main-page detection

The navbar checks:

```rust
IFRAME_URL
```

to determine whether the main dashboard page is currently being displayed.

The idea is:

```text
no iframe URL
    → main dashboard

iframe URL exists
    → external content/page selected
```

---

# 133. Browser storage

The navbar uses browser storage keys such as:

```text
admindash3_search
admindash3_admin
admindash3_admin_username
admindash3_admin_password
```

The purpose is to preserve useful UI state across F5.

The supplied code intentionally stores the admin login state and credentials in browser storage.

### Important security lesson

This is convenient for a homelab/dashboard project, but browser storage is **not a secure password vault**.

For a production authentication system, credentials should be handled with a proper authentication/session design.

This project is currently optimized around a simple learning/homelab model.

---

# 134. `browser_storage_get()`

This helper reads browser storage.

The project uses JavaScript evaluation through `js_sys` because the required `web-sys` storage feature is not enabled.

This is a good example of using:

```text
Rust
 ↓
WASM bindings
 ↓
browser JavaScript API
```

when a browser feature is not directly available through the current Rust bindings/configuration.

---

# 135. Search result highlighting

`HighlightedTag` renders a tag while highlighting the portion matching the search query.

This demonstrates a common UI technique:

```text
raw text
   ↓
split around match
   ↓
render matching part differently
```

It also demonstrates how Rust string manipulation can directly drive UI presentation.

---

# 136. `NavEntry`

`NavEntry` recursively renders a `NavItem`.

This is where the recursive navbar data structure becomes UI.

The important idea is:

```text
NavItem
   ↓
NavEntry
   ↓
children
   ↓
NavEntry
   ↓
children...
```

Recursive data + recursive component rendering.

---

# 137. Navigation links and iframe state

When a normal navbar link is clicked, the component can update:

```rust
IFRAME_URL
```

The application therefore uses shared state to communicate:

```text
Navbar
   ↓
IFRAME_URL
   ↓
main application layout
```

This is a concrete example of why global state exists.

---

# 138. `authenticate_admin_server()`

The navbar contains:

```rust
#[server]
async fn authenticate_admin_server(...)
```

Its job is intentionally small.

It checks:

```rust
credentials.is_admin()
```

and returns:

```text
Ok(())
```

for valid credentials or:

```text
401 Invalid login credentials
```

for invalid credentials.

The actual credential rules live in the data model/configuration.

---

# 139. Admin login flow

The login flow is roughly:

```text
User presses \
      ↓
login modal opens
      ↓
username/password fields
      ↓
authenticate_admin_server()
      ↓
valid?
   ┌──┴──┐
  yes    no
   │      │
   ▼      ▼
ADMIN_AUTH    error message
   │
   ▼
DASHBOARD_REFRESH_KEY += 1
   │
   ▼
dashboard reloads
   │
   ▼
admin controls appear
```

This is a great example of state-driven UI.

---

# 140. Admin logout flow

Logout resets:

```rust
ADMIN_AUTH
```

removes the browser storage values, increments:

```rust
DASHBOARD_REFRESH_KEY
```

and reopens/focuses the search interface.

The important pattern is:

```text
authentication state changes
        ↓
refresh trigger changes
        ↓
dashboard reloads
        ↓
UI changes from admin → visitor
```

---

# 141. Keyboard shortcuts

The current navbar behavior includes:

```text
\       → open Admin Login when logged out
Escape  → close the login modal
```

When logged in, the corresponding admin flow can open the logout confirmation.

The login modal also explicitly tells the user about the keyboard shortcuts.

This is a good reminder:

> Keyboard interaction is just another source of events.

Buttons, mouse clicks, keyboard presses, drag/drop, and browser events all eventually modify state.

---

# 142. `src/history.rs`

This file implements the dashboard history UI and snapshot operations.

It has two halves:

```text
history filesystem logic
+
history Dioxus UI
```

The history system allows administrators to:

- inspect snapshots,
- see differences,
- revert,
- clear history.

---

# 143. History constants

The file uses:

```rust
pub const HISTORY_DIR: &str = dashboard_data::HISTORY_DIR;
pub const MAX_SNAPSHOTS: usize = dashboard_data::MAX_HISTORY_SNAPSHOTS;
pub const SNAPSHOTS_PER_PAGE: usize = 10;
```

This is a nice example of reusing configuration from another module instead of duplicating it.

---

# 144. `HistorySnapshot`

Represents one snapshot.

Conceptually:

```text
filename
timestamp/label
preview/diff information
```

It is the object displayed by the history page.

---

# 145. `HistoryDiffLine`

Represents one line of a difference preview.

A diff needs to answer:

```text
What changed?
```

rather than merely:

```text
Here is the entire old file.
```

---

# 146. `HistoryDiffKind`

The diff line has a kind, allowing the UI to distinguish different types of lines.

Conceptually:

```text
context
added
removed
```

This lets the UI present changes clearly.

---

# 147. `list_history()`

```rust
pub fn list_history()
    -> anyhow::Result<Vec<HistorySnapshot>>
```

This scans the history directory and builds a list of usable snapshots.

It returns a `Vec` because the UI needs a collection.

---

# 148. `is_safe_snapshot_filename()`

Before touching a snapshot, the filename is checked.

This is an important security principle:

> Never blindly trust a filename supplied to a filesystem operation.

Especially important when a filename ultimately comes from the browser.

---

# 149. `timestamp_label_from_filename()`

Converts the machine-friendly filename:

```text
dashboard-20260921_061530_123.json
```

into a human-friendly timestamp label.

This keeps the stored filename simple and sortable while making the UI pleasant.

---

# 150. `build_history_preview()`

This compares a snapshot against the current dashboard JSON.

It builds a limited diff preview using:

```text
HISTORY_DIFF_CONTEXT
HISTORY_DIFF_MAX_LINES
```

The purpose is to avoid dumping an enormous JSON difference into the page.

---

# 151. `revert_history_snapshot()`

This restores a selected history snapshot.

The function requires admin credentials.

The conceptual flow is:

```text
selected snapshot
      ↓
validate filename
      ↓
read snapshot
      ↓
validate dashboard JSON
      ↓
write dashboard
      ↓
dashboard refresh
```

This is a great example of a high-impact operation being protected by validation and authorization.

---

# 152. `clear_history()`

This removes history snapshots.

It returns:

```rust
anyhow::Result<usize>
```

so the caller can know how many snapshots were removed.

That allows the UI to say something meaningful like:

```text
Cleared 17 snapshots.
```

---

# 153. `HistoryPage()`

This is the Dioxus UI for the history system.

It maintains UI state such as:

```text
current page
selected snapshot
busy state
error message
status message
revert confirmation
clear confirmation
```

This is another excellent example of component-local signals.

---

# 154. History server functions

The UI has three server boundaries:

```text
load_history_server()
revert_history_server()
clear_history_server()
```

Each one performs the security check / delegates to the filesystem logic.

Again:

```text
UI
 ↓
server function
 ↓
ordinary Rust function
```

That separation is worth copying in future projects.

---

# 155. `src/bin/data_button_v2_migrator.rs`

This is a standalone Rust binary.

Unlike the main web application, it is designed to be run from the command line.

Usage:

```text
cargo run --bin data_button_v2_migrator -- <input data_buttons.json> [output data_buttons.v2.json]
```

---

# 156. `main()` in the migrator

It:

1. reads command-line arguments,
2. requires an input path,
3. optionally accepts an output path,
4. defaults the output filename if necessary,
5. calls the shared migration logic,
6. writes the migrated JSON,
7. prints a result message.

This is a great example of building a useful tool from the same Rust library/module code as the application.

---

# 157. `default_output_path()`

If the user does not specify an output path, this function creates one.

For example:

```text
data_buttons.json
```

becomes:

```text
data_buttons.v2.json
```

It uses:

```rust
PathBuf
```

to safely manipulate filesystem paths.

---

# 158. Why the migrator is valuable

A migration tool is better than manually rewriting a data file because:

```text
old data
  ↓
repeatable program
  ↓
new data
```

means the transformation is:

- repeatable,
- testable,
- automatable,
- less dependent on manual editing.

This is exactly the kind of small tool that becomes extremely useful in real Rust projects.

---

# 159. How a button edit travels through the whole application

Let's follow one complete example.

Suppose an admin edits a URL row.

### Step 1 — User clicks pencil

`buttons.rs` receives the click.

```text
onclick
   ↓
row_editor.set(...)
```

---

### Step 2 — Modal appears

Dioxus sees that:

```rust
row_editor()
```

contains a value.

Therefore:

```text
RowEditorModal
```

is rendered.

---

### Step 3 — User types

The input event runs:

```rust
oninput
```

and updates the draft:

```rust
current.draft.url = value;
```

---

### Step 4 — User presses Save

The save callback is async:

```rust
on_save: move |_| async move {
    ...
}
```

---

### Step 5 — UI calls server function

```rust
save_row_server(...)
    .await
```

---

### Step 6 — Server function calls data layer

The server function delegates to:

```rust
save_row(...)
```

in `dashboard_data.rs`.

---

### Step 7 — Data layer validates

The input is converted and checked.

---

### Step 8 — Existing dashboard is backed up

The history system gets a snapshot.

---

### Step 9 — New JSON is written

The dashboard file is updated.

---

### Step 10 — Refresh key changes

The UI does:

```rust
*DASHBOARD_REFRESH_KEY.write() += 1;
```

---

### Step 11 — Dashboard reloads

`use_server_future()` notices the relevant state changed.

---

### Step 12 — New payload reaches `Buttons`

The component renders the new data.

---

## The complete chain

```text
User
 ↓
Dioxus event
 ↓
local signal
 ↓
modal draft
 ↓
server function
 ↓
dashboard_data.rs
 ↓
validation
 ↓
history backup
 ↓
JSON write
 ↓
refresh signal
 ↓
server reload
 ↓
new DashboardPayload
 ↓
Dioxus rerender
```

That is the core architecture of Admindash3.

---

# 160. How login travels through the application

The login flow is similar.

```text
User enters credentials
        ↓
Navbar signal
        ↓
authenticate_admin_server()
        ↓
credentials.is_admin()
        ↓
ADMIN_AUTH
        ↓
DASHBOARD_REFRESH_KEY += 1
        ↓
load_dashboard(credentials)
        ↓
DashboardPayload
        ↓
Buttons sees is_admin = true
        ↓
pencils + drag controls appear
```

This is an excellent example of **one state change causing many UI changes**.

---

# 161. How autosort works

Autosort is also a nice architectural example.

```text
Admin clicks Autosort
       ↓
autosort_enabled changes locally
       ↓
set_autosort_server()
       ↓
system_preferences.json
       ↓
DASHBOARD_REFRESH_KEY += 1
       ↓
dashboard reload
       ↓
payload.autosort
       ↓
Buttons renders sorted copy
```

The important detail is:

> Autosort changes the **display order**, not necessarily the stored button order.

That distinction keeps the underlying data stable.

---

# 162. How history works

Every important dashboard write follows the idea:

```text
current file
    ↓
snapshot
    ↓
new file
```

The history page then provides:

```text
history directory
      ↓
list snapshots
      ↓
preview changes
      ↓
choose snapshot
      ↓
confirm
      ↓
restore
```

This is effectively a small, purpose-built version-control system for the dashboard JSON.

It is not Git, but the mental model is similar:

```text
snapshot
diff
restore
```

---

# 163. The most useful Rust concepts to study in this project

If you want to learn Rust by reading Admindash3, study these in roughly this order:

### Level 1 — Basics

- `let`
- `mut`
- `String`
- `&str`
- `Vec`
- `Option`
- `Result`
- `match`
- `if let`
- functions

### Level 2 — Rust structure

- structs
- enums
- methods
- `impl`
- traits
- `derive`
- modules
- `pub`

### Level 3 — Ownership

- borrowing
- `&T`
- ownership
- `.clone()`
- `move`
- closures

### Level 4 — Application Rust

- filesystem APIs
- `Path`
- `PathBuf`
- JSON serialization
- error propagation with `?`
- iterators
- `map`
- `filter`
- `flat_map`
- `collect`

### Level 5 — Async

- `async fn`
- `.await`
- futures
- server functions

### Level 6 — Dioxus

- `#[component]`
- `Element`
- `rsx!`
- `use_signal`
- `GlobalSignal`
- `use_effect`
- `use_drop`
- event handlers

### Level 7 — Browser/WASM

- `web_sys`
- `wasm_bindgen`
- browser events
- localStorage
- DOM focus
- keyboard events

---

# 164. A Rust iterator example from the project

This style appears in tag counting:

```rust
payload
    .buttons
    .iter()
    .flat_map(|button| button.rows.iter())
    .filter_map(|row| match row {
        ButtonRowView::Link { hashtag_tokens, .. } =>
            Some(hashtag_tokens),

        ButtonRowView::Divider { .. } =>
            None,
    })
    .flat_map(|tags| tags.iter())
    .filter(|tag| tag.eq_ignore_ascii_case(managed_tag))
    .count()
```

At first glance this looks terrifying.

Don't panic. 😄

Read it one line at a time.

```text
buttons
  ↓
iter()
```

Look at each button.

```text
flat_map(...)
```

Get all rows from all buttons.

```text
filter_map(...)
```

Keep only Link rows and extract their tags.

```text
flat_map(...)
```

Flatten all tag lists.

```text
filter(...)
```

Keep matching tags.

```text
count()
```

Count them.

The whole thing means:

> "Count how many URL rows use this managed tag."

This is a perfect piece of code to rewrite as a beginner exercise using ordinary `for` loops, then compare the two versions.

---

# 165. Try rewriting iterator code with loops

For learning, you could temporarily imagine:

```rust
let mut count = 0;

for button in &payload.buttons {
    for row in &button.rows {
        match row {
            ButtonRowView::Link { hashtag_tokens, .. } => {
                for tag in hashtag_tokens {
                    if tag.eq_ignore_ascii_case(managed_tag) {
                        count += 1;
                    }
                }
            }

            ButtonRowView::Divider { .. } => {}
        }
    }
}
```

The iterator version is shorter.

The loop version may be easier to understand initially.

### Learning trick

Write both.

Then compare them.

That is much better than memorizing iterator syntax without understanding it.

---

# 166. Ownership lessons hidden in `buttons.rs`

One of the hardest beginner concepts is why code needs:

```rust
let button_name = button.name.clone();
```

instead of simply using:

```rust
button.name
```

The answer is usually ownership.

The UI closure may need to own the value after the current loop iteration has moved on.

The `.clone()` creates an independent `String`.

A good exercise is to deliberately remove a `.clone()` and read the compiler error.

Rust's compiler is actually a pretty good teacher here.

---

# 167. Why the project uses many small state objects

Instead of having one enormous struct like:

```text
EverythingInTheWholeApplication
```

the project separates:

```text
ButtonDraft
ButtonEditorState
RowDraft
RowEditorState
SearchResult
AdminCredentials
DashboardPayload
HistorySnapshot
```

Each one represents one concept.

This is a major software-engineering lesson:

> Give data a name that describes what it represents.

Named types make large programs much easier to understand.

---

# 168. Why UI state and stored data are different

This is another important lesson.

For example:

```rust
ButtonDraft
```

is not the same thing as:

```rust
ButtonGroup
```

The draft exists temporarily in the browser.

The stored model exists in the dashboard JSON.

So:

```text
Browser form
    ↓
ButtonDraft
    ↓
server function
    ↓
validation
    ↓
ButtonGroup
    ↓
JSON
```

This distinction prevents a lot of messy code.

---

# 169. Why server functions should stay small

Consider:

```rust
async fn save_row_server(...)
```

It does not implement every detail of JSON editing.

Instead it acts as a doorway:

```text
browser
   ↓
authorization
   ↓
dashboard_data::save_row()
```

This is good architecture.

If the JSON format changes, you mostly change:

```text
dashboard_data.rs
```

rather than rebuilding every UI event handler.

---

# 170. Why validation belongs in the data layer

Imagine only the UI validates:

```text
URL cannot be empty
```

Then another caller could bypass the UI.

For example:

```text
CLI tool
future API
test
another server function
```

The data layer should still reject invalid data.

That is why `dashboard_data.rs` contains validation functions.

The rule is:

> Never trust the UI to be your only line of validation.

---

# 171. Why history belongs near data writes

The application backs up dashboard data before changing it.

That means the write path knows:

```text
something is about to change
```

This is safer than hoping every UI caller remembers:

```text
Oh! I should make a backup first.
```

Centralizing important invariants is a very useful software design pattern.

---

# 172. Why the application uses a refresh key

A beginner might ask:

> "Why not just reload everything manually after every save?"

The refresh key gives the application a simple reactive mechanism:

```rust
*DASHBOARD_REFRESH_KEY.write() += 1;
```

The dashboard loader is then tied to that state.

This gives one consistent refresh mechanism for:

- login,
- logout,
- button changes,
- row changes,
- tag changes,
- reorder operations,
- autosort changes,
- history restores.

One little integer acts like a global "please refresh" bell.

---

# 173. What to look at when you get a borrow-checker error

When Rust says something about:

```text
borrowed value
moved value
cannot borrow
use of moved value
```

ask these questions:

### Question 1

Who owns this value?

### Question 2

Am I moving it?

### Question 3

Does a closure need to own it?

### Question 4

Could I borrow it instead?

### Question 5

Would a clone be appropriate?

### Question 6

Did I accidentally move the value into a closure with `move`?

This thought process will eventually become second nature.

---

# 174. What to look at when you get an `.await` error

If you see:

```text
await is only allowed inside async functions and blocks
```

look at the surrounding function.

This:

```rust
onclick: move |_| {
    some_async_function().await;
}
```

is wrong.

Use:

```rust
onclick: move |_| async move {
    some_async_function().await;
}
```

The outer event handler must return an async block.

---

# 175. What to look at when Dioxus state doesn't update

Check:

```text
Did I use the signal?
Did I call .set(...)?
Did I accidentally clone a stale value?
Is this signal local or global?
Does this component actually read the signal?
```

Remember:

```text
read signal
   ↓
component depends on signal
   ↓
signal changes
   ↓
component can rerender
```

If the component never reads the signal, changing it may not produce the behavior you expect.

---

# 176. What to look at when browser-only code fails on the server

If you see things such as:

```rust
web_sys::window()
```

remember:

> The server does not have a browser window.

Browser-specific code should be conditionally compiled or otherwise kept on the client side.

The project uses patterns such as:

```rust
#[cfg(target_arch = "wasm32")]
```

and:

```rust
#[cfg(feature = "web")]
```

for browser-specific behavior.

---

# 177. Recommended reading order for a Rust beginner

Do not start by trying to understand all 7,000+ lines at once.

Use this order:

```text
1. data_navbar.rs
2. footer.rs
3. main.rs
4. dashboard_data.rs
5. buttons.rs
6. navbar.rs
7. history.rs
8. build.rs
9. migrator
```

Why?

Because the complexity gradually increases.

---

# 178. Beginner exercise: build a tiny counter

Before changing Admindash3, create a tiny Dioxus component:

```text
Count: 0

[-] [ + ]
```

Learn:

- `use_signal`
- `onclick`
- `set`
- reading a signal
- rerendering

Once that makes sense, the rest of the UI becomes much less mysterious.

---

# 179. Beginner exercise: add a "Last refreshed" label

Use a signal containing a String:

```text
Last refreshed: ...
```

Then update it whenever the dashboard refreshes.

Learn:

- signals,
- state changes,
- passing state into RSX,
- formatting strings.

---

# 180. Beginner exercise: add a button counter

Give each dashboard button a click counter.

For example:

```text
Proxmox       7 clicks
TrueNAS       3 clicks
Network      12 clicks
```

Do not worry about persistence yet.

The goal is simply to learn:

```text
button click
   ↓
state changes
   ↓
UI updates
```

---

# 181. Beginner exercise: add a "favorite" star

Add:

```text
☆ Proxmox
★ TrueNAS
```

Clicking the star toggles it.

Concepts:

- `bool`
- `use_signal`
- conditional RSX
- event handlers

Bonus:

Persist favorites into JSON.

Now you have your first tiny data model.

---

# 182. Beginner exercise: add a dark-mode switch

Create:

```text
[ Dark Mode OFF ]
```

When clicked:

```text
[ Dark Mode ON ]
```

Then conditionally change CSS classes.

Concepts:

- boolean signals,
- conditional rendering,
- dynamic class values.

Bonus:

Save it to `system_preferences.json`.

Congratulations: you have just recreated a smaller version of an existing Admindash3 feature.

---

# 183. Beginner exercise: build a "Random Button"

Add:

```text
🎲 Surprise Me
```

Clicking it selects a random dashboard button.

Concepts:

- `Vec`
- indexes
- state
- selecting an item
- UI events

Bonus:

Make it select only buttons containing a particular tag.

---

# 184. Beginner exercise: tag statistics

Build a small panel:

```text
Tag statistics

#proxmox      8
#network      5
#storage      4
#backup       9
```

This is perfect practice for the iterator code already present in `buttons.rs`.

Start with nested `for` loops.

Then rewrite it using:

```text
map
filter
flat_map
collect
```

---

# 185. Beginner exercise: dashboard search v2

Build a second search mode.

Search by:

```text
button name
row name
URL
tag
```

Then add:

```text
Search results: 17
```

Concepts:

- structs,
- iterators,
- strings,
- case-insensitive comparison,
- filtering,
- rendering lists.

This is one of the best exercises for understanding the existing `SearchResult`.

---

# 186. Beginner exercise: "Recently used"

Track the last five clicked dashboard buttons.

Display:

```text
Recently used

1. Proxmox
2. TrueNAS
3. Veeam
4. Network
5. Docker
```

Concepts:

- `Vec`
- inserting/removing values
- avoiding duplicates
- state
- persistence

Bonus:

Save it to a JSON file.

---

# 187. Beginner exercise: health-status dashboard

Create a new row type:

```text
Status
```

with:

```text
🟢 Online
🟡 Warning
🔴 Offline
```

This is a bigger Rust exercise.

You would need to think about:

```rust
enum RowKind {
    Link,
    Divider,
    Status,
}
```

Then update:

- stored data,
- parser,
- validation,
- editor,
- renderer.

This teaches you how one new domain concept travels through an entire application.

---

# 188. Beginner exercise: add icons to buttons

Add an optional icon field:

```text
name
icon
rows
```

For example:

```text
🖥 Proxmox
💾 TrueNAS
🌐 Network
🐳 Docker
```

You will practice:

- changing structs,
- changing JSON,
- serde,
- UI rendering,
- migration/default values.

This is a deceptively good exercise because it touches almost every layer.

---

# 189. Beginner exercise: undo the last change

The project already has history snapshots.

Build a simple:

```text
↶ Undo Last Change
```

button.

Do not build a complete history UI.

Just:

```text
find newest snapshot
   ↓
revert
   ↓
refresh
```

This lets you study the existing history code while adding a small feature.

---

# 190. Intermediate exercise: history diff viewer

Improve the existing history page.

Display:

```text
Removed:
- Old URL

Added:
+ New URL
```

Learn:

- enums,
- string comparison,
- collections,
- conditional RSX,
- reusable components.

---

# 191. Intermediate exercise: drag-and-drop zones

Add a visible insertion marker:

```text
Button A
────────────
Drop here
────────────
Button B
```

instead of relying only on the dragged card.

Concepts:

- drag events,
- temporary state,
- conditional classes,
- indexes.

---

# 192. Intermediate exercise: keyboard navigation

Build keyboard shortcuts:

```text
Arrow Up
Arrow Down
Enter
Escape
```

to navigate the dashboard.

The navbar already contains many of these concepts.

Try implementing a simpler version yourself before reading the existing code.

---

# 193. Intermediate exercise: command palette

Build:

```text
Ctrl+K
```

to open a command palette.

Commands could include:

```text
Search dashboard
Open history
Toggle autosort
Open random button
Logout
```

This would be a fantastic Dioxus practice project because it combines:

- keyboard events,
- signals,
- lists,
- filtering,
- focus,
- server actions.

---

# 194. Intermediate exercise: tag manager improvements

Add:

```text
Rename tag
```

For example:

```text
#prox → #proxmox
```

Then update every row that uses the old tag.

This teaches an important data problem:

> Changing a shared identifier may require updating many records.

---

# 195. Advanced beginner project: dashboard import/export

Create:

```text
Export Dashboard
Import Dashboard
```

Export:

```text
data_buttons.v2.json
data_tags.json
system_preferences.json
```

into a downloadable package/file.

Import validates everything before replacing the current configuration.

Concepts:

- serialization,
- validation,
- filesystem,
- error handling,
- multi-step operations,
- backups.

---

# 196. Advanced project: SQLite backend

The current application uses JSON files.

Try replacing only the data layer with SQLite.

Keep the UI almost unchanged.

The goal becomes:

```text
buttons.rs
navbar.rs
history.rs
       │
       ▼
same server functions
       │
       ▼
new database layer
```

This teaches a huge real-world architecture lesson:

> A good UI should not care where its data comes from.

---

# 197. Advanced project: proper authentication

The current project is deliberately simple.

A more production-oriented version could introduce:

```text
hashed passwords
sessions
cookies
session expiration
logout invalidation
CSRF considerations
authorization checks
```

This is a much larger project and should be treated as a separate learning stage.

Do not mix it into the beginner dashboard while you are still learning basic ownership and Dioxus signals.

---

# 198. Advanced project: multiple users

Build:

```text
Admin
Editor
Viewer
```

permissions.

For example:

```text
Viewer
  → can browse

Editor
  → can edit buttons/rows

Admin
  → can edit
  → manage tags
  → history
  → system settings
```

This would turn the current simple `is_admin()` check into a real authorization model.

---

# 199. Advanced project: dashboard plugins

Imagine:

```text
Plugins

Proxmox
TrueNAS
Veeam
Docker
Network
```

Each plugin could provide:

```text
name
icon
URL
health check
optional actions
```

Then the dashboard becomes a mini homelab control center.

This is a fun place to experiment with Rust traits and enums.

---

# 200. Advanced project: live service monitoring

Turn a dashboard row into:

```text
Proxmox
🟢 Online
Latency: 4ms
```

The server could periodically check services.

Concepts:

- async Rust,
- HTTP clients,
- background tasks,
- time intervals,
- server state,
- UI refresh.

This is where the project starts becoming a genuinely interesting homelab application.

---

# 201. Advanced project: dashboard statistics

Add:

```text
Most clicked
Recently used
Unused buttons
Most common tags
Links by category
Changes this week
```

Now the dashboard becomes a small analytics application.

This is excellent practice for:

- iterators,
- collections,
- sorting,
- aggregation,
- reusable components.

---

# 202. A great way to learn: break things deliberately

One of the best ways to learn Rust is to create controlled compiler errors.

Try:

### Remove a `.clone()`

See what ownership error appears.

### Remove `mut`

See what mutation error appears.

### Change `Option<String>` to `String`

See what breaks.

### Remove `.await`

See what async error appears.

### Move a value into a closure

See what gets moved.

### Change an enum

See which `match` statements the compiler complains about.

Then fix the error.

This turns the compiler into a tutor.

---

# 203. A great way to learn Dioxus: change one thing at a time

Pick something tiny.

For example:

```text
Change button background
```

Then:

```text
Change button text
```

Then:

```text
Add a signal
```

Then:

```text
Make the signal change on click
```

Then:

```text
Save the value
```

Then:

```text
Load it again
```

This gradually builds:

```text
UI
 ↓
state
 ↓
events
 ↓
server
 ↓
data
 ↓
persistence
```

That is essentially the whole application in miniature.

---

# 204. Suggested learning roadmap

## Stage 1 — Rust basics

Build:

```text
CLI calculator
```

Learn:

- variables,
- functions,
- match,
- Result.

---

## Stage 2 — Rust data structures

Build:

```text
CLI bookmark manager
```

Learn:

- structs,
- enums,
- Vec,
- Option,
- JSON.

---

## Stage 3 — Dioxus

Build:

```text
Todo app
```

Learn:

- components,
- RSX,
- signals,
- events.

---

## Stage 4 — Dioxus + server

Build:

```text
Todo app with JSON persistence
```

Learn:

- server functions,
- async,
- filesystem.

---

## Stage 5 — Admindash3

Now study:

```text
buttons.rs
dashboard_data.rs
navbar.rs
history.rs
```

You will recognize most of the concepts.

---

# 205. A useful mental model for the entire project

When reading any function, ask:

### 1. What layer am I in?

```text
UI?
Server boundary?
Data layer?
Build tool?
```

### 2. What is the input?

```text
String?
Struct?
Signal?
Credentials?
Path?
```

### 3. What is the output?

```text
Element?
Result?
Vec?
Updated file?
```

### 4. Who owns the data?

```text
Component?
Closure?
Server function?
Data layer?
```

### 5. Can this fail?

If yes, expect:

```rust
Result
```

and perhaps:

```rust
?
```

### 6. Is this browser-only?

Look for:

```text
web_sys
wasm_bindgen
js_sys
window
document
```

### 7. Is this persistent?

Look for:

```text
std::fs
JSON
history
preferences
```

Those seven questions will take you surprisingly far.

---

# 206. The "big picture" cheat sheet

```text
                     AD MINDASH3
                         │
                         ▼
                    ┌─────────┐
                    │ main.rs │
                    └────┬────┘
                         │
             ┌───────────┼───────────┐
             ▼           ▼           ▼
          Navbar       Buttons      Footer
             │           │
             │           │
             └─────┬─────┘
                   │
                   ▼
          Dioxus Server Functions
                   │
                   ▼
          ┌────────────────────┐
          │ dashboard_data.rs  │
          └─────────┬──────────┘
                    │
       ┌────────────┼───────────────┐
       ▼            ▼               ▼
     JSON         Tags         Preferences
       │
       ▼
 dashboard_history/
```

---

# 207. The three most important files

If you only have time to study three files, study:

## `main.rs`

Learn:

```text
modules
global state
components
server functions
application composition
```

## `buttons.rs`

Learn:

```text
Dioxus
signals
events
RSX
closures
modals
drag/drop
async
```

## `dashboard_data.rs`

Learn:

```text
Rust structs
enums
JSON
filesystem
Result
error handling
validation
CRUD
data migration
```

Together, those three files teach a surprisingly large portion of practical Rust + Dioxus.

---

# 208. The biggest lesson from Admindash3

The application is not really "a bunch of buttons."

It is a small example of a real application architecture:

```text
                    USER
                      │
                      ▼
                   UI state
                      │
                      ▼
                  Dioxus events
                      │
                      ▼
                server functions
                      │
                      ▼
                 data layer
                      │
              ┌───────┼────────┐
              ▼       ▼        ▼
             JSON   validation history
              │
              ▼
          persistent data
              │
              ▼
           refresh signal
              │
              ▼
              UI
```

Once you understand that loop, adding features becomes much easier.

---

# 209. Final beginner challenge

If you want one project that will really test whether you understand the code, build this:

## "Admindash3 Mini"

Create a smaller dashboard from scratch.

It should have:

```text
+ Add Button
+ Add Link
+ Search
+ Tags
+ Favorites
+ Delete
+ Edit
+ JSON persistence
```

Do **not** copy the existing code line-for-line.

Instead, use the existing project as your reference.

Start with:

```text
main.rs
```

Then create:

```text
data.rs
```

Then:

```text
dashboard.rs
```

Then add:

```text
search.rs
```

Finally:

```text
history.rs
```

When you can build that without constantly copying the original code, you will have moved from:

> "I can read Rust."

to:

> "I can design a Rust application."

That is a very different milestone.

---

# 210. Final cheat sheet

## Rust

```text
let             variable
mut             mutable variable
&               borrow/reference
.clone()        make a copy
Option<T>       maybe a value
Result<T,E>      success/failure
match           pattern matching
struct          named data
enum            one of several forms
impl            add behavior
trait           shared behavior
derive          generate trait implementations
Vec<T>          growable list
Path            filesystem path
?               return error if operation fails
async            asynchronous function
.await           wait for async result
move             closure takes ownership
```

## Dioxus

```text
#[component]    component
Element         component output
rsx!             UI markup
use_signal       component state
GlobalSignal     shared application state
use_effect       side effect
use_drop         cleanup
EventHandler     child → parent event
```

## Admindash3

```text
main.rs
    application composition

buttons.rs
    dashboard UI + admin editing

navbar.rs
    navigation + search + authentication

dashboard_data.rs
    JSON + validation + CRUD + persistence

history.rs
    snapshots + diff + revert

data_navbar.rs
    navbar data model

footer.rs
    version display

build.rs
    Git hash at compile time

data_button_v2_migrator.rs
    old JSON → new JSON migration
```

---

# 211. One last piece of advice

Don't try to understand every line before you run the application.

Instead:

```text
Read a small section
      ↓
Run the app
      ↓
Change one thing
      ↓
Observe what happens
      ↓
Read the compiler error
      ↓
Fix it
      ↓
Repeat
```

Rust can feel unusually strict when you're learning it.

That's partly the point.

The compiler is constantly forcing you to answer questions like:

```text
Who owns this?
Can this be missing?
Can this fail?
Can this change?
Where can this code run?
What kind of thing is this?
```

Those questions feel annoying at first.

Eventually they become the things that make larger programs easier to reason about.

And Admindash3 is actually a pretty fun playground for learning them because every concept has a visible result:

```text
change a signal      → UI changes
change a struct      → compiler teaches you
change JSON          → parser teaches you
change ownership     → borrow checker teaches you
change a server call → dashboard changes
change a component   → UI changes
change history       → rollback changes the whole dashboard
```

So the best way to learn this project is not to treat it as a finished product that must never be touched.

Treat it as a **Rust laboratory**.

Break it.

Fix it.

Add ridiculous features.

Make a button that says "DO NOT PRESS."

Then make it count how many times you pressed it.

Then persist the count.

Then make it show the last person who pressed it.

Then realize you've accidentally built another database feature.

That's how you learn. 🚀

---

## Source coverage note

This guide is based on the Rust source files supplied with the project for this documentation task:

```text
./build.rs
./src/main.rs
./src/buttons.rs
./src/dashboard_data.rs
./src/data_navbar.rs
./src/footer.rs
./src/history.rs
./src/navbar.rs
./src/bin/data_button_v2_migrator.rs
```

The supplied source dump does not include the project's `Cargo.toml`, CSS files, Docker configuration, JSON data files, or test files, so those are intentionally not documented here as if their contents were known.

That keeps this guide grounded in the actual source rather than inventing details that were not supplied.

