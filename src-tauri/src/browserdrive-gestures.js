// The user's gestures of a browser under WebKit, for the rule of the hand (ESC-679, docs/AGENT-BROWSERS.md § 9).
// Under WKWebView and WebKitGTK the isolated world `escouade-agent` keeps nothing from one evaluation to the next
// (ESC-692): no listener of Escouade's world outlives the call that set it. So this script, run at the start of each
// document in the page's world, counts the user's own gestures (pointer pressed, wheel, key, touch: `isTrusted`
// events; WebKit's gestures of the agent are the isolated world's synthetic ones, never trusted), and answers the
// count when Escouade's world asks by an event, inside that very call (`browserdrive.js`, `gestures`). The page can
// silence or forge it: a forged gesture only gives the user the hand; a silenced one leaves them « Reprendre la
// main ». Nothing here talks to Escouade.
(function () {
  'use strict';
  if (window.top !== window) return;
  var EVENT = 'escouade-drive-gestures';
  var dispatch = window.dispatchEvent.bind(window);
  var Custom = window.CustomEvent;
  var count = 0;
  function heard(e) {
    if (e.isTrusted) count++;
  }
  ['pointerdown', 'wheel', 'keydown', 'touchstart'].forEach(function (t) {
    window.addEventListener(t, heard, { capture: true, passive: true });
  });
  window.addEventListener(EVENT + '-pull', function () {
    var n = count;
    count = 0;
    dispatch(new Custom(EVENT, { detail: String(n) }));
  });
})();
