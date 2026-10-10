# Glossaire de l’interface

Les mots d’Escouade et leur traduction fixe : un même mot français se traduit toujours de la même façon, dans la fenêtre comme dans les textes du backend (`tr!`) et sur le site. Le mode d’emploi des catalogues est dans `README.md`, à côté.

Chaque tâche ajoute ses mots **dans sa section** (en bas), pour que les tâches menées en parallèle n’écrivent jamais les mêmes lignes. Les sections communes (en haut) ne bougent plus sans l’accord du contrôleur.

## Ton et typographie

- **Français :** tutoiement, phrases simples et concrètes, sans emphase ; apostrophe typographique ’ et guillemets « » (avec espaces insécables, comme le code existant).
- **Anglais :** même ton, « you » ; impératif pour les actions ; majuscule au premier mot seulement (« Add an account », pas « Add An Account ») ; apostrophe typographique ’ et guillemets “ ”.
- Une phrase entière par clé, jamais de morceaux recollés : l’ordre des mots change d’une langue à l’autre. Un mot de balisage au milieu d’une phrase passe par `<Rich k="…">`.
- « Aucun … » / « Rien à … » se disent « No … » / « Nothing to … ».
- Unités : les durées relatives s’écrivent avec une espace (« 5 min ago », « 3 h ago », « 4 days ago » ; « il y a 5 min ») ; les comptes à rebours et les durées sont compacts (« 4d 12h », « 1h48 », « 5m 12s » ; « 4j 12h »). Tout passe par `format.ts`.

## Noms propres (jamais traduits)

| Français | Anglais |
|---|---|
| Escouade | Escouade |
| Kanban | Kanban |
| Claude, Claude Code | Claude, Claude Code |
| Jira, Trello, GitHub, GitHub Issues | Jira, Trello, GitHub, GitHub Issues |
| isola | isola |

## Mots de l’app

| Français | Anglais | Note |
|---|---|---|
| agent | agent | |
| projet | project | |
| ticket | ticket | |
| worktree | worktree | |
| pilote auto | autopilot | |
| pilote auto en pause | autopilot paused | |
| en boucle | looping | un agent, un ticket |
| place (libre) | (free) slot | « 3 places libres » → “3 free slots” |
| Vue d’ensemble | Overview | |
| Réglages | Settings | |
| réglages du projet | project settings | |
| Éditeur | Editor | |
| onglet | tab | |
| arborescence | file tree | |
| source (de l’éditeur) | source | le projet ou le worktree d’un agent |
| Historique | History | |
| Non commités | Uncommitted | |
| Non enregistré / Enregistré | Unsaved / Saved | |
| Modifié / Ajouté / Supprimé | Modified / Added / Deleted | état git d’un fichier |
| commit, commiter | commit | |
| merger | merge | |
| pousser | push | |
| pull request (PR) | pull request (PR) | |
| branche, branche cible | branch, target branch | |
| conflit | conflict | |
| rebase | rebase | |
| dépôt | repository | |
| diff | diff | |
| tour | turn | d’une conversation |
| outil | tool | |
| conversation | conversation | |
| Réfléchit | Thinking | |
| question / réponse | question / answer | |
| autorisation, demande d’autorisation | permission, permission request | |
| Toujours autoriser | Always allow | carte de permission |
| Autoriser / Refuser | Allow / Deny | carte de permission |
| consignes | instructions | données à Claude |
| pièce jointe | attachment | |
| brouillon | draft | |
| file d’attente | queue | |
| étiquette | label | Jira, GitHub… |
| jeton | token | d’une intégration, d’une API |
| tokens | tokens | consommés par Claude |
| modèle | model | |
| effort | effort | |
| mode de permission | permission mode | |
| Coût | Cost | |
| quota | quota | |
| fenêtre de 5 h | 5-hour window | quota |
| hebdomadaire, Hebdo | weekly, Weekly | quota |
| 5 h / 7j | 5h / W | libellés compacts des quotas |
| remise à zéro, réinitialisation | reset | quota, compteur |
| limite d’usage | usage limit | |
| reprise, reprise automatique | resume, auto-resume | après la limite d’usage |
| Pause au-delà du quota | Pause above quota | |
| Préparation / Démontage | Setup / Teardown | étapes d’un worktree, d’un lancement de test |
| critères | criteria | d’un ticket |
| Bilan des critères | Criteria report | |
| boucles | loops | d’un ticket |
| Valider (un ticket) | Approve | |
| Renvoyer (un ticket) | Send back | |
| Remplir automatiquement | Fill in automatically | |
| commande de lancement | launch command | |
| lancement de test | test launch | |
| Lancer (une commande) | Run | |
| Relancer (une commande) | Restart | |
| Lancer (un ticket) | Start | |
| Stopper | Stop | |
| journal (d’un lancement) | log | |
| terminal | terminal | |
| notification | notification | |
| mise à jour | update | |
| redémarrer | restart | l’app |
| compte | account | compte Claude |
| serveur MCP | MCP server | |

