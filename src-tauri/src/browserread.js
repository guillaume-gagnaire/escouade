// The fixed scripts of Escouade's read bridge (ESC-668, docs/AGENT-BROWSERS.md § 6): compiled in the
// app, run in the isolated world `escouade-agent` of a browser's main frame, never in the page's
// world (which lies: it can rewrite `JSON.stringify` or `querySelector` there). The core calls
// `__escouadeRun(<arguments in JSON>)`, and reads back one JSON string. No script of an agent is
// ever run: the arguments are values, never code.
//
// What comes back is the page's content, which Escouade does not trust; the core masks it again
// (`browserread::mask_html`) and marks it as third-party data.
(function (args) {
  'use strict';
  var COLLECTOR = '__escouadeConsole';
  var EVENT = 'escouade-console-relay';

  function elements(selector) {
    if (!selector) return [document.documentElement];
    try {
      return Array.prototype.slice.call(document.querySelectorAll(selector));
    } catch (e) {
      return null;
    }
  }

  // A visible element: laid out, not hidden by its style.
  function visible(el) {
    var r = el.getBoundingClientRect();
    if (r.width <= 0 || r.height <= 0) return false;
    var s = getComputedStyle(el);
    return s.visibility !== 'hidden' && s.display !== 'none';
  }

  // The fields whose value never leaves (the core masks the serialized HTML again).
  function maskClone(root) {
    var masked = 0;
    var all = [root].concat(Array.prototype.slice.call(root.querySelectorAll('*')));
    for (var i = 0; i < all.length; i++) {
      var el = all[i];
      var tag = (el.localName || '').toLowerCase();
      if (tag === 'input') {
        var type = (el.getAttribute('type') || '').toLowerCase();
        if ((type === 'password' || type === 'hidden') && el.hasAttribute('value')) {
          el.setAttribute('value', '«masqué»');
          masked++;
        }
      } else if (tag === 'meta') {
        var name = ((el.getAttribute('name') || '') + ' ' + (el.getAttribute('property') || '') + ' ' + (el.getAttribute('http-equiv') || '')).toLowerCase();
        if (/csrf|xsrf|token|nonce/.test(name) && el.hasAttribute('content')) {
          el.setAttribute('content', '«masqué»');
          masked++;
        }
      } else if (tag === 'script' && el.textContent && el.textContent.trim()) {
        el.textContent = '«masqué»';
        masked++;
      }
    }
    return masked;
  }

  function read(a) {
    var found = elements(a.selector);
    if (found === null) return { error: 'selector' };
    var parts = [];
    var masked = 0;
    for (var i = 0; i < found.length; i++) {
      if (a.what === 'text') {
        parts.push(found[i].innerText || '');
      } else {
        var copy = found[i].cloneNode(true);
        masked += maskClone(copy);
        parts.push(copy.outerHTML);
      }
    }
    return { url: location.href, title: document.title, matched: found.length, parts: parts, masked: masked };
  }

  // The box of the first visible element, in CSS pixels of the viewport, with what a capture needs.
  function rect(a) {
    var found = elements(a.selector);
    if (found === null) return { error: 'selector' };
    var view = {
      innerWidth: innerWidth,
      innerHeight: innerHeight,
      scrollX: scrollX,
      scrollY: scrollY,
      dpr: devicePixelRatio || 1,
      docWidth: Math.max(document.documentElement.scrollWidth, document.body ? document.body.scrollWidth : 0),
      docHeight: Math.max(document.documentElement.scrollHeight, document.body ? document.body.scrollHeight : 0),
    };
    if (!a.selector) return { view: view, box: null };
    for (var i = 0; i < found.length; i++) {
      if (visible(found[i])) {
        var r = found[i].getBoundingClientRect();
        return { view: view, box: { x: r.x, y: r.y, width: r.width, height: r.height } };
      }
    }
    return { view: view, box: null, none: true };
  }

  // WebKit: the console relay of the page's world (`RELAY`) tells this world by an event; this
  // collector keeps what it tells (500 at most) until the core takes it. Installed at the first
  // drain of each document; the relay then gives what it kept before.
  function drain() {
    var c = window[COLLECTOR];
    if (!c) {
      c = { entries: [], dropped: 0 };
      Object.defineProperty(window, COLLECTOR, { value: c });
      window.addEventListener(
        EVENT,
        function (e) {
          if (typeof e.detail !== 'string' || e.detail.length > 4000) return;
          if (c.entries.length >= 500) {
            c.entries.shift();
            c.dropped++;
          }
          c.entries.push(e.detail);
        },
        true,
      );
      window.dispatchEvent(new CustomEvent(EVENT + '-ready'));
    }
    var out = { entries: c.entries, dropped: c.dropped };
    c.entries = [];
    c.dropped = 0;
    return out;
  }

  var result;
  try {
    if (args.run === 'read') result = read(args);
    else if (args.run === 'rect') result = rect(args);
    else if (args.run === 'drain') result = drain();
    else result = { error: 'unknown' };
  } catch (e) {
    result = { error: 'threw', message: String((e && e.message) || e).slice(0, 200) };
  }
  return JSON.stringify(result);
})
