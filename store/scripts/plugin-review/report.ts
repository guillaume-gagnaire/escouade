import type { Attested } from './attest';
import type { BenchResult } from './bench';
import type { Grown } from './diff';
import type { TagCheck } from './download';
import { KINDS, type Lead, type LeadKind } from './leads';

// The review's report (docs/STORE.md §4.2, step 6), in French, the user's language: written outside the repository
// (`~/.escouade-review/<id>/<version>.md`), since it may quote a third party's code. What the script found comes
// first; the section « Constats de la revue » is the reader's (`/review-plugin`), who also settles the verdict. A short
// public summary is written beside it, to go with the signature (STORE-8).

export type Verdict = 'Vérifier' | 'Demander des changements' | 'Refuser';

export interface ReviewData {
  id: string;
  version: string;
  date: string;
  store: string;
  /** Set when a fingerprint or an identity did not hold: nothing else was read. */
  integrity?: string;
  repository?: string;
  tag?: string;
  commit?: string;
  versionSha256?: string;
  archive?: { sha256: string; size: number };
  tagCheck?: TagCheck;
  /** The folder the plugin was unpacked in, and the review's own. */
  folder?: string;
  work?: string;
  files?: number;
  refusedEntries?: string[];
  manifest?: Record<string, unknown> | null;
  binaries?: { platform: string; sha256: string; attested: Attested }[];
  leads?: Lead[];
  base?: { version: string; grown: Grown[]; files: number; added: number; removed: number; diff: string } | null;
  bench?: BenchResult;
}

const PERMISSION_WORDS: Record<string, (v: unknown) => string> = {
  events: (v) => `reçoit les faits ${list(v)}`,
  network: (v) => `se connecte à ${list(v)}`,
  secrets: (v) => `garde les secrets ${list(v)}`,
  commands: (v) => `lance les commandes ${list(v)}`,
  decide: () => 'décide des autorisations des agents',
};

function list(v: unknown): string {
  const a = Array.isArray(v) ? v.map((x) => `\`${String(x)}\``) : [];
  return a.length ? a.join(', ') : '(aucun)';
}

/** What the manifest asks for, one line each, in words. */
export function permissionLines(manifest: Record<string, unknown> | null | undefined): string[] {
  const p = (manifest?.permissions ?? {}) as Record<string, unknown>;
  const out = Object.entries(p)
    .filter(([, v]) => (Array.isArray(v) ? v.length > 0 : v === true))
    .map(([k, v]) => (PERMISSION_WORDS[k] ? PERMISSION_WORDS[k]!(v) : `\`${k}\` : ${JSON.stringify(v)}`));
  if (manifest?.run) out.push('lance un programme');
  else if (manifest) out.push('déclaratif : aucun programme');
  const contributes = Object.keys((manifest?.contributes ?? {}) as object);
  if (contributes.length) out.push(`ajoute ${contributes.map((c) => `\`${c}\``).join(', ')}`);
  return out;
}

/** What keeps the version from « Vérifier » whatever the code says; empty when the reading alone decides. */
export function blockers(d: ReviewData): string[] {
  if (d.integrity) return [d.integrity];
  const out: string[] = [];
  if (d.tagCheck?.state === 'moved') out.push(`l’étiquette ${d.tag} pointe désormais sur ${d.tagCheck.commit}, pas sur le commit relu`);
  if (d.tagCheck?.state === 'gone') out.push(`l’étiquette ${d.tag} n’existe plus sur le dépôt`);
  if (d.refusedEntries?.length) out.push(`l’archive a des entrées refusées (${d.refusedEntries.length})`);
  for (const b of d.binaries ?? [])
    if (b.attested.state !== 'verified') out.push(`le binaire ${b.platform} n’a pas d’attestation vérifiée pour ce commit`);
  if (d.bench?.played && !d.bench.passed) out.push('le banc de conformité échoue');
  return out;
}

/** The verdict the script proposes: the reader settles it. */
export function proposed(d: ReviewData): { verdict: Verdict | null; why: string } {
  if (d.integrity) return { verdict: 'Refuser', why: `ce qui serait lu n’est pas ce que le store sert : ${d.integrity}` };
  const b = blockers(d);
  if (b.length) return { verdict: 'Demander des changements', why: b.join(' ; ') };
  return { verdict: null, why: 'à décider à la lecture du code : rien de mécanique ne s’y oppose' };
}

const fence = (s: string) => {
  const ticks = (s.match(/`{3,}/g) ?? []).reduce((n, t) => Math.max(n, t.length + 1), 3);
  return '`'.repeat(ticks);
};
const code = (s: string) => `\`${s.replace(/`/g, 'ʼ')}\``;

