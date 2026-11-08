import { defineZone } from '../types';

// « Continuer sur <harness> » (ESC-88): an agent goes on on another harness by a handover, when the quota of its own is
// spent or when the user asks. `{harness}`: a harness's name (`engineName`).
export default defineZone('handover', {
  continueOn: 'Continue on {harness}',
  another: 'Continue on another harness',
  menu: 'Continue on another harness…',
  back: 'Go back to {harness}',
  backHint: 'The {harness} quota is back: the agent stays on its current harness until you bring it back.',
  waitTurn: 'Wait for the turn to end',
  separator: {
    limitedUntil: 'Continued on {harness} · {from} quota spent until {time}',
    limited: 'Continued on {harness} · {from} quota spent',
    asked: 'Continued on {harness} · the agent was on {from}',
  },
  file: {
    title: 'Handover file',
    hint: 'Written by Escouade from the conversation: the agent’s first message on {harness}.',
  },
  turnOn: 'on {harness}',
  settings: {
    onLimit: 'Continue on the next harness when every account of a harness is spent',
    onLimitDesc:
      'The agent moves on by itself, and the autopilot follows it for tickets. Otherwise it waits for the reset. The harness it left isn’t used again before its reset, and the agent never goes back to it by itself.',
    model: 'Handover model · {harness}',
    modelName: 'Handover model',
    modelDesc: 'The model of an agent that moves to this harness.',
    best: 'The best one ({model})',
  },
});
