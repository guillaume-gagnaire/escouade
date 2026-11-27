// whats-up v1
// What's up ? — release notes window for any web app: one file, no dependency, Shadow DOM.
// <script type="module" src="/whats-up/whats-up.js"></script>
// <whats-up src="/whats-up/" app="Mon app" lang="fr"></whats-up>
// Theme: --wu-fond, --wu-texte, --wu-accent, --wu-doux, --wu-bord, --wu-rayon, --wu-police on the element or above.

/** Sortable key of a note id: `2026-11-27` < `2026-11-27-2` < … < `2026-11-27-10`. */
export const cle = (id) => {
  const suffixe = id.slice(11);
  return `${id.slice(0, 10)}#${String(/^\d+$/.test(suffixe) ? Number(suffixe) : 1).padStart(4, '0')}`;
};

const trier = (ids) => [...ids].sort((a, b) => (cle(a) < cle(b) ? -1 : cle(a) > cle(b) ? 1 : 0));

/** Notes to show, oldest first: the newest alone on a first visit, else every note newer than the last one seen. */
export function dues(ids, vu) {
  const tries = trier(ids);
  if (!tries.length) return [];
  if (vu == null) return [tries.at(-1)];
  return tries.filter((id) => cle(id) > cle(vu));
}

const EXTERNE = /^\s*(?:[a-z][a-z0-9+.-]*:|\/\/)/i;
const urlPermise = (u) => /^\s*data:/i.test(u) || !EXTERNE.test(u);