function attestedText(a: Attested): string {
  switch (a.state) {
    case 'verified':
      return `attestation vérifiée (workflow ${code(a.workflow)})`;
    case 'other-commit':
      return `**attestation d’un autre commit** (${a.commit})`;
    case 'refused':
      return `**attestation refusée** : ${a.why}`;
    case 'unchecked':
      return `**attestation non vérifiée** : ${a.why}`;
  }
}

function tagText(d: ReviewData): string {
  const t = d.tagCheck;
  if (!t) return 'non comparée';
  switch (t.state) {
    case 'same':
      return `l’étiquette ${code(d.tag ?? '')} pointe toujours sur ce commit`;
    case 'moved':
      return `**l’étiquette ${code(d.tag ?? '')} a été déplacée** sur ${t.commit}`;
    case 'gone':
      return `**l’étiquette ${code(d.tag ?? '')} n’existe plus** sur le dépôt`;
    case 'unknown':
      return `non comparée (${t.why})`;
  }
}

/** The report, in Markdown. */
export function renderReport(d: ReviewData): string {
  const out: string[] = [];
  const p = proposed(d);
  out.push(`# Revue de ${d.id}@${d.version}`, '');
  if (d.bench && !d.bench.played)
    out.push(`> **Le banc de conformité n’a pas été lancé** : ${d.bench.why}. Décide en le sachant : rien du plugin n’a tourné.`, '');
  out.push(`- Date : ${d.date}`, `- Store : ${d.store}`);
  out.push(`- Verdict proposé : **${p.verdict ?? 'à décider'}** — ${p.why}`);
  if (d.folder) out.push(`- Dossier extrait : ${code(d.folder)} (à lire, jamais à exécuter)`);
  out.push('');

  if (d.integrity) {
    out.push('## Intégrité', '', `**Refusé avant toute lecture** : ${d.integrity}.`, '');
    return out.join('\n');
  }

  out.push('## Identité', '');
  out.push(`- Dépôt : ${d.repository}`, `- Étiquette : ${code(d.tag ?? '')}, commit ${code(d.commit ?? '')} : ${tagText(d)}`);
  out.push(`- Empreinte de la version (SHA-256 de version.json) : ${code(d.versionSha256 ?? '')}`);
  out.push(`- Archive : ${code(d.archive?.sha256 ?? '')}, ${d.archive?.size} octets, empreinte vérifiée ; ${d.files} fichiers extraits`);
  for (const r of d.refusedEntries ?? []) out.push(`- **Entrée refusée** : ${r}`);
  if (d.binaries?.length)
    for (const b of d.binaries) out.push(`- Binaire ${b.platform} : ${code(b.sha256)}, empreinte vérifiée ; ${attestedText(b.attested)}`);
  else out.push('- Binaires : aucun');
  out.push('');

  out.push('## Permissions', '');
  if (!d.manifest) out.push('**Manifeste absent ou illisible.**');
  for (const l of permissionLines(d.manifest)) out.push(`- ${l}`);
  out.push('');

  if (d.base) {
    out.push(`## Depuis la v${d.base.version}`, '');
    if (d.base.grown.length) {
      out.push('Ce qui grandit :', '');
      for (const g of d.base.grown) out.push(`- **${code(g.what)}** : ${g.added}`);
    } else out.push('Aucune permission ni contribution de plus.');
    out.push(
      '',
      `${d.base.files} fichiers changés, +${d.base.added} −${d.base.removed} ; le diff complet : ${code(d.base.diff)}. Ne lire en détail que lui.`,
      '',
    );
  } else out.push('## Première revue', '', 'Aucune version vérifiée avant celle-ci : tout le code est à lire.', '');

  out.push('## Pistes', '');
  out.push('Ce que le script a repéré, pour savoir où regarder ; une piste n’est pas un constat.', '');
  const leads = d.leads ?? [];
  if (!leads.length) out.push('Aucune.');
  for (const kind of Object.keys(KINDS) as LeadKind[]) {
    const mine = leads.filter((l) => l.kind === kind);
    if (!mine.length) continue;
    out.push(`### ${KINDS[kind]} (${mine.length})`, '');
    for (const l of mine.slice(0, 60))
      out.push(`- ${code(`${l.file}${l.line ? `:${l.line}` : ''}`)} — ${l.why}${l.text ? ` : ${code(l.text)}` : ''}`);
    if (mine.length > 60) out.push(`- … et ${mine.length - 60} de plus (dans le .json)`);
    out.push('');
  }

  out.push('## Banc de conformité', '');
  const b = d.bench;
  if (!b) out.push('Non lancé.');
  else if (!b.played) out.push(`Non lancé : ${b.why}.`);
  else {
    out.push(`${b.passed ? 'Passé' : '**Échoué**'}, dans ${code(b.command)}.`, '');
    for (const o of b.outcomes)
      out.push(
        `- ${o.verdict === 'pass' ? '✓' : o.verdict === 'fail' ? '✗' : '–'} ${o.scenario} : ${o.said}${o.missing ? ` — manque : ${o.missing}` : ''}`,
      );
    if (b.refused.length) {
      out.push('', 'Refusé par le bac à sable (connexions, écritures) :', '');
      for (const r of b.refused) out.push(`- ${code(r)}`);
    }
    const said = b.seen.filter((s) => s.kind === 'stderr' || s.kind === 'log');
    if (said.length) {
      const body = said.map((s) => s.line ?? `[${s.kind}] ${s.message ?? ''}`).join('\n');
      const f = fence(body);
      out.push('', 'Ce que le plugin a écrit de lui-même :', '', f, body, f);
    }
    if (b.raw) {
      const f = fence(b.raw);
      out.push('', f, b.raw, f);
    }
  }
  out.push('');

  out.push('## Non vérifié', '');
  const unverified: string[] = [];
  if (b && !b.played) unverified.push('le comportement du plugin (banc non lancé)');
  for (const x of d.binaries ?? []) if (x.attested.state !== 'verified') unverified.push(`la provenance du binaire ${x.platform}`);
  if (d.tagCheck?.state === 'unknown') unverified.push('l’étiquette sur GitHub');
  unverified.push('les dépendances que le plugin téléchargerait à l’installation (elles ne sont pas dans l’archive)');
  for (const u of unverified) out.push(`- ${u}`);
  out.push(
    '',
    '## Constats de la revue',
    '',
    '_À rédiger par la lecture du code : chaque constat avec son fichier et sa ligne, ce qu’il permet, et le verdict final._',
    '',
  );
  return out.join('\n');
}

