import type { FC } from 'react';

// The app's line icons (src/components/IconSprite.svelte), drawn on a 16 × 16 grid with a 1.5 stroke.
const PATHS = {
  search: 'M7 12.5a5.5 5.5 0 1 0 0-11 5.5 5.5 0 0 0 0 11zM14.5 14.5l-3.6-3.6',
  overview: 'M2.5 2.5h4.5v4.5H2.5zM9 2.5h4.5v4.5H9zM2.5 9h4.5v4.5H2.5zM9 9h4.5v4.5H9z',
  plus: 'M8 3v10M3 8h10',
  stats: 'M3 13.5V9M8 13.5V3M13 13.5V6.5',
  puzzle: 'M6 2.5h2.5a1.5 1.5 0 0 1 3 0V5h2v3a1.5 1.5 0 0 0 0 3v2.5H11a1.5 1.5 0 0 0-3 0H5.5V11a1.5 1.5 0 0 1 0-3V5H6z',
  sliders: 'M2.5 4.5h6M11.5 4.5h2M2.5 11.5h2M7.5 11.5h6M10 3v3M6 10v3',
  branch: 'M4.75 5.25v5.5M11.25 6.75c0 2.6-2.1 3.4-5.6 4.25',
  chevron: 'M4.5 6.5L8 10l3.5-3.5',
  right: 'M6.5 4L10.5 8l-4 4',
  code: 'M5.5 4.5L2 8l3.5 3.5M10.5 4.5L14 8l-3.5 3.5',
  speaker: 'M3 6h2.5L9 3v10L5.5 10H3zM11.5 6a3 3 0 0 1 0 4',
  chat: 'M2.5 3.5h11v7.5H7l-3 2.5v-2.5H2.5z',
  kanban: 'M2.5 2.5h3v11h-3zM6.5 2.5h3v7h-3zM10.5 2.5h3v9h-3z',
  pr: 'M4.5 5.5v6M11.5 6.5v5M9 3.5l2.5 0 0 2.5M4.5 2a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM4.5 11a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3zM11.5 11a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3z',
  doc: 'M4 1.5h5.5L12.5 4.5v10H4zM9 1.5v3.5h3.5M6 8h4.5M6 10.5h4.5',
  check: 'M3.5 8.5l3 3 6-6.5',
  more: 'M3.5 8h.01M8 8h.01M12.5 8h.01',
  split: 'M2.5 3h11v10h-11zM8 3v10',
  mic: 'M8 1.5a2 2 0 0 0-2 2V8a2 2 0 0 0 4 0V3.5a2 2 0 0 0-2-2zM4 7.5a4 4 0 0 0 8 0M8 11.5V14',
  clip: 'M10.5 5L5.8 9.7a1.2 1.2 0 0 0 1.7 1.7l5-5a2.5 2.5 0 0 0-3.5-3.5L3.9 8a3.8 3.8 0 0 0 5.4 5.4L13.5 9',
  workflow: 'M2.5 3.5h4v3h-4zM9.5 9.5h4v3h-4zM4.5 6.5v4.5h5',
  store: 'M2.5 6l1.2-3.5h8.6L13.5 6M2.5 6v7.5h11V6M2.5 6h11M6 9h4',
  globe: 'M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM1.5 8h13M8 1.5c2 2 2 11 0 13M8 1.5c-2 2-2 11 0 13',
  shield: 'M8 1.5l5.5 2v4.5c0 3.2-2.4 5.6-5.5 6.5-3.1-.9-5.5-3.3-5.5-6.5V3.5z',
  key: 'M10 2.5a3.5 3.5 0 1 0 0 7 3.5 3.5 0 0 0 0-7zM7.5 8.5L2 14M4 12l1.5 1.5',
  bell: 'M4 11V7a4 4 0 0 1 8 0v4l1.5 1.5h-11zM6.5 14h3',
  user: 'M8 2a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM2.5 14.5c.7-2.7 2.9-4.5 5.5-4.5s4.8 1.8 5.5 4.5',
  palette: 'M8 1.5a6.5 6.5 0 0 0 0 13c1 0 1.3-.8.9-1.6-.5-.9.1-1.9 1.1-1.9h1.8c1.5 0 2.7-1.2 2.7-2.7 0-3.7-2.9-6.8-6.5-6.8z',
  keyboard: 'M1.5 4h13v8h-13zM4 6.5h.01M6.5 6.5h.01M9 6.5h.01M11.5 6.5h.01M4.5 9.5h7',
  download: 'M8 2v8.5M4.5 7L8 10.5 11.5 7M2.5 13.5h11',
} as const;

export type IconName = keyof typeof PATHS;

export const Icon: FC<{ name: IconName; size?: number; color?: string; width?: number }> = ({
  name,
  size = 16,
  color = 'currentColor',
  width = 1.5,
}) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 16 16"
    fill="none"
    stroke={color}
    strokeWidth={width}
    strokeLinecap="round"
    strokeLinejoin="round"
    style={{ flex: 'none', display: 'block' }}
  >
    {name === 'branch' ? (
      <>
        <circle cx="4.75" cy="3.75" r="1.5" />
        <circle cx="4.75" cy="12.25" r="1.5" />
        <circle cx="11.25" cy="5.25" r="1.5" />
      </>
    ) : null}
    <path d={PATHS[name]} />
  </svg>
);
