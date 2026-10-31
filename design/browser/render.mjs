// The three directions of ESC-311 (a browser inside Escouade), drawn over the app's own preview
// (design/app-redesign/index.html) so each one sits in the real window: `node design/browser/render.mjs`.
// a-onglet.png: the page as a view of the agent, beside the conversation and its terminal.
// a-tablette(-en).png, a-mobile(-en).png, a-retreci(-en).png: its widths (the user's addition): a tablet's and a
// phone's, centred on a neutral ground, and a tablet's narrowed to a room narrower than it (the CI window, 1028 px).
// b-panneau.png: the page as a tab of the right panel, beside « Non commités » and « PR et CI ».
// c-fenetre.png: the page in a window of its own, tied to the project.
// etats.png: the states every direction shares (loading, waiting for the launch, not answering, stopped…).

import { dirname, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const dir = dirname(fileURLToPath(import.meta.url));
const app = pathToFileURL(join(dir, '..', 'app-redesign', 'index.html')).href;
const SIZE = { width: 1440, height: 900 };

const ICONS = `<svg width="0" height="0" style="position:absolute"><defs>
<symbol id="b-back" viewBox="0 0 16 16"><path d="M10 3.5 5.5 8l4.5 4.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></symbol>
<symbol id="b-fwd" viewBox="0 0 16 16"><path d="M6 3.5 10.5 8 6 12.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></symbol>
<symbol id="b-reload" viewBox="0 0 16 16"><path d="M13 8a5 5 0 1 1-1.5-3.6M13 2.8v2.7h-2.7" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></symbol>
<symbol id="b-tools" viewBox="0 0 16 16"><path d="M5.5 4.5 2 8l3.5 3.5M10.5 4.5 14 8l-3.5 3.5" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></symbol>
<symbol id="b-ext" viewBox="0 0 16 16"><path d="M9 2.5h4.5V7M13.5 2.5 7.5 8.5M12 9.5v3a1 1 0 0 1-1 1H3.5a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h3" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"/></symbol>
<symbol id="b-monitor" viewBox="0 0 16 16"><rect x="2" y="2.75" width="12" height="8.5" rx="1.4" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M6 13.25h4M8 11.25v2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></symbol>
<symbol id="b-tablet" viewBox="0 0 16 16"><rect x="3.5" y="2" width="9" height="12" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M7.25 11.75h1.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></symbol>
<symbol id="b-phone" viewBox="0 0 16 16"><rect x="5" y="2" width="6" height="12" rx="1.4" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M7.4 11.75h1.2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></symbol>
<symbol id="b-expand" viewBox="0 0 16 16"><path d="M9.5 2.75h3.75V6.5M6.5 13.25H2.75V9.5M13.25 2.75 9 7M2.75 13.25 7 9" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></symbol>
<symbol id="b-page" viewBox="0 0 16 16"><rect x="2" y="3" width="12" height="10" rx="1.6" fill="none" stroke="currentColor" stroke-width="1.4"/><path d="M2 6h12" stroke="currentColor" stroke-width="1.4"/><circle cx="4.2" cy="4.6" r=".55" fill="currentColor"/><circle cx="5.9" cy="4.6" r=".55" fill="currentColor"/></symbol>
</defs></svg>`;

const CSS = `
.bw { display:flex; flex-direction:column; flex:1; min-height:0; min-width:0; }
.bw-bar { display:flex; align-items:center; gap:4px; height:46px; padding:0 10px; border-bottom:1px solid var(--line); flex:none; }
.bw-bar .ib { display:inline-grid; place-items:center; width:28px; height:28px; border-radius:7px; color:var(--muted); }
.bw-bar .ib:hover { background:var(--hover); color:var(--text); }
.bw-bar .ib[disabled] { opacity:.38; }
.bw-bar .ib svg { width:16px; height:16px; }
.bw-addr { flex:1; min-width:0; display:flex; align-items:center; gap:8px; height:30px; margin:0 4px; padding:0 10px; border-radius:7px; background:var(--well); box-shadow:inset 0 0 0 1px var(--line); font:12.5px/1 'JetBrains Mono', ui-monospace, Consolas, monospace; white-space:nowrap; overflow:hidden; }
.bw-addr .o { color:var(--muted); } .bw-addr .pth { color:var(--text); overflow:hidden; text-overflow:ellipsis; }
.bw-addr .dot { flex:none; width:7px; height:7px; border-radius:50%; background:var(--run); box-shadow:0 0 0 3px color-mix(in oklch, var(--run) 18%, transparent); }
.bw-addr .dot.wait { background:var(--wait); box-shadow:0 0 0 3px color-mix(in oklch, var(--wait) 18%, transparent); }
.bw-addr .dot.bad { background:var(--del); box-shadow:0 0 0 3px color-mix(in oklch, var(--del) 18%, transparent); }
.bw-owner { display:inline-flex; align-items:center; gap:6px; height:28px; padding:0 9px; border-radius:7px; color:var(--soft); font-size:12px; font-weight:550; white-space:nowrap; box-shadow:inset 0 0 0 1px var(--line); }
.bw-owner .k { color:var(--muted); font-weight:500; } .bw-owner svg { width:12px; height:12px; color:var(--muted); }
.bw-dt { display:inline-flex; align-items:center; gap:6px; height:28px; padding:0 9px; border-radius:7px; color:var(--soft); font-size:12px; font-weight:550; box-shadow:inset 0 0 0 1px var(--line); white-space:nowrap; }
.bw-dt svg { width:14px; height:14px; } .bw-dt kbd { font:500 10.5px Inter, sans-serif; color:var(--muted); padding:1px 4px; border-radius:4px; box-shadow:inset 0 0 0 1px var(--line-2); }
.bw-dt.on { background:var(--sel); color:var(--text); box-shadow:inset 0 0 0 1px color-mix(in oklch, var(--p) 40%, transparent); }
.bw-sep { width:1px; height:18px; background:var(--line); margin:0 4px; }
.bw-load { height:2px; flex:none; background:linear-gradient(90deg, var(--p-ink) 0 62%, transparent 62%); }
.bw-view { position:relative; flex:1; min-height:0; overflow:hidden; background:#fff; }
.bw-foot { display:flex; align-items:center; gap:10px; height:28px; padding:0 12px; border-top:1px solid var(--line); color:var(--muted); font-size:11.5px; flex:none; }
.bw-foot b { color:var(--soft); font-weight:550; } .bw-foot .sp { flex:1; }
.bw-foot .d { width:6px; height:6px; border-radius:50%; background:var(--run); }

/* The developed app: someone else's site, in its own colours. */
.site { font:14px/1.45 system-ui, 'Segoe UI', sans-serif; color:#1d2433; height:100%; background:#f6f7fb; }
.site header { display:flex; align-items:center; gap:22px; height:52px; padding:0 24px; background:#fff; border-bottom:1px solid #e3e6ee; }
.site header .lg { font-weight:700; color:#3b5bdb; letter-spacing:-.01em; }
.site header nav { display:flex; gap:16px; color:#5b6478; font-size:13.5px; } .site header nav .on { color:#1d2433; font-weight:600; }
.site main { padding:22px 24px; }
.site h2 { margin:0 0 14px; font-size:19px; letter-spacing:-.01em; }
.site .search { display:flex; gap:8px; margin-bottom:14px; } .site .search input { flex:1; max-width:340px; height:34px; padding:0 10px; border:1px solid #cfd4e0; border-radius:6px; font:inherit; background:#fff; } .site .search button { height:34px; padding:0 14px; border:0; border-radius:6px; background:#3b5bdb; color:#fff; font:600 13px system-ui; }
.site table { width:100%; border-collapse:collapse; background:#fff; border:1px solid #e3e6ee; border-radius:8px; font-size:13.5px; }
.site th { text-align:left; padding:9px 12px; color:#5b6478; font-weight:600; font-size:12px; text-transform:uppercase; letter-spacing:.03em; border-bottom:1px solid #e3e6ee; }
.site td { padding:9px 12px; border-bottom:1px solid #eef0f5; } .site td.m { color:#5b6478; }
.site .tag { display:inline-block; padding:1px 7px; border-radius:10px; background:#e7f5ec; color:#22784a; font-size:12px; font-weight:600; }
.site .tag.off { background:#f1f2f6; color:#6b7387; }
.site .pager { display:flex; justify-content:space-between; margin-top:12px; color:#5b6478; font-size:13px; }
.site .search input { color:#1d2433 !important; -webkit-text-fill-color:#1d2433; }
.site .pager a { color:#3b5bdb; font-weight:600; }

/* A: the agent's views get a third one, its page. */
#view-agents.bw-a > .conv > :not(.head):not(.bw) { display:none !important; }
/* B: the right panel, wider while it shows a page. */
#side.bw-b { width:58% !important; }
#side.bw-b > .side-pane, #side.bw-b > [id^='side-'] { display:none !important; }
`;

const SITE = (narrow) => `<div class="site">
  <header><span class="lg">demo-api</span><nav><span class="on">Utilisateurs</span><span>Factures</span>${narrow ? '' : '<span>Webhooks</span><span>Réglages</span>'}</nav></header>
  <main>
    <h2>Utilisateurs</h2>
    <div class="search"><input value="marie@" /><button>Rechercher</button></div>
    <table>
      <thead><tr><th>Nom</th><th>E-mail</th>${narrow ? '' : '<th>Inscrit le</th>'}<th>Statut</th></tr></thead>
      <tbody>
        <tr><td>Marie Lefèvre</td><td class="m">marie@lefevre.fr</td>${narrow ? '' : '<td class="m">12 mars 2026</td>'}<td><span class="tag">Actif</span></td></tr>
        <tr><td>Marie-Anne Roux</td><td class="m">marie@roux-conseil.com</td>${narrow ? '' : '<td class="m">3 juin 2026</td>'}<td><span class="tag">Actif</span></td></tr>
        <tr><td>Marie Dubois</td><td class="m">MARIE@dubois.net</td>${narrow ? '' : '<td class="m">28 août 2026</td>'}<td><span class="tag off">Suspendu</span></td></tr>
      </tbody>
    </table>
    <div class="pager"><span>3 résultats · 50 par page</span><a>Suivants →</a></div>
  </main>
</div>`;

// The bar every direction shares: back, forward, reload, the address, whose page it is, devtools, out to the system browser.
const bar = ({ owner = true, devtoolsLabel = true, dot = '' } = {}) => `<div class="bw-bar">
  <button class="ib" aria-label="Retour"><svg><use href="#b-back"/></svg></button>
  <button class="ib" aria-label="Avancer" disabled><svg><use href="#b-fwd"/></svg></button>
  <button class="ib" aria-label="Recharger"><svg><use href="#b-reload"/></svg></button>
  <div class="bw-addr"><i class="dot ${dot}"></i><span class="o">localhost:41731</span><span class="pth">/users?search=marie@</span></div>
  ${owner ? '<span class="bw-owner"><svg class="i"><use href="#i-branch"/></svg>pagination-users <span class="k">· web</span></span>' : ''}
  <span class="bw-sep"></span>
  <button class="bw-dt"><svg><use href="#b-tools"/></svg>${devtoolsLabel ? 'Devtools <kbd>F12</kbd>' : ''}</button>
  <button class="ib" aria-label="Ouvrir dans le navigateur du système"><svg><use href="#b-ext"/></svg></button>
</div>`;

const browser = (opts = {}) => `<div class="bw">${bar(opts)}<div class="bw-view">${SITE(opts.narrow)}</div>
<div class="bw-foot"><i class="d"></i><span>Lancement <b>web</b> prêt en 2,7 s</span><span class="sp"></span><span>Page à part : ni Escouade, ni tes cookies</span></div></div>`;

async function open(page, hash) {
  await page.goto('about:blank');
  await page.goto(`${app}#${hash}`);
  await page.waitForTimeout(400);
  await page.evaluate(
    ([css, icons]) => {
      const s = document.createElement('style');
      s.textContent = css;
      document.head.append(s);
      document.body.insertAdjacentHTML('afterbegin', icons);
    },
    [CSS, ICONS],
  );
}

// The launches in the sidebar: the worktree's web says its page is open.
const launches = `(() => {
  const g = document.querySelector('#test-group'); if (!g) return;
  g.hidden = false; g.querySelector('.group-label').textContent = 'pagination-users';
  const rows = g.querySelectorAll('.row-item'); const web = rows[rows.length - 1];
  web.querySelector('.st').innerHTML = '<svg class="i s14" style="width:13px;height:13px;vertical-align:-2px;margin-right:4px"><use href="#b-page"/></svg>page ouverte';
  web.style.background = 'var(--sel)';
})()`;

async function directionA(page) {
  await open(page, 'agents');
  await page.evaluate(`(() => {
    ${launches};
    const v = document.querySelector('#view-agents'); v.classList.add('bw-a');
    const seg = document.querySelector('#vseg');
    seg.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', 'false'));
    seg.insertAdjacentHTML('beforeend', '<button aria-pressed="true" aria-label="Page du lancement"><svg class="i s14"><use href="#b-page"/></svg><span class="lbl">Page</span></button>');
    document.querySelector('#h-launch').hidden = true;
    document.querySelector('.conv').insertAdjacentHTML('beforeend', ${JSON.stringify(browser())});
    const side = document.querySelector('#side'); if (side) side.hidden = true;
  })()`);
}

async function directionB(page) {
  await open(page, 'git');
  await page.evaluate(`(() => {
    ${launches};
    const side = document.querySelector('#side'); side.classList.add('bw-b');
    const tabs = side.querySelector('.utabs');
    tabs.querySelectorAll('[role=tab]').forEach((b) => b.setAttribute('aria-selected', 'false'));
    tabs.querySelector('[data-side=hosts]').innerHTML = 'PR et CI';
    tabs.querySelector('[data-side=loop]').remove();
    tabs.querySelector('.side-x').insertAdjacentHTML('beforebegin', '<button role="tab" aria-selected="true">Page <span class="n" style="display:inline-block;width:6px;height:6px;border-radius:50%;background:var(--run)"></span></button><span style="flex:1"></span>');
    side.insertAdjacentHTML('beforeend', ${JSON.stringify(browser({ owner: false, devtoolsLabel: false, narrow: true }))});
    const plan = document.querySelector('#plan'); if (plan) plan.style.display = 'none';
  })()`);
}

async function directionC(page, backdrop) {
  // The main window behind, as it is; the page's own window on top, tied to the project by its title and tint.
  await page.setContent(`<!doctype html><html><head><style>
    body { margin:0; width:1440px; height:900px; background:#000 url(data:image/png;base64,${backdrop}) 0 0/1440px 900px; font-family:Inter, system-ui, sans-serif; overflow:hidden; }
    .shade { position:absolute; inset:0; background:rgba(0,0,0,.28); }
    .win { position:absolute; left:250px; top:96px; width:1120px; height:760px; display:flex; flex-direction:column; border-radius:10px; overflow:hidden; background:var(--panel); box-shadow:0 0 0 1px var(--line-2), 0 30px 80px -10px rgba(0,0,0,.7); }
    .tb { display:flex; align-items:center; gap:10px; height:40px; padding:0 0 0 14px; background:var(--frame); flex:none; font-size:12.5px; color:var(--soft); }
    .tb .lg { width:18px; height:18px; border-radius:5px; background:#35c1bd; display:grid; place-items:center; color:#08090a; font:700 11px Inter; }
    .tb .proj { display:inline-flex; align-items:center; gap:6px; font-weight:600; color:var(--text); } .tb .proj i { width:8px; height:8px; border-radius:2px; background:var(--p); }
    .tb .k { color:var(--muted); } .tb .sp { flex:1; }
    .tb .pages { display:flex; gap:2px; margin-left:14px; }
    .tb .pg { display:inline-flex; align-items:center; gap:6px; height:28px; padding:0 10px; border-radius:7px 7px 0 0; color:var(--muted); font-weight:550; margin-top:12px; }
    .tb .pg.on { background:var(--panel); color:var(--text); } .tb .pg .d { width:6px; height:6px; border-radius:50%; background:var(--run); } .tb .pg .d.off { background:var(--dim); }
    .tb .ctl { display:flex; } .tb .ctl span { width:46px; height:40px; display:grid; place-items:center; color:var(--muted); }
  </style></head><body><div class="shade"></div></body></html>`);
  // Borrow the app's tokens and fonts from the preview.
  const tokens = await (async () => {
    const p = await page.context().newPage();
    await p.goto(`${app}#agents`);
    const styles = await p.evaluate(() => [...document.querySelectorAll('style')].map((s) => s.textContent).join('\n'));
    await p.close();
    return styles;
  })();
  await page.evaluate(
    ([tokens, css, icons, html]) => {
      const s = document.createElement('style');
      s.textContent = tokens + css + 'body{background-color:#000 !important}';
      document.head.prepend(s);
      document.body.insertAdjacentHTML('afterbegin', icons);
      document.body.insertAdjacentHTML('beforeend', html);
    },
    [
      tokens,
      CSS,
      ICONS,
      `<div class="win"><div class="tb"><span class="lg">&gt;</span><span class="proj"><i></i>demo-api</span><span class="k">· Pages</span>
        <div class="pages"><span class="pg on"><i class="d"></i>pagination-users · web</span><span class="pg"><i class="d"></i>refacto-auth · Front</span><span class="pg"><i class="d off"></i>main · Front</span></div>
        <span class="sp"></span><span class="ctl"><span>–</span><span>□</span><span>✕</span></span></div>${browser()}</div>`,
    ],
  );
  // The title bar's window controls are the OS's own: drawn here as the app's, with its icons.
  await page.evaluate(() => {
    const c = document.querySelector('.ctl');
    c.innerHTML = ['i-min', 'i-max', 'i-close'].map((i) => `<span><svg class="i s14"><use href="#${i}"/></svg></span>`).join('');
  });
}

// Every state, one bar each, the same in all three directions.
async function states(page) {
  const tokens = await (async () => {
    const p = await page.context().newPage();
    await p.goto(`${app}#agents`);
    const html = await p.evaluate(() => ({ css: [...document.querySelectorAll('style')].map((s) => s.textContent).join('\n'), sprite: document.querySelector('svg[style*="display: none"], svg[width="0"]')?.outerHTML ?? '' }));
    await p.close();
    return html;
  })();
  const msg = (dot, title, text, actions, tone = '') => `
    <div class="bw-view st ${tone}"><div class="msg"><i class="dot ${dot}"></i><b>${title}</b><p>${text}</p><div class="acts">${actions}</div></div></div>`;
  const row = (label, view, opts = {}) => `<section><h3>${label}</h3><div class="card">${bar({ dot: opts.dot ?? '', owner: false, devtoolsLabel: true })}${opts.load ? '<div class="bw-load"></div>' : ''}${view}</div></section>`;
  const html = `
  <div class="grid">
    ${row('Le lancement démarre : la page attend son adresse', msg('wait', 'web démarre…', 'La page s’ouvre ici dès que <span class="mono">localhost:41731</span> répond. Préparation <span class="mono">npm ci</span> terminée, api prête.', '<button class="btn">Voir les logs</button>'), { dot: 'wait' })}
    ${row('Chargement', `<div class="bw-view" style="opacity:.55">${SITE(true)}</div>`, { load: true, dot: 'wait' })}
    ${row('La page ne répond pas', msg('bad', 'localhost:41731 ne répond pas', 'Le serveur tourne encore mais ne répond plus depuis 20 s.', '<button class="btn primary">Recharger</button><button class="btn">Voir les logs</button>'), { dot: 'bad' })}
    ${row('Le lancement s’est arrêté', msg('bad', 'Le lancement « web » s’est arrêté (code 1)', 'La page reste figée telle qu’elle était. Relance pour la retrouver à la même adresse.', '<button class="btn primary">Relancer</button><button class="btn">Voir les logs</button>'), { dot: 'bad' })}
    ${row('Un lien vers une autre origine', `<div class="bw-view">${SITE(true)}<div class="bw-toast"><svg class="i s14"><use href="#b-ext"/></svg><span><b>github.com</b> s’est ouvert dans ton navigateur : ici, seul <span class="mono">localhost:41731</span> s’affiche.</span></div></div>`)}
    ${row('Plusieurs worktrees : une page chacun', `<div class="bw-view">${SITE(true)}<div class="menu-pages"><div class="h">Pages de demo-api</div>
      <div class="it on"><i class="d"></i><b>pagination-users</b><span>web · :41731</span></div>
      <div class="it"><i class="d"></i><b>refacto-auth</b><span>Front · :41742</span></div>
      <div class="it"><i class="d off"></i><b>main</b><span>Front · arrêté</span></div></div></div>`)}
  </div>
  <div class="grid two">
    <section><h3>Réglage par projet (Réglages › Projets › demo-api)</h3><div class="card set">
      <div class="r"><div><b>Ouvrir les pages des lancements</b><p>Quand un lancement est prêt, sa page s’ouvre toute seule.</p></div>
      <span class="seg inline" role="radiogroup"><button role="radio" aria-selected="true">Dans Escouade</button><button role="radio" aria-selected="false">Dans le navigateur du système</button></span></div></div></section>
    <section><h3>Petite largeur : la barre se resserre</h3><div class="card narrow">
      <div class="bw-bar"><button class="ib"><svg><use href="#b-back"/></svg></button><button class="ib"><svg><use href="#b-reload"/></svg></button><div class="bw-addr"><i class="dot"></i><span class="pth">/users?search=marie@</span></div><button class="bw-dt"><svg><use href="#b-tools"/></svg></button><button class="ib"><svg><use href="#i-more"/></svg></button></div></div></section>
  </div>`;
  await page.setContent(`<!doctype html><html><head><style>${tokens.css}
    html, body { height:auto; overflow:visible; }
    body { margin:0; padding:28px 32px; background:var(--frame); color:var(--text); font:13px/1.45 'Inter Variable', Inter, system-ui, sans-serif; }
    .grid { display:grid; grid-template-columns:repeat(3, minmax(0, 1fr)); gap:22px 20px; } .grid.two { grid-template-columns:minmax(0, 1.3fr) minmax(0, 1fr); margin-top:22px; }
    h3 { margin:0 0 8px; font-size:12px; font-weight:550; color:var(--muted); }
    .card { display:flex; flex-direction:column; height:250px; border-radius:12px; overflow:hidden; background:var(--panel); box-shadow:0 0 0 1px var(--line); }
    .card.set, .card.narrow { height:auto; } .card.narrow { width:330px; }
    .bw-view.st { display:grid; place-items:center; background:var(--panel); }
    .msg { max-width:330px; text-align:center; } .msg b { display:block; margin:10px 0 4px; font-size:13.5px; } .msg p { margin:0 0 12px; color:var(--muted); font-size:12.5px; }
    .msg .dot { display:inline-block; width:9px; height:9px; border-radius:50%; background:var(--wait); } .msg .dot.bad { background:var(--del); }
    .msg .acts { display:flex; gap:8px; justify-content:center; }
    .bw-toast { position:absolute; left:12px; right:12px; bottom:12px; display:flex; gap:9px; align-items:flex-start; padding:10px 12px; border-radius:10px; background:var(--raise); color:var(--soft); font-size:12.5px; box-shadow:0 0 0 1px var(--line-2), 0 10px 30px -8px rgba(0,0,0,.6); }
    .bw-toast svg { flex:none; margin-top:2px; color:var(--p-ink); } .bw-toast b { color:var(--text); }
    .menu-pages { position:absolute; top:6px; right:10px; width:250px; padding:6px; border-radius:10px; background:var(--raise); box-shadow:0 0 0 1px var(--line-2), 0 14px 34px -8px rgba(0,0,0,.6); }
    .menu-pages .h { padding:6px 8px; color:var(--muted); font-size:11.5px; font-weight:550; }
    .menu-pages .it { display:flex; align-items:center; gap:8px; height:30px; padding:0 8px; border-radius:7px; color:var(--soft); }
    .menu-pages .it.on { background:var(--sel); color:var(--text); } .menu-pages .it span { margin-left:auto; color:var(--muted); font-size:11.5px; }
    .menu-pages .d { width:6px; height:6px; border-radius:50%; background:var(--run); } .menu-pages .d.off { background:var(--dim); }
    .set .r { display:flex; align-items:center; gap:20px; padding:16px 18px; } .set .r > div { flex:1; } .set p { margin:2px 0 0; color:var(--muted); font-size:12.5px; }
    ${CSS}</style></head><body>${tokens.sprite}${ICONS}${html}</body></html>`);
}

const browserApp = await chromium.launch();
const ctx = await browserApp.newContext({ viewport: SIZE, deviceScaleFactor: 1, colorScheme: 'dark' });
const page = await ctx.newPage();
const shot = (name) => page.screenshot({ path: join(dir, `${name}.png`) });

await directionA(page);
await shot('a-onglet');
await directionB(page);
await shot('b-panneau');
await open(page, 'agents');
const backdrop = (await page.screenshot()).toString('base64');
await directionC(page, backdrop);
await shot('c-fenetre');
await page.setViewportSize({ width: 1440, height: 900 });
await states(page);
await page.screenshot({ path: join(dir, 'etats.png'), fullPage: true });
// Its widths, in French and in English.
const T = {
  fr: {
    back: 'Retour', fwd: 'Avancer', reload: 'Recharger', widths: 'Largeur de la page', full: 'Pleine largeur', tablet: 'Tablette', mobile: 'Mobile',
    shrunk: (w) => `Rétréci à ${w} px`, detach: 'Ouvrir dans une fenêtre à part', system: 'Ouvrir dans le navigateur du système',
    foot: 'Lancement <b>web</b> prêt en 2,7 s', apart: 'Page à part : ni Escouade, ni tes cookies',
    tip: (w) => `La zone est plus étroite que 768 px : la page y est rétrécie à ${w} px.`, page: 'Page',
  },
  en: {
    back: 'Back', fwd: 'Forward', reload: 'Reload', widths: 'Page width', full: 'Full width', tablet: 'Tablet', mobile: 'Mobile',
    shrunk: (w) => `Narrowed to ${w} px`, detach: 'Open in a separate window', system: 'Open in the system browser',
    foot: 'Launch <b>web</b> ready in 2.7 s', apart: 'A page apart: neither Escouade nor your cookies',
    tip: (w) => `The area is narrower than 768 px: the page is narrowed to ${w} px.`, page: 'Page',
  },
};
const RCSS = `
.bw-modes { display:inline-flex; gap:2px; padding:2px; border-radius:8px; background:rgba(var(--fg-rgb), .035); flex:none; }
.bw-modes button { display:inline-grid; place-items:center; width:26px; height:24px; border-radius:6px; color:var(--muted); }
.bw-modes button svg { width:14px; height:14px; }
.bw-modes button[aria-checked=true] { background:var(--sel); color:var(--text); }
.bw-width { flex:none; margin:0 4px; color:var(--muted); font-size:11.5px; white-space:nowrap; font-feature-settings:'tnum'; }
.bw-width.shrunk { color:var(--wait); }
.bw-view.device { background:var(--well); display:flex; justify-content:center; }
.bw-view.device > .dev { height:100%; overflow:hidden; border-inline:1px solid var(--line-2); background:#fff; }
.bw-tip { position:absolute; z-index:5; padding:7px 10px; border-radius:8px; background:var(--raise); color:var(--soft); font-size:12px; max-width:280px; box-shadow:0 0 0 1px var(--line-2), 0 10px 26px -8px rgba(0,0,0,.6); }
.site.phone header { gap:14px; padding:0 14px; } .site.phone main { padding:16px 14px; } .site.phone .search input { max-width:none; }
.site.phone td.m { display:none; } .site.phone th:nth-child(2) { display:none; }
`;
const rbar = (L, mode, label, small) => small ? `<div class="bw-bar">
  <button class="ib" aria-label="${L.back}"><svg><use href="#b-back"/></svg></button>
  <button class="ib" aria-label="${L.reload}"><svg><use href="#b-reload"/></svg></button>
  <div class="bw-addr"><i class="dot"></i><span class="o">localhost:41731</span><span class="pth">/users?search=marie@</span></div>
  ${label}
  <button class="bw-dt"><svg><use href="#b-tools"/></svg></button>
  <button class="ib" aria-label="…"><svg><use href="#i-more"/></svg></button>
</div>` : `<div class="bw-bar">
  <button class="ib" aria-label="${L.back}"><svg><use href="#b-back"/></svg></button>
  <button class="ib" aria-label="${L.fwd}" disabled><svg><use href="#b-fwd"/></svg></button>
  <button class="ib" aria-label="${L.reload}"><svg><use href="#b-reload"/></svg></button>
  <div class="bw-addr"><i class="dot"></i><span class="o">localhost:41731</span><span class="pth">/users?search=marie@</span></div>
  <span class="bw-owner"><svg class="i"><use href="#i-branch"/></svg>pagination-users <span class="k">· web</span></span>
  <span class="bw-sep"></span>
  <div class="bw-modes" role="radiogroup" aria-label="${L.widths}">
    <button role="radio" aria-checked="${mode === 'full'}" aria-label="${L.full}"><svg><use href="#b-monitor"/></svg></button>
    <button role="radio" aria-checked="${mode === 'tablet'}" aria-label="${L.tablet}"><svg><use href="#b-tablet"/></svg></button>
    <button role="radio" aria-checked="${mode === 'mobile'}" aria-label="${L.mobile}"><svg><use href="#b-phone"/></svg></button>
  </div>
  ${label}
  <button class="bw-dt"><svg><use href="#b-tools"/></svg>Devtools <kbd>F12</kbd></button>
  <button class="ib" aria-label="${L.detach}"><svg><use href="#b-expand"/></svg></button>
  <button class="ib" aria-label="${L.system}"><svg><use href="#b-ext"/></svg></button>
</div>`;

async function widths(page, mode, lang, small) {
  const L = T[lang];
  await page.setViewportSize(small ? { width: 1028, height: 779 } : SIZE);
  await page.goto('about:blank');
  await page.goto(`${app}${lang === 'en' ? '?lang=en' : ''}#agents`);
  await page.waitForTimeout(400);
  await page.evaluate(([css, icons]) => {
    const st = document.createElement('style');
    st.textContent = css;
    document.head.append(st);
    document.body.insertAdjacentHTML('afterbegin', icons);
  }, [CSS + RCSS, ICONS]);
  // The room first, measured; the bar after, by its width.
  await page.evaluate(`(() => {
    ${launches};
    const v = document.querySelector('#view-agents'); v.classList.add('bw-a');
    const seg = document.querySelector('#vseg');
    seg.querySelectorAll('button').forEach((b) => b.setAttribute('aria-pressed', 'false'));
    seg.insertAdjacentHTML('beforeend', '<button aria-pressed="true"><svg class="i s14"><use href="#b-page"/></svg><span class="lbl">${L.page}</span></button>');
    document.querySelector('#h-launch').hidden = true;
    const side = document.querySelector('#side'); if (side) side.hidden = true;
    document.querySelector('.conv').insertAdjacentHTML('beforeend', '<div class="bw"><div class="bw-bar"></div><div class="bw-view device"><div class="dev"></div></div><div class="bw-foot"><i class="d"></i><span>${L.foot}</span><span class="sp"></span><span>${L.apart}</span></div></div>');
  })()`);
  const room = await page.evaluate(() => Math.round(document.querySelector('.bw-view').getBoundingClientRect().width));
  const device = mode === 'tablet' ? 768 : 390;
  const w = Math.min(device, room);
  const shrunk = w < device;
  const label = `<span class="bw-width${shrunk ? ' shrunk' : ''}">${shrunk ? L.shrunk(w) : `${device} px`}</span>`;
  await page.evaluate(([html, site, w, tip]) => {
    document.querySelector('.bw .bw-bar').outerHTML = html;
    const dev = document.querySelector('.bw .dev');
    dev.style.width = w + 'px';
    dev.innerHTML = site;
    if (tip) {
      const label = document.querySelector('.bw-width').getBoundingClientRect();
      document.body.insertAdjacentHTML('beforeend', `<div class="bw-tip" style="left:${label.left - 120}px;top:${label.bottom + 8}px">${tip}</div>`);
    }
  }, [rbar(L, mode, label, small), SITE(mode === 'mobile').replace('<div class="site">', `<div class="site${mode === 'mobile' ? ' phone' : ''}">`), w, shrunk ? L.tip(w) : '']);
}

for (const lang of ['fr', 'en']) {
  const end = lang === 'en' ? '-en' : '';
  await widths(page, 'tablet', lang, false);
  await shot(`a-tablette${end}`);
  await widths(page, 'mobile', lang, false);
  await shot(`a-mobile${end}`);
  await widths(page, 'tablet', lang, true);
  await shot(`a-retreci${end}`);
}
await browserApp.close();
