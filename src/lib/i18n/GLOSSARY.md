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
| 5h / 7j | 5h / W | libellés compacts des quotas |
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

## L2 — langues, menus natifs

| Français | Anglais | Note |
|---|---|---|
| Application | Application | onglet des réglages |
| Langue de l’interface | Interface language | |
| Langue des textes rédigés par Claude | Language of texts written by Claude | |
| Système (Français) | System (French) | la langue du système, nommée dans celle de l’interface |
| Comme l’interface | Same as the interface | |
| Français, English | Français, English | une langue proposée est écrite dans elle-même (`LANG_NAMES`) |
| Afficher / Quitter | Show / Quit | menu de l’icône de la barre des tâches |
| agent en attente | agent waiting | infobulle de l’icône |
| À propos d’Escouade, Masquer Escouade, Masquer les autres, Tout afficher, Quitter Escouade | About Escouade, Hide Escouade, Hide Others, Show All, Quit Escouade | menu de l’app sous macOS : les mots et les majuscules de macOS |
| Édition : Annuler, Rétablir, Couper, Copier, Coller, Tout sélectionner | Edit: Undo, Redo, Cut, Copy, Paste, Select All | idem |
| Fenêtre : Placer dans le Dock, Réduire/Agrandir, Fermer | Window: Minimize, Zoom, Close | idem |

## L3 — editor

| Français | Anglais | Note |
|---|---|---|
| Nouveau fichier / Nouveau dossier | New file / New folder | arborescence, menu |
| Ignoré par git | Ignored by git | fichier de l’arborescence |
| Voir les changements | Show changes | le texte comparé à sa version de référence |
| Comparer | Compare | le fichier comparé à la version du disque |
| Recharger | Reload | |
| Garder ma version | Keep my version | |
| bloc (d’une comparaison) | block | « Annuler ce bloc » → “Revert this block”, « Prendre ce bloc » → “Take this block” ; remplace le « chunk » de CodeMirror |
| corbeille | trash | « Il part dans la corbeille. » → “It goes to the trash.” |
| propre (un worktree sans modification) | clean | |
| modif. | change(s) | « 4 modif. » → “4 changes” |
| liste tronquée | list truncated | l’arborescence trop grande pour être lue en entier |
| Tabulations / Espaces | Tabs / Spaces | l’indentation, dans la barre d’état |
| Texte | Plain text | langue d’un fichier inconnu |
| Définitions | Definitions | liste des endroits où mène un identifiant |
| Suppr | Del | la touche, dans le menu de l’arborescence |

## L4 — settings

| Français | Anglais | Note |
|---|---|---|
| Réseau / À propos | Network / About | onglets des réglages (les autres : Application, Claude Code, Notifications, Projets → Projects, Kanban, Intégrations → Integrations, Terminaux → Terminals) |
| Identité | Identity | le groupe « Nom, dossier, couleur » d’un projet |
| Zone sensible | Danger zone | « Fermer le projet » |
| Lancement (section des commandes de lancement) | Launch | la section de la barre latérale et du projet ; « Lancement de test » reste Test launch |
| Rédigé par l’agent / Direct, avec un message proposé | Written by the agent / Direct, with a suggested message | qui écrit les commits d’un projet |
| Remplacer les commandes / Ignorer | Replace the commands / Ignore | la proposition de Claude, lue en entier avant d’être prise |
| proposé, proposition (de Claude) | suggested, suggestion | « 2 commandes proposées » → “2 commands suggested” |
| écarté (une commande refusée pour ce qu’elle cacherait) | left out | « 1 écartée » → “1 left out” |
| vide = … | blank = … | ce que fait un champ laissé vide |
| introuvable | not found | un shell, un exécutable |
| Sous-dossier | Subfolder | le dossier, dans le worktree ou le projet, où une commande tourne |
| Me prévenir pour / Canaux | Notify me about / Channels | notifications |
| Processus (inactifs) | Processes (idle) | « Arrêter les processus Claude inactifs » → “Stop idle Claude processes” |
| Ignorer la vérification des certificats TLS | Skip TLS certificate verification | |
| Exclusions (du proxy) | Exclusions | la liste NO_PROXY |
| Données locales | Local data | |

## L5 — integrations, boardSettings

