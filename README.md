# Admindash3
Re-written in rust because "Rust is sensible".

# How to launch
* Install git, rust, and dioxus (dioxuslabs.com for wasm toolchain + cli), ```curl -sSL https://dioxus.dev/install.sh | bash```
* EDIT AND CHMOD your data_buttons.json and data_navbar.json so that the docker service can read and write to them. For example, ```chmod 777 data_buttons.json ; chmod 777 data_navbar.json```
* Serve debug version via:   ```cd app; dx serve --port 9999 --addr 0.0.0.0```
* Serve release version via: ```cd app; dx bundle --web --release; cd /target/dx/admindash3/release/web/public ; IP=0.0.0.0 PORT=9999 ../server```
* Serve docker final version via: ```cd app; bash ./Dockerfile_BuildContainer_from_cwd.sh && docker compose up -d```

# What I learned / still learning...
So basically I wanted to... Load various ./data_*.json files for navitems or booksmarks containing buttons and plot on a webpage. Eventually load from sqlite3 tables, with history/undo enabled.

I wanted to continue learning rust concepts BUT ALSO "works before WASM, becomes reactive after WASM" via Dioxus.
* HashMap is unordered → use Vec / BTreeMap depending on goal to sort just the Button Name Keys.
* Serde needs your Rust structs to match the JSON shape exactly else use serde_json::Value!!! + serde_json::feature of preserve_order.
* Dioxus abstracts away a lot of the runtime details (love this! basically everything in a single .rs file)
* server-side data loading keeps your client clean <-- this was the #[server] stuff, very cool!
* usemounted() vs hydration recovery mechanisms that we can use to optimize the front end (_pending stuff!)

I also learned that if a upstream dep was not updated to work AFTER I DID "rustup update", i need to fix cargo.toml, or temporarily compile with a older version >>> _rustup override set 1.98.1_ for example!!

# Dioxus Flow
```bash

The jist of it is basically...
                 
                         ┌──────────────────────┐
                         │       SERVER         │
                         │                      │
                         │  data_buttons.json   │ (or sql db)
                         │          │           │
                         │          ▼           │
                         │      load_json()     │
                         │                      │
                         │  data_navbar.json    │ (or sql db)
                         │          │           │
                         │          ▼           │
                         │     load_navbar()    │
                         └──────────┬───────────┘
                                    │
                                    ▼
                              SSR / Hydration
                                    │
                    ┌───────────────┴──────────────┐
                    ▼                              ▼
              BUTTONS data                    NavItem data
                    │                              │
                    ▼                              ▼
              Main dashboard                    Navbar
                                                   │
                                                   ▼
                                               NavEntry
```

# Todo
* STRETCHGOAL: Do the whole thing again for fun practice with SSR like Topcoat/Axum/Actix, but tbh <3 Dioxus.
* STRETCHGOAL: Do the whole thing for cp + rshell to merge in this Dioxus. (+bonus for cp/ nginx)
* STRETCHGOAL: Make navbar and buttons hover to edit (css use hover).
* Hydration SSR of HTML > WASM works, but the addressbar changes for a sec (i dont mind it, but it violates W3C and WCAG!)
* Add env_logger/log and replace all println() with debug!().
* Fix the css grid when MANY>4 buttons are there, the last button's row isnt centered
* CSS make the navbar>navitems highlight which is selected (bigger area to click to make it easier to click!!) (underline the navitem for example).
* Make middle clicking the searchbox listpopup to work in new window
* Would be cool if you switch to CP/ it underlines it (WITH A KEYBOARD SHORTCUT TOO specific to CP and WIKI for example so that i dont have to click between 'em ?), like ALT+1, ALT+2, etc...
* time to refactor, move the static global variable into a arcnewmutexnew thingy, so that we can make a dynamic pencil/plus/minus in navbar or in buttons to hover edit.
* if there is a typo in the data.json it seems that the frontend doesnt reflect this in release mode, maybe figure out a error message for this
* Noticed a bug maybe? if you click searcbox > type something > result list pops up > you press down to highlight something in the middle > press enter > go back, the dropdown remains open but the down arrow is reset to top. this might be a thing we can fix useEffect() or something?
* make the container a bit smaller, im jealous of SSR/AXUM (tower http cors like kansat-be example)
* Challenge accepted - migrate from a json to a rusqlite so that when we decide to EDIT/ADD BUTTONS... it will.. The SQLite database becomes your persistent storage, while the BTreeMap becomes your working/in-memory representation, and have crud endpoints?

* v2.2.x Future if you click the wiki or the cp, it should adjust the title in the browser to say Admindash3 (cp) or (wiki) or whatever the navbarjson had!!!
* argon2ize the json file secrets. (password hashed)
* minor bug, but you click a pencil icon and scroll you will see that it scrolls the page while the pencil icon is open

@@@IMPORTANT@@@
- if you F5 AFTER LOGGING IN AS ADMIN, there is a flicker. Need to figure out what use_server_future or thingy is causing this.
- BUGFOUND >> The upstream WASM has changed, and v2.4.3 was compiled on rust version old, so in order to compile --- WE MUST USE " rustup override set 1.98.1", I need to update and test the new cargo.toml ASAP to make sure this still IS NOT a problem. (wasm-bindgen does not compile on 1.99.1 at the moment!!!!!!!!!!!!!"
- MAKE THE COMMENTS FIELD a WYSIG to support images for url rows that is allows a long string of text + images attachments (or a separate attachments)
- featreq: add a very subtle highlight saying newly updated on the button as well as the button row on first login, and also on every save of a button or a buttonrow. The highlight should only last 15 seconds, look professional and go away, then only re-appear after logout/login 

- upstream bug --- rust 1.99 and llvm23 --strip-all removes the wasm binden unstable custom section that wasm-bindgen used... so im forced to use rust1.98 until this is fixed with: rustup override set 1.98.1  then compile it... Check this again next month?

- The "Edit Row" and "Edit Button" are missing the red X to close it at the top right. We need to check and also make sure the other modals are all ok too?
- the "Edit Row" needs a scrollbar when zoomed in too far, other modals might also - need to check all of them.

- when typing in the searchbox, we want the list to be sorted by button name first, then hashtags, then by urlname 

v3, v4, v5:
- refactor code so that there is a "./src/adminbuttons" module, and future cp/rm modules
- integrate copypasta
- integrate nginx rprox (to allow typing http://cp/ to redirect to iframe'd page) (after copypasta is rewritten of'co.)
- integrate remoteshell
- integrate kanban/wekan
