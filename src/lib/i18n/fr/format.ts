import type { Tree } from '../types';

// The units of `format.ts`: numbers, amounts, durations, dates and sizes as the design writes them.
export default {
  /** Token counts: « 1,2 k », « 2,35 M », « 3,20 Md ». */
  tokens: { thousand: '{n} k', million: '{n} M', billion: '{n} Md' },
  /** A non-breaking space: « $ » never goes to a line of its own in a narrow card. */
  usd: '{amount} $',
  percent: '{n} %',
  duration: { hours: '{h}h {m}m', minutes: '{m}m {s}s' },
  countdown: { days: '{d}j {h}h', hours: '{h}h{m}' },
  ago: { now: 'à l’instant', minutes: 'il y a {n} min', hours: 'il y a {n} h', yesterday: 'hier', days: 'il y a {n} j' },
  /** To follow « depuis ». */
  since: { underMinute: '< 1 min', minutes: '{n} min', hours: '{n} h', days: '{n} j' },
  bytes: { megabytes: '{n} Mo', gigabytes: '{n} Go' },
  when: { today: 'à {time}', day: 'le {date} à {time}' },
  /** The keys of a shortcut, as written on the keyboard of the language. */
  keys: { shift: 'Maj', enter: 'Entrée', escape: 'Échap' },
} as const satisfies Tree;
