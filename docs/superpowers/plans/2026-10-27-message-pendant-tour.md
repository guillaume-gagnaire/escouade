# Message pendant un tour : modifiable et annulable avant l'envoi — plan d'implémentation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## Contexte

Aujourd'hui, un message posté pendant un tour part **tout de suite** au process (`src-tauri/src/core.rs` `deliver` → `proc.send_message`) :
- **Claude** le met dans sa propre file et le lit à l'étape suivante (SPEC.md:199).
- **Codex et Copilot** le lisent à l'étape suivante (`turn/steer`, `MidTurn::Immediate`).
- **Cursor et OpenCode** : c'est le process d'Escouade qui le garde jusqu'à la fin du tour (`queue` / `held` dans `engine/{cursor,opencode}/process.rs`).

Une fois le message dans le harness, rien ne permet de le retirer : chez Claude, le seul recours est `interrupt` + `cancel_queued`, qui coupe le tour et vide toute la file. L'utilisateur veut pouvoir **modifier ou annuler** un message posté pendant un tour, avant que l'agent ne le reçoive.

**Design approuvé en chat (approche A : la file vit côté fenêtre) :**
- Un message tapé dans le composer pendant un tour est gardé par la fenêtre, dans une file d'envoi (*outbox*). Il apparaît en bulle « en attente », avec ses actions.
- Il part ensuite par le chemin actuel (`api.sendMessage`), inchangé. Le backend ne le voit qu'au moment où il part, si bien que le journal de conversation, qui n'accepte qu'ajouts et patches, n'a rien à retirer.
- **Mode Délai** (Claude `queued`, Codex/Copilot `immediate`) : le message part au bout d'un délai réglable, « Délai avant l'envoi pendant un tour » : Désactivé / 5 / **10 (défaut)** / 20 / 30 s. « Désactivé » redonne le comportement actuel.
- **Mode Fin de tour** (Cursor/OpenCode, nouveau `MidTurn::TurnEnd`) : le message part à la fin du tour, un seul par fin de tour. Ce mode ne dépend pas du réglage.
- **Actions :**
  - « Modifier » : le texte et les pièces jointes reviennent dans le champ, devant ce qui a été tapé depuis.
  - « Annuler » : le message est abandonné, sans confirmation.
  - « Envoyer maintenant » : en mode Délai seulement.
- **Stop ou fin du tour pendant un délai :** le compte à rebours continue.
- **Agent archivé, app quittée, fenêtre rechargée :** les messages gardés reviennent dans le brouillon (texte gardé ; pièces jointes perdues, comme pour un brouillon). Agent supprimé : ils sont abandonnés.
- **Envoi refusé à l'échéance :** le message revient dans le champ, avec un toast (comme aujourd'hui).
- **Hors file :**
  - les réponses à une question ou à une demande d'autorisation ;
  - un harness `refused` ;
  - la préparation d'un worktree (`holdMessage`, inchangé) ;
  - les messages venus du MCP, de claude.ai, d'un autre agent ou de la conversation vocale.

**Spec:** `docs/superpowers/specs/2026-10-27-message-pendant-tour-design.md`. Écrite en tâche 0, à partir de cette section et de la section 1/3 du design approuvé en chat.

**Goal :** rendre modifiable et annulable un message posté pendant un tour, tant qu'il n'a pas été transmis à l'agent.

**Architecture :**
- Un module de file d'envoi côté Svelte (`src/lib/outbox.svelte.ts`) garde les messages par agent, avec un minuteur (mode Délai) ou un crochet de fin de tour (mode Fin de tour).
- Il les lâche par le chemin d'envoi existant.
- Côté Rust, deux ajouts seulement : le variant `MidTurn::TurnEnd` et le réglage `midTurnDelaySecs`.

**Tech Stack :** Svelte 5 (runes), TypeScript, Vitest + Testing Library, Rust (Tauri 2), `cargo test`.

**Exécution recommandée : Native** (superpowers:executing-plans), suivie d'une relecture de toute la branche. Les tâches 4 et 5 consomment directement les interfaces des tâches 1 à 3, et un seul contexte les tient mieux que 7 sous-agents à froid. Une erreur coûte peu tant que la PR n'est pas fusionnée.

