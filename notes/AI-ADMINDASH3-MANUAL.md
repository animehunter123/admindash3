Heres a nice AND ACCURATE summary of everything in this app (i even added the recent tag stuff in here too, worth a read.)

# GUI Features & User Guide

Admindash3 is designed to make frequently used administration links and tools easy to find and launch from one simple dashboard.

The dashboard is intentionally designed to be fast and easy to use, while providing additional management features when an administrator signs in.

---

## 🏠 Dashboard

The main dashboard displays your configured button groups and links.

Each group contains buttons that can be used to open the configured URL.

Clicking a button normally opens the destination in a new browser tab.

The dashboard can contain both:

* 🔗 **URL buttons** — links to websites, applications, tools, documentation, etc.
* ➖ **Dividers** — visual separators that help organize large groups of buttons.

---

## 🔎 Quick Search

The dashboard includes a quick-search box for finding buttons without having to browse through every group.

Search matches against:

* Button names
* Hashtags/tags assigned to the button

For example, a button named:

```text
Rust Documentation
```

can be found by searching:

```text
rust
```

If the button has tags such as:

```text
#rust #documentation #programming
```

you can also search for:

```text
programming
```

or:

```text
#programming
```

The search box is intentionally forgiving. Users do not need to remember whether a tag was written with `#` or without it.

### ⌨️ Search Keyboard Shortcuts

The search box supports keyboard navigation:

| Key           | Action                           |
| ------------- | -------------------------------- |
| `/`           | Focus the search box             |
| `Ctrl + K`    | Focus and select the search text |
| `↑` / `↓`     | Move through search results      |
| `Tab`         | Move to the next result          |
| `Shift + Tab` | Move to the previous result      |
| `Enter`       | Open the selected result         |
| `Esc`         | Close the search results         |

Search navigation wraps around, so moving past the last result returns to the first result.

---

## 🌐 Embedded Pages

Some dashboard links can be displayed directly inside the dashboard using the embedded page view.

When a page is opened in the embedded view, the dashboard displays it inside the application rather than immediately leaving the dashboard.

The quick-search interface is hidden while an embedded page is active to keep the workspace uncluttered.

---

# 🔐 Administrator Features

Additional editing features become available after an administrator signs in.

Normal users can use the dashboard without seeing the administrative editing controls.

Once authenticated, administrators can manage buttons, URLs, tags, and ordering directly from the GUI.

---

## ➕ Add a Button

Administrators will see a:

**`+ Button`**

control in the dashboard toolbar.

This opens the button editor where a new button group can be created.

A button group can contain multiple URL entries and dividers.

---

## ✏️ Edit Buttons and URLs

Administrators can use the pencil **✎** controls to edit existing dashboard entries.

Depending on what is being edited, the administrator can modify things such as:

* Button/group names
* URL names
* URLs
* Hashtags/tags
* Other configured URL information

Changes are saved through the backend and the dashboard is refreshed automatically.

---

## ➕ Add URLs and Dividers

Inside a button group, administrators can add additional rows.

Rows can be:

* 🔗 URLs
* ➖ Dividers

Dividers are useful for visually separating related tools inside a large button group.

For example:

```text
Proxmox
PBS
TrueNAS
────────────
Docker
Portainer
Grafana
────────────
Documentation
```

This makes large collections of administration tools much easier to scan.

---

# 🏷️ Tags

Tags provide another way to organize and find dashboard buttons.

Tags are stored separately from the main button configuration, allowing the dashboard to maintain a managed list of available tags.

Tags are displayed using the familiar `#tag` format.

For example:

```text
#rust
#docker
#proxmox
#networking
```

---

## 🏷️ Tag Manager

Administrators have access to the:

**`Tags`**

management button.

The Tag Manager provides a simple interface for maintaining the dashboard's available tag list.

Administrators can:

* Add new tags
* Remove existing tags
* Review the current sorted tag list
* Save the changes

Tags are automatically normalized so they are stored consistently.

For example, these inputs:

```text
rust
#rust
##rust
```

are normalized to:

```text
#rust
```

Tags must contain at least **4 characters**.

---

## 💡 Tag Suggestions

When adding or editing a URL, the hashtag field provides suggestions from the dashboard's available tags.

You can either type a tag manually or select one of the suggested tags.

Suggested tags can also be added using the:

**`+ #tag`**

buttons displayed near the tag field.

This makes it much quicker to apply commonly used tags without having to remember their exact spelling.

The system also prevents the same tag from being accidentally added multiple times.

---

## 🔍 Tags and Search

Tags are not just organizational labels—they are also part of the dashboard's search system.

For example, suppose a button is named:

```text
Docker Portainer
```

and has:

```text
#docker #containers #management
```

Searching for:

```text
containers
```

can find the button even though the word `containers` does not appear in the button name.

Matching tags are highlighted in the search results to make it clear why the result was found.

---

# ↕️ Drag & Drop Ordering

Administrators can reorder URLs and dividers directly inside a button group's editor.

Rows can be dragged and dropped into a new position.

For example:

```text
01  Proxmox
02  PBS
03  TrueNAS
04  ─────────
05  Docker
```

can be rearranged by dragging a row to another position.

The new order is saved by the backend, so the arrangement remains after the dashboard is refreshed.

Both URL rows and divider rows can be moved.

This makes reorganizing a large button group much easier than manually editing configuration files.

---

# 🔒 Administrator vs Normal User

The dashboard has two basic user experiences.

### Normal User

Normal users can:

* View button groups
* Open URLs
* Search for buttons
* Search by tags
* Use keyboard navigation
* Use embedded pages where configured

Administrative editing controls are not displayed.

### Administrator

Administrators have everything available to normal users, plus:

* Add button groups
* Edit buttons
* Edit URLs
* Add URLs
* Add dividers
* Reorder rows with drag & drop
* Manage available tags
* Add tags to URLs
* Remove managed tags

This keeps the normal dashboard clean while still providing a complete management interface when needed.

---

# ⚡ Designed for Fast Interaction

The dashboard is designed so that normal navigation and searching feel immediate.

Search results are handled locally once the dashboard data has loaded, while administrative changes are sent to the backend and the dashboard is refreshed after successful changes.

The result is a dashboard that can be used like a normal bookmark/start page while still providing a full administration interface when needed.

---

## 🧭 Typical Workflow

A typical user might use Admindash3 like this:

1. Open the dashboard.
2. Press `/` or `Ctrl + K`.
3. Search for a tool such as `proxmox`, `docker`, or `backup`.
4. Use `↑` / `↓` to select the desired result.
5. Press `Enter`.
6. The selected application opens.

An administrator can additionally:

1. Sign in.
2. Create or edit a button group.
3. Add URLs and dividers.
4. Assign useful tags.
5. Reorder the rows with drag & drop.
6. Manage the available tag list.
7. Save the changes.
8. Continue using the dashboard immediately.

The goal is simple: **keep the everyday interface clean and fast, while putting the complicated administration work behind the administrator controls.**
