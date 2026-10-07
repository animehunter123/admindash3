Yep — **you're exactly right: this is in `main.rs`**, and it's essentially the same hydration problem we solved for the navbar.

The important difference is that your navbar entries are `<a href="...">`, so the **browser itself has something to do before WASM arrives**.

Your dashboard buttons are currently:

```rust
button {
    onclick: ...
}
```

Before WASM hydrates, that `onclick` doesn't exist yet. So the browser sees a button with **no native action** → click disappears into the void.

We can solve it using the **same pending-navigation trick we already built for the navbar**, except instead of queuing a URL, we'll queue **which dashboard button was clicked**.

---

# The idea

Currently:

```text
Browser loads SSR HTML
        ↓
User clicks button
        ↓
<button> has no WASM onclick yet
        ↓
💀 click disappears
```

We'll make the button effectively behave like:

```text
<a href="#__pending_button?name=...">
```

So:

```text
Browser loads SSR HTML
        ↓
User clicks button
        ↓
Browser changes URL fragment
        ↓
WASM loads
        ↓
App sees __pending_button
        ↓
selected = clicked button
        ↓
Modal opens
        ↓
temporary fragment removed
```

This is basically the **exact same architecture you already understand from the navbar**, which makes it a good next step.

---

# 1. Add the pending-button recovery to `App`

You already have this in `navbar.rs`:

```rust
#__pending_nav?url=...
```

We're going to make another fragment:

```text
#__pending_button?name=...
```

Inside `App()`, I'd put the new effect **after your existing `BUTTONS` effect**:

```rust
use_effect(move || {
    if let Some(Ok(buttons)) = buttons() {
        *BUTTONS.write() = buttons.clone();
    }
});
```

Add this immediately underneath it:

```rust
// -----------------------------------------------------------------------------
// RECOVER BUTTON CLICK THAT HAPPENED BEFORE WASM HYDRATION
// -----------------------------------------------------------------------------
//
// The dashboard buttons normally use a Dioxus onclick:
//
//     onclick: move |_| {
//         selected.set(...)
//     }
//
// But before WASM hydrates, that onclick does not exist yet.
//
// Therefore the SSR version of the button uses:
//
//     #__pending_button?name=<button name>
//
// The browser understands that URL immediately.
//
// Once WASM starts, this effect checks for the temporary fragment,
// finds the matching button, opens its modal, and then removes
// the temporary fragment from the browser URL.
// -----------------------------------------------------------------------------
use_effect(move || {
    let Some(win) = web_sys::window() else {
        return;
    };

    let location = win.location();

    let Ok(hash) = location.hash() else {
        return;
    };

    // -------------------------------------------------------------------------
    // Does the URL contain our queued button click?
    //
    // Example:
    //
    //     #__pending_button?name=PRINTERS
    // -------------------------------------------------------------------------
    if let Some(query_string) = hash.strip_prefix("#__pending_button?") {
        web_sys::console::log_1(
            &format!(
                "WASM mounted - found pending dashboard button: {:?}",
                query_string
            )
            .into(),
        );

        // ---------------------------------------------------------------------
        // Our format is:
        //
        //     #__pending_button?name=<encoded button name>
        // ---------------------------------------------------------------------
        if let Some(encoded_name) = query_string.strip_prefix("name=") {
            #[cfg(target_arch = "wasm32")]
            {
                let decoded = js_sys::decode_uri_component(encoded_name)
                    .ok()
                    .and_then(|value| value.as_string());

                if let Some(button_name) = decoded {
                    web_sys::console::log_1(
                        &format!(
                            "Opening queued dashboard button: {:?}",
                            button_name
                        )
                        .into(),
                    );

                    // ---------------------------------------------------------
                    // BUTTONS now contains the data loaded by load_json().
                    //
                    // Find the button that the user clicked before WASM existed.
                    // ---------------------------------------------------------
                    if let Some(urls) = BUTTONS.read().get(&button_name).cloned() {
                        selected.set(Some((button_name, urls)));
                    }
                }
            }
        }

        // ---------------------------------------------------------------------
        // Remove the temporary fragment.
        //
        // This changes:
        //
        //     /#__pending_button?name=PRINTERS
        //
        // back to:
        //
        //     /
        //
        // WITHOUT reloading the page.
        // ---------------------------------------------------------------------
        if let Ok(history) = win.history() {
            let _ = history.replace_state_with_url(
                &wasm_bindgen::JsValue::NULL,
                "",
                Some(
                    &location
                        .pathname()
                        .unwrap_or_else(|_| "/".to_string()),
                ),
            );
        }
    }
});
```

### One important thing

This effect uses `selected`, so **move your `selected` signal declaration above this effect**.

Right now you have:

```rust
let mut selected = use_signal(|| None::<(String, Vec<(String, String)>)>);
```

after your `BUTTONS` effect.

Move it up so the ordering becomes:

```rust
let buttons = use_server_future(|| async { load_json().await }).unwrap();

let navbar = use_server_future(|| async { load_navbar().await }).unwrap();

let mut selected =
    use_signal(|| None::<(String, Vec<(String, String)>)>);
```

