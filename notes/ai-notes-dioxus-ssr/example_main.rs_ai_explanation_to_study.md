Yes—this is a **Dioxus Fullstack server-side-rendered application**, specifically an SSR-only app. In this example, the server generates the HTML for every page request, and there is no client-side WebAssembly/JavaScript bundle to make the page reactive in the browser.

## SSR versus CSR

You already know CSR:

1. Browser downloads a mostly empty HTML shell.
2. Browser downloads JavaScript or WebAssembly.
3. The client-side application starts.
4. The application fetches data.
5. The browser constructs and updates the DOM.

For example, a CSR response might initially contain only:

```html
<div id="app"></div>
<script src="/app.js"></script>
```

The browser then runs `app.js`, which creates the page.

With SSR:

1. Browser requests `/post/1`.
2. The Dioxus server matches the route.
3. Dioxus runs the relevant Rust components on the server.
4. The server loads the post data.
5. Dioxus converts the resulting component tree into HTML.
6. The browser receives complete HTML.

Dioxus describes SSR as loading data on the server before sending HTML to the client, while CSR generally sends a skeleton page and loads data in the browser. cite [dioxuslabs](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr/)

For `/post/1`, the browser may receive something conceptually similar to:

```html
<h1>Post 1</h1>
<p>first post</p>
```

The important detail is that this example does **not** hydrate the page afterward. Hydration would mean sending a client-side Dioxus bundle that reconnects the rendered HTML to signals, event handlers, and reactive updates. This example intentionally has no client-side bundle.

## Program entry point

```rust
use dioxus::prelude::*;

fn main() {
    dioxus::launch(|| rsx! { Router::<Route> { } });
}
```

### `use dioxus::prelude::*`

This imports the commonly used Dioxus types, macros, traits, and hooks, including:

- `rsx!`
- `Element`
- `Routable`
- `Router`
- `ReadSignal`
- `use_loader`
- `HttpError`

### `dioxus::launch(...)`

This starts the Dioxus application.

The closure passed to `launch` is the root component:

```rust
|| rsx! { Router::<Route> { } }
```

It renders a Dioxus router whose route type is `Route`.

Conceptually, this is the Dioxus equivalent of starting a web framework with an application router:

```rust
let app = Router::new()
    .route("/", home)
    .route("/post/:id", post);
```

