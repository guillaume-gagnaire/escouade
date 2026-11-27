// The script of the templates of the agents' pages (ESC-674, docs/AGENT-PAGES.md § 7), shipped with the app. It reads
// nothing but its own document, which the core wrote with the agent's data as text: it never makes HTML of that text.
// It sets the theme (`?theme=dark|light` of the page's address), switches the tabs a page declares, proposes an option
// of the page's request (`POST _/choose`: a proposal the user confirms in Escouade's band, AP-4), and draws a diagram
// with the embedded Mermaid, without network, in `securityLevel: 'strict'`.
(function () {
  'use strict';
  var root = document.documentElement;
  var theme = new URLSearchParams(location.search).get('theme');
  if (theme === 'dark' || theme === 'light') root.setAttribute('data-theme', theme);
  var dark = root.getAttribute('data-theme') !== 'light';

  function all(sel, from) {
    return Array.prototype.slice.call((from || document).querySelectorAll(sel));
  }

  // A review's tabs: a ticket of a lot, or all of them.
  all('.tabs [role="tab"]').forEach(function (tab) {
    tab.addEventListener('click', function () {
      var want = tab.getAttribute('data-tab');
      all('.tabs [role="tab"]').forEach(function (t) {
        t.setAttribute('aria-selected', String(t === tab));
      });
      all('[data-ticket]').forEach(function (el) {
        if (!want || el.getAttribute('data-ticket') === want) el.removeAttribute('data-hidden-by-tab');
        else el.setAttribute('data-hidden-by-tab', '');
      });
    });
  });

  // A plate's renders, dark or light (« Sombre · Clair »): the page's theme first.
  all('.plate').forEach(function (plate) {
    var modes = all('.modes [role="tab"]', plate);
    function show(mode) {
      plate.setAttribute('data-theme-shown', mode);
      modes.forEach(function (m) {
        m.setAttribute('aria-selected', String(m.getAttribute('data-mode') === mode));
      });
    }
    modes.forEach(function (m) {
      m.addEventListener('click', function () {
        show(m.getAttribute('data-mode'));
      });
    });
    show(dark ? 'dark' : 'light');
  });

  // The request's options: a click proposes, nothing more. The core checks the webview, the request and its options;
  // the band of Escouade, outside the page, shows the request's own words and sends on the user's gesture.
  var buttons = all('[data-option]');
  var holder = document.querySelector('[data-ask]');
  var multi = holder && holder.hasAttribute('data-multi');
  buttons.forEach(function (b) {
    b.addEventListener('click', function () {
      if (b.disabled) return;
      var id = b.getAttribute('data-option');
      if (multi) b.setAttribute('aria-pressed', String(b.getAttribute('aria-pressed') !== 'true'));
      else
        buttons.forEach(function (x) {
          x.setAttribute('aria-pressed', String(x.getAttribute('data-option') === id));
        });
      var chosen = buttons
        .filter(function (x) {
          return x.getAttribute('aria-pressed') === 'true';
        })
        .map(function (x) {
          return x.getAttribute('data-option');
        })
        .filter(function (v, i, a) {
          return a.indexOf(v) === i;
        });
      try {
        fetch('_/choose', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ options: chosen }),
        }).catch(function () {});
      } catch (e) {}
    });
  });

  // ---------- the diagram ----------
  var section = document.getElementById('diagram');
  if (!section) return;
  var stage = section.querySelector('.stage');
  var canvas = section.querySelector('.canvas');
  var source = document.getElementById('diagram-source');
  var failed = section.querySelector('.failed');
  var view = { s: 1, x: 0, y: 0 };
  function apply() {
    canvas.style.transform = 'translate(' + view.x + 'px,' + view.y + 'px) scale(' + view.s + ')';
    section.setAttribute('data-zoom', String(Math.round(view.s * 100)));
  }
  function size() {
    var svg = canvas.querySelector('svg');
    if (!svg) return null;
    var box = svg.viewBox && svg.viewBox.baseVal;
    var w = (box && box.width) || svg.getBoundingClientRect().width / view.s;
    var h = (box && box.height) || svg.getBoundingClientRect().height / view.s;
    return { w: w, h: h };
  }
  function fit() {
    var d = size();
    if (!d) return;
    var sw = stage.clientWidth - 24;
    var s = Math.min(1.5, sw / d.w);
    // Room above for the zoom's bar (44 px), and a margin below.
    stage.style.setProperty('--h', Math.ceil(d.h * s + 60) + 'px');
    var sh = stage.clientHeight - 60;
    s = Math.max(0.1, Math.min(s, sh / d.h));
    view.s = s;
    view.x = Math.max(12, (stage.clientWidth - d.w * s) / 2);
    view.y = Math.max(46, (stage.clientHeight - d.h * s + 32) / 2);
    apply();
  }
  function zoom(f, cx, cy) {
    var s = Math.max(0.1, Math.min(6, view.s * f));
    if (cx == null) {
      cx = stage.clientWidth / 2;
      cy = stage.clientHeight / 2;
    }
    view.x = cx - ((cx - view.x) * s) / view.s;
    view.y = cy - ((cy - view.y) * s) / view.s;
    view.s = s;
    apply();
  }
  all('[data-zoom]', section).forEach(function (b) {
    b.addEventListener('click', function () {
      var z = b.getAttribute('data-zoom');
      if (z === 'fit') fit();
      else zoom(z === 'in' ? 1.25 : 0.8);
    });
  });
  stage.addEventListener(
    'wheel',
    function (e) {
      if (!e.ctrlKey && !e.metaKey) return;
      e.preventDefault();
      var r = stage.getBoundingClientRect();
      zoom(e.deltaY < 0 ? 1.1 : 0.9, e.clientX - r.left, e.clientY - r.top);
    },
    { passive: false },
  );
  var drag = null;
  stage.addEventListener('pointerdown', function (e) {
    drag = { x: e.clientX - view.x, y: e.clientY - view.y };
    stage.setPointerCapture(e.pointerId);
  });
  stage.addEventListener('pointermove', function (e) {
    if (!drag) return;
    view.x = e.clientX - drag.x;
    view.y = e.clientY - drag.y;
    apply();
  });
  stage.addEventListener('pointerup', function () {
    drag = null;
  });
  stage.addEventListener('keydown', function (e) {
    var step = 40;
    if (e.key === '+' || e.key === '=') zoom(1.25);
    else if (e.key === '-') zoom(0.8);
    else if (e.key === '0') fit();
    else if (e.key === 'ArrowLeft') view.x += step;
    else if (e.key === 'ArrowRight') view.x -= step;
    else if (e.key === 'ArrowUp') view.y += step;
    else if (e.key === 'ArrowDown') view.y -= step;
    else return;
    e.preventDefault();
    apply();
  });

  // Each node keeps its id from the source (`data-node`), for `drive_browser` to aim at (`nodes.<id>`).
  function tag() {
    all('g.node, g[id^="state-"], g.actor, rect.actor, g.classGroup', canvas).forEach(function (g) {
      var id = g.getAttribute('data-id');
      if (!id) {
        var m = /(?:^|-)(?:flowchart|state|classId|entity)-(.+?)-\d+$/.exec(g.id || '');
        id = m ? m[1] : g.getAttribute('name');
      }
      if (id) g.setAttribute('data-node', id);
    });
  }
  function fail(e) {
    section.setAttribute('data-state', 'failed');
    failed.hidden = false;
    source.hidden = false;
    source.classList.add('shown');
    var why = document.createElement('code');
    why.textContent = String((e && (e.message || e.str)) || e || '').slice(0, 500);
    failed.appendChild(document.createElement('br'));
    failed.appendChild(why);
  }
  var mermaid = window.mermaid;
  if (!mermaid) return fail('Mermaid is not here');
  var css = getComputedStyle(root);
  var font = css.getPropertyValue('--font').trim() || 'system-ui';
  mermaid.initialize({
    startOnLoad: false,
    securityLevel: 'strict',
    theme: dark ? 'dark' : 'default',
    fontFamily: font,
    deterministicIds: true,
    maxTextSize: 65536,
    htmlLabels: false,
    flowchart: { htmlLabels: false },
  });
  mermaid
    .render('escouade-diagram', source.textContent)
    .then(function (out) {
      canvas.innerHTML = out.svg;
      // Its own size, from its view box: Mermaid gives it a width of 100 %, which the zoom would follow.
      var svg = canvas.querySelector('svg');
      var box = svg && svg.viewBox && svg.viewBox.baseVal;
      if (box && box.width) {
        svg.setAttribute('width', String(box.width));
        svg.setAttribute('height', String(box.height));
        svg.style.maxWidth = 'none';
      }
      tag();
      section.setAttribute('data-state', 'drawn');
      fit();
    })
    .catch(function (e) {
      var stray = document.getElementById('descouade-diagram');
      if (stray) stray.remove();
      fail(e);
    });
  addEventListener('resize', function () {
    if (section.getAttribute('data-state') === 'drawn') fit();
  });
})();