| Français | Anglais | Note |
|---|---|---|
| Comptes connectés | Connected accounts | Jira, Trello, GitHub |
| Connecter / Déconnecter | Connect / Disconnect | un compte |
| trousseau (du système) | system keychain | là où le système garde les jetons |
| source (liée) | (linked) source | le projet Jira, le tableau Trello ou le dépôt GitHub associé à un projet d’Escouade |
| Projet / Tableau / Dépôt | Project / Board / Repository | ce que chaque service appelle l’endroit de ses tickets |
| Correspondance des statuts | Status mapping | l’état que prend le ticket externe selon la colonne |
| statut (d’un ticket externe) | status | |
| Commenter | Comment | un commentaire écrit sur le ticket externe |
| étiqueté | labeled | tickets importés par leur étiquette |
| Valider et merger / Valider + PR / Valider et pousser | Approve and merge / Approve + PR / Approve and push | le bouton d’un ticket « À tester » |
| Laisser en l’état | Leave as is | ce que fait « Valider » quand rien n’est fusionné ni poussé |
| Stratégie (de merge) | Strategy | Merge commit / Squash / Rebase restent tels quels |
| En cas de conflit | On conflict | Me demander / L’agent résout / Annuler → Ask me / Let the agent resolve / Cancel |
| En parallèle | In parallel | le nombre d’agents qui prennent des tickets en même temps |
| Relancer les tests avant | Re-run the tests first | |
| Commande de tests | Test command | |
| Message de commit généré | Generated commit message | |
| Pris dès qu’une place se libère | Picked up as soon as a slot is free | un ticket « À faire » |
| attend (un ticket attend un autre) | waits for / is waiting for | « DEM-5 attend DEM-3 » |
| Synchronisation | Sync | le groupe des réglages qui répercute le Kanban sur les tickets externes |
| Import automatique | Automatic import | |
| Vérifier toutes les | Check every | « Vérifier toutes les 15 min » → “Check every 15 min” |
| PR en brouillon | Draft PR | |

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
| Cet agent / Tout le projet | This agent / Whole project | les deux portées du panneau des fichiers non commités |
| Voir le diff | Show diff | bouton du panneau des fichiers |
| Commit… / Commit tout… | Commit… / Commit all… | « Commiter » (le bouton de la fenêtre de commit) → “Commit” |
| Régénérer | Regenerate | le message de commit proposé par Haiku |
| Unifié / Côte à côte | Unified / Side by side | les deux présentations d’un diff |
| Jamais commité | Never committed | les fichiers copiés dans les worktrees que le commit laisse de côté |
| Abandonner les modifications | Discard changes | menu d’un fichier non commité et sa confirmation |
| Basculer (sur la branche de base) | Switch (to the base branch) | avant un merge : « Basculer et merger » → “Switch and merge” |
| Squash (un seul commit) | Squash (a single commit) | option du merge |
| Archiver / Restaurer / Archivés | Archive / Restore / Archived | un agent |
| Dupliquer la conversation | Duplicate the conversation | menu d’un agent |
| remote control | remote control | écrit en minuscules, comme dans la conversation |
| Terminaux / terminal terminé | Terminals / exited | un terminal dont le shell s’est arrêté |
| À voir (un agent à regarder) | Needs a look | l’info-bulle d’un onglet de projet qui clignote |
| Réduire / Agrandir / Restaurer (la fenêtre) | Minimize / Maximize / Restore | boutons de la barre de titre |

## L8 — conv, composer

| Français | Anglais | Note |
|---|---|---|
| sous-agent | subagent | |
| tâche de fond | background task | |
| En cours / Question / Prêt / Terminé / Erreur | Running / Question / Ready / Done / Error | état d’un agent (en-tête de la conversation) |
| Bas / Moyen / Élevé / Très élevé / Max | Low / Medium / High / Very high / Max | niveaux d’effort |
| Demander / Plan / Édits auto / Bypass | Ask / Plan / Accept edits / Bypass | modes de permission (« Auto » reste « Auto ») |
| Approuver le plan | Approve the plan | carte de permission d’un plan |
| Continuer à planifier | Keep planning | refus d’un plan |
| Valider (les réponses à une question) | Submit | carte de question |
| Tester | Test | bouton d’un agent, lance son worktree |
| Préparer le lancement | Prepare launch | |
| Disposition | Layout | classique / côte à côte → classic / side by side |
| archivé | archived | |

