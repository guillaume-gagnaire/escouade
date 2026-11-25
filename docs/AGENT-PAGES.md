# Les pages des agents (2.3) : étude et design

ESC-661, sur `release/2.3`, le 2026-11-25. Une étude sans code : elle conçoit, propose des choix et découpe les tickets. Rien n'est construit. Les choix restent à l'utilisateur (§ 13). Les maquettes sont dans [`design/agent-pages/`](../design/agent-pages/README.md).

## 0. En bref

- **Un outil MCP, `show_page`**, ouvert à tous les harness (Claude Code, Codex, OpenCode, Copilot CLI, Cursor CLI, Vibe…) puisqu'il passe par le serveur MCP d'Escouade. Un agent y montre à l'utilisateur une page HTML : un **modèle** d'Escouade rempli de ses données (une revue avec choix, un schéma, une comparaison), son **propre HTML**, ou **la page à soi** que l'utilisateur a réglée une fois. Une page se met à jour par versions. Elle reste sur le poste.
- **Elle s'ouvre dans une ligne de « Navigateurs »**, à côté de la conversation de l'agent (le cas phare). L'agent la **pilote** avec `drive_browser`, l'outil de pilotage des navigateurs d'ESC-663 : il fait défiler, met en évidence, change la largeur. L'utilisateur **reprend la main** quand il veut.
- **Les choix reviennent par `ask_user`.** Une page avec des choix porte une demande de la boîte « À toi », la même que celle d'aujourd'hui. Un clic dans la page **propose** une réponse. La réponse ne part qu'au geste de l'utilisateur dans une **bande d'Escouade, hors de la page**. Ensuite, même circuit, même délai de 10 s, même message à l'agent.
- **La sécurité tient en trois règles.** La page vit dans une webview à part, sous une origine à elle (`escouade-page://`) : CSP stricte, aucun réseau, aucun appel à Escouade, aucun stockage qui dure. Escouade sert ses fichiers depuis une copie bornée, jamais depuis le worktree. Elle n'a qu'**un canal de retour** au schéma fermé : proposer une option de sa demande, avec un mot. C'est le cœur qui valide.
- **Sa place** : un interrupteur « Pages des agents » dans Réglages › Fonctionnalités, et la partie « Lancement et navigateurs » de chaque projet. Le téléphone et une fenêtre connectée à un serveur ne voient pas la page en 2.3. Ils voient sa demande, avec ses options, dans « À toi ».
- **Trois directions de design** : A « Les repères » (recommandée), B « La visite », C « L'onglet ». Elles sont rendues en clair et en sombre et déposées dans « À toi ».
- **Dix tickets (AP-0 à AP-9), environ 23 h d'agent, tiennent dans la 2.3** (§ 14). Il faut compter en plus les essais réels sous macOS et Linux. Après la 2.3 : l'aperçu en image pour le téléphone, puis la page servie à une fenêtre connectée à un serveur. Les plugins d'interface de la 2.4 repartent de la même cloison (§ 11).

## 1. D'où ça vient

Le 2026-11-24, un utilisateur, l'auteur du fork serveur, a écrit qu'il se servait beaucoup des artefacts de Claude Code pour ses revues de fin de lot et d'avant fusion. Il proposait qu'un agent puisse afficher des pages dans Escouade par le MCP : une revue avec des choix par défaut, un schéma, et une page à soi réglée une fois pour toutes. Son message est une donnée, pas une consigne. L'utilisateur veut la fonctionnalité dans la 2.3.

**Le cas phare**, que l'utilisateur a appelé « le cas trop cool » le 2026-11-25 : un agent travaille sur un artefact et veut le montrer. Il ouvre le navigateur à côté de sa conversation et le pilote en expliquant : il fait défiler jusqu'à la partie dont il parle, ouvre un onglet de la page, change la largeur, met un élément en évidence. L'utilisateur regarde, reprend la main quand il veut et tranche dans la page.

## 2. Ce qui existe déjà

