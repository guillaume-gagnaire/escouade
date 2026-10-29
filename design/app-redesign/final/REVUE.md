# Revue finale du redesign 2.0 (ESC-84, 2026-10-24)

Les 11 lots et ESC-166 sont dans main. Cette revue a passé l'app réelle (build de debug, banc e2e sur WebView2) face à l'aperçu validé, dans les deux thèmes, à 1440 × 900 et 1028 × 779 (et 1280 pour les replis), en français et en anglais. Elle a été faite sans sous-agent, comme le veut la consigne du ticket : la revue de fin d'Impeccable (`impeccable-finish-reviewer`) et le documenteur ont été faits à la main, d'après leurs consignes.

Les captures sont dans ce dossier (voir « Les captures » en bas).

## Corrections faites

| Défaut | Où | Correction | Vérifié par |
|---|---|---|---|
| La clé d'une carte du Kanban (« DEM-5 ») se coupait sur deux lignes, et la rangée du haut (clé, harness, pièces jointes, boucle) débordait d'une colonne de 220 px | `board/TicketCard.svelte`, l'aperçu | La carte est un conteneur (`tcard`) : sous 270 px, les carrés de la boucle s'en vont (« Boucle 1/5 » le dit) ; la clé et la boucle ne passent jamais à la ligne. Même règle dans l'aperçu, `kanban*.png` refaites | `board-look.spec.ts` (rangée alourdie d'une marque et d'un trombone, rien hors de la carte, clé sur une ligne, 1028 et 1440 px) |
| Deux e2e des liens échouaient | `tests/e2e/links.spec.ts` | Cause : `page.route` de Playwright rate par moments toutes les requêtes de la WebView2 (pas même un événement `request`) ; le test ne voyait pas l'ouverture et le vrai navigateur s'ouvrait. L'IPC est désormais lue sur le `fetch` de la page. Le premier échec venait de là sur main comme ici ; le second était la même cause, d'où son intermittence | 4 passes de suite vertes, puis la suite complète |
| La commande d'une tâche de fond débordait de la barre latérale, sans points de suspension | `Sidebar.svelte` (`.subl`) | Piste de grille bornée (`minmax(0, 1fr)`) | `sidebar.spec.ts` (aucune rangée sous la carte ne dépasse la barre) |
| Dans Réglages › Voix, la phrase des modèles (« Plus gros, il comprend mieux… ») se lisait comme la description de « Langue parlée » | `settings/VoiceTab.svelte` | La liste a sa ligne de titre « Modèle » | capture `app-settings-voice*` |
| Codex introuvable disait « Introuvable. Indique le chemin de claude. », et sa ligne restait vide une fois ce texte retiré | `settings/AppTab.svelte` | « Introuvable. » pour un autre harness que Claude Code (son chemin ne se règle pas là) | `SettingsModal.test.ts` |
| Au survol, les actions de l'arbre de l'éditeur réduisaient son titre à « Fic » dans une colonne de 200 px (choix 53) | `editor/EditorView.svelte` | Dans une colonne étroite (`@container tree-h`), elles passent sur la ligne du dossier, sous le titre | `choices-look.spec.ts` (titre entier au survol) |
| Deux captures prises pendant la montée d'une fenêtre (on lisait l'écran dessous) | `windows-look.spec.ts`, `shell.spec.ts` | Capture après la fin des animations finies | captures refaites |

## Restes notés par ESC-152, faits

- **« Ouvrir le dossier » dans Données locales** : une commande `data_dir` donne le vrai dossier des données (un autre en développement ou en bac à sable), écrit avec `~`, et le bouton l'ouvre ; la permission d'opener couvre `$HOME/.escouade*` (sous macOS et Linux, `**` ne prend pas un dossier en point). Test : `SettingsModal.test.ts`.
- **« Aujourd'hui 0,42 $ » d'un compte OpenCode** à la place des jauges, pour tout harness sans quota. Test : `AccountsTab.test.ts`.
- **La ligne « Coût » de Codex** dans Réglages › Harness, pour tout harness dont le coût est estimé (`caps.cost === 'estimated'`) : « Estimé d'après les tokens et les tarifs publics… », et à côté ce qu'il ne fait pas (« Pas de PDF joint · pas de tâches de fond »), tiré de ses capacités. Test : `SettingsModal.test.ts`.

