import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { agent, fakeBackend, resetApp } from '../test/ipc';
import {
  normalize,
  permissionAnswer,
  questionAnswer,
  turnOver,
  withoutInstruction,
  withoutPassage,
  type AnswerWords,
  type SpokenQuestion,
} from './conversation';
import { conversation, spokenWords } from './conversation.svelte';
import { conversationOf } from './conversations.svelte';
import { dictation } from './dictation.svelte';
import { setLang } from './i18n';
import { clearDrafts, draftIfAny, draftOf } from './question-answers.svelte';
import { app } from './state.svelte';
import type { Agent, ConvItem, PermissionItem, QuestionItem, VoiceEvent } from './types';

const FR = {
  yes: 'oui|ouais|ok|d’accord|vas-y|bien sûr',
  no: 'non|refuse|pas question',
  always: 'toujours',
} as AnswerWords;

describe('spoken answers', () => {
  it('reads words without case, accents nor punctuation', () => {
    expect(normalize('  Oui, D’accord !  ')).toBe("oui d'accord");
    expect(normalize('Évidemment…')).toBe('evidemment');
  });

  it('allows, denies or always allows a permission, and understands nothing that says both', () => {
    expect(permissionAnswer('Oui, vas-y.', FR, true)).toBe('allow');
    expect(permissionAnswer('D’accord', FR, true)).toBe('allow');
    expect(permissionAnswer('Non merci.', FR, true)).toBe('deny');
    expect(permissionAnswer('Oui, toujours.', FR, true)).toBe('always');
    // « Toujours » where the permission cannot be kept: once.
    expect(permissionAnswer('Toujours', FR, false)).toBe('allow');
    expect(permissionAnswer('Oui… enfin non.', FR, true)).toBeNull();
    expect(permissionAnswer('Je ne sais pas trop.', FR, true)).toBeNull();
    // A word inside another is not the word.
    expect(permissionAnswer('nonobstant', FR, true)).toBeNull();
  });

  describe('an answer said to a question (ESC-131)', () => {
    const one = (options: string[], free = false): SpokenQuestion => ({ options, multi: false, free });
    const several = (options: string[], free = false): SpokenQuestion => ({ options, multi: true, free });
    const picks = (heard: string, q: SpokenQuestion) => {
      const r = questionAnswer(heard, q, spokenWords());
      return r?.kind === 'answer' ? (r.picks.length ? r.picks : r.none ? 'none' : { text: r.text }) : r === null ? null : r.kind;
    };

    describe('in French', () => {
      beforeEach(() => setLang('fr'));

      it('finds the option by its name, near enough: no case nor accents, a word of it no other has', () => {
        const q = one(['SQLite', 'Postgres', 'Postgres avec Docker']);
        expect(picks('SQLite, s’il te plaît.', q)).toEqual(['SQLite']);
        expect(picks('Postgres avec Docker', q)).toEqual(['Postgres avec Docker']);
        expect(picks('Plutôt Postgres.', q)).toEqual(['Postgres']);
        expect(picks('sqlite', q)).toEqual(['SQLite']);
        expect(picks('Avec docker.', q)).toEqual(['Postgres avec Docker']);
        expect(picks('Une base éphémère', one(['Base persistante', 'Base éphémère']))).toEqual(['Base éphémère']);
        expect(picks('Je veux la version locale', one(['Version locale', 'Version distante']))).toEqual(['Version locale']);
        // The beginning of a word stands for it: « Postgres » for « PostgreSQL ».
        expect(picks('Postgres', one(['PostgreSQL', 'SQLite']))).toEqual(['PostgreSQL']);
      });

      it('finds it by its rank, the last one too, and a number said alone', () => {
        const q = one(['A', 'B', 'C']);
        expect(picks('La deuxième.', q)).toEqual(['B']);
        expect(picks('La troisième', q)).toEqual(['C']);
        expect(picks('Le premier', q)).toEqual(['A']);
        expect(picks('La dernière.', q)).toEqual(['C']);
        expect(picks('2', q)).toEqual(['B']);
        expect(picks('Option trois.', q)).toEqual(['C']);
        expect(picks('La une', q)).toEqual(['A']);
      });

      it('takes several for a question of several choices', () => {
        const q = several(['Tests unitaires', 'Tests e2e', 'Lint']);
        expect(picks('La première et la troisième.', q)).toEqual(['Tests unitaires', 'Lint']);
        expect(picks('Unitaires et lint', q)).toEqual(['Tests unitaires', 'Lint']);
        expect(picks('1 et 2', q)).toEqual(['Tests unitaires', 'Tests e2e']);
        expect(picks('Toutes.', q)).toEqual(['Tests unitaires', 'Tests e2e', 'Lint']);
        expect(picks('Aucune.', q)).toBe('none');
        expect(picks('Les deux', several(['Windows', 'macOS']))).toEqual(['Windows', 'macOS']);
      });

      it('takes what was said as the free answer when no option is named and the question takes one', () => {
        expect(picks('Escouade-front', one(['web-shop', 'admin'], true))).toEqual({ text: 'Escouade-front' });
        // The « un » of a sentence is no rank.
        expect(picks('Un thème sombre', one(['Clair', 'Système'], true))).toEqual({ text: 'Un thème sombre' });
        expect(picks('Mon appli', { options: [], multi: false, free: true })).toEqual({ text: 'Mon appli' });
        // An option named still wins.
        expect(picks('Admin', one(['web-shop', 'admin'], true))).toEqual(['admin']);
      });

      it('does not understand nothing, two options for one choice, a rank beyond the list, nor no option without a free answer', () => {
        const q = one(['SQLite', 'Postgres'], true);
        expect(picks('', q)).toBeNull();
        expect(picks('…', q)).toBeNull();
        expect(picks('SQLite ou Postgres', q)).toBeNull();
        expect(picks('La première et la deuxième', q)).toBeNull();
        expect(picks('La cinquième', q)).toBeNull();
        expect(picks('Aucune idée', one(['SQLite', 'Postgres']))).toBeNull();
      });

      it('hears « répète » and « annule » as commands, never as answers', () => {
        const q = one(['SQLite', 'Postgres'], true);
        expect(picks('Répète.', q)).toBe('repeat');
        expect(picks('Tu peux répéter ?', q)).toBe('repeat');
        expect(picks('Annule.', q)).toBe('cancel');
        expect(picks('Laisse tomber', q)).toBe('cancel');
      });
    });

    describe('in English', () => {
      beforeEach(() => setLang('en'));
      afterEach(() => setLang('fr'));

      it('finds options by name and rank, several, a free answer, and the commands', () => {
        const q = one(['SQLite', 'Postgres', 'Postgres with Docker'], true);
        expect(picks('Postgres with Docker, please.', q)).toEqual(['Postgres with Docker']);
        expect(picks('The second one.', q)).toEqual(['Postgres']);
        expect(picks('The last one', q)).toEqual(['Postgres with Docker']);
        expect(picks('Number one', q)).toEqual(['SQLite']);
        expect(picks('A dark theme', one(['Light', 'System'], true))).toEqual({ text: 'A dark theme' });
        expect(picks('The fifth', q)).toBeNull();
        const m = several(['Unit tests', 'E2E tests', 'Lint']);
        expect(picks('The first and the third', m)).toEqual(['Unit tests', 'Lint']);
        expect(picks('All of them', m)).toEqual(['Unit tests', 'E2E tests', 'Lint']);
        expect(picks('None', m)).toBe('none');
        expect(picks('Both', several(['Windows', 'macOS']))).toEqual(['Windows', 'macOS']);
        expect(picks('Say that again?', q)).toBe('repeat');
        expect(picks('Never mind.', q)).toBe('cancel');
      });
    });
  });

  it('shows a message and an answer without what the conversation added to them', () => {
    expect(withoutInstruction('Ajoute un test\n\n<escouade-voice>Conversation vocale : …</escouade-voice>')).toEqual({
      text: 'Ajoute un test',
      spoken: true,
    });
    expect(withoutInstruction('Un message <escouade-voice> au milieu')).toEqual({
      text: 'Un message <escouade-voice> au milieu',
      spoken: false,
    });
    expect(withoutPassage('Fait.\n\n<spoken>C’est fa')).toBe('Fait.');
    expect(withoutPassage('Rien à cacher')).toBe('Rien à cacher');
  });

  it('finds the end of the turn after the ones counted, and its passage', () => {
    const items: ConvItem[] = [
      { kind: 'turn', id: 't0', ts: 0, durationMs: 1, cost: 0, tokens: 0, isError: false, interrupted: false, error: null },
      { kind: 'user', id: 'u1', text: 'x', images: 0, ts: 1, queued: false },
      { kind: 'text', id: 'a', text: 'Réponse', spoken: 'Premier passage', streaming: false },
      { kind: 'text', id: 'b', text: 'Suite', spoken: 'Le passage', streaming: false },
    ];
    expect(turnOver(items, 1)).toBeNull();
    items.push({ kind: 'turn', id: 't1', ts: 2, durationMs: 1, cost: 0, tokens: 0, isError: false, interrupted: false, error: null });
    expect(turnOver(items, 1)?.spoken).toBe('Le passage');
    expect(turnOver(items, 0)?.turn.id).toBe('t0');
  });
});

