# Utiliser

## Lancer un agent

Choisis « Codex » dans le menu « Harness » de « Nouvel agent », ou fais-en le harness par défaut d’un projet (Réglages › Projets › Projet, « Harness par défaut »). Chaque agent a son `codex app-server`, sur le premier compte actif de Codex qui a encore du quota.

Les modèles et leurs efforts sont ceux que Codex liste pour le compte ; avant la première lecture, le menu ne propose que GPT-6-Luna. Le modèle choisi par le `config.toml` du compte y est aussi. Codex prend le modèle, l’effort et le mode au tour suivant.

Un message envoyé pendant que Codex travaille entre dans son tour au lieu d’attendre. Les images jointes passent ; les PDF non. Seule la commande `/compact` est proposée. Codex ne dit pas le coût : Escouade l’estime d’après les tokens et le tarif public d’OpenAI, et l’écrit « estimé ».

Un agent Codex ne reprend pas sur un autre compte : à la limite, il attend la remise à zéro, ou continue sur un autre harness.

## Les modes

Les modes passent par le bac à sable de Codex. Ce que Codex demande arrive en carte ; ce que son bac à sable lui permet, il le fait sans demander.

| Mode | Chez Codex |
|---|---|
| Plan | Son mode Plan, bac à sable en lecture seule : il étudie, peut te poser des questions, puis propose son plan sur une carte. |
| Demander | Bac à sable : écriture dans le dossier de l’agent, réseau coupé. Une commande qui en sort demande, `git add` et `git commit` compris (Codex protège le dossier `.git`). |
| Édits auto | Pareil, et le plugin accepte d’office `git add` et `git commit` du worktree de l’agent (voir l’onglet Permissions). |
| Auto | Le relecteur automatique de Codex décide des demandes à ta place, et la conversation dit ce qu’il a approuvé. `git add` et `git commit` passent comme en Édits auto. |
| Bypass | Codex ne demande rien et sort de son bac à sable. |

Une carte de Codex a « Autoriser », « Pour cette session » et « Refuser ».

## La vue Codex

La « Vue Codex » fait tourner l’agent dans l’interface de Codex, dans un terminal de la fenêtre, sur la même conversation. Ses hooks sont déclarés pour cette seule interface, par sa ligne de commande : rien n’est écrit dans ton `config.toml`. Dans cette vue, les demandes se répondent dans l’interface de Codex, et le plugin n’accepte rien d’office.

## Une discussion de l’onglet Docs

Une discussion tourne dans le bac à sable en lecture seule du mode Plan, quel que soit le mode : Codex n’y écrit rien.

## Les projets privés

Pour un projet privé, le plugin déclare à Codex deux profils de permissions où les chemins des autres projets sont refusés (`deny`), à la place de son bac à sable habituel. Le bac à sable de Codex les applique à toutes ses commandes : c’est une barrière, pas une simple règle.

Ses limites :

- en Bypass, Codex n’a pas de bac à sable : la clôture ne tient plus pour ses commandes ;
- avec une clôture, un `git commit` qui lit un fichier (`-F`, `--file`) va à la carte, même en Édits auto ;
- les sessions et l’historique de Codex sont ceux du compte : pour qu’un projet privé ne les partage pas, donne-lui un compte réservé à lui seul.