Then your existing `BUTTONS` effect and new pending-button effect can both access `selected`.

---

# 2. Change the dashboard `button` into an `<a>`

This is the really important part.

You currently have:

```rust
for (button_name , urls) in buttons.iter() {

    button {
        class: "
            w-full bg-gray-300 hover:bg-gray-400 text-gray-800 font-medium
            py-2 px-4 rounded-md shadow-sm hover:shadow-md transition hover:scale-105
            active:scale-95
            truncate overflow-hidden whitespace-nowrap
        ",

        onclick: {
            let button_name = button_name.clone();
            let urls = urls.clone();

            move |_| {
                selected.set(Some((button_name.clone(), urls.clone())));
            }
        },

        "{button_name}"
    }
}
```

We're going to make the SSR HTML have an actual browser action.

Change it to:

```rust
for (button_name, urls) in buttons.iter() {

    a {
        // -----------------------------------------------------------------
        // BEFORE WASM:
        //
        // The browser will immediately navigate to this temporary fragment.
        //
        // Example:
        //
        //     #__pending_button?name=PRINTERS
        //
        // Once WASM loads, our effect above sees this fragment and
        // opens the appropriate modal.
        // -----------------------------------------------------------------
        href: {
            #[cfg(target_arch = "wasm32")]
            {
                let encoded =
                    js_sys::encode_uri_component(button_name);

                format!(
                    "#__pending_button?name={}",
                    encoded
                )
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                format!(
                    "#__pending_button?name={}",
                    button_name
                )
            }
        },

        class: "
            block
            w-full
            bg-gray-300
            hover:bg-gray-400
            text-gray-800
            font-medium
            py-2
            px-4
            rounded-md
            shadow-sm
            hover:shadow-md
            transition
            hover:scale-105
            active:scale-95
            truncate
            overflow-hidden
            whitespace-nowrap
            text-center
        ",

        onclick: {
            let button_name = button_name.clone();
            let urls = urls.clone();

            move |event| {
                // -------------------------------------------------------------
                // WASM IS NOW RUNNING.
                //
                // We don't actually want to follow the temporary href.
                //
                // Instead, Dioxus handles the click immediately.
                // -------------------------------------------------------------
                event.prevent_default();

                selected.set(Some((
                    button_name.clone(),
                    urls.clone()
                )));
            }
        },

        "{button_name}"
    }
}
```

Now you have **both behaviors**:

### Before WASM

The browser sees:

```html
<a href="#__pending_button?name=PRINTERS">
```

and handles it itself.

### After WASM

Dioxus handles:

```rust
onclick: ...
```

and prevents the temporary navigation:

```rust
event.prevent_default();
```

That's the exact same principle as your navbar.

---

# 3. There's a small issue we should account for

Your `BUTTONS` keys are uppercased here:

```rust
hash01.insert(button_name.to_string().to_uppercase(), vect01.clone());
```

So if your JSON says:

```text
Printers
```

your `BUTTONS` actually contains:

```text
PRINTERS
```

That's why the pending fragment should be generated from the **rendered `button_name`**, which is already the uppercase BTreeMap key.

So this:

```rust
button_name
```

will match:

```rust
BUTTONS.read().get(&button_name)
```

perfectly.

---

# What happens now

Let's say the page has:

```text
[ SERVERS ] [ NETWORK ] [ STORAGE ]
```

and the user is **absurdly fast** and clicks:

```text
STORAGE
```

before WASM finishes loading.

The SSR link is effectively:

```text
/#__pending_button?name=STORAGE
```

The browser immediately changes the fragment.

Then WASM loads.

Your effect sees:

```text
#__pending_button?name=STORAGE
```

finds:

```rust
BUTTONS["STORAGE"]
```

and executes:

```rust
selected.set(Some((
    "STORAGE".to_string(),
    urls
)));
```

which causes this existing code to render:

```rust
if let Some((name, urls)) = selected() {
    ...
}
```

and your modal appears.

Then:

```rust
history.replace_state_with_url(...)
```

changes the URL back to:

```text
/
```

without another page load.

---

## So now both parts of your application use the same hydration strategy

You've essentially got:

```text
                 SSR HTML
                    │
             ┌──────┴──────┐
             │             │
          Navbar         Buttons
             │             │
       <a href=...>    <a href=...>
             │             │
             │             │
             ▼             ▼
      pending_nav      pending_button
             │             │
             └──────┬──────┘
                    │
                WASM loads
                    │
             recover click
                    │
          ┌─────────┴─────────┐
          │                   │
     IFRAME_URL           selected
          │                   │
          ▼                   ▼
       iframe               modal
```

And **this is a really nice pattern for you to study**, because the browser isn't actually "waiting for WASM." We're making the **SSR HTML itself useful**, and then WASM comes along afterward and interprets the temporary state.

One caveat: I'd eventually refactor the duplicated `#__pending_*` recovery logic into a small helper, because your `navbar.rs` and `main.rs` are now doing almost the same thing. **But don't do that yet.** For studying, having the two implementations side-by-side is actually useful—you can see exactly how the hydration handoff works before we abstract it.

