import { defineZone } from '../types';

// The background fetch (ESC-561): its settings (All projects › Git, and each project's Git), and what the Git panel
// says of it beside « Fetch ».
export default defineZone('fetch', {
  settings: {
    group: 'Git',
    auto: 'Fetch automatically (git fetch)',
    autoDesc:
      'In the background, Escouade fetches every repository that has a remote, one at a time, never asking for credentials: the commits to pull show up by themselves.',
    every: 'Interval',
    everyDesc: 'Between two fetches of a repository, 1 min to 24 h.',
    timeout: 'Time limit',
    timeoutDesc:
      'Past it, the background fetch is given up and tried again at the next interval, 30 s to 30 min. A fetch by hand keeps its 3 min.',
    project: 'Fetch automatically (git fetch)',
    projectDesc: 'This project’s background fetch, for a heavy repository or one behind a VPN.',
    modes: { default: 'As the general setting', always: 'Always', never: 'Never' },
    generalOn: 'General setting: every {every}, time limit of {timeout}.',
    generalOff: 'General setting: off.',
    openGeneral: 'General setting…',
    projectEvery: 'The project’s own interval',
    projectEveryDesc: 'Empty: the general setting’s ({every}).',
    projectTimeout: 'The project’s own time limit',
    projectTimeoutDesc: 'Empty: the general setting’s ({timeout}).',
    clamped: '{typed} is out of its bounds ({min} to {max}): brought back to {value}.',
    unitMin: 'min',
    unitSec: 's',
  },
  minutes: '{n} min',
  hours: '{n} h',
  hoursMinutes: '{h} h {m} min',
  seconds: '{n} s',
  minutesSeconds: '{m} min {s} s',
  panel: {
    fetched: 'Fetched {ago}',
    never: 'Never fetched',
    at: 'Last fetch: {when}',
    failed: 'Automatic fetch: {reason}',
    reasons: {
      credentials: 'it asks for credentials. Click « Fetch » once: it goes on after that.',
      timeout: 'not done within {timeout}. The time limit is set in the settings.',
      network: 'the network or the remote cannot be reached.',
      other: 'git refused (hover for why).',
    },
    openSettings: 'Settings',
    sshNote:
      'Your Git for Windows 2.47.0 has an ssh that stalls on big fetches: the automatic fetch goes through Windows’ own. Update Git (2.47.0(2) or later).',
  },
});
