// The three directions of the « Pull requests » view (ESC-65), drawn in the real app's window by previews.spec.ts: the
// app's tokens, its global classes (.btn, .segmented, .icon-btn, .num) and its icon sprite, on the pull requests the app
// read from the fake GitHub (`project_prs`). A preview, not the view: nothing here is wired to the app.
(() => {
  const ME = 'ada';
  const icon = (name, size = 14) =>
    `<svg class="icon" width="${size}" height="${size}" aria-hidden="true" style="fill:none;stroke:currentColor;stroke-width:1.5;stroke-linecap:round;stroke-linejoin:round;flex:none"><use href="#i-${name}"/></svg>`;
  const esc = (s) => String(s ?? '').replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);

  const age = (created) => {
    const s = Math.max(0, Date.now() / 1000 - created);
    if (s < 3600) return `${Math.max(1, Math.round(s / 60))} min`;
    if (s < 86400) return `${Math.round(s / 3600)} h`;
    if (s < 86400 * 14) return `${Math.round(s / 86400)} j`;
    return `${Math.round(s / 86400 / 7)} sem`;
  };
  const tone = (p) => {
    if (p.draft) return 'dim';
    if (p.checks === 'failure' || p.mergeable === false) return 'fail';
    if (p.changesRequested || p.checks === 'pending') return 'wait';
    return 'ok';
  };
  const ciIcon = (p) => ({ failure: 'close', success: 'check', pending: 'clock', skipped: 'pr' })[p.checks ?? 'skipped'];
  const ciWord = (p) =>
    p.checks ? { failure: 'CI en échec', success: 'CI verte', pending: 'CI en cours', skipped: 'CI ignorée' }[p.checks] : 'Pas de CI';
  const ciTone = (p) => ({ failure: 'fail', success: 'ok', pending: 'wait' })[p.checks] ?? 'dim';
  const forkOf = (p) => (p.fork ? (p.headRepo ?? '').split('/')[0] : null);
  const branchOf = (p) => (p.fork ? `${forkOf(p)}:${p.head}` : p.head);
  const asked = (p) => (p.reviewers ?? []).includes(ME);

  /** What its reviews say, the most pressing first. */
  function reviewBits(p) {
    const bits = [];
    if (p.changesRequested) bits.push({ t: 'Changements demandés', s: 'Changements', k: 'wait', i: 'edit' });
    if (p.approvals) bits.push({ t: p.approvals > 1 ? `${p.approvals} approbations` : '1 approbation', s: String(p.approvals), k: 'ok', i: 'check' });
    if (p.openThreads) bits.push({ t: p.openThreads > 1 ? `${p.openThreads} fils ouverts` : '1 fil ouvert', s: String(p.openThreads), k: 'muted', i: 'read' });
    if (asked(p)) bits.push({ t: 'Ta relecture', s: 'Toi', k: 'ask', i: 'user' });
    return bits;
  }
  const chip = (b, short) => `<span class="x-chip ${b.k}" title="${esc(b.t)}">${icon(b.i, 12)}${esc(short ? b.s : b.t)}</span>`;
  const conflict = (p) => (p.mergeable === false ? `<span class="x-chip fail">${icon('swap', 12)}Conflit</span>` : '');
  const tags = (p) =>
    (p.draft ? `<span class="x-tag">Brouillon</span>` : '') +
    (p.fork ? `<span class="x-tag fork" title="Depuis le fork ${esc(p.headRepo)}">${icon('branch', 11)}fork ${esc(forkOf(p))}</span>` : '') +
    (p.agent ? `<span class="x-tag agent">${icon('agents', 11)}${esc(p.agent)}</span>` : '');

  const FILTERS = [
    ['mine', 'Les miennes'],
    ['review', 'Relecture demandée'],
    ['all', 'Toutes'],
  ];
  const filtered = (prs, f) => (f === 'mine' ? prs.filter((p) => p.author === ME) : f === 'review' ? prs.filter(asked) : prs);

  function header(prs, o) {
    const n = prs.length;
    const known = o.state !== 'loading' && o.state !== 'error';
    const seg = FILTERS.map(
      ([k, label]) =>
        `<button role="tab" aria-selected="${o.filter === k}">${label}${known ? `<span class="x-count num">${filtered(prs, k).length}</span>` : ''}</button>`,
    ).join('');
    const sub =
      o.state === 'loading' ? 'acme/demo · lecture…' : o.state === 'error' ? 'acme/demo' : `${n} ouverte${n > 1 ? 's' : ''} sur acme/demo`;
    return `<header class="x-head">
      <div class="x-title"><h1>Pull requests</h1><div class="x-crumbs"><span>demo-api</span><span aria-hidden="true">·</span><span class="num">${sub}</span></div></div>
      <div class="segmented x-filters" role="tablist" aria-label="Filtre">${seg}</div>
      ${known ? '<span class="x-read">Lu il y a 12 s</span>' : ''}
      <button class="icon-btn" aria-label="Rafraîchir">${icon('restart', 15)}</button>
    </header>`;
  }

  function stateBody(o) {
    if (o.state === 'loading')
      return `<div class="x-state"><span class="x-spin"></span><p>Lecture des PR ouvertes de acme/demo…</p></div>`;
    if (o.state === 'error')
      return `<div class="x-state"><div class="x-error" role="alert">${icon('info', 16)}<div><b>Hébergeur injoignable.</b><p>GitHub a répondu « 502 Bad Gateway ». La vue se relira toute seule dans une minute.</p></div><button class="btn">${icon('restart', 14)}Réessayer</button></div></div>`;
    if (o.state === 'empty') {
      const what =
        o.filter === 'mine'
          ? 'Aucune PR ouverte par toi (@ada).'
          : o.filter === 'review'
            ? 'Personne ne t’a demandé de relecture.'
            : 'Aucune PR ouverte sur acme/demo.';
      return `<div class="x-state"><span class="x-empty-icon">${icon('pr', 22)}</span><p class="x-empty-t">${what}</p><p>Les PR ouvertes chez GitHub, par toi ou par d'autres, s'affichent ici.</p><button class="btn ghost">${icon('pr', 14)}Toutes les PR</button></div>`;
    }
    return null;
  }

  // ---------- A: the dense list ----------
  function listA(prs, o) {
    const rows = prs
      .map((p) => {
        const bits = reviewBits(p);
        const on = p.number === o.selected;
        return `<div class="a-row ${on ? 'on' : ''} ${p.draft ? 'draft' : ''}" role="row">
          <span class="a-st ${tone(p)}">${icon(p.draft ? 'pr' : ciIcon(p), 14)}</span>
          <span class="a-num num">#${p.number}</span>
          <span class="a-title"><span class="a-t">${esc(p.title)}</span>${tags(p)}</span>
          <span class="a-rev">${conflict(p)}${bits.map((b) => chip(b, false)).join('')}</span>
          <span class="a-ci ${ciTone(p)}">${esc(ciWord(p))}</span>
          <span class="a-by">@${esc(p.author)}</span>
          <span class="a-age num">${age(p.created)}</span>
          <span class="a-act">${
            on
              ? `<button class="btn x-sm">${icon('agents', 13)}Lancer un agent</button><button class="icon-btn" aria-label="Ouvrir dans le navigateur">${icon('ext', 14)}</button>`
              : ''
          }</span>
        </div>`;
      })
      .join('');
    return `<div class="a-list" role="table">${rows}</div>`;
  }

  // ---------- B: columns by what each pull request waits for ----------
  const COLS = [
    ['fix', 'À corriger', 'var(--del)', (p) => !p.draft && (p.checks === 'failure' || p.mergeable === false)],
    ['changes', 'Changements demandés', 'var(--wait)', (p) => !p.draft && p.changesRequested],
    ['review', 'À relire', 'var(--info)', (p) => !p.draft && !p.approvals],
    ['ready', 'Prêtes à merger', 'var(--run)', () => true],
  ];
  function columnsB(prs, o) {
    const drafts = prs.filter((p) => p.draft);
    const left = prs.filter((p) => !p.draft);
    const placed = new Set();
    const cols = COLS.map(([k, label, color, test]) => {
      const mine = left.filter((p) => !placed.has(p.number) && test(p));
      mine.forEach((p) => placed.add(p.number));
      return [k, label, color, mine];
    });
    cols.push(['draft', 'Brouillons', 'var(--idle)', drafts]);
    const card = (p) => {
      const on = p.number === o.selected;
      return `<div class="b-card ${on ? 'on' : ''} ${asked(p) ? 'ask' : ''}">
        <div class="b-top num"><span>#${p.number}</span><span>·</span><span>@${esc(p.author)}</span><span>·</span><span>${age(p.created)}</span><span class="b-ci ${ciTone(p)}" title="${esc(ciWord(p))}">${icon(ciIcon(p), 13)}</span></div>
        <div class="b-title">${esc(p.title)}</div>
        <div class="b-branch">${icon('branch', 12)}<span>${esc(branchOf(p))}</span><span class="b-arrow">→</span><span>${esc(p.base)}</span></div>
        <div class="b-row">${tags(p)}${conflict(p)}${reviewBits(p)
          .map((b) => chip(b, false))
          .join('')}</div>
        ${on ? `<div class="b-act"><button class="btn x-sm">${icon('agents', 13)}Lancer un agent</button><button class="icon-btn" aria-label="Ouvrir dans le navigateur">${icon('ext', 14)}</button></div>` : ''}
      </div>`;
    };
    return `<div class="b-board">${cols
      .map(
        ([k, label, color, items]) => `<section class="b-col">
          <div class="b-head"><span class="b-ring" style="--tone:${color}"></span><span>${label}</span><span class="b-count num">${items.length}</span></div>
          <div class="b-cards">${items.length ? items.map(card).join('') : `<div class="b-empty">Aucune</div>`}</div>
        </section>`,
      )
      .join('')}</div>`;
  }

  // ---------- C: the list and the pull request read beside it ----------
  function splitC(prs, o) {
    const sel = prs.find((p) => p.number === o.selected) ?? prs[0];
    const rows = prs
      .map((p) => {
        const on = p.number === sel?.number;
        const bits = reviewBits(p);
        return `<div class="c-row ${on ? 'on' : ''}">
          <span class="a-st ${tone(p)}">${icon(p.draft ? 'pr' : ciIcon(p), 14)}</span>
          <div class="c-main">
            <div class="c-t">${esc(p.title)}</div>
            <div class="c-meta num"><span>#${p.number}</span><span>@${esc(p.author)}</span><span>${age(p.created)}</span>${tags(p)}${conflict(p)}${bits
              .map((b) => chip(b, true))
              .join('')}</div>
          </div>
        </div>`;
      })
      .join('');
    if (!sel) return '';
    const bits = reviewBits(sel);
    const fetchRef = sel.fork ? `refs/pull/${sel.number}/head` : `origin/${sel.head}`;
    const detail = `<aside class="c-detail">
      <div class="c-dh">
        <div class="c-dnum num"><span class="a-st ${tone(sel)}">${icon(sel.draft ? 'pr' : ciIcon(sel), 14)}</span>#${sel.number} · ${sel.draft ? 'brouillon' : 'ouverte'} · créée il y a ${age(sel.created)} par @${esc(sel.author)}</div>
        <h2>${esc(sel.title)}</h2>
        <div class="b-branch">${icon('branch', 12)}<span>${esc(branchOf(sel))}</span><span class="b-arrow">→</span><span>${esc(sel.base)}</span>${sel.fork ? `<span class="x-tag fork">fork ${esc(forkOf(sel))}</span>` : ''}</div>
      </div>
      <div class="c-actions">
        <button class="btn primary">${icon('agents', 14)}Lancer un agent sur cette PR</button>
        <button class="btn">${icon('ext', 14)}Ouvrir dans le navigateur</button>
        <button class="btn ghost" disabled title="Ticket suivant">${icon('spark', 14)}Faire relire par un agent</button>
      </div>
      <dl class="c-facts">
        <dt>CI</dt><dd class="${ciTone(sel)}">${icon(ciIcon(sel), 13)}${esc(ciWord(sel))}</dd>
        <dt>Relectures</dt><dd>${bits.length ? bits.map((b) => chip(b, false)).join('') : '<span class="muted">Aucune encore</span>'}</dd>
        <dt>Conflit</dt><dd>${sel.mergeable === false ? `<span class="fail">Conflit avec ${esc(sel.base)}</span>` : sel.mergeable === true ? 'Aucun' : '<span class="muted">Pas encore connu</span>'}</dd>
      </dl>
      <div class="c-launch">
        <b>${icon('agents', 13)}« Lancer un agent » fera</b>
        <p>Un worktree sur <code>${esc(sel.fork ? `pr-${sel.number}` : sel.head)}</code>, récupéré par <code>${esc(fetchRef)}</code>, et un agent dedans sur le harness du projet. Rien n'est poussé ni publié chez GitHub.</p>
      </div>
    </aside>`;
    return `<div class="c-split"><div class="c-list">${rows}</div>${detail}</div>`;
  }

  const CSS = `
  .esc65 { flex: 1; min-width: 0; min-height: 0; display: flex; flex-direction: column; }
  .esc65 .x-head { flex: none; display: flex; align-items: center; gap: 14px; min-height: 60px; padding: 0 14px 0 22px; border-bottom: 1px solid var(--line); white-space: nowrap; }
  .esc65 .x-title { display: grid; gap: 2px; margin-right: auto; min-width: 0; }
  .esc65 h1 { margin: 0; font-size: 15px; font-weight: 620; letter-spacing: -0.015em; }
  .esc65 .x-crumbs { display: flex; gap: 6px; color: var(--muted); font-size: 12px; }
  .esc65 .x-filters button { gap: 6px; }
  .esc65 .x-count { color: var(--dim); font-weight: 500; font-size: 11.5px; }
  .esc65 .x-filters button[aria-selected='true'] .x-count { color: var(--soft); }
  .esc65 .x-read { color: var(--dim); font-size: 12px; }
  .esc65 .x-chip { display: inline-flex; align-items: center; gap: 4px; height: 20px; padding: 0 7px; border-radius: 6px; font-size: 11.5px; font-weight: 520; white-space: nowrap; color: var(--soft); background: rgba(var(--fg-rgb), 0.04); box-shadow: inset 0 0 0 1px var(--line); }
  .esc65 .x-chip.ok { color: var(--run); } .esc65 .x-chip.wait { color: var(--wait); } .esc65 .x-chip.fail { color: var(--del); }
  .esc65 .x-chip.muted { color: var(--muted); }
  .esc65 .x-chip.ask { color: var(--wait); background: var(--wait-bg); box-shadow: inset 0 0 0 1px var(--wait-line); }
  .esc65 .x-tag { display: inline-flex; align-items: center; gap: 4px; height: 18px; padding: 0 6px; border-radius: 5px; font-size: 11px; font-weight: 520; color: var(--muted); box-shadow: inset 0 0 0 1px var(--line-2); white-space: nowrap; flex: none; }
  .esc65 .x-tag.fork { color: var(--info); box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--info) 35%, transparent); }
  .esc65 .x-tag.agent { color: var(--p-ink); background: var(--p-soft); box-shadow: none; }
  .esc65 .x-sm { height: 26px; padding: 0 9px; font-size: 12px; }
  .esc65 .ok { color: var(--run); } .esc65 .wait { color: var(--wait); } .esc65 .fail { color: var(--del); } .esc65 .dim, .esc65 .muted { color: var(--muted); }
  .esc65 .x-state { flex: 1; display: grid; place-content: center; justify-items: center; gap: 8px; color: var(--muted); text-align: center; padding: 40px; }
  .esc65 .x-state p { margin: 0; max-width: 46ch; }
  .esc65 .x-empty-icon { width: 44px; height: 44px; border-radius: 12px; display: grid; place-items: center; background: var(--raise); box-shadow: inset 0 0 0 1px var(--line); color: var(--muted); margin-bottom: 4px; }
  .esc65 .x-empty-t { color: var(--text); font-weight: 560; font-size: 14px; }
  .esc65 .x-state .btn { margin-top: 8px; }
  .esc65 .x-spin { width: 18px; height: 18px; border-radius: 50%; box-shadow: inset 0 0 0 2px var(--line-2); border-top: 2px solid var(--soft); }
  .esc65 .x-error { display: flex; align-items: flex-start; gap: 12px; text-align: left; max-width: 560px; padding: 14px 16px; border-radius: 10px; color: var(--del); background: color-mix(in oklch, var(--del) 7%, var(--panel)); box-shadow: inset 0 0 0 1px color-mix(in oklch, var(--del) 35%, transparent); }
  .esc65 .x-error b { color: var(--text); font-weight: 600; } .esc65 .x-error p { color: var(--soft); margin-top: 3px; }
  .esc65 .x-error .btn { flex: none; align-self: center; }
  /* A */
  .esc65 .a-list { flex: 1; min-height: 0; overflow: auto; padding: 6px 10px 12px; }
  .esc65 .a-row { display: grid; grid-template-columns: 18px 46px minmax(0, 1fr) auto 92px 80px 44px; position: relative; align-items: center; gap: 10px; height: 34px; padding: 0 10px; border-radius: 7px; }
  .esc65 .a-row + .a-row { box-shadow: 0 -1px 0 var(--line); }
  .esc65 .a-row.on { background: var(--hover); box-shadow: inset 0 0 0 1px var(--line-2); }
  .esc65 .a-row.on + .a-row { box-shadow: none; }
  .esc65 .a-row.draft .a-t { color: var(--muted); }
  .esc65 .a-st { display: grid; place-items: center; }
  .esc65 .a-st.ok { color: var(--run); } .esc65 .a-st.wait { color: var(--wait); } .esc65 .a-st.fail { color: var(--del); } .esc65 .a-st.dim { color: var(--idle); }
  .esc65 .a-num { color: var(--muted); font-size: 12px; }
  .esc65 .a-title { display: flex; align-items: center; gap: 7px; min-width: 0; }
  .esc65 .a-t { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 500; }
  .esc65 .a-rev { display: flex; gap: 5px; justify-content: flex-end; }
  .esc65 .a-ci { font-size: 12px; white-space: nowrap; }
  .esc65 .a-by { color: var(--muted); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .esc65 .a-age { color: var(--muted); font-size: 12px; text-align: right; }
  .esc65 .a-act { position: absolute; right: 6px; top: 3px; bottom: 3px; display: flex; align-items: center; gap: 4px; padding-left: 24px; background: linear-gradient(90deg, transparent, var(--panel) 22px); }
  .esc65 .a-act:empty { display: none; }
  .esc65 .a-row.on .a-act { background: linear-gradient(90deg, transparent, color-mix(in oklch, var(--panel) 92%, var(--text)) 22px); }
  /* B */
  .esc65 .b-board { flex: 1; min-height: 0; display: grid; grid-template-columns: repeat(5, minmax(184px, 1fr)); gap: 12px; padding: 14px 16px; overflow: auto; }
  .esc65 .b-col { min-height: 0; display: flex; flex-direction: column; gap: 8px; }
  .esc65 .b-head { display: flex; align-items: center; gap: 8px; height: 30px; padding: 0 6px; font-weight: 600; white-space: nowrap; }
  .esc65 .b-ring { width: 10px; height: 10px; border-radius: 50%; box-shadow: inset 0 0 0 2px var(--tone); }
  .esc65 .b-count { color: var(--dim); font-weight: 500; }
  .esc65 .b-cards { flex: 1; min-height: 0; overflow: auto; display: flex; flex-direction: column; gap: 8px; margin: 0 -4px; padding: 0 4px 4px; }
  .esc65 .b-card { display: grid; gap: 7px; padding: 11px 12px 10px; border-radius: 10px; background: var(--raise); box-shadow: inset 0 0 0 1px var(--line), 0 1px 2px rgba(0,0,0,calc(0.3 * var(--shade))); }
  .esc65 .b-card.on { box-shadow: inset 0 0 0 1px var(--line-3), 0 4px 12px -4px rgba(0,0,0,calc(0.5 * var(--shade))); }
  .esc65 .b-card.ask { background: color-mix(in oklch, var(--wait) 5%, var(--raise)); box-shadow: inset 0 0 0 1px var(--wait-line); }
  .esc65 .b-top { display: flex; align-items: center; gap: 6px; color: var(--muted); font-size: 12px; font-weight: 500; }
  .esc65 .b-ci { margin-left: auto; display: grid; }
  .esc65 .b-title { font-weight: 560; line-height: 1.35; display: -webkit-box; -webkit-box-orient: vertical; -webkit-line-clamp: 2; overflow: hidden; }
  .esc65 .b-branch { display: flex; align-items: center; gap: 5px; min-width: 0; color: var(--muted); font: 11.5px var(--mono); white-space: nowrap; overflow: hidden; }
  .esc65 .b-branch span:not(.b-arrow):not(.x-tag) { overflow: hidden; text-overflow: ellipsis; }
  .esc65 .b-arrow { color: var(--dim); }
  .esc65 .b-row { display: flex; flex-wrap: wrap; gap: 5px; }
  .esc65 .b-row:empty { display: none; }
  .esc65 .b-act { display: flex; gap: 4px; padding-top: 2px; }
  .esc65 .b-empty { display: grid; place-items: center; min-height: 70px; border-radius: 10px; color: var(--dim); font-size: 12px; box-shadow: inset 0 0 0 1px var(--line); border: 1px dashed var(--line-2); }
  /* C */
  .esc65 .c-split { flex: 1; min-height: 0; display: grid; grid-template-columns: minmax(380px, 46%) minmax(0, 1fr); }
  .esc65 .c-list { min-height: 0; overflow: auto; padding: 6px 8px 12px; border-right: 1px solid var(--line); }
  .esc65 .c-row { display: grid; grid-template-columns: 18px minmax(0, 1fr); gap: 10px; align-items: start; padding: 8px 10px; border-radius: 8px; }
  .esc65 .c-row .a-st { padding-top: 2px; }
  .esc65 .c-row.on { background: var(--hover); box-shadow: inset 0 0 0 1px var(--line-2); }
  .esc65 .c-main { display: grid; gap: 4px; min-width: 0; }
  .esc65 .c-t { font-weight: 520; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .esc65 .c-meta { display: flex; align-items: center; gap: 7px; color: var(--muted); font-size: 12px; min-width: 0; overflow: hidden; }
  .esc65 .c-meta .x-chip { height: 18px; padding: 0 5px; }
  .esc65 .c-detail { min-height: 0; overflow: auto; padding: 22px 26px; display: grid; align-content: start; gap: 20px; }
  .esc65 .c-dh { display: grid; gap: 8px; }
  .esc65 .c-dnum { display: flex; align-items: center; gap: 7px; color: var(--muted); font-size: 12px; }
  .esc65 .c-detail h2 { margin: 0; font-size: 17px; font-weight: 620; letter-spacing: -0.015em; line-height: 1.3; }
  .esc65 .c-actions { display: flex; flex-wrap: wrap; gap: 8px; }
  .esc65 .c-facts { margin: 0; display: grid; grid-template-columns: 96px 1fr; gap: 10px 14px; align-items: center; padding: 14px 0; border-top: 1px solid var(--line); border-bottom: 1px solid var(--line); }
  .esc65 .c-facts dt { color: var(--muted); font-size: 12px; }
  .esc65 .c-facts dd { margin: 0; display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .esc65 .c-launch { display: grid; gap: 6px; color: var(--soft); }
  .esc65 .c-launch b { display: flex; align-items: center; gap: 6px; font-weight: 600; color: var(--text); }
  .esc65 .c-launch p { margin: 0; max-width: 64ch; color: var(--muted); }
  .esc65 .c-launch code { font: 12px var(--mono); color: var(--soft); background: var(--well); padding: 1px 5px; border-radius: 4px; box-shadow: inset 0 0 0 1px var(--line); }
  `;

  /** The sidebar's views, with « PR » between Kanban and Docs: shown as on. */
  function sidebar(count) {
    const seg = document.querySelector('.side .seg');
    if (!seg) return;
    let pr = seg.querySelector('[data-esc65]');
    const buttons = [...seg.querySelectorAll('button')];
    if (!pr) {
      const kanban = buttons[1];
      pr = kanban.cloneNode(false);
      pr.setAttribute('data-esc65', '');
      kanban.after(pr);
    }
    pr.innerHTML = `${icon('pr', 14)}PR`;
    seg.style.gridTemplateColumns = '1.25fr 1.2fr 0.8fr 1fr';
    seg.querySelectorAll('button').forEach((b) => { b.style.gap = '5px'; b.style.whiteSpace = 'nowrap'; });
    seg.querySelectorAll('button').forEach((b) => {
      const on = b === pr;
      b.classList.toggle('on', on);
      b.setAttribute('aria-pressed', String(on));
    });
  }

  window.__esc65 = (prs, o) => {
    let style = document.getElementById('esc65-style');
    if (!style) {
      style = document.head.appendChild(document.createElement('style'));
      style.id = 'esc65-style';
      style.textContent = CSS;
    }
    const all = o.state === 'one' ? prs.slice(0, 1) : o.state ? (o.state === 'empty' ? [] : prs) : prs;
    sidebar(o.state === 'loading' || o.state === 'error' ? '' : all.length);
    const shown = filtered(all, o.filter);
    const body =
      stateBody({ ...o, state: o.state === 'one' ? null : o.state }) ??
      (shown.length === 0
        ? stateBody({ ...o, state: 'empty' })
        : o.dir === 'a'
          ? listA(shown, o)
          : o.dir === 'b'
            ? columnsB(shown, o)
            : splitC(shown, o));
    const work = document.querySelector('.work');
    work.innerHTML = `<main class="esc65">${header(all, o)}${body}</main>`;
  };
})();
