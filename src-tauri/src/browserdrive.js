// The fixed scripts of Escouade's driving (ESC-679, AB-7; docs/AGENT-BROWSERS.md § 3.3, § 6, § 9): compiled in the
// app, run in the isolated world `escouade-agent` of a browser's main frame, never in the page's world. The core calls
// this library with its arguments in JSON (values, never code) and reads back one JSON string. No script of an agent
// is ever run.
//
// What it does in the page: find an element (by a selector, a visible text, or a selector the core made from a key of a
// template), say what kind it is (a password, a file, a download, a field of a form that goes to another origin: the
// core refuses those), scroll, put a highlight, fill a field or choose an option (under WebKit, where the core has no
// real input), click (under WebKit: a synthetic click), and count the user's own gestures (`isTrusted`, not the
// agent's) for the rule of the hand. Nothing of the page's text goes back but what the core asks: « found » or not,
// a box, a kind, the address and the title.
(function (args) {
  'use strict';
  var STATE = '__escouadeDrive';
  var MARK = 'escouade-drive-highlight';
  var GESTURES = 'escouade-drive-gestures';

  // The state of this world in this document: the user's gestures and the agent's window. Kept from one call to the
  // next under WebView2 alone: under WebKit the world is new at each call (ESC-692), so the gestures come from the
  // page's relay (`browserdrive-gestures.js`) and nothing else here waits for a later call.
  function state() {
    var s = window[STATE];
    if (s) return s;
    s = { gestures: 0, agentUntil: 0 };
    Object.defineProperty(window, STATE, { value: s });
    var heard = function (e) {
      if (!e.isTrusted) return;
      if (performance.now() < s.agentUntil) return;
      s.gestures++;
    };
    ['pointerdown', 'wheel', 'keydown', 'touchstart'].forEach(function (t) {
      window.addEventListener(t, heard, { capture: true, passive: true });
    });
    return s;
  }

  function visible(el) {
    var r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return false;
    var st = getComputedStyle(el);
    return st.visibility !== 'hidden' && st.display !== 'none';
  }

  function norm(t) {
    return String(t || '')
      .replace(/\s+/g, ' ')
      .trim()
      .toLowerCase();
  }

  var CLICKABLE = 'a[href],button,[role="button"],[role="tab"],[role="link"],[role="menuitem"],[role="option"],input[type="submit"],input[type="button"],input[type="reset"],summary,label,select,[onclick],[tabindex]';

  // The element a target names: the first visible one.
  function find(t) {
    if (!t) return { error: 'target' };
    var list;
    if (t.text != null) {
      var want = norm(t.text);
      if (!want) return { error: 'target' };
      var all = Array.prototype.slice.call(document.querySelectorAll(CLICKABLE));
      var exact = null;
      var part = null;
      for (var i = 0; i < all.length; i++) {
        var el = all[i];
        if (!visible(el)) continue;
        var txt = norm(el.innerText || el.value || el.getAttribute('aria-label') || '');
        if (txt === want) {
          exact = el;
          break;
        }
        if (!part && txt.indexOf(want) >= 0) part = el;
      }
      return { el: exact || part };
    }
    try {
      list = document.querySelectorAll(t.css);
    } catch (e) {
      return { error: 'selector' };
    }
    for (var j = 0; j < list.length; j++) if (visible(list[j])) return { el: list[j] };
    return { el: null, hidden: list.length > 0 };
  }

  function sameOrigin(url) {
    try {
      return new URL(url, location.href).origin === location.origin;
    } catch (e) {
      return false;
    }
  }

  // What the core needs to know of an element before it acts on it: never its value nor its text.
  function kind(el) {
    var tag = (el.localName || '').toLowerCase();
    var type = tag === 'input' ? (el.getAttribute('type') || 'text').toLowerCase() : '';
    var link = el.closest ? el.closest('a') : null;
    var label = tag === 'label' ? el.control : null;
    var form = el.form || (el.closest ? el.closest('form') : null);
    var crossForm = false;
    if (form) {
      var action = form.getAttribute('action');
      crossForm = action != null && action !== '' && !sameOrigin(action);
      // A button that sends elsewhere than its form.
      var fa = el.getAttribute && el.getAttribute('formaction');
      if (fa && !sameOrigin(fa)) crossForm = true;
    }
    var download = false;
    if (link) {
      var href = link.getAttribute('href') || '';
      download = link.hasAttribute('download') || /^\s*(blob|data|filesystem):/i.test(href);
    }
    var text = ['text', 'search', 'email', 'url', 'tel', 'number', 'password'];
    var editable = (tag === 'input' && text.indexOf(type) >= 0) || tag === 'textarea' || !!el.isContentEditable;
    var role = (el.getAttribute && el.getAttribute('role')) || '';
    var tabEl = el.closest ? el.closest('[role="tab"]') : null;
    return {
      tag: tag,
      type: type,
      password: type === 'password' || !!(label && label.type === 'password'),
      file: type === 'file' || !!(label && label.type === 'file'),
      download: download,
      crossForm: crossForm,
      editable: editable && !el.disabled && !el.readOnly,
      select: tag === 'select',
      tab: role === 'tab' || !!tabEl,
    };
  }

  function box(el) {
    var r = el.getBoundingClientRect();
    return { x: r.x, y: r.y, width: r.width, height: r.height };
  }

  function inView(r) {
    return r.y >= 0 && r.x >= 0 && r.y + r.height <= innerHeight && r.x + r.width <= innerWidth;
  }

  function reduced() {
    return matchMedia && matchMedia('(prefers-reduced-motion: reduce)').matches;
  }

  function where() {
    return { url: location.href, title: document.title, scrollY: Math.round(scrollY), innerWidth: innerWidth, ready: document.readyState };
  }

  // The element, made visible for a gesture: its box in the view after it.
  function locate(a) {
    var f = find(a.target);
    if (f.error) return { error: f.error };
    if (!f.el) return { found: false, hidden: !!f.hidden };
    var el = f.el;
    if (a.reveal && !inView(box(el))) el.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'auto' });
    return { found: true, kind: kind(el), box: box(el) };
  }

  // The `<iframe>` a target names in this document (ESC-682, AB-11), made visible: its address (the `src` the
  // document gave it, resolved) and its rank among the document's iframes of that origin, for the core to tell its
  // frame; never anything of the frame's content.
  function frame(a) {
    var f = find(a.target);
    if (f.error) return { error: f.error };
    if (!f.el) return { found: false, hidden: !!f.hidden };
    var el = f.el;
    var tag = (el.localName || '').toLowerCase();
    if (tag !== 'iframe' && tag !== 'frame') return { found: true, frame: false };
    if (a.reveal && !inView(box(el))) el.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'auto' });
    var origin = function (u) {
      try {
        return new URL(u, location.href).origin;
      } catch (e) {
        return '';
      }
    };
    var src = '';
    try {
      src = new URL(el.getAttribute('src') || 'about:blank', location.href).href;
    } catch (e) {}
    var rank = 0;
    var all = document.querySelectorAll('iframe,frame');
    for (var i = 0; i < all.length && all[i] !== el; i++) {
      if (origin(all[i].getAttribute('src') || 'about:blank') === origin(src)) rank++;
    }
    return { found: true, frame: true, src: src, rank: rank, box: box(el) };
  }

  function scrollTo(a) {
    var f = find(a.target);
    if (f.error) return { error: f.error };
    if (!f.el) return { found: false, hidden: !!f.hidden };
    f.el.scrollIntoView({ block: 'center', inline: 'nearest', behavior: reduced() ? 'auto' : 'smooth' });
    return { found: true };
  }

  function scrollBy(a) {
    window.scrollBy({ top: a.by, left: 0, behavior: reduced() ? 'auto' : 'smooth' });
    return { done: true };
  }

  // Escouade's contours in the document, found by their mark: nothing is kept in this world from one call to the next
  // (under WebKit it is a new world each time, ESC-692).
  function clearMarks() {
    var old = document.querySelectorAll('[data-escouade="' + MARK + '"]');
    for (var i = 0; i < old.length; i++) old[i].parentNode.removeChild(old[i]);
  }

  // A contour at the project's colour, with its label, over the element: one at a time. Escouade's own element, placed
  // in the document's coordinates so that it scrolls with the page without a listener (the page sees it in its DOM;
  // it learns nothing from it).
  function highlight(a) {
    var f = find(a.target);
    if (f.error) return { error: f.error };
    if (!f.el) return { found: false, hidden: !!f.hidden };
    clearMarks();
    var el = f.el;
    if (!inView(box(el))) el.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'auto' });
    var r = el.getBoundingClientRect();
    var x = r.x + scrollX;
    var y = r.y + scrollY;
    var host = document.createElement('div');
    host.setAttribute('data-escouade', MARK);
    host.style.cssText = 'all:initial;position:absolute;pointer-events:none;z-index:2147483647;left:0;top:0;width:0;height:0';
    var root = host.attachShadow ? host.attachShadow({ mode: 'closed' }) : host;
    var color = /^#[0-9a-fA-F]{6}$/.test(a.color || '') ? a.color : '#7c6cf0';
    var ring = document.createElement('div');
    ring.style.cssText =
      'position:absolute;box-sizing:border-box;border:2px solid ' +
      color +
      ';border-radius:6px;box-shadow:0 0 0 4px ' +
      color +
      '33;left:' +
      (x - 4) +
      'px;top:' +
      (y - 4) +
      'px;width:' +
      (r.width + 8) +
      'px;height:' +
      (r.height + 8) +
      'px';
    root.appendChild(ring);
    if (a.label) {
      var tag = document.createElement('div');
      tag.textContent = a.label;
      tag.style.cssText =
        'position:absolute;font:600 12px/1.4 system-ui,sans-serif;color:#fff;background:' +
        color +
        ';padding:2px 8px;border-radius:4px;white-space:nowrap;max-width:60vw;overflow:hidden;text-overflow:ellipsis;left:' +
        Math.max(4, x - 4) +
        'px;top:' +
        (r.y > 28 ? y - 26 : y + r.height + 6) +
        'px';
      root.appendChild(tag);
    }
    document.documentElement.appendChild(host);
    return { found: true };
  }

  function clear() {
    clearMarks();
    return { done: true };
  }

  // The native setter of a field's value: an app that controls its fields (React) hears it.
  function setValue(el, value) {
    var proto = el.localName === 'textarea' ? HTMLTextAreaElement.prototype : el.localName === 'select' ? HTMLSelectElement.prototype : HTMLInputElement.prototype;
    var desc = Object.getOwnPropertyDescriptor(proto, 'value');
    if (desc && desc.set) desc.set.call(el, value);
    else el.value = value;
  }

  // Focuses a field and selects what it holds, for the real typing that follows (WebView2: `Input.insertText`).
  function focus(a) {
    var f = find(a.target);
    if (f.error) return { error: f.error };
    if (!f.el) return { found: false };
    var el = f.el;
    el.focus();
    if (el.isContentEditable) {
      var range = document.createRange();
      range.selectNodeContents(el);
      var sel = getSelection();
      sel.removeAllRanges();
      sel.addRange(range);
    } else if (typeof el.select === 'function') {
      try {
        el.select();
      } catch (e) {}
    }
    return { found: true, focused: document.activeElement === el };
  }

  // WebKit: the text put in by the native setter, and the events an app listens to.
  function fill(a) {
    var f = find(a.target);
    if (f.error) return { error: f.error };
    if (!f.el) return { found: false };
    var el = f.el;
    el.focus();
    if (el.isContentEditable) {
      el.textContent = a.text;
    } else {
      setValue(el, a.text);
    }
    el.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText', data: a.text }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    return { found: true };
  }

  function choose(a) {
    var f = find(a.target);
    if (f.error) return { error: f.error };
    if (!f.el) return { found: false };
    var el = f.el;
    var want = String(a.value);
    var at = -1;
    for (var i = 0; i < el.options.length; i++) {
      var o = el.options[i];
      if (o.value === want) {
        at = i;
        break;
      }
      if (at < 0 && norm(o.label || o.text) === norm(want)) at = i;
    }
    if (at < 0) return { found: true, option: false };
    if (el.options[at].disabled) return { found: true, option: false };
    el.focus();
    setValue(el, el.options[at].value);
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    return { found: true, option: true };
  }

  // WebKit: a synthetic click (`isTrusted` false: a page that wants the user's own ignores it).
  function click(a) {
    var f = find(a.target);
    if (f.error) return { error: f.error };
    if (!f.el) return { found: false };
    if (typeof f.el.focus === 'function') f.el.focus();
    f.el.click();
    return { found: true };
  }

  // WebKit: a key in the focused element, synthetic.
  function key(a) {
    var el = document.activeElement || document.body;
    var init = { key: a.key, code: a.code, bubbles: true, cancelable: true };
    var go = el.dispatchEvent(new KeyboardEvent('keydown', init));
    el.dispatchEvent(new KeyboardEvent('keyup', init));
    if (go && a.key === 'Enter' && el.form && typeof el.form.requestSubmit === 'function') el.form.requestSubmit();
    return { done: true };
  }

  // The kind of the focused element, before a key goes to it (Enter in a field of a form that goes elsewhere).
  function focused() {
    var el = document.activeElement;
    if (!el || el === document.body) return { kind: null };
    return { kind: kind(el) };
  }

  function history(a) {
    if (a.dir === 'back') window.history.back();
    else window.history.forward();
    return { done: true };
  }

  // The user's gestures since the last time, counted while no gesture of the agent's was under way.
  function gestures() {
    var s = state();
    var n = s.gestures;
    s.gestures = 0;
    var relayed = 0;
    var take = function (e) {
      var k = parseInt(e.detail, 10);
      if (k > 0 && k < 100000) relayed += k;
    };
    window.addEventListener(GESTURES, take, true);
    try {
      window.dispatchEvent(new CustomEvent(GESTURES + '-pull'));
    } finally {
      window.removeEventListener(GESTURES, take, true);
    }
    return { gestures: n + relayed };
  }

  // A gesture of the agent's under way for `ms`: the events it makes are not the user's.
  function arm(a) {
    var s = state();
    s.agentUntil = performance.now() + Math.max(0, Math.min(5000, a.ms | 0));
    return { done: true };
  }

  var result;
  try {
    state();
    var run = args.run;
    if (run === 'locate') result = locate(args);
    else if (run === 'frame') result = frame(args);
    else if (run === 'scroll_to') result = scrollTo(args);
    else if (run === 'scroll') result = scrollBy(args);
    else if (run === 'highlight') result = highlight(args);
    else if (run === 'clear') result = clear();
    else if (run === 'focus') result = focus(args);
    else if (run === 'fill') result = fill(args);
    else if (run === 'choose') result = choose(args);
    else if (run === 'click') result = click(args);
    else if (run === 'key') result = key(args);
    else if (run === 'focused') result = focused();
    else if (run === 'history') result = history(args);
    else if (run === 'gestures') result = gestures();
    else if (run === 'arm') result = arm(args);
    else if (run === 'where') result = {};
    else result = { error: 'unknown' };
    if (result && !result.error) {
      var w = where();
      for (var k in w) result[k] = w[k];
    }
  } catch (e) {
    result = { error: 'threw', message: String((e && e.message) || e).slice(0, 200) };
  }
  return JSON.stringify(result);
})
