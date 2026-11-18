# Dépannage

Les messages ci-dessous sont ceux qu’Escouade et le plugin ACP affichent, mot pour mot ; `{…}` est ce qui change d’une fois à l’autre.

## L’agent ne démarre pas

**« Prime Agent ne tourne qu’en Bypass (il ne demande jamais avant d’agir) : active « Autoriser le mode Bypass » dans Réglages › Agents, ou choisis un autre harness. »**
Prime Agent n’a que ce mode. Allume « Autoriser le mode Bypass » après avoir lu l’onglet Permissions, ou n’utilise pas Prime Agent.

**« Prime Agent n’a pas de mode qui ne change rien (son manifeste n’en dit aucun) : il ne tient pas une discussion en lecture seule. »**
Une discussion de l’onglet Docs ne tourne pas sur Prime Agent. Choisis un autre harness pour elle.

**« Prime Agent lit et écrit les fichiers lui-même, sans les faire passer par Escouade (son manifeste le dit) : la clôture des projets privés ne tiendrait pas, l’agent ne démarre pas dans ce projet privé. »**
Un de tes projets est privé : chaque agent a alors une clôture, et Prime Agent ne peut pas la tenir. Le message vaut pour tout projet tant qu’un projet est privé.

**« Prime Agent introuvable. Installe-le ou indique son chemin dans les réglages. »**
Escouade n’a pas trouvé `prime-agent` dans le `PATH`. Installe Prime Agent (onglet Installer), ajoute `~/.local/bin` au `PATH`, et relance Escouade.

**« Aucun compte Prime Agent : ajoute-en un dans Réglages › Comptes et harness. »**
Ajoute le compte de la machine.

**« Prime Agent n’a pas ouvert la session de l’agent : {raison}. S’il demande une connexion, connecte-le dans un terminal. »**
Prime Agent a refusé la session, ou son démon n’a pas démarré. Lance `prime-agent` dans un terminal : `/login` s’il n’a pas de fournisseur, et `prime-agent doctor` (plus bas).

## Pendant un tour

**« Prime Agent ne reprend pas sa session : l’agent repart d’une session neuve. »**
Prime Agent 0.10.3 ne reprend pas de session par ACP (`loadSession: false`), et ses sessions partent avec la connexion (`--no-session`). Après un redémarrage, l’agent repart d’une session neuve, sans le contexte d’avant ; Escouade 2.2.3 le dit, et redonne à cette session ses consignes (le protocole de son ticket) et un résumé de ce qu’il a fait, avec son prochain message. Avec Escouade 2.2.2, rien n’est dit ni redonné : redis-lui ce qu’il doit savoir.

**« Prime Agent : {erreur} »**
Prime Agent a fini le tour en erreur, avec ses propres mots (un modèle injoignable, un contexte trop long, une clé refusée). Lis-les.

## Son démon

`prime-agent --mode acp` ne travaille pas seul : il démarre, s’il n’y en a pas déjà un, le démon de Prime Agent (`prime-agent --mode daemon`), qui tient les sessions, leurs workers et leurs noyaux Python. Ce démon est **un pour tout l’utilisateur** : le même sert tes sessions à toi dans un terminal et tous les agents d’Escouade, quel que soit leur worktree. Son adresse : sous Linux et macOS, `/tmp/prime-agent-rust-<uid>/daemon.sock` (`$TMPDIR` s’il est posé) ; sous Windows, le tube nommé `\\.\pipe\prime-agent-daemon`.

Ce qui reste quand un agent s’arrête ou quand Escouade se ferme :

- **La session de l’agent, son worker et son noyau** : rien. Le plugin lance Prime Agent avec `--no-session` : la session appartient à la connexion d’Escouade, et part avec elle (aussitôt quand Escouade ferme son entrée, 30 s après si le process est tué).
- **Le démon** : sous Linux et macOS, il reste, vide (environ 60 Mo de mémoire), jusqu’à ce que tu l’arrêtes : il se détache du groupe de process qu’Escouade arrête. Sous Windows, il reste dans le Job de l’agent qui l’a démarré et s’arrête avec lui, en emportant les sessions des autres agents qui s’en servaient à ce moment-là (lu dans le code de Prime Agent, pas essayé sous Windows).

Les commandes, dans un terminal :

- `prime-agent status` : les démons qui répondent, leurs sessions et leur âge.
- `prime-agent list` : les agents de Prime Agent en vie (ceux d’Escouade n’y restent pas).
- `prime-agent doctor` : ce qui ne va pas ; `prime-agent doctor --fix` retire les sockets morts et arrête les services orphelins inactifs.
- `prime-agent shutdown` : arrête tous ses agents et le démon, après confirmation ; `--force` sans confirmation, et tue ce qui ne répond plus. **Il arrête aussi tes sessions à toi.**

`prime-agent update` arrête le démon pour se mettre à jour : les agents d’Escouade en cours perdent alors leur session. Mets à jour quand aucun ne tourne. Un démon garde l’environnement avec lequel il a démarré : après un changement de `PATH`, de certificats (`NODE_EXTRA_CA_CERTS`) ou de fournisseur, `prime-agent shutdown` puis relance l’agent.

## Le plugin ACP

Les messages du plugin qui fait tourner Prime Agent (« Le plugin de Prime Agent a redémarré. », « … s’est arrêté après {n} plantages… ») sont dans la documentation du plugin ACP.
