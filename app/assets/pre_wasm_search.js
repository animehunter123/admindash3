/* Admindash3 pre-WASM progressive search (inline or src).
 * Index element: #ad3-pre-wasm-search-index (prefer <pre>, not <script>).
 * Stops when window.__AD3_WASM_READY is set by Dioxus after WASM mounts.
 */
(function () {
  if (window.__AD3_PRE_WASM_SEARCH_INSTALLED) return;
  window.__AD3_PRE_WASM_SEARCH_INSTALLED = true;

  function ready() {
    return !!window.__AD3_WASM_READY;
  }

  function indexEl() {
    return document.getElementById("ad3-pre-wasm-search-index");
  }

  function index() {
    var el = indexEl();
    if (!el) return [];
    var raw = (el.textContent || el.innerText || "").trim();
    if (!raw) return [];
    try {
      var data = JSON.parse(raw);
      return Array.isArray(data) ? data : [];
    } catch (e) {
      if (!window.__AD3_SEARCH_PARSE_WARNED) {
        window.__AD3_SEARCH_PARSE_WARNED = true;
        console.warn("[ad3 pre-wasm search] index JSON parse failed", e);
      }
      return [];
    }
  }

  // Mirror navbar.rs:
  //   let query = search().trim().to_lowercase();
  //   let normalized_query = query.trim_start_matches('#').to_string();
  function normQuery(raw) {
    var q = String(raw || "").trim().toLowerCase();
    // Rust trim_start_matches('#') strips every leading '#'
    while (q.charAt(0) === "#") q = q.slice(1);
    return q;
  }

  function norm(s) {
    return String(s || "").toLowerCase();
  }

  // Tag token without leading '#', lowercased — same as
  // tag.to_lowercase().trim_start_matches('#')
  function tagKey(tag) {
    var t = String(tag || "").toLowerCase();
    while (t.charAt(0) === "#") t = t.slice(1);
    return t;
  }

  // hashtag_tokens in Rust come from a BTreeSet → sorted lexicographically.
  // Re-sort here so secondary sort keys always match WASM even if JSON order drifts.
  function sortedTags(tags) {
    var copy = (tags || []).slice();
    copy.sort(function (a, b) {
      var aa = String(a), bb = String(b);
      if (aa < bb) return -1;
      if (aa > bb) return 1;
      return 0;
    });
    return copy;
  }

  // Matching (navbar.rs):
  //   Buttonboard link: button name OR row name OR any hashtag token
  //   Navbar item:      item name OR url  (tags unused)
  //   Dividers: never
  // Does NOT search comments or (for Buttonboard) the URL field.
  function filter(rawQuery) {
    var q = normQuery(rawQuery);
    if (!q) return [];
    var data = index();
    var hits = [];
    for (var i = 0; i < data.length; i++) {
      var e = data[i];
      var tags = sortedTags(e.tags);
      var buttonOk = norm(e.button_name).indexOf(q) !== -1;
      var nameOk = norm(e.row_name).indexOf(q) !== -1;
      var matchedTags = [];
      for (var t = 0; t < tags.length; t++) {
        if (tagKey(tags[t]).indexOf(q) !== -1) matchedTags.push(tags[t]);
      }
      var urlOk = false;
      // Only navbar items match on URL (collect_nav_hits in navbar.rs)
      if (e.is_nav_item) {
        urlOk = norm(e.url || "").indexOf(q) !== -1;
      }
      if (!buttonOk && !nameOk && !matchedTags.length && !urlOk) continue;
      hits.push({
        button_name: e.button_name,
        row_name: e.row_name,
        url: e.url || "",
        comments: e.comments || "",
        all_tags: tags,
        matched_tags: matchedTags,
        has_secret: !!e.has_secret,
        is_nav_item: !!e.is_nav_item
      });
    }
    // Sort (navbar.rs):
    //   button_name.to_lowercase()
    //   then all_tags.join(" ").to_lowercase()
    //   then row_name.to_lowercase()
    hits.sort(function (a, b) {
      var ab = norm(a.button_name);
      var bb = norm(b.button_name);
      if (ab < bb) return -1;
      if (ab > bb) return 1;
      var at = (a.all_tags || []).join(" ").toLowerCase();
      var bt = (b.all_tags || []).join(" ").toLowerCase();
      if (at < bt) return -1;
      if (at > bt) return 1;
      var an = norm(a.row_name);
      var bn = norm(b.row_name);
      if (an < bn) return -1;
      if (an > bn) return 1;
      return 0;
    });
    return hits;
  }

  function ensureDropdown(input) {
    var id = "ad3-pre-wasm-results";
    var existing = document.getElementById(id);
    if (existing) return existing;
    var parent = input.parentElement;
    if (!parent) return null;
    try {
      if (window.getComputedStyle(parent).position === "static") {
        parent.style.position = "relative";
      }
    } catch (e) {}
    var div = document.createElement("div");
    div.id = id;
    div.setAttribute("role", "listbox");
    // Same classes as the Dioxus search dropdown in navbar.rs
    div.className =
      "absolute right-0 mt-2 w-[28rem] max-[430px]:fixed max-[430px]:left-2 " +
      "max-[430px]:right-2 max-[430px]:top-16 max-[430px]:mt-0 max-[430px]:w-auto " +
      "bg-white rounded-md shadow-xl border border-gray-200 max-h-[calc(100dvh-100px)] " +
      "overflow-y-auto [scrollbar-gutter:stable] z-50";
    parent.appendChild(div);
    return div;
  }

  function hideDropdown() {
    var el = document.getElementById("ad3-pre-wasm-results");
    if (el) el.remove();
  }

  function esc(s) {
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  // Mirror HighlightedTag in navbar.rs: yellow chip on the matched substring.
  function highlightTagHtml(tag, query) {
    var raw = String(tag || "");
    var without = raw.replace(/^#+/, "");
    var q = normQuery(query);
    var lower = without.toLowerCase();
    var idx = q ? lower.indexOf(q) : -1;
    var pill =
      'class="rounded-full bg-blue-100 px-2 py-1 text-[11px] font-medium text-blue-700"';
    if (idx < 0) {
      return "<span " + pill + ">" + esc(raw) + "</span>";
    }
    var before = without.slice(0, idx);
    var matched = without.slice(idx, idx + q.length);
    var after = without.slice(idx + q.length);
    return (
      "<span " +
      pill +
      ">#" +
      esc(before) +
      '<span class="rounded-sm bg-yellow-300 px-0.5 text-gray-900">' +
      esc(matched) +
      "</span>" +
      esc(after) +
      "</span>"
    );
  }

  function render(input, hits, selected) {
    if (ready()) {
      hideDropdown();
      return;
    }
    if (!hits.length) {
      hideDropdown();
      return;
    }
    var box = ensureDropdown(input);
    if (!box) return;
    var q = input.value || "";
    var html = "";
    for (var i = 0; i < hits.length; i++) {
      var h = hits[i];
      var sel = i === selected;
      // visible_tags: matched only if any matched, else all (navbar.rs)
      var visible =
        h.matched_tags && h.matched_tags.length
          ? h.matched_tags
          : h.all_tags || [];
      var tagHtml = "";
      for (var t = 0; t < visible.length; t++) {
        var tag = visible[t];
        var isMatched =
          h.matched_tags &&
          h.matched_tags.some(function (m) {
            return m === tag;
          });
        if (isMatched) {
          tagHtml += highlightTagHtml(tag, q);
        } else {
          tagHtml +=
            '<span class="rounded-full bg-blue-100 px-2 py-1 text-[11px] font-medium text-blue-700">' +
            esc(tag) +
            "</span>";
        }
      }
      var comments = "";
      if (h.comments && String(h.comments).trim()) {
        comments =
          '<div class="mt-1 truncate text-xs text-gray-500" title="' +
          esc(h.comments) +
          '">' +
          esc(h.comments) +
          "</div>";
      }
      // Non-admin SSR: never expose secret text — same "Secret hidden" chip as WASM
      var secret = "";
      if (h.has_secret) {
        secret =
          '<div class="mt-2 text-xs text-gray-700">' +
          '<span class="inline-flex items-center gap-2 rounded-md bg-amber-50 px-2 py-1 text-amber-700">' +
          "<span>🔑</span><span>Secret hidden</span>" +
          "</span></div>";
      }
      var href = h.url ? esc(h.url) : "#";
      var rowClass = sel
        ? "block w-full border-b border-gray-100 bg-blue-50 px-4 py-3 pr-12 text-left text-blue-800"
        : "block w-full border-b border-gray-100 px-4 py-3 pr-12 text-left text-gray-800 hover:bg-blue-50 hover:text-blue-800";
      html +=
        '<a href="' +
        href +
        '" target="_blank" rel="noopener noreferrer" class="' +
        rowClass +
        '">' +
        '<div class="font-medium break-words">' +
        esc(h.row_name) +
        "</div>" +
        '<div class="mt-1 text-xs text-gray-500">' +
        esc(h.button_name) +
        "</div>" +
        comments +
        (tagHtml
          ? '<div class="mt-2 flex flex-wrap gap-2">' + tagHtml + "</div>"
          : "") +
        secret +
        "</a>";
    }
    box.innerHTML = html;
  }

  var selected = 0;
  var lastHits = [];
  var warnedEmpty = false;

  function handleQuery(input) {
    if (ready() || !input) return;
    var q = input.value || "";
    if (!norm(q)) {
      hideDropdown();
      lastHits = [];
      return;
    }
    if (!indexEl() && !warnedEmpty) {
      warnedEmpty = true;
      console.warn(
        "[ad3 pre-wasm search] #ad3-pre-wasm-search-index not in DOM yet — will retry on next keystroke"
      );
    }
    var hits = filter(q);
    if (!hits.length && indexEl() && index().length === 0 && !warnedEmpty) {
      warnedEmpty = true;
      console.warn("[ad3 pre-wasm search] index element present but empty array");
    }
    lastHits = hits;
    selected = 0;
    render(input, hits, selected);
  }

  function onInput(ev) {
    if (ready()) return;
    var input = ev.target;
    if (!input || input.id !== "autocomplete_field") return;
    handleQuery(input);
  }

  function onKey(ev) {
    if (ready()) return;
    var input = ev.target;
    if (!input || input.id !== "autocomplete_field") return;
    if (ev.key === "ArrowDown" && lastHits.length) {
      ev.preventDefault();
      selected = (selected + 1) % lastHits.length;
      render(input, lastHits, selected);
    } else if (ev.key === "ArrowUp" && lastHits.length) {
      ev.preventDefault();
      selected = (selected - 1 + lastHits.length) % lastHits.length;
      render(input, lastHits, selected);
    } else if (ev.key === "Enter" && lastHits.length) {
      var hit = lastHits[selected];
      if (hit && hit.url) {
        ev.preventDefault();
        window.open(hit.url, "_blank", "noopener,noreferrer");
      }
    } else if (ev.key === "Escape") {
      hideDropdown();
      lastHits = [];
    } else {
      // keyup path for engines that lag input events
      handleQuery(input);
    }
  }

  function onDocClick(ev) {
    if (ready()) return;
    var t = ev.target;
    if (!t) return;
    if (t.id === "autocomplete_field") return;
    if (t.closest && t.closest("#ad3-pre-wasm-results")) return;
    hideDropdown();
  }

  document.addEventListener("input", onInput, true);
  document.addEventListener("keyup", onKey, true);
  document.addEventListener("keydown", onKey, true);
  document.addEventListener("click", onDocClick, true);

  function tryInitial() {
    if (ready()) return;
    var input = document.getElementById("autocomplete_field");
    if (input && input.value) handleQuery(input);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", tryInitial);
  } else {
    tryInitial();
  }
  var tries = 0;
  var timer = setInterval(function () {
    if (ready() || ++tries > 50) {
      clearInterval(timer);
      return;
    }
    tryInitial();
  }, 100);

  console.info("[ad3 pre-wasm search] listeners installed");
})();

