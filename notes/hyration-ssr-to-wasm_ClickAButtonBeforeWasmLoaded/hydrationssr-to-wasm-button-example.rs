// -----------------------------------------------------------------------------
// DIOXUS SSR -> WASM HYDRATION DEMO
// -----------------------------------------------------------------------------
//
// This tiny application demonstrates the basic lifecycle:
//
//
//
//      SERVER
//        |
//        |  SSR
//        v
//   HTML is generated
//        |
//        v
//     BROWSER
//        |
//        |  WASM loads
//        v
//    DIOXUS HYDRATES
//        |
//        v
//   onclick becomes active
//
//
//
// The important thing to understand:
//
// SSR is responsible for creating the initial HTML.
//
// WASM is responsible for making that existing HTML interactive.
//
// We are NOT waiting for WASM to create the page from scratch.
// The browser already received the HTML from the server.
// -----------------------------------------------------------------------------

use dioxus::prelude::*;


// -----------------------------------------------------------------------------
// MAIN
// -----------------------------------------------------------------------------
//
// dioxus::launch() starts the Dioxus application.
//
// In a web application, Dioxus can use SSR to generate the initial HTML.
//
// The browser then receives that HTML and later loads the WASM application,
// which hydrates the existing HTML.
// -----------------------------------------------------------------------------

fn main() {

    dioxus::launch(App);

}


// -----------------------------------------------------------------------------
// APP COMPONENT
// -----------------------------------------------------------------------------

#[component]
fn App() -> Element {

    // -------------------------------------------------------------------------
    // This is a normal Dioxus signal.
    //
    // Initially:
    //
    //     clicked = false
    //
    // SSR can render the initial value.
    //
    // Once WASM hydrates the page, clicking the button can modify this signal.
    // -------------------------------------------------------------------------

    let mut clicked = use_signal(|| false);


    rsx! {

        // ---------------------------------------------------------------------
        // PAGE
        // ---------------------------------------------------------------------

        div {
            class: "p-10",

            // -------------------------------------------------------------
            // HELLO WORLD
            //
            // This text can be rendered by SSR.
            // -------------------------------------------------------------

            h1 {
                class: "text-3xl font-bold",

                "Hello World!"
            }


            // -------------------------------------------------------------
            // THIS BUTTON IS THE INTERESTING PART
            // -------------------------------------------------------------
            //
            // During SSR:
            //
            //     Dioxus generates the HTML for this button.
            //
            // The browser can therefore SEE the button even before WASM
            // has finished loading.
            //
            // BUT...
            //
            // The Rust onclick handler isn't running yet.
            //
            // After WASM loads:
            //
            //     Dioxus hydrates this element
            //
            // and the onclick becomes active.
            // -------------------------------------------------------------

            button {

                class: "
                    mt-4
                    px-4
                    py-2
                    bg-blue-500
                    text-white
                    rounded
                ",

                // ---------------------------------------------------------
                // THIS IS THE WASM INTERACTIVE PART
                // ---------------------------------------------------------
                //
                // When WASM has hydrated the page, clicking this button
                // executes this Rust closure.
                //
                // Before hydration, there is no active Dioxus event
                // handler attached to the browser button yet.
                // ---------------------------------------------------------

                onclick: move |_| {

                    // -----------------------------------------------------
                    // Change our signal.
                    //
                    // This causes Dioxus to render again.
                    // -----------------------------------------------------

                    clicked.set(true);

                },

                "Click Me!"
            }


            // -----------------------------------------------------------------
            // RESULT
            // -----------------------------------------------------------------
            //
            // Initially:
            //
            //     clicked == false
            //
            // so this isn't displayed.
            //
            // After the WASM onclick runs:
            //
            //     clicked == true
            //
            // and Dioxus renders the message.
            // -----------------------------------------------------------------

            if clicked() {

                p {
                    class: "mt-4 text-green-600",

                    "🎉 Hello WASM!"
                }

            } else {

                p {
                    class: "mt-4 text-gray-500",

                    "Waiting for the button..."
                }

            }
        }
    }
}
