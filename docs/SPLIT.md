# L'écran partagé : deux volets dans la zone de travail (étude, ESC-392)

Demande relayée par l'utilisateur : partager la zone de travail en deux, verticalement ou horizontalement, pour voir par exemple un agent et un terminal, ou deux agents, côte à côte ; et le faire **en glissant un agent ou un terminal depuis la barre latérale**.

Cette étude lit le code de `main` (8a31f225, 2026-11-08), propose un modèle, trois directions rendues en aperçu (`design/split-view/index.html`) et un découpage en lots. Les choix de l'utilisateur sont à la fin (§ 9), et ce qui a été construit dans le même ticket pour la 2.2 beta, au § 10.

## En bref

- Aujourd'hui la zone de travail montre **une seule chose à la fois**, choisie par une chaîne de `{#if}` dans `App.svelte` sur des drapeaux par projet (`board`, `prs`, `docs`, `editor`, `selectedLaunch`, `selectedTerm`, `ui.selectedAgent`). Chaque `select…`/`open…` de `state.svelte.ts` éteint tous les autres. Tout ce qui « suit ce qui est à l'écran » (notifications vues, dictée, raccourcis, réponses au clavier, tiroir de fichiers) lit `app.agent` ou `app.agentShown`.
- Le **piège de nom** : `ui.layout === 'split'` (Ctrl+Maj+L, « Côte à côte ») existe déjà et veut dire « conversation à gauche, fichiers non commités et leur diff à droite ». Le nouvel écran partagé doit avoir un autre nom dans le code (`panes`) et dans l'interface (« Ouvrir à côté », « Volets »), et la question de leur cohabitation est posée plus bas.
- **Proposition** : deux volets au plus, à côté ou l'un sur l'autre, qui prennent un **agent, un terminal ou un lancement** (pas le Kanban, les PR, les Docs ni l'éditeur dans un premier temps) ; la disposition retenue **par projet** et rendue au redémarrage ; un **volet actif** que suivent la saisie, la voix, les raccourcis et la barre de statut. Direction **B** (des panneaux posés sur le cadre, une boussole de cinq cases pendant le geste), **choisie par l'utilisateur le 2026-11-08**.
- **Taille** : cinq lots, de S à M, environ 8 à 11 jours en tout ; le lot 1 (le modèle des volets, sans geste) livre déjà la fonction par le menu et le clavier.

## 1. L'existant

### Ce que montre la zone de travail

`App.svelte` dessine la fenêtre : barre de titre, barre des projets (`ProjectRail`), barre de statut, et au milieu soit une vue globale (`overview`, `stats`, `plugins`), soit le projet : sa **barre latérale** (`Sidebar`) et son **panneau de travail** (`.work`). Dans `.work`, par ordre de priorité :

| Contenu | Composant | Piloté par | Retenu |
|---|---|---|---|
| Kanban | `Board` | `app.board[projectId]` → `app.boardOn` | non (la session) |
| Pull requests | `PrsView` | `app.prs[projectId]` → `app.prsOn` | non |
| Docs (spec, décisions, documents, discussions) | `DocsView` | `app.docs[projectId]` → `app.docsOn` | non |
| Éditeur | `EditorView` | `app.editor[projectId].on` → `app.editorOn` | oui (ses onglets) |
| Journal d'un lancement (ou d'une étape de recette) | `RunView` | `app.selectedLaunch[projectId]` → `app.runCommand` | non |
| Terminal | `TerminalView` | `app.selectedTerm[projectId]` → `app.term` | non |
| Agent : sa conversation | `Conversation` | `app.ui.selectedAgent[projectId]` → `app.agent` | oui (`ui`, `setUi`) |
| … ou un de ses sous-agents | `SubagentView` | `app.subView[agentId]` | la session |
| … ou une de ses tâches de fond | `TaskView` | `app.taskView[agentId]` | la session |
| … ou la vue du harness (son interface plein écran dans un PTY) | `AgentTerminal`, dans `Conversation` | `inHarnessView(agent)` | oui (par agent) |
| … ou la page du lancement de test | dans `Conversation` (`pageView`) | par agent | oui |
| Aucun agent | `.noagent` | — | — |