describe('the credits', () => {
  it('name every voice the app offers, with its licences and the credit it asks for', async () => {
    const fs = await import('node:fs');
    const voices = JSON.parse(fs.readFileSync('crates/escouade-voice-local/src/speech/voices.json', 'utf8')) as {
      id: string;
      license: string;
      attribution: string | null;
      exception?: boolean;
    }[];
    const credits = fs.readFileSync('CREDITS.md', 'utf8');
    expect(voices.length).toBeGreaterThan(10);
    for (const v of voices) {
      const line = credits.split('\n').find((l) => l.startsWith(`| \`${v.id}\` |`));
      expect(line, v.id).toBeTruthy();
      expect(line).toContain(v.license);
      if (v.attribution) expect(line).toContain(v.attribution);
      // A voice offered by exception: what its licences really are.
      if (v.exception) {
        expect(line).toContain('aucune déclarée (exception)');
        expect(line).toContain('ElevenLabs v4');
      }
    }
    expect(credits).toContain('### Fuze, une exception');
    for (const engine of ['Piper', 'eSpeak NG', 'ONNX Runtime', 'whisper.cpp']) expect(credits).toContain(engine);
  });
});

describe('the conversation', () => {
  let backend: ReturnType<typeof fakeBackend>;
  let reading = 0;
  let a: Agent;
  const SESSION = 7;

  beforeEach(() => {
    setLang('fr');
    reading = 0;
    backend = fakeBackend({
      conversation_start: () => ({ kind: 'listening', session: SESSION }),
      conversation_say: () => ++reading,
      get_conversation: () => [],
    });
    a = agent({ id: 'a1', name: 'Alice' });
    resetApp({ agents: [a] });
  });

  afterEach(() => {
    conversation.clear();
    clearDrafts();
  });

  const ev = (e: Record<string, unknown>) => conversation.take({ session: SESSION, ...e } as VoiceEvent);
  const said = () => backend.called('conversation_say').map((c) => c.args.text);
  const tick = () => new Promise((r) => setTimeout(r, 0));

  async function startTalking() {
    await conversation.start('a1');
    expect(conversation.phase).toBe('listening');
  }

  it('listens, sends what was said once its silence came, and reads the passage of the answer', async () => {
    await startTalking();
    ev({ kind: 'convSpeech' });
    expect(conversation.phase).toBe('hearing');
    ev({ kind: 'convTranscribing' });
    expect(conversation.phase).toBe('transcribing');
    ev({ kind: 'convHeard', text: 'Ajoute un test', error: null });
    await tick();
    expect(conversation.phase).toBe('working');
    expect(backend.called('conversation_send')[0].args).toEqual({ id: 'a1', text: 'Ajoute un test' });
    // The agent works: nothing read yet.
    const items: ConvItem[] = [{ kind: 'user', id: 'u', text: 'Ajoute un test', images: 0, ts: 1, queued: false }];
    conversation.observe(a, items);
    expect(said()).toEqual([]);
    items.push(
      { kind: 'text', id: 'm', text: 'J’ai ajouté `a.test.ts`.', spoken: 'J’ai ajouté le test.', streaming: false },
      { kind: 'turn', id: 't', ts: 2, durationMs: 1, cost: 0, tokens: 0, isError: false, interrupted: false, error: null },
    );
    conversation.observe(a, items);
    await tick();
    expect(conversation.phase).toBe('speaking');
    expect(said()).toEqual(['J’ai ajouté le test.']);
    // Read to its end: it listens again (the backend does by itself).
    ev({ kind: 'convSpoken', reading: 1, error: null });
    expect(conversation.phase).toBe('listening');
    // Read once only.
    conversation.observe(a, items);
    expect(said()).toHaveLength(1);
  });

  it('says once that the voice on line failed, and pauses when no voice of Piper stands in (ESC-132)', async () => {
    backend = fakeBackend({
      conversation_start: () => ({ kind: 'listening', session: SESSION, notice: 'Aucune clé ElevenLabs. Piper lit.' }),
      conversation_say: () => ++reading,
      get_conversation: () => [],
    });
    await startTalking();
    expect(app.toasts.at(-1)).toMatchObject({ text: 'Aucune clé ElevenLabs. Piper lit.' });
    ev({ kind: 'convHeard', text: 'Résume', error: null });
    await tick();
    conversation.observe(a, [
      { kind: 'text', id: 'm', text: '…', spoken: 'Un résumé.', streaming: false },
      { kind: 'turn', id: 't', ts: 2, durationMs: 1, cost: 0, tokens: 0, isError: false, interrupted: false, error: null },
    ]);
    await tick();
    ev({ kind: 'convNotice', message: 'OpenAI ne répond pas. La conversation continue avec la voix locale (Piper).' });
    expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: expect.stringContaining('Piper') });
    expect(conversation.phase).toBe('speaking');
    ev({ kind: 'convSpoken', reading: 1, error: 'OpenAI refuse : quota. La conversation est en pause.', paused: true });
    expect(conversation.phase).toBe('paused');
    expect(app.toasts.at(-1)).toMatchObject({ kind: 'error', text: 'OpenAI refuse : quota. La conversation est en pause.' });
    expect(backend.called('conversation_hold').length).toBeGreaterThan(0);
    // « Reprendre »: it listens again.
    conversation.resume();
    expect(conversation.phase).toBe('listening');
  });

  it('says the answer is in the conversation when the agent wrote no passage', async () => {
    await startTalking();
    ev({ kind: 'convHeard', text: 'Bonjour', error: null });
    await tick();
    conversation.observe(a, [
      { kind: 'text', id: 'm', text: 'Bonjour !', streaming: false },
      { kind: 'turn', id: 't', ts: 2, durationMs: 1, cost: 0, tokens: 0, isError: false, interrupted: false, error: null },
    ]);
    expect(said()).toEqual(['C’est fait. La réponse est dans la conversation.']);
  });

  it('speaking over the reading cuts it: what is said next is heard', async () => {
    await startTalking();
    ev({ kind: 'convHeard', text: 'Résume', error: null });
    await tick();
    conversation.observe(a, [
      { kind: 'text', id: 'm', text: '…', spoken: 'Un long résumé.', streaming: false },
      { kind: 'turn', id: 't', ts: 2, durationMs: 1, cost: 0, tokens: 0, isError: false, interrupted: false, error: null },
    ]);
    await tick();
    ev({ kind: 'convBargeIn', reading: 1 });
    expect(conversation.phase).toBe('hearing');
    // The end of the reading cut off never comes; if it did, it would not count.
    ev({ kind: 'convSpoken', reading: 1, error: null });
    expect(conversation.phase).toBe('hearing');
    ev({ kind: 'convHeard', text: 'Non, plutôt la page d’accueil', error: null });
    await tick();
    expect(backend.called('conversation_send').map((c) => c.args.text)).toEqual(['Résume', 'Non, plutôt la page d’accueil']);
  });

  it('nothing understood: it listens again without sending anything', async () => {
    await startTalking();
    ev({ kind: 'convHeard', text: '  ', error: null });
    await tick();
    expect(conversation.phase).toBe('listening');
    expect(backend.called('conversation_send')).toHaveLength(0);
    expect(backend.called('conversation_listen')).toHaveLength(1);
  });

  it('reads a permission and a spoken « oui » allows it', async () => {
    await startTalking();
    ev({ kind: 'convHeard', text: 'Supprime le build', error: null });
    await tick();
    const perm: PermissionItem = {
      kind: 'permission',
      id: 'p1',
      toolUseId: 'tu',
      toolName: 'Bash',
      input: { command: 'rm -rf build' },
      canAlways: true,
      defaultNo: false,
      decision: null,
      ts: 3,
      title: null,
    };
    conversation.observe({ ...a, pending: ['p1'] }, [perm]);
    expect(conversation.phase).toBe('question');
    expect(said()[0]).toBe('Alice demande la permission : Bash rm -rf build. Tu autorises ?');
    ev({ kind: 'convSpoken', reading: 1, error: null });
    expect(conversation.phase).toBe('question');
    ev({ kind: 'convHeard', text: 'Oui, vas-y.', error: null });
    await tick();
    expect(backend.called('answer_permission')[0].args).toMatchObject({ id: 'a1', requestId: 'p1', decision: 'allow' });
    expect(conversation.phase).toBe('working');
  });

  /** A request of the agent: its questions, as its harness gives them. */
  const request = (id: string, questions: QuestionItem['questions'], extra: Partial<QuestionItem> = {}): QuestionItem => ({
    kind: 'question',
    id,
    toolUseId: 'tu',
    questions,
    answers: null,
    ts: 3,
    ...extra,
  });
  const later = () => new Promise((r) => setTimeout(r, 5));

  /** It speaks to the agent, which asks `q`. */
  async function asked(q: QuestionItem) {
    await startTalking();
    ev({ kind: 'convHeard', text: 'Configure le projet', error: null });
    await tick();
    a = { ...a, pending: [q.id] };
    app.agents[a.id] = a;
    conversation.observe(a, [q]);
  }

  /** The reading under way ends, then `text` is heard. */
  async function answers(text: string) {
    ev({ kind: 'convSpoken', reading, error: null });
    ev({ kind: 'convHeard', text, error: null });
    await tick();
  }

  it('asks a single question aloud, without its choices, and the option said answers it', async () => {
    conversation.flashMs = 0;
    await asked(request('q1', [{ question: 'Quelle base de données ?', options: [{ label: 'SQLite' }, { label: 'Postgres' }] }]));
    expect(said()[0]).toBe('Quelle base de données ?');
    expect(conversation.ask).toMatchObject({ index: 0, total: 1, multi: false, free: true });
    await answers('La deuxième.');
    expect(draftIfAny('q1')?.picks['Quelle base de données ?']).toEqual(['Postgres']);
    await later();
    expect(backend.called('answer_question')[0].args).toMatchObject({ answers: { 'Quelle base de données ?': 'Postgres' } });
    expect(conversation.phase).toBe('working');
    expect(conversation.ask).toBeNull();
  });

  it('asks a request of three questions one at a time, and sends the three answers together after the last', async () => {
    conversation.flashMs = 0;
    const q = request('q3', [
      { question: 'Quelle base ?', header: 'Base', options: [{ label: 'PostgreSQL' }, { label: 'SQLite' }] },
      {
        question: 'Quels tests ?',
        header: 'Tests',
        multiSelect: true,
        options: [{ label: 'Unitaires' }, { label: 'E2E' }, { label: 'Lint' }],
      },
      { question: 'Quel nom ?', header: 'Nom', options: [{ label: 'web-shop' }, { label: 'admin' }] },
    ]);
    await asked(q);
    expect(said().at(-1)).toBe('Question 1 sur 3. Quelle base ?');
    // A rank beyond the list: not understood, asked again.
    await answers('La cinquième.');
    expect(said().at(-1)).toBe('Je n’ai pas compris. Question 1 sur 3. Quelle base ?');
    expect(conversation.missed).toBe(true);
    await answers('SQLite');
    expect(draftIfAny('q3')?.picks['Quelle base ?']).toEqual(['SQLite']);
    await later();
    expect(said().at(-1)).toBe('Question 2 sur 3. Quels tests ? Plusieurs réponses possibles.');
    expect(conversation.ask).toMatchObject({ index: 1, multi: true });
    expect(conversation.missed).toBe(false);
    await answers('La première et la troisième.');
    expect(draftIfAny('q3')?.picks['Quels tests ?']).toEqual(['Unitaires', 'Lint']);
    await later();
    expect(said().at(-1)).toBe('Question 3 sur 3. Quel nom ?');
    await answers('Escouade-front');
    expect(draftIfAny('q3')?.texts['Quel nom ?']).toBe('Escouade-front');
    await later();
    expect(backend.called('answer_question')).toHaveLength(1);
    expect(backend.called('answer_question')[0].args).toMatchObject({
      requestId: 'q3',
      answers: { 'Quelle base ?': 'SQLite', 'Quels tests ?': 'Unitaires, Lint', 'Quel nom ?': 'Escouade-front' },
    });
    // What was read never listed the choices.
    for (const s of said()) expect(s).not.toMatch(/PostgreSQL|Unitaires|web-shop|choix sont/);
  });

  it('lights what was understood a moment before the next question', async () => {
    await asked(
      request('q4', [
        { question: 'A ?', options: [{ label: 'Oui' }, { label: 'Non' }] },
        { question: 'B ?', options: [{ label: 'Un' }, { label: 'Deux' }] },
      ]),
    );
    expect(conversation.flashMs).toBe(700);
    conversation.flashMs = 60;
    await answers('Non');
    expect(conversation.understood).toEqual({ picks: ['Non'], text: null });
    expect(conversation.ask?.index).toBe(0);
    const before = said().length;
    await later();
    expect(said()).toHaveLength(before);
    await new Promise((r) => setTimeout(r, 80));
    expect(said().at(-1)).toBe('Question 2 sur 2. B ?');
    expect(conversation.understood).toBeNull();
  });

  it('asks again an answer not understood, twice at most, then pauses on the question, the answers given kept', async () => {
    conversation.flashMs = 0;
    await asked(
      request('q5', [
        { question: 'Quelle base ?', options: [{ label: 'SQLite' }, { label: 'Postgres' }] },
        { question: 'Quel port ?', options: [{ label: '5432' }, { label: '5433' }] },
      ]),
    );
    await answers('SQLite');
    await later();
    await answers('Le premier ou le deuxième ?');
    expect(said().at(-1)).toBe('Je n’ai pas compris. Question 2 sur 2. Quel port ?');
    await answers('La septième');
    expect(said().at(-1)).toBe('Je n’ai pas compris. Question 2 sur 2. Quel port ?');
    await answers('La neuvième');
    expect(said().at(-1)).toBe('Je n’ai pas compris ta réponse. La conversation est en pause : réponds dans la fenêtre.');
    ev({ kind: 'convSpoken', reading, error: null });
    expect(conversation.phase).toBe('paused');
    expect(backend.called('answer_question')).toHaveLength(0);
    expect(draftIfAny('q5')?.picks['Quelle base ?']).toEqual(['SQLite']);
    // « Reprendre »: the question it paused on, asked again.
    conversation.resume();
    expect(said().at(-1)).toBe('Question 2 sur 2. Quel port ?');
    expect(conversation.phase).toBe('question');
  });

  it('« répète » asks the question again; « annule » leaves the card to be answered in the window', async () => {
    await asked(request('q6', [{ question: 'Quelle base ?', options: [{ label: 'SQLite' }, { label: 'Postgres' }] }]));
    await answers('Répète, s’il te plaît.');
    expect(said()).toEqual(['Quelle base ?', 'Quelle base ?']);
    expect(conversation.ask?.misses).toBe(0);
    await answers('Laisse tomber.');
    expect(said().at(-1)).toBe('D’accord : la question attend ta réponse dans la fenêtre.');
    expect(conversation.ask).toBeNull();
    ev({ kind: 'convSpoken', reading, error: null });
    expect(conversation.phase).toBe('paused');
    expect(backend.called('answer_question')).toHaveLength(0);
    // Answered in the window: back to waiting for the agent.
    a = { ...a, pending: [] };
    app.agents[a.id] = a;
    conversation.observe(a, []);
    expect(conversation.phase).toBe('working');
  });

  it('takes an option chosen in the window (the overlay, its key, the card) and goes on to the next question', async () => {
    conversation.flashMs = 0;
    await asked(
      request('q7', [
        { question: 'Quelle base ?', options: [{ label: 'SQLite' }, { label: 'Postgres' }] },
        { question: 'Quels tests ?', multiSelect: true, options: [{ label: 'Unitaires' }, { label: 'E2E' }] },
        { question: 'Quel port ?', options: [{ label: '5432' }, { label: '5433' }] },
      ]),
    );
    conversation.choose(1);
    expect(draftIfAny('q7')?.picks['Quelle base ?']).toEqual(['Postgres']);
    await later();
    expect(said().at(-1)).toBe('Question 2 sur 3. Quels tests ? Plusieurs réponses possibles.');
    // Several choices: each click ticks, « Valider » takes them.
    conversation.choose(0);
    conversation.choose(1);
    conversation.choose(0);
    expect(draftIfAny('q7')?.picks['Quels tests ?']).toEqual(['E2E']);
    expect(conversation.ask?.index).toBe(1);
    conversation.validate();
    await later();
    expect(said().at(-1)).toBe('Question 3 sur 3. Quel port ?');
    // Ticked by hand in the card.
    draftOf('q7').picks['Quel port ?'] = ['5433'];
    conversation.answeredInWindow('q7');
    await later();
    expect(backend.called('answer_question')[0].args).toMatchObject({
      answers: { 'Quelle base ?': 'Postgres', 'Quels tests ?': 'E2E', 'Quel port ?': '5433' },
    });
  });

  it('the request answered entirely in the window meanwhile: back to waiting for the agent', async () => {
    await asked(
      request('q8', [
        { question: 'A ?', options: [{ label: 'X' }] },
        { question: 'B ?', options: [{ label: 'Y' }] },
      ]),
    );
    ev({ kind: 'convSpoken', reading, error: null });
    a = { ...a, pending: [] };
    app.agents[a.id] = a;
    conversation.observe(a, []);
    expect(conversation.phase).toBe('working');
    expect(conversation.ask).toBeNull();
  });

  it('asks aloud the single question of every harness: Claude Code, Codex, Copilot CLI (with no choices too), OpenCode', async () => {
    const cases: [string, QuestionItem, string, Record<string, string>][] = [
      [
        'Claude Code (AskUserQuestion)',
        request('c1', [
          {
            question: 'Quelle base ?',
            header: 'Base',
            multiSelect: false,
            options: [
              { label: 'PostgreSQL', description: 'Relationnelle' },
              { label: 'SQLite', description: 'Embarquée' },
            ],
          },
        ]),
        'SQLite',
        { 'Quelle base ?': 'SQLite' },
      ],
      [
        'Codex (requestUserInput), its question with its id',
        request('c2', [
          {
            id: 'lang',
            question: 'Quel langage ?',
            header: 'Langage',
            multiSelect: false,
            options: [
              { label: 'Rust', description: '' },
              { label: 'Go', description: '' },
            ],
          },
        ]),
        'Le premier',
        { 'Quel langage ?': 'Rust' },
      ],
      [
        'Copilot CLI (ask_user), a question asked in text, without choices',
        request('c3', [{ question: 'Quel nom pour la branche ?', header: '', multiSelect: false, options: [] }]),
        'fix-login',
        { 'Quel nom pour la branche ?': 'fix-login' },
      ],
      [
        'OpenCode, several choices',
        request('c4', [
          {
            question: 'Quels systèmes ?',
            header: '',
            multiSelect: true,
            options: [
              { label: 'Windows', description: '' },
              { label: 'Linux', description: '' },
            ],
          },
        ]),
        'Les deux',
        { 'Quels systèmes ?': 'Windows, Linux' },
      ],
      [
        'a form of an MCP server (Copilot: elicitation), a field to fill',
        request('c5', [{ id: 'port', question: 'Port', multiSelect: false, options: [], field: 'number', required: true }], {
          form: { message: 'Configuration', source: 'db' },
        }),
        '5432',
        { Port: '5432' },
      ],
    ];
    for (const [harness, q, heard, sent] of cases) {
      a = agent({ id: 'a1', name: 'Alice' });
      resetApp({ agents: [a] });
      conversation.clear();
      conversation.flashMs = 0;
      reading = 0;
      backend = fakeBackend({
        conversation_start: () => ({ kind: 'listening', session: SESSION }),
        conversation_say: () => ++reading,
        get_conversation: () => [],
      });
      await asked(q);
      expect(said()[0], harness).toBe(q.questions[0].question + (q.questions[0].multiSelect ? ' Plusieurs réponses possibles.' : ''));
      await answers(heard);
      await later();
      expect(backend.called('answer_question')[0]?.args, harness).toMatchObject({ requestId: q.id, answers: sent });
    }
  });

  it('asks the question of an agent that asks while it listens (the agent of a ticket), and one waiting at the start', async () => {
    await startTalking();
    const q = request('t1', [{ question: 'On continue ?', options: [{ label: 'Oui' }, { label: 'Non' }] }]);
    a = { ...a, pending: ['t1'] };
    app.agents[a.id] = a;
    conversation.observe(a, [q]);
    expect(said()[0]).toBe('On continue ?');
    expect(conversation.phase).toBe('question');
    // Already waiting when the conversation starts.
    conversation.clear();
    reading = 0;
    backend = fakeBackend({
      conversation_start: () => ({ kind: 'listening', session: SESSION }),
      conversation_say: () => ++reading,
      get_conversation: () => [q],
    });
    conversationOf('a1').items = [q];
    await conversation.start('a1');
    expect(said()[0]).toBe('On continue ?');
  });

  it('asks for the model, then for the voice, before it listens; « Arrêter » closes the microphone', async () => {
    let step = 0;
    backend = fakeBackend({
      conversation_start: () =>
        [
          { kind: 'needsVoice', voice: 'fr_FR-siwis-medium', size: 63_206_169 },
          { kind: 'listening', session: SESSION },
        ][step++],
      speech_download: () => null,
      get_conversation: () => [],
    });
    await conversation.start('a1');
    expect(conversation.phase).toBe('offer');
    expect(conversation.offer).toEqual({ kind: 'voice', id: 'fr_FR-siwis-medium', size: 63_206_169 });
    await conversation.download();
    expect(backend.called('speech_download')[0].args).toEqual({ id: 'fr_FR-siwis-medium' });
    expect(conversation.phase).toBe('listening');
    conversation.stop();
    expect(conversation.phase).toBe('idle');
    expect(backend.called('conversation_stop')).toHaveLength(1);
  });

  it('a dictation under way is called off by a conversation', async () => {
    dictation.phase = 'listening';
    await startTalking();
    expect(dictation.phase).toBe('idle');
  });

  it('keeps the transcript of the turn read for the right agent only', async () => {
    await startTalking();
    conversationOf('a1');
    ev({ kind: 'convHeard', text: 'Bonjour', error: null });
    await tick();
    conversation.observe(agent({ id: 'other' }), [
      { kind: 'turn', id: 't', ts: 2, durationMs: 1, cost: 0, tokens: 0, isError: false, interrupted: false, error: null },
    ]);
    expect(said()).toEqual([]);
  });
});
