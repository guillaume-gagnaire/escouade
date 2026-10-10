import { defineZone } from '../types';

// The units of `format.ts` in English: « 1.2k », « $0.10 », « 42% », « 4d 12h », « 312 MB ».
export default defineZone('format', {
  tokens: { thousand: '{n}k', million: '{n}M', billion: '{n}B' },
  usd: '${amount}',
  percent: '{n}%',
  duration: { hours: '{h}h {m}m', minutes: '{m}m {s}s' },
  countdown: { days: '{d}d {h}h', hours: '{h}h{m}' },
  ago: { now: 'just now', minutes: '{n} min ago', hours: '{n}h ago', yesterday: 'yesterday', days: '{n}d ago' },
  /** To follow “for”. */
  since: { underMinute: '< 1 min', minutes: '{n} min', hours: '{n}h', days: '{n}d' },
  bytes: { megabytes: '{n} MB', gigabytes: '{n} GB' },
  when: { today: 'at {time}', day: 'on {date} at {time}' },
  keys: { shift: 'Shift', enter: 'Enter', escape: 'Esc' },
});
