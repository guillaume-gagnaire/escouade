import { defineZone } from '../types';

// The message field, and the cards that wait for an answer (permissions, questions).
export default defineZone('composer', {
  effort: {
    low: { label: 'Low', title: 'Quick answers, little thinking' },
    medium: { label: 'Medium', title: 'Balanced' },
    high: { label: 'High', title: 'Deep thinking' },
    xhigh: { label: 'Very high', title: 'Very deep thinking' },
    max: { label: 'Max', title: 'Maximum thinking, uses more tokens' },
  },
  mode: {
    auto: { label: 'Auto', title: 'A classifier approves safe actions and asks about the rest' },
    default: { label: 'Ask', title: 'Claude asks for your approval before each sensitive action' },
    plan: { label: 'Plan', title: 'Claude analyzes and proposes a plan without changing anything' },
    acceptEdits: { label: 'Accept edits', title: 'File edits are accepted without asking' },
    bypassPermissions: { label: 'Bypass', title: 'No permission requests (only for safe environments)' },
    autoUnavailableDetail: 'unavailable with Haiku',
    autoUnavailableTitle: 'Auto mode isn’t available with Haiku',
    bypassToast: 'Bypass mode: Claude will act without any permission request.',
  },

  attachments: {
    kilobytes: '{n} KB',
    unreadable: 'Couldn’t read {name}',
    unsupported: '“{name}” can’t be attached. Supported files are images (PNG, JPEG, GIF, WebP), PDFs and text files.',
    tooBig: '{name} is over {max}',
    notText: '{name} isn’t a text file, so it can’t be attached.',
    totalTooBig: '{name} wasn’t attached: the files of a message are limited to {max} in total.',
    remove: 'Remove {name}',
  },
  attach: {
    label: 'Attach a file',
    title: 'Attach an image, a PDF or a text file (or paste / drop it)',
  },

  placeholder: {
    denyPermission: 'Tell Claude what to do instead (this denies the request)…',
    answerQuestion: 'Answer the question or write a free answer…',
    sendTo: 'Send a message to {name}…',
    describe: 'Describe the task to give to Claude…',
  },

  bar: {
    effort: 'Effort',
    effortTitle: 'Thinking effort',
    effortUnsupported: 'Haiku doesn’t support effort',
    mode: 'Mode',
    interrupt: 'Interrupt ({key})',
    stop: 'Stop',
  },

  send: {
    label: 'Send',
    titleQueued: 'Claude takes it into account at its next step · {enter} to send',
    title: '{enter} to send · {newline} for a new line',
    alreadyWaiting: 'A message is already waiting for the setup to finish.',
    filesKept: 'Attached files don’t go with an answer: they stay ready for your next message.',
  },

  permission: {
    title: 'Claude is asking for permission',
    titlePlan: 'Claude proposes a plan',
    keepPlanningMessage: 'Keep planning: the plan doesn’t work for me yet.',
    allow: 'Allow',
    approvePlan: 'Approve the plan',
    always: 'Always allow',
    approvePlanAndEdits: 'Approve and accept edits',
    deny: 'Deny',
    keepPlanning: 'Keep planning',
    denyHint: 'To deny and explain what to do instead, write it in the field below.',
    denied: 'Denied',
    allowed: 'Allowed',
    alwaysAllowed: 'Always allowed',
    cancelled: 'Request canceled',
  },

  question: {
    waiting: 'Claude is waiting for your answer',
    orWrite: 'or answer freely in the field below',
    submit: 'Submit',
    unanswered: 'Question left unanswered',
  },
});
