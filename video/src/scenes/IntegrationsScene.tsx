import type { FC } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp, typed } from '../anim';
import { useCues } from '../cues';
import { TABS, useDemo } from '../data';
import { useFmt, type Tr } from '../lang';
import type { Ticket } from '../ui/Board';
import { PointerPath } from '../ui/Cursor';
import { ImportModal, IntegrationsTab, JiraIssue, type Issue } from '../ui/Integrations';
import { SettingsShell } from '../ui/Settings';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { AppWindow, camPath, Stage, Title } from '../ui/Stage';
import { AppToast } from '../ui/Toast';
import { BoardView, placesText, ticketAgent, useBoard } from './boardCommon';

const SOURCE = 'ATL — Atlas';

/** The Jira issues and the tickets they become, in the language of the picture. */
function dataOf(tr: Tr) {
  const todo = tr('À faire', 'To Do');
  const me = tr('Léa M.', 'Lea M.');
  const [high, low, medium] = [tr('Haute', 'High'), tr('Basse', 'Low'), tr('Moyenne', 'Medium')];
  const titles = {
    rate: tr("Limiter le débit de l'API publique", 'Rate-limit the public API'),
    docs: tr("Documenter l'API des factures", 'Document the invoices API'),
    rounding: tr('Arrondi faux sur les factures en devises', 'Wrong rounding on foreign-currency invoices'),
    remind: tr('Relancer les factures impayées par e-mail', 'Send email reminders for unpaid invoices'),
    pdf: tr('Exporter les factures en PDF', 'Export invoices as PDF'),
    numbering: tr('Numérotation continue des factures', 'Continuous invoice numbering'),
    alert: tr('Alerte quand un paiement échoue', 'Alert when a payment fails'),
  };
  const ALL: Issue[] = [
    { key: 'ATL-1312', title: titles.rate, kind: 'Story', meta: [high, 'Hugo R.', todo], criteria: 2 },
    { key: 'ATL-1305', title: titles.docs, kind: tr('Tâche', 'Task') as Issue['kind'], meta: [low, 'Hugo R.', todo] },
    { key: 'ATL-1294', title: titles.rounding, kind: 'Bug', meta: [tr('Critique', 'Critical'), me, todo], criteria: 2 },
    { key: 'ATL-1291', title: titles.remind, kind: 'Story', meta: [medium, me, todo], criteria: 3 },
    { key: 'ATL-1287', title: titles.pdf, kind: 'Story', meta: [high, me, todo], criteria: 3 },
    {
      key: 'ATL-1270',
      title: titles.numbering,
      kind: 'Story',
      meta: [high, me, tr('En cours', 'In Progress')],
      criteria: 2,
      imported: true,
    },
  ];
  const QUERY = tr('factures', 'invoice');
  const SEARCHED = ALL.filter((i) => i.title.toLowerCase().includes(QUERY));
  const MINE = SEARCHED.filter((i) => i.meta.includes(me));
  /** Ticked in this order; the one already imported cannot be. */
  const PICKS = MINE.filter((i) => !i.imported);

  const ROUNDING = [
    tr('Montants arrondis au centime, devise par devise', 'Amounts rounded to the cent, currency by currency'),
    tr('Tests verts', 'Tests pass'),
  ];
  const ROUNDING_DONE = [
    tr('Arrondi au centime par devise', 'Rounding to the cent per currency'),
    tr('Tests des devises', 'Currency tests'),
  ];
  const asCriteria = (texts: string[]) => texts.map((text) => ({ text, ok: false }));
  /** The tickets imported from Jira, in the order of the list: they take the board's next keys. */
  const IMPORTED: Ticket[] = [
    {
      key: 'DEM-7',
      title: titles.rounding,
      column: 'todo',
      loop: [1, 5],
      criteria: asCriteria(ROUNDING),
      external: { service: 'jira', key: 'ATL-1294' },
    },
    {
      key: 'DEM-8',
      title: titles.remind,
      column: 'todo',
      loop: [1, 5],
      criteria: asCriteria([
        tr('Relance à J+7, puis à J+15', 'Reminder at D+7, then at D+15'),
        tr('Lien de paiement dans l’e-mail', 'Payment link in the email'),
        tr('Tests verts', 'Tests pass'),
      ]),
      external: { service: 'jira', key: 'ATL-1291' },
    },
    {
      key: 'DEM-9',
      title: titles.pdf,
      column: 'todo',
      loop: [1, 5],
      criteria: asCriteria([
        tr('Mise en page de la facture', 'Invoice layout'),
        tr('Lien de téléchargement', 'Download link'),
        tr('Tests verts', 'Tests pass'),
      ]),
      external: { service: 'jira', key: 'ATL-1287' },
    },
  ];
  const LABELLED: Ticket = {
    key: 'DEM-10',
    title: titles.alert,
    column: 'todo',
    loop: [1, 5],
    criteria: asCriteria([
      tr('Alerte dans la minute', 'Alert within a minute'),
      tr('Montant et client dans l’alerte', 'Amount and customer in the alert'),
      tr('Tests verts', 'Tests pass'),
    ]),
    external: { service: 'jira', key: 'ATL-1318' },
  };
  return {
    titles,
    ALL,
    QUERY,
    SEARCHED,
    MINE,
    PICKS,
    ROUNDING,
    ROUNDING_DONE,
    IMPORTED,
    LABELLED,
    // As the app names them: key and title, cut at 40 characters on a word.
    AGENT_ROUNDING: ticketAgent('DEM-7', titles.rounding),
    AGENT_REMIND: ticketAgent('DEM-8', titles.remind),
  };
}