## L9 — runs, shell

| Français | Anglais | Note |
|---|---|---|
| Lancement (la section des commandes) | Launch | comme dans les réglages : « Commandes de lancement… » → “Launch commands…” |
| Tout lancer / Tout arrêter | Run all / Stop all | |
| Proposer des commandes | Suggest commands | |
| Configurer | Configure | |
| prêt / en cours / arrêt… / arrêté / terminé / planté | ready / running / stopping… / stopped / done / crashed | l’état d’une commande de lancement, en minuscules (« planté (code 2) » → “crashed (code 2)”) |
| recette | recipe | les commandes que l’agent propose pour tester son travail |
| Préparation n / processus n | Setup n / process n | les noms donnés aux étapes d’une recette quand l’agent n’en a pas donné |
| non attendu | skipped | une étape qu’un test en échec n’attend plus |
| Rouvrir | Reopen | le navigateur d’un test |
| Voir le log / Voir les logs | View log / View logs | |
| racine du worktree | worktree root | |
| Processus terminé | Process exited | un terminal dont le shell s’est arrêté |
| Session 5 h / Hebdo / reset | 5-hour session / Weekly / resets in | barre d’état, telle qu’elle est aujourd’hui (la tâche K5 la refait) |
| Aperçu de l’onglet | Tab preview | fenêtre d’un nouveau projet |
| Nouveautés (d’une version) | What’s new | notes d’une mise à jour |
| prête · Redémarrer (mise à jour) | ready · Restart | barre d’état |
| Quitter quand même | Quit anyway | quand des fichiers ne sont pas enregistrés |

## L10 — errors, backend

| Français | Anglais | Note |
|---|---|---|
| introuvable | not found | « {path} introuvable » → “{path} not found” ; une erreur du backend, en minuscules comme en français |
| existe déjà | already exists | |
| en lecture seule | read-only | |
| chemin hors du dossier | path outside the folder | |
| racine (de la source) | root (of the source) | |
| dépôt distant / branche distante | remote / remote branch | « Plusieurs dépôts distants » → “Several remotes” |
| tirer (des commits) | pull | « 3 commits à tirer » → “3 commits to pull” |
| Fetch terminé | Fetch done | |
| Déjà à jour | Already up to date | |
| mettre de côté (des modifications) | stash | |
| HEAD détachée | detached HEAD | |
| janv., févr. … / S12 / 27/09 | Jan, Feb … / W12 / Sep 27 | les étapes des statistiques, écrites par le backend |
| Autoriser Bash : npm test ? | Allow Bash: npm test? | notification du système ; « Approuver le plan » → “Approve the plan” |
| Tâche terminée | Task done | notification du système |
| activité d’un agent : Lit, Cherche, Modifie, Écrit, Lance, Délègue, Rédige, Réfléchit | Reading, Searching, Editing, Writing, Running, Delegating, Replying, Thinking | le verbe seul est le début du verbe suivi de ce qu’il touche (« Lit src/a.ts » → “Reading src/a.ts”) |
| Contexte compacté / Nouvelle conversation Claude (contexte vidé) | Context compacted / New Claude conversation (context cleared) | notices de la conversation |
| Implémentation conforme au ticket / Tests verts | Implementation matches the ticket / Tests pass | critères par défaut d’un ticket |
| aucun ticket ne démarre | no tickets will start | ce que dit le Kanban (`BoardIssue`), comme `board.claudeMissing` |
| Interrompu / Erreur : … / Bilan des critères manquant | Interrupted / Error: … / Criteria report missing | pourquoi un ticket est bloqué |
| Validation… / Validation interrompue / Validation en cours | Approving… / Approval interrupted / Approval under way | l’étape d’un ticket « À tester » qu’on valide |
| prêt à tester / bloqué | ready to review / blocked | notification d’un ticket |
| ⤵ Mergé dans … / ⇡ Poussé sur … / ◇ Laissé dans le worktree / ∅ Aucune modification | ⤵ Merged into … / ⇡ Pushed to … / ◇ Left in the worktree / ∅ No changes | ce que devient un ticket validé |
| recette (de lancement) | recipe | ce que l’agent propose pour lancer son worktree |
| (copie), (copie 2) | (copy), (copy 2) | le nom d’un agent dupliqué |
| La préparation / le démontage du worktree a échoué sur … | The worktree setup / the worktree teardown failed on … | |
| Worktree préparé (3 commandes, 12 s). | Worktree set up (3 commands, 12 s). | |
| Ko / Mo | KB / MB | tailles écrites par le backend |
| Assignés à moi / Sprint actif / À faire / Mes cartes | Assigned to me / Active sprint / To do / My cards | filtres de l’import |
| Ouverte / Fermée | Open / Closed | états d’une issue GitHub |
| Carte | Card | type d’un ticket Trello dans l’import ; « Tâche », « Sous-tâche » de Jira restent “Task”, “Sub-task” |
| Non assigné(e) | Unassigned | |
| refuse ces identifiants / refuse l’accès / limite les requêtes | refuses these credentials / denies access / limits the requests | refus d’un service (401, 403, 429) |
| injoignable / réponse illisible | can’t be reached / unreadable answer | |
| trousseau du système illisible | the system keychain can’t be read | |
| Synchro interrompue | Sync interrupted | |
| Plus tard | Later | bouton de la notification d’un redémarrage pour une mise à jour |
| Enregistre d’abord tes fichiers | Save your files first | |
| shell « … » introuvable | shell “…” not found | |
| terminal fermé | terminal closed | |

