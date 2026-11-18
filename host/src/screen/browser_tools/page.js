(function (refs) {
  // The page-side half of Codync's browser tools (host/src/screen/browser_tools/). Each tool call
  // evaluates this factory, binds it to the refs of the document that is open then, and runs one op.
  // Adapted from OpenBot nightly's browser engine (src/backend/browser-cdp.ts, browser-tools.ts): a
  // compact numbered tree, refs that die with the document, and answers in band as {ok:false,error},
  // because Codync's CDP pipe reports a page exception as one generic line without its text.
  "use strict";

  var MAX_ELEMENTS = 400;
  var MAX_VISITED = 4000;
  var MAX_TEXT = 6000;
  var MAX_NAME = 160;
  var MAX_VALUE = 120;
  var MAX_OPTIONS = 20;

  // wait_for's dom-quiet reads this observer's last change, so it starts with the first op of a page.
  if (!window.__codyncMutations) {
    var watched = { at: Date.now(), count: 0 };
    window.__codyncMutations = watched;
    try {
      new MutationObserver(function () {
        watched.at = Date.now();
        watched.count += 1;
      }).observe(document, { childList: true, subtree: true, attributes: true, characterData: true });
    } catch (error) {
      // A document that cannot be watched is never quiet; the other conditions still work.
    }
  }

  function cut(value, max) {
    var text = String(value === undefined || value === null ? "" : value)
      .replace(/\s+/g, " ")
      .trim();
    return text.length > max ? text.slice(0, max) + "…" : text;
  }

  function textOf(node) {
    if (!node) return "";
    var raw = node.innerText === undefined ? node.textContent || "" : node.innerText;
    return cut(raw, MAX_NAME * 4);
  }

  function labelOf(el) {
    var aria = el.getAttribute && el.getAttribute("aria-label");
    if (aria && aria.trim()) return cut(aria, MAX_NAME);
    var by = el.getAttribute && el.getAttribute("aria-labelledby");
    if (by) {
      var named = by
        .split(/\s+/)
        .map(function (id) {
          return document.getElementById(id);
        })
        .filter(Boolean)
        .map(function (node) {
          return textOf(node);
        })
        .join(" ");
      if (named.trim()) return cut(named, MAX_NAME);
    }
    if (el.id) {
      var own = null;
      try {
        own = document.querySelector('label[for="' + CSS.escape(el.id) + '"]');
      } catch (error) {
        own = null;
      }
      if (own && textOf(own)) return cut(textOf(own), MAX_NAME);
    }
    var wrapped = el.closest && el.closest("label");
    if (wrapped && textOf(wrapped)) return cut(textOf(wrapped), MAX_NAME);
    var attributes = ["alt", "placeholder", "title"];
    for (var index = 0; index < attributes.length; index += 1) {
      var value = el.getAttribute && el.getAttribute(attributes[index]);
      if (value && value.trim()) return cut(value, MAX_NAME);
    }
    if (el.tagName === "INPUT" || el.tagName === "TEXTAREA") {
      return cut(el.getAttribute("name") || "", MAX_NAME);
    }
    return cut(textOf(el), MAX_NAME);
  }

  function roleOf(el) {
    var explicit = el.getAttribute && el.getAttribute("role");
    if (explicit) return explicit.toLowerCase().split(/\s+/)[0];
    var tag = el.tagName ? el.tagName.toLowerCase() : "";
    var type = String(el.getAttribute && el.getAttribute("type")).toLowerCase();
    switch (tag) {
      case "a":
      case "area":
        return el.hasAttribute("href") ? "link" : "";
      case "button":
      case "summary":
        return "button";
      case "input":
        if (type === "checkbox" || type === "radio" || type === "range" || type === "file" || type === "color") return type;
        if (type === "submit" || type === "button" || type === "reset" || type === "image") return "button";
        if (type === "hidden") return "";
        if (type === "search") return "searchbox";
        return "textbox";
      case "textarea":
        return "textbox";
      case "select":
        return el.multiple ? "listbox" : "combobox";
      case "option":
        return "option";
      case "h1":
      case "h2":
      case "h3":
      case "h4":
      case "h5":
      case "h6":
        return "heading";
      case "img":
        return "img";
      case "dialog":
        return "dialog";
      case "iframe":
        return "iframe";
      default:
        if (el.isContentEditable) return "textbox";
        if (el.hasAttribute && el.hasAttribute("tabindex")) return "group";
        return "";
    }
  }

  function shown(el) {
    if (!el || !el.isConnected) return false;
    if (el === document.body || el === document.documentElement) return true;
    if (el.hidden) return false;
    if (el.getAttribute && el.getAttribute("aria-hidden") === "true") return false;
    if (typeof el.checkVisibility === "function" && !el.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true })) {
      return false;
    }
    var style = getComputedStyle(el);
    if (style.display === "none" || style.visibility === "hidden" || style.visibility === "collapse") return false;
    var box = el.getBoundingClientRect();
    return box.width > 0 && box.height > 0;
  }

  function valueOf(el) {
    var tag = el.tagName ? el.tagName.toLowerCase() : "";
    if (tag === "select") {
      var picked = [];
      for (var index = 0; index < el.options.length; index += 1) {
        var option = el.options[index];
        if (option.selected) picked.push(cut(option.label || option.value, MAX_VALUE));
      }
      return picked.join(", ");
    }
    if (tag === "input") {
      var type = String(el.type).toLowerCase();
      if (type === "password" || type === "hidden" || type === "file" || type === "checkbox" || type === "radio") return "";
      return cut(el.value, MAX_VALUE);
    }
    if (tag === "textarea") return cut(el.value, MAX_VALUE);
    if (el.isContentEditable) return cut(textOf(el), MAX_VALUE);
    return "";
  }

  function statesOf(el) {
    var states = [];
    if (el.disabled || (el.getAttribute && el.getAttribute("aria-disabled") === "true")) states.push("disabled");
    if (el.checked === true) states.push("checked");
    var expanded = el.getAttribute && el.getAttribute("aria-expanded");
    if (expanded === "true" || expanded === "false") states.push("expanded=" + expanded);
    if (el.tagName === "INPUT" && String(el.type).toLowerCase() === "password") states.push("secret");
    if (el.required) states.push("required");
    return states;
  }

  function lineOf(entry) {
    return entry.role + ' "' + entry.name + '" <' + entry.tag + ">";
  }

  function snapshot() {
    var elements = [];
    var visited = 0;
    var truncated = false;
    refs.clear();
    function walk(el, depth) {
      if (truncated) return;
      visited += 1;
      if (visited > MAX_VISITED) {
        truncated = true;
        return;
      }
      if (!shown(el)) return;
      var role = roleOf(el);
      var next = depth;
      if (role) {
        if (elements.length >= MAX_ELEMENTS) {
          truncated = true;
          return;
        }
        var entry = {
          ref: elements.length + 1,
          depth: depth > 8 ? 8 : depth,
          role: role,
          name: labelOf(el),
          tag: el.tagName ? el.tagName.toLowerCase() : "node",
          value: valueOf(el),
          states: statesOf(el)
        };
        elements.push(entry);
        refs.set(entry.ref, { el: el, label: lineOf(entry) });
        next = depth + 1;
      }
      var children = el.shadowRoot ? el.shadowRoot.children : el.children;
      if (!children) return;
      for (var index = 0; index < children.length && !truncated; index += 1) {
        walk(children[index], next);
      }
    }
    var body = document.body || document.documentElement;
    if (body) walk(body, 0);
    var pageText = body ? cut(body.innerText || body.textContent || "", MAX_TEXT) : "";
    return { ok: true, url: location.href, title: document.title, elements: elements, truncated: truncated, text: pageText };
  }

  function entryOf(request) {
    var number = Number(request && request.ref);
    var entry = refs.get(number);
    if (!entry || !entry.el || !entry.el.isConnected) return null;
    return entry;
  }

  function missing(request) {
    return { ok: false, error: "The page has no element " + Number(request && request.ref) + "; take a new snapshot." };
  }

  function box(request) {
    var entry = entryOf(request);
    if (!entry) return missing(request);
    var el = entry.el;
    if (!shown(el)) {
      return { ok: false, error: "Element " + Number(request.ref) + " is not visible on the page; take a new snapshot." };
    }
    try {
      el.scrollIntoView({ block: "center", inline: "center", behavior: "instant" });
    } catch (error) {
      try {
        el.scrollIntoView();
      } catch (ignored) {
        // An element the page keeps out of view stays where it is.
      }
    }
    var rect = el.getBoundingClientRect();
    var x = Math.round(rect.left + rect.width / 2);
    var y = Math.round(rect.top + rect.height / 2);
    if (x < 0 || y < 0 || x > window.innerWidth || y > window.innerHeight) {
      return {
        ok: false,
        error: "Element " + Number(request.ref) + " is outside the visible page; scroll to it and take a new snapshot."
      };
    }
    return {
      ok: true,
      x: x,
      y: y,
      width: Math.round(rect.width),
      height: Math.round(rect.height),
      label: entry.label
    };
  }

  function center() {
    return { ok: true, x: Math.round(window.innerWidth / 2), y: Math.round(window.innerHeight / 2) };
  }

  function focus(request) {
    var entry = entryOf(request);
    if (!entry) return missing(request);
    var el = entry.el;
    var tag = el.tagName ? el.tagName.toLowerCase() : "";
    var type = String(el.type || "").toLowerCase();
    if (request.typing) {
      var textField =
        (tag === "input" && ["", "text", "search", "email", "url", "tel", "number", "password"].indexOf(type) >= 0) ||
        tag === "textarea" ||
        el.isContentEditable;
      if (!textField) {
        return { ok: false, error: "Element " + Number(request.ref) + " is not a text field; take a new snapshot." };
      }
    }
    try {
      el.focus({ preventScroll: false });
    } catch (error) {
      try {
        el.focus();
      } catch (ignored) {
        // A control the page will not focus is reported below.
      }
    }
    try {
      if (el.isContentEditable) {
        var selection = window.getSelection();
        var range = document.createRange();
        range.selectNodeContents(el);
        if (!request.replace) range.collapse(false);
        selection.removeAllRanges();
        selection.addRange(range);
      } else if (typeof el.setSelectionRange === "function" && typeof el.value === "string") {
        if (request.replace) el.setSelectionRange(0, el.value.length);
        else el.setSelectionRange(el.value.length, el.value.length);
      }
    } catch (error) {
      // A field that keeps no selection takes the text at its caret.
    }
    var active = document.activeElement === el;
    if (!active && document.activeElement && document.activeElement.shadowRoot) {
      active = document.activeElement.shadowRoot.activeElement === el;
    }
    if (!active) {
      return { ok: false, error: "The page would not focus element " + Number(request.ref) + "; take a new snapshot." };
    }
    return { ok: true, label: entry.label, value: valueOf(el) };
  }

  function select(request) {
    var entry = entryOf(request);
    if (!entry) return missing(request);
    var el = entry.el;
    if (el.tagName !== "SELECT") {
      return { ok: false, error: "Element " + Number(request.ref) + " is not a select; take a new snapshot." };
    }
    var wanted = Array.isArray(request.values) ? request.values.map(String) : [];
    if (!wanted.length) return { ok: false, error: "Name at least one option value or label." };
    var picked = [];
    for (var index = 0; index < el.options.length; index += 1) {
      var option = el.options[index];
      var candidates = [option.value, cut(option.label || "", MAX_VALUE), textOf(option)];
      var matches = wanted.some(function (want) {
        return candidates.indexOf(want) >= 0;
      });
      if (matches) {
        option.selected = true;
        picked.push(cut(option.label || option.value, MAX_VALUE));
      }
    }
    if (!picked.length) {
      var offered = [];
      for (var index = 0; index < el.options.length && index < MAX_OPTIONS; index += 1) {
        offered.push(JSON.stringify(cut(el.options[index].label || el.options[index].value, 60)));
      }
      return {
        ok: false,
        error:
          "Element " +
          Number(request.ref) +
          " has no option " +
          JSON.stringify(wanted.join(", ")) +
          ". Options: " +
          offered.join(", ") +
          (el.options.length > MAX_OPTIONS ? ", …" : "")
      };
    }
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
    return { ok: true, selected: picked, label: entry.label };
  }

  function check(request) {
    var entry = entryOf(request);
    if (!entry) return missing(request);
    var el = entry.el;
    var type = String(el.type || "").toLowerCase();
    var role = String(el.getAttribute("role") || "").toLowerCase();
    if (type !== "checkbox" && type !== "radio" && role !== "checkbox" && role !== "switch") {
      return {
        ok: false,
        error: "Element " + Number(request.ref) + " is not a checkbox or radio button; take a new snapshot."
      };
    }
    var wanted = request.checked === true;
    if (el.checked === wanted) return { ok: true, checked: wanted, unchanged: true, label: entry.label };
    el.click();
    if (el.checked !== wanted) {
      el.checked = wanted;
      el.dispatchEvent(new Event("input", { bubbles: true }));
      el.dispatchEvent(new Event("change", { bubbles: true }));
    }
    return { ok: true, checked: el.checked === true, label: entry.label };
  }

  function probe(request) {
    var now = Date.now();
    var watched = window.__codyncMutations || { at: now, count: 0 };
    var hasText = true;
    if (request && typeof request.text === "string" && request.text.length) {
      hasText = false;
      var body = document.body;
      if (body) {
        try {
          hasText = body.innerText.indexOf(request.text) >= 0;
        } catch (error) {
          hasText = (body.textContent || "").indexOf(request.text) >= 0;
        }
      }
    }
    return {
      ok: true,
      readyState: document.readyState,
      url: location.href,
      title: document.title,
      quietMs: now - watched.at,
      mutations: watched.count,
      hasText: hasText
    };
  }

  async function evaluate(request) {
    try {
      var value = (0, eval)(String(request.source));
      if (request.await) value = await value;
      var encoded;
      try {
        encoded = JSON.stringify(value);
      } catch (error) {
        return { ok: false, error: "The value is not JSON-serializable: " + String((error && error.message) || error) };
      }
      return { ok: true, json: encoded === undefined ? "null" : encoded };
    } catch (error) {
      var message = String((error && error.message) || error);
      // A page whose Content Security Policy forbids eval still takes Runtime.evaluate itself.
      if (/Content Security Policy|unsafe-eval|eval\(\)/i.test(message)) {
        return { ok: false, fallback: true, error: message };
      }
      return { ok: false, error: message };
    }
  }

  return {
    snapshot: snapshot,
    box: box,
    center: center,
    focus: focus,
    select: select,
    check: check,
    probe: probe,
    evaluate: evaluate
  };
})