Dioxus routes are represented as enum variants. The router parses the incoming URL, selects the matching enum variant, and renders the corresponding component. The `Routable` derive macro generates much of this routing behavior and validates the route structure at compile time. cite [dioxuslabs](https://dioxuslabs.com/learn/0.7/essentials/router/)

## The route enum

```rust
#[derive(Routable, Clone, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Home,

    #[route("/post/:id")]
    Post { id: u32 },
}
```

This defines two routes.

### Home route

```rust
#[route("/")]
Home,
```

The URL:

```text
/
```

maps to the `Home` component.

### Post route

```rust
#[route("/post/:id")]
Post { id: u32 },
```

This matches URLs such as:

```text
/post/1
/post/2
/post/123
```

The `:id` portion is a dynamic path segment. Dioxus parses it and converts it to the declared Rust type:

```rust
id: u32
```

Therefore:

```text
/post/2
```

becomes conceptually:

```rust
Route::Post { id: 2 }
```

A URL such as:

```text
/post/abc
```

does not match successfully because `"abc"` cannot be parsed as a `u32`.

The route variant name also determines which component Dioxus renders. The `Post` variant corresponds to the `Post` component, and `Home` corresponds to `Home`. Dioxus’s router documentation describes each route variant as a page/view that is parsed from the URL and rendered as a component. cite [dioxuslabs](https://dioxuslabs.com/learn/0.7/essentials/router/)

## The home component

```rust
#[component]
fn Home() -> Element {
    rsx! {
        h1 { "home"  }
        ul {
            li { a { href: "/post/1", "Post 1" } }
            li { a { href: "/post/2", "Post 2" } }
            li { a { href: "/post/3", "Post 3 (404)" } }
        }
    }
}
```

### `#[component]`

This tells Dioxus that `Home` is a component function.

The function returns:

```rust
Element
```

An `Element` is Dioxus’s representation of a component’s rendered output.

### `rsx!`

`rsx!` is Dioxus’s JSX-like markup syntax. This:

```rust
rsx! {
    h1 { "home" }
}
```

produces an element equivalent to:

```html
<h1>home</h1>
```

The full component produces approximately:

```html
<h1>home</h1>
<ul>
    <li><a href="/post/1">Post 1</a></li>
    <li><a href="/post/2">Post 2</a></li>
    <li><a href="/post/3">Post 3 (404)</a></li>
</ul>
```

Because this is SSR-only, the `<a>` elements behave like ordinary HTML links. Clicking one causes a new HTTP request to the server. There is no client-side router intercepting the click and replacing only part of the page.

In a normal client-enabled Dioxus application, navigation can happen in the browser without a complete page reload. That is not what this particular example is demonstrating.

## The post component

```rust
#[component]
fn Post(id: ReadSignal<u32>) -> Element {
```

This component receives the route parameter as `id`.

The route definition declared:

```rust
Post { id: u32 }
```

but the component receives:

```rust
ReadSignal<u32>
```

A `ReadSignal<T>` is a reactive read-only value in Dioxus. You read its current value by calling it like a function:

```rust
id()
```

So this:

```rust
id()
```

returns the current numeric route parameter.

For `/post/1`, it returns:

```rust
1
```

For `/post/2`, it returns:

```rust
2
```

Even though this example does not have a client bundle, Dioxus still uses its normal component and signal abstractions while rendering on the server. The signal is useful because the component API is compatible with Dioxus’s normal reactive model.

## Loading the post

```rust
let post_data = use_loader(move || get_post(id()))?;
```

This is the most important line.

### `use_loader`

`use_loader` runs an asynchronous data-loading operation and integrates the result with Dioxus rendering, loading, suspense, and error handling.

Dioxus 0.7 introduced `use_loader` as a hook intended for data loading that works across SSR and CSR. It accepts a callback returning a `Result`, and errors can propagate through Dioxus’s error/suspense system. cite [dioxuslabs](https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum/)

The callback is:

```rust
move || get_post(id())
```

That means:

1. Read the route parameter with `id()`.
2. Call `get_post(...)`.
3. Wait for the returned future.
4. Use the resulting string as `post_data`.

The `move` keyword moves the captured `id` signal into the closure so the closure owns what it needs.

### Why the `?` is present

```rust
use_loader(move || get_post(id()))?;
```

The `?` means:

- If the loader is still pending, suspend rendering.
- If it succeeds, continue and assign the loaded value.
- If it fails, propagate the error to Dioxus’s error handling.

In this SSR-only example, the server waits for the loader to finish before producing the final HTML. Therefore `/post/1` does not initially render a loading screen and then fetch the data in the browser. The server waits for `"first post"` and includes it in the generated response.

Dioxus’s SSR process generally runs components, waits for relevant server-side futures, and then renders the resulting component tree into HTML. cite [dioxuslabs](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr/)

## Rendering the post

```rust
rsx! {
    h1 { "Post {id}" }
    p { "{post_data}" }
}
```

This generates:

```html
<h1>Post 1</h1>
<p>first post</p>
```

There is a small conceptual detail here:

```rust
"Post {id}"
```

uses Dioxus’s formatting syntax to display the signal’s value. In practical terms, it renders the current route ID.

The second expression:

```rust
"{post_data}"
```

renders the string returned from `get_post`.

For `/post/2`, the result is approximately:

```html
<h1>Post 2</h1>
<p>second post</p>
```

## The server function

```rust
#[get("/api/post/{id}")]
async fn get_post(id: u32) -> Result<String, HttpError> {
```

This defines an asynchronous server-side function.

The attribute:

```rust
#[get("/api/post/{id}")]
```

associates the function with the HTTP path:

```text
/api/post/{id}
```

The function accepts an unsigned integer and returns either:

```rust
Ok(String)
```

or:

```rust
Err(HttpError)
```

The endpoint is conceptually:

```text
GET /api/post/1
GET /api/post/2
GET /api/post/3
```

In a fullstack Dioxus application, server-function attributes allow a Rust function to be used as a server-side operation and exposed through the generated server routing machinery. In this SSR-only program, `Post` uses the function while rendering on the server.

## The fake database lookup

```rust
match id {
    1 => Ok("first post".to_string()),
    2 => Ok("second post".to_string()),
    _ => HttpError::not_found("Post not found")?,
}
```

This is just a hard-coded lookup.

It behaves like this:

| ID | Result |
|---:|---|
| `1` | `"first post"` |
| `2` | `"second post"` |
| Anything else | HTTP 404 |

In a real application, this could instead query PostgreSQL:

```rust
let post = sqlx::query_scalar!(
    "SELECT body FROM posts WHERE id = $1",
    id
)
.fetch_optional(&pool)
.await?;
```

The example avoids a database so it can demonstrate routing and SSR with minimal code.

## The 404 behavior

```rust
HttpError::not_found("Post not found")?
```

This creates a Dioxus HTTP error representing:

```text
404 Not Found
```

The `?` propagates that error out of `get_post`.

For this URL:

```text
/post/3
```

the flow is:

1. The router matches `Route::Post { id: 3 }`.
2. Dioxus renders `Post`.
3. `Post` calls `get_post(3)`.
4. `get_post` returns a not-found error.
5. Rendering fails with an HTTP 404 error.
6. The server responds with HTTP status `404`.

That is different from merely rendering a page containing the text `"Post not found"` with an HTTP 200 status. Search engines, browsers, reverse proxies, monitoring systems, and clients can distinguish the real 404 status correctly.

## Request flow

### Request for `/`

```text
Browser
   │
   │ GET /
   ▼
Dioxus server
   │
   ├─ Router matches Route::Home
   ├─ Renders Home
   ├─ Converts RSX to HTML
   ▼
Browser receives HTML
```

The response is approximately:

```html
<h1>home</h1>
<ul>
    <li><a href="/post/1">Post 1</a></li>
    <li><a href="/post/2">Post 2</a></li>
    <li><a href="/post/3">Post 3 (404)</a></li>
</ul>
```

### Request for `/post/1`

```text
Browser
   │
   │ GET /post/1
   ▼
Dioxus server
   │
   ├─ Router matches Route::Post { id: 1 }
   ├─ Post calls get_post(1)
   ├─ get_post returns "first post"
   ├─ Dioxus renders the component
   ▼
Browser receives HTML
```

The response is approximately:

```html
<h1>Post 1</h1>
<p>first post</p>
```

### Request for `/post/3`

```text
Browser
   │
   │ GET /post/3
   ▼
Dioxus server
   │
   ├─ Router matches Route::Post { id: 3 }
   ├─ get_post(3) returns HttpError::not_found(...)
   ▼
HTTP 404 response
```

## What this application does not do

This application does **not** provide:

- Client-side navigation.
- Browser-side signal updates.
- `onclick` event handling.
- A WebAssembly bundle.
- Hydration.
- Automatic partial page updates.
- Browser-side data fetching after the initial HTML.

The HTML is generated on the server and displayed by the browser. If you added a button such as:

```rust
button {
    onclick: move |_| count += 1,
    "Increment"
}
```

it would not work in this SSR-only mode because there is no client-side Dioxus runtime to receive and process the click.

## SSR-only versus normal Dioxus SSR

There are two related concepts:

### SSR with hydration

The server initially renders HTML, then the browser downloads a Dioxus client bundle and hydrates the page. This gives you:

- Fast initial HTML.
- Search-engine-readable content.
- Client-side interactivity afterward.
- Signals and event handlers in the browser.

### SSR-only

The server renders HTML, but the browser receives no client bundle. This gives you:

- Very small client requirements.
- Simple server-rendered pages.
- HTML that works without JavaScript.
- No client-side reactivity or event handling.

The example you posted is the second type. It is suitable for static or mostly static pages, documentation, blogs, simple content sites, and server-rendered pages where normal HTML links and full requests are acceptable. Dioxus also supports hybrid approaches where server-rendered HTML is later hydrated to become interactive. cite [dioxuslabs](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr/)

In one sentence: **the Rust code is executing on the server, `rsx!` is being converted into HTML there, `use_loader` waits for the post data before rendering, and the browser receives ordinary finished HTML rather than a client-side Dioxus application.**
