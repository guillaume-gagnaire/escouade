# Utiliser

## Lancer un agent

Choisis « Copilot CLI » dans le menu « Harness » de « Nouvel agent », ou fais-en le harness par défaut d’un projet (Réglages › Projets › Projet, « Harness par défaut »). Chaque agent a son `copilot --server --stdio`, sans le serveur MCP GitHub que Copilot branche de lui-même, sans accès au dossier temporaire du système, et sans les jetons GitHub dans l’environnement de ses commandes.

Le menu des modèles écrit le multiplicateur de chacun (`×3`), ou son coût relatif sur un compte Free ; `auto` n’est jamais proposé. Le modèle et le mode changent tout de suite, même pendant un tour.

Un message envoyé pendant un tour y entre aussitôt. Les images jointes passent ; les PDF non. Seule la commande `/compact` est proposée, suivie si tu veux de ce qu’il faut garder. Le coût d’un tour vient des crédits d’IA de ses appels. Un agent Copilot ne reprend pas sur un autre compte.

## Les modes

Copilot n’a pas de bac à sable quand Escouade le pilote : il demande, et ce sont les règles du plugin qui répondent sans carte aux demandes sûres. Le reste arrive en carte.

| Mode | Chez Copilot CLI |
|---|---|
| Demander | Les lectures et les écritures du dossier de l’agent passent (sauf une écriture dans `.git` ou dans la configuration de Copilot : `.copilot`, `.github/copilot`, `.github/hooks`) ; les commandes, `git add` et `git commit` compris, le web, les outils MCP et les chemins hors du dossier demandent. |
| Édits auto | Pareil, et `git add` et `git commit` du worktree passent quand la commande n’est que cela. |
| Plan | Copilot étudie et propose un plan sans rien modifier : ses lectures du dossier passent, une écriture demande. |
| Auto | Copilot enchaîne seul jusqu’au bout de sa tâche (son mode autopilot), avec les règles d’Édits auto. |
| Bypass | Copilot ne demande rien (son mode `allow-all`). |

Une carte de Copilot a « Autoriser », « Pour cette session » et « Refuser » ; pas de « Toujours », qui écrirait une règle durable dans la config de Copilot. Un formulaire d’un serveur MCP devient une carte à champs. Un plafond de crédits de la session atteint devient une carte qui propose d’en ajouter, de le lever ou d’arrêter.

## La vue Copilot CLI

La « Vue Copilot CLI » fait tourner l’agent dans l’interface de Copilot, dans un terminal de la fenêtre, sur la même session. Ses hooks passent par un plugin propre à l’agent, donné à cette seule interface : rien n’est écrit dans ton dossier de Copilot. Dans cette vue, ses demandes se répondent dans l’interface de Copilot, où tu les vois : les règles du plugin n’y répondent pas. Pour que rien n’y passe sans toi :

- en Édits auto et en Auto, chaque écriture demande : Copilot ne sait pas permettre les écritures du dossier sans celles de son `.git` (un hook, `core.fsmonitor`, puis un `git commit` sans question), et rien ne prouve qu’un refus par chemin couvre les fichiers sous `.git`. En Auto, seuls `git add` et `git commit` passent sans question ;
- l’outil d’écriture de Copilot est refusé dans `.git`, `.copilot`, `.github/copilot`, `.github/hooks` et dans le dossier des fichiers joints, dans tous les modes (`--deny-tool=write(…)`). Ce refus ne vise que cet outil : en Bypass, une commande (`--allow-all`) peut encore écrire dans ces dossiers ;
- une discussion y est tenue en lecture seule : ses écritures et ses commandes sont refusées ;
- dès qu’un projet privé existe, un agent a une clôture et n’a pas de vue Copilot CLI, et la fenêtre le dit : rien n’y tient la clôture, il reste dans sa conversation, qui la tient.

## Une discussion de l’onglet Docs

Une discussion ne change rien : ses lectures du dossier passent, et Escouade refuse ses écritures et ses commandes.

## Les projets privés

Pour un projet privé, le plugin refuse aussitôt, sans carte, toute demande de Copilot qui nomme un chemin d’un autre projet : le chemin lu ou écrit, ceux que sa commande peut toucher, le dossier où elle tourne, le texte de la commande. « Pour cette session » n’est plus proposé, pour que chaque demande passe devant ce refus.

Ses limites :

- c’est un garde-fou, pas une barrière : une commande qui atteint un chemin sans le nommer passe ;
- en Bypass, Copilot ne demande rien : la clôture ne tient plus ;
- la vue Copilot CLI est refusée à un agent clôturé ;
- les sessions de Copilot sont celles du compte : pour qu’un projet privé ne les partage pas, donne-lui un compte réservé à lui seul.
