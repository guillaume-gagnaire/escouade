import { defineZone } from '../types';

// The window on the remote API of the core (ESC-495): see the French catalog.
export default defineZone('remote', {
  band: {
    lost: 'Connection to the core lost · retrying…',
    stopped: 'Connection to the core refused: {reason}',
  },
  error: {
    deskOnly: '“{command}” is done on this computer, not at a distance.',
    notServedYet: '“{command}” is not served at a distance yet.',
    policy: 'The policies of this computer forbid the remote API.',
    token: 'The token of this device is missing, unknown or revoked.',
    version: 'The core and this window are not of the same version.',
    tooMany: 'Too many connections to the core are open.',
    busy: 'Too many calls waiting for their answer: try again in a moment.',
    tooLarge: 'Too large a sending for the core.',
    lost: 'Connection to the core lost before the answer: “{command}” may not have been done.',
    offline: 'Connection to the core lost: “{command}” was not sent.',
  },
  term: {
    lost: '[{size} lost during the cut]',
  },
});
