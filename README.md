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
MAJOR V GOALS: v3, v4, v5:
- refactor code so that there is a "./src/buttonboard, navbar, cp, kanban, rshell" modules.
- integrate copypasta
- integrate nginx rprox (to allow typing http://cp/ to redirect to iframe'd page) (after copypasta is rewritten of'co.)
- integrate remoteshell
- integrate kanban/wekan
* Hydration SSR of HTML > WASM works, but the addressbar changes for a sec (i dont mind it, but it violates W3C and WCAG!)
* Add env_logger/log and replace all println() with debug!().
* Fix the css grid when MANY>4 buttons are there, the last button's row isnt centered (it is currently 2,3,5 cols tailwind)
* CSS make the navbar>navitems highlight which is selected (bigger area to click to make it easier to click!!) (underline the navitem for example). Also... Would be cool if you switch to CP/ it underlines it (WITH A KEYBOARD SHORTCUT TOO specific to CP and WIKI for example so that i dont have to click between 'em ?), like CTRL+1, CTRL+2, etc...
* time to refactor, move the static global variable into a arcnewmutexnew thingy, so that we can make a dynamic pencil/plus/minus in navbar or in buttons to hover edit.
* Challenge accepted - migrate from a json to a rusqlite so that when we decide to EDIT/ADD BUTTONS... it will.. The SQLite database becomes your persistent storage, while the BTreeMap becomes your working/in-memory representation, and have crud endpoints?

* v2.2.x Future if you click the wiki or the cp, it should adjust the title in the browser to say Admindash3 (cp) or (wiki) or whatever the navbarjson had!!! +update the favicon?
* Encrypt and argon2ize the json file secrets. (password hashed)

@@@IMPORTANT@@@ DIFFICULT BUGS
- if you F5 AFTER LOGGING IN AS ADMIN, there is a flicker. Need to figure out what use_server_future or thingy is causing this.
- MAKE THE COMMENTS FIELD a WYSIG to support images for url rows that is allows a long string of text + images attachments (or a separate attachments)
- featreq: add a very subtle highlight saying newly updated on the button as well as the button row on first login, and also on every save of a button or a buttonrow. The highlight should only last 15 seconds, look professional and go away, then only re-appear after logout/login 
- sometimes if you TYPE TOO FAST, the search diddnt actually work or take into the textinput field (rare issue)
- for some reason the _sourcecodes_.tar is 30M, probly due to ~/.git being huge? idk (300+ commits and ai garbage maybe)
- fix the Dockerfile_BuildContainer_from_cwd.sh script so that it actually uses a "rust:latest" instead of Ubuntu, this is so that we no longer need RUST+DIOXUS installed on the node attempting to build the container



@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@

MINOR GOALS:
- 

v2.8.4
- for the Manage Navbar Modal, add a collapsed by default to the entire list by default so that the user can navigate a bit easier when moving stuff they want to move arround (also add a collapse/uncollapse all toggle).  Make it visually easier for the user!!!
- for the Manage Navbar Modal the Pencil Edit is kinda too grey make it same font color as normal black kinda like the clone button to the right of it.
- for the Move/Clone modals like "Move Navbar Item" currently they are Top level, nest under Something or something -- something, can it also be visually collapse/uncollapse toggle button and look easier similar to the Manage Navbar request above. And of course make it visually easier for the user!!! It just seems kinda hard to understand when I click "Move Navbar Item" where it actually will move it to as a end user. (does this make sense?)
- FOR ALL MODALS with text input type fields that are REQUIRED ONLY... make the minimum required fields always be RED input text in ALL button/navbar modals across the entire codebase -- to make it visually easier for the user to know the minimum required text input fields. So that they cant click save or whatever until at minimum the required fields have text or whatever in them.
- for navitems that are submenus with urls underneath them, we should add a ">" or something (please think for something visually easy for users) to indicate "Oh when I click this or mouse over, its a sub-row), or otherwise oh this is a url link"
- for the delete button or delete url or any deletes anywhere please ensure there is a prompt ARE YOU SURE you want to delete, as i noticed the "Delete Button" currently just deletes and returns to main page, when it should first modal prompt are you sure, then when user says ok delete and return to main page etc. Check modals for buttons, and same for navbar modals too...
- BUG: the navbar restores acting funny, it does work I think, BUT you need to control F5 and ONLY THEN will you see the reverted navbar. Can you check the state here.
- the dashboard_backups should now be ./backups/dashboard location instead. Please fix all places in the code so it now saves to this folder instead. Also ensure that the export zip button also is updated to handle this change as well.



















UBER HARD CHANGES -- (NOT IMPLEMENTED YET!!!!!!!!!!!)
- 
- add a tailwind.css with variables i can use, instead of hard coding a bunch of tailwind everywhere? not sure how difficult this is
schema change request...
- make a bin/migrator v4 which will allow a new field in the data_buttons json which has a SITE field which can be OR/FA/SW/OTHER. Thus, every URL must contain a SITE, and a new modal to add/save/change site fields (just like we did with the +Tags should be handled). Thus a migrator script to add the SITE field should be added, and then all of the edit modals should handle this appropriately. The migrator and also the create button urls should default to the OTHER by default. Finally, the url list inside the buttons AND the search results from the start menu should show a tiny breadcrumb (ONLY if the field is NOT OTHER), this way it looks nice in the result screens. Each site result in both should be a slightly different highlight so that the user can quickly identify by looking at the list to scroll.
- migratorv4 also needs... add a description field to the button which is shown only when you open the button list of urls. (this is a json change, so a ./src/bin/migrator is required for this v4 --- see the ./src/bin/migrators source code for this schema change, and FIX EVERYWHERE in the APP to allow this new field).

- maybe a editable tags/rename menu option in the tag modal??????????????
- maybe a IMPORT json option?????????????
- maybe a COMMAND PALLETE CTRL SHIFT P?
- myabe add ICON to the buttonboard?