/** The settings' body moves up this far to show the sources and the status mapping. */
const SCROLL = 236;
/** Agents in parallel on the board, as the previous scene left it. */
const PARALLEL = 3;

/** A Jira account connected and its project linked; tickets imported; their status and comments kept up to date; a label imports one alone. */
export const IntegrationsScene: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr, usd } = useFmt();
  const { status: STATUS } = useDemo();
  const { AGENT_CSV, AGENT_LOGIN, boardAgents, CSV, DONE, LOGIN, SEARCH } = useBoard();
  const { titles, ALL, QUERY, SEARCHED, MINE, PICKS, ROUNDING, ROUNDING_DONE, IMPORTED, LABELLED, AGENT_ROUNDING, AGENT_REMIND } =
    dataOf(tr);
  const merged = tr('⤵ Mergé dans main · squash', '⤵ Merged into main · squash');
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
        outcome: merged,
        doneMeta: tr(`1 boucle · ${usd(0.54)}`, `1 loop · ${usd(0.54)}`),
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
        activity: tr('Lit src/billing/currency.ts', 'Reads src/billing/currency.ts'),
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
      progress: ['Route GET /invoices.csv', tr('Séparateur point-virgule', 'Semicolon separator')],
      activity: tr('Lance npm test', 'Runs npm test'),
      agent: { name: AGENT_CSV, status: 'running' },
    },
    {
      ...SEARCH,
      column: 'doing',
      partial: false,
      loop: [1, 5],
      activity: tr('Modifie src/search/index.ts', 'Edits src/search/index.ts'),
      agent: { name: SEARCH.agent!.name, status: 'running' },
    },
    {
      ...LOGIN,
      column: 'done',
      outcome: merged,
      doneMeta: tr(`3 boucles · ${usd(1.86)}`, `3 loops · ${usd(1.86)}`),
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
                activity: tr('Réfléchit', 'Thinking'),
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
    return {
      ...t,
      wait:
        i === 0
          ? tr("Pris dès qu'une place se libère", 'Picked up as soon as a slot frees up')
          : tr(`En attente d'une place (${doing}/${PARALLEL})`, `Waiting for a slot (${doing}/${PARALLEL})`),
    };
  });

  const status = frame >= doneAt ? tr('À tester', 'To Review') : tr('En cours', 'In Progress');
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
                { name: AGENT_CSV, tag: tr('DEM-5 · boucle 1/5', 'DEM-5 · loop 1/5') },
                { name: SEARCH.agent!.name, tag: tr('DEM-4 · boucle 1/5', 'DEM-4 · loop 1/5') },
                ...(frame >= startAt && frame < doneAt
                  ? [
                      {
                        name: AGENT_ROUNDING,
                        tag: frame >= reviewAt ? tr('DEM-7 · à tester', 'DEM-7 · to review') : tr('DEM-7 · boucle 1/5', 'DEM-7 · loop 1/5'),
                        status: frame >= reviewAt ? ('done' as const) : ('running' as const),
                      },
                    ]
                  : []),
                ...(frame >= nextAt ? [{ name: AGENT_REMIND, tag: tr('DEM-8 · boucle 1/5', 'DEM-8 · loop 1/5') }] : []),
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
                    { states: { jira: tr('En cours', 'In Progress') }, comment: false },
                    { states: { jira: tr('En cours', 'In Progress') }, comment: true },
                    { states: { jira: tr('À tester', 'To Review') }, comment: true },
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
                text={tr('3 tickets importés depuis Jira', '3 tickets imported from Jira')}
                tone="ok"
                enter={frame >= imported && frame < imported + 70 ? pop(frame, fps, imported, 16) : 0}
              />
              <AppToast
                text={tr('1 ticket importé depuis Jira dans demo-api', '1 ticket imported from Jira into demo-api')}
                tone="ok"
                enter={frame >= autoAt ? pop(frame, fps, autoAt, 16) : 0}
              />
              <JiraIssue
                enter={frame >= syncOut ? 0 : frame >= syncIn ? pop(frame, fps, syncIn, 18) : 0}
                style={{ left: 22, top: 112 }}
                issueKey="ATL-1294"
                title={titles.rounding}
                status={status}
                flash={Math.max(flash(syncIn + 10), flash(doneAt))}
                comments={[
                  {
                    text: (
                      <>
                        {tr('Escouade : DEM-7 est prêt à tester.', 'Escouade: DEM-7 is ready for review.')}
                        <Para title={tr('Critères :', 'Criteria:')} lines={ROUNDING.map((r) => `✓ ${r}`)} />
                        <Para title={tr('Ce qui a été fait :', 'What was done:')} lines={ROUNDING_DONE.map((r) => `- ${r}`)} />
                      </>
                    ),
                    enter: pop(frame, fps, reviewAt + 4, 18),
                  },
                  {
                    text: tr(`Escouade : DEM-7 est terminé — ${merged}`, `Escouade: DEM-7 is done — ${merged}`),
                    enter: pop(frame, fps, doneAt + 4, 18),
                  },
                ]}
              />
              <JiraIssue
                enter={frame >= syncOut ? pop(frame, fps, syncOut + 2, 18) : 0}
                style={{ right: 22, top: 130 }}
                issueKey="ATL-1318"
                title={titles.alert}
                status={tr('À faire', 'To Do')}
                labels={[
                  { name: tr('paiements', 'payments'), enter: 1 },
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
          <BoardView tickets={tickets} autopilot places={placesText(tr, free)} pressedImport={press(openImport)} />
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