À côté de la conversation, **le tiroir** `SidePanel` (fichiers non commités, historique, workflow, PR et CI) s'ouvre par-dessus (`app.filesOpen`, Ctrl+Maj+B) ou se **pose à côté** quand `ui.layout === 'split'` (Ctrl+Maj+L) : c'est le seul « deux choses à la fois » de la zone de travail aujourd'hui.

**Fenêtres à part** : la fenêtre « Pages » d'un projet (`PagesWindow`, ESC-311) tient les pages détachées des lancements de test. Il n'y a pas d'autre fenêtre de travail ; les modales (`app.modal`) passent au-dessus de tout.

### Ce qui suppose un seul contenu

1. **La sélection s'exclut elle-même.** `selectAgent` met `selectedTerm`, `selectedLaunch`, `board`, `docs`, `prs` à faux ; `selectTerm` et `selectLaunch` éteignent le reste et ferment l'éditeur ; `openBoard`, `openDocs`, `openPrs` aussi. Il n'existe pas de notion de « ce qui est montré » : c'est le premier drapeau vrai dans l'ordre de `App.svelte`.
2. **`app.agentShown`** (« l'agent à l'écran ») est un seul agent ; `markSeen()` et `onScreen(id)` décident sur lui qu'une notification est vue. Avec deux agents montrés, celui de droite clignoterait alors qu'on le regarde.
3. **La barre latérale surligne un seul élément** (« Another view fills the main area: no agent is highlighted », `Sidebar.svelte`).
4. **Le champ de saisie** reçoit le focus par un compteur global `app.focusComposer` : tout composer monté le prend.
5. **Les réponses au clavier** (`answer-keys.svelte.ts`) écoutent `keydown` sur la fenêtre en phase de capture dès qu'une carte de question attend : deux conversations ouvertes avec chacune une question se disputeraient Ctrl+Entrée et Alt+chiffre.
6. **La dictée** vise « le champ qui a le focus, sinon le dernier montré » (`dictation.focused()`, sur une liste de cibles enregistrées) : avec deux composers, « le dernier montré » est arbitraire.
7. **Les raccourcis** : Ctrl+Tab passe à l'agent suivant à partir de `app.agent` ; Ctrl+S, Ctrl+Maj+F lisent `app.editorOn`.
8. **L'éditeur** suit l'agent choisi (`selectAgent` change `ed.source`).
9. **Les terminaux** : une instance xterm par terminal, gardée hors des composants (`lib/terminals.ts`) et **parquée** dans un `div` hors écran de 900 × 500 quand elle n'est pas montrée ; `mountTerminal` la déplace dans son conteneur et refait `fit()`. Un terminal ne peut être que dans **un** conteneur à la fois — ce qui convient : le même terminal dans deux volets n'a pas de sens.

## 2. Le modèle

### Ce qui va dans un volet

Un **volet** montre un *contenu* : `{ kind: 'agent', id } | { kind: 'term', id } | { kind: 'launch', id }`, et plus tard `{ kind: 'board' } | { kind: 'prs' } | { kind: 'docs' } | { kind: 'file', path }`.

- **Agents et terminaux** : la demande. Un agent dans un volet garde tout ce qu'il montre seul (sous-agent, tâche de fond, vue du harness, page du lancement, bandeau du plan).
- **Le journal d'un lancement** : c'est un terminal en lecture seule, même code (`RunView`, `launchLog`), même coût ; le laisser dehors serait une exception à expliquer. Proposé dans le lot 1.
- **Le Kanban, les PR, les Docs** : utiles (un agent à côté de son ticket), mais ce sont des vues larges, pensées pour toute la zone ; leurs états (`board`, `docs`, `prs`) sont par projet et non par volet. **Plus tard**, derrière une question à l'utilisateur.
- **Un fichier de l'éditeur** : l'éditeur a déjà ses onglets et son arbre ; le mettre dans un volet, c'est le couper en deux. Plus naturel : « l'éditeur » entier comme contenu, plus tard.
- **Le tiroir de fichiers** (`SidePanel`) reste ce qu'il est : il appartient au **volet actif** quand il est un agent.

