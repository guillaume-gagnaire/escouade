# Un message posté pendant un tour, modifiable et annulable avant l'envoi — conception

Date : 2026-10-27. Sans ticket (contribution du fork Metalzoid). Fait foi pour les noms et les formes.

## But

Aujourd'hui un message posté pendant un tour part **tout de suite** au process (`Core::deliver` → `send_message`) :
Claude Code le met dans sa file et le lit à l'étape suivante du tour, Codex et Copilot CLI le lisent à l'étape suivante
(`turn/steer`, `session.send` en mode `immediate`), Cursor et OpenCode l'attendent dans le process d'Escouade jusqu'à
la fin du tour (`queue`, `held`). Une fois chez le harness, rien ne le retire : chez Claude Code, le seul recours est
`interrupt` avec `cancel_queued`, qui coupe le tour et vide toute la file.

Désormais **la fenêtre garde le message** le temps qu'il peut l'être, et le montre avec « Modifier », « Annuler » et,
quand il attend un délai, « Envoyer maintenant ».

## Décisions de l'utilisateur (2026-10-27, ne pas les rouvrir)

1. **Rétention hybride, selon le harness.** Cursor et OpenCode : gardé jusqu'à la fin du tour (il l'était déjà, rien ne
   change pour l'agent). Claude Code, Codex, Copilot CLI : un **délai de grâce**, puis envoyé comme aujourd'hui (l'agent
   le lit toujours à l'étape suivante).
2. **« Modifier » rend le message au composer** : texte et pièces jointes reviennent dans le champ, devant ce qui y a été
   tapé depuis ; on le renvoie normalement (un nouveau délai démarre). Pas d'édition dans la bulle.
3. **Le délai est un réglage** : « Délai avant l'envoi pendant un tour », Désactivé / 5 / **10 (défaut)** / 20 / 30 s.
   « Désactivé » : le comportement d'avant (envoi immédiat, pas d'annulation).
4. **La file vit dans la fenêtre** (approche A), pas dans le cœur.

## Conception

### Quand un message est gardé

Un message tapé dans le composer d'un agent au travail (`running` ou `waiting`). Jamais :

- une réponse à une question ou à une demande d'autorisation (le texte tapé pendant qu'elle attend) ;
- pour un harness `refused` ;
- pendant la préparation d'un worktree (`holdMessage` et son toast « Un message attend déjà la fin de la préparation. »,
  inchangés) ;
- un message venu du serveur MCP, de claude.ai, d'un autre agent, ou dit dans une conversation vocale : ils ne passent pas
  par le composer.

Un message envoyé alors qu'un message plus ancien du même agent est encore gardé est gardé aussi, dans le mode de ce
dernier, même au repos : il ne le double jamais.

### Deux modes

- **Délai** (`midTurn` `queued` : Claude Code ; `immediate` : Codex, Copilot CLI) : la bulle dit « Envoi dans N s », le
  compte à rebours chaque seconde. À l'échéance, il part par `send_message`, comme avant (puis « transmis pendant le
  tour »). La fin du tour, ou Stop, ne l'arrête pas : il reste annulable jusqu'au bout, et part alors comme premier
  message du tour suivant.
- **Fin de tour** (`midTurn` `turnEnd`, nouveau : Cursor, OpenCode) : la bulle dit « Attend la fin du tour ». Le premier
  message gardé part à la fin du tour et en lance un ; les suivants attendent la fin de celui-là (un message par tour,
  comme Cursor le faisait déjà ; OpenCode les fusionnait). Ce mode ne dépend pas du réglage.

`MidTurn::Queued` décrivait Claude Code comme « attendant la fin du tour » : c'est faux (il lit le message à l'étape
suivante, voir `docs/SPEC.md`), et Cursor et OpenCode, qui l'attendent vraiment, partageaient la même valeur. Le variant
`TurnEnd` les sépare ; côté backend, il reste « mis en file » comme `Queued` (seul `Immediate` est comparé dans
`push_user_from`), donc un message du serveur MCP garde son chemin.

### Les actions

- **Modifier** : la bulle disparaît, son minuteur avec ; texte et pièces jointes au brouillon de l'agent, devant le texte
  déjà là (séparé par une ligne vide), ses pièces jointes d'abord ; le champ affiché se met à jour et prend le focus.
- **Annuler** : la bulle disparaît, sans confirmation (« Modifier » est le geste sûr) ; les copies de ses fichiers joints
  sont effacées, jamais un fichier joint là où il est.
- **Envoyer maintenant** (mode Délai seulement) : il part tout de suite, et avec lui, avant lui, les messages plus anciens
  du même agent encore gardés. En mode Fin de tour, il voudrait dire « interrompre » : c'est Stop.

### Ce qui l'emporte

- **Agent archivé** : ses messages gardés reviennent à son brouillon, rien ne part, même un minuteur qui arrive à échéance
  après l'archivage.
- **Agent supprimé** : ses messages gardés sont abandonnés.
- **Fenêtre fermée ou rechargée, redémarrage pour une mise à jour** : les messages gardés reviennent au brouillon et son
  texte est écrit dans les préférences tout de suite. Les pièces jointes sont perdues, comme celles de tout brouillon.
- **Envoi refusé à l'échéance** (process mort, fichier refusé, compte manquant) : comme un envoi refusé d'aujourd'hui, le
  message revient au champ et l'erreur s'affiche ; les messages gardés suivants partent quand même, dans l'ordre.

### Où

- `src/lib/outbox.svelte.ts` : la file, par agent (`hold`, `heldOf`, `holdModeOf`, `sendNow`, `editHeld`,
  `cancelHeld`), ses minuteurs, ses crochets (fin de tour, agent retiré, fenêtre qui part).
- `src/lib/send.svelte.ts` : l'envoi d'un brouillon et le compte rendu d'un refus, partagés par le composer et la file.
- `src/lib/drafts.ts` : `giveBack`, sorti du composer, et le signal `app.draftReturned` qui fait relire son brouillon au
  composer affiché.
- `AppState.onTurnEnded` : l'agent passe de « au travail » à autre chose.
- `HeldMessage.svelte` : la bulle (la bulle de `UserMessage`, sa note et ses trois liens).
- Rust : `MidTurn::TurnEnd` (Cursor, OpenCode) et `Settings::mid_turn_delay_secs` (défaut 10 ; une autre valeur que 0,
  5, 10, 20, 30 devient 10, au chargement comme à l'enregistrement).

## Approches écartées

- **B. La file dans le cœur** : messages dans l'état de l'agent, persistés, minuteurs tokio, crochet de fin de tour dans
  chacun des cinq harness, commandes `cancel/release/edit`. Le journal de conversation (ajouts et patches seulement)
  devrait apprendre un état « annulé », et « Modifier » renvoyer les pièces jointes en base64 à la fenêtre. Trois à
  quatre fois plus gros, au cœur de l'envoi, pour gagner la survie à un rechargement de la fenêtre.
- **C. Le retrait natif** (`unqueue` dans le process de Cursor et d'OpenCode, un délai dans le cœur pour les autres) :
  l'élément est déjà au journal (`queued`), il faudrait aussi un état « annulé » ; les défauts de A et de B à la fois.

## Hors de ce lot

- Une file visible et annulable depuis le serveur MCP.
- L'infobulle « {engine} en tient compte dès sa prochaine étape » d'un message du serveur MCP mis en file chez Cursor ou
  OpenCode (il attend en fait la fin du tour) : inchangée.
