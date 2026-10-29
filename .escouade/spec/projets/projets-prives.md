---
title: "Projets privés"
subject: "Projets"
sources: ["docs/SPEC.md"]
tickets: ["ESC-139","ESC-140","ESC-142"]
decisions: []
compared: null
---

## Ce que c'est

Un projet peut être déclaré privé. Il est alors cloisonné dans les deux sens : les agents des autres projets ne le voient pas, et ses agents ne voient que lui. Le cloisonnement vaut dans le serveur MCP d'Escouade (voir *Serveur MCP*, « Ce qu'un agent voit et touche »), dans les fichiers que le harness d'un agent peut lire ou écrire, et dans ce qu'Escouade fait passer d'un projet à l'autre.

## Comportement

### Repère

Un cadenas suit le nom du projet, sur son onglet et parmi les projets des réglages (« Projet privé : invisible pour les agents des autres projets »).

### Ses fichiers

À chaque démarrage d'un agent (son process, ou son interface en vue terminal), Escouade calcule les chemins que son harness doit lui refuser, en lecture comme en écriture, seulement s'il existe un projet privé :

- pour un agent d'un projet qui n'est pas privé : le dossier de chaque projet privé, son dossier des worktrees et le dossier de chacun de ses agents, ses données dans le dossier d'Escouade (`attachments/<agent>`, `tickets/<ticket>`) et ses sessions de harness (`projects/<dossier>` de chaque dossier de configuration de Claude Code) ;
- pour un agent d'un projet privé : tout cela pour chaque autre projet, plus les fichiers joints et les dossiers de sessions de Claude Code qui ne sont pas à son projet (lus sur le disque au démarrage) ;
- pour les deux : le reste du dossier d'Escouade (`state.json`, qui tient tous les projets, conversations, journaux, jetons, comptes ; seuls `attachments` et `tickets` restent, par propriétaire), et ce que les harness gardent mêlé de tous les projets (`history.jsonl`, `file-history`, `todos`… de Claude Code et son `.claude.json` ; `sessions`, `history.jsonl` et bases de Codex ; `session-state` de Copilot CLI ; `storage` d'OpenCode), de chaque compte et du dossier par défaut ;
- jamais le dossier de l'agent ni un dossier au-dessus (un projet dans le dossier d'un autre). Un chemin sous un autre de la liste (un worktree sous le dossier de son projet) n'y est pas répété.

Par harness (pour Cursor CLI, Mistral Vibe, Antigravity CLI et Gemini CLI : la ligne « Ce qui protège » de la table de *Harness*) :