/** A note's CSS without @import nor external url(): a note never loads anything from elsewhere. */
export function filtrerCss(css) {
  return css
    .replace(/@import[^;]*;/gi, '')
    .replace(/url\(\s*(['"]?)([^'")]*)\1\s*\)/gi, (m, _q, u) => (urlPermise(u) ? m : 'none'));
}

const TEXTES = {
  fr: { marque: (app) => `Nouveautés de ${app}`, precedent: 'Précédent', suivant: 'Suivant', fin: 'C’est parti', fermer: 'Fermer', ecrans: 'Écrans', position: (i, n) => `${i} sur ${n}` },
  en: { marque: (app) => `What's new in ${app}`, precedent: 'Previous', suivant: 'Next', fin: "Let's go", fermer: 'Close', ecrans: 'Screens', position: (i, n) => `${i} of ${n}` },
};

const BALISES_SCENE = new Set('div span p b i strong em ul ol li br svg path line circle rect polyline polygon g'.split(' '));
const BALISES_TEXTE = new Set(['b', 'i', 'strong', 'em', 'br']);
const ATTRIBUTS = new Set('class style aria-hidden d viewbox x y x1 y1 x2 y2 cx cy r rx ry width height points fill stroke stroke-width stroke-linecap stroke-linejoin'.split(' '));

// Notes are written by our own writer and reviewed before commit: this is a safety net, not a security boundary.
function nettoyer(html, balises) {
  const t = document.createElement('template');
  t.innerHTML = String(html ?? '');
  const parcourir = (parent) => {
    for (const el of [...parent.children]) {
      if (!balises.has(el.localName)) { el.remove(); continue; }
      for (const a of [...el.attributes]) {
        const nom = a.name.toLowerCase();
        const styleExterne = nom === 'style' && /url\s*\(/i.test(a.value) && filtrerCss(a.value) !== a.value;
        if (!ATTRIBUTS.has(nom) || styleExterne) el.removeAttribute(a.name);
      }
      parcourir(el);
    }
  };
  parcourir(t.content);
  return t.content;
}

const STYLE = `
:host { all: initial; }
.voile { position: fixed; inset: 0; z-index: 2147483000; display: grid; place-items: center; padding: 16px;
  background: color-mix(in srgb, var(--wu-texte, #1a1a1a) 40%, transparent); }
.fenetre { box-sizing: border-box; width: min(560px, 100%); max-height: calc(100vh - 32px); overflow: auto; padding: 20px;
  display: grid; grid-template-columns: minmax(0, 1fr); gap: 12px; background: var(--wu-fond, #fff); color: var(--wu-texte, #1a1a1a);
  border: 1px solid var(--wu-bord, #e2e2e2); border-radius: var(--wu-rayon, 12px);
  font: 15px/1.5 var(--wu-police, system-ui, -apple-system, 'Segoe UI', sans-serif);
  box-shadow: 0 24px 64px color-mix(in srgb, var(--wu-texte, #000) 25%, transparent); }
.entete { display: flex; align-items: center; gap: 8px; margin: 0; font-size: 13px; color: var(--wu-doux, #666); }
.marque { font-weight: 600; color: var(--wu-accent, #4f46e5); }
.date { flex: 1; }
.fermer { all: unset; cursor: pointer; width: 28px; height: 28px; display: grid; place-items: center; border-radius: 50%; font-size: 18px; color: var(--wu-doux, #666); }
.scene { position: relative; box-sizing: border-box; height: 340px; overflow: hidden; display: grid; grid-template-columns: minmax(0, 1fr); place-items: center; padding: 16px;
  border: 1px solid var(--wu-bord, #e2e2e2); border-radius: calc(var(--wu-rayon, 12px) * .75);
  background: color-mix(in srgb, var(--wu-accent, #4f46e5) 6%, var(--wu-fond, #fff)); }
.scene > * { max-width: 100%; min-width: 0; }
.etiquette { font-size: 12px; font-weight: 600; letter-spacing: .02em; text-transform: uppercase; color: var(--wu-doux, #666); }
h2 { margin: 0; font-size: 20px; line-height: 1.3; }
.texte { margin: 0; color: var(--wu-texte, #1a1a1a); }
.pied { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
.points { flex: 1; display: flex; gap: 6px; }
.points button { all: unset; cursor: pointer; width: 8px; height: 8px; border-radius: 50%; background: var(--wu-bord, #d4d4d4); }
.points button[aria-current] { background: var(--wu-accent, #4f46e5); }
.bouton { all: unset; cursor: pointer; padding: 8px 14px; border-radius: calc(var(--wu-rayon, 12px) * .6); font-weight: 600; font-size: 14px;
  border: 1px solid var(--wu-bord, #d4d4d4); color: var(--wu-texte, #1a1a1a); }
.bouton[disabled] { opacity: .4; cursor: default; }
.principal { background: var(--wu-accent, #4f46e5); border-color: var(--wu-accent, #4f46e5); color: var(--wu-fond, #fff); }
button:focus-visible { outline: 2px solid var(--wu-accent, #4f46e5); outline-offset: 2px; }
@media (prefers-reduced-motion: reduce) { *, *::before, *::after { animation: none !important; transition: none !important; } }
`;

const Base = typeof HTMLElement === 'undefined' ? class {} : HTMLElement;

class WhatsUp extends Base {
  connectedCallback() {
    if (this.shadowRoot) return;
    this.attachShadow({ mode: 'open' });
    const param = new URLSearchParams(location.search).get('whats-up');
    this.charger(param === 'apercu' || param === 'revoir').catch(() => this.fermer(false));
  }

  /** Reopens the newest note (a « What's new » menu entry); writes nothing on closing. */
  ouvrir() {
    return this.charger(true).catch(() => this.fermer(false));
  }

  get cleStockage() {
    return `whats-up:${this.getAttribute('app') || ''}`;
  }

  lireVu() {
    if (this.hasAttribute('vu')) return this.getAttribute('vu');
    try { return localStorage.getItem(this.cleStockage); } catch { return null; }
  }

  async charger(revoir) {
    const src = (this.getAttribute('src') || '/whats-up/').replace(/\/?$/, '/');
    const index = await (await fetch(`${src}index.json`, { cache: 'no-cache' })).json();
    const ids = Array.isArray(index?.ids) ? index.ids : [];
    const aMontrer = revoir ? dues(ids, null) : dues(ids, this.lireVu());
    if (!aMontrer.length) return;
    const fiches = await Promise.all(aMontrer.map(async (id) => {
      const fiche = await (await fetch(`${src}${id}.json`)).json();
      const css = await fetch(`${src}${id}.css`).then((r) => (r.ok ? r.text() : ''), () => '');
      return { ...fiche, id, css: filtrerCss(css) };
    }));
    this.ecrans = fiches.flatMap((fiche) => (fiche.ecrans || []).map((ecran) => ({ fiche, ecran })));
    if (!this.ecrans.length) return;
    this.vuAEcrire = revoir ? null : aMontrer.at(-1);
    this.retour = document.activeElement;
    this.monter(fiches);
    this.aller(0);
  }

  monter(fiches) {
    const t = TEXTES[this.getAttribute('lang')] || TEXTES.fr;
    this.t = t;
    const racine = this.shadowRoot;
    // The notes' CSS goes in as text only: re-serializing it (innerHTML +=) would let a `</style>` in it out.
    const style = document.createElement('style');
    style.textContent = STYLE + fiches.map((f) => f.css).join('\n');
    const gabarit = document.createElement('template');
    gabarit.innerHTML = `
      <div class="voile">
        <div class="fenetre" role="dialog" aria-modal="true" aria-labelledby="wu-titre">
          <p class="entete"><span class="marque"></span><span class="date"></span>
            <button type="button" class="fermer">×</button></p>
          <div class="scene" aria-hidden="true"></div>
          <span class="etiquette"></span>
          <h2 id="wu-titre"></h2>
          <p class="texte"></p>
          <div class="pied">
            <div class="points" role="group"></div>
            <button type="button" class="bouton precedent"></button>
            <button type="button" class="bouton principal suivant"></button>
          </div>
        </div>
      </div>`;
    racine.replaceChildren(style, gabarit.content);
    const $ = (s) => racine.querySelector(s);
    $('.marque').textContent = `✦ ${t.marque(this.getAttribute('app') || '')}`;
    $('.fermer').setAttribute('aria-label', t.fermer);
    $('.points').setAttribute('aria-label', t.ecrans);
    $('.precedent').textContent = t.precedent;
    $('.fermer').onclick = () => this.fermer(true);
    $('.precedent').onclick = () => this.aller(this.i - 1);
    $('.suivant').onclick = () => (this.i === this.ecrans.length - 1 ? this.fermer(true) : this.aller(this.i + 1));
    $('.voile').onclick = (e) => { if (e.target === e.currentTarget) this.fermer(true); };
    $('.fenetre').onkeydown = (e) => this.touche(e);
    this.ecrans.forEach(({ ecran }, k) => {
      const b = document.createElement('button');
      b.type = 'button';
      b.setAttribute('aria-label', `${k + 1}. ${ecran.etiquette}`);
      b.onclick = () => this.aller(k);
      $('.points').append(b);
    });
  }

  aller(k) {
    const n = this.ecrans.length;
    this.i = Math.max(0, Math.min(n - 1, k));
    const { fiche, ecran } = this.ecrans[this.i];
    const $ = (s) => this.shadowRoot.querySelector(s);
    const lang = this.getAttribute('lang') === 'en' ? 'en' : 'fr';
    $('.date').textContent = new Date(`${fiche.id.slice(0, 10)}T12:00:00Z`).toLocaleDateString(lang, { day: 'numeric', month: 'long', year: 'numeric', timeZone: 'UTC' });
    // Replaced at each screen, so its animations play again.
    $('.scene').replaceChildren(nettoyer(ecran.scene, BALISES_SCENE));
    $('.etiquette').textContent = `${this.t.position(this.i + 1, n)} · ${ecran.etiquette}`;
    $('h2').textContent = ecran.titre;
    $('.texte').replaceChildren(nettoyer(ecran.texte, BALISES_TEXTE));
    $('.precedent').disabled = this.i === 0;
    $('.suivant').textContent = this.i === n - 1 ? this.t.fin : this.t.suivant;
    $('.points').hidden = n < 2;
    [...$('.points').children].forEach((b, j) => (j === this.i ? b.setAttribute('aria-current', 'step') : b.removeAttribute('aria-current')));
    $('.suivant').focus();
  }

  touche(e) {
    if (e.key === 'Escape') { e.preventDefault(); this.fermer(true); }
    else if (e.key === 'ArrowRight') this.aller(this.i + 1);
    else if (e.key === 'ArrowLeft') this.aller(this.i - 1);
    else if (e.key === 'Tab') {
      // Focus stays in the window while it is open.
      const f = [...this.shadowRoot.querySelectorAll('button:not([disabled]):not([hidden])')].filter((b) => !b.closest('[hidden]'));
      const actif = this.shadowRoot.activeElement;
      const j = f.indexOf(actif);
      const suivant = e.shiftKey ? (j <= 0 ? f.at(-1) : f[j - 1]) : (j === f.length - 1 ? f[0] : f[j + 1]);
      e.preventDefault();
      suivant?.focus();
    }
  }

  fermer(parUtilisateur) {
    if (this.shadowRoot) this.shadowRoot.innerHTML = '';
    if (!parUtilisateur) return;
    const p = new URLSearchParams(location.search);
    if (p.has('whats-up')) {
      p.delete('whats-up');
      const s = p.toString();
      history.replaceState(history.state, '', `${location.pathname}${s ? `?${s}` : ''}${location.hash}`);
    }
    if (this.vuAEcrire) {
      try { localStorage.setItem(this.cleStockage, this.vuAEcrire); } catch { /* private mode: seen for this page only */ }
      this.dispatchEvent(new CustomEvent('whats-up:vu', { detail: { id: this.vuAEcrire }, bubbles: true, composed: true }));
      this.vuAEcrire = null;
    }
    if (this.retour?.focus) this.retour.focus();
  }
}

if (typeof customElements !== 'undefined' && !customElements.get('whats-up')) customElements.define('whats-up', WhatsUp);