### Combien de volets, dans quel sens

- **Deux**, côte à côte (séparation verticale) ou l'un sur l'autre (horizontale). C'est la demande, et c'est ce que la largeur permet : la fenêtre a un minimum de 1000 px (`.root { min-width: 1000px }`), dont 52 px de barre des projets et 276 px de barre latérale ; il reste ~660 px de travail, soit deux volets de 330 px — à peine un composer.
- **Plus de deux** (une grille, ou trois colonnes) n'est utile que sur grand écran et multiplie les cas (poignées en croix, focus, retenue). Le modèle se garde la porte ouverte (un arbre de séparations, comme VS Code), mais l'interface du premier lot s'arrête à deux. La direction C (onglets) répond autrement au besoin de « plus » : plusieurs contenus par volet.

### La poignée

Le composant `Splitter` existe (barre latérale, tiroir, éditeur) : il rend une largeur ou une hauteur, se règle au clavier (flèches, `step`), revient à la valeur par défaut au double-clic (`reset`). La poignée des volets en est une instance ; la part est un **ratio** (0,5 par défaut), borné pour que chaque volet garde 320 px de large ou 200 px de haut.

### Ce qui est retenu

- **Par projet**, dans `ui` (déjà écrit par `setUi`, relu au démarrage) : `ui.panes[projectId] = { dir: 'row' | 'col', ratio, items: [Contenu, Contenu], active: 0 | 1 }`. Changer de projet et revenir rend la disposition, comme `selectedAgent` aujourd'hui.
- **Au redémarrage** : un agent revient ; un **terminal**, non (ses process sont tués à la fermeture) : son volet se ferme et l'autre prend toute la place. Un lancement revient s'il tourne encore (il ne tourne plus après un redémarrage) : même règle que le terminal.
- **Par fenêtre** : il n'y a qu'une fenêtre de travail ; rien à décider tant que ça ne change pas.

### Quand le contenu d'un volet disparaît

- **Agent archivé ou supprimé, terminal fermé ou dont le process est sorti** : le volet se ferme, l'autre s'étend. Pour un terminal sorti (`exitedTerms`), le volet garde le terminal avec son code de sortie tant qu'on ne le ferme pas — c'est ce que fait la vue seule aujourd'hui.
- **Un agent qui passe dans un autre projet** n'existe pas. **Un projet fermé** : sa disposition est oubliée.
- **Le même contenu glissé dans l'autre volet** : il y **passe** (les deux volets échangent), on ne le montre jamais deux fois.

### Ce que devient « ouvrir » quelque chose

Choisir un agent ou un terminal dans la barre latérale (clic) **remplace le contenu du volet actif**. Ouvrir le Kanban, les PR, les Docs ou l'éditeur **prend toute la zone** comme aujourd'hui, sans défaire la disposition : la refermer (ou revenir à un agent) rend les deux volets. C'est ce qui garde les vues larges hors du modèle sans les casser.

## 3. Le glisser-déposer depuis la barre latérale

### Ce qu'on glisse

Une carte d'agent, une ligne de terminal, une ligne de lancement. Le geste part après 4 px de déplacement (un clic reste un clic). L'app glisse déjà en **DnD HTML5** (les tuiles de `ProjectRail`, les documents de `DocumentsSpace`), ce que permet `"dragDropEnabled": false` de `tauri.conf.json` (Tauri ne capte pas le dépôt, la page le reçoit). Deux voies :

- **DnD HTML5**, comme les tuiles : le moins de code, le dépôt de fichiers sur le composer coexiste (il lit `dataTransfer.files`, on lira un type à nous, `application/x-escouade-pane`). Mais l'image qui suit le pointeur est une capture figée de l'élément, sans contrôle ; Échap annule (natif). Sous WebKitGTK/Wayland, l'image de glisser manque parfois : à essayer.
- **Geste de pointeur** (`pointerdown` + `setPointerCapture`), comme le `Splitter` : la carte qui suit et les zones se dessinent comme on veut, sur les trois moteurs pareil ; Échap est à écouter soi-même.

Proposé : le **geste de pointeur**, pour que l'aperçu choisi se construise tel qu'il est rendu et se comporte pareil sous WebView2, WKWebView et WebKitGTK.

