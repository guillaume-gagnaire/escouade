# La vue Workspace

Date : 2026-11-25, mise à jour le 26 novembre après essai. Conçue sur maquettes avec l'utilisateur ; la toile reprend
le design de l'app, pas celui des maquettes (voir « Choix validés »).

## Le but

Voir d'un regard ce que font les agents d'un projet, comme une salle de contrôle : qui travaille, qui attend une
réponse, qui a lancé quoi, ce qui est sorti. Aujourd'hui, cette information est éparpillée : la barre latérale montre
un agent à la fois en détail, le plan est au-dessus de la conversation, les sous-agents sont une liste, la CI une
pastille.

La vue ne remplace rien. C'est une autre façon de regarder le même état, d'où l'on peut aussi répondre à une question
et écrire à un agent sans ouvrir sa conversation.

## Buts, hors périmètre, à éviter

Buts :

- une toile par projet, où chaque agent non archivé est un objet relié à ce qu'il utilise, lance et produit, et une
  toile de tous les projets depuis la « Vue d'ensemble » ;
- lisible de haut (les boussoles en piles) comme de près (un agent et tout son détail) ;
- répondre aux questions et aux permissions, et écrire à un agent, depuis la toile ;
- du mouvement seulement là où quelque chose se passe.

Hors périmètre (première version) :

- déplacer les cartes à la main et mémoriser leur place : Escouade range ;
- une mini-carte, une miniature de la page d'un artefact, modifier le plan depuis la toile ;
- la toile dans un volet de l'écran partagé.

À éviter :

- des flux décoratifs permanents : un trait animé veut toujours dire « ça tourne en ce moment » ;
- une seconde logique de question, de permission ou d'envoi : la toile monte les composants de la conversation ;
- une bibliothèque de graphe : le dépôt n'en a pas et n'en a pas besoin (`WorkflowDiagram.svelte` dessine déjà des
  flèches et zoome à la main).

## Les approches

Trois compositions ont été maquettées.

- **La boussole** (retenue) : l'agent au centre, un sens par côté. L'effet « flux » reste, les traits sortent par
  quatre côtés au lieu d'un. Coût : un cadre est grand, deux agents dépliés remplissent un écran.
- **Les couloirs** : quatre bandes horizontales, une colonne par agent. Le plus dense, mais c'est un tableau et les flux
  disparaissent.
- **La scène et les coulisses** : un seul agent déplié. Le plus aéré, mais on ne voit plus deux agents travailler
  ensemble, ce qui est le but.

Le coût de la boussole (la place) est payé par les **altitudes** : le niveau de zoom change ce qui est montré. C'est
l'utilisateur qui a choisi la boussole et ses deux altitudes.

## L'entrée et la sortie

