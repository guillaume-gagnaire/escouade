---
title: "Pilotage de Claude Code"
subject: "Harness"
sources: ["docs/SPEC.md","tests/fixtures/claude-artifacts","https://code.claude.com/docs/en/code-mode (lue le 2026-11-20)","https://code.claude.com/docs/en/agent-teams (lue le 2026-11-20)","Essais réels d'ESC-553 sur Claude Code 2.1.329 (bilan du ticket)"]
tickets: ["ESC-106","ESC-133","ESC-400","ESC-553"]
decisions: [136,"les-fonctionnalites-experimentales-de-cl"]
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
- Il reçoit toujours `CLAUDE_CODE_ARTIFACT=1`, discussion comprise : ses artefacts, comme dans un terminal. En stream-json, Claude Code se tient pour une session de l'Agent SDK (`CLAUDE_CODE_ENTRYPOINT=sdk-cli`) et coupe ses artefacts sans cette variable : ni outil `Artifact`, ni skill `artifact-design` (« Unknown skill: artifact-design »). Ce qui les coupe pour l'utilisateur les coupe toujours : `CLAUDE_CODE_DISABLE_ARTIFACT`, ses réglages de Claude Code, son organisation. Supprimer un artefact demande son accord, sur une carte de permission. La vue Claude Code, interactive, les a d'elle-même.
- Il lit sans demander le dossier de ses fichiers joints : `--add-dir=<données>/attachments/<agent>` à chaque lancement, en chat comme en vue Claude Code. L'agent d'un ticket qui a des fichiers reçoit en plus `--add-dir=<dossier du ticket>/files`. Sans fichier, pas d'argument, et le dossier n'est pas créé (voir *Conversation*, « Fichiers joints »).
- Ce qu'Escouade dit aux agents et fait écrire à Claude suit « Langue des textes rédigés par Claude » (voir *Langues*).
- Il ne reçoit jamais les fonctionnalités expérimentales du réglage du plugin (voir *Fonctionnalités expérimentales*).

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

### Fonctionnalités expérimentales

Le plugin Claude Code (`dev.escouade.claude-code`, 2.2.8) a un groupe de réglages « Fonctionnalités expérimentales » : les drapeaux `CLAUDE_CODE_EXPERIMENTAL_*` de Claude Code, que la vue Claude Code reçoit **seule**.

**Les drapeaux.**