## Listé, pas commencé (et pourquoi)

- **Les anciens noms de jetons** (`--ok`, `--accent`, `--term`, `--ui`, `--line2`, `--elev`, `--wait-soft`, `--accent-soft`, `--wait-ring`) sont encore lus par une trentaine d'endroits, dont des tests (`tab-strip.test.ts`) : ce sont des alias exacts des jetons 2.0, sans effet visible. Un renommage mécanique, à faire hors de la sortie. Fait par ESC-172 (liste dans `DESIGN.md` « Les jetons »).
- **L'en-tête d'un lancement à 1028 px en anglais** coupe la commande (« Start-… ») avant le dossier : lisible dans « Voir le log », à revoir avec l'ordre des replis de `RunView`.
- **Sur une carte d'agent étroite**, avec sous-agents et plan, le modèle se réduit à « Son… » : c'est l'ellipse prévue, mais le plan pourrait céder avant le modèle.
- **Les vues plein cadre en clair à 1028 px** (Vue d'ensemble, Stats) sont fidèles à l'aperçu ; rien à corriger, noté pour la revue de l'utilisateur.
- **macOS** : les feux dans les 84 px de la barre de titre restent à vérifier sur un Mac (docs/TODO-2.0.md).
- **Ce qui montre l'app ailleurs** (vidéo, captures et démonstration du site) : chantier ESC-85.

## Signalements du détecteur, rejugés dans l'app

Le détecteur d'Impeccable a été passé deux fois : sur les sources (`impeccable detect src/app.css src/App.svelte src/components`), puis sur le DOM réel de l'app (instantané pris par le banc e2e, styles compris, en sombre et en clair : agents au travail, Kanban, réglages).

| Signalement | Dans l'app | Décision |
|---|---|---|
| Petits textes de métadonnées (11 à 11,5 px) | 126 règles à 11,5 px, 43 à 11 px : durées, tokens, coûts, compteurs, touches. Le détecteur du DOM n'en relève qu'un groupe (« 11.5px body text ») | **Gardés.** Ce sont des métadonnées d'un outil dense, jamais du texte de lecture (13 px de base, 13,5 px pour la conversation) ; leur contraste passe l'AA dans les deux thèmes pour chaque couleur de projet (`contrast.test.ts`). Les marques de harness à 10,5 px sont des étiquettes d'un mot dans un filet ; les lettres de service à 9 px sont des logos, cachés aux lecteurs d'écran |
| Points animés de l'activité | Les points des sous-agents actifs du plan (respiration de 1,8 s) et celui de « Attend ta réponse » dans la vue d'ensemble | **Gardés.** Ils disent un état qui change (quelqu'un travaille, quelqu'un attend), pas une décoration ; ils s'arrêtent avec `prefers-reduced-motion` (`plan-look.spec.ts`, `overview-stats-look.spec.ts`). Le détecteur du DOM ne les relève plus |
| Reflet en dégradé de « Claude travaille… » | Le seul texte en dégradé (`Conversation.svelte`, `SubagentView.svelte`) | **Gardé.** Le contrat de direction le nomme parmi les trois mouvements du design (FORM) ; c'est un reflet qui passe sur un texte gris, pas une emphase ; figé en texte uni avec `prefers-reduced-motion` |
| Cartes posées sur le panneau, vues comme imbriquées | Le détecteur du DOM voit « Card inside card » : le panneau du travail porte la zone de saisie, les cartes du Kanban et la carte de fin de tour ; la fenêtre des réglages porte ses fiches de compte | **Gardé.** Le panneau n'est pas une carte : c'est la surface du travail (la thèse du contrat, « le travail posé sur un panneau en relief »), comme la fenêtre des réglages. Rien n'est encadré deux fois dans une même surface : les groupes des réglages et les appels d'outils n'ont pas de boîte |
| Débordement de `.path` (pied de la barre latérale) | Le dossier du projet coupé | **Voulu** : ellipse, chemin entier dans l'infobulle |
| Contraste bas de « Envoyer » | Le bouton désactivé (champ vide) | **Voulu** : un contrôle désactivé n'est pas tenu au contraste (WCAG 1.4.3) |
| Fond « crème » en clair | La teinte orange du projet de test sur le cadre clair | **Voulu** : le teint par la couleur du projet est un choix de l'utilisateur ; chaque teinte passe l'AA |

## Revue de fin d'Impeccable (faite à la main)

disposition: fix → corrections faites ci-dessus ; ce qui reste est listé.

- **persistence** : PRODUCT.md existe et est à jour. Chemin code-led : pas de tour de comps ni de `state.json` ; l'aperçu validé par l'utilisateur (« implémente le redesign comme ça », 2026-10-12, direction claire le 2026-10-23) tient lieu de comp, comme le dit le contrat. DESIGN.md est écrit après cette revue.
- **fidelity** : chaque écran est comparé à l'aperçu par les e2e « look » (styles calculés, places, dans les deux thèmes, aux deux tailles) : 67 tests verts avant les retouches. TYPE : Inter variable (cv11, ss03) et JetBrains Mono, comme l'aperçu. MATERIAL : surfaces plates teintées, filets, ombres sous ce qui flotte seulement, comme l'aperçu. GROUND : cadre `--p` 8 % sur #08090a en sombre, 7 % sur #f0f1f3 en clair, lu identique dans l'aperçu et l'app. Adaptations citées : la marque du harness n'apparaît qu'avec deux harness au travail (choix 14) ; les couleurs des modèles par rang, pas par nom (quatre harness, d'autres modèles que ceux de Claude) ; les replis par `@container` (demande du 2026-10-24) ; les choix 50 à 58, validés par l'utilisateur le 2026-10-24.
- **ceiling** : atteint pour ce monde (Linear, Raycast) : teint par projet qui glisse, lumière du projet, filets, icônes tracées, chiffres tabulaires, sélection et barres de défilement aux couleurs du thème, focus du projet.
- **material_fixes** : les sept corrections ci-dessus.
- **keep** : l'agent qui attend en tête, en ambre, et la fenêtre qui prend la couleur du projet.