### Les zones de dépôt

Sur le volet survolé, cinq zones : **gauche, droite, haut, bas** (ouvrir à côté dans ce sens) et **milieu** (remplacer). Avec deux volets déjà ouverts, les bords **remplacent** le volet qu'ils touchent (on ne passe pas à trois). Les directions diffèrent sur la façon de les montrer :

- **A** : la moitié visée se remplit, « Ouvrir à droite · refacto-auth » écrit dedans ; les bords se marquent d'un trait.
- **B** : une boussole de cinq cases au centre du volet survolé ; la case sous le pointeur s'allume et l'emplacement se dessine (« Git Bash · à droite »). Les cases sont grandes (46 px) : on vise sans précision.
- **C** : lâcher sur la barre d'onglets ajoute un onglet (un trait marque la place) ; sur le bord d'un volet, un nouveau volet.

### Pendant le geste

La carte glissée reste à sa place, en pointillés et estompée ; une copie compacte (nom, modèle ou dossier) suit le pointeur ; un bandeau dit les règles (« Bord : à côté · Milieu : remplacer · Échap : annuler »). Lâcher hors de la zone de travail, ou Échap, **annule** sans rien changer. Le volet créé prend le focus.

### Sans glisser : le clavier et le menu

Le glisser ne doit pas être la seule voie (accessibilité, pavé tactile, habitude) :

- **Menu contextuel** d'une carte d'agent, d'un terminal, d'un lancement : « Ouvrir à côté » (à droite), « Ouvrir en dessous ». Le menu existe déjà (`agentMenu` dans `Sidebar.svelte`).
- **Clic avec modificateur** : Ctrl+clic (⌘+clic) sur une carte = ouvrir à côté ; comme un lien dans un nouvel onglet.
- **Raccourcis** (libres aujourd'hui dans `shortcuts.ts`) :
  - **Ctrl+\\** : partager / revenir à un seul volet (le contenu actif reste) ;
  - **Ctrl+Alt+←/→/↑/↓** : passer le focus au volet dans ce sens ;
  - **Ctrl+Maj+\\** : changer le sens (côte à côte ↔ l'un sur l'autre) ;
  - **Ctrl+Maj+W** : fermer le volet actif (pas Ctrl+W, qui efface un mot dans bash et zsh) ; le × de son en-tête aussi.
  - Dans un terminal, Ctrl+\ envoie SIGQUIT sous Unix : le raccourci doit passer par `isAppShortcut` (`lib/terminals.ts`), comme les autres raccourcis de l'app, et l'utilisateur doit pouvoir le changer s'il s'en sert.
- **La boussole de B se pilote au clavier** : sur une carte qui a le focus, Ctrl+Entrée ouvre la boussole, les flèches choisissent la case, Entrée pose. C'est l'argument d'accessibilité de B.
- **La palette** (Ctrl+K cherche dans les conversations) n'a pas d'actions aujourd'hui ; ne pas l'inventer pour ça.

## 4. Ce qui suit le volet actif

Un seul volet est **actif** : celui qui a reçu le dernier clic ou focus. On le voit (trait, anneau ou onglet selon la direction) et la barre de statut le nomme (« Volet actif : tests-e2e »).

| Ce qui suit | Aujourd'hui | Avec deux volets |
|---|---|---|
| Saisie, `focusComposer` | tout composer monté | le composer du volet actif seulement (le compteur devient `{ paneIndex, n }`) |
| Dictée (raccourci) | champ qui a le focus, sinon le dernier montré | inchangé si un champ a le focus ; sinon **le composer du volet actif** (la cible porte son volet) |
| Conversation à la voix | lancée sur un agent (`conversation.start(agentId)`) | l'agent du volet actif ; elle **reste attachée** à son agent si le focus change de volet (l'overlay dit son nom) |
| Réponses au clavier (Ctrl+Entrée, Alt+chiffre, 1–9 de la liste) | la carte en attente à l'écran | la carte du **volet actif** ; l'autre carte se répond à la souris ou en activant son volet |
| Ctrl+Tab (agent suivant) | sélectionne l'agent suivant | change l'agent du volet actif |
| Ctrl+F / Ctrl+Maj+F | recherche du terminal (`TerminalView`), de l'éditeur | celle du volet qui a le focus (déjà le cas : les écouteurs sont sur leur vue) |
| Notifications, « vu » | `agentShown` seul, fenêtre au premier plan | **tout agent montré dans un volet** est vu ; `agentShown` devient `agentsShown` |
| Barre latérale | un élément surligné | les deux, chacun avec la marque de sa place (« gauche », « droite », « haut », « bas ») ; l'actif plus appuyé |
| Barre de statut | l'agent choisi (branche, contexte) | l'agent du volet actif |
| Tiroir de fichiers, Ctrl+Maj+B | l'agent choisi | l'agent du volet actif ; il s'ouvre dans ce volet |
| Éditeur (`ed.source`) | suit l'agent choisi | suit l'agent du volet actif |
| `app.agent` (lu partout) | l'agent choisi | **l'agent du volet actif** : c'est ce qui garde le reste du code tel quel |