/** The public summary that goes with a signature: short, without a line of the plugin's code. */
export function renderPublic(d: ReviewData): string {
  const network = ((d.manifest?.permissions ?? {}) as { network?: unknown }).network;
  const hosts = Array.isArray(network) && network.length ? network.join(', ') : 'aucun';
  const checked = ['empreintes de l’archive et de version.json', `commit ${d.commit?.slice(0, 12)} comparé à l’étiquette ${d.tag}`];
  if (d.bench?.played) checked.push('banc de conformité sans réseau');
  if (d.binaries?.some((b) => b.attested.state === 'verified')) checked.push('provenance des binaires');
  if (d.base) checked.push(`diff depuis la v${d.base.version}`);
  return [
    `# ${d.id}@${d.version}`,
    '',
    `Relu le ${d.date.slice(0, 10)}.`,
    '',
    `- Permissions relues : ${permissionLines(d.manifest).join(' ; ') || 'aucune'}`,
    `- Réseau vers : ${hosts}`,
    `- Vérifié : ${checked.join(' ; ')}`,
    '',
  ].join('\n');
}

const PERMISSION_WORDS_EN: Record<string, (v: unknown) => string> = {
  events: (v) => `receives the facts ${list(v)}`,
  network: (v) => `connects to ${list(v)}`,
  secrets: (v) => `keeps the secrets ${list(v)}`,
  commands: (v) => `runs the commands ${list(v)}`,
  decide: () => 'decides the permissions of the agents',
};

/** The public summary in English, for the store's English pages (`<version>.public.en.md`). */
export function renderPublicEn(d: ReviewData): string {
  const network = ((d.manifest?.permissions ?? {}) as { network?: unknown }).network;
  const hosts = Array.isArray(network) && network.length ? network.join(', ') : 'none';
  const p = (d.manifest?.permissions ?? {}) as Record<string, unknown>;
  const perms = Object.entries(p)
    .filter(([, v]) => (Array.isArray(v) ? v.length > 0 : v === true))
    .map(([k, v]) => (PERMISSION_WORDS_EN[k] ? PERMISSION_WORDS_EN[k]!(v) : `\`${k}\`: ${JSON.stringify(v)}`));
  perms.push(d.manifest?.run ? 'runs a program' : 'declarative: no program');
  const checked = ['fingerprints of the archive and of version.json', `commit ${d.commit?.slice(0, 12)} compared to the tag ${d.tag}`];
  if (d.bench?.played) checked.push('conformance bench without network');
  if (d.binaries?.some((b) => b.attested.state === 'verified')) checked.push('provenance of the binaries');
  if (d.base) checked.push(`diff since v${d.base.version}`);
  return [
    `# ${d.id}@${d.version}`,
    '',
    `Reviewed on ${d.date.slice(0, 10)}.`,
    '',
    `- Permissions read: ${perms.join('; ')}`,
    `- Network to: ${hosts}`,
    `- Checked: ${checked.join('; ')}`,
    '',
  ].join('\n');
}
