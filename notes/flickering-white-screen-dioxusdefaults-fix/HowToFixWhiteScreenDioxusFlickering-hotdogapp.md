TLDR:

TWO THINGS:
1. Basically if you do "dx new" and you select feature "fullstack" THEN THE WHITE FLICKER IS GONE. Otherwise, it is SSR and you will see a WHITEFLICKER on the HERO ALWAYS!!!!!!!!!!!!!!!!!!

2. AND THEN if you need to LOAD A data.json from the SERVER SIDE, use_server_future WILL FIX THIS SO IT DOESNT FLICKER TOOOOOO!!!!!!!!!!!!

Thats it!!!!!!!!!!!!!!!!!!!

























MY NOTES:
07ced0fb37 switched from use_resource to use_server_future and the flickering is fixed!!! 2 weeks ago 47acbb088e added todo for env_logger/log BEFORE DOCKERIZE Here is what the diff is, can you tell me whats wrong so that all my new dioxus apps i can remember to get rid of flickering?


YES. 🎯 You found the exact commit that explains it.

And this is actually much simpler than what we were chasing with the CSS.

## The fix was `use_server_future`

Your own commit message says it:

> **"switched from use_resource to use_server_future and the flickering is fixed!!!"**

And the diff confirms the only meaningful change related to the loading behavior was:

### Slow version

```rust
let buttons = use_resource(|| async { load_json().await.unwrap() });
```

### Fast version

```rust
let buttons = use_server_future(|| async { load_json().await })?;
```

That's the thing you want to remember.

---

# Why this fixed the flicker

Your `load_json()` is a `#[server]` function:

```rust
#[server]
async fn load_json() -> Result<BTreeMap<...>, ServerFnError>
```

So when you used:

```rust
use_resource(...)
```

Dioxus essentially had to:

```text
Start application
       ↓
Render App
       ↓
Start async load_json()
       ↓
Nothing available yet
       ↓
Render loading/empty state
       ↓
Server request completes
       ↓
BUTTONS appear
       ↓
UI rerenders
```

That gap is what you were seeing as the flicker.

With:

```rust
use_server_future(...)
```

Dioxus can execute that server future as part of the server-side rendering process and transfer the result into the client-side application.

Conceptually:

```text
Server starts
    ↓
load_json()
    ↓
HTML is generated WITH buttons
    ↓
Browser receives already-populated page
    ↓
WASM starts
    ↓
Dioxus hydrates existing UI
```

So there isn't that obvious:

```text
"empty/loading page"
        ↓
"OH, HERE'S THE APP"
```

transition.

---

# This is the part to remember for your future Dioxus apps

If you have a server function like:

```rust
#[server]
async fn get_something() -> Result<MyData, ServerFnError> {
    ...
}
```

and that data is required to render your initial page, **prefer:**

```rust
let data = use_server_future(|| async {
    get_something().await
})?;
```

rather than:

```rust
let data = use_resource(|| async {
    get_something().await.unwrap()
});
```

### In other words:

**`use_resource`**

> "Start this async thing when the app is running."

**`use_server_future`**

> "This is server data needed for the initial page; get it during SSR and carry it into the client."

That's why it made such a dramatic difference.

---

## And your diff gives us another clue

Look at the old version:

```rust
//if let Some(buttons) = buttons() { // <--- this is if use_resource and SLOW
if let Some(Ok(buttons)) = buttons() {
```

That comment you wrote at the time is basically your future-self documentation. 😂

You already knew exactly what was happening:

```text
use_resource = SLOW
use_server_future = FIXED FLICKER
```

And then your later Admindash3 code still has:

```rust
let buttons = use_server_future(|| async { load_json().await }).unwrap();
let navbar = use_server_future(|| async { load_navbar().await }).unwrap();
```

So you successfully carried the fix forward.

---

# The "rule" I'd put in your brain

For your Dioxus dashboard architecture:

### Initial/server data

Use:

```rust
use_server_future
```

For example:

```rust
let buttons = use_server_future(|| async {
    load_json().await
})?;

let navbar = use_server_future(|| async {
    load_navbar().await
})?;
```

### Client-only async work

Use:

```rust
use_resource
```

when the operation genuinely belongs to the running client and **doesn't need to be part of the initial SSR render**.

---

## So the complete answer to "what did we do?"

It wasn't:

* ❌ `build.rs`
* ❌ `Dioxus.toml`
* ❌ Tailwind
* ❌ `document::Stylesheet`
* ❌ some special CSS trick

It was this:

```rust
// ❌ caused initial loading/flicker
let buttons = use_resource(|| async {
    load_json().await.unwrap()
});
```

changed to:

```rust
// ✅ SSR-friendly; fixed the initial flicker
let buttons = use_server_future(|| async {
    load_json().await
})?;
```

And then you also used the same pattern for the navbar.

**That commit is basically the smoking gun.** 🔫

So when you spin up a new Dioxus project, the thing to remember is:

> **If the initial page needs server data, don't casually reach for `use_resource`; use `use_server_future` so the initial render can contain that data.**

That is the lesson from your `07ced0fb37` commit.


