import type { FC, ReactNode } from 'react';
import { useCurrentFrame, useVideoConfig } from 'remotion';
import { pop, ramp } from '../anim';
import { useCues } from '../cues';
import { onSonnet, TABS, useDemo } from '../data';
import { useFmt } from '../lang';
import { C } from '../theme';
import { Shell } from '../ui/Shell';
import { AgentsSidebar } from '../ui/Sidebar';
import { Field, GroupHead, SettingRow, SettingsWindow, Toggle, type SettingsPageId } from '../ui/SettingsWindow';
import { AppWindow, Stage, Title } from '../ui/Stage';

const Key: FC<{ children: ReactNode }> = ({ children }) => (
  <span
    style={{
      padding: '6px 14px',
      borderRadius: 9,
      boxShadow: `inset 0 0 0 1px ${C.line3}, inset 0 -3px 0 rgba(255, 255, 255, 0.08)`,
      background: C.elev2,
      fontSize: 30,
      fontWeight: 600,
    }}
  >
    {children}
  </span>
);

const Platform: FC<{ name: string; detail: string; enter: number; logo: ReactNode }> = ({ name, detail, enter, logo }) => (
  <div
    style={{
      width: 480,
      padding: '28px 30px',
      borderRadius: 18,
      background: C.panel,
      boxShadow: `inset 0 0 0 1px ${C.line2}, 0 30px 80px rgba(0,0,0,0.45)`,
      display: 'flex',
      alignItems: 'center',
      gap: 24,
      opacity: Math.min(1, enter),
      transform: `translateY(${(1 - Math.min(1, enter)) * 40}px)`,
    }}
  >
    {logo}
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
      <span style={{ fontSize: 36, fontWeight: 800, letterSpacing: '-0.02em' }}>{name}</span>
      <span style={{ fontSize: 17, color: C.muted }}>{detail}</span>
    </div>
  </div>
);

