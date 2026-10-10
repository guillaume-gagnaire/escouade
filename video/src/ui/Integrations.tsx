// The « Intégrations » tab of the settings, « Importer des tickets » and a ticket on Jira's side
// (IntegrationsTab.svelte and ImportModal.svelte of the app).

import type { CSSProperties, FC, ReactNode } from 'react';
import { useFmt, type Tr } from '../lang';
import { C, MONO, soft } from '../theme';
import { columnsOf } from './Board';
import { Switch } from './Modal';
import { ServiceBadge, SERVICES, type Service } from './services';
import { Button } from './Sidebar';

/** A group of the settings: its title, its note, its rows. `glow` rings it. */
const Group: FC<{ title: string; note?: string; glow?: number; enter?: number; children: ReactNode }> = ({
  title,
  note,
  glow = 0,
  enter = 1,
  children,
}) =>
  enter <= 0 ? null : (
    <section
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: 6,
        padding: '14px 12px 8px',
        margin: '0 -12px',
        borderRadius: 10,
        boxShadow: glow ? `0 0 0 2px ${soft(C.spark, 100 * glow)}` : 'none',
        opacity: Math.min(1, enter),
        transform: `translateY(${(1 - Math.min(1, enter)) * 14}px)`,
      }}
    >
      <span style={{ fontSize: 14.5, fontWeight: 700 }}>{title}</span>
      {note ? <span style={{ fontSize: 12.5, color: C.muted, lineHeight: 1.4 }}>{note}</span> : null}
      <div style={{ display: 'flex', flexDirection: 'column', marginTop: 4 }}>{children}</div>
    </section>
  );

/** A select of the settings, closed. */
const Select: FC<{ value: string; dim?: boolean; width?: number; flash?: number }> = ({ value, dim, width, flash = 0 }) => (
  <span
    style={{
      width,
      flex: width ? 'none' : 1,
      minWidth: 0,
      height: 34,
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '0 10px 0 12px',
      borderRadius: 6,
      border: `1px solid ${flash ? C.spark : C.line2}`,
      background: flash ? soft(C.spark, 14 * flash) : C.bg,
      color: dim ? C.dim : C.text,
      fontSize: 13.5,
      whiteSpace: 'nowrap',
      overflow: 'hidden',
    }}
  >
    <span style={{ flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis' }}>{value}</span>
    <span style={{ color: C.dim, fontSize: 11 }}>▾</span>
  </span>
);

export interface Account {
  service: Service;
  /** « Connecté · … », or not connected, or being checked. */
  label?: string;
  checking?: boolean;
  pressed?: number;
}

const containerOf = (tr: Tr): Record<Service, string> => ({
  jira: tr('Projet', 'Project'),
  trello: tr('Tableau', 'Board'),
  github: tr('Dépôt', 'Repository'),
});

/** The « Intégrations » tab of demo-api: accounts, linked sources, status mapping. */
export const IntegrationsTab: FC<{
  accounts: Account[];
  /** The connected services, in the app's order, each with its linked container (`null`: none). */
  sources: readonly (readonly [Service, string | null])[];
  sourceFlash?: Partial<Record<Service, number>>;
  /** Each column's state, per linked service, and whether it is commented. */
  mapping: { states: Partial<Record<Service, string>>; comment: boolean }[];
  mappingGlow?: number;
  mappingEnter?: number;
}> = ({ accounts, sources, sourceFlash = {}, mapping, mappingGlow, mappingEnter = 1 }) => {
  const { tr } = useFmt();
  const CONTAINER = containerOf(tr);
  const linked = sources.filter(([, name]) => name).map(([s]) => s);
  return (
    <>
      <Group
        title={tr('Comptes connectés', 'Connected accounts')}
        note={tr(
          "Les jetons restent sur cette machine, à part des réglages, et ne servent qu'aux appels de ces services.",
          'Tokens stay on this machine, stored apart from the settings, and are only used for calls to these services.',
        )}
      >
        {accounts.map((a) => (
          <div key={a.service} style={{ height: 54, display: 'flex', alignItems: 'center', gap: 13, borderBottom: `1px solid ${C.line}` }}>
            <ServiceBadge service={a.service} size={26} />
            <span style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 2 }}>
              <span style={{ fontSize: 14.5, fontWeight: 700 }}>{SERVICES[a.service].name}</span>
              <span style={{ fontSize: 12.5, color: a.label ? C.ok : C.dim }}>
                {a.checking
                  ? tr('Vérification…', 'Checking…')
                  : a.label
                    ? tr(`Connecté · ${a.label}`, `Connected · ${a.label}`)
                    : tr('Non connecté', 'Not connected')}
              </span>
            </span>
            {a.label ? (
              <Button small>{tr('Déconnecter', 'Disconnect')}</Button>
            ) : (
              <Button small pressed={a.pressed}>
                {tr('Connecter…', 'Connect…')}
              </Button>
            )}
          </div>
        ))}
      </Group>
      <Group title={tr('Sources liées à demo-api', 'Sources linked to demo-api')}>
        {sources.map(([s, name]) => (
          <div key={s} style={{ height: 48, display: 'flex', alignItems: 'center', gap: 12 }}>
            <span style={{ width: 170, display: 'flex', alignItems: 'center', gap: 9, fontSize: 14, fontWeight: 600 }}>
              <ServiceBadge service={s} size={20} />
              {SERVICES[s].name}
            </span>
            <span style={{ width: 62, fontSize: 12.5, color: C.dim }}>{CONTAINER[s]}</span>
            <Select value={name ?? tr('Aucun', 'None')} dim={!name} width={340} flash={sourceFlash[s]} />
          </div>
        ))}
      </Group>
      <Group
        title={tr('Correspondance des statuts', 'Status mapping')}
        note={tr(
          "L'état que prend le ticket externe quand son ticket arrive dans la colonne ; « Commenter » y ajoute un commentaire.",
          'The state the external ticket takes when its ticket reaches the column; “Comment” adds a comment to it.',
        )}
        glow={mappingGlow}
        enter={mappingEnter}
      >
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: `120px repeat(${linked.length}, minmax(0, 1fr)) 86px`,
            gap: '6px 10px',
            alignItems: 'center',
          }}
        >
          <span style={{ fontSize: 12, fontWeight: 700, color: C.dim }}>Kanban</span>
          {linked.map((s) => (
            <span key={s} style={{ display: 'flex', alignItems: 'center', gap: 7, fontSize: 12.5, fontWeight: 700, color: C.muted }}>
              <ServiceBadge service={s} size={17} />
              {SERVICES[s].name}
            </span>
          ))}
          <span style={{ fontSize: 12, fontWeight: 700, color: C.dim, textAlign: 'center' }}>{tr('Commenter', 'Comment')}</span>
          {columnsOf(tr).map((col, i) => (
            <Row key={col.id}>
              <span style={{ display: 'flex', alignItems: 'center', gap: 8, fontSize: 13.5, fontWeight: 600 }}>
                <span style={{ width: 8, height: 8, borderRadius: '50%', background: col.color }} />
                {col.label}
              </span>
              {linked.map((s) => (
                <Select key={s} value={mapping[i].states[s] ?? tr('— inchangé', '— unchanged')} dim={!mapping[i].states[s]} />
              ))}
              <span style={{ display: 'flex', justifyContent: 'center' }}>
                <Switch on={mapping[i].comment} />
              </span>
            </Row>
          ))}
        </div>
      </Group>
    </>
  );
};

