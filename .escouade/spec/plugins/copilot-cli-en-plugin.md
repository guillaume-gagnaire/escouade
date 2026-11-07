---
title: "Copilot CLI en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.5, §4.6 (INT-10.3)","docs/TODO-2.2.md, INT-10","Ticket ESC-224","plugins/copilot/README.en.md","tests/fixtures/engines/copilot-1.0.98.session-events.schema.json"]
tickets: ["ESC-224","ESC-346","ESC-352","ESC-362","ESC-369"]
decisions: []
compared: null
---

# Copilot CLI en plugin

Copilot CLI est le premier harness sorti du cœur qui n'est pas ACP (INT-10.3) : sa conversation passe par le plugin de première main `dev.escouade.copilot-cli` (« Copilot CLI »), un process à lui (`escouade-plugins copilot`), sa crate `escouade-plugin-copilot` dans `plugins/copilot/`, livré et signé avec l'app, au registre des plugins livrés et sur le store (sa page pointe vers ce dossier). Pour l'utilisateur, ses agents, ses comptes, ses crédits, ses textes et sa vue terminal ne changent pas.

## Ce que fait le plugin

- Il tient le client de `copilot --server --stdio` (le protocole de son SDK, JSON-RPC 2.0, protocole 3) et la traduction de ses événements. Le cœur trouve `copilot`, demande au plugin la description du lancement, la vérifie et lance Copilot ; ses messages passent en `Content-Length`.
- **Le plugin ouvre la session** : protocole vérifié, connexion, session nouvelle, reprise ou copiée, ses modes, le serveur d'Escouade, son modèle, les modèles du compte. L'agent est prêt quand c'est fait. Une session reprise qui n'existe plus en donne une nouvelle, et la conversation le dit.
- **Un refus arrête l'agent et dit pourquoi** : un autre protocole, aucune connexion, une session que Copilot n'ouvre pas.
- Ce que Copilot écrit revient en éléments de la conversation, comme le faisait le cœur : textes, raisonnements, outils sous les noms de Claude Code, permissions, questions, formulaires, fin d'un plan, plafond de crédits d'une session, tâches, sous-agents, crédits du mois, remplissage du contexte, jetons et coût de chaque tour.
- **Les permissions sûres** sont répondues par les règles d'Escouade, sans carte : une lecture dans le dossier de l'agent, une écriture là hors du mode Plan et hors de `.git`, `git add` et `git commit` de son worktree en « Accepter les modifications » et « Auto ». Copilot garde cette dernière règle : son plugin demande à `git` où sont le dépôt et ses hooks.
- **Ce qui a toujours sa carte** (ESC-362), même dans le dossier de l'agent :
  - une écriture dans `.git`, en entier et quelle que soit la casse, le fichier `.git` d'un worktree et les noms que Windows prend pour lui compris ; sinon un hook écrit là passerait au `git commit` suivant ;
  - un chemin dont la forme résolue par Copilot (`resolvedPath`, un lien suivi) sort du dossier, ou un déplacement dont un bout en sort ;
  - une commande où le shell verrait autre chose que `git add` et `git commit` : `;`, `|`, `&` seul, une redirection, un saut de ligne, ou lancée hors du worktree (`resolvedWorkingDirectory`). Son texte est lu comme son shell le lit, selon `shellType` ; un shell que ce nom ne dit pas doit se lire pareil en PowerShell et en POSIX ;
  - ce que la politique de Copilot garde pour un humain (`managedApprovalRequired`) et une sortie de son bac à sable (`requestSandboxBypass`).
- **Un projet privé** : une permission qui nomme un chemin clôturé est refusée d'office, sans carte, y compris par les chemins résolus d'une commande (`resolvedPaths`). C'est une garde, comme avant (voir *Projets privés*). Pour qu'elle voie chaque demande, un agent clôturé n'a pas « Pour cette session » ni l'acceptation des éditions d'un plan approuvé : ses écritures restent demandées une à une. Un `git commit` qui lit un fichier (`-F`, `--file` et leurs formes) lui passe par une carte, le chemin pouvant être un lien. La clôture et la lecture seule tiennent à travers un redémarrage du plugin, qui les reprend du cœur.
- `/compact` passe par sa requête, un titre de session par `session.name.set`.
- Le plugin ne lit aucun secret : le jeton d'Escouade et la connexion du compte arrivent à Copilot par le cœur, et son client n'appelle jamais ce qui rend un jeton en clair.

## Ce qui reste au cœur

Le moteur `Copilot` du registre garde ce qui n'est pas la conversation : ses capacités et le sens de ses modes, les comptes (`copilot login` dans le dossier de chacun), le quota et les modèles d'un compte lus par un serveur à lui, les textes rédigés (`copilot -p`), le fichier de l'entrée d'Escouade, la vue terminal (son PTY, ses hooks, le journal de sa session).

**La vue passe par le module** (`view/translate`, ESC-352) : le cœur lit le journal et passe ses lignes au plugin, une seule traduction pour les deux. Plugin coupé, le terminal marche et la conversation dit qu'elle ne le suit plus. Voir *Le contrat des harness en process*.

## Un arrêt se dit

Coupé pour l'app ou pas chargé, le plugin n'est plus branché derrière Copilot : un agent de Copilot dit au lancement « Le plugin Copilot CLI (dev.escouade.copilot-cli) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa session ; rallumé, il reprend. Un plugin qui plante en plein tour redémarre et la conversation le dit ; après trois plantages, l'agent s'arrête et sa session est gardée. Un plugin redémarré reprend la session de l'agent, ce qu'il traduisait à ce moment pouvant se perdre.

## Ce que le contrat a gagné

Rien n'est propre à Copilot dans le cœur : sa ligne de la table des harness livrés est vide, ce qu'il lui faut est déclaré dans son manifeste. Tout harness peut donc dire `framing: "contentLength"`, une ligne écrite avant de fermer son entrée (`closeLine`), que son plugin ouvre lui-même la session (`opens`). `agent/rename` et `agent/compact` sont ceux d'OpenCode, et les plugins peuvent y répondre par ce qu'il faut écrire au harness. Voir *Le contrat des harness en process*.

## Un module, plus un process (ESC-346)

Le plugin est un module (`run.wasm`), en version `2.2.0`, et n'a plus d'entrée `escouade-plugins copilot`. Ce qui précède reste vrai pour la conversation, à deux points près : `git add` et `git commit` sont jugés par Escouade (`fs/gitProtected`, racine `agentRepo`), et une adresse dont l'hôte n'est pas en ASCII n'est permise que pour une fois. Voir *Tous les plugins en WASM*.