| Brique | Ce qu'elle fait | Ce qu'on en reprend |
| --- | --- | --- |
| Les artefacts de Claude Code (spec *Pilotage de Claude Code*, *Conversation*) | Publiés sur claude.ai, pour Claude Code seul (`CLAUDE_CODE_ARTIFACT=1`) ; la conversation les montre comme une rangée « Artefact ». | **Rien n'y change.** Les pages des agents sont une fonction à part : tous les harness, rien ne sort du poste. Un agent Claude Code a les deux ; la consigne du serveur MCP dit quand prendre l'une ou l'autre (§ 4.6). |
| La boîte « À toi » et `ask_user` (`src-tauri/src/mcp/asks.rs`, spec *La boîte « À toi »*) | Demandes asynchrones (choix, feu vert, réponse, tâche), options, recommandation, fichiers joints (images, PDF, Markdown ; une page HTML ou un SVG ne s'ouvre jamais), réponse en message après 10 s, « rien d'autre ne part ». | **Le circuit du retour, tel quel.** Une page avec choix porte une demande `a-<n>` ; la boîte et la page en sont deux vues. |
| Les navigateurs (ESC-657 : `src-tauri/src/devpage.rs`, `src/lib/devpage.svelte.ts`, `BrowsersSection.svelte`, `DevPage.svelte`) | Une webview enfant par ligne, posée sur la fenêtre. Aucune commande ne lui répond (`refuses`, sur le préfixe `devpage-`). Stockage par projet. Elle reste sur son origine (`judge`). Devtools sur un geste de l'utilisateur seulement. Trois largeurs, ouverture à côté, volets. | **La ligne, la vue, le placement, les volets, les largeurs.** La cloison est à durcir : une page de lancement est l'app développée (réseau permis, son origine `http`), une page d'agent n'a droit à rien (§ 8). |
| Le panneau de droite (`SidePanel.svelte`) | Non commités, Historique, PR et CI, à côté de la conversation. | La direction C l'essaie (§ 12). |
| Les parties d'un projet (ESC-658) et Réglages › Fonctionnalités (ESC-550) | Couper une partie dans un projet, une fonctionnalité dans toute l'app. | La place de la fonction (§ 9). |
| L'API distante, le téléphone, le serveur d'équipe | Les commandes `devpage_*` sont « du poste seulement » ; une webview ne se montre pas à distance. | La même règle pour la page ; sa demande, elle, va partout (§ 9). |

## 3. Le modèle : une page, ses versions, sa demande

- **Une page** appartient à l'agent qui l'a montrée : à son projet, et à son ticket s'il en a un. Elle a un numéro à elle, `p-<n>`, jamais redonné (comme `a-<n>`). Elle a aussi un titre, un modèle et ses **versions**.
- **Une version** est une copie figée de ce qu'Escouade sert : le HTML et ses fichiers, ou les données d'un modèle. `show_page` sur une page existante (`page: "p-12"`) en ajoute une. La vue s'ouvre sur la dernière, et « v2 ▾ » remonte aux précédentes. Une page garde ses **10 dernières versions**.
- **Sa demande** : une page qui porte des choix a une demande `a-<n>` de la boîte, créée par le même appel (§ 6). Une nouvelle version garde la même demande, sauf si l'agent en donne une autre. Une demande répondue fige les choix de la page : elle montre la réponse.
- **Où elle est gardée** : `<données>/pages/<projet>/<p-n>/v<k>/`, copiée au moment de l'appel. Elle survit donc au worktree, comme les fichiers d'une demande. Le registre des pages (numéro, auteur, titre, versions, demande, état) va dans l'état durable (`state.json` et ses copies).
- **Sa durée de vie** suit son agent. Elle reste tant que l'agent existe, et 30 jours après qu'il est archivé ou supprimé (le réglage « Garder les fichiers des demandes closes » : 7, 30 ou 90 jours). La ligne de « Navigateurs », elle, vit le temps du lancement d'Escouade, comme un navigateur : elle se rouvre par la carte de la conversation.
- **Bornes**, tenues avant de copier : 20 pages par agent ; HTML de 2 Mo au plus ; 64 fichiers et 10 Mo par version ; données d'un modèle de 256 Ko au plus ; 500 Mo pour toutes les pages du poste, la plus ancienne page close part d'abord. Chaque refus dit quoi faire (« Mets à jour p-12 au lieu d'en ouvrir une autre »).

## 4. L'outil MCP

### 4.1 `show_page` : montrer ou mettre à jour une page

| Entrée | Ce que c'est |
| --- | --- |
| `title` | Une ligne, 120 caractères au plus ; c'est le nom de la ligne de « Navigateurs » et de la carte. |
| `template` | `review` (la revue), `diagram` (le schéma), `compare` (la comparaison d'options), `html` (sa propre page), `custom` (la page à soi de l'utilisateur, § 7.4). |
| `data` | Pour un modèle : son JSON, au schéma du modèle (§ 7), 256 Ko au plus. Refusé avec la liste des champs qui ne vont pas. |
| `html` | Pour `html` : le document. On donne `html` ou `file`, pas les deux. |
| `file` | Pour `html` : un fichier `.html` du dossier de travail de l'agent, avec le chemin vérifié sur le fichier ouvert comme pour `ask_user` (liens qui en sortent refusés). Ses fichiers relatifs (`./app.js`, `img/x.png`) viennent avec lui, **pris dans son dossier et ses sous-dossiers seulement**. |
| `page` | Le numéro d'une page de l'agent, pour en faire une nouvelle version. |
| `ask` | Facultatif : la demande que porte la page. Mêmes champs que `ask_user` en `choice` ou `confirm` (titre, options, `multi`, `recommend`, `unblocks`, `tickets`, `urgency`, `decision`). La page la montre, la boîte aussi. |
| `open` | `beside` (le défaut : à côté de la conversation si l'utilisateur la regarde, § 5) ou `quiet` (la ligne attend, avec son point). |

**Réponse** : `{ page: "p-12", version: 2, ask: "a-31", opened: "beside" | "waiting", bytes }`. Une erreur dit ce qui manque, sans jamais dire le contenu d'un fichier.

### 4.2 `drive_browser` : piloter la page montrée

Le nom vient d'ESC-663 (`docs/AGENT-BROWSERS.md`) : un seul outil pilote toutes les lignes de « Navigateurs », et une page d'agent en est une. Sa cible s'écrit `page: "p-12"` ou `browser: "page:p-12"`, la forme est à trancher en AP-0.

Une action à la fois. Le cœur la traduit en un script **fixe d'Escouade**, paramétré par des valeurs encodées en JSON. **Jamais un script de l'agent.** Ce script tourne dans un **monde isolé** de la page, par la poignée native de la webview (`with_webview`) : `Page.createIsolatedWorld` puis `Runtime.evaluate` du protocole DevTools de WebView2, en process et sans port ; `run_javascript_in_world` de WebKitGTK ; `evaluateJavaScript:inFrame:inContentWorld:` de WKWebView (macOS 11+, à vérifier sur la CI Mac). **Pas de `webview.eval`.** La sonde d'ESC-663 l'a montré sur une vraie webview (WebView2, WebKitGTK 2.52) : `eval` tourne dans le monde de la page, qui peut réécrire `JSON.stringify`, `querySelector` ou `click` et fausser nos scripts ; Tauri n'en rend pas non plus le résultat. Le monde isolé lit juste malgré la réécriture, et son résultat revient par le rappel natif, jamais par une IPC de la page.

| Action | Paramètres | Effet |
| --- | --- | --- |
| `scroll_to` | `target` | Fait défiler jusqu'à l'élément (doux, sauf avec `prefers-reduced-motion`). |
| `highlight` | `target`, `label` (60) | Entoure l'élément d'un contour à la couleur du projet, avec une étiquette « pagination le montre ». Un seul à la fois. Sous Windows, c'est `Overlay.highlightNode` qui le dessine : le navigateur le peint, il est absent du DOM, la page ne le voit pas. Sous WebKit, c'est un élément posé depuis le monde isolé, que la page peut voir (sans en rien tirer). |
| `clear` | — | Retire la mise en évidence. |
| `width` | `full` \| `tablet` \| `mobile` | Les trois largeurs des navigateurs. |
| `tab` | `target` | Clique un onglet *déclaré* de la page (`role="tab"`, ou un modèle). Rien d'autre ne se clique. |
| `version` | `n` | Montre une version. |

- `target` : l'`id` d'un élément (`#crit-3`) ou une clé du modèle (`criteria.2`), 200 caractères au plus. Un sélecteur CSS simple est aussi permis, sans `:has()` ni plus de quatre niveaux. L'élément introuvable est dit à l'agent.
- **Chaque geste laisse une trace** dans la conversation : une rangée numérotée (« ③ Met en évidence le critère 3 ») et un repère sur le rail de la page (direction A, § 12). L'utilisateur sait donc toujours ce que l'agent a fait.
- **Reprendre la main.** Le premier geste de l'utilisateur dans la page (défilement, clic, touche) ou « Reprendre la main » suspend le pilotage. Pendant ce temps, `drive_browser` répond « L'utilisateur a repris la main : il te la rendra par « La rendre », ou écris-lui. » sans rien faire. « La rendre » rouvre le pilotage. Une page qui n'est pas affichée ne se pilote pas (« La page p-12 n'est pas à l'écran »).
- **Bornes** : 4 gestes par seconde, 600 par page et par heure.

### 4.3 Les autres outils

- `get_page` (`page`) : l'état d'une page de l'agent, c'est-à-dire ses versions, si elle est à l'écran, qui a la main, et sa demande avec sa réponse. **Jamais ce que l'utilisateur a fait dedans** (où il a défilé, ce qu'il a tapé).
- `close_page` (`page`) : ferme la ligne. La page reste gardée et sa carte la rouvre.
- Pas de `list_pages` : la réponse de `show_page` et `get_page` suffisent.

### 4.4 La trace dans la conversation

L'appel `show_page` se dessine comme une **carte** et non comme une rangée d'outil. Elle montre l'icône d'une page, le titre, « Page de pagination · modèle Revue · v2 · à côté » et « Afficher ». Si la page porte une demande, son pied dit l'état de la demande : « Attend ton choix · 3 options · a-31 » en ambre, puis « Tu as choisi A · Fusionner dans main ». Chaque `drive_browser` devient une rangée numérotée sous la carte. La forme exacte dépend de la direction choisie (§ 12). Une carte ouvre toujours la **dernière** version ; ses versions se lisent dans la vue.

### 4.5 Qui peut montrer une page

| Qui | Permis | Ce que ça donne |
| --- | --- | --- |
| Un agent (ticket ou non), sur tout harness | Oui | Sa ligne, à côté de sa conversation. |
| L'Overseer | Oui, `html` ou modèle, sans `file` (il n'a pas de worktree) | Dans sa conversation. |
| Une discussion ou un brainstorm des Docs | Oui, sans `file` | Dans la discussion, à côté. |
| Claude hors Escouade (`Caller::External`) | **Seulement avec `ask`** (proposé, P9) | Pas de conversation où l'ouvrir : la page se voit par sa demande dans « À toi » (« Ouvrir la page »). |
| Un agent d'un autre projet | Non | Une page n'existe que dans le projet de son auteur. |

**Projet privé** : ses pages ne sont connues que de ses agents. Une page ne nomme ni ne lit rien d'un autre projet : les données d'un modèle sont prises telles quelles, sans lien qu'Escouade suivrait. Le stockage est séparé par projet, et rien de la page ne part au serveur d'équipe ni dans les statistiques.

### 4.6 La consigne aux agents

Les instructions du serveur MCP ajoutent une phrase, donnée seulement quand la fonctionnalité est allumée : « Pour montrer à l'utilisateur quelque chose qui se lit mieux en page qu'en texte (une revue de fin de lot ou d'avant fusion, un schéma, des options à comparer), ouvre-la par `show_page` et pilote-la par `drive_browser` en l'expliquant. Mets-y tes choix par `ask` plutôt que dans ton texte. N'y mets ni secret ni contenu d'un tiers que tu n'as pas relu. » Un agent Claude Code garde ses artefacts pour ce qui doit être publié sur claude.ai.

## 5. Où elle s'ouvre, ce qui la signale

- **Une ligne de « Navigateurs »**, clé `page:<p-n>`, dans la section de la barre latérale, avec l'icône d'une page au lieu du point de chargement. Elle a son nom, la marque du ticket et une étiquette : « choix » en ambre tant que sa demande attend, « v2 » sinon. Ses gestes sont ceux d'un navigateur : clic, `Ctrl`+clic, glisser dans un volet, menu, croix.
- **À côté de la conversation**, d'après la règle de la page d'un test (ESC-311). Si l'utilisateur regarde l'agent, la page s'ouvre dans un volet à sa droite. Sinon, la ligne attend avec son point et la carte de la conversation l'ouvre. `open: quiet` la laisse toujours attendre. Une page déjà ouverte qui reçoit une version se recharge sur place, sans bouger.
- **Dans la boîte « À toi »** : sa demande y est, comme toute demande. Son panneau y gagne « Ouvrir la page » (la ligne, à côté de l'agent) et une vignette du modèle. On répond dans la boîte ou dans la page : c'est la même demande.
- **Ce qui la signale** : le point et l'étiquette de la ligne, la carte dans la conversation, et la demande dans la boîte avec son compteur et le résumé des notifications. **Aucune notification de plus** pour une page sans choix : elle n'attend rien.
- **Le panneau de droite** ne porte la page que dans la direction C (§ 12).

## 6. Le retour vers l'agent : `ask_user`, sans autre geste

```
agent ──show_page(ask)──▶ cœur : crée p-12 v2 et a-31 (le même dépôt que ask_user)
                              │
             ┌────────────────┴────────────────┐
             ▼                                 ▼
   page (cloisonnée)                    boîte « À toi »
   options dessinées                   options natives
   clic sur A ──POST _/choose──▶ cœur : a-31 ouverte ? A ∈ options ? webview = p-12 ?
                                    │  oui : une PROPOSITION, rien n'est répondu
                                    ▼
             bande d'Escouade sous la page : « Choisi dans la page : A · Fusionner dans main »
             [Annuler] [Envoyer à pagination ⏎]   ◀── geste natif de l'utilisateur
                                    │
                                    ▼
             inbox_answer(a-31, A, mot) : la même commande que la boîte
             10 s pour « Annuler », puis le message à l'agent :
             « [Escouade · ta demande a-31 « Fusionner POR-41 ? »] L'utilisateur a choisi A… »
```

- **Le canal** : une seule adresse de la page, `escouade-page://p-12/_/choose`, en `POST`, avec un corps JSON fermé `{ "options": ["a"], "note": "…" }`. La note fait 2 000 caractères au plus, le corps 4 Ko au plus. Tout autre chemin, méthode ou champ est refusé (404). Le cœur vérifie que **la webview qui appelle est celle de p-12** (l'étiquette passée au gestionnaire du schéma). Il vérifie aussi que la demande liée est ouverte, que ses options contiennent celles proposées et que `multi` est respecté. Une proposition par seconde au plus.
- **Ce qui part** : rien tant que l'utilisateur n'a pas fait le geste dans la bande. Un script de la page peut appeler `_/choose` sans clic, il ne fait qu'allumer la bande. Le libellé que la bande montre est **celui de la demande, lu dans le cœur**, jamais celui que la page affiche. Une page qui écrirait « A : supprimer la branche » à côté d'un bouton dont l'option est « Fusionner » serait démentie sous les yeux de l'utilisateur.
- **Le reste de la boîte vaut tel quel** : 10 s pour annuler, « Revenir sur ma réponse » tant que l'agent ne l'a pas lue, la trace, la délégation à un agent (jamais un feu vert), `decision: true`. La page reçoit la réponse : le cœur exécute un script fixe, dans le monde isolé, qui coche l'option et fige les autres. Une réponse donnée dans la boîte allume la même chose dans la page.
- **Une demande `confirm` avec `gate`** (un feu vert) se montre dans la page mais **ne se propose pas depuis la page** : la bande dit « Un feu vert : réponds-y dans la boîte ou ici », avec ses deux boutons natifs.
- **Ce que la page ne peut pas faire par ce canal** : répondre à une autre demande, en créer une, déléguer, reporter, joindre un fichier, lancer quoi que ce soit, ou lire la réponse d'un autre.

## 7. Les modèles et la page à soi

Un modèle est **une page d'Escouade** (HTML, CSS et script livrés avec l'app, relus comme son code), servie dans la même cloison que n'importe quelle page. L'agent n'y met que des **données**. Escouade les rend en texte, avec le Markdown assaini de la conversation pour les champs `md`. Un agent ne peut donc pas y glisser de script. Les modèles suivent les jetons de l'app (clair et sombre, teint du projet) et sa langue.

### 7.1 `review` : la revue de fin de lot ou d'avant fusion

```json
{
  "subject": { "ticket": "POR-41", "branch": "ticket/por-41", "target": "main", "loop": "2/5" },
  "facts": { "files": 6, "added": 214, "removed": 38, "tests": { "passed": 48, "total": 48 } },
  "criteria": [{ "text": "La liste montre 25 adhérents par page", "state": "met", "md": "Vérifié par `users.spec.ts`." }],
  "changes": [{ "path": "src/lib/pagination.ts", "added": 58, "removed": 0, "diff": "…(facultatif, 200 Ko)" }],
  "risks": [{ "md": "Le tri n'est pas gardé dans l'adresse." }],
  "sections": [{ "id": "perf", "title": "Performances", "md": "…" }]
}
```

Avec `ask`, son verdict : « Fusionner », « Renvoyer avec un mot », « Attendre ». Pour un lot (fin de lot de l'Overseer ou du chef), `subject.tickets` en liste et une section par ticket, chacune avec son ancre pour `drive_browser`. Un diff se dessine avec les couleurs de l'éditeur, replié au-delà de 400 lignes.

### 7.2 `diagram` : le schéma

`{ "kind": "mermaid", "source": "flowchart LR …", "md": "légende" }`. **Mermaid est embarqué** dans le modèle : c'est un fichier de l'app, sans réseau, en `securityLevel: 'strict'` (sans HTML dans les nœuds ni clic). La source fait 64 Ko au plus. Le zoom et le déplacement viennent du modèle. `drive_browser highlight` vise un nœud par son id. Plus tard, d'autres moteurs pourront s'ajouter (D2 compilé en WASM, Graphviz) sans changer le contrat.

### 7.3 `compare` : la comparaison d'options

`{ "options": [{ "id": "a", "title": "…", "md": "…", "images": ["shots/a.png"], "pros": [], "cons": [] }], "criteria": ["…"] }`. Les images viennent du dossier de travail, copiées dans la version, et ce sont des images vérifiées par leur contenu comme pour `ask_user` (un SVG n'en est jamais une). Le modèle reprend les **planches** de la boîte : chaque option sur son fond, sa ligne de crédits, le sceau « Recommandé ». « Sombre · Clair » s'affiche quand une option a deux rendus. Avec `ask`, chaque planche porte son option.

### 7.4 La page à soi (`custom`)

- **Ce que c'est** : un fichier HTML que l'utilisateur choisit une fois. Il est servi dans la même cloison. L'agent le remplit de ses données par `show_page(template: "custom", data)`.
- **Où on la règle** : Réglages › Projets › *(projet)* › Lancement, dans un groupe « Pages des agents » : « Page à soi : [Choisir un fichier…] ». Ce choix est gardé sur le poste. Le dépôt peut aussi la proposer par une clé `pages.custom: design/ma-revue.html` du `.escouade.yml`. La règle des parties s'applique : le poste l'emporte sur le dépôt, l'équipe impose (`enforced`) ou recommande (`defaults`). Une page du dépôt n'a pas besoin d'un accord comme une commande de lancement, puisqu'elle ne fait rien tourner hors de la cloison. Le réglage dit pourtant d'où elle vient (« Proposée par le dépôt »).
- **Ce qu'elle reçoit**, avant ses propres scripts, en lecture seule (`Object.freeze`) : `window.escouade = { page: "p-12", version: 2, template: "custom", title, agent: { name }, ticket: { key, title } | null, project: { name }, theme: "dark" | "light", lang: "fr" | "en", data, ask: { title, options, recommend, multi, answered } | null, choose(options, note) }`. Il n'y a ni chemin, ni jeton, ni rien d'autre. `choose` poste sur `_/choose`. Un événement `escouade:answered` lui dit la réponse.
- **Une page à soi par poste ou par projet**, pas par modèle : un agent qui veut « sa » revue prend `custom`. Plusieurs pages à soi nommées (`pages.custom: { revue: …, lot: … }`) viendraient après la 2.3, si le besoin se montre.
- **Sans page à soi réglée**, `template: "custom"` est refusé : « L'utilisateur n'a pas réglé de page à soi pour ce projet : prends `review`, `diagram`, `compare` ou `html`. »

## 8. La sécurité : le point dur

### 8.1 La menace

Le HTML d'une page est écrit par un agent. Cet agent a pu lire une issue, une page web ou un fichier hostile, et y recopier une consigne ou un script. Une page n'est donc **jamais digne de confiance**, qu'elle vienne d'un modèle (les données) ou d'un `html` libre (tout). La page à soi vient de l'utilisateur ou de son dépôt, mais les données qu'elle reçoit viennent d'un agent. Elle est traitée de la même façon.

Ce qu'on refuse qu'une page obtienne : parler à Escouade (IPC, commandes, plugins), lire un fichier, joindre le réseau (exfiltrer, charger), garder ou lire un état d'une autre page, ouvrir une fenêtre ou une adresse, télécharger, demander une permission (caméra, micro, position, presse-papiers), répondre seule à une demande, se faire passer pour l'interface d'Escouade.

### 8.2 La cloison

1. **Une webview à part**, étiquette `agentpage-<p-n>`, enfant de la fenêtre comme un navigateur. `refuses` refuse aussi ce préfixe : aucune commande d'Escouade ni d'un plugin ne lui répond. La capacité ne nomme que les webviews de la fenêtre, comme aujourd'hui.
2. **Une origine à elle** : le schéma `escouade-page://` enregistré par Escouade (`register_asynchronous_uri_scheme_protocol`), une adresse par page (`escouade-page://p-12/`). Le gestionnaire **ne sert qu'à la webview de la page qu'il nomme**, l'étiquette étant vérifiée par `UriSchemeContext::webview_label`. Sous Windows, WebView2 sert un schéma maison sous `http://escouade-page.localhost/`, la même origine pour toutes les pages. C'est ce contrôle par étiquette qui empêche p-12 de lire p-13.
3. **Rien que la copie** : le gestionnaire sert les fichiers de la version, depuis `<données>/pages/…`, par leur chemin relatif nettoyé (pas de `..`, pas de lien). Il ne sert jamais le worktree ni un chemin absolu. Le type vient du contenu.
4. **La CSP**, en en-tête de chaque réponse et aussi en `<meta>` posé en tête du document servi, au cas où un moteur ignorerait l'en-tête d'un schéma maison :
   `default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; media-src 'self' blob:; connect-src 'self'; frame-src 'none'; child-src 'none'; worker-src 'none'; object-src 'none'; form-action 'none'; base-uri 'none'; manifest-src 'none'`. `connect-src 'self'` n'atteint que le schéma de la page, qui ne sert que ses fichiers et `_/choose`. Pas de `unsafe-eval`, sauf pour le HTML libre si P4 le permet.
5. **Aucune navigation** hors de sa page. `on_navigation` n'accepte que `escouade-page://p-12/…`. Un lien vers le web n'ouvre rien de lui-même : la bande d'Escouade demande « La page veut ouvrir https://… · Ouvrir dans le navigateur du système · Ignorer ». Une navigation **sans** geste (un script qui change `location`) ne se fait pas : c'est ce qui ferme l'exfiltration par l'adresse. `on_new_window` vaut toujours `Deny`.
6. **Aucun stockage qui dure** : la webview est `incognito` (InPrivate de WebView2, magasin non persistant de WKWebView, contexte éphémère de WebKitGTK). Rien ne reste d'une ouverture à l'autre, et rien n'est partagé avec un navigateur, une autre page ou la fenêtre.
7. **Aucune permission** : `on_permission_request` refuse tout (caméra, micro, position, notifications, presse-papiers en lecture, capteurs, MIDI). `on_download` refuse tout téléchargement.
8. **Les fuites que la CSP ne couvre pas** : les `<link rel="dns-prefetch|preconnect|prefetch|prerender">` sont retirés du HTML servi, avec `X-DNS-Prefetch-Control: off`. Un script d'initialisation d'Escouade retire `RTCPeerConnection` (et ses préfixes) avant ceux de la page, et `frame-src 'none'` empêche de le retrouver dans un cadre neuf.
9. **Ce qu'Escouade exécute dans la page tourne dans un monde isolé** (§ 4.2) : la page ne peut ni le fausser ni lire ce qu'il rend.
10. **Des devtools sur un geste de l'utilisateur seulement**, comme pour les navigateurs. Ni la page ni l'agent ne les ouvrent.
11. **Des tailles bornées** (§ 3), tenues avant de copier.
12. **Ce que la page ne peut pas imiter** : la bande de confiance au-dessus (« Écrite par pagination · hors réseau, sans accès à Escouade ni à tes fichiers ») et la bande du choix en dessous sont du DOM de la fenêtre, **hors du rectangle de la webview**. La webview native se dessine dans son rectangle et nulle part ailleurs. Une page peut dessiner un faux bouton « Envoyer », mais rien ne part sans la vraie bande.

### 8.3 Ce qu'une injection de consignes peut et ne peut pas obtenir

| Une page piégée peut | Elle ne peut pas |
| --- | --- |
| Mal présenter les choses à l'utilisateur, comme tout texte d'agent. | Répondre seule : elle propose, l'utilisateur confirme dans la bande d'Escouade, qui affiche le libellé du cœur. |
| Proposer une option de **sa** demande. | Toucher une autre demande, une autre page, un autre projet. |
| Envoyer une note à son agent, en même temps qu'une réponse que l'utilisateur confirme. Cet agent est déjà son auteur : rien de nouveau ne sort. | Joindre le réseau, lire un fichier, appeler Escouade ou un plugin, garder un état, ouvrir une fenêtre ou une adresse sans geste, télécharger, demander une permission. |
| Afficher « colle cette commande dans ton terminal ». C'est le risque qui reste : la bande de confiance le dit, et la consigne aux agents interdit d'y mettre ce qu'ils n'ont pas relu. | Écrire dans le presse-papiers sans geste ; avec un geste, c'est ce que fait n'importe quelle page. |

### 8.4 Par système : ce que la webview des navigateurs garde déjà, et ce qui manque

| Mesure | Windows (WebView2) | macOS (WKWebView) | Linux (WebKitGTK) |
| --- | --- | --- | --- |
| Aucune commande (`refuses` sur l'étiquette) | Déjà là pour `devpage-` : ajouter `agentpage-`. | Idem. | Idem. |
| Schéma à soi, servi à sa seule webview | À faire. Servi en `http://escouade-page.localhost/` (`WebResourceRequested`) : **même origine pour toutes les pages**, d'où la vérification par étiquette. | À faire. `WKURLSchemeHandler`, vraie origine `escouade-page://p-12`. | À faire. `webkit_web_context_register_uri_scheme`, vraie origine. |
| CSP en en-tête + `<meta>` | À faire. En-tête honoré sur `WebResourceRequested`. | À faire. **À vérifier par la sonde** : en-tête d'un schéma maison honoré ; le `<meta>` couvre l'autre cas. | À faire. Idem, à vérifier. |
| Aucune navigation sans geste ; le web par la bande | `judge` existe : variante stricte à faire. | Idem. | Idem. |
| Pas de stockage qui dure (`incognito`) | Manque : les navigateurs ont un dossier par projet. InPrivate par page. | Manque : magasin par projet (macOS 14+). Magasin non persistant. | Manque : dossier par projet. Contexte éphémère. |
| Permissions refusées (`on_permission_request`) | Manque. `PermissionRequested` → refuser. Désactiver aussi le presse-papiers en lecture accordé d'office (`clipboard`). | Manque. Seule la capture média passe par le délégué ; le reste n'est pas offert à une page `escouade-page:`. | Manque. Signal `permission-request` → refuser. |
| Téléchargements refusés (`on_download`) | Manque. | Manque. | Manque. |
| WebRTC hors jeu | Manque. Le script d'initialisation retire `RTCPeerConnection` (WebView2 n'a pas d'interrupteur par webview) ; la sonde le vérifie aussi dans un cadre. | Manque. Même script ; WebRTC actif par défaut depuis macOS 11. | Déjà coupé par défaut (`enable-webrtc` faux) ; le script reste pour la défense en profondeur. |
| Préchargements DNS retirés | Manque : Chromium les fait malgré la CSP. | Manque (WebKit le fait aussi). | Manque. |
| Devtools sur geste seulement | Déjà là (fenêtre à part). | Déjà là (détaché). | Déjà là (attaché). |
| Les bandes d'Escouade hors du rectangle | Déjà vrai (webview enfant posée au-dessus du DOM, dans son rectangle). | Idem. | Idem. |

Chaque ligne « à vérifier » devient un test de `devpage_probe` étendu (`agentpage_probe`), avec une vraie webview en CI sous Linux (Xvfb) et macOS, et un e2e sous Windows. Une page piège essaie tour à tour `fetch` vers le web, `<img>` distant, `location=`, `window.open`, `RTCPeerConnection` dans un cadre, `dns-prefetch`, `localStorage` d'une ouverture à l'autre, `fetch('/p-13/index.html')`, `__TAURI_INTERNALS__`, `_/choose` avec une option étrangère, et une demande de caméra. Tout doit échouer. Les essais réels sur un Mac et sous Linux (pas seulement en CI) restent à faire par l'utilisateur ou sur une machine d'essai (§ 14).

### 8.5 Ce qu'on ne promet pas

- Une page lente ou qui boucle n'est pas tuée : elle ne bloque que sa webview. La fermer reste le geste.
- Le canal par le temps (une page qui mesure ce qu'elle reçoit) n'a rien à mesurer : elle ne reçoit que sa réponse.
- `unsafe-inline` reste pour les scripts : une page libre a besoin de ses scripts. C'est la cloison qui protège, pas l'absence de script (P4).

## 9. La place dans l'app

- **Réglages › Fonctionnalités** : un interrupteur « Pages des agents », allumé par défaut, à côté de « Boîte « À toi » ». Il dépend du serveur MCP : serveur éteint, pas de pages, comme pas de boîte. Coupé, `show_page`, `drive_browser`, `get_page` et `close_page` quittent la liste des outils de tous les harness, la consigne se tait, les lignes se ferment et les cartes disent « Les pages des agents sont coupées (Réglages › Fonctionnalités) ». Rien n'est effacé.
- **La partie d'un projet** : « Lancement et navigateurs » (ESC-658) porte aussi les pages, puisqu'elles vivent dans ses lignes. Coupée dans un projet, ses outils de page y sont refusés avec la phrase des parties. Il faut ajouter les outils à `parts::of_tool`, comme la spec des parties l'annonçait (« s'ils en reçoivent un »). Sa description dit « son lancement, ses navigateurs et les pages de ses agents ».
- **Le téléphone (app mobile)** : il ne voit pas la page, et une webview ne se montre pas à distance. Il voit **sa demande** dans « À toi », avec ses options natives, et une ligne « Une page l'accompagne, à voir sur ton poste ». Après la 2.3, un aperçu en image (PNG rendu par le poste, joint à la demande) pourra s'ajouter (AP-10).
- **Une fenêtre connectée à un serveur** : en 2.3, même chose que le téléphone. La carte dit « Page sur le serveur : elle se verra dans une prochaine version ; sa demande est dans À toi ». Après la 2.3 (AP-11), le serveur servira la version par l'API distante (`page_bundle`, octets bruts, bornés), et la fenêtre la montrera **dans sa propre cloison locale**, avec le canal relayé au serveur.
- **L'API distante** : les commandes `page_*` de la fenêtre sont « du poste seulement » (`desk-only`), comme `devpage_*`. Seules les demandes passent, avec un champ `page` (`{ id, title, template, version }`) dans `inbox_get`.
- **Le serveur d'équipe** : une page ne quitte jamais le poste. Elle n'est ni synchronisée ni copiée dans les flux d'équipe. Sa demande reste une demande locale, comme aujourd'hui.
- **Les statistiques d'usage**, si l'utilisateur les laisse : `page_shown` (modèle, avec ou sans demande, taille par tranche), `page_driven` (action), `page_choice_proposed` puis `ask_answered` (`via: "page"`). Jamais un titre, un texte, un chemin ni un nom.
- **Journaux** : `app.log` ne dit que « page p-12 v2 shown by agent e6e0… (review, ask a-31, 41 KB) », « driven (highlight) », « choice proposed », « closed ». L'activité du serveur MCP garde le projet, le modèle et le numéro.

## 10. Avec ESC-663 : un seul canal, un seul modèle d'outils

ESC-663 étudie en même temps le pilotage des navigateurs par un agent (`docs/AGENT-BROWSERS.md`, sur `ticket/esc-663`). Les deux agents se sont écrit le 2026-11-25 et s'accordent sur ce qui suit ; AP-0 (§ 14) n'a plus qu'à reprendre le nom de la cible et le monde isolé dans les deux docs :

- **Un seul outil de pilotage pour toutes les lignes de « Navigateurs »** : `drive_browser` (le nom d'ESC-663). Il vise une page d'agent (`page: "p-12"`) ou un navigateur (sa clé). Les mêmes actions (`scroll_to`, `highlight`, `clear`, `width`, `tab`), la même règle « reprendre la main / la rendre », la même trace numérotée dans la conversation, les mêmes bornes. Un navigateur de l'app développée peut en avoir de plus (taper, cliquer, lire le DOM pour tester) : ce sont ses gestes à lui, sous ses propres garde-fous.
- **Un seul canal étroit d'une webview cloisonnée vers le cœur** : un `POST` sur le schéma de la page, au schéma fermé, vérifié par l'étiquette de la webview et validé par le cœur. Pour une page d'agent, ce canal ne porte qu'une proposition de choix. Une page de lancement n'en a **aucun** : c'est l'app développée, qui n'a rien à proposer à Escouade.
- **Un seul sens pour piloter** : le cœur vers la page, par des scripts fixes d'Escouade paramétrés en JSON, exécutés dans un **monde isolé** (pas de `webview.eval`, § 4.2), jamais un script d'un agent. Sous Windows, ESC-663 fait passer les clics et la frappe par `Input.*` du protocole DevTools (`isTrusted` vrai) ; sous WebKit, un clic du monde isolé n'est pas « trusted ». Si ESC-663 veut qu'un agent exécute son propre JavaScript dans l'app développée (pour tester), c'est un autre outil, refusé sur une page d'agent.

## 11. Pour la 2.4 : les plugins d'interface

`docs/TODO-2.4.md` prévoit des plugins qui déclarent un panneau, une vue ou une commande. La page d'un agent et le panneau d'un plugin posent le même problème : du HTML qui n'est pas digne de confiance, cloisonné, avec un canal étroit vers l'app. La cloison du § 8 est faite pour servir aux deux.

- **Ce qui sert tel quel** : la webview à part refusée par son étiquette, l'origine par schéma (`escouade-plugin://<id>/`) servie à sa seule webview depuis les fichiers du paquet, la CSP, `incognito`, les permissions et téléchargements refusés, la bande de confiance hors du rectangle, le pilotage par scripts fixes dans un monde isolé, et la ligne de « Navigateurs » ou un volet comme place.
- **Ce qu'il faudrait y ajouter** :
  - un canal **à requêtes et réponses** (un `id` par requête) au lieu d'une seule proposition ;
  - des **abonnements** poussés par le cœur (agents, tickets) ;
  - un **contrôle par permission** du manifeste (lire les agents, ouvrir un ticket…), chaque message vérifié contre ce que le plugin a déclaré et que l'utilisateur a accepté ;
  - un **stockage persistant** propre au plugin (ce qu'une page d'agent n'a pas) ;
  - les **jetons du thème** injectés ;
  - la **place déclarée** dans le manifeste (panneau, vue, barre d'état) ;
  - ce que la revue « Vérifié par Escouade » du store doit lire de plus (le HTML et ses messages).
- **Rien de cela n'entre dans la 2.3.** Le seul engagement est de ne pas fermer la porte : le gestionnaire du schéma et la validation du canal s'écrivent en modules qui prennent une table de messages permis. La 2.3 n'en remplit qu'une ligne, `choose`.

## 12. Le design : trois directions

Avec le skill `impeccable`, en mode Operate : la surface est neuve dans un monde visuel établi (`DESIGN.md`), donc seule la composition est ouverte. Sept structures ont été tirées du cas phare, de la plus attendue à la moins attendue :

1. la page dans une ligne de « Navigateurs », en volet à côté de la conversation, avec la bande du choix dessous ;
2. un onglet du panneau de droite ;
3. le pupitre : la page à gauche, un rail natif à droite (critères, étapes, choix) ;
4. la carte qui se déplie dans la colonne de la conversation ;
5. la visite : la page domine, la conversation se réduit à la narration de l'agent, comme des sous-titres ;
6. les notes en marge, ancrées aux sections de la page ;
7. les repères : chaque geste de l'agent est un repère numéroté, à la fois dans la conversation et sur un rail le long de la page.

Le tirage d'impeccable (`concept-seed --scope surface --mode operate`, clé `32b8d056`) a distribué les n° 7, 5 et 2, et non la première, la plus attendue. Les trois montrent les mêmes deux écrans, avec la même revue fictive (POR-41, agent « pagination ») :

- `#revue` : l'agent ouvre la revue et met en évidence le troisième critère ;
- `#choisi` : l'utilisateur a repris la main et choisi A dans la page, et la bande attend son geste.

Chaque écran est rendu en sombre et en clair, à 1440 × 900 et à 1024 × 728. Les cinq briques communes aux trois directions : la bande de confiance au-dessus de la page, la bande du choix d'Escouade en dessous, « pagination pilote · Reprendre la main », la carte dans la conversation et la ligne de « Navigateurs » avec son étiquette « choix ».

- **A — « Les repères »** (`a-*.png`) : la conversation et la page côte à côte, à parts égales. Chaque geste de l'agent est un **repère numéroté**, une rangée « ③ Met en évidence le critère 3 » dans la conversation et le même ③ sur un rail d'Escouade le long de la page, où se voit aussi la partie à l'écran. Un repère cliqué ramène la page où l'agent l'avait mise. La carte a un pied « Attend ton choix · 3 options · a-31 » et « 3 repères ». Les choix sont dans la page, et la bande les reprend en puces (A · Fusionner, B, C), puis demande le geste. *Pour* : le lien entre ce que dit l'agent et ce qu'il montre se lit d'un coup d'œil, on peut le rejouer, la conversation reste entière. *Contre* : à 1024, chaque moitié est étroite et la page passe en colonne.
- **B — « La visite »** (`b-*.png`) : la page prend les deux tiers. La conversation se réduit à la **narration** : la phrase en cours en grand (« étape 3 sur 4 »), les étapes de la visite avec leur heure, l'ancien texte replié. La barre de la page porte la visite (‹ ● ● ━ ● ›) et la carte a des vignettes et « Revoir la visite ». *Pour* : la plus proche du cas phare, une démonstration qu'on suit, et la page large se lit bien. *Contre* : la conversation n'est plus une conversation tant que la visite dure, et l'agent doit découper son explication en étapes (un champ `step` de `drive_browser`, en plus).
- **C — « L'onglet »** (`c-*.png`) : la page est un onglet « Revue · POR-41 » du **panneau de droite**, à côté de Non commités, Historique et PR. La conversation garde sa colonne de 780 px. Les choix ne sont pas dans la page mais **en natif au pied du panneau**, avec leurs options, la recommandation, un mot et « Choisir A ». *Pour* : le plus simple à construire et le plus sûr (la page ne fait qu'afficher, il n'y a pas de proposition à valider), et la plus proche d'aujourd'hui. *Contre* : la page est étroite (≈ 500 px à 1440 et la moitié à 1024), le panneau n'est pas un navigateur (pas de volet ni de largeurs), et le pilotage s'y voit moins.

**Choisie par l'utilisateur le 2026-11-25 : A.** Ma recommandation était la même : Elle sert le cas phare sans sacrifier la conversation. Ses repères sont le seul geste qui rend le pilotage lisible après coup (« qu'est-ce qu'il m'a montré ? »). Elle reprend tels quels la ligne, les volets et les largeurs d'ESC-657. On peut lui prendre plus tard le « Revoir la visite » de B (rejouer les repères dans l'ordre) sans changer sa structure. C reste un bon repli si l'utilisateur préfère une 2.3 plus courte : AP-3 et AP-4 y coûtent une heure de moins chacun.

## 13. Les choix, tranchés

**L'utilisateur a répondu le 2026-11-25 (demande a-36) : la direction A « Les repères », et les recommandations P1 à P10 telles quelles.** Chacun est une décision proposée (`.escouade/decisions/`), acceptée à la validation du ticket ; l'option retenue est en gras.

| # | Question | Options | Recommandé |
| --- | --- | --- | --- |
| P1 | Les outils | **`show_page`, `get_page`, `close_page`, et le pilotage par `drive_browser` d'ESC-663 (cible `page: "p-12"` ou `browser: "page:p-12"`, en AP-0)** · un seul outil `page` à actions | `show_page` + `get_page` + `close_page` + `drive_browser` |
| P2 | Où la page s'ouvre | **Une ligne de « Navigateurs », à côté de la conversation, et sa demande dans « À toi »** · un onglet du panneau de droite · les deux | La ligne et la boîte |
| P3 | Le geste qui envoie | **Un clic dans la page propose, la bande d'Escouade envoie** · le clic dans la page suffit | La bande |
| P4 | Le HTML libre | **Scripts permis dans la cloison (sans `unsafe-eval`)** · HTML sans script (assaini) · scripts et `unsafe-eval` | Scripts, sans `unsafe-eval` |
| P5 | Le réseau | **Aucun, sans exception** · une liste d'origines par projet | Aucun |
| P6 | La page à soi | **Le poste, et le dépôt qui la propose (`pages.custom`), le poste l'emporte** · le poste seulement | Le poste et le dépôt |
| P7 | La place | **Un interrupteur dans Fonctionnalités et la partie « Lancement et navigateurs »** · une partie à elle | Fonctionnalités + Lancement et navigateurs |
| P8 | À distance | **En 2.3, la page reste sur le poste et sa demande va partout ; l'aperçu en image et la page servie à une fenêtre après** · tout dans la 2.3 | Poste seulement en 2.3 |
| P9 | Qui montre | **Agents, Overseer, discussions ; Claude hors Escouade seulement avec une demande** · Claude hors Escouade aussi sans demande, dans un navigateur sans agent | Avec une demande seulement |
| P10 | Durée de vie | **Avec son agent, puis le délai des fichiers de la boîte ; 20 pages par agent, 10 versions** · le temps du lancement d'Escouade seulement | Avec son agent |
| P11 | La direction de design | **A « Les repères »** · B « La visite » · C « L'onglet » | A |

## 14. Les tickets

Dans l'ordre. Les heures sont des heures d'agent Claude Code (Opus, effort moyen), relecture et e2e compris. Le Kanban en tient six en parallèle.

| Ticket | Ce qu'il fait | Dépend de | Heures | 2.3 ? |
| --- | --- | --- | --- | --- |
| **AP-0** | Réunir avec ESC-663 : un nom d'outil de pilotage, ses actions, la règle de la main, le canal (§ 10). Mise à jour de cette étude, de la sienne et des décisions. Pas de code. | ESC-663 rendu | 0 h 45 | Oui |
| **AP-1** | La cloison : schéma `escouade-page://` servi à sa seule webview (étiquette), webview `agentpage-*` refusée par `refuses`, CSP en en-tête et en `<meta>`, `incognito`, permissions et téléchargements refusés, navigation stricte et la bande « La page veut ouvrir… », préchargements retirés, `RTCPeerConnection` retiré. `agentpage_probe` en CI (Linux, macOS) et page piège en e2e Windows (§ 8.4). | — | 3 h 30 | Oui |
| **AP-2** | Le modèle et l'outil : `pages.rs` (registre `p-<n>`, versions, bornes, copie depuis le worktree avec ses fichiers relatifs, état durable, purge), `show_page`, `get_page`, `close_page`, projet privé, interrupteur de Fonctionnalités, `parts::of_tool`, consigne du serveur MCP, journaux, statistiques. | AP-1 | 3 h | Oui |
| **AP-3** | La vue (dans la direction choisie) : la ligne `page:<p-n>` de « Navigateurs », l'ouverture à côté de la conversation, la barre (versions, la main, largeurs), la bande de confiance, la carte dans la conversation et ses états, fr et en, clair et sombre, 1440 et 1024. | AP-2 | 3 h | Oui |
| **AP-4** | Le retour par `ask_user` : `ask` de `show_page` (le même dépôt que `ask_user`), le canal `_/choose` validé par le cœur, la bande du choix et son geste (`inbox_answer`), la réponse renvoyée à la page, « Ouvrir la page » et la vignette dans « À toi », le feu vert jamais proposé par la page. | AP-2, AP-3 | 2 h 30 | Oui |
| **AP-5** | Les modèles `review`, `diagram` (Mermaid embarqué, `strict`) et `compare` (planches de la boîte), leurs schémas de données validés par le cœur, leurs ancres pour le pilotage. | AP-1 | 3 h | Oui |
| **AP-6** | Le pilotage : `drive_browser` (scripts fixes paramétrés, dans un monde isolé par système), reprendre la main et la rendre, la trace numérotée et les repères (direction A), bornes. Partagé avec le ticket d'ESC-663 qui pilote les navigateurs. | AP-3, AP-0 | 2 h 30 | Oui |
| **AP-7** | La page à soi : réglage du poste, `pages.custom` du `.escouade.yml` et de l'équipe, `window.escouade`, refus sans page réglée. | AP-2, AP-5 | 1 h 30 | Oui |
| **AP-8** | À distance et docs : `page_*` « du poste seulement », champ `page` des demandes servi au téléphone (« Une page l'accompagne… »), carte d'une fenêtre sur un serveur. Doc du site fr et en (« Les pages des agents »), page de spec passée de « À trancher » à construite. | AP-4 | 1 h 30 | Oui |
| **AP-9** | **L'e2e du cas phare** (Windows) : un faux agent appelle `show_page(review, ask)`, la page s'ouvre à côté de sa conversation, `drive_browser` fait défiler et met en évidence (capture réelle par `PrintWindow`, comme `browsers.spec.ts`), l'utilisateur reprend la main, clique A dans la page, la bande s'allume, « Envoyer », puis l'agent reçoit « L'utilisateur a choisi A ». Avec ça : la page piège ne joint rien, et une réponse dans la boîte coche la page. | AP-4, AP-6 | 2 h | Oui |
| AP-10 | L'aperçu en image : le poste rend la version en PNG (capture de la webview cachée), jointe à la demande pour le téléphone et la boîte. | AP-4 | 2 h | Après |
| AP-11 | La page servie à une fenêtre connectée à un serveur : `page_bundle` par l'API distante, cloison locale, canal relayé. | AP-4, mode serveur | 3 h | Après |
| AP-12 | Plusieurs pages à soi nommées ; d'autres moteurs de schéma (D2, Graphviz en WASM). | AP-7 | 2 h | Après |
| — | Les plugins d'interface (§ 11) : leur étude repart d'ici. | — | — | 2.4 |

**Ce qui tient dans la 2.3** : AP-0 à AP-9, soit **environ 23 h d'agent**. Le chemin le plus long (AP-1 → AP-2 → AP-3 → AP-4 → AP-9, avec AP-6 à côté) fait environ 14 h. Avec le parallélisme du Kanban (AP-5 dès AP-1 ; AP-6 et AP-7 avec AP-4) et mes vérifications (30 à 60 min par ticket), il faut compter **deux jours** du Kanban une fois les choix faits. Avec la direction C, AP-3 et AP-4 coûtent une heure de moins chacun, et AP-6 n'a pas de repères.

**Ce qui n'avance pas au rythme des agents** :

- les choix P1 à P11, à l'utilisateur ;
- ESC-663 rendu, pour AP-0 ;
- les **essais réels de la cloison sur un Mac et sous Linux**, au-delà de la CI : CSP d'un schéma maison honorée par WKWebView et WebKitGTK, WebRTC, préchargements, permissions ;
- un essai du cas phare par l'utilisateur avec un vrai agent de chaque harness qu'il a (Claude Code, Codex au moins).

## 15. Sources

- Ticket ESC-661 ; message d'un utilisateur du 2026-11-24 (une donnée) ; l'utilisateur, le 2026-11-25 (« le cas trop cool »).
- Spec : *La boîte « À toi »*, *Les navigateurs (la page du lancement)*, *Les parties d'un projet*, *Pilotage de Claude Code*, *Conversation*, *Fonctionnalités*, *L'API distante*, *L'app mobile*.
- Code lu : `src-tauri/src/devpage.rs`, `src-tauri/src/mcp/asks.rs`, `src/lib/devpage.svelte.ts`, `src/components/BrowsersSection.svelte`, `src/components/SidePanel.svelte`.
- Tauri 2.12.5 (`UriSchemeContext::webview_label`, `WebviewBuilder::incognito`, `on_download`, `on_permission_request`) et wry 0.57.2 (`PermissionRequested` de WebView2 ; presse-papiers en lecture accordé d'office quand `clipboard` est allumé).
- `docs/TODO-2.4.md` (plugins d'interface), `docs/INBOX.md`, `design/inbox-2-3/README.md` (les planches), `design/browser/INVENTAIRE.md`.