/** The cells of a grid row, as they are. */
const Row: FC<{ children: ReactNode }> = ({ children }) => <>{children}</>;

export interface Issue {
  key: string;
  title: string;
  kind: 'Story' | 'Bug' | 'Tâche' | 'Task';
  meta: string[];
  criteria?: number;
  imported?: boolean;
}

const KIND_COLOR: Record<Issue['kind'], string> = { Story: C.ok, Bug: C.del, Tâche: C.info, Task: C.info };

/** « Importer des tickets », on Jira; `checked` of the rows are ticked, `critGlow` lights the criteria found. */
export const ImportModal: FC<{
  enter: number;
  /** The linked Jira project. */
  source: string;
  issues: Issue[];
  checked: string[];
  query: string;
  focus?: boolean;
  searching?: boolean;
  filter: boolean;
  critGlow?: number;
  importPressed?: number;
}> = ({ enter, source, issues, checked, query, focus, searching, filter, critGlow = 0, importPressed = 0 }) => {
  const { tr, trx, plural } = useFmt();
  if (enter <= 0) return null;
  const e = Math.min(1, enter);
  const n = checked.length;
  const all = issues.filter((i) => !i.imported);
  const allOn = all.length > 0 && all.every((i) => checked.includes(i.key));
  return (
    <div
      style={{
        position: 'absolute',
        inset: 0,
        background: `rgba(10, 9, 8, ${0.55 * e})`,
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 40,
      }}
    >
      <div
        style={{
          width: 940,
          height: 720,
          borderRadius: 14,
          background: C.panel,
          border: `1px solid ${C.line2}`,
          boxShadow: '0 30px 90px rgba(0, 0, 0, 0.6)',
          opacity: e,
          transform: `translateY(${(1 - e) * 30}px) scale(${0.96 + 0.04 * e})`,
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'flex-start', padding: '22px 20px 15px 26px' }}>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            <span style={{ fontSize: 19.5, fontWeight: 700 }}>{tr('Importer des tickets', 'Import tickets')}</span>
            <span style={{ fontSize: 13.5, color: C.muted }}>
              {tr('Dans « À faire » du Kanban de demo-api', 'Into “To do” in the demo-api Kanban')}
            </span>
          </div>
          <div style={{ flex: 1 }} />
          <span style={{ color: C.dim, fontSize: 17 }}>✕</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 5, padding: '0 26px 13px' }}>
          {/* Its only linked source, Jira's, picked. */}
          <span
            style={{
              height: 46,
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              padding: '0 13px',
              borderRadius: 8,
              border: `1px solid ${C.line2}`,
              background: C.elev,
            }}
          >
            <ServiceBadge service="jira" size={22} />
            <span style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
              <span style={{ fontSize: 14, fontWeight: 700 }}>{SERVICES.jira.name}</span>
              <span style={{ fontFamily: MONO, fontSize: 11, color: C.dim }}>{source}</span>
            </span>
            {n ? (
              <span
                style={{
                  minWidth: 20,
                  height: 20,
                  padding: '0 6px',
                  borderRadius: 10,
                  background: C.spark,
                  color: '#1b1512',
                  fontFamily: MONO,
                  fontSize: 11.5,
                  fontWeight: 700,
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                }}
              >
                {n}
              </span>
            ) : null}
          </span>
          <div style={{ flex: 1 }} />
          <span style={{ fontSize: 13, fontWeight: 600, color: C.muted }}>{tr('⚙ Gérer les sources', '⚙ Manage sources')}</span>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 11, padding: '15px 26px 13px', borderTop: `1px solid ${C.line}` }}>
          <span
            style={{
              height: 40,
              display: 'flex',
              alignItems: 'center',
              gap: 11,
              padding: '0 13px',
              borderRadius: 6,
              border: `1px solid ${focus ? C.spark : C.line2}`,
              background: C.bg,
              fontSize: 14.5,
            }}
          >
            <span style={{ color: C.dim }}>⌕</span>
            <span style={{ color: query ? C.text : C.dim }}>
              {query || tr('Rechercher par clé ou par texte…', 'Search by key or text…')}
            </span>
            {focus ? <span style={{ width: 2, height: 18, marginLeft: -9, background: C.text }} /> : null}
          </span>
          <span style={{ display: 'flex', gap: 7 }}>
            {[tr('Assignés à moi', 'Assigned to me'), tr('Sprint actif', 'Active sprint'), tr('À faire', 'To do')].map((f, i) => {
              const on = i === 0 && filter;
              return (
                <span
                  key={f}
                  style={{
                    height: 28,
                    padding: '0 12px',
                    display: 'flex',
                    alignItems: 'center',
                    borderRadius: 99,
                    border: `1px solid ${on ? C.spark : C.line2}`,
                    background: on ? C.elev2 : 'transparent',
                    color: on ? C.text : C.muted,
                    fontSize: 13,
                    fontWeight: 600,
                  }}
                >
                  {f}
                </span>
              );
            })}
          </span>
        </div>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 13,
            padding: '9px 26px 9px 41px',
            borderTop: `1px solid ${C.line}`,
            borderBottom: `1px solid ${C.line}`,
            background: C.bg,
          }}
        >
          <Check on={allOn} />
          <span style={{ fontSize: 13, fontWeight: 600, color: C.muted }}>
            {searching
              ? tr('Recherche…', 'Searching…')
              : `${issues.length} ${plural(issues.length, 'résultat', 'résultats', 'result', 'results')}`}
          </span>
          <div style={{ flex: 1 }} />
          <span style={{ fontFamily: MONO, fontSize: 11.5, color: C.dim }}>lea@demo.dev · {source}</span>
        </div>
        <div
          style={{ flex: 1, minHeight: 0, display: 'flex', flexDirection: 'column', padding: '7px 11px', opacity: searching ? 0.45 : 1 }}
        >
          {issues.map((i) => {
            const on = checked.includes(i.key);
            return (
              <div
                key={i.key}
                style={{
                  display: 'flex',
                  alignItems: 'flex-start',
                  gap: 13,
                  padding: '12px 15px 12px 30px',
                  borderRadius: 6,
                  background: on ? soft(C.spark, 9) : 'transparent',
                  opacity: i.imported ? 0.5 : 1,
                }}
              >
                <span style={{ marginTop: 2 }}>
                  <Check on={on || !!i.imported} />
                </span>
                <div style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 6 }}>
                  <div style={{ display: 'flex', alignItems: 'baseline', gap: 10 }}>
                    <span style={{ fontFamily: MONO, fontSize: 12, color: C.dim }}>{i.key}</span>
                    <span style={{ fontSize: 15, fontWeight: 600 }}>{i.title}</span>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 11, fontSize: 12.5, color: C.muted }}>
                    <span
                      style={{
                        fontSize: 11.5,
                        fontWeight: 700,
                        padding: '1px 7px',
                        borderRadius: 3,
                        background: C.elev2,
                        color: KIND_COLOR[i.kind],
                      }}
                    >
                      {i.kind}
                    </span>
                    <span style={{ whiteSpace: 'pre' }}>{i.meta.join('  ·  ')}</span>
                    {i.criteria ? (
                      <span
                        style={{
                          color: C.ok,
                          padding: '1px 7px',
                          margin: '-1px 0',
                          borderRadius: 4,
                          boxShadow: critGlow ? `0 0 0 1.5px ${soft(C.ok, 100 * critGlow)}` : 'none',
                          background: critGlow ? soft(C.ok, 14 * critGlow) : 'transparent',
                        }}
                      >
                        {trx(<>✓ {i.criteria} critères détectés</>, `✓ ${i.criteria} criteria detected`)}
                      </span>
                    ) : null}
                  </div>
                </div>
                {i.imported ? (
                  <span
                    style={{
                      flex: 'none',
                      fontSize: 12,
                      fontWeight: 600,
                      padding: '3px 9px',
                      borderRadius: 99,
                      background: C.elev2,
                      color: C.muted,
                    }}
                  >
                    {tr('Déjà dans le Kanban', 'Already in the Kanban')}
                  </span>
                ) : null}
              </div>
            );
          })}
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 15, padding: '15px 20px 17px 26px', borderTop: `1px solid ${C.line}` }}>
          <span style={{ fontSize: 13.5, fontWeight: 600 }}>
            {n
              ? tr(
                  `${n} ticket${n > 1 ? 's' : ''} sélectionné${n > 1 ? 's' : ''}`,
                  `${n} ${plural(n, '', '', 'ticket', 'tickets')} selected`,
                )
              : tr('Aucun ticket sélectionné', 'No ticket selected')}
          </span>
          <div style={{ flex: 1 }} />
          <span style={{ fontSize: 12.5, color: C.dim }}>{tr('Boucles max', 'Max loops')}</span>
          <span style={{ display: 'flex', gap: 2, padding: 2, borderRadius: 6, background: C.bg, border: `1px solid ${C.line}` }}>
            {[3, 5, 8].map((l) => (
              <span
                key={l}
                style={{
                  height: 26,
                  minWidth: 30,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  borderRadius: 3,
                  background: l === 5 ? C.elev2 : 'transparent',
                  color: l === 5 ? C.text : C.muted,
                  fontFamily: MONO,
                  fontSize: 12,
                  fontWeight: 600,
                }}
              >
                {l}
              </span>
            ))}
          </span>
          <Button>{tr('Annuler', 'Cancel')}</Button>
          <span style={{ display: 'flex', opacity: n ? 1 : 0.5, transform: `scale(${1 - 0.06 * importPressed})` }}>
            <Button primary={C.spark}>
              {n
                ? tr(`Importer ${n} ticket${n > 1 ? 's' : ''}`, `Import ${n} ${plural(n, '', '', 'ticket', 'tickets')}`)
                : tr('Importer', 'Import')}
            </Button>
          </span>
        </div>
      </div>
    </div>
  );
};

