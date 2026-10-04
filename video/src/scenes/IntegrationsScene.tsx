import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { STATUS, TABS } from '../data';
import type { Ticket } from '../ui/Board';
import { PointerPath } from '../ui/Cursor';
import { ImportModal, IntegrationsTab, JiraIssue, type Issue } from '../ui/Integrations';
import { SettingsShell } from '../ui/Settings';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, camPath, Stage, Title } from '../ui/Stage';
import { AppToast } from '../ui/Toast';
import { AGENT_CSV, AGENT_LOGIN, boardAgents, BoardView, CSV, DONE, LOGIN, SEARCH } from './boardCommon';

const ALL: Issue[] = [
  { key: 'ATL-1312', title: "Limiter le débit de l'API publique", kind: 'Story', meta: ['Haute', 'Hugo R.', 'À faire'], criteria: 2 },
  { key: 'ATL-1305', title: "Documenter l'API des factures", kind: 'Tâche', meta: ['Basse', 'Hugo R.', 'À faire'] },
  { key: 'ATL-1294', title: 'Arrondi faux sur les factures en devises', kind: 'Bug', meta: ['Critique', 'Léa M.', 'À faire'], criteria: 2 },
  {
    key: 'ATL-1291',
    title: 'Relancer les factures impayées par e-mail',
    kind: 'Story',
    meta: ['Moyenne', 'Léa M.', 'À faire'],
    criteria: 3,
  },
  { key: 'ATL-1287', title: 'Exporter les factures en PDF', kind: 'Story', meta: ['Haute', 'Léa M.', 'À faire'], criteria: 3 },
  {
    key: 'ATL-1270',
    title: 'Numérotation continue des factures',
    kind: 'Story',
    meta: ['Haute', 'Léa M.', 'En cours'],
    criteria: 2,
    imported: true,
  },
];
const QUERY = 'factures';
const SEARCHED = ALL.filter((i) => /factures/.test(i.title));
const MINE = SEARCHED.filter((i) => i.meta.includes('Léa M.'));
/** Ticked in this order; the one already imported cannot be. */
const PICKS = MINE.filter((i) => !i.imported);
const SOURCE = 'ATL — Atlas';

const ROUNDING = ['Montants arrondis au centime, devise par devise', 'Tests verts'];
const ROUNDING_DONE = ['Arrondi au centime par devise', 'Tests des devises'];
/** The tickets imported from Jira, in the order of the list: they take the board's next keys. */
const IMPORTED: Ticket[] = [
  {
    key: 'DEM-7',
    title: 'Arrondi faux sur les factures en devises',
    column: 'todo',
    loop: [1, 5],
    criteria: ROUNDING.map((text) => ({ text, ok: false })),
    external: { service: 'jira', key: 'ATL-1294' },
  },
  {
    key: 'DEM-8',
    title: 'Relancer les factures impayées par e-mail',
    column: 'todo',
    loop: [1, 5],
    criteria: ['Relance à J+7, puis à J+15', 'Lien de paiement dans l’e-mail', 'Tests verts'].map((text) => ({ text, ok: false })),
    external: { service: 'jira', key: 'ATL-1291' },
  },
  {
    key: 'DEM-9',
    title: 'Exporter les factures en PDF',
    column: 'todo',
    loop: [1, 5],
    criteria: ['Mise en page de la facture', 'Lien de téléchargement', 'Tests verts'].map((text) => ({ text, ok: false })),
    external: { service: 'jira', key: 'ATL-1287' },
  },
];
const LABELLED: Ticket = {
  key: 'DEM-10',
  title: 'Alerte quand un paiement échoue',
  column: 'todo',
  loop: [1, 5],
  criteria: ['Alerte dans la minute', 'Montant et client dans l’alerte', 'Tests verts'].map((text) => ({ text, ok: false })),
  external: { service: 'jira', key: 'ATL-1318' },
};
/** As the app names them: key and title, cut at 40 characters on a word. */
const AGENT_ROUNDING = 'dem-7-arrondi-faux-sur-les-factures-en';
const AGENT_REMIND = 'dem-8-relancer-les-factures-impayees-par';

/** The settings' body moves up this far to show the sources and the status mapping. */
const SCROLL = 236;
/** Agents in parallel on the board, as the previous scene left it. */
const PARALLEL = 3;

