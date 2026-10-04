import type { FC } from 'react';

/** The external ticket systems (src/lib/integrations.ts of the app). */
export type Service = 'jira' | 'trello' | 'github';

export const SERVICES: Record<Service, { name: string; letter: string; color: string; ink: string }> = {
  jira: { name: 'Jira', letter: 'J', color: 'oklch(0.62 0.17 255)', ink: '#fff' },
  trello: { name: 'Trello', letter: 'T', color: 'oklch(0.66 0.12 230)', ink: '#fff' },
  github: { name: 'GitHub Issues', letter: 'GH', color: '#e8e3dc', ink: '#1b1917' },
};

/** A service's coloured letter. */
export const ServiceBadge: FC<{ service: Service; size?: number }> = ({ service, size = 22 }) => (
  <span
    style={{
      minWidth: size,
      height: size,
      padding: '0 3px',
      flex: 'none',
      borderRadius: Math.round(size / 4),
      background: SERVICES[service].color,
      color: SERVICES[service].ink,
      fontSize: Math.round(size * 0.45),
      fontWeight: 800,
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
    }}
  >
    {SERVICES[service].letter}
  </span>
);
