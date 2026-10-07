Yes—**hydration** is the right term. The server first renders HTML, then the browser downloads the Dioxus WebAssembly bundle and “hydrates” that HTML by attaching the client-side runtime, event handlers, signals, and effects. Dioxus’s hydration process runs after the initial HTML exists and lets the WASM application take over the page. cite [dioxuslabs](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr/)

Here is a minimal Dioxus 0.7 example that changes the page background from white to blue after 30 seconds.

## `src/main.rs`

```rust
use dioxus::prelude::*;

#[cfg(not(feature = "server"))]
use gloo_timers::future::TimeoutFuture;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut is_blue = use_signal(|| false);

    // This runs in the browser after hydration.
    #[cfg(not(feature = "server"))]
    use_effect(move || {
        spawn(async move {
            TimeoutFuture::new(30_000).await;
            is_blue.set(true);
        });
    });

    let background = if is_blue() { "blue" } else { "white" };

    rsx! {
        div {
            style: "min-height: 100vh; background-color: {background};",

            h1 { "Hydrated Dioxus app" }

            p {
                "The background will become blue after 30 seconds."
            }

            p {
                "Current color: {background}"
            }
        }
    }
}
```

## Required `Cargo.toml`

The relevant parts of `Cargo.toml` would be:

```toml
[package]
name = "hydrated-example"
version = "0.1.0"
edition = "2024"

[dependencies]
dioxus = { version = "0.7", features = ["fullstack"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
gloo-timers = { version = "0.3", features = ["futures"] }

[features]
default = []
web = ["dioxus/web"]
server = ["dioxus/server"]
```

A Dioxus Fullstack application has two builds: a server build that performs SSR and a web/WASM build that runs in the browser. The `server` and `web` Cargo features select the appropriate Dioxus functionality for each build. cite [dioxuslabs](https://dioxuslabs.com/learn/0.7/essentials/fullstack/project_setup/)

## What happens

### Initial request

When the browser requests the page:

```text
GET /
```

the server renders:

```html
<div style="min-height: 100vh; background-color: white;">
    <h1>Hydrated Dioxus app</h1>
    <p>The background will become blue after 30 seconds.</p>
    <p>Current color: white</p>
</div>
```

The server must initially render `is_blue` as `false`, so the server HTML and the client’s initial state match. Matching initial output is important for successful hydration.

### Browser hydration

The browser then downloads the generated WebAssembly client bundle.

The WASM application starts with:

```rust
let mut is_blue = use_signal(|| false);
```

Dioxus hydrates the existing HTML rather than needing to construct a completely unrelated page from scratch. It attaches the client-side Dioxus runtime to the server-rendered markup.

### The 30-second delay

This code starts after hydration:

```rust
use_effect(move || {
    spawn(async move {
        TimeoutFuture::new(30_000).await;
        is_blue.set(true);
    });
});
```

`TimeoutFuture::new(30_000)` waits for 30,000 milliseconds:

```text
30,000 ms = 30 seconds
```

Then:

```rust
is_blue.set(true);
```

updates the signal.

Dioxus notices that the signal changed, re-renders the affected expressions, and changes:

```rust
let background = if is_blue() { "blue" } else { "white" };
```

from:

```rust
"white"
```

to:

```rust
"blue"
```

The browser DOM then effectively becomes:

```html
<div style="min-height: 100vh; background-color: blue;">
```

The `use_effect` hook is deliberately used because it runs on the client after hydration. Dioxus recommends putting client-only or nondeterministic behavior in effects so it does not cause the server-rendered HTML and hydrated client state to disagree. cite [dioxuslabs](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr/)

## Run it

For a Fullstack Dioxus project, use the Dioxus CLI rather than only `cargo run`:

```bash
dx serve --web
```

The Fullstack build performs roughly this process:

```text
Rust server
    ├─ Renders initial HTML
    ├─ Serves the HTML
    └─ Serves the generated WASM bundle

Browser
    ├─ Receives SSR HTML immediately
    ├─ Downloads WASM
    ├─ Hydrates the HTML
    └─ Changes the background after 30 seconds
```

The key distinction from your original SSR-only example is that this version includes a **web target** and a browser-side WASM bundle. Without the web/WASM build, `use_effect`, signals, and event handlers cannot continue running in the browser after the server sends the HTML.
