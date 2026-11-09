# Utiliser

## Lancer un agent

Choisis « Cursor CLI » dans le menu « Harness » de « Nouvel agent », ou fais-en le harness par défaut d’un projet (Réglages › Projets › Projet, « Harness par défaut »). Chaque agent a son `agent acp` et son propre dossier de configuration.

Le menu « Modèle » liste les modèles de ton compte, Auto en tête, avec leurs efforts (de Bas à Très élevé). Une offre Free ne fait tourner qu’Auto : un autre modèle finit le tour en erreur, en le disant. Le modèle et le mode changent tout de suite.

Un message envoyé pendant un tour attend sa fin : Cursor arrêterait le tour en cours pour lui. Les images jointes passent ; pas les PDF, pas de commandes `/` dans Escouade (pour compacter son contexte, tape `/summarize` dans sa vue terminal), pas de « Dupliquer la conversation ». Cursor ne dit ni les tokens ni le coût d’un tour.

Les skills, commandes, agents et hooks de Claude Code de ton compte et du projet tournent aussi sous Cursor. Un hook qui refuse une écriture la bloque, sans que Cursor en donne la raison.

## Les modes

Cursor n’a pas de bac à sable sous Windows. **Il lit et écrit les fichiers lui-même**, même hors du dossier de l’agent, sans les faire passer par Escouade, qui ne peut donc pas l’en empêcher. Ce qu’il demande arrive à Escouade, qui en fait une carte, sauf ce que ses règles acceptent :

| Mode | Chez Cursor CLI |
|---|---|
| Demander | Son mode Agent. Chaque demande de Cursor (commande, web, outil MCP, écriture soumise) est une carte. |
| Édits auto | Son mode Agent. Une écriture ou une suppression qu’il soumet passe quand tous ses chemins sont dans le dossier de l’agent, hors `.git` ; le reste est une carte. |
| Plan | Son mode Plan : il étudie, puis propose son plan sur une carte. Approuvé, il le met en œuvre. |
| Bypass | Son mode Agent, et Escouade accepte tout ce qu’il demande. |

Une carte de Cursor a « Autoriser », « Pour cet agent » et « Refuser » : « Pour cet agent » garde la règle (`Shell(git status)`, par exemple) dans le dossier de configuration de l’agent, pour lui seul et ses reprises. Ses questions, son plan, ses tâches et ses sous-agents passent dans la conversation et le bandeau « Plan ».

Cursor n’a pas de mode qui ne change rien : il ne tient pas de discussion de l’onglet Docs.

## La vue Cursor CLI

La « Vue Cursor CLI » fait tourner l’agent dans l’interface de Cursor, dans un terminal de la fenêtre. Son suivi est limité : Escouade lit le journal de l’interface, sans savoir quand Cursor attend ton autorisation ou ta réponse ; regarde le terminal. Ce que tu y dis passe avant le message suivant, quand l’agent revient dans la vue d’Escouade.

## Les projets privés

Pour un projet privé, Escouade refuse aussitôt, sans carte, toute demande de Cursor qui nomme un chemin d’un autre projet : son titre, ses arguments, les chemins qu’elle touche.

Ses limites, qui sont grandes :

- Cursor lit et écrit les fichiers sans demander : la clôture ne tient pas pour ces lectures et ces écritures ;
- une commande qui atteint un chemin sans le nommer passe ;
- en Bypass, Escouade accepte ce qui ne nomme pas un chemin clôturé.