- **Claude Code** : un fichier de réglages à l'agent (`<données>/fence/<agent>.json`, par `--settings`, qui porte aussi le refus du serveur d'Escouade puisque Claude Code ne lit que le dernier `--settings` ; en vue Claude Code, dans le fichier de ses hooks) : `permissions.deny` avec `Read(//c/…)`, `Read(//c/…/**)`, `Edit(…)` pour chaque chemin (Read, Grep, Glob, les commandes de fichiers que son Bash et son PowerShell reconnaissent, les redirections ; Edit et Write), et hors de Windows natif `sandbox.filesystem.denyRead`/`denyWrite`, qui ne valent que si l'utilisateur a activé son bac à sable. Refusé en « Demander » comme en Bypass, dans les deux sens. **Garde-fou** : un script qui ouvre le fichier lui-même passe ; avec son bac à sable (macOS, Linux, WSL2), ses commandes sont tenues par le système.
- **Codex** : deux profils de permissions déclarés à son processus (`-c`, `default_permissions`) : `escouade-fence-plan` (`:read-only`, chaque chemin `deny`) et `escouade-fence` (le premier, plus son dossier et le dossier temporaire en écriture) ; le fil et chaque tour nomment celui du mode. **Barrière** : son bac à sable refuse ces chemins à toutes ses commandes (« Access to the path … is denied » sous Windows), sauf en Bypass (`danger-full-access`), qui n'en a pas.
- **OpenCode** : des règles de session ajoutées après celles du mode (la dernière qui correspond l'emporte) : `read`, `edit`, `list`, `external_directory` à `deny` pour chaque chemin et ce qui est dessous, sous Windows dans les deux écritures, et `bash` à `deny` pour une commande qui le nomme. **Garde-fou** : pas de bac à sable.
- **Copilot CLI** : il demande une permission pour tout chemin hors du dossier de l'agent ; Escouade refuse d'office, sans carte, celle qui en nomme un (`path`, `fileName`, `possiblePaths`, le texte de la commande), avec la raison « Refusé par Escouade : ce chemin est celui d'un autre projet, et l'un des deux est privé. ». **Garde-fou** : rien n'est demandé en Bypass (`allow-all`), ni à Escouade en vue Copilot CLI ; une commande peut atteindre un chemin qu'elle ne nomme pas.

### Quand le réglage change

Changer « Projet privé » ne coupe rien sur le moment ; le serveur MCP le prend dès l'appel suivant. Un agent dont le process a démarré sur une autre confidentialité (son projet privé ou non, et la liste des projets privés) le relance avant son tour suivant, entre deux tours seulement (ni tour en cours, ni préparation, ni tâche de fond, ni remote control ; session gardée, `--resume`). Une interface de vue terminal garde les siennes jusqu'à son prochain lancement. Un chemin créé depuis (un agent, un ticket, un fichier joint ou une session d'un autre projet) ne relance rien : il attend le prochain démarrage de l'agent.

### Ce qu'Escouade fait passer d'un projet à l'autre

- **Copier les réglages** (assistant « Nouveau projet ») : un projet privé n'est jamais proposé comme source, et l'assistant le dit (voir *Nouveau projet*). Copier un projet non privé dans un projet privé reste possible : rien du privé n'en sort.
- **Dupliquer la conversation** : la copie est toujours dans le projet de l'original (même session, même dépôt). Aucune action ne déplace un agent, une conversation ou un ticket vers un autre projet, et un historique rouvert ne revient qu'au projet dont il est (même id).
- **Dépendances** : « Après » et `after` ne prennent que des tickets du projet du ticket. Par le serveur, un ticket d'un autre projet est introuvable comme un ticket inexistant ; `split_ticket` fait des tickets du projet de l'agent, après le sien. Déplacer un ticket avant celui d'un autre projet est refusé : « <clé> est dans un autre projet. »
- **Ce qu'Escouade écrit aux agents** (consignes et protocole d'un ticket, premier message, boucles, relances, retours de tests, conflits, préparation, messages venus du serveur, passation à un autre harness, noms d'agents, branches, messages de commit et textes rédigés) est fait du ticket, du projet et de la conversation de l'agent seuls, jamais du nom ni du contenu d'un autre projet. Les messages venus du serveur sont signés du nom de l'agent appelant, qui ne voit que ce qu'on lui laisse voir. Les textes rédigés sont demandés sans session gardée (Claude Code `--no-session-persistence`, Codex `--ephemeral`, OpenCode et Copilot CLI : session effacée une fois lue) : rien d'une demande ne reste pour la suivante.
- **Ce que l'app envoie par ailleurs** : les notifications (« projet · agent ») et le nom d'une session de remote control (« projet · agent », vu sur claude.ai) ne nomment que le projet de l'agent et vont à l'utilisateur, jamais à un agent. Comptes et fichiers partagés entre projets : voir docs/TODO-2.0.md.

## Réglages

- Réglages › Projets, groupe « Confidentialité » : « Projet privé », éteint par défaut, enregistré avec le projet. Sous le réglage, « Ses fichiers » dit, harness par harness, « Garde-fou » ou « Barrière », avec ce qui n'est pas garanti. Un projet rouvert avec son historique garde son propre réglage.
- Groupe « Nouveaux projets » (sous les réglages du projet, même sans projet ouvert) : « Nouveaux projets privés », éteint par défaut.
- Pour que sessions, mémoire et historique du harness ne soient pas partagés non plus : un compte réservé (voir *Comptes*, « Un compte réservé au projet privé »).

## Limites

- Le cloisonnement des fichiers est un « garde-fou » ou une « barrière » selon le harness, jamais une vraie barrière pour tous : un compte système par projet n'est pas proposé, les harness, leurs comptes et les outils de l'utilisateur n'y suivraient pas.
- Entre projets qui ne sont pas privés, rien n'est cloisonné.
