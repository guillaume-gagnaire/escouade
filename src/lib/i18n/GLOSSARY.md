# Glossaire de l’interface

Les mots d’Escouade et leur traduction fixe : un même mot français se traduit toujours de la même façon, dans la fenêtre comme dans les textes du backend (`tr!`) et sur le site. Chaque tâche qui ajoute un mot de l’interface le complète ici.

## Ton et typographie

- **Français :** tutoiement, phrases simples et concrètes, sans emphase ; apostrophe typographique ’ et guillemets « » (avec espaces insécables, comme le code existant).
- **Anglais :** même ton, « you » ; impératif pour les actions ; majuscule au premier mot seulement (« Add an account », pas « Add An Account ») ; apostrophe typographique ’ et guillemets “ ”.
- Une phrase entière par clé, jamais de morceaux recollés : l’ordre des mots change d’une langue à l’autre. Un mot de balisage au milieu d’une phrase passe par `Rich` et un `{nom}`.

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
| Vue d’ensemble | Overview | |
| Réglages | Settings | |
| Éditeur | Editor | |
| Historique | History | |
| Non commités | Uncommitted | |
| commit, commiter | commit | |
| merger | merge | |
| pousser | push | |
| pull request (PR) | pull request (PR) | |
| branche | branch | |
| remise à zéro | reset | quota, compteur |
| quota | quota | |
| limite d’usage | usage limit | |
| Pause au-delà du quota | Pause above quota | |
| Toujours autoriser | Always allow | carte de permission |
| Autoriser / Refuser | Allow / Deny | carte de permission |
| Préparation / Démontage | Setup / Teardown | étapes d’un worktree, d’un lancement de test |
| critères | criteria | d’un ticket |
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
| Réfléchit | Thinking | |
| modèle | model | |
| effort | effort | |
| mode de permission | permission mode | |
| Coût | Cost | |
| tokens | tokens | |

## Colonnes du Kanban

| Français | Anglais |
|---|---|
| À faire | To do |
| En cours | In progress |
| À tester | To test |
| Terminé | Done |

## Touches

| Français | Anglais |
|---|---|
| Ctrl | Ctrl |
| Maj | Shift |
| Entrée | Enter |
| Échap | Esc |
| Alt | Alt |

`keyLabel` (`src/lib/platform.ts`) écrit les touches d’un raccourci dans la langue de l’interface : on peut lui donner `Ctrl+Shift+F` ou `Ctrl+Maj+F`, `Ctrl+Enter` ou `Ctrl+Entrée`.