- Le bouton « Éditeur » de la barre de titre (`TitleBar.svelte`) devient un menu à deux lignes, « Éditeur » et
  « Workspace », ouvert par `menu.showAt`. Le glisser du bouton vers un volet garde son sens actuel (l'éditeur).
- « Workspace » ouvre la toile du projet courant à la place de la zone de travail. La barre latérale reste.
- La « Vue d'ensemble » a un bouton « Workspace » : la toile de tous les projets, par-dessus elle.
- La toile a un bouton « Quitter le workspace » en haut à gauche (le premier dans l'ordre du clavier), et `Échap` fait
  de même quand aucun champ n'a le focus. On revient à la vue d'avant : le Kanban, les docs, les PR, un terminal,
  l'éditeur… tels qu'ils étaient, ou la liste de la « Vue d'ensemble ».
- État : un drapeau par projet, à côté de `board`, `docs`, `prs` dans `state.svelte.ts`, avec `openWorkspace` et
  `closeWorkspace`. Il passe devant les vues sœurs sans les éteindre (elles lui cèdent, et reviennent en quittant) ;
  les autres `open*` l'éteignent. La toile de tous les projets : `overviewCanvas`, `openOverviewCanvas`,
  `closeOverviewCanvas`.
- Double-clic sur une carte d'agent ou de sous-agent, ou `Entrée` sur la carte : quitte la toile et ouvre sa
  conversation (`selectAgent`, `openSubagent`) ; sur une tâche de fond, sa sortie (`openTask`).

## La toile

- Fond à points, repris du workspace de Kata avec les couleurs d'Escouade : `radial-gradient` d'un point de 1,2 px tous
  les 18 px, dont le pas et l'origine suivent la caméra.
- Se déplacer : glisser le fond, ou la molette (sauf sur une liste qui défile encore dans ce sens, celle du plan ou
  l'encart d'une question). Zoomer : Ctrl/⌘ + molette ou pincement, autour du pointeur, de 30 % à
  250 %. Clavier : `+`, `-`, `0` (ajuster).
- Une barre flottante en haut au centre : `−`, le pourcentage, `+`, « Ajuster », puis « n agents · n questions ».
- À l'ouverture, la toile est à 100 % : le premier agent (ou la première colonne) en haut à gauche, sous la barre
  flottante. « Ajuster » (et `0`) cadre tout. La caméra n'est pas mémorisée.
- Les agents sont rangés par Escouade, l'un sous l'autre, dans l'ordre de la barre latérale. Un agent lancé par un
  autre (`drivenAgents`) garde sa place, relié à lui par un trait pointillé « a lancé » qui longe la gauche des cartes ;
  sa carte le dit aussi en mots (« lancé par … »), pour un lecteur d'écran.
- Sur la toile de tous les projets, chaque projet est une colonne de ses agents sous son nom (avec sa couleur), les
  projets côte à côte dans l'ordre du rail, ceux sans agent omis.

## Les altitudes

Le zoom choisit ce qu'on montre : à mi-hauteur sous 60 % (jusqu'au zoom minimal, 30 %), de près à partir de 60 %.

### À mi-hauteur : des piles

La boussole, mais chaque côté est une pile : « Outils ×4 », « Équipe ×4 », « Livrables ×3 », et le plan réduit à son
titre et sa jauge. Un clic sur une pile la déplie sans changer le zoom.

### De près : la boussole dépliée

Un cadre par agent (un agent et tout ce qui lui appartient), l'agent au centre.

**Un agent qui attend l'utilisateur est toujours déplié**, quelle que soit l'altitude : sa carte et sa question se
voient sans zoomer.

## La boussole, côté par côté

### Au centre : la carte de l'agent

- En-tête : point d'état, nom, état en mots.
- Une ligne : modèle, effort, durée, jetons, coût (les données de la carte de la barre latérale).
- Une ligne de ce qu'il fait, et sa ligne d'avancement (`progressLine`) s'il en a une.
- En bas, la **barre de saisie** : un champ d'une ligne, un bouton Paramètres, un bouton Dicter. Paramètres ouvre un
  menu : modèle, effort, mode (`api.setAgentOptions`), « Joindre un fichier ». Dicter est le `MicButton` de la
  conversation, branché sur ce champ.
- `Entrée` envoie **par le même chemin que la conversation** : ce que `Composer.svelte` décide aujourd'hui avant
  `sendDraft` (le texte répond-il à la question en attente, le mode retenu, la boîte d'envoi, le brouillon rendu en
  cas de refus) est sorti dans une fonction partagée, appelée par les deux (`lib/send.svelte.ts`).
- Le brouillon est celui de l'agent (`getDraft`, `setDraft`) : ce qu'on tape ici se retrouve dans sa conversation en
  quittant la toile, et l'inverse. Des fichiers déjà joints au brouillon sont dits par une pastille « n fichiers » à
  côté du champ.
- Contour vert quand il tourne, jaune avec un halo quand il attend, rouge en erreur.

**La question.** Quand l'agent a des demandes en attente (`requests`), sa carte grandit vers le bas et montre, entre
ses lignes et la barre de saisie, l'encart jaune de la conversation : `QuestionCard` et `PermissionCard` montés tels
quels, avec plusieurs questions, les réponses multiples, les descriptions, « Envoyer ». Ces deux composants prennent
un élément de conversation, pas `requests` : la toile charge la conversation d'un agent tant qu'il attend (voir « D'où
viennent les données »). La question d'un sous-agent (`sub`) s'affiche sur la carte du sous-agent, qui grandit comme
celle d'un agent ; la carte de l'agent prend un contour jaune et dit « question de <sous-agent> ». La carte ne dépasse pas 70 %
de la hauteur de la fenêtre (à 100 % de zoom) ; au-delà, l'encart défile dans la carte. Les objets voisins se
décalent ; rien ne se chevauche.

### Au nord : le plan

Une bande horizontale au-dessus de l'agent, reliée par un trait : la pastille « Plan », son titre, « 3/7 tâches ·
43 % », la jauge en segments de `PlanBanner`, puis une tuile par tâche (numéro, titre, ce qu'elle fait, sa jauge). La
tâche en cours est cerclée de vert, les tâches faites sont estompées. Au-delà de 9 tâches, la bande montre les faites
repliées en « n faites », la ou les tâches en cours et les suivantes. Sans plan, pas de bande. Sa hauteur est celle
du bandeau mesuré, liste comprise à la hauteur que le lecteur lui a donnée dans la conversation ; la liste y défile à
la molette.

### À l'ouest : l'outillage

Les skills et les plugins que l'agent et ses sous-agents ont utilisés, dessinés au-dessus de sa carte (sous le plan),
deux par rangée autour d'une allée où passe le trait du plan, dans l'ordre de leur premier appel. Les huit derniers
appelés sont montrés, les autres dans une pile « + n outils » qu'un clic déplie. En cartes plus carrées (coins de 3 px, liseré de couleur en haut)
pour dire « de la doc, pas un agent » :

- **skill** : icône de parchemin, violet. Un skill est un appel de l'outil `Skill` ; son nom est `input.skill`.
- **plugin** : icône de boîte, bleu. Un plugin est un serveur d'outils : le `<serveur>` d'un outil
  `mcp__<serveur>__<outil>`, avec son nombre d'appels.

Aucun trait permanent. Pendant qu'un appel est en cours (`status: 'running'`), la carte de l'outil a un anneau vert et
un flux part d'elle vers l'agent ; à la fin de l'appel, le trait disparaît.

### À l'est : l'équipe

Les sous-agents et les tâches de fond de l'agent (`sidebarSubs`, `sidebarBg`), reliés à lui.

- Un sous-agent a la même carte que l'agent, en plus petit : nom (son type), pastille « tâche n » quand on la connaît
  (`planTask`), modèle, durée, jetons, ce qu'il fait (`doing`). Tant qu'il tourne, sa carte a un champ et Dicter, sans
  Paramètres (`api.messageSubagent`).
- Un sous-agent lancé par un sous-agent (`parent`) est posé à droite de son parent et relié à lui.
- Une tâche de fond est une carte au contour pointillé : son genre, sa commande, sa durée. Celle d'un sous-agent est
  posée à sa droite.
- En cours : trait vert animé, avec un point qui circule de l'agent vers lui. Terminé : trait gris fixe. Échec : rouge.
- Les sous-agents terminés sont repliés en une ligne « + n terminés · n échecs », qu'un clic déplie. Un échec non vu
  reste visible hors de la pile.

### Au sud : la zone d'atterrissage

Une zone matérialisée sous l'agent : contour pointillé, fond hachuré, repères d'angle, titre « Zone d'atterrissage ·
n posés, n en approche ». Elle est toujours dessinée ; vide, elle dit « Rien n'a encore atterri ». Un seul trait y
descend depuis l'agent.

Un livrable **en approche** (pas fini) plane au-dessus de son emplacement, dessiné en pointillé ; fini, il **se pose**.

Ce qui s'y pose :

- **L'artefact** : une petite fenêtre de navigateur (trois points, l'adresse), le titre de la page, « Artefact ·
  version n · il y a x min », et « Ouvrir ↗ » qui l'ouvre dans le navigateur. Une carte par adresse ; une nouvelle
  publication à la même adresse met la carte à jour. Source : les appels de l'outil `Artifact` et `artifactLink`
  (`lib/tools.ts`). Le titre est celui que l'appel donne (`title`), sinon le nom du fichier publié ; « version n »
  est le nombre de publications vues à cette adresse. Tant que l'appel tourne, l'adresse n'est pas connue :
  l'artefact est en approche, sous le nom de son fichier.
- **La famille « ruban »** : CI distante, déploiement (la carte de tests a le même dessin, sous la carte qui les lance :
  voir « D'où viennent les données »). Même carte, coins de 6 px, un ruban animé en tête :
  rayures en biais vertes pour une CI, tirets bleus pour un déploiement. En échec, le ruban se fige et passe au rouge.
  - en-tête : le genre, son numéro s'il en a un, l'état et la durée ;
  - une ligne : la commande (tests), ou la branche (CI distante, déploiement), jamais le commit ;
  - les étapes en pastilles reliées, quand on les connaît ; sinon « n contrôles, n en échec » ;
  - en pied : où elle tourne, et un bouton : « Sortie » (locale, ouvre `RunView` ou `TaskView`) ou « Ouvrir ↗ »
    (l'hébergeur, par l'adresse du contrôle ou du journal du déploiement).

## D'où viennent les données

La fenêtre ne charge les conversations qu'une à la fois, et les relâche après dix minutes
(`lib/conversations.svelte.ts`). Or l'outillage, les artefacts et la CI locale se lisent dans les appels d'outils.
Charger la conversation de chaque agent pour la toile coûterait en mémoire et en appels.

Choix validé : le cœur tient, pour chaque agent, un **résumé de toile** qu'il envoie avec l'agent et met à jour à chaque
appel d'outil : les skills et plugins utilisés (nom, nombre d'appels, appel en cours ou non), pour lui et pour chacun
de ses sous-agents, et ses artefacts (adresse, titre, nombre de publications, heure). Seule exception : la
conversation d'un agent qui attend est chargée par la toile, le temps de sa question, pour monter `QuestionCard` et
`PermissionCard` sans les réécrire.

| Besoin | Aujourd'hui | À faire |
|---|---|---|
| État, activité, coût d'un agent | `Agent`, `cardActivity`, `cardSpend` | rien |
| Plan | `planView` (`lib/plan.ts`) | rien |
| Sous-agents, tâches de fond | `PlanState.agents`, `bgTasks` ; `sidebarSubs` et `sidebarBg` ne servent que l'agent choisi et plafonnent leurs lignes | les lire pour tous les agents, sans plafond d'affichage |
| Agents pilotés | `drivenAgents` | rien |
| Questions, permissions | `QuestionCard`, `PermissionCard`, sur un élément de conversation | charger la conversation de l'agent qui attend |
| Écrire, régler, dicter | `Composer`, `setAgentOptions`, `MicButton` | sortir le chemin d'envoi de `Composer` |
| Skills, plugins, artefacts | dans les appels d'outils des conversations | **cœur** : le résumé de toile |
| CI distante | `BranchCi` (un état, un total, les échecs) ; `PrDetail.checks` pour la PR d'un agent | une carte ; ses étapes sont les contrôles de la PR quand l'agent en a une |
| Tests (CI locale) | les tâches de fond ; les appels de terminal, dans le résumé de toile | les reconnaître (ci-dessous) |
| Déploiement | `Deployment`, `PrDeployment`, `app.environments`, la vue « Environnements » | une carte, rattachée à un agent |

Précisions :

- `PlanState` garde au plus 30 sous-agents et 40 tâches de fond : au plafond, la pile dit « 30+ terminés ».
- **CI distante** : sans PR, la carte montre le résumé de `BranchCi` de la branche de l'agent, « n contrôles, n en
  échec ».
- **Tests (CI locale)** : une commande de tests lancée par l'agent ou un sous-agent, en tâche de fond ou au terminal :
  le réglage `testCommand` du Kanban (ou une commande qui commence par lui), ou une commande qui ressemble à une suite
  de tests, reconnue par une liste de motifs dans une fonction pure testée (`cargo test`, `cargo nextest`, `npm test`,
  `npm run test`, `pnpm test`, `yarn test`, `bun test`, `vitest`, `jest`, `pytest`, `go test`, `make test`, `mvn test`,
  `gradle test`, `dotnet test`, `rspec`, `phpunit`…), aussi dans un script, un lanceur ou un shell. La dernière de
  chacun est une carte collée sous la carte de l'agent ou du sous-agent qui la lance, pas dans la zone d'atterrissage ;
  « Sortie » ouvre `TaskView` ou l'appel dans la conversation. Une commande coupée par un redémarrage dit « arrêté ».
  Les lancements du Kanban (`test:<agent>:…`) sont les serveurs de la recette : ils ne comptent pas. Validé par
  l'utilisateur : mieux vaut une carte de trop qu'une CI manquée.
- **Déploiement** : le cœur les lit déjà. Un déploiement se pose chez un agent quand il vient de sa PR
  (`PrDeployment`) ou que son commit est sur la branche de l'agent (le cœur le dit : `ProjectEnvironments.branches`),
  le plus récent par environnement. Les autres restent dans la vue « Environnements ».
  En approche tant que son état est en cours, posé une fois fini.

## Construction

- Le placement est une fonction pure, `lib/workspace-layout.ts` : de l'état des agents vers des boîtes et des traits,
  pour chaque altitude. Testée seule (vitest), comme `lib/workflow-diagram.ts`.
- Les traits sont un seul `<svg>` sous les cartes ; les cartes sont du HTML positionné, pour garder les champs, le
  focus et les composants existants.
- Composants sous `src/components/workspace/` : la vue, la boussole, la carte d'agent, la bande du plan, la carte
  d'outil, la pile, la carte d'équipe, l'encart des questions, la zone d'atterrissage, la carte d'artefact, la carte à
  ruban, les traits.
- Couleurs et rayons : les variables de `app.css` (`--run`, `--wait`, `--del`, `--info`, `--raise`, `--line-2`…), dans
  les deux thèmes. Une seule couleur à ajouter, le violet des skills.
- Textes par `t()`, en français et en anglais.
- Accessibilité : la toile est une région ; chaque carte (outil, agent, sous-agent, tâche de fond, zone
  d'atterrissage) est atteignable au clavier dans l'ordre nord, ouest, centre, est, sud, et `Entrée` ouvre ce qu'ouvre le
  double-clic ; les états et le trait « a lancé » sont dits en mots et pas seulement en couleur ou en dessin ;
  `prefers-reduced-motion` fige flux, halo et ruban.

## Choix validés

- Le cœur tient un résumé de toile par agent.
- Écrire depuis une carte passe par le chemin d'envoi sorti de `Composer`, partagé avec la conversation.
- La question d'un sous-agent s'affiche sur la carte du sous-agent.
- Une CI locale se reconnaît aussi aux commandes qui ressemblent à des tests.
- La spec reste dans le dépôt, à côté des autres specs.
- Les lancements du Kanban (`test:<agent>:…`) sont les serveurs de la recette, pas des tests : ils ne comptent pas comme
  CI locale. Une CI locale est une commande de tests lancée par l'agent ou un sous-agent.
- Les tests ont leur case à part : une carte de tests collée sous la carte de l'agent ou du sous-agent qui les lance,
  et non dans la zone d'atterrissage.
- Un déploiement se rattache à un agent par sa PR, et aussi par sa branche : le cœur apprend à savoir si le commit d'un
  déploiement est sur la branche d'un agent.
- Écrire à un sous-agent suit la règle actuelle du cœur (un sous-agent en arrière-plan qui a un `taskId`).
- Les cartes à ruban montrent la branche, pas le commit ni la version, qu'aucune donnée de la fenêtre ne porte.
- Le bouton Éditeur ouvre un menu sur tout le bouton.
- La toile reprend le design de l'app, pas celui des maquettes : la carte d'un agent est celle de la barre latérale
  (point d'état, état en minuscules, modèle · durée, température, tokens · coût), le plan est le bandeau actuel (pastille
  « Plan », titre, « n/m tâches · %  », jauge en segments, lignes numérotées 01, 02… avec leur cercle d'état), posé
  au-dessus de l'agent ; surfaces, bordures, rayons et boutons sont ceux de la zone de travail, et le fond de la toile
  prend la couleur de la zone de travail, teintée par la couleur du projet.

- Les agents s'empilent l'un sous l'autre ; la toile de tous les projets met les projets côte à côte, en colonnes.
- À l'ouverture, la toile est à 100 % ; deux altitudes, piles sous 60 %, tout déplié au-delà.

## Mes choix, à corriger

L'utilisateur n'a pas tranché ces points ; j'ai pris ma recommandation.

- La toile montre les agents du projet ouvert (ou de tous les projets, depuis la « Vue d'ensemble »), sans les archivés.
- Les sous-agents terminés sont repliés.
- On peut écrire à un sous-agent tant qu'il tourne.
- La carte d'artefact n'a pas de miniature de la page.
- La caméra n'est pas mémorisée ; le seuil des piles est 60 %.
- Un déploiement se pose chez l'agent dont il déploie la PR ou la branche.

## Laissé à qui construit

- Les dimensions exactes des cartes et les écarts, à partir des maquettes.
- L'algorithme de rangement des cadres et l'évitement des chevauchements quand une carte grandit.
- Le dessin exact des deux icônes (parchemin, boîte) dans le jeu d'icônes de l'app.
- La durée des transitions entre altitudes.