/** A Jira account connected and its project linked; tickets imported; their status and comments kept up to date; a label imports one alone. */
export const IntegrationsScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const w = (line: string, word: string) => c.word(line, word);
  const press = (at: number) => ramp(frame, at - 3, 3) - ramp(frame, at + 3, 4);
  const flash = (at: number) => ramp(frame, at, 4) - ramp(frame, at + 10, 10);

  // Connect the Jira account, link its project (its mapping comes pre-filled), save.
  const connectAt = w('connect', 'connecte') - 2;
  const connected = connectAt + 14;
  const linkAt = w('connect', 'source');
  const scrollAt = linkAt - 16;
  const saveAt = c.end('connect') + 2;
  const settings = frame < saveAt + 5 ? pop(frame, fps, 2, 18) : 0;

  // Import: search, filter, tick, import.
  const openImport = w('import', 'importer') - 2;
  const searchAt = w('import', 'cherche') - 4;
  const typedAt = searchAt + 4;
  const searched = typedAt + Math.ceil((QUERY.length / 30) * fps) + 6;
  const filterAt = w('import', 'filtre') - 2;
  const tickAt = w('import', 'coche');
  const ticks = [tickAt, tickAt + 6, tickAt + 12];
  const importAt = c.end('import') + 3;
  const imported = importAt + 6;
  const query = typed(QUERY, frame, fps, typedAt, 30);
  const issues = frame >= filterAt ? MINE : frame >= searched ? SEARCHED : ALL;
  const checked = PICKS.filter((_, i) => frame >= ticks[i]).map((i) => i.key);

  // The pilot takes the first one into the last free place at once, and Jira says « En cours » too. Ready to test, it
  // gives its place to the next one and comments; validated, Jira says « À tester » and gets the outcome. Then a
  // labelled ticket comes in alone, and waits for a place.
  const startAt = imported + 12;
  const syncIn = Math.max(c.at('sync') - 4, c.end('arrive') + 22);
  const reviewAt = w('sync', 'prêt') - 2;
  const nextAt = reviewAt + 10;
  const validateAt = Math.max(reviewAt + 18, w('sync', 'validé') - 10);
  const doneAt = validateAt + 12;
  const syncOut = c.at('auto') - 8;
  const labelAt = w('auto', 'étiquette');
  const autoAt = w('auto', 'seul') - 4;

  const rounding = (): Ticket => {
    const base = IMPORTED[0];
    if (frame >= doneAt)
      return {
        ...base,
        column: 'done',
        enter: pop(frame, fps, doneAt, 16),
        outcome: '⤵ Mergé dans main · squash',
        doneMeta: '1 boucle · 0,54 $',
        agent: { name: AGENT_ROUNDING, status: 'done' },
      };
    if (frame >= reviewAt)
      return {
        ...base,
        column: 'review',
        enter: pop(frame, fps, reviewAt, 16),
        criteria: ROUNDING.map((text) => ({ text, ok: true })),
        progress: ROUNDING_DONE,
        step: frame < validateAt + 2 ? undefined : frame < validateAt + 7 ? 'Tests…' : 'Merge…',
        agent: { name: AGENT_ROUNDING, status: 'done' },
        pressed: { approve: press(validateAt) },
      };
    if (frame >= startAt)
      return {
        ...base,
        column: 'doing',
        enter: pop(frame, fps, startAt, 16),
        activity: 'Lit src/billing/currency.ts',
        agent: { name: AGENT_ROUNDING, status: 'running' },
      };
    return { ...base, enter: pop(frame, fps, imported + 2, 16) };
  };
  const [, remind, pdf] = IMPORTED;
  const others: Ticket[] = [
    {
      ...CSV,
      column: 'doing',
      loop: [1, 5],
      criteria: CSV.criteria!.map((x, i) => ({ ...x, ok: i < 2 })),
      progress: ['Route GET /invoices.csv', 'Séparateur point-virgule'],
      activity: 'Lance npm test',
      agent: { name: AGENT_CSV, status: 'running' },
    },
    {
      ...SEARCH,
      column: 'doing',
      partial: false,
      loop: [1, 5],
      activity: 'Modifie src/search/index.ts',
      agent: { name: SEARCH.agent!.name, status: 'running' },
    },
    {
      ...LOGIN,
      column: 'done',
      outcome: '⤵ Mergé dans main · squash',
      doneMeta: '3 boucles · 1,86 $',
      agent: { name: AGENT_LOGIN, status: 'done' },
    },
    ...DONE,
  ];
  const all: Ticket[] = [
    ...(frame >= imported
      ? [
          rounding(),
          frame >= nextAt
            ? {
                ...remind,
                column: 'doing' as const,
                enter: pop(frame, fps, nextAt, 16),
                activity: 'Réfléchit',
                agent: { name: AGENT_REMIND, status: 'running' as const },
              }
            : { ...remind, enter: pop(frame, fps, imported + 6, 16) },
          { ...pdf, enter: pop(frame, fps, imported + 10, 16) },
        ]
      : []),
    ...(frame >= autoAt ? [{ ...LABELLED, enter: pop(frame, fps, autoAt, 16) }] : []),
    ...others,
  ];
  const doing = all.filter((t) => t.column === 'doing').length;
  const free = PARALLEL - doing;
  // The queue says what each ticket waits for, as the board does (waitLabel).
  const queue = all.filter((t) => t.column === 'todo');
  const tickets = all.map((t) => {
    const i = queue.indexOf(t);
    if (i < 0) return t;
    return { ...t, wait: i === 0 ? "Pris dès qu'une place se libère" : `En attente d'une place (${doing}/${PARALLEL})` };
  });

  const status = frame >= doneAt ? 'À tester' : 'En cours';
  const cam = camPath(frame, [
    [c.at('arrive') - 4, 22, { x: 470, y: 360, zoom: 1.55 }],
    [c.end('arrive') + 2, 20, { x: 750, y: 422, zoom: 1 }],
  ]);
  return (
    <Stage>
      <Title />
      <AppWindow cam={cam}>
        <Shell
          tabs={TABS}
          status={{ ...STATUS, active: 2 + doing }}
          sidebar={
            <AgentsSidebar
              board
              archived={frame >= doneAt ? 2 : 1}
              agents={boardAgents([
                { name: AGENT_CSV, tag: 'DEM-5 · boucle 1/5' },
                { name: SEARCH.agent!.name, tag: 'DEM-4 · boucle 1/5' },
                ...(frame >= startAt && frame < doneAt
                  ? [
                      {
                        name: AGENT_ROUNDING,
                        tag: frame >= reviewAt ? 'DEM-7 · à tester' : 'DEM-7 · boucle 1/5',
                        status: frame >= reviewAt ? ('done' as const) : ('running' as const),
                      },
                    ]
                  : []),
                ...(frame >= nextAt ? [{ name: AGENT_REMIND, tag: 'DEM-8 · boucle 1/5' }] : []),
              ])}
              enters={[1, 1, 1, 1, pop(frame, fps, startAt), pop(frame, fps, nextAt)]}
              badge={frame >= reviewAt && frame < doneAt ? 1 : undefined}
            />
          }
          overlay={
            <>
              <SettingsShell
                tab="integrations"
                enter={settings}
                scroll={SCROLL * ramp(frame, scrollAt, 14)}
                changed={frame >= linkAt ? ['integrations'] : []}
                savePressed={press(saveAt)}
              >
                <IntegrationsTab
                  accounts={[
                    {
                      service: 'jira',
                      label: frame >= connected ? 'lea@demo.dev · demo.atlassian.net' : undefined,
                      checking: frame >= connectAt + 2 && frame < connected,
                      pressed: press(connectAt),
                    },
                    { service: 'trello', label: '@lea' },
                    { service: 'github', label: '@lea · via gh' },
                  ]}
                  sources={[
                    ...(frame >= connected ? [['jira', frame >= linkAt ? SOURCE : null] as const] : []),
                    ['trello', null],
                    ['github', null],
                  ]}
                  sourceFlash={{ jira: flash(linkAt) }}
                  // The defaults a linked project gets: in progress until done, to test once done; comments on the last two.
                  mapping={[
                    { states: {}, comment: false },
                    { states: { jira: 'En cours' }, comment: false },
                    { states: { jira: 'En cours' }, comment: true },
                    { states: { jira: 'À tester' }, comment: true },
                  ]}
                  mappingEnter={frame >= linkAt ? pop(frame, fps, linkAt + 2, 18) : 0}
                  mappingGlow={ramp(frame, linkAt + 6, 6) - ramp(frame, saveAt - 6, 6)}
                />
              </SettingsShell>
              <ImportModal
                enter={frame >= openImport && frame < imported ? pop(frame, fps, openImport + 3, 18) : 0}
                source={SOURCE}
                issues={issues}
                checked={checked}
                query={query}
                focus={frame >= searchAt && frame < filterAt}
                searching={frame >= typedAt + 6 && frame < searched}
                filter={frame >= filterAt}
                critGlow={ramp(frame, w('import', 'critères') - 4, 8) - ramp(frame, importAt - 2, 6)}
                importPressed={press(importAt)}
              />
              <AppToast
                text="3 tickets importés depuis Jira"
                tone="ok"
                enter={frame >= imported && frame < imported + 70 ? pop(frame, fps, imported, 16) : 0}
              />
              <AppToast
                text="1 ticket importé depuis Jira dans demo-api"
                tone="ok"
                enter={frame >= autoAt ? pop(frame, fps, autoAt, 16) : 0}
              />
              <JiraIssue
                enter={frame >= syncOut ? 0 : frame >= syncIn ? pop(frame, fps, syncIn, 18) : 0}
                style={{ left: 22, top: 112 }}
                issueKey="ATL-1294"
                title="Arrondi faux sur les factures en devises"
                status={status}
                flash={Math.max(flash(syncIn + 10), flash(doneAt))}
                comments={[
                  {
                    text: (
                      <>
                        Escouade : DEM-7 est prêt à tester.
                        <Para title="Critères :" lines={ROUNDING.map((r) => `✓ ${r}`)} />
                        <Para title="Ce qui a été fait :" lines={ROUNDING_DONE.map((r) => `- ${r}`)} />
                      </>
                    ),
                    enter: pop(frame, fps, reviewAt + 4, 18),
                  },
                  { text: 'Escouade : DEM-7 est terminé — ⤵ Mergé dans main · squash', enter: pop(frame, fps, doneAt + 4, 18) },
                ]}
              />
              <JiraIssue
                enter={frame >= syncOut ? pop(frame, fps, syncOut + 2, 18) : 0}
                style={{ right: 22, top: 130 }}
                issueKey="ATL-1318"
                title="Alerte quand un paiement échoue"
                status="À faire"
                labels={[
                  { name: 'paiements', enter: 1 },
                  { name: 'claude-ready', enter: pop(frame, fps, labelAt - 2, 12) },
                ]}
                comments={[]}
              />
              <PointerPath
                keys={[
                  [connectAt - 22, 1000, 380],
                  [connectAt, 1162, 311, true],
                  [linkAt - 14, 960, 340],
                  [linkAt, 900, 292, true],
                  [saveAt - 14, 1060, 640],
                  [saveAt, 1170, 743, true],
                  [Math.max(saveAt + 6, openImport - 16), 1040, 220],
                  [openImport, 962, 81, true],
                  [searchAt - 10, 700, 250],
                  [searchAt, 600, 243, true],
                  [filterAt - 8, 380, 300],
                  [filterAt, 362, 288, true],
                  [ticks[0] - 6, 340, 360],
                  [ticks[0], 330, 381, true],
                  [ticks[1], 330, 448, true],
                  [ticks[2], 330, 515, true],
                ]}
                to={ticks[2] + 16}
              />
              <PointerPath
                keys={[
                  [importAt - 12, 1060, 660],
                  [importAt, 1131, 748, true],
                ]}
              />
              <PointerPath
                keys={[
                  [validateAt - 14, 1000, 640],
                  [validateAt, VALIDATE.x, VALIDATE.y, true],
                ]}
              />
            </>
          }
        >
          <BoardView
            tickets={tickets}
            autopilot
            places={free ? `${free} ${free > 1 ? 'places libres' : 'place libre'}` : 'Toutes les places sont prises'}
            pressedImport={press(openImport)}
          />
        </Shell>
      </AppWindow>
    </Stage>
  );
};

/** « Valider et merger » on DEM-7's card, to test. */
const VALIDATE = { x: 990, y: 444 };

/** A paragraph of a Jira comment: its title, then its lines. */
const Para: FC<{ title: string; lines: string[] }> = ({ title, lines }) => (
  <span style={{ display: 'block', marginTop: 7 }}>
    {title}
    {lines.map((l) => (
      <span key={l} style={{ display: 'block' }}>
        {l}
      </span>
    ))}
  </span>
);