## L11 — textes pour Claude

Ce qu’Escouade dit à Claude ou lui fait écrire suit « Langue des textes rédigés par Claude » (`Core::lang().claude` côté Rust, `tIn(app.lang.claude, …)` côté fenêtre). Les clés du JSON que le code relit dans ses réponses restent les mêmes dans les deux langues : `criteres`, `n`, `ok`, `note`, `avancement`, `lancement`, `preparation`, `processus`, `nom`, `commande`, `dossier`, `env`, `url`, `ouvrir`, `demontage`, `commandes`, et la clôture ```` ```escouade ````.

| Français | Anglais | Note |
|---|---|---|
| Tu travailles en autonomie sur le ticket … | You are working autonomously on Escouade ticket … | protocole d’un ticket |
| bilan (des critères), termine par le bilan | (criteria) report, end with the report | |
| Boucle 2/5 / Critères non atteints | Loop 2/5 / Criteria not met | |
| Reprends … là où tu en étais | Pick up … where you left off | |
| revérifie les critères | check the criteria again | |
| Retour de test sur … | Test feedback on … | « Renvoyer » un ticket |
| Prépare le lancement de test / Ports réservés | Prepare the test launch / Ports reserved | |
| L’utilisateur a refusé cette action. | The user refused this action. | refus sans message |
| Continue à planifier : … | Keep planning: … | refus d’un plan |
| Cette conversation a été copiée depuis … | This conversation was copied from … | consigne d’une copie |
| est pris par un agent / est revenu « À faire » | is picked up by an agent / is back in “To do” | commentaires publiés dans Jira, Trello, GitHub |
| est prêt à tester / est terminé / critères atteints | is ready to review / is done / criteria met | idem |
| balises `<tache>`, `<fichiers>`, `<sujets-recents>`, `<lancement>` | `<task>`, `<files>`, `<recent-subjects>`, `<launch>` | cadres des questions ; le faux `claude` reconnaît les deux |

## K — accounts

| Français | Anglais | Note |
|---|---|---|
| Principal | Main | le compte Claude habituel de l’utilisateur |
| connexion (d’un compte Claude) | sign-in | « Pas connecté » → “Not signed in” ; « Connexion expirée » → “Sign-in expired” |
| relancer Claude Code (pour un compte) | run Claude Code again | « relance Claude Code pour ce compte » → “run Claude Code again for this account” |
| compte en cours | current account | celui sur lequel partiraient les nouveaux agents |

## M — mcp

| Français | Anglais | Note |
|---|---|---|
| Claude (hors Escouade) | Claude (outside Escouade) | qui appelle le serveur MCP sans être un agent d’Escouade |
| client inconnu | unknown client | une requête refusée avant que son jeton dise d’où elle vient |
| journal d’activité | activity log | ce que le serveur MCP a reçu et répondu |
| requête refusée | request refused | « Requête refusée : jeton inconnu » → “Request refused: unknown token” |
| en-tête (HTTP) | (HTTP) header | « en-tête Host », “Host header” |

## G — branches

| Français | Anglais | Note |
|---|---|---|