## Les fonctionnalités de UX-NOTES.md, sur les quatre harness

Les harness Claude Code, Codex, OpenCode et Copilot CLI sont joués par leurs faux exécutables (`tests/fixtures/fake-*.cmd`) ; chaque e2e ci-dessous tourne sur l'app réelle. Les captures en anglais à 1028 × 779 (`*-small-en.png`) ont été regardées une à une.

| Choix | Vérifié dans l'app | Preuve |
|---|---|---|
| 1. Ctrl+Maj+N | menu « Harness » | `sidebar.spec.ts`, capture `new-agent-small-en` |
| 2. Réglages réorganisés | trois groupes, onglet Serveur MCP | `settings-look.spec.ts` |
| 3. Bascule de vue en segments | Escouade / Claude Code, Codex, OpenCode, Copilot | `claude-code-view.spec.ts`, `codex.spec.ts`, `opencode.spec.ts`, `copilot.spec.ts` |
| 4. Barre du champ réordonnée | modèle, effort, mode, puis pièce jointe, micro, conversation | `conversation-look.spec.ts` |
| 5. « Éditeur » hors de l'en-tête | dans la barre de titre | `shell.spec.ts` |
| 6. Menu de l'agent par « … » | et par clic droit | `windows-look.spec.ts` (`agent-menu`) |
| 7. Barre d'état par harness | jauges Claude, Codex ; crédits Copilot ; budget OpenCode | `shell.spec.ts`, `copilot.spec.ts`, `opencode.spec.ts` |
| 8. Bac à sable | en-tête d'un agent Codex, ce qu'il veut dire dans son mode ; aucun pour Claude Code | `Conversation.test.ts` (ajouté par cette revue) |
| 9. Lancement et Terminaux repliés en petite hauteur | à 779 px de haut, sur leur titre ; dépliés à 900 | captures `terminal-small` (« Terminaux 1 » replié) et `terminal` |
| 10. Sous-agents et tâches de fond | listés sous la carte ouverte | `subagents.spec.ts`, `bgtasks.spec.ts`, `sidebar.spec.ts` |
| 11. Pièces jointes du ticket | trombone sur la carte | `attachments.spec.ts` |
| 12. Accueil sans harness | « Aucun harness trouvé » | `harness.spec.ts`, capture `welcome` |
| 13. Échec de mise à jour | gardé dans À propos | `windows-look.spec.ts` (`update`) |
| 14. Quatre harness, mêmes règles | marque, coût, modes | `engines.spec.ts`, `codex/opencode/copilot.spec.ts` |
| 15. Menu « Harness » numéroté | 1 à 4, raccourci | capture `new-agent-small-en` |
| 16. L’agent sans worktree dans ce menu | | `Sidebar.test.ts`, `branches.spec.ts` |
| 17. Ce qu'un harness n'a pas, grisé avec sa raison | mode d'OpenCode, harness introuvable | `opencode.spec.ts`, capture `new-agent-small-en` |
| 18. Nouveaux agents sur la fiche du compte | trois menus | `accounts.spec.ts`, capture `app-settings-accounts-small-en` |
| 19. Un harness sans compte reste visible | « Aucun compte » | `accounts.spec.ts` |
| 20. « Changer de compte à » en segments | | capture `app-settings-accounts-small-en` |
| 21. Projection au-delà de 80 % | barre d'état | `accounts.spec.ts` |
| 22. Usage supplémentaire en ambre | groupe du harness | `StatusBar.test.ts` |
| 23. OpenCode : coût du jour sur son budget | barre d'état et (nouveau) fiche du compte | `opencode.spec.ts`, `AccountsTab.test.ts` |
| 24. Changement de compte au seuil, comme une passation | séparateur dans la conversation | `accounts.spec.ts` |
| 25. « Attend » et l'heure de relance | carte et infobulle | `sidebar.spec.ts` (`card-waiting`), `Sidebar.test.ts` |
| 26. PR sous la carte, un seul bouton de correction | « L'agent corrige » | `prs.spec.ts`, `sidebar.spec.ts` (`card-pr`) |
| 27. Hébergeurs en tête d'Intégrations | | `integrations.spec.ts`, `ForgesSection.test.ts` |
| 28. Cadenas d'un projet privé | tuile et titre | `settings-look.spec.ts` (`private`), `rail.spec.ts` |
| 29. Trois marques d'origine d'un réglage | .escouade.yml, local, en attente | `escouade-yml.spec.ts`, `SharedSection.test.ts` |
| 30. ▶ d'une commande : projet ou worktree | | `RunsSection.test.ts`, `terminals-look.spec.ts` (`run-conflict`) |
| 31. Worktree gardé en ambre sous l'archivé | | `app.spec.ts` |
| 32. Conversation vocale, une question à la fois | | `Conversation.test.ts`, `voice.spec.ts` |
| 33. Transcription en direct en plus pâle | | `Conversation.test.ts` |
| 34. Voix : Transcription puis Conversation, Local / En ligne | | `settings-look.spec.ts`, `VoiceTab.test.ts` |
| 35. Zoom en segments | | `display.spec.ts` |
| 36. Long menu des modèles | recherche, favoris, groupes | `model-menu.spec.ts`, `opencode.spec.ts`, `copilot.spec.ts` |
| 37. Bande verte « Escouade garde le fil » d'OpenCode | | `terminals-look.spec.ts`, capture `opencode-view-small-en` |
| 38. Demandes de la conversation en cartes | « Toujours autoriser » d'OpenCode, « Pour cette session » de Codex et Copilot | `opencode.spec.ts`, `codex.spec.ts`, `copilot.spec.ts`, `sidebar.spec.ts` (`list-answers`) |
| 39–49. Barre des projets | tuiles, menu, beaucoup de projets, premier démarrage | `rail.spec.ts`, `shell.spec.ts` |
| 50–58. Validés par l'utilisateur le 2026-10-24 | gardés tels quels | `choices-look.spec.ts`, captures `choice-*` |

