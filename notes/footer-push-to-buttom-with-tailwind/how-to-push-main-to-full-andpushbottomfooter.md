Yep — that's a CSS layout issue rather than a Dioxus issue. Right now the footer is just another element in the document flow, so it appears immediately after the button grid.

What you want is the classic **sticky footer layout**:

```text
┌──────────────────────────────┐
│                              │
│          NAVBAR              │
│                              │
│          BUTTONS             │
│                              │
│                              │
│                              │
│                              │
├──────────────────────────────┤
│          FOOTER              │
└──────────────────────────────┘
```

And importantly, if there are enough buttons to make the page taller than the browser, the footer should **naturally move below the content**, rather than sitting over it.

### 1. Change your outer page container

Your `App` currently has your navbar and content directly inside the top-level `rsx!`.

Wrap the actual application in a flex column:

```rust
rsx! {
    document::Link {
        rel: "icon",
        href: asset!("/assets/favicon.ico")
    }

    document::Stylesheet {
        href: asset!("/assets/tailwind.css")
    }

    // -------------------------------------------------------------------------
    // PAGE WRAPPER
    //
    // min-h-screen means:
    //
    //     "At least as tall as the browser viewport."
    //
    // flex flex-col means:
    //
    //     navbar
    //     content
    //     footer
    //
    // are arranged vertically.
    // -------------------------------------------------------------------------
    div {
        class: "min-h-screen flex flex-col",

        // ---------------------------------------------------------------------
        // NAVBAR
        // ---------------------------------------------------------------------

        if let Some(Ok(items)) = navbar() {
            Navbar {
                items: items.clone()
            }
        }


        // ---------------------------------------------------------------------
        // MAIN CONTENT
        //
        // flex-1 is the important part.
        //
        // It tells this section:
        //
        //     "Take up all remaining vertical space."
        //
        // Therefore the footer gets pushed to the bottom when there isn't
        // enough content to fill the screen.
        // ---------------------------------------------------------------------

        main {
            class: "flex-1",

            // -------------------------------------------------------------
            // YOUR EXISTING IFRAME
            // -------------------------------------------------------------

            if let Some(url) = IFRAME_URL() {
                div {
                    class: "w-full",

                    iframe {
                        src: "{url}",
                        class: "w-full h-screen border-0",
                        title: "Embedded application",
                    }
                }
            }

            // -------------------------------------------------------------
            // YOUR EXISTING BUTTONS
            // -------------------------------------------------------------

            else if let Some(Ok(buttons)) = buttons() {
                div {
                    class: "container mx-auto px-4 mt-6",

                    div {
                        class: "grid grid-cols-2 md:grid-cols-3 lg:grid-cols-5 gap-3",

                        for (button_name, urls) in buttons.iter() {

                            // Your existing button...
                        }
                    }
                }

                // Your existing modal...
            }
        }


        // ---------------------------------------------------------------------
        // FOOTER
        //
        // Because <main> has flex-1, this naturally gets pushed toward the
        // bottom of the viewport when the content is short.
        //
        // If the content becomes taller than the viewport, the footer simply
        // appears after the content as normal.
        // ---------------------------------------------------------------------

        Footer {}
    }
}
```

### The magic is really only these three things

The important hierarchy is:

```text
id="0qqqpk"
div  min-h-screen flex flex-col
│
├── Navbar
│
├── main  flex-1
│   │
│   └── your buttons / iframe / modal
│
└── Footer
```

And the two Tailwind classes doing the heavy lifting are:

```text
min-h-screen
flex-1
```

`min-h-screen` says:

> Make the whole page at least one browser viewport tall.

`flex flex-col` says:

> Stack everything vertically.

`flex-1` says:

> Main content gets all the leftover space.

So if you only have three buttons:

```text
Navbar
│
│
│
│
│
Main
│
│
Footer  ← pushed down
```

But if you have 100 buttons:

```text
Navbar
Main
Main
Main
Main
Main
Main
Footer  ← naturally below everything
```

That's preferable to `fixed bottom-0`, because **`fixed` would make the footer sit on top of your content when the page gets long**.

One tiny caveat in your current app: your iframe has `h-screen`, so when an iframe is displayed, it intentionally takes a full viewport height itself. That's fine for now, but it means the footer will be below that full-height iframe rather than immediately visible. That's probably the behavior you'd want.