## Colonnes du Kanban

| Français | Anglais | Id |
|---|---|---|
| À faire | To do | `todo` |
| En cours | In progress | `doing` |
| À tester | To review | `review` |
| Terminé | Done | `done` |

## Touches

| Français | Anglais |
|---|---|
| Ctrl | Ctrl |
| Maj | Shift |
| Entrée | Enter |
| Échap | Esc |
| Alt | Alt |

`keyLabel` (`src/lib/platform.ts`) écrit les touches d’un raccourci dans la langue de l’interface : on peut lui donner `Ctrl+Shift+F` ou `Ctrl+Maj+F`, `Ctrl+Enter` ou `Ctrl+Entrée` (écrire les nouveaux en anglais).

## L3 — editor

| Français | Anglais | Note |
|---|---|---|

## L4 — settings

| Français | Anglais | Note |
|---|---|---|

## L5 — integrations, boardSettings

| Français | Anglais | Note |
|---|---|---|

## L6 — board, stats

| Français | Anglais | Note |
|---|---|---|
| Après validation | After approval | résumé de ce que fait « Valider », dans l’en-tête du Kanban |
| Passer en tête | Move to top | menu d’un ticket à faire |
| Après (un ticket) | After | les tickets que celui-ci attend |
| Boucles max | Max loops | |
| Critères d’acceptation | Acceptance criteria | le formulaire d’un ticket |
| Objectif partiel | Partial goal | |
| Ce qui a été fait | What was done | |
| Avancement | Progress | |
| Resynchroniser | Resync | ticket importé |
| Lancer quand même | Start anyway | |
| L’agent résout | Let the agent resolve | conflit de merge |
| Tester (▶) | Test | lancement de test |
| Arrêter (les tests) | Stop | |
| Abandonner (les modifications) | Discard | |
| Abandonner les modifications ? | Discard your changes? | |
| statut d’un agent : En cours / Question / Prêt / Terminé / Erreur | Running / Question / Ready / Done / Error | « En cours » est « In progress » pour une colonne du Kanban, « Running » pour un agent |
| Entrée / Cache / Sortie | Input / Cache / Output | les séries de tokens |
| prompt | prompt | |
| Tout voir / Réduire | Show all / Show less | les listes des statistiques |
| Jour / Semaine / Mois | Day / Week / Month | la période des statistiques |

## L7 — nav, git

| Français | Anglais | Note |
|---|---|---|

## L8 — conv, composer

| Français | Anglais | Note |
|---|---|---|

## L9 — runs, shell

| Français | Anglais | Note |
|---|---|---|

## L10 — errors, backend

| Français | Anglais | Note |
|---|---|---|

## K — accounts

| Français | Anglais | Note |
|---|---|---|

## M — mcp

| Français | Anglais | Note |
|---|---|---|

## G — branches

| Français | Anglais | Note |
|---|---|---|