/** The settings of the 2.2, page by page as the voice names them; some shortcuts; then Windows, macOS and Linux. */
export const More: FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const c = useCues();
  const { tr } = useFmt();
  const { agents, status, refactor } = useDemo();
  const w = (word: string) => c.word('extras', word);
  const glow = (from: number, to: number) => ramp(frame, from - 2, 8) - ramp(frame, to - 6, 8);
  const open = pop(frame, fps, w('réglages') - 8, 16);
  const keys = pop(frame, fps, w('raccourcis') - 4, 14) - ramp(frame, w('mises') - 6, 8);
  const page: SettingsPageId = frame >= w('mises') - 6 ? 'general' : frame >= w('proxy') - 6 ? 'advanced' : 'agents';
  const out = ramp(frame, c.at('os') - 10, 14);
  const os = (word: string) => pop(frame, fps, c.word('os', word) - 6, 16);
  const pages: Record<SettingsPageId, { title: string; desc: string; body: ReactNode }> = {
    agents: {
      title: 'Agents',
      desc: tr(
        'Les nouveaux agents, les textes rédigés, le remote control et le pilotage d’Escouade',
        'New agents, written texts, remote control and how Escouade is driven',
      ),
      body: (
        <>
          <GroupHead>{tr('Nouveaux agents', 'New agents')}</GroupHead>
          <SettingRow label={tr('Harness par défaut', 'Default harness')}>
            <Field select>Claude Code</Field>
          </SettingRow>
          <SettingRow label={tr('Modèle, effort et mode', 'Model, effort and mode')}>
            <Field select>{tr('Opus 5.5 · Élevé · Auto', 'Opus 5.5 · High · Auto')}</Field>
          </SettingRow>
          <SettingRow label={tr('Nommer l’agent d’après sa première demande', 'Name the agent from its first request')}>
            <Toggle on />
          </SettingRow>
          <SettingRow
            label={tr('Arrêter les processus des agents inactifs après', 'Stop idle agent processes after')}
            hint={tr('minutes, 0 = jamais', 'minutes, 0 = never')}
            glow={glow(w('arrêt'), w('proxy'))}
          >
            <Field width={120}>30</Field>
          </SettingRow>
          <GroupHead>Remote control</GroupHead>
          <SettingRow label={tr('Activer le remote control des nouveaux agents', 'Turn on remote control for new agents')}>
            <Toggle on={false} />
          </SettingRow>
        </>
      ),
    },
    advanced: {
      title: tr('Avancé', 'Advanced'),
      desc: tr(
        'Réseau, terminaux, pièces jointes, recherche par le sens et données locales',
        'Network, terminals, attachments, search by meaning and local data',
      ),
      body: (
        <>
          <GroupHead>Proxy</GroupHead>
          <SettingRow label={tr('Proxy HTTP(S)', 'HTTP(S) proxy')} glow={glow(w('proxy'), w('raccourcis'))}>
            <Field mono width={340}>
              {tr('http://proxy.entreprise.fr:8080', 'http://proxy.company.com:8080')}
            </Field>
          </SettingRow>
          <SettingRow label={tr('Exclusions', 'Exclusions')} hint={tr('NO_PROXY, séparées par des virgules', 'NO_PROXY, comma-separated')}>
            <Field mono width={340}>
              {tr('localhost, *.interne', 'localhost, *.internal')}
            </Field>
          </SettingRow>
          <SettingRow label={tr('Appliquer aussi le proxy aux terminaux intégrés', 'Also apply the proxy to the built-in terminals')}>
            <Toggle on />
          </SettingRow>
          <GroupHead>{tr('Certificats', 'Certificates')}</GroupHead>
          <SettingRow
            label={tr('Ignorer la vérification des certificats TLS', 'Skip TLS certificate checks')}
            hint={tr('Pour un proxy qui déchiffre le trafic', 'For a proxy that decrypts the traffic')}
          >
            <Toggle on={false} />
          </SettingRow>
        </>
      ),
    },
    general: {
      title: tr('Général', 'General'),
      desc: tr('Thème, zoom, langue, mises à jour et ce qui te concerne', 'Theme, zoom, language, updates and what concerns you'),
      body: (
        <>
          <GroupHead>{tr('Apparence', 'Appearance')}</GroupHead>
          <SettingRow label={tr('Thème', 'Theme')}>
            <Field select>{tr('Sombre', 'Dark')}</Field>
          </SettingRow>
          <SettingRow label={tr('Langue de l’interface', 'Interface language')}>
            <Field select>{tr('Français', 'English')}</Field>
          </SettingRow>
          <GroupHead>{tr('Mises à jour', 'Updates')}</GroupHead>
          <SettingRow
            label={tr('Installer les mises à jour automatiquement', 'Install updates automatically')}
            hint={tr(
              'Escouade redémarre d’elle-même quand aucun agent ne travaille et que tout est enregistré.',
              'Escouade restarts by itself when no agent is working and everything is saved.',
            )}
            glow={glow(w('mises'), c.at('os'))}
          >
            <Toggle on />
          </SettingRow>
          <SettingRow
            label={tr('Recevoir les versions bêta', 'Receive beta versions')}
            hint={tr('Les bêtas arrivent avant tout le monde.', 'Betas arrive before everyone else.')}
          >
            <Toggle on={false} />
          </SettingRow>
        </>
      ),
    },
    accounts: { title: '', desc: '', body: null },
    services: { title: '', desc: '', body: null },
    voice: { title: '', desc: '', body: null },
    notifications: { title: '', desc: '', body: null },
    plugins: { title: '', desc: '', body: null },
  };
  const shown = pages[page];
  return (
    <Stage>
      <Title />
      <div style={{ opacity: 1 - out, transform: `scale(${1 - 0.06 * out})` }}>
        <AppWindow>
          <Shell
            tabs={TABS}
            status={status}
            view="settings"
            sidebar={<AgentsSidebar agents={onSonnet(agents)} selected={refactor} />}
            overlay={
              <SettingsWindow page={page} title={shown.title} desc={shown.desc} enter={open}>
                {shown.body}
              </SettingsWindow>
            }
          />
        </AppWindow>
        {keys > 0 ? (
          <div
            style={{
              position: 'absolute',
              left: 0,
              right: 0,
              top: 900,
              display: 'flex',
              justifyContent: 'center',
              gap: 34,
              opacity: Math.min(1, keys),
              transform: `translateY(${(1 - Math.min(1, keys)) * 30}px)`,
              zIndex: 60,
            }}
          >
            {[
              ['Ctrl', 'K', tr('chercher', 'search')],
              ['Ctrl', 'N', tr('nouvel agent', 'new agent')],
              ['Ctrl', 'J', tr('agent qui attend', 'waiting agent')],
            ].map(([a, b, label]) => (
              <div
                key={label}
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  padding: '12px 18px',
                  borderRadius: 14,
                  background: C.panel,
                  boxShadow: `inset 0 0 0 1px ${C.line2}, 0 20px 60px rgba(0,0,0,0.6)`,
                }}
              >
                <Key>{a}</Key>
                <Key>{b}</Key>
                <span style={{ fontSize: 22, color: C.soft, marginLeft: 6 }}>{label}</span>
              </div>
            ))}
          </div>
        ) : null}
      </div>
      <div style={{ position: 'absolute', left: 0, right: 0, top: 400, display: 'flex', justifyContent: 'center', gap: 30 }}>
        <Platform
          name="Windows"
          detail={tr('10 et 11 · installeur .exe', '10 and 11 · .exe installer')}
          enter={os('windows')}
          logo={
            <svg width="84" height="84" viewBox="0 0 20 20">
              <rect x="0" y="0" width="9.4" height="9.4" fill="#4cc2ff" />
              <rect x="10.6" y="0" width="9.4" height="9.4" fill="#4cc2ff" />
              <rect x="0" y="10.6" width="9.4" height="9.4" fill="#4cc2ff" />
              <rect x="10.6" y="10.6" width="9.4" height="9.4" fill="#4cc2ff" />
            </svg>
          }
        />
        <Platform
          name="macOS"
          detail={tr('11 et plus · .dmg universel', '11 or later · universal .dmg')}
          enter={os('mac')}
          logo={
            <svg width="84" height="84" viewBox="0 0 24 24" fill={C.text}>
              <path d="M16.4 12.6c0-2.4 2-3.5 2-3.6-1.1-1.6-2.8-1.8-3.4-1.8-1.4-.1-2.8.9-3.5.9-.7 0-1.9-.9-3.1-.8-1.6 0-3 .9-3.8 2.3-1.6 2.8-.4 7 1.2 9.3.8 1.1 1.7 2.4 2.9 2.3 1.2 0 1.6-.7 3-.7s1.8.7 3.1.7c1.3 0 2.1-1.1 2.8-2.3.9-1.3 1.3-2.6 1.3-2.6s-2.5-1-2.5-3.7zM14.2 5.6c.6-.8 1.1-1.9 1-3-1 0-2.1.7-2.8 1.4-.6.7-1.1 1.8-1 2.9 1.1.1 2.1-.6 2.8-1.3z" />
            </svg>
          }
        />
        <Platform
          name="Linux"
          detail={tr('AppImage et .deb · x86-64', 'AppImage and .deb · x86-64')}
          enter={os('linux')}
          logo={
            <svg width="84" height="84" viewBox="0 0 24 24">
              <ellipse cx="12" cy="13.5" rx="6.6" ry="8.5" fill="#1d1f22" stroke={C.soft} strokeWidth="0.6" />
              <ellipse cx="12" cy="15.5" rx="4.3" ry="5.6" fill="#f2f3f4" />
              <circle cx="10" cy="8.6" r="1.1" fill="#f2f3f4" />
              <circle cx="14" cy="8.6" r="1.1" fill="#f2f3f4" />
              <circle cx="10.2" cy="8.8" r="0.5" fill="#101114" />
              <circle cx="13.8" cy="8.8" r="0.5" fill="#101114" />
              <path d="M10.4 10.6h3.2l-1.6 1.6z" fill="#f2b233" />
              <ellipse cx="8.6" cy="22" rx="2.4" ry="1" fill="#f2b233" />
              <ellipse cx="15.4" cy="22" rx="2.4" ry="1" fill="#f2b233" />
            </svg>
          }
        />
      </div>
    </Stage>
  );
};
