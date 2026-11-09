# Admindash3
Re-written in rust because "Rust is sensible".

# How to launch
* Requires a Linux Host Atm. Install git, rust, and dioxus +wasm-toolchain via ```curl -sSL https://dioxus.dev/install.sh | bash``` 
* Edit AND Chmod755 your data_buttons.*.json and data_navbar.*.json so that the docker service can read and write to them. For example, ```chmod 777 data_buttons.*.json ; chmod 777 data_navbar.*.json```
* Serve non-docker debug version via:   ```cd app; dx serve --port 9999 --addr 0.0.0.0```
* Serve non-docker release version via: ```cd app; dx bundle --web --release; cd /target/dx/admindash3/release/web/public ; IP=0.0.0.0 PORT=9999 ../server```
* Serve docker version via (host compile required): ```cd app; bash ./Dockerfile_BuildContainer_from_cwd.sh && docker compose up -d```
* Now use the page as a "Bookmarks Manager" and login modify the navbar/buttons, or test out the wasm highspeed search (when you search something and it will now sort result list by BUTTON NAME > HASHTAGS > URLROWNAME which is really awesome).

# What I learned / still learning...
So basically I wanted to... Load various ./data_*.json files for navitems or booksmarks containing buttons and plot on a webpage. Eventually load from postgres or sqlite3 tables, with history/undo enabled.

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
                         │  data_buttons*json   │ (or sql db)
                         │          │           │
                         │          ▼           │
                         │      load_json()     │
                         │                      │
                         │  data_navbar*json    │ (or sql db)
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
* STRETCHGOAL: Do the whole thing for nginx proxy (for http://cp/) + cp + rshell + kanban to merge in this Dioxus. (+bonus for cp/ nginx)
* STRETCHGOAL: Make navbar and buttons hover to edit (css use hover).
* STRETCHGOAL: Add support ssh:// and rdp:// orwhatever it officially is to the links (with a gpo later or whatever).
MAJOR V GOALS: v3, v4, v5:
- refactor code so that there is a "./src/buttonboard, navbar, cp, kanban, rshell" modules.
* Hydration SSR of HTML > WASM works, but the addressbar changes for a sec (i dont mind it, but it violates W3C and WCAG!)
* Add env_logger/log and replace all println() with debug!().
* Fix the css grid when MANY>4 buttons are there, the last button's row isnt centered (it is currently 2,3,5 cols tailwind)
* CSS make the navbar>navitems highlight which is selected (bigger area to click to make it easier to click!!) (underline the navitem for example). Also... Would be cool if you switch to CP/ it underlines it (WITH A KEYBOARD SHORTCUT TOO specific to CP and WIKI for example so that i dont have to click between 'em ?), like CTRL+1, CTRL+2, etc...
* Refactor, move the static global variable into a arcnewmutexnew thingy, so that we can make a dynamic pencil/plus/minus in navbar or in buttons to hover edit.
* DB Challenge accepted - migrate from a json to a rusqlite so that when we decide to EDIT/ADD BUTTONS... it will.. The SQLite database becomes your persistent storage, while the BTreeMap becomes your working/in-memory representation, and have crud endpoints?
* Future if you click the wiki or the cp, it should adjust the title in the browser to say Admindash3 (cp) or (wiki) or whatever the navbarjson had!!! +update the favicon?
* Argon2ize the json file secrets. (password hashed)
* Make a jpeg in the git repo, and then add to README.md (confirm that forg/gh actually work with it).

@@@IMPORTANT@@@ DIFFICULT BUGS
- If you F5 AFTER LOGGING IN AS ADMIN, there is a flicker. Need to figure out what use_server_future or thingy is causing this.
- MAKE all "Input Text" FIELD a WYSIG to support images for url rows that is allows a long string of text + images attachments (or a separate attachments)
- Featreq: add a very subtle highlight saying newly updated on the button as well as the button row on first login, and also on every save of a button or a buttonrow. The highlight should only last 15 seconds, look professional and go away, then only re-appear after logout/login 
- Sometimes if you TYPE TOO FAST, the search didnt actually work or take into the textinput field (rare issue due to hydration)
- For some reason the git repo atm / _sourcecodes_.tar is 30M, probly due to ~/.git being huge? idk (300+ commits and ai garbage maybe)
- Fix the Dockerfile_BuildContainer_from_cwd.sh script so that it actually uses a "rust:latest" instead of Ubuntu, this is so that we no longer need RUST+DIOXUS installed on the node attempting to build the container (This is actually very important so that we dont need a linux host with rust/wasm/dx!!!)



@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@
@@@@@@@@@@@@@@@@@@@@@@@@@

MINOR GOALS:
- 

v2.8.14

- Edit Navbar Item >> needs a Delete Button in the modal (similar to how other modals are, please follow the same process) and look at buttons rs and navbar rs code.

- the dashboard_backups should now be ./backups/dashboard location instead. Please fix all places in the code so it now saves to this folder instead. Also ensure that the export zip button also is updated to handle this change as well.

- The search results dropdownlist has the first highlighted but when you move your mouse to the next one it is highlighted (and the first one stays highlighted so there are 2 in highlighted blue instead of 1, can this be fixed?)... I guess in other words... ype a letter and it is highlighted but then mouse and it highlights another item and doesnt unhighlight the first



















UBER HARD CHANGES -- (NOT IMPLEMENTED YET!!!!!!!!!!!)
- 
- add a tailwind.css with variables i can use, instead of hard coding a bunch of tailwind everywhere? not sure how difficult this is
_+Schema change request (yet again!)..._
* Schema add URL>SITE: make a bin/migrator v4 which will allow a new field in the data_buttons json which has a SITE field which can be OR/FA/SW/OTHER. Thus, every URL must contain a SITE, and a new modal to add/save/change site fields (just like we did with the +Tags should be handled). Thus a migrator script to add the SITE field should be added, and then all of the edit modals should handle this appropriately. The migrator and also the create button urls should default to the OTHER by default. Finally, the url list inside the buttons AND the search results from the start menu should show a tiny breadcrumb (ONLY if the field is NOT OTHER), this way it looks nice in the result screens. Each site result in both should be a slightly different highlight so that the user can quickly identify by looking at the list to scroll.
* Schema add BUTTON>DESCRIPTION: make a migrator v4 which... add a description field to the button which is shown only when you open the button list of urls. (this is a json change, so a ./src/bin/migrator is required for this v4 --- see the ./src/bin/migrators source code for this schema change, and FIX EVERYWHERE in the APP to allow this new field).
- Schema add BUTTON>BUTTONICON (HTTP LINK TO ico?): maybe add ICON to the buttonboard? (Schema change yet again... ugh)

- maybe a editable tags/rename menu option in the tag modal??????????????
- maybe a IMPORT json option????????????? (This is actually going to to be the final step so that no json server backups are the 3-2-1 backup)
- maybe a COMMAND PALLETE CTRL SHIFT P? (idk, see vscode)


finishedthing i think...
- the footer seems to have a white bar above it, maybe like a margin-top with calc(var(--spacing) * 8) or something im not sure, why this is appearing.
something else i had typed before the wasm was mounted instead of scanning what the input field currently has immediately and then showing the searchdropdown
- when i search for something, then login, it logs in but the search dropdown doesnt appear and i have to start searching again... fix this so it stays open behing the login modal and if you sign in or close the sign in modal the search result list is shown
- move the green admin to the left of the search box so that after logging in it doesnt shift the search results over. >> moved but the admin card looks funny


next thing i want...
*when you do pre wasm search, make sure it looks exactly the same as when it was mounted so the user doesnt see any difference in styling or flickering *hashtags, and highlighting are missing

