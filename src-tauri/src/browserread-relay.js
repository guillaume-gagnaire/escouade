// The console relay of a browser under WebKit (ESC-668, docs/AGENT-BROWSERS.md § 3 bis): WebKitGTK
// and WKWebView give no console to the app, so this script, run at the start of each document in
// the page's world, wraps `console.*`, `onerror`, `unhandledrejection` and the `fetch` and XHR that
// fail, and tells the isolated world `escouade-agent` by an event (a JSON string). The core takes
// what the collector of that world kept (`browserread.js`, `drain`). The page can silence or forge
// all of it: what comes from here is marked `from: "page"`. Nothing here talks to Escouade.
(function () {
  'use strict';
  if (window.top !== window) return;
  var EVENT = 'escouade-console-relay';
  var dispatch = window.dispatchEvent.bind(window);
  var Custom = window.CustomEvent;
  var stringify = JSON.stringify;
  var now = Date.now;
  var backlog = [];
  var ready = false;

  function tell(entry) {
    entry.t = now();
    var text;
    try {
      text = stringify(entry);
    } catch (e) {
      return;
    }
    if (!ready) {
      if (backlog.length >= 500) backlog.shift();
      backlog.push(text);
      return;
    }
    dispatch(new Custom(EVENT, { detail: text }));
  }

  // The collector says it listens: what came before goes to it.
  window.addEventListener(EVENT + '-ready', function () {
    ready = true;
    var kept = backlog;
    backlog = [];
    for (var i = 0; i < kept.length; i++) dispatch(new Custom(EVENT, { detail: kept[i] }));
  });

  function show(v) {
    if (typeof v === 'string') return v;
    try {
      var s = stringify(v);
      return s === undefined ? String(v) : s;
    } catch (e) {
      return String(v);
    }
  }

  ['log', 'info', 'warn', 'error', 'debug'].forEach(function (level) {
    var original = console[level];
    if (typeof original !== 'function') return;
    console[level] = function () {
      var parts = [];
      for (var i = 0; i < arguments.length; i++) parts.push(show(arguments[i]));
      tell({ kind: 'console', level: level, text: parts.join(' ').slice(0, 1000) });
      return original.apply(this, arguments);
    };
  });

  window.addEventListener(
    'error',
    function (e) {
      var target = e.target;
      if (target && target !== window && (target.src || target.href)) {
        tell({ kind: 'network', method: 'GET', url: String(target.src || target.href), error: 'failed', type: String(target.localName || '') });
        return;
      }
      tell({ kind: 'console', level: 'error', source: 'exception', text: String(e.message || '').slice(0, 1000), url: String(e.filename || ''), line: e.lineno || 0 });
    },
    true,
  );
  window.addEventListener('unhandledrejection', function (e) {
    var r = e.reason;
    tell({ kind: 'console', level: 'error', source: 'exception', text: 'Unhandled rejection: ' + show(r && r.message ? r.message : r).slice(0, 1000) });
  });

  var fetch0 = window.fetch;
  if (typeof fetch0 === 'function') {
    window.fetch = function (input, init) {
      var method = String((init && init.method) || (input && input.method) || 'GET').toUpperCase();
      var url = String((input && input.url) || input);
      return fetch0.apply(this, arguments).then(
        function (response) {
          if (response.status >= 400) tell({ kind: 'network', method: method, url: response.url || url, status: response.status, type: 'fetch' });
          return response;
        },
        function (error) {
          tell({ kind: 'network', method: method, url: url, error: String((error && error.message) || error).slice(0, 200), type: 'fetch' });
          throw error;
        },
      );
    };
  }

  var XHR = window.XMLHttpRequest;
  if (XHR) {
    var open0 = XHR.prototype.open;
    XHR.prototype.open = function (method, url) {
      var xhr = this;
      var m = String(method || 'GET').toUpperCase();
      var u = String(url);
      xhr.addEventListener('loadend', function () {
        if (xhr.status === 0) tell({ kind: 'network', method: m, url: u, error: 'failed', type: 'xhr' });
        else if (xhr.status >= 400) tell({ kind: 'network', method: m, url: xhr.responseURL || u, status: xhr.status, type: 'xhr' });
      });
      return open0.apply(this, arguments);
    };
  }
})();