const Check: FC<{ on: boolean }> = ({ on }) => (
  <span
    style={{
      width: 17,
      height: 17,
      flex: 'none',
      borderRadius: 4,
      border: `1.5px solid ${on ? C.spark : C.line2}`,
      background: on ? C.spark : 'transparent',
      color: '#1b1512',
      fontSize: 12,
      fontWeight: 800,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}
  >
    {on ? '✓' : ''}
  </span>
);

const STATUS_COLOR: Record<string, string> = {
  'À faire': C.muted,
  'En cours': C.info,
  'À tester': C.wait,
  Terminé: C.ok,
  'To Do': C.muted,
  'In Progress': C.info,
  'To Review': C.wait,
  Done: C.ok,
};

export interface IssueComment {
  text: ReactNode;
  enter: number;
}

/** A ticket as Jira shows it: its status, its labels and the comments Escouade leaves on it, with the account it is connected to. `flash` lights its status. */
export const JiraIssue: FC<{
  enter: number;
  issueKey: string;
  title: string;
  status: string;
  flash?: number;
  labels?: { name: string; enter: number }[];
  comments: IssueComment[];
  style?: CSSProperties;
}> = ({ enter, issueKey, title, status, flash = 0, labels = [], comments, style }) => {
  const { tr } = useFmt();
  return enter <= 0 ? null : (
    <div
      style={{
        position: 'absolute',
        width: 520,
        borderRadius: 12,
        background: '#f7f8f9',
        color: '#172b4d',
        boxShadow: '0 30px 90px rgba(0, 0, 0, 0.6)',
        overflow: 'hidden',
        opacity: Math.min(1, enter),
        transform: `translateX(${(1 - Math.min(1, enter)) * 60}px)`,
        zIndex: 35,
        ...style,
      }}
    >
      <div
        style={{
          height: 40,
          display: 'flex',
          alignItems: 'center',
          gap: 9,
          padding: '0 16px',
          background: '#ffffff',
          borderBottom: '1px solid #dfe1e6',
        }}
      >
        <ServiceBadge service="jira" size={20} />
        <span style={{ fontSize: 13, fontWeight: 700, color: '#44546f' }}>Jira</span>
        <span style={{ fontSize: 13, color: '#8590a2' }}>/ Atlas /</span>
        <span style={{ fontFamily: MONO, fontSize: 12.5, color: '#44546f' }}>{issueKey}</span>
      </div>
      <div style={{ padding: '16px 18px 18px', display: 'flex', flexDirection: 'column', gap: 12 }}>
        <span style={{ fontSize: 19, fontWeight: 700, lineHeight: 1.3 }}>{title}</span>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, flexWrap: 'wrap' }}>
          <span
            style={{
              height: 28,
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '0 11px',
              borderRadius: 4,
              background: STATUS_COLOR[status] ?? C.muted,
              color: '#ffffff',
              fontSize: 13,
              fontWeight: 700,
              boxShadow: flash ? `0 0 0 ${3 * flash}px ${soft(STATUS_COLOR[status] ?? C.muted, 45)}` : 'none',
              transform: `scale(${1 + 0.08 * flash})`,
            }}
          >
            {status}
            <span style={{ fontSize: 10 }}>▾</span>
          </span>
          {labels
            .filter((l) => l.enter > 0)
            .map((l) => (
              <span
                key={l.name}
                style={{
                  height: 24,
                  display: 'flex',
                  alignItems: 'center',
                  padding: '0 9px',
                  borderRadius: 4,
                  background: '#e9f2ff',
                  border: '1px solid #cce0ff',
                  color: '#0c66e4',
                  fontFamily: MONO,
                  fontSize: 12,
                  opacity: Math.min(1, l.enter),
                  transform: `scale(${0.7 + 0.3 * l.enter})`,
                }}
              >
                {l.name}
              </span>
            ))}
        </div>
        {comments.some((c) => c.enter > 0) ? (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 9, marginTop: 4 }}>
            <span style={{ fontSize: 12, fontWeight: 700, letterSpacing: '0.04em', textTransform: 'uppercase', color: '#626f86' }}>
              {tr('Commentaires', 'Comments')}
            </span>
            {comments
              .filter((c) => c.enter > 0)
              .map((c, i) => (
                <div
                  key={i}
                  style={{
                    display: 'flex',
                    gap: 10,
                    opacity: Math.min(1, c.enter),
                    transform: `translateY(${(1 - Math.min(1, c.enter)) * 14}px)`,
                  }}
                >
                  <span
                    style={{
                      width: 30,
                      height: 30,
                      flex: 'none',
                      borderRadius: '50%',
                      background: '#0c66e4',
                      color: '#ffffff',
                      fontSize: 11.5,
                      fontWeight: 700,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                    }}
                  >
                    LM
                  </span>
                  <div style={{ flex: 1, display: 'flex', flexDirection: 'column', gap: 3 }}>
                    <span style={{ fontSize: 12.5, fontWeight: 700 }}>
                      {tr('Léa Martin ', 'Lea Martin ')}
                      <span style={{ fontWeight: 400, color: '#626f86' }}>{tr("· à l'instant", '· just now')}</span>
                    </span>
                    <span
                      style={{
                        padding: '8px 11px',
                        borderRadius: 6,
                        background: '#ffffff',
                        border: '1px solid #dfe1e6',
                        fontSize: 13.5,
                        lineHeight: 1.45,
                      }}
                    >
                      {c.text}
                    </span>
                  </div>
                </div>
              ))}
          </div>
        ) : null}
      </div>
    </div>
  );
};