## Global Constraints

- Branche partie de `origin/main` (`git fetch origin` d'abord), poussée sur `fork`, PR vers `guillaume-gagnaire/escouade:main` (`gh pr create --repo guillaume-gagnaire/escouade --head Metalzoid:<branche>`).
- Commits `type(scope): phrase longue en français`, avec un corps en français et la ligne `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commentaires de code en anglais, dans le ton du fichier.
- Toute chaîne d'interface existe dans `src/lib/i18n/fr/*` **et** `src/lib/i18n/en/*`. Les messages Rust passent par `tr!("fr", "en")`.
- Valeurs permises du réglage : `0, 5, 10, 20, 30` (secondes) ; défaut `10` ; toute autre valeur devient `10`.
- Changements visibles décrits dans `CHANGELOG.md` et `CHANGELOG.en.md` (`## [Non publié]` › Nouveautés) et dans `docs/SPEC.md`.
- Vérifs locales :
  - `npm test`, `npm run check`, `npm run format:check` ;
  - `PATH=<scratchpad>/venv/bin:$PATH cargo test` dans `src-tauri` (cmake absent du Mac : voir la mémoire `escouade-local-test-quirks`) ;
  - échec connu, et sans rapport : `attachments.test.ts` › encodages Windows.

## Review Focus

1. **Ordre.** Un message envoyé au repos alors qu'un message plus ancien est encore gardé ne doit pas le doubler. Il est gardé lui aussi, dans le mode du dernier message gardé. Test en tâche 4.
2. **Agent archivé avec des messages gardés.** Rien ne part vers l'agent archivé ; tout revient dans son brouillon, y compris si un minuteur arrive à échéance après l'archivage. Test en tâche 4.
3. **Rechargement ou fermeture pendant l'attente.** Le texte est remis dans le brouillon **et écrit dans les préférences** avant `flushDrafts` (`pagehide`, `onRestartWarned`). Test en tâche 4.
4. **Refus du backend à l'échéance.** Le message revient dans le champ et un toast s'affiche. Les messages gardés suivants partent quand même, dans l'ordre. Test en tâche 4.
5. **« Modifier » alors que le champ contient déjà du texte et des pièces jointes.** Le texte gardé passe devant, séparé par une ligne vide ; les pièces jointes sont fusionnées, les siennes d'abord. Le Composer affiché se met à jour. Test en tâche 3 (drafts) et en tâche 5 (Composer).

---

## Fichiers

| Fichier | Rôle |
|---|---|
| `src-tauri/src/engine/registry.rs` | variant `MidTurn::TurnEnd` ; doc de `Queued` corrigée |
| `src-tauri/src/engine/{cursor,opencode}/mod.rs` | `mid_turn: MidTurn::TurnEnd` |
| `src-tauri/src/model.rs` | champ `mid_turn_delay_secs` (défaut 10) et `normalize_mid_turn_delay()` |
| `src-tauri/src/core.rs` (~1671, ~2133) | appelle la normalisation au chargement et à l'enregistrement |
| `src/lib/types.ts` | `midTurn` accepte `'turnEnd'` ; `Settings.midTurnDelaySecs` |
| `src/lib/settings.svelte.ts`, `src/components/settings/AppTab.svelte` | rangée du réglage, groupe « Tous les harness » |
| `src/lib/drafts.ts` | `giveBack` (déplacé du Composer) et signal `app.draftReturned` |
| `src/lib/send.svelte.ts` (nouveau) | `sendDraft`, `reportRefusal`, partagés par le Composer et la file d'envoi |
| `src/lib/state.svelte.ts` | `onTurnEnded(f)`, `draftReturned` |
| `src/lib/outbox.svelte.ts` (nouveau) | la file d'envoi |
| `src/components/conv/HeldMessage.svelte` (nouveau) | bulle gardée : compte à rebours et actions |
| `src/components/conv/UserMessage.svelte` | prop optionnelle `extra` (snippet), rendue dans `.meta` |
| `src/components/Conversation.svelte` (~802, ~912) | affiche les bulles gardées après `conv.waiting` |
| `src/components/Composer.svelte` | `send()` garde le message ; recharge le brouillon sur `app.draftReturned` ; infobulle |
| i18n `fr`/`en` : `conv.ts`, `composer.ts`, `settings.ts` | textes |
| `docs/SPEC.md` (199, 703), `docs/ENGINES.md` (82, 145), `CHANGELOG*.md` | documentation |

---

### Task 0 : branche, spec et plan dans le dépôt

- [ ] `git fetch origin && git switch -c feat/message-pendant-tour origin/main`
- [ ] Écrire `docs/superpowers/specs/2026-10-27-message-pendant-tour-design.md`, en français, dans la forme des specs voisines (`2026-10-26-branche-agent-actif-design.md`). Contenu : le contexte et le design de ce plan, les 5 points de *Review Focus* comme cas limites, et l'approche A avec la raison du rejet de B et C (B : file côté cœur, 3 à 4 fois plus gros ; C : retrait natif, il faudrait un état « annulé » dans le journal).
- [ ] Copier ce plan dans `docs/superpowers/plans/2026-10-27-message-pendant-tour.md`.
- [ ] Commit : `docs(composer): spec et plan — un message posté pendant un tour, modifiable et annulable avant l'envoi`

### Task 1 : `MidTurn::TurnEnd` pour Cursor et OpenCode

**Files :** `src-tauri/src/engine/registry.rs:58-70`, `engine/cursor/mod.rs:145,908`, `engine/opencode/mod.rs:381`, `engine/registry_tests.rs`, `src/lib/types.ts:603`, `src/test/ipc.ts` (`CURSOR_ENGINE`), `docs/ENGINES.md:82,145`

**Interfaces — Produces :**
- `MidTurn::TurnEnd`, sérialisé `"turnEnd"` ;
- côté front : `EngineCaps.midTurn: 'queued' | 'immediate' | 'turnEnd' | 'refused'`.

- [ ] **Test (Rust), dans `registry_tests.rs` :** `the_harnesses_that_hold_a_message_until_the_turn_ends_say_so` :
  - les caps de Cursor et d'OpenCode valent `MidTurn::TurnEnd` ;
  - Claude reste `Queued`, Codex et Copilot `Immediate` ;
  - `serde_json::to_value(MidTurn::TurnEnd) == json!("turnEnd")`.

  Remplacer aussi l'assertion de `cursor/mod.rs:908` par `TurnEnd`.
- [ ] Lancer `cargo test the_harnesses_that_hold` : il doit échouer (variant inconnu).
- [ ] Ajouter le variant `TurnEnd`, documenté « It waits in Escouade's process for the end of the turn, then starts the next one (Cursor, OpenCode) ». Corriger la doc de `Queued` : « Its harness queues it and reads it at the running turn's next step (Claude Code) ». Passer Cursor et OpenCode à `TurnEnd`.

  `agent.rs:1006-1009` ne change pas : seul `Immediate` y est comparé, donc `TurnEnd` reste « mis en file » côté backend, et les messages du MCP gardent leur comportement.
- [ ] Front : ajouter `'turnEnd'` dans `types.ts` ; `CURSOR_ENGINE.caps.midTurn = 'turnEnd'` dans `src/test/ipc.ts`. `ENGINES.md` : table (`turnEnd` : « gardé par Escouade jusqu'à la fin du tour ») et phrase de la ligne 82.
- [ ] Lancer `cargo test registry` et `cursor`, puis `npm run check` : tout doit passer.
- [ ] Commit : `feat(engines): Cursor et OpenCode disent qu'un message envoyé pendant un tour attend sa fin (MidTurn::TurnEnd)`

### Task 2 : le réglage « Délai avant l'envoi pendant un tour »

**Files :** `src-tauri/src/model.rs` (struct ~136, défaut ~609, `impl Settings` ~485, tests ~2980), `src-tauri/src/core.rs:1671,2133`, `src/lib/types.ts` (`Settings`), `src/lib/settings.svelte.ts:87-99`, `src/components/settings/AppTab.svelte` (groupe `allHarnesses`, ligne ~295, après la rangée `idleStop`), `src/lib/i18n/{fr,en}/settings.ts`, `src/test/ipc.ts` (`SETTINGS`), `docs/SPEC.md:703`

**Interfaces — Produces :**
- `app.settings.midTurnDelaySecs: number` (0 = désactivé) ;
- Rust : `Settings::mid_turn_delay_secs: u32` et `Settings::normalize_mid_turn_delay(&mut self)`.

- [ ] **Test (Rust), dans `model.rs` :** `the_delay_before_a_message_sent_during_a_turn_is_ten_seconds_and_one_of_its_steps` :
  - `Settings::default().mid_turn_delay_secs == 10` ;
  - `serde_json::from_value::<Settings>(json!({}))` donne 10 ;
  - après `normalize_mid_turn_delay()`, 7 devient 10, et 0, 5, 20, 30 restent tels quels ;
  - la sérialisation produit `v["midTurnDelaySecs"] == json!(10)`.
- [ ] Lancer `cargo test the_delay_before` : il doit échouer.
- [ ] Ajouter le champ (doc : « « Délai avant l'envoi pendant un tour », in seconds (0: off): a message typed during a turn waits this long in the window, where it can be edited or cancelled (`outbox.svelte.ts`) »), son défaut 10, et `normalize_mid_turn_delay`. L'appeler juste après `normalize_attachments()` dans `save_settings`, et à côté de `normalize_display()` au chargement (`core.rs:1671`).
- [ ] Front :
  - `Settings.midTurnDelaySecs: number` ; `'midTurnDelaySecs'` dans `SETTINGS_OF.claude` ;
  - `SETTINGS` des tests à `midTurnDelaySecs: 0`, avec le commentaire `// The tests opt in to the delay: a message sent during a turn goes at once, as before.` ;
  - dans AppTab, une rangée `<Row label={t('settings.claude.midTurnDelay')} desc={t('settings.claude.midTurnDelayDesc')}>` portant `<Chips>` aux valeurs numériques `[0, 5, 10, 20, 30]`.
- [ ] i18n (texte figé) :

  | clé | fr | en |
  |---|---|---|
  | `midTurnDelay` | Délai avant l’envoi pendant un tour | Delay before sending during a turn |
  | `midTurnDelayDesc` | Un message envoyé pendant qu’un agent travaille attend ce délai : tu peux le modifier ou l’annuler d’ici là. Cursor et OpenCode le gardent jusqu’à la fin du tour, quel que soit ce réglage. | A message sent while an agent works waits this long: you can edit or cancel it until then. Cursor and OpenCode keep it until the end of the turn, whatever this setting. |
  | `midTurnDelayOff` | Désactivé | Off |
  | `midTurnDelaySecs` | `{s} s` | `{s} s` |
- [ ] **Test (Vitest),** dans le fichier de test des réglages qui couvre l'onglet Harness (`SettingsModal.test.ts`, celui qui teste `attachKeepDays`) : `saves the delay before a message sent during a turn` : un clic sur « 20 s » doit envoyer `save_settings` avec `midTurnDelaySecs: 20`.
- [ ] SPEC.md:703 : ajouter le réglage dans la liste de Réglages › Harness, groupe « Tous les harness ».
- [ ] Lancer `cargo test model`, `npm test -- settings`, puis `npm run check` : tout doit passer.
- [ ] Commit : `feat(settings): « Délai avant l'envoi pendant un tour », de désactivé à 30 s, 10 s par défaut`

### Task 3 : rendre au brouillon et envoyer hors du Composer (refactor sans changement visible)

**Files :** `src/lib/drafts.ts`, `src/lib/state.svelte.ts` (~294, à côté de `composerPut`), `src/lib/send.svelte.ts` (nouveau), `src/components/Composer.svelte:509-583`, `src/lib/drafts.test.ts`

**Interfaces — Produces :**
- `giveBack(owner: string, back: string, backFiles: DraftAttachment[]): void` dans `drafts.ts`. Il fusionne `{ text: since.text.trim() ? `${back}\n\n${since.text}` : back, files: [...backFiles, ...since.files] }` dans `getDraft(owner)` → `setDraft`, puis pose `app.draftReturned = { agentId: owner, n: <compteur> }`.
- `app.draftReturned = $state<{ agentId: string; n: number } | null>(null)`.
- `sendDraft(agentId: string, body: string, files: DraftAttachment[]): Promise<void>` dans `send.svelte.ts`. C'est le corps actuel de l'appel `api.sendMessage(...)` du Composer (pièces jointes `kind !== 'file'`, puis `$state.snapshot(f.file)`).
- `reportRefusal(owner: string, e: unknown): void` dans `send.svelte.ts`. C'est le `catch` actuel du Composer, hors `giveBack` : `missingAccountOf` → `conversationOf(owner).noAccount`, sinon `app.toast(errorText(e), 'error')`.

- [ ] **Test, dans `drafts.test.ts` :** `gives a message back before what was typed since, its files first, and tells the composer` :
  - partant d'un brouillon `{ text: 'b', files: [f2] }`, `giveBack(id, 'a', [f1])` donne `getDraft(id)` = `{ text: 'a\n\nb', files: [f1, f2] }` et `app.draftReturned?.agentId === id` ;
  - partant d'un brouillon vide, le texte vaut `'a'`.
- [ ] Lancer `npm test -- drafts` : il doit échouer.
- [ ] Implémenter `giveBack` et `send.svelte.ts`. Dans le Composer :
  - `deliver` utilise `sendDraft` et `reportRefusal` ;
  - le `giveBack` local est supprimé au profit de celui de `drafts` ;
  - un `$effect` calqué sur celui de `composerPut` (Composer.svelte:221) : quand `app.draftReturned?.agentId === draftFor`, recharger `text`/`files` depuis `getDraft(draftFor)`, puis `autosize`, et placer le curseur en fin de texte.
- [ ] Lancer `npm test -- Composer drafts` : tout doit passer, les tests existants de remise d'un message refusé inclus.
- [ ] Commit : `refactor(composer): rendre un message au brouillon et l'envoyer, hors du composer`

### Task 4 : la file d'envoi (`outbox.svelte.ts`)

**Files :** `src/lib/outbox.svelte.ts`, `src/lib/outbox.test.ts`, `src/lib/state.svelte.ts` (hooks : ~426-505 ; événement `agent` : ~598)

**Interfaces :**
- **Consumes :** `giveBack`, `sendDraft`, `reportRefusal` (tâche 3) ; `capsOf(engine).midTurn` (tâche 1) ; `app.settings.midTurnDelaySecs` (tâche 2) ; `api.attachDiscard(owner, path)` ; `flushDrafts` ; `app.onAgentRemoved` ; `app.onRestartWarned`.
- **Produces :**
  ```ts
  export type HoldMode = 'delay' | 'turnEnd';
  export interface Held { id: string; agentId: string; body: string; files: DraftAttachment[]; mode: HoldMode; at: number | null }
  export function holdModeOf(agent: Agent): HoldMode | null;
  export function hold(agentId: string, body: string, files: DraftAttachment[], mode: HoldMode): Held;
  export function heldOf(agentId: string): Held[]; // reactive
  export function sendNow(id: string): void;
  export function editHeld(id: string): void;
  export function cancelHeld(id: string): void;
  ```
  Dans `AppState` : `onTurnEnded(f: (agent: Agent) => void): () => void`. Il est appelé sur l'événement `agent` quand `prev` était au travail (`running`/`waiting`) et que `next` ne l'est plus.

**Règles à implémenter :**
- **`holdModeOf(agent)`**, dans cet ordre :
  1. l'agent a déjà des messages gardés → le mode du dernier ;
  2. l'agent n'est pas au travail, ou `agent.setup` / `agent.worktreePending` est posé → `null` ;
  3. `midTurn` vaut `'turnEnd'` → `'turnEnd'` ;
  4. `midTurn` vaut `'queued'` ou `'immediate'` → `'delay'` si `midTurnDelaySecs > 0`, sinon `null` ;
  5. `'refused'` → `null`.
- **`hold`** : id `held-<n>`. En mode Délai, `at = Date.now() + s*1000` et un `setTimeout` lâche le message à l'échéance. En mode Fin de tour, `at = null`.
- **Lâcher un message** (`release`) le retire de la liste, puis l'envoie par une chaîne de promesses propre à l'agent (ordre garanti). Si l'agent est archivé au moment du départ, le message va dans le brouillon (`giveBack`) au lieu de partir. En cas d'échec : `reportRefusal` + `giveBack`, et les suivants partent quand même.
- **`sendNow(id)`** lâche dans l'ordre ce message et tous ceux, plus anciens, du même agent.
- **`editHeld`** retire le message, coupe son minuteur, puis `giveBack(agentId, body, files)` et `app.focusComposer++`. **`cancelHeld`** retire le message, coupe son minuteur, et appelle `api.attachDiscard(agentId, f.file.path)` pour chaque copie (`f.file && !f.file.origin`, comme `remove` dans Composer.svelte:451).
- **`onTurnEnded(agent)`** : agent archivé → tous ses messages gardés vont au brouillon. Sinon, seul le **premier** message en mode Fin de tour part.
- **`onAgentRemoved`** : les messages de l'agent et leurs minuteurs sont abandonnés. **`pagehide` et `onRestartWarned`** : tous les messages gardés vont au brouillon (`giveBack`), puis `flushDrafts()`. Ce dernier appel est explicite, car l'écouteur de `drafts.ts` a pu passer avant.

- [ ] **Tests, dans `outbox.test.ts`** (`vi.useFakeTimers()`, `fakeBackend`, `resetApp`, `agent({ status: 'running' })`, `app.settings.midTurnDelaySecs = 10`) :
  - `holds a message sent to Claude during a turn and sends it ten seconds later` : `holdModeOf` vaut `'delay'` ; aucun `send_message` à 9 999 ms ; un `send_message` `{ id, text: 'x' }` à 10 000 ms ; `heldOf(id)` est vide ;
  - `keeps counting down when the turn ends or is stopped` : passage de `running` à `done` à 3 s ; aucun `send_message` avant 10 s, un seul à 10 s ;
  - `drops the messages held for an agent that is removed` : après l'événement `agentRemoved`, `heldOf` est vide et rien ne part à 10 s ;
  - `sends nothing held when the delay is off, nor to an agent at rest, nor for a harness that refuses` : `holdModeOf` vaut `null` dans ces trois cas ;
  - `keeps a message to Cursor until the end of the turn, one per end` : avec deux messages gardés en mode Fin de tour, un passage de `running` à `done` envoie seulement le premier ; un deuxième passage envoie le second ;
  - `sends the older held messages first when one is sent now` : avec A puis B gardés, `sendNow(B)` envoie A puis B, dans cet ordre ;
  - `holds a message sent at rest behind one still held` *(Review Focus 1)* : l'agent passe en `done` avec A encore gardé ; `holdModeOf` vaut `'delay'` ;
  - `gives an edited message back to the draft and sends nothing` : `editHeld` laisse `getDraft(id).text` à `'x'` et n'appelle jamais `send_message` ;
  - `drops a cancelled message and erases its copied files` : un seul `attach_discard`, pour le fichier copié, aucun pour un fichier d'origine ;
  - `never sends to an archived agent: its messages go back to its draft` *(Review Focus 2)* : vérifié par la fin de tour **et** par l'échéance d'un minuteur ;
  - `writes the held messages to the draft when the window goes` *(Review Focus 3)* : après `dispatchEvent(new Event('pagehide'))`, `readPref('draft.<id>')` contient le texte ;
  - `gives a refused message back and still sends the next one` *(Review Focus 4)* : `send_message` rejette A ; A revient dans le brouillon avec un toast d'erreur, et B part.
- [ ] Lancer `npm test -- outbox` : il doit échouer (module absent).
- [ ] Implémenter `onTurnEnded` dans `AppState` (calqué sur `onAgentRemoved` / `runHooks`), puis `outbox.svelte.ts` : un `$state<Record<string, Held[]>>`, des minuteurs dans une `Map<string, ReturnType<typeof setTimeout>>`, et l'enregistrement des crochets au chargement du module, comme `drafts.ts:73-76`.
- [ ] Lancer `npm test -- outbox state` : tout doit passer.
- [ ] Commit : `feat(composer): une file d'envoi garde un message posté pendant un tour, le temps du délai ou jusqu'à la fin du tour`

### Task 5 : le Composer garde le message, la conversation montre la bulle

**Files :** `src/components/Composer.svelte` (`send()` ~483, infobulle ~793), `src/components/conv/HeldMessage.svelte` (nouveau), `src/components/conv/UserMessage.svelte`, `src/components/Conversation.svelte:802,912`, `src/lib/i18n/{fr,en}/{conv,composer}.ts`, tests `Composer.test.ts`, `conv/HeldMessage.test.ts`, `Conversation.test.ts`

**Interfaces — Consumes :** tout l'API de la tâche 4.

- [ ] **`send()` :** une fois `answering` calculé, si `!answering` et que `holdModeOf(agent)` donne un mode, appeler `hold(agent.id, body, files.slice(), mode)`, puis `text = ''` et `files = []`, et `return`. Pas de `deliver`.
- [ ] **`UserMessage` :** nouvelle prop `extra?: Snippet`, rendue en dernier dans `.meta`. La condition qui affiche `.meta` l'inclut.
- [ ] **`HeldMessage.svelte` :** props `{ held: Held; engine: EngineId | null }`.
  - Il rend `UserMessage` avec un `UserItem` construit à partir du message gardé (`images` = nombre de `kind === 'image'`, `files` = noms des autres, `queued: false`).
  - Dans `extra` :
    - la note : `conv.held.delay` (`{s}` = `Math.max(0, Math.ceil((at - now) / 1000))`, `now` relu chaque seconde par un `setInterval` dans un `$effect`) ou `conv.held.turnEnd`, avec l'infobulle `turnEndHint` ;
    - puis les boutons « Envoyer maintenant » (mode Délai seulement), « Modifier », « Annuler » : liens discrets, style `.note` de `UserMessage`, sans icône.
- [ ] **`Conversation.svelte` :** `{#each heldOf(agent.id) as held (held.id)}<HeldMessage {held} engine={agent.engine} />{/each}` juste après `conv.waiting`. La condition d'état vide (ligne 802) tient compte de `heldOf(agent.id).length`.
- [ ] **Infobulle d'« Envoyer »** quand l'agent travaille et qu'aucune demande n'attend :
  - mode Délai → `composer.send.titleHeld` ;
  - mode Fin de tour → `titleTurnEnd` ;
  - `null` → `titleQueued` (actuelle).
- [ ] **i18n** (texte figé) :

  | clé | fr | en |
  |---|---|---|
  | `conv.held.delay` | Envoi dans {s} s | Sending in {s} s |
  | `conv.held.turnEnd` | Attend la fin du tour | Waiting for the end of the turn |
  | `conv.held.turnEndHint` | {engine} le recevra au début de son prochain tour | {engine} gets it at the start of its next turn |
  | `conv.held.sendNow` | Envoyer maintenant | Send now |
  | `conv.held.edit` | Modifier | Edit |
  | `conv.held.cancel` | Annuler | Cancel |
  | `composer.send.titleHeld` | Envoyé à {engine} dans {s} s, annulable d’ici là · {enter} pour envoyer | Sent to {engine} in {s} s, can be cancelled until then · {enter} to send |
  | `composer.send.titleTurnEnd` | {engine} le recevra à la fin de son tour, annulable d’ici là · {enter} pour envoyer | {engine} gets it at the end of its turn, can be cancelled until then · {enter} to send |
- [ ] **Tests, dans `Composer.test.ts`** (`app.settings.midTurnDelaySecs = 10` dans chaque test concerné) :
  - `holds a message typed while Claude works, and empties the field` : `setup({ status: 'running' })`, taper puis Entrée ; aucun `send_message`, `heldOf(a.id)` contient un message, le champ est vide ;
  - `sends at once an answer to a waiting question, even with a delay` : réutiliser le cas existant de la ligne ~292 ; l'appel `answer_question` a lieu et `heldOf` reste vide ;
  - `sends at once while Claude works when the delay is off` : `midTurnDelaySecs = 0`, un `send_message` immédiat ;
  - `brings an edited message back before what was typed since` *(Review Focus 5)* : garder `'a'`, taper `'b'`, `editHeld` → `textarea.value === 'a\n\nb'` ;
  - mettre à jour l'attente de l'infobulle anglaise (ligne ~757) : avec un délai de 10 s, `'Sent to Claude in 10 s, can be cancelled until then · Enter to send'`.
- [ ] **Tests, dans `HeldMessage.test.ts`** (horloge simulée) :
  - `counts down the seconds before a held message goes` : « Envoi dans 10 s », puis « Envoi dans 9 s » une seconde plus tard ;
  - `offers to send now only a message held for a delay` : pas de « Envoyer maintenant » en mode Fin de tour, où « Attend la fin du tour » s'affiche ;
  - `edits, cancels and sends now from its buttons` : chaque bouton appelle respectivement `editHeld`, `cancelHeld` et `sendNow` avec l'id (`vi.mock('../../lib/outbox.svelte')`).
- [ ] **Test, dans `Conversation.test.ts` :** `shows the messages held for an agent after its items` : une bulle « Attend la fin du tour » apparaît pour un message gardé d'un agent Cursor (`CURSOR_ENGINE` ajouté à `app.engines`).
- [ ] Lancer `npm test -- Composer HeldMessage Conversation UserMessage`, puis `npm run check` : tout doit passer.
- [ ] Commit : `feat(composer): un message posté pendant un tour s'affiche « Envoi dans 10 s » ou « Attend la fin du tour », avec Modifier, Annuler et Envoyer maintenant`

### Task 6 : documentation et vérification de bout en bout

**Files :** `docs/SPEC.md:187,199`, `CHANGELOG.md`, `CHANGELOG.en.md`

- [ ] **SPEC.md:199.** Remplacer « messages envoyés pendant que Claude travaille : transmis tout de suite… » par la règle complète, dans le style dense du paragraphe :
  - les deux modes et le réglage ;
  - les trois actions ;
  - l'ordre ;
  - la fin de tour (un message par fin de tour) ;
  - Stop et le compte à rebours qui continue ;
  - archivé / quitté / rechargé → brouillon ; supprimé → abandonné ; refus → champ + toast ;
  - ce qui n'est pas gardé (réponses, MCP, claude.ai, voix, préparation du worktree).

  SPEC.md:187 : ajouter « Envoi dans N s » et « Attend la fin du tour » aux notes de bulle.
- [ ] **CHANGELOG.md**, sous `## [Non publié]` › Nouveautés : « Un message posté pendant qu’un agent travaille attend 10 s avant de partir (réglable, ou désactivé) : tu peux le modifier, l’annuler ou l’envoyer tout de suite. Chez Cursor et OpenCode, il attend la fin du tour, et reste annulable jusque-là. » Mettre la traduction dans `CHANGELOG.en.md`.
- [ ] **Vérification complète :**
  - `npm test` : tout passe, à part l'échec connu d'`attachments.test.ts` ;
  - `npm run check` et `npm run format:check` : 0 erreur ;
  - dans `src-tauri` : `PATH=<scratchpad>/venv/bin:$PATH cargo test` (relancer seuls les tests instables connus avant d'y voir une régression) ;
  - `cargo clippy --all-targets -- -D warnings` : 0 warning.
- [ ] **Essai réel dans l'app** (`ESCOUADE_DATA_DIR="$TMPDIR/escouade-sandbox" npm run tauri dev`, après `npm ci`) :
  1. un agent Claude lance une longue commande (`sleep 30`) ; un message posté pendant ce temps affiche « Envoi dans 10 s » et le décompte ;
  2. « Modifier » remet le texte dans le champ ;
  3. un renvoi suivi d'« Annuler » fait disparaître la bulle, et rien n'arrive à Claude ;
  4. un renvoi laissé à échéance part, avec « transmis pendant le tour », et Claude le lit à l'étape suivante ;
  5. avec le réglage sur « Désactivé », le message part tout de suite, comme avant ;
  6. si Cursor ou OpenCode est installé : « Attend la fin du tour », puis un nouveau tour démarre à la fin du tour ;
  7. un rechargement de la fenêtre (Cmd+R) pendant l'attente remet le texte dans le champ.
- [ ] Commit : `docs(composer): SPEC et CHANGELOG — un message posté pendant un tour, modifiable et annulable avant l'envoi`
- [ ] Fin de branche : appliquer superpowers:finishing-a-development-branch (push sur `fork`, PR vers upstream, corps de PR en français terminé par `🤖 Generated with [Claude Code](https://claude.com/claude-code)`), **après accord de l'utilisateur pour pousser**.
