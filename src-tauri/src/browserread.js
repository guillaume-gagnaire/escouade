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
  var EVENT = 'escouade-console-relay';
  // A batch of the relay: 500 entries of 4 000 characters at most, escaped once more in JSON.
  var BATCH_MAX = 4200000;

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

  // WebKit: asks the console relay of the page's world (`RELAY`) for what it kept, by an event it
  // answers at once, inside this call. Nothing is kept in this world from one call to the next:
  // under WKWebView, it does not outlive the evaluation (ESC-692). 500 entries at most.
  function drain() {
    var out = { entries: [], dropped: 0 };
    function take(e) {
      if (typeof e.detail !== 'string' || e.detail.length > BATCH_MAX) return;
      var batch;
      try {
        batch = JSON.parse(e.detail);
      } catch (err) {
        return;
      }
      if (!batch || !Array.isArray(batch.entries)) return;
      if (typeof batch.dropped === 'number' && batch.dropped > 0) out.dropped += Math.floor(batch.dropped);
      for (var i = 0; i < batch.entries.length; i++) {
        var entry = batch.entries[i];
        if (typeof entry !== 'string' || entry.length > 4000) continue;
        if (out.entries.length >= 500) {
          out.entries.shift();
          out.dropped++;
        }
        out.entries.push(entry);
      }
    }
    window.addEventListener(EVENT, take, true);
    try {
      window.dispatchEvent(new CustomEvent(EVENT + '-drain'));
    } finally {
      window.removeEventListener(EVENT, take, true);
    }
    return out;
  }

  // How many calls this world has seen since it was made: 1 when it is new. The probe's measure of
  // whether an isolated world outlives an evaluation (ESC-692); nothing else uses it.
  function world() {
    var n = (window.__escouadeWorldCalls || 0) + 1;
    window.__escouadeWorldCalls = n;
    return { calls: n };
  }

  var result;
  try {
    if (args.run === 'read') result = read(args);
    else if (args.run === 'rect') result = rect(args);
    else if (args.run === 'drain') result = drain();
    else if (args.run === 'world') result = world();
    else result = { error: 'unknown' };
  } catch (e) {
    result = { error: 'threw', message: String((e && e.message) || e).slice(0, 200) };
  }
  return JSON.stringify(result);
})
