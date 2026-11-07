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
    default: { label: 'Ask', title: '{engine} asks for your approval before each sensitive action' },
    plan: { label: 'Plan', title: '{engine} analyzes and proposes a plan without changing anything' },
    acceptEdits: { label: 'Accept edits', title: 'File edits are accepted without asking' },
    bypassPermissions: { label: 'Bypass', title: 'No permission requests (only for safe environments)' },
    autoUnavailableDetail: 'unavailable with this model',
    autoUnavailableTitle: 'Auto mode isn’t available with this model, as this account’s Claude Code says',
    /** A mode its harness lacks, greyed in its place (choice 17): `{harness}`, `{mode}`. */
    missing: '{harness} has no {mode} mode.',
    missingAuto: '{harness} has no Auto mode: nothing decides for you.',
    bypassToast: 'Bypass mode: {engine} will act without any permission request.',
  },

  attachments: {
    kilobytes: '{n} KB',
    unreadable: 'Couldn’t read {name}',
    unsupported: '“{name}” can’t be attached. Supported files are images (PNG, JPEG, GIF, WebP), PDFs and text files.',
    tooBig: '{name} is over {max}',
    notText: '{name} isn’t a text file, so it can’t be attached.',
    remove: 'Remove {name}',
    copying: 'Copying {name}',
    warnCopy: 'Big file: {size}, copied to the disk',
    warnOrigin: 'Big file: {size}, attached where it is, not copied',
    original: 'Original file, read where it is',
  },
  attach: {
    label: 'Attach a file',
    title: 'Attach a file (or paste / drop it)',
  },
  /** Files dragged over the composer: the whole field takes them. */
  drop: {
    title: 'Drop to attach to your message',
    detail: 'Images, PDFs and texts go in the message; the rest, by its path',
  },

  placeholder: {
    denyPermission: 'Tell {engine} what to do instead (this denies the request)…',
    answerQuestion: 'Answer the question or write a free answer…',
    sendTo: 'Send a message to {name}…',
    describe: 'Describe the task to give to {engine}…',
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
    titleQueued: '{engine} takes it into account at its next step · {enter} to send',
    titleHeld: 'Sent to {engine} in {s} s, can be cancelled until then · {enter} to send',
    titleTurnEnd: '{engine} gets it at the end of its turn, can be cancelled until then · {enter} to send',
    title: '{enter} to send · {newline} for a new line',
    alreadyWaiting: 'A message is already waiting for the setup to finish.',
    filesKept: 'Attached files don’t go with an answer: they stay ready for your next message.',
  },

  permission: {
    title: '{engine} is asking for permission',
    titlePlan: '{engine} proposes a plan',
    keepPlanningMessage: 'Keep planning: the plan doesn’t work for me yet.',
    allow: 'Allow',
    approvePlan: 'Approve the plan',
    resumesIn: 'Work will resume in {mode} mode',
    forSession: 'For this session',
    sessionAllowed: 'Allowed for this session',
    forAgent: 'For this agent',
    agentAllowed: 'Allowed for this agent',
    always: 'Always allow',
    approvePlanAndEdits: 'Approve and accept edits',
    deny: 'Deny',
    keepPlanning: 'Keep planning',
    denyHint: 'To deny and explain what to do instead, write it in the field below.',
    denied: 'Denied',
    allowed: 'Allowed',
    alwaysAllowed: 'Always allowed',
    /** Answered through the MCP server, not by the user in the window: « par <author> », then « via Escouade ». */
    by: 'by',
    via: 'via Escouade',
    cancelled: 'Request canceled',
  },

  question: {
    waiting: '{engine} is waiting for your answer',
    asked: '{engine} asked you a question',
    orWrite: 'or answer freely in the field below',
    saidAnswer: 'Answer said: “{text}”',
    submit: 'Submit',
    unanswered: 'Question left unanswered',
    answered: 'Answered',
    form: 'Form from {source}',
    formNoSource: 'Form',
    required: 'required',
    fieldPlaceholder: 'Your answer',
    numberPlaceholder: 'A number',
  },

  model: {
    multiplier: '×{n}',
    multiplierTitle: 'Costs {n} times the base rate',
    reading: 'list being read',
    fromConfig: 'from config.toml',
    price: {
      low: 'low cost',
      medium: 'medium cost',
      high: 'high cost',
      veryHigh: 'very high cost',
    },
  },
});