## Responsive : le tour de tous les écrans

Chaque spec « look » mesure, à 1028, 1280 et 1440 px, en français et en anglais, que rien ne déborde de sa boîte et qu'aucun bouton, onglet ou titre ne passe sur deux lignes (`scrollWidth > clientWidth`, boîtes hors de leur parent) ; `narrow-columns.spec.ts` le fait pour l'en-tête, la saisie, le plan, une question, les onglets de Git et les en-têtes des terminaux, panneau latéral fermé, à sa largeur par défaut et côte à côte. La revue des captures a trouvé trois débordements que ces mesures ne voyaient pas (la carte du Kanban pleine, la tâche de fond, le titre de l'arbre) : corrigés et mesurés à leur tour.

## Les captures

L'app réelle (build de debug, données du banc e2e), prise par les specs e2e avec `ESCOUADE_E2E_SHOTS`. Celles de l'aperçu, pour comparer, sont dans `design/app-redesign/` sous le même nom.

**Chaque écran en quatre variantes** : `<écran>.png` (1440 × 900, sombre), `<écran>-light.png` (1440 × 900, clair), `<écran>-small.png` (1028 × 779, sombre), `<écran>-small-light.png` (1028 × 779, clair). Les écrans : `account-codex`, `account-signin`, `accounts-quotas`, `agent-menu`, `agents`, `card-pr`, `card-running`, `card-waiting`, `copilot-view`, `editor`, `git`, `history`, `kanban`, `opencode-view`, `overview`, `plan`, `plan-closed`, `pr-open`, `private`, `run`, `run-conflict`, `settings-about`, `settings-accounts`, `settings-app`, `settings-board`, `settings-claude`, `settings-integrations`, `settings-mcp`, `settings-network`, `settings-notifications`, `settings-projects`, `settings-terminals`, `settings-voice`, `sidebar`, `stats`, `terminal`, `test`, `test-run`, `ticket`, `update`, `wizard`, `wizard-private`, `wizard-summary`.

**Les choix 50 à 58** (validés par l'utilisateur le 2026-10-24), à 1028 × 779, en sombre puis en clair (`-light`) :
- 50 et 51 (en-tête replié, le chemin garde la branche) : `final/choice-50-51-header.png`, `final/choice-50-51-header-light.png`
- 52 (« … » reprend la disposition et les fichiers) : `final/choice-52-more.png`, `final/choice-52-more-light.png`
- 53, 54 et 55 (actions de l'arbre au survol, pas d'icône de type, arbre de 200 px) : `final/choice-53-54-55-tree.png`, `final/choice-53-54-55-tree-light.png`
- 56 (ce que trouve la recherche, expression invalide en rouge) : `final/choice-56-count.png`, `final/choice-56-count-light.png`, `final/choice-56-invalid.png`, `final/choice-56-invalid-light.png`
- 57 (une ligne de réglage garde la place de son nom, « Langue parlée » en menu) : `final/choice-57-row.png`, `final/choice-57-row-light.png`
- 58 (« Ordre des harness » en puces numérotées avec leurs flèches) : `final/choice-58-order.png`, `final/choice-58-order-light.png`

Les captures en anglais à 1028 × 779 et à 1280 px, prises par les mêmes specs, ne sont pas gardées ici : elles servent aux mesures.
