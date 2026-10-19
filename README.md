# Admindash3
Re-written in rust because "Rust is sensible".

# How to launch
* Install git, rust, and dioxus (dioxuslabs.com for wasm toolchain + cli), ```curl -sSL https://dioxus.dev/install.sh | bash```
* EDIT AND CHMOD your data_buttons.json and data_navbar.json so that the docker service can read and write to them. For example, ```chmod 777 data_buttons.json ; chmod 777 data_navbar.json``` <-- no longer necessary, use the GUI to add button then a row for url bookmarks.
* Serve debug version via:   ```cd app; dx serve --port 9999 --addr 0.0.0.0```
* Serve release version via: ```cd app; dx bundle --web --release; cd /target/dx/admindash3/release/web/public ; IP=0.0.0.0 PORT=9999 ../server```
* Serve docker final version via: ```cd app; bash ./Dockerfile_BuildContainer_from_cwd.sh && docker compose up -d```
* Now use the buttons, or search something and it will now sort result list by BUTTON NAME > HASHTAGS > URLROWNAME

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
* STRETCHGOAL: Do the whole thing for cp + rshell + kanban to merge in this Dioxus. (+bonus for cp/ nginx)
* STRETCHGOAL: Make navbar and buttons hover to edit (css use hover).
* STRETCHGOAL: Add support ssh:// and rdp:// orwhatever it officially is to the links (with a gpo later or whatever).
* Hydration SSR of HTML > WASM works, but the addressbar changes for a sec (i dont mind it, but it violates W3C and WCAG!)
* Add env_logger/log and replace all println() with debug!().
* Fix the css grid when MANY>4 buttons are there, the last button's row isnt centered (it is currently 2,3,5 cols tailwind)
* CSS make the navbar>navitems highlight which is selected (bigger area to click to make it easier to click!!) (underline the navitem for example). Also... Would be cool if you switch to CP/ it underlines it (WITH A KEYBOARD SHORTCUT TOO specific to CP and WIKI for example so that i dont have to click between 'em ?), like CTRL+1, CTRL+2, etc...
* time to refactor, move the static global variable into a arcnewmutexnew thingy, so that we can make a dynamic pencil/plus/minus in navbar or in buttons to hover edit.
* Challenge accepted - migrate from a json to a rusqlite so that when we decide to EDIT/ADD BUTTONS... it will.. The SQLite database becomes your persistent storage, while the BTreeMap becomes your working/in-memory representation, and have crud endpoints?

* v2.2.x Future if you click the wiki or the cp, it should adjust the title in the browser to say Admindash3 (cp) or (wiki) or whatever the navbarjson had!!! +update the favicon?
* Encrypt and argon2ize the json file secrets. (password hashed)

@@@IMPORTANT@@@
- if you F5 AFTER LOGGING IN AS ADMIN, there is a flicker. Need to figure out what use_server_future or thingy is causing this.
- MAKE THE COMMENTS FIELD a WYSIG to support images for url rows that is allows a long string of text + images attachments (or a separate attachments)
- featreq: add a very subtle highlight saying newly updated on the button as well as the button row on first login, and also on every save of a button or a buttonrow. The highlight should only last 15 seconds, look professional and go away, then only re-appear after logout/login 
- sometimes if you TYPE TOO FAST, the search diddnt actually work or take into the textinput field (rare issue)
- for some reason the _sourcecodes_.tar is 30M, probly due to ~/.git being huge? idk (300+ commits and ai garbage maybe)
- fix the Dockerfile_BuildContainer_from_cwd.sh script so that it actually uses a "rust:latest" instead of Ubuntu, this is so that we no longer need RUST+DIOXUS installed on the node attempting to build the container




v2.7.7 goals (COMPLETED!!!!!!!!!!!!!!!!!!!!!!)
- if the user has the edit url modal or edit button modal (clone/move modals too?) which has input text fields to type in, then the ¥ backslash trigger to toggle the login/logout screen should be disabled, this will allow them to enter url names or secrets which have the backslash character, so that those edit modals' modal input fields will treat typing the backslash as a normal character. 
- search results > sometimes the pencil icon is not in the center of the row, as some results have more data, can the pencil still remain on the right side but centered vertically correctly on mouseover for the search result? Make sure the pencil icon still remains mouse clickable on hover in every scenario, like after you open a button to see the rows, we should be able to hover the pencil and left click it to open the respective edit modal.
- Also the backslash button that toggles the login screen, in both logged in and logged out state works perfectly, but a macbook japanese keyboard when you type backslash it does this character ¥ (looks like a yen symbol), we should make this do the same behaviour as well.
(2.7.7 still doesnt have moving url working yet)
- a Move URL button should be added next to the clone, so that a url can moved from inside one button to another existing button OR a new button to be created. It should allow selecting existing button destination (which should already be sorted by default), If it moves into a new button, present the create new button modal prepopulated with the url information already. And of course if this url row was the only button and you clicked move, it would just rename the button in same modal mentioned above. Please consider all cases to make sure we got them all.

v2.7.8 goals (COMPLETED!!!!!!!!!!!!!!!!!!!!!!)
- change the footer to say built with dioxus v0.7.10 instead of rust. *BUT LEAVE THAT ROW COMMENTED IN THE CODE SO I CAN GO BACK TO IT, put a comment with @@@@ symbols so it sticks out please, so i can comment and go back and forth.*
- all of the modals should be movable, and all of the modals should be resizable top/bottom left/right while still being responsive. They all look fine, just need the movable and resizable capability.
- the automatic backups are just a single json, maybe they should be a folder and complete copy of all the local *.json files like the tags and navbars too, but in a way that does not break the current code of the minidiff tool. The export backup button already does this nicely as a zip file, so maybe the same kinda thing? I want it to be the same as the export zips, but without breaking ANY functionality of the existing app as json files are extremely important.
- When you Add Row Divider, it should auto populate the inputtext field with the next available divider name like DIVIDER1 or DIVIDER2 or so on, so that it is unique in the json list for that specific button and parsing/rendering rows doesnt break anything.

v2.7.9 (COMPLETE)
- found a bug where the WIDTH only WHEN DRAGGING some modals (i think) are forced to have a maximum width and stay in the middle, so the width is limited for the user - which i want to allow them to go do any width if they drag. I want to keep the width the usual default when you open it, but WHEN the user decides to resize any modal, it is freeform and they can do anything. Also when they resize the browser should keep the scrollbar appearing in case they resize it to be smaller vertically.
- after logging in as admin, and opening up a button, sometimes if the text of the card is long or short the Drag button and edit button appear on top of each other, can they remain on the rightside but not on top of each other, maybe side by side?










@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@


v2.8.1 (CURRENTLY WORKING ON THIS)
- BUG FOUND if you login as admin > then mouse over the first button and click pencil edit icon the modal appears, but if you press tabkey its tabbing through the buttons in the background, it should have autofocused on the open'd modal. Need to check this for all possible locations to ensure the app is keyboard compatible. This happens also on the +Button +Tags +History? etc... check em all. Whenever any modal is open the TAB key should only behav on the topmost modal
- fix the footer.rs and build.rs to add a variable which somehow gets the "dioxus-web" version from Cargo.toml into a env variable jsut like the rust one (leave existing stuff in place), but now make the footer show that version. (Leave the existing @@@ code so i can show either the rust version or a hardcoded string, adjust comments appropriately)











(NOT IMPLEMENTED YET!!!!!!!!!!!)
schema change request...
- make a bin/migrator v4 which will allow a new field in the data_buttons json which has a SITE field which can be OR/FA/SW/OTHER. Thus, every URL must contain a SITE, and a new modal to add/save/change site fields (just like we did with the +Tags should be handled). Thus a migrator script to add the SITE field should be added, and then all of the edit modals should handle this appropriately. The migrator and also the create button urls should default to the OTHER by default. Finally, the url list inside the buttons AND the search results from the start menu should show a tiny breadcrumb (ONLY if the field is NOT OTHER), this way it looks nice in the result screens. Each site result in both should be a slightly different highlight so that the user can quickly identify by looking at the list to scroll.
- migratorv4 also needs... add a description field to the button which is shown only when you open the button list of urls. (this is a json change, so a ./src/bin/migrator is required for this v4 --- see the ./src/bin/migrators source code for this schema change, and FIX EVERYWHERE in the APP to allow this new field).










v3, v4, v5:
- refactor code so that there is a "./src/adminbuttons" module, and future cp/rm modules
- integrate copypasta
- integrate nginx rprox (to allow typing http://cp/ to redirect to iframe'd page) (after copypasta is rewritten of'co.)
- integrate remoteshell
- integrate kanban/wekan