Le dernier point est la clé du coût : en gardant `app.agent` = « l'agent du volet actif », les dizaines de lecteurs de `app.agent` continuent de marcher, et seuls les endroits listés plus haut (notifications, surlignage, focus, réponses au clavier) apprennent qu'il peut y en avoir deux.

## 5. La technique

- **Deux xterm visibles.** Chacun a son instance, son `FitAddon` et son `ResizeObserver` (`TerminalView`, `AgentTerminal`) ; la poignée fait des `fit()` en rafale : les grouper par `requestAnimationFrame` et ne prévenir le PTY (`term_resize`) qu'à la fin du geste (ou toutes les 100 ms), sinon un programme plein écran (vim, la vue Claude Code) se redessine à chaque pixel. Chaque xterm prend un **contexte WebGL** (`WebglAddon`) : les navigateurs en limitent le nombre (~16 dans Chromium, moins dans WebKitGTK) ; deux visibles ne posent pas de problème, mais les terminaux **parqués** en gardent un aussi — à vérifier avec beaucoup de terminaux ouverts, et `onContextLoss` repasse déjà au rendu DOM.
- **Les vues « terminal » des harness** (`AgentTerminal`, ESC-105) : leur taille de départ compte (le PTY partait à 127×25 et se redessinait mal) ; `roomOf` mesure le conteneur avant de créer le PTY : il faut qu'il mesure le **volet**, pas la zone entière.
- **Deux conversations qui défilent.** `Conversation` dessine par tranches et retient la position de lecture par agent : rien n'est global, deux instances cohabitent. Le `{#key app.agent.id}` de `App.svelte` devient un `{#key pane.item.id}` par volet. La largeur d'un volet de conversation descend à ~400 px : l'en-tête d'un agent replie déjà dans son menu « … » ce qui ne tient plus (ESC-166, `HeaderFolded`) ; le composer devra replier modèle et effort de la même façon (les aperçus le montrent).
- **WebView2 (Windows)** : rien de propre au partage ; le DnD HTML5 y marche (`dragDropEnabled: false`), le geste de pointeur aussi (§ 3).
- **WebKitGTK (Linux)** : fragile (mémoire du projet : le plantage de `text-wrap: pretty` dans WebKitGTK 2.50, ESC-290). Deux conversations doublent la mise en page de texte : **ne pas réintroduire `text-wrap: pretty`/`balance`** dans les en-têtes des volets, et essayer le geste sous Wayland (WSLg) avant de livrer. Le pointeur capturé (`setPointerCapture`) y marche, c'est déjà celui du `Splitter`.
- **La fenêtre étroite.** Sous ~1200 px de travail, deux volets côte à côte deviennent inutilisables : proposer l'un sur l'autre par défaut, et en dessous de 1000 px (le minimum de la fenêtre) ne montrer que le volet actif avec une pastille « 2 volets ». Le zoom de l'interface (`watchZoom`) réduit la place de la même façon.
- **Performance** : deux conversations qui reçoivent un flux en même temps, c'est ce que fait déjà « Vue d'ensemble » à plus grande échelle ; pas d'inquiétude.

