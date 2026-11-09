# Permissions de Slack

Ce que le plugin peut faire, ce qui part vers Slack, et ce qu’il ne peut pas faire.

## Ce qu’il est

Un programme en TypeScript (`dist/plugin.mjs`), qu’Escouade compile en WASM et fait tourner isolé, dans Escouade : il n’a pas de processus à lui, pas d’accès à ton disque, et pas de Node à installer. Il ne fait que ce que l’hôte d’Escouade lui sert.

## Ce qu’il demande

| Permission | Ce que dit la fenêtre d’installation |
| --- | --- |
| `run` : `dist/plugin.mjs` | « Tourne dans Escouade, isolé » |
| `events` : `ticket.column`, `agent.status` | « Reçoit ces événements » |
| `network` : `slack.com` | « Se connecte à slack.com », appliqué par Escouade : il ne joint aucun autre hôte, ni une adresse de ta machine ou de ton réseau local |
| `secrets` : `token` | « Garde un secret : token » |

Les permissions `events`, `network` et `secrets` se retirent dans l’onglet **Permissions** de sa page : son programme est relancé sans elles. Pour l’arrêter tout à fait, désactive-le.

## Les faits qu’il reçoit

Seulement `ticket.column` et `agent.status`, et seulement ceux des projets où il est actif. Un projet privé ne lui envoie rien tant que tu ne l’y as pas activé. Il reçoit le fait entier (le projet, le ticket ou l’agent concerné, ce qui a changé), et n’en poste que deux cas : un ticket arrivé dans « À tester », un agent qui attend ta réponse.

## Ce qu’il lit

- Le fichier `channels.json` de son propre dossier, qu’Escouade lui donne au démarrage. Aucun autre fichier.
- Le jeton, qu’il demande à Escouade à chaque fait.

## Ce qui part vers Slack

Pour chaque message, une requête `POST https://slack.com/api/chat.postMessage`, depuis ta machine, avec :

- le jeton du bot, dans l’en-tête `Authorization` ;
- le nom du salon (`escouade-<projet>`, ou celui de `channels.json`) ;
- le texte : la clé et le titre du ticket, ou le nom de l’agent.

Escouade ne laisse partir que des requêtes en https vers `slack.com` et ses sous-domaines, sans suivre de redirection, 10 secondes et 1 Mio au plus, et jamais vers une adresse de ta machine ou de ton réseau local. Le proxy de **Réglages › Avancé › Réseau** est suivi.

## Ce qui n’y part jamais

Ni la conversation de l’agent, ni la description du ticket, ni ses critères, ni ton code, ni un chemin de ta machine. Le jeton ne part que vers `slack.com` : le plugin refuse d’envoyer un jeton qui ne commence pas par `xoxb-`.

## Le secret

Le jeton est le seul secret. Escouade le garde dans le trousseau du système, sous un nom propre au plugin, et ne le donne qu’à lui, quand il le demande. Il est masqué dans le journal.

## Ce qu’il ne peut pas faire

- Lancer un programme sur ta machine.
- Lire tes fichiers ou ceux de tes agents, ou écrire un fichier.
- Joindre un autre hôte que `slack.com`.
- Décider des autorisations de tes agents.
- Lire les messages de Slack : il n’appelle que `chat.postMessage`. Ne donne à son bot que le droit `chat:write`, c’est le seul dont il se sert.