| Réglage | Variable | Version | Où il marche |
| --- | --- | --- | --- |
| Code Mode | `CLAUDE_CODE_EXPERIMENTAL_CODE_MODE` | 2.1.323 (2.1.326 avec les Agent Teams) | une session `claude` interactive dans un terminal ; un abonnement Claude ou un compte Console ; Opus 5.5, Sonnet 5.5, Haiku 5.5, Fable 5.2 |
| Agent Teams | `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` | 2.1.32 | une session interactive (`-p` ne forme pas d'équipe) |
| Autres drapeaux | des noms de `CLAUDE_CODE_EXPERIMENTAL_*`, séparés par des espaces ou des virgules, chacun à `1` | — | ce que Claude Code en fait |

- **Les autres** ne prennent que des noms `CLAUDE_CODE_EXPERIMENTAL_` suivis de majuscules, chiffres et `_` : jamais une autre variable, jamais une valeur, ni le Code Mode ou les Agent Teams, qui ont leur interrupteur (et ses refus). Ce qui n'est pas pris est dit à l'ouverture de l'interface : « Pas passés à l'interface : PATH (seuls des noms de CLAUDE_CODE_EXPERIMENTAL_* le sont, et le Code Mode et les Agent Teams ont leur interrupteur). »
- **Pour tous les projets, puis par projet.** Les trois réglages de la page du plugin valent pour tous les projets. La page Plugins d'un projet les règle autrement : Code Mode et Agent Teams « Comme pour tous les projets » (par défaut), « Activé » ou « Désactivé » ; « Autres drapeaux » remplace la liste de tous les projets (vide : celle de tous les projets ; « aucun » : aucun). Les six réglages sont gardés sur la machine : le `.escouade.yml` d'un dépôt ne les donne jamais (`repo: false`), comme Claude Code ignore le Code Mode dans les réglages d'un projet.
- **Où ils vont.** Dans le bloc `env` du fichier de réglages privé de l'interface (`--settings <fichier>`), que Claude Code lit comme les réglages de l'utilisateur ; un `0` posé par l'utilisateur lui-même l'emporte toujours. Jamais une variable du process, jamais un argument ; jamais le lancement de la conversation, d'une discussion ni d'un agent de ticket.

**Le Code Mode n'est pas passé**, et la conversation le dit en ouvrant l'interface (« Code Mode non passé à l'interface : <raison>. »), quand :

- le remote control est demandé pour cet agent (« le remote control est demandé pour cet agent, et Claude Code ne prend pas les deux ») ;
- le compte passe par une passerelle ou un fournisseur cloud : `ANTHROPIC_BASE_URL`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_USE_BEDROCK`, `_VERTEX`, `_FOUNDRY`, `_MANTLE`, `_ANTHROPIC_AWS`, `_ANTHROPIC_GOOGLE_CLOUD`, dans les variables du compte, celles d'Escouade ou le `env` du `settings.json` du compte, ou un `apiKeyHelper` (« le compte passe par <nom> (une passerelle ou un fournisseur cloud), et le Code Mode ne marche qu'avec l'API d'Anthropic ») : son nom seul est dit, jamais sa valeur ;
- la version lue de Claude Code est trop ancienne (« Claude Code 2.1.301 est trop ancien (il faut la 2.1.323 au moins) : mets-le à jour ») ; une version inconnue laisse Claude Code juger ;
- le modèle ne le prend pas (« le modèle claude-opus-4-8 ne le prend pas (Opus 5.5, Sonnet 5.5, Haiku 5.5 ou Fable 5.2) ») : un alias (`opus`, `sonnet`…) suit le plus récent de sa famille, un nom inconnu est laissé à Claude Code ;
- l'agent est une discussion.

Les Agent Teams ne sont pas passées avec une version plus ancienne que la 2.1.32, et c'est dit de même.

**Un agent en Code Mode reste dans son terminal.**

- Claude Code écrit `{"type":"repl-mode","replMode":true}` dans le transcript d'une session commencée en Code Mode (avant le premier message, puis après chaque tour). Escouade le lit en suivant la vue, ou dans le premier Mio du transcript d'une session qu'il va reprendre ou copier : l'agent est marqué pour de bon (`codeMode`), et sa conversation le dit une fois : « En Code Mode : cet agent se reprend dans son terminal. Claude Code ne reprend une session commencée en Code Mode que dans son interface, jamais dans la conversation d'Escouade. »
- Au-dessus de son terminal, une bande le redit (« En Code Mode : cet agent se reprend dans son terminal ») ; au-dessus de sa conversation, la même bande offre « Ouvrir le terminal ». « Vue Escouade » est grisé dans l'en-tête et dans le menu de sa carte, avec la même phrase.
- Escouade ne démarre jamais son process stream-json : ni la reprise automatique, ni un ticket, ni un message, ni le retour à la vue d'Escouade (refusé, avec la même phrase). Claude Code le refuserait : « Failed to resume session: This session was started in Code Mode (experimental), which isn't available because for now it only works in the interactive terminal, so it can't be resumed here. »
- Son interface reprend la session avec le Code Mode, même si le réglage est éteint depuis : sans lui, Claude Code quitte (« This session was started in Code Mode (experimental), which is off, so it can't be resumed. »). Jamais avec le remote control, que Claude Code ne prend pas en Code Mode ; la conversation dit : « Le remote control n'est pas lancé : Claude Code ne le prend pas en Code Mode, et la session de cet agent en a besoin. »
- Une session commencée sans le Code Mode se reprend sans lui, la variable posée ou pas : l'agent n'est pas marqué.

**Les permissions et la clôture.** En Code Mode, Claude écrit du JavaScript que l'outil `REPL` lance, et chaque appel d'outil fait par le code passe par les règles `deny` et `ask` et par les hooks `PreToolUse` et `PermissionRequest` sous son nom habituel (`Bash`, `Read`, un outil MCP…) :

- Les hooks de la vue Claude Code reçoivent l'appel `REPL` avec son code (`tool_input.code`, et `description`), puis chaque appel qu'il fait. La notification d'une permission demandée dit « Autoriser Bash : mkdir e-made ? » pour un appel du code, et « Lancer du code : <description> ? » pour le code lui-même, quand une règle `ask` le demande. Le serveur des hooks répond toujours `{}` aussitôt : il ne décide rien et ne fait jamais échouer un appel du code.
- La clôture d'un projet privé, faite de règles `deny` (`Read(…)`, `Edit(…)`), tient pour le code : `file(…).text()` et un `cat` d'un chemin clôturé sont refusés, mode bypass compris.
- Ce qu'Escouade refuse à une discussion et l'outil `mcp__escouade` sans « Les agents peuvent piloter Escouade » restent refusés : le Code Mode n'est jamais passé à une discussion, et une règle `deny` vaut pour les appels du code.

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

Dans Plugins › Claude Code › Réglages, le groupe « Fonctionnalités expérimentales » : « Code Mode » et « Agent Teams » (éteints par défaut), « Autres drapeaux » (vide). Dans la page Plugins d'un projet : les mêmes, pour ce projet (voir *Fonctionnalités expérimentales*).

## Limites

- La vue Claude Code n'est jamais proposée pour un agent de ticket.
- Une session ne se reprend que du même côté, Windows ou WSL.
- Un artefact ne se supprime pas sans l'accord de l'utilisateur ; un agent en stream-json ne suit pas les nouvelles versions d'un artefact publiées ailleurs (Claude Code le réserve à une session interactive).
- Le Code Mode et les Agent Teams ne marchent que dans la vue Claude Code : la conversation d'Escouade (stream-json) les ignore, et un agent en Code Mode ne revient jamais dans la conversation d'Escouade. Ce que fait le code n'est lu que par les appels qu'il fait (hooks, transcript) : le JavaScript lui-même n'est jamais montré comme une commande.
- La première session en Code Mode d'un compte demande une confirmation dans le terminal (« Yes, turn on Code Mode ») : l'utilisateur y répond lui-même.