## 6. Le reste de l'app

- **Le contrôle à distance et le téléphone** (`docs/REMOTE.md`) : le remote control de claude.ai suit **une conversation**, pas l'écran ; rien ne change. Le téléphone (option A, l'interface web servie dans le tunnel, encore à construire) a une mise en page mobile qui ne montre qu'un contenu : **un écran partagé n'y existe pas**, le volet actif y est montré seul. La disposition étant dans `ui` (par machine), un téléphone qui la lit doit l'ignorer.
- **Vue d'ensemble, Stats, Plugins** prennent la zone entière comme aujourd'hui.
- **Les plugins qui ajoutent des vues** (`PluginsView`, vues des harness décrites par plugin, PLG-*) : leurs vues sont des pages de la vue Plugins ou des vues d'agent ; une vue d'agent vit dans la conversation et suit donc l'agent dans son volet. Aucun contrat de plugin ne change ; un futur « contenu de volet » fourni par un plugin est hors de cette étude.
- **La fenêtre « Pages »** reste à part ; on pourrait plus tard y préférer un volet, hors sujet ici.
- **Le « Côte à côte » existant** (`ui.layout = 'split'`, conversation + fichiers) : il devient un **réglage du volet** d'agent (son tiroir posé à côté) ; dans un écran partagé, le tiroir posé prendrait trop de place : il s'ouvre par-dessus. À trancher par l'utilisateur s'il veut garder Ctrl+Maj+L tel quel.

## 7. Les trois directions

Rendues dans `design/split-view/` (page de choix `index.html`, PNG `a-*`, `b-*`, `c-*`, source `apercu.html`, capture `render.mjs`), dans la langue de la 2.0 et des réglages 2.2 (direction A) : panneau en relief, filets, couleur du projet pour l'actif.

