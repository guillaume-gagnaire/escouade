---
title: "Pilotage de Claude Code"
subject: "Harness"
sources: ["docs/SPEC.md"]
tickets: ["ESC-106","ESC-133"]
decisions: []
compared: null
---

## Ce que c'est

Escouade pilote Claude Code dans un chat natif : chaque agent est un process `claude` en stream-json, et Escouade montre sa conversation, sans TUI embarqué. **1 agent = 1 conversation.** La vue Claude Code montre à la place l'interface de Claude Code elle-même, dans un terminal, sur la même session. Escouade tient aussi Claude Code à jour à sa place.

## Comportement

### Le process d'un agent

- Chaque agent est un process `claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages`, lancé dans le dossier du projet, ou dans le worktree de l'agent.
- Les demandes de permission et `AskUserQuestion` remontent par le protocole de contrôle (permission prompts « host »). Elles s'affichent en cartes « Claude attend ta réponse » (voir *Conversation*).
- Le modèle, l'effort et le mode de permission se changent en cours de conversation.
- Après un redémarrage, l'agent reprend sa session par `--resume <session_id>`.
- Le process reçoit le `claude` et le `CLAUDE_CONFIG_DIR` du compte de l'agent, et reprend sa session dans ce dossier. Le compte Principal ne reçoit jamais `CLAUDE_CONFIG_DIR` (voir *Comptes*).
- Le serveur MCP d'Escouade lui est donné par `--mcp-config` tant que « Les agents peuvent piloter Escouade » est activé ; sinon l'agent en est tenu à l'écart (voir *Serveur MCP*).
- Tant que « Les agents tiennent une liste de tâches » est actif (par défaut), il reçoit `CLAUDE_CODE_ENABLE_TODO_TOOLS=1` : les outils de liste de tâches de Claude Code, d'où se lit l'avancée de l'agent (voir *Avancée d'un plan*).
- Il lit sans demander le dossier de ses fichiers joints : `--add-dir=<données>/attachments/<agent>` à chaque lancement, en chat comme en vue Claude Code. L'agent d'un ticket qui a des fichiers reçoit en plus `--add-dir=<dossier du ticket>/files`. Sans fichier, pas d'argument, et le dossier n'est pas créé (voir *Conversation*, « Fichiers joints »).
- Ce qu'Escouade dit aux agents et fait écrire à Claude suit « Langue des textes rédigés par Claude » (voir *Langues*).

### Vue Claude Code

- Elle s'ouvre par le bouton « Vue Claude Code » de l'en-tête de l'agent, ou d'office pour les nouveaux agents avec « Ouvrir les nouveaux agents dans l'interface de Claude Code ». Jamais pour un agent de ticket.
- L'agent tourne dans l'interface de Claude Code, dans un terminal, sur la même session. Un seul process à la fois : changer de vue arrête l'un et reprend la session dans l'autre. Faits et formats : docs/PROTOCOL.md.
- La vue ne dépend que de l'identité de l'agent : la fin de l'interface, qui met l'agent à jour, ne la relance pas.

#### Session à reprendre

Claude Code quitte aussitôt sur une session qu'il ne trouve pas (« No conversation found with session ID… », code 1), et un agent qui n'a rien envoyé n'a pas encore de session : Claude Code l'écrit au premier message. Avant chaque lancement de l'interface, « Relancer » compris, Escouade cherche donc le transcript de la session (`<dossier du compte>/projects/*/<session>.jsonl`) dans le dossier du compte de l'agent ; pour une copie qui n'a pas encore le sien, celui de l'original. Trois issues :

- **Trouvé** : l'interface reprend la session (`--resume`).
- **Dans le dossier d'un autre compte du même côté** (Windows ou WSL) : il est copié dans celui du compte de l'agent, la session est reprise, et la conversation le dit : « La session était dans le dossier du compte « A » : elle a été copiée dans celui de « B » pour être reprise. » Si la copie échoue : « La session est dans le dossier du compte « A », mais elle n'a pas pu être copiée dans celui de « B » (<erreur>) : l'interface repart en nouvelle session. »
- **Absent partout** : la session est abandonnée et l'interface démarre une nouvelle session (`--session-id`). La conversation dit « Session Claude introuvable : l'interface repart en nouvelle session. » si l'agent avait quelque chose à perdre (des messages envoyés ou des entrées lues) ; un agent qui n'a jamais rien envoyé repart sans rien dire. Pour une copie dont l'original manque, toujours : « La session de l'original est introuvable : la copie repart en nouvelle session. »

#### Sortie de l'interface

