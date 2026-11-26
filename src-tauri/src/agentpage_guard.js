// What Escouade runs in every frame of an agent's page before the page's own scripts (ESC-667,
// docs/AGENT-PAGES.md § 8.2): what the CSP does not cover, or not everywhere. Each property it sets
// can be neither written nor redefined by the page; what it keeps of the browser it keeps in this
// closure, out of the page's reach.
(function () {
  'use strict';
  var W = window;
  var define = Object.defineProperty;
  var lock = function (target, key, value) {
    try {
      define(target, key, { value: value, writable: false, configurable: false, enumerable: false });
    } catch (e) {}
  };

  // WebRTC: the CSP does not cover it, and WebView2 has no switch per webview.
  var rtc = ['RTCPeerConnection', 'webkitRTCPeerConnection', 'mozRTCPeerConnection', 'RTCDataChannel',
    'RTCSessionDescription', 'RTCIceCandidate', 'RTCRtpSender', 'RTCRtpReceiver', 'RTCRtpTransceiver',
    'RTCDtlsTransport', 'RTCIceTransport', 'RTCSctpTransport', 'RTCCertificate', 'RTCDTMFSender',
    'RTCEncodedAudioFrame', 'RTCEncodedVideoFrame', 'RTCError', 'RTCErrorEvent', 'RTCPeerConnectionIceEvent',
    'RTCTrackEvent', 'RTCDataChannelEvent', 'RTCStatsReport'];
  for (var i = 0; i < rtc.length; i++) {
    try { delete W[rtc[i]]; } catch (e) {}
    lock(W, rtc[i], undefined);
  }

  // Code made from text (P4: scripts, never `unsafe-eval`). The CSP refuses it, but WebView2 lets
  // `eval` through it (and `new Function` when its devtools may open): refused here as well.
  var refuse = function () {
    throw new EvalError('An agent page may not run code made from text (Escouade).');
  };
  lock(W, 'eval', function () { return refuse(); });
  // `Function` and its kin, reached by any function's `constructor`: each keeps its prototype, so
  // that `instanceof Function` still holds.
  var kinds = [
    Function.prototype,
    Object.getPrototypeOf(async function () {}),
    Object.getPrototypeOf(function* () {}),
    Object.getPrototypeOf(async function* () {}),
  ];
  for (var j = 0; j < kinds.length; j++) {
    var stop = function () { return refuse(); };
    stop.prototype = kinds[j];
    lock(kinds[j], 'constructor', stop);
    if (j === 0) lock(W, 'Function', stop);
  }

  // Prefetches a script adds: the CSP does not stop them all (a preconnection, a DNS prefetch, the
  // rules of speculation), and they leak by their address alone. Their `rel` (or `type`) is taken
  // off before they reach the document, by every way a page has to put them there.
  var proto = function (c) { return W[c] && W[c].prototype; };
  var E = proto('Element'), N = proto('Node'), D = proto('Document'), F = proto('DocumentFragment');
  var getAttr = E.getAttribute, removeAttr = E.removeAttribute, setAttr = E.setAttribute;
  var all = E.querySelectorAll, allIn = F.querySelectorAll;
  var leaks = /(^|\s)(dns-prefetch|preconnect|prefetch|prerender)(\s|$)/i;
  var speculation = /speculationrules/i;
  var tame = function (node) {
    if (!node || (node.nodeType !== 1 && node.nodeType !== 11)) return node;
    var found = node.nodeType === 1 ? [node] : [];
    try {
      var inside = (node.nodeType === 1 ? all : allIn).call(node, 'link, script');
      for (var n = 0; n < inside.length; n++) found.push(inside[n]);
    } catch (e) {}
    for (var m = 0; m < found.length; m++) {
      var el = found[m], name = (el.localName || '').toLowerCase();
      if (name === 'link' && leaks.test(getAttr.call(el, 'rel') || '')) removeAttr.call(el, 'rel');
      if (name === 'script' && speculation.test(getAttr.call(el, 'type') || '')) setAttr.call(el, 'type', 'text/plain');
    }
    return node;
  };
  var wrap = function (target, name) {
    var original = target && target[name];
    if (typeof original !== 'function') return;
    lock(target, name, function () {
      for (var a = 0; a < arguments.length; a++) tame(arguments[a]);
      return original.apply(this, arguments);
    });
  };
  ['appendChild', 'insertBefore', 'replaceChild'].forEach(function (n) { wrap(N, n); });
  [E, F, D, proto('CharacterData'), proto('DocumentType')].forEach(function (t) {
    ['append', 'prepend', 'before', 'after', 'replaceWith', 'replaceChildren'].forEach(function (n) {
      if (t && Object.prototype.hasOwnProperty.call(t, n)) wrap(t, n);
    });
  });
  wrap(E, 'insertAdjacentElement');
  // HTML given as text: parsed apart (a template's content is inert), tamed, given back as text.
  var template = document.createElement('template');
  var clean = function (html) {
    if (typeof html !== 'string' || !/<(link|script)/i.test(html)) return html;
    var t = template.cloneNode(false);
    t.innerHTML = html;
    tame(t.content);
    return t.innerHTML;
  };
  var setter = function (target, key) {
    var d = target && Object.getOwnPropertyDescriptor(target, key);
    if (!d || !d.set) return;
    define(target, key, {
      get: d.get, configurable: false, enumerable: d.enumerable,
      set: function (v) { return d.set.call(this, clean(String(v))); },
    });
  };
  setter(E, 'innerHTML');
  setter(E, 'outerHTML');
  setter(proto('ShadowRoot'), 'innerHTML');
  var textIn = function (target, name, at) {
    var original = target && target[name];
    if (typeof original !== 'function') return;
    lock(target, name, function () {
      var args = Array.prototype.slice.call(arguments);
      for (var a = at; a < args.length; a++) args[a] = clean(String(args[a]));
      return original.apply(this, args);
    });
  };
  textIn(E, 'insertAdjacentHTML', 1);
  textIn(E, 'setHTMLUnsafe', 0);
  textIn(proto('ShadowRoot'), 'setHTMLUnsafe', 0);
  textIn(D, 'write', 0);
  textIn(D, 'writeln', 0);
  var R = proto('Range');
  if (R && typeof R.createContextualFragment === 'function') {
    var fragment = R.createContextualFragment;
    lock(R, 'createContextualFragment', function () { return tame(fragment.apply(this, arguments)); });
  }
  // A `rel` (or `type`) changed on an element already in the document.
  var refuseLeak = function (value) { return leaks.test(String(value)) ? '' : value; };
  var L = proto('HTMLLinkElement'), S = proto('HTMLScriptElement');
  var relSet = L && Object.getOwnPropertyDescriptor(L, 'rel');
  if (relSet && relSet.set) {
    define(L, 'rel', { get: relSet.get, configurable: false, enumerable: relSet.enumerable, set: function (v) { relSet.set.call(this, refuseLeak(v)); } });
  }
  var typeSet = S && Object.getOwnPropertyDescriptor(S, 'type');
  if (typeSet && typeSet.set) {
    define(S, 'type', { get: typeSet.get, configurable: false, enumerable: typeSet.enumerable, set: function (v) { typeSet.set.call(this, speculation.test(String(v)) ? 'text/plain' : v); } });
  }
  var attr = function (el, name, value) {
    var n = String(name).toLowerCase(), tag = (el && el.localName || '').toLowerCase();
    if (tag === 'link' && n === 'rel') return refuseLeak(value);
    if (tag === 'script' && n === 'type' && speculation.test(String(value))) return 'text/plain';
    return value;
  };
  lock(E, 'setAttribute', function (name, value) { return setAttr.call(this, name, attr(this, name, value)); });
  var setAttrNS = E.setAttributeNS;
  lock(E, 'setAttributeNS', function (ns, name, value) { return setAttrNS.call(this, ns, name, attr(this, String(name).split(':').pop(), value)); });
  var A = proto('Attr'), valueSet = A && Object.getOwnPropertyDescriptor(A, 'value');
  if (valueSet && valueSet.set) {
    define(A, 'value', { get: valueSet.get, configurable: false, enumerable: valueSet.enumerable, set: function (v) { valueSet.set.call(this, attr(this.ownerElement, this.name, v)); } });
  }
  var T = proto('DOMTokenList');
  ['add', 'toggle', 'replace', 'supports'].forEach(function (n) {
    var original = T && T[n];
    if (typeof original !== 'function' || n === 'supports') return;
    lock(T, n, function () {
      for (var a = 0; a < arguments.length; a++) if (leaks.test(String(arguments[a]))) return false;
      return original.apply(this, arguments);
    });
  });
  var tokens = T && Object.getOwnPropertyDescriptor(T, 'value');
  if (tokens && tokens.set) {
    define(T, 'value', { get: tokens.get, configurable: false, enumerable: tokens.enumerable, set: function (v) { tokens.set.call(this, refuseLeak(v)); } });
  }
})();