| | A — un panneau, un filet | B — des panneaux posés sur le cadre | C — des groupes d'onglets |
|---|---|---|---|
| Séparation | filet d'1 px et sa poignée | 8 px de cadre, chaque volet son panneau | filet, une barre d'onglets par volet |
| Volet actif | trait de couleur sur l'en-tête, composer allumé | anneau de la couleur du projet | onglet actif marqué, trait de couleur |
| Geste | la moitié visée se remplit | boussole de cinq cases, emplacement dessiné | trait d'insertion dans les onglets, ou bord |
| Clavier | menu et raccourcis | idem, et la boussole au clavier | idem, et les onglets (Ctrl+PgUp/PgDn) |
| Coût | le plus bas | bas (un composant de plus) | le plus haut (état d'onglets, retenue, menu) |

**Choisie : B** (l'utilisateur, 2026-11-08, sur les aperçus). Le volet actif est ce qui décide où vont la saisie, la voix et les réponses au clavier : il doit se voir au premier coup d'œil, et l'anneau de B le dit mieux que le trait de A. La boussole rend le geste précis et a un équivalent clavier naturel. C est plus puissante mais double la barre latérale, qui est déjà la liste de ce qu'on peut ouvrir.

## 8. Découpage en lots

| Lot | Livre seul | Dépend de | Taille |
|---|---|---|---|
| **S1. Le modèle des volets** | `ui.panes` par projet (retenu, rendu au démarrage) ; `App.svelte` rend un ou deux volets avec la poignée (`Splitter`) ; `app.agent` = agent du volet actif ; « Ouvrir à côté » / « Ouvrir en dessous » au menu contextuel et Ctrl+clic ; Ctrl+\\, Ctrl+Alt+flèches ; volet qui se ferme quand son contenu part ; tests unitaires du modèle et e2e d'un agent + terminal. | — | M (3–4 j) |
| **S2. Ce qui suit le volet actif** | `agentsShown` et « vu » pour les deux ; surlignage et marques de place dans la barre latérale ; focus du composer, dictée, conversation à la voix, réponses au clavier, tiroir, éditeur, barre de statut sur le volet actif ; e2e de deux agents avec chacun une question. | S1 | M (2–3 j) |
| **S3. Le geste** | glisser depuis la barre latérale (pointeur, seuil, carte qui suit, zones de la direction choisie, Échap), boussole au clavier si B ; e2e du geste. | S1 | S–M (1,5–2 j) |
| **S4. Terminaux et vues de harness à deux** | `fit()` groupés, `term_resize` à la fin du geste, mesure du volet pour `AgentTerminal`, passage de WebGL sous charge ; essai Windows, macOS, Linux (WSLg). | S1 | S (1 j) |
| **S5. La fenêtre étroite et le reste** | repli sous la largeur utile, cohabitation avec « Côte à côte » (Ctrl+Maj+L), page de spec à jour, captures du site et docs `/docs`. | S1–S4 | S (1 j) |

Ensuite, si l'utilisateur le veut : **S6** le Kanban, les PR, les Docs ou l'éditeur dans un volet (M), **S7** plus de deux volets (M–L), **S8** des onglets par volet (direction C, L).

## 9. Les choix de l'utilisateur (2026-11-08)

1. **La direction** : **B**.
2. **Le nombre de volets** : **plus de deux**. Construit en arbre de séparations, quatre au plus, chaque volet gardant 320 px de large ou 200 px de haut.
3. **Ce qui va dans un volet** : agents, terminaux et lancements, **et l'éditeur** (ajouté par l'utilisateur). Le Kanban, les PR et les Docs prennent toute la zone.
4. **Ce qui est retenu** : la disposition **par projet**, rendue au redémarrage, sans ce qui n'existe plus (les terminaux).

Par défaut, faute d'autre réponse : le « Côte à côte » existant (conversation + fichiers, Ctrl+Maj+L) reste tel quel hors écran partagé ; dans un volet, ses boutons disparaissent et le tiroir s'ouvre par-dessus.

## 10. Ce qui est construit (ESC-392, pour la 2.2 beta)

Les lots S1 à S5 en un, avec ces écarts au découpage :

- **Le modèle** : `src/lib/panes.ts` (fonctions pures : placer, fermer, élaguer, ratio, tourner, voisin, zones, relecture), `ui.panes` par projet (champ `panes` du `UiState` Rust, gardé tel quel). `app.panes` est l'arbre élagué des contenus disparus ; `app.panesOn` dit qu'il est à l'écran (pas sous le Kanban, les PR, les Docs).
- **La sélection suit le volet actif** : `selectAgent`, `selectTerm`, `selectLaunch` et `openEditor` remplacent le contenu du volet actif (`paneShow`) ; `activatePane` sélectionne le contenu d'un volet. `app.agent`, `app.term`, `app.runCommand` restent donc ceux du volet actif.
- **L'interface** : `components/panes/PaneGrid.svelte` (l'arbre, la poignée), `Pane.svelte` (un panneau, l'anneau, le ×), `PaneDrop.svelte` (la carte qui suit, la boussole, l'emplacement) ; `lib/pane-drag.svelte.ts` (le geste de pointeur, seuil de 5 px, clic avalé après un glisser, Échap).
- **Ce qui suit le volet actif** : `lib/pane-context.ts` dit à une vue si son volet est l'actif (focus du composer, d'un terminal à son montage) ; `mountTerminal` ne reparque un terminal que depuis son propre conteneur ; `keysFor` (réponses au clavier), `shownAgents` (vus), la dictée vers `[data-pane-active]`.
- **Raccourcis** : Ctrl+\ (code `Backslash`), Ctrl+Maj+\, Ctrl+Alt+flèches, Ctrl+Maj+W, passés par `isAppShortcut` dans les terminaux ; Ctrl+clic et le menu contextuel.
- **Pas encore** : la boussole pilotée au clavier depuis une carte qui a le focus (le menu et Ctrl+clic en tiennent lieu), le menu « Ouvrir à côté » sur une ligne de lancement (le glisser et Ctrl+clic y sont), et le repli à un seul volet sous la largeur utile (les cases barrées l'empêchent d'arriver par le geste ; une fenêtre réduite après coup garde ses volets).
- **Tests** : `src/lib/panes.test.ts` (la disposition) ; `tests/e2e/split-screen.spec.ts` (glisser un agent puis un terminal, changer de volet actif, Échap, fermer un volet, rien qui déborde à 1024 × 728, cases barrées et remplacement).
