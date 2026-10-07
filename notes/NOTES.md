# How I made the dockerfile
https://dioxuslabs.com/learn/0.7/tutorial/deploy

# How to shrink the wasm
The official way is "dx build --release". They also said that in the future they will use wasm-opt, but you can also run it this way: 
wasm-opt dist/assets/dioxus/APP_NAME_bg.wasm -o dist/assets/dioxus/APP_NAME_bg.wasm -Oz

# Front End Library Metric Comparison (for Dioxus instead of actix/axum + tera or minijinja)
In the browser, if YOU DO DEV TOOLS >> NETWORK TAB, the numbers actually tell a nice story. TLDR IS.. current architecture intentionally renders once before the button data is available. That's a common pattern in client-driven SPAs, but it's not the only way to build a Dioxus app.

Dioxus:
DOMContentLoaded: 53 ms
Finish: 369 ms

Flask:
DOMContentLoaded: 329 ms
Finish: 456 ms

So Dioxus is getting something on screen much sooner, but it's initially an incomplete UI that later fills in.
Flask is slower to start showing anything, but the first thing it shows is already complete.
That's why the Flask version feels smoother even though it's actually slower overall.

# Frontend Bug Test Ideas
* SSR HTML appears before WASM
* fast clicks before WASM hydration
* typing before WASM loads
* navbar navigation
* modal opening/closing
* Escape
* search /
* search Ctrl+K
* arrow wrapping
* clicking outside search results
* dynamic data_navbar.json
* iframe navigation