- « Aussitôt » veut dire : moins de 20 s après son lancement, avec un code autre que 0. Un `/exit` (code 0) n'est jamais « aussitôt » : « Claude Code s'est arrêté. » et « Relancer ».
- Aussitôt, avec « No conversation found » dans ses premiers Ko de sortie : la session est abandonnée comme ci-dessus, avec le même avertissement, et la vue relance l'interface d'elle-même, une seule fois, sur la nouvelle session, sous la note « Session Claude introuvable : l'interface repart en nouvelle session. » (6 s une fois l'interface lancée).
- Aussitôt pour une autre raison, ou une deuxième fois de suite : aucune relance. « Claude Code s'est arrêté dès son lancement (code N). Regarde sa sortie ci-dessus, puis relance-le. » (sans « (code N) » si le process a été tué) et « Relancer » ; la sortie reste lisible dans le terminal.
- « Relancer » est un lancement manuel : il passe toujours et remet à zéro le compte des sorties aussitôt. Ce compte refuse aussi un lancement automatique à la deuxième sortie de suite : « Claude Code s'arrête dès son lancement (2 fois de suite) : regarde sa sortie, puis clique sur « Relancer ». » Changer de vue le remet à zéro.

### Mise à jour de Claude Code

- Chaque `claude` qu'Escouade lance reçoit `DISABLE_AUTOUPDATER=1` : agents, vue Claude Code, lectures courtes (modèles, version, `claude -p`, `claude mcp`), connexion d'un compte ; tous les comptes, toutes les plateformes. Sans elle, Claude Code se met à jour en arrière-plan à son lancement (`npm install -g` pour une installation npm), et Escouade, qui arrête chaque process avec tout son arbre, couperait l'installation à mi-chemin : la commande `claude` disparaîtrait. La variable ne coupe que la vérification en arrière-plan, et la mise à jour des plugins (que `FORCE_AUTOUPDATE_PLUGINS` rétablirait) ; `claude update` marche toujours.
- À la place, Escouade lance `claude update` (qui met à jour une installation native comme npm) 5 s après son démarrage, puis une fois par jour tant qu'il est ouvert. Il prend le `claude` de Principal et les réglages réseau. Il tourne dans un process à part, détaché de tout job, dans son propre groupe de process sous Unix, sa sortie dans `<données>/claude-update.log` : ni l'arrêt d'un agent ni la fermeture d'Escouade ne le coupent.
- Le résultat va dans `app.log` et sous l'interrupteur de Réglages › Harness : « À jour (X.Y.Z). », « Mis à jour en X.Y.Z (depuis la A.B.C). », « Échec de la mise à jour : <dernière ligne de sa sortie> », « Mise à jour en cours… ». Éteint : « Coupée : Claude Code n'est plus mis à jour, ni par Escouade ni par lui-même. Lance claude update de temps en temps. »
- Si l'utilisateur a coupé lui-même la mise à jour de Claude Code, Escouade ne la fait pas non plus. Elle le lit comme Claude Code : `DISABLE_UPDATES` ou `DISABLE_AUTOUPDATER` vrais (`1`, `true`, `yes`, `on`) dans l'environnement d'Escouade ou dans le `env` du `settings.json` de Principal, `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` défini, ou `autoUpdates: false` dans son `.claude.json` (sauf une installation native qui l'a écrit elle-même, `autoUpdatesProtectedForNative`). L'onglet dit alors « Coupée dans Claude Code lui-même (<comment>) : Escouade ne le met pas à jour non plus. »

### Claude Code introuvable

- La connexion d'un compte, le lancement d'un agent (et la vue Claude Code) et Réglages › Harness disent : « Claude Code est introuvable : réinstalle-le (`<installeur>`, ou `npm install -g @anthropic-ai/claude-code`), ou indique son chemin dans Réglages › Harness. »
- Si une installation npm coupée a laissé une trace (un lien `.claude-*` dans un dossier du PATH, `/opt/homebrew/bin`, `/usr/local/bin` ou `%APPDATA%\npm`, ou un paquet mis de côté `@anthropic-ai/.claude-code-*` à côté) : « L'installation de Claude Code a été interrompue (il en reste <chemin>) : relance `npm install -g @anthropic-ai/claude-code` dans un terminal. » Ce message remplace aussi « Claude Code s'est arrêté avant la connexion. » quand la connexion d'un compte ou un agent s'arrête avant de démarrer.

## Réglages

Dans Réglages › Harness :

- le chemin de Claude Code (voir *Claude Code dans WSL* pour un Claude Code installé dans WSL) ;
- « Ouvrir les nouveaux agents dans l'interface de Claude Code » ;
- « Mettre à jour Claude Code automatiquement » (allumé par défaut).

« Les agents tiennent une liste de tâches » est actif par défaut.

## Limites

- La vue Claude Code n'est jamais proposée pour un agent de ticket.
- Une session ne se reprend que du même côté, Windows ou WSL.
