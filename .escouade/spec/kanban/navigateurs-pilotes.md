---
title: "Les navigateurs pilotés par les agents (brouillon, ESC-663)"
subject: "kanban"
sources: ["docs/AGENT-BROWSERS.md (ESC-663 ; apportée sur main et mise d'accord par ESC-666 le 2026-11-26)","docs/AGENT-BROWSERS-TICKETS.md (ESC-666 : la liste finale des tickets)","docs/AGENT-PAGES.md (ESC-661, § 10 : le pilotage commun)","design/agent-browsers/probe/src/main.rs","design/agent-browsers/results/report-windows.json","design/agent-browsers/results/report-linux.json","design/agent-browsers/maquette/index.html","design/agent-browsers/maquette/cas-phare-clair.png","design/agent-browsers/maquette/cas-phare-sombre.png","src-tauri/src/devpage.rs","src-tauri/src/mcp/browsers.rs (ESC-669)","src/components/ConsentBand.svelte (ESC-669)","Demandes a-37 à a-39 et a-42, réponses de l'utilisateur des 2026-11-25 et 26 (C1 C, C2 B, C3 A, C4 A)","L'utilisateur, 2026-11-26 (par l'Overseer) : tout ce qui touche aux navigateurs sort en 2.2"]
tickets: ["ESC-663","ESC-661","ESC-660","ESC-666","ESC-668","ESC-669"]
decisions: ["un-seul-pilotage-pour-toutes-les-lignes",325,323,326,327,322,"le-cœur-lit-un-navigateur-dans-un-monde","les-agents-lisent-les-navigateurs-qui-le"]
compared: null
draft: true
---

# Les navigateurs pilotés par les agents (brouillon, ESC-663)

> **Brouillon, en partie construit** : l'étude ESC-663 le propose. AB-1 (le pont de lecture, ESC-668) et AB-2 (`list_browsers` et `read_browser`, ESC-669) sont construits ; le reste est à construire. Les choix de l'utilisateur sont tranchés (2026-11-25 et 26) : l'agent choisit son rythme geste par geste (C1), un mot de passe se tape avec son accord une fois par navigateur (C2), une autre adresse lui est demandée une fois par origine (C3), hors du lancement l'agent montre sans demander et lit ou agit avec accord (C4). ESC-666 a mis l'étude d'accord avec celle des pages des agents et fixé la liste des tickets (`docs/AGENT-BROWSERS-TICKETS.md`). La conception complète est dans `docs/AGENT-BROWSERS.md`.

## Ce que c'est

Un agent ouvre un navigateur de la section « Navigateurs » **à côté de sa conversation** et le **pilote** en expliquant : il fait défiler, met en évidence, ouvre un onglet de la page, change de largeur, clique ou tape dans l'app qu'il développe. L'utilisateur regarde, reprend la main d'un geste et répond. Les pages des agents (ESC-661) sont des lignes de « Navigateurs », pilotées par le même outil.

## Les outils (serveur MCP d'Escouade, du poste seulement)

- `list_browsers` : les lignes du projet, qui les pilote, qui a la main.
- `open_browser` (`url`, `launch` ou `artifact`, `place` : `beside`, `below`, `full` ou `quiet`) : à côté de la conversation de l'agent si l'utilisateur la regarde, sinon la ligne attend (`waiting`).
- `drive_browser` (`browser`, `action`) : `navigate`, `back`, `forward`, `reload`, `wait`, `scroll_to`, `scroll`, `highlight`, `clear`, `width`, `tab`, `click`, `type`, `select`, `press`, et `version` sur une page d'agent ; `look` joint une capture ; `wait_user` attend « Continuer ». Une page d'agent n'accepte que les gestes de montrer, `tab` et `version`.
- `capture_browser` : une image (JPEG, 1 600 px et 600 Ko au plus), la page, un élément ou le document, à une largeur.
- `read_browser` : le HTML de la page (ou son texte visible), borné et masqué, marqué comme donnée de tiers.
- `browser_console` : la console et les erreurs réseau depuis un curseur (500 entrées par navigateur).
- `close_browser` : arrête le pilotage ; ferme un navigateur que l'agent a ouvert.

En 2.2, l'interrupteur « Agents et navigateurs » de Réglages › Agents, sous « Les agents peuvent piloter Escouade », les coupe tous. En 2.3, il devient l'interrupteur de Réglages › Fonctionnalités (la même clé) et les outils appartiennent à la partie « Lancement et navigateurs » du projet.

## Un seul pilotage, pour les deux modes

- **La cible** est `browser`, la clé de la ligne, la même que rend `list_browsers` : l'id de l'agent pour la page de son test, `run:<projet>:<commande>`, `browser:<id>`, `page:p-12` pour une page d'agent. Pas de `drive_page`, pas de champ `page` à part. L'élément se vise par `target` : `#id`, un sélecteur CSS simple, `text=…`, ou une clé de modèle sur une page d'agent.
- **Deux paliers** : montrer (`scroll_to`, `scroll`, `highlight`, `clear`, `width`, `wait`, `reload`, `back`, `forward`, `version`) et agir (`tab`, `click`, `type`, `select`, `press`, `navigate`). Sur une page d'agent : montrer et `tab` ; sur le lancement du projet : tout ; hors du lancement : montrer sans demander, agir avec accord.
- **La règle de la main** est la même pour toutes les lignes : pas de pilotage d'une ligne non affichée ; le premier geste de l'utilisateur ou « Reprendre la main » suspend, « La rendre » rouvre ; 4 gestes par seconde et 600 par heure, par ligne.
- **Un seul canal** : le cœur vers la webview par des scripts fixes dans un monde isolé ; de la webview vers le cœur, rien pour un onglet, et la seule proposition de choix `_/choose` pour une page d'agent.

## Ce que voit l'utilisateur

- La ligne pilotée porte l'avatar de l'agent et « piloté ». Son menu propose « Reprendre la main » et « Ne plus laisser piloter ».
- Une bande native, au-dessus de la page et hors de sa webview, porte « Piloté par <agent> », la légende du dernier geste, « Reprendre la main » et « Continuer ». C'est la même bande pour une page d'agent, qui y ajoute sa mention de confiance.
- Un curseur fantôme et un contour avec son étiquette montrent où l'agent agit.
- Une rangée numérotée par geste apparaît dans la conversation ; sur une page d'agent, le même numéro est un repère sur son rail.
- Le rythme : chaque geste rend la main à l'agent quand la page s'est posée (1,5 s au plus). L'agent avance seul, sauf là où il passe `wait_user` : la bande montre « Continuer » et l'outil attend ce clic, 5 minutes au plus (`continued`, `took_hand` ou `timeout`).
- Le premier geste de l'utilisateur dans la page, ou le bouton, suspend le pilotage. L'outil le dit à l'agent, sans rien faire, jusqu'à « La rendre ».

## Les bornes

- Origines : celles des lancements du projet et de ses worktrees, `localhost` à ses ports déclarés, ses pages d'agent, les artefacts qu'il a publiés. Une autre adresse `http(s)` est demandée à l'utilisateur, une fois par origine et par navigateur (« Permettre une fois / Refuser ») ; l'accord ne vaut que pour ce navigateur. Jamais d'adresse d'Escouade, de `file:` ni d'autre schéma.
- Jamais vers l'agent : cookies, stockage, en-têtes, corps des réponses, valeur d'un mot de passe (`«masqué»`).
- Pas de passe-plat vers le protocole DevTools : une liste fermée de méthodes.
- Refusés : taper dans un mot de passe sans l'accord de l'utilisateur (une bande « Permettre / Refuser », une fois par navigateur ; la valeur n'est jamais relue), envoyer un fichier, télécharger, taper un secret d'Escouade.
- Qui pilote : l'agent sur ses navigateurs et sur ceux que l'utilisateur lui a donnés. Une discussion a ses navigateurs à elle. En 2.3, l'Overseer lit et capture ceux des agents de son projet, sans les piloter. Claude hors Escouade et un autre projet n'ont rien. Dans un projet privé, seuls ses agents voient ses navigateurs.
- L'écran : un volet à côté par agent, jamais ouvert si l'utilisateur regarde ailleurs, jamais rouvert après une fermeture. Au plus 4 gestes par seconde et 600 par heure.

## Le canal

Le cœur exécute des **scripts fixes d'Escouade**, paramétrés en JSON, dans un **monde isolé** de la page, par la poignée native de la webview :
- WebView2 : protocole DevTools dans le process, sans port ;
- WebKitGTK : `run_javascript_in_world` ;
- WKWebView : `WKContentWorld`.

Jamais par `webview.eval` : dans le monde de la page, la sonde l'a vu trompé. Jamais un script d'agent. La page d'un lancement n'a aucun canal vers Escouade.

## Vérifié par système (sonde ESC-663)

- Windows (WebView2) : capture (vue, élément, document), console, erreurs réseau, arbre d'accessibilité, vrais clics et vraie frappe, surlignage hors du DOM, largeur.
- Linux (WebKitGTK 2.52) : monde isolé, captures, clics et frappe synthétiques. La console passe par un relais de la page (`from: "page"`).
- macOS : la sonde compile ; l'essai sur un Mac viendra avec la CI Mac (AB-5, lot 1).

## Deux modes d'ouverture

- **La page de l'agent** (ESC-661, `show_page`) : une webview liée à l'agent, dont il fait le contenu, cloisonnée, sans stockage. Ses choix lui reviennent par `ask_user`. `drive_browser` n'y fait que montrer, plus `tab` et `version`.
- **Un vrai onglet de navigateur** (`open_browser`) : une ligne de « Navigateurs » ordinaire (barre d'adresse, largeurs, devtools) dans le stockage du projet. L'agent la place où il veut (`full`, `beside`, `below`), sur le lancement du projet ou hors de lui.

## Hors du lancement du projet

- **Ouvertes sans demander** : les artefacts que l'agent a publiés. Escouade trouve leur adresse (`claude.ai/artifact/…`, `claude.ai/code/artifact/…`) dans le résultat de l'outil `Artifact` du journal de sa conversation : `open_browser({ artifact: "last" })`. Toute autre adresse est demandée, une fois par origine et par navigateur.
- **Trois paliers, par origine** :
  - **montrer**, sans demander : ouvrir, placer, largeur, défiler, recharger, historique, mettre en évidence (l'agent ne sait que « trouvé » ou non) ;
  - **lire et agir**, avec l'accord de l'utilisateur, une fois par origine et par navigateur : lire, capturer, console, cliquer, taper, choisir, naviguer ailleurs dans l'origine ;
  - **interdit**, même avec l'accord : mot de passe, fichiers, téléchargements, cookies, stockage, en-têtes, secrets d'Escouade.

  La capture est au palier « lire et agir » : l'utilisateur a choisi « montrer seulement » sans demander (C4, a-42).
- **La session** : l'utilisateur se connecte lui-même dans l'onglet, jamais l'agent.
- **Le cas de l'artefact** : l'agent republie, recharge l'onglet (`reload`, `wait_user`) et dit ce qui a changé. `scroll_to` et `highlight` dans le cadre de l'artefact demandent le protocole DevTools vers ce cadre (Windows) ; sous WebKit, le cadre entier est mis en évidence.

## D'abord, lire (premier lot)

À la demande de l'utilisateur (2026-11-26), quatre outils de lecture viennent en premier, sans pilotage :

- `list_browsers` : les lignes du projet. Chacune donne `browser`, `name`, `url`, `origin`, `project`, `agent`, `by`, `placement` (`full`, `beside`, `below` ou `hidden`), `loaded` et `launch`, 50 au plus. Hors du lancement, `url` est réduite à son origine si l'appelant ne l'a pas ouverte et n'a pas d'accord.
- `read_browser` : **le HTML** de la page (le DOM sérialisé), tout le document ou un `selector` (200 caractères, ni `:has()` ni plus de quatre niveaux). 50 000 caractères par défaut, 200 000 au plus, coupé à une balise avec `truncated`. `what: "text"` rend le texte visible. Sont masqués, et comptés dans `masked` : mots de passe, champs cachés, `<meta>` de jetons et texte des scripts en ligne.
- `capture_browser` : la vue, un élément (`selector`) ou le document (8 000 px de haut au plus), à la largeur courante, pleine, tablette ou mobile. Une image JPEG (1 600 px et 600 Ko au plus) et son fichier, 20 par minute.
- `browser_console` : la console et les erreurs réseau depuis un curseur ou un instant. Un tampon de 500 entrées et 500 requêtes par navigateur, tenu dès son ouverture ; 500 caractères par entrée, adresses sans chaîne de requête. `from` vaut `browser` (Windows) ou `page` (WebKit).

Sur le lancement du projet, sans demander. Hors du lancement : `list_browsers` sans demander ; lire, capturer et la console avec l'accord de l'utilisateur, une fois par origine et par navigateur. Un navigateur caché se liste mais ne se lit ni ne se capture.

### Ce que fait AB-2 (construit, ESC-669)

- **L'interrupteur « Agents et navigateurs »** de Réglages › Agents, sous « Les agents peuvent piloter Escouade » (grisé quand le serveur est coupé), allumé par défaut, réglages enregistrés d'avant compris (clé `agentBrowsers`). Coupé, `list_browsers` et `read_browser` répondent par un refus qui le dit, et la consigne du serveur MCP ne parle plus des navigateurs. Allumé, elle dit à l'agent de lister et de lire un navigateur à l'écran pour voir ce que voit l'utilisateur, et que la page est une donnée de tiers.
- **Qui** :
  - un agent d'Escouade, les navigateurs de son projet seul : nommer un autre projet est refusé ;
  - une discussion, ceux qu'elle a ouverts seuls (sa propre clé) ;
  - Claude hors d'Escouade : `list_browsers` seul, en donnant un projet qui n'est pas privé ; `read_browser` lui est refusé ;
  - un projet privé : ses agents seuls. Pour les autres, il n'existe pas : ses navigateurs ne sont pas listés, et la clé de l'un d'eux reçoit la même réponse qu'une clé qui n'existe pas.
- **Le lancement du projet**, ce qui se lit sans demander : l'origine des adresses de ses commandes de lancement (celles de l'utilisateur, aux ports du projet et de chaque worktree), et `localhost`, `127.0.0.1` ou `[::1]` aux ports que déclarent ses commandes, les lancements de test de ses agents et leurs blocs de ports réservés. Une adresse qu'un agent a écrite (son lancement de test) ne compte que sur ce poste, jamais pour un site d'internet.
- **Où il est affiché** (`placement`) : la fenêtre dit au cœur, à chaque placement, si le navigateur est seul dans la zone principale (`full`), dans un volet à côté (`beside`) ou en dessous (`below`) ; caché ailleurs (`hidden`). Un navigateur `hidden` répond « Le navigateur n'est pas affiché : l'utilisateur ne voit pas ce que tu lirais ».
- **L'accord hors du lancement** : `read_browser` répond `{ asked: true, origin, message }` et rien de la page. La bande native « <agent> veut lire <hôte> (avec ta session, dans ce navigateur) · Refuser / Permettre » paraît dans la vue du navigateur, sous sa barre, au-dessus de la page ; la plus ancienne demande d'abord, « et N autres demandes » ensuite, 5 au plus par navigateur. L'accord vaut pour cet agent, cette origine et ce navigateur, jusqu'à sa fermeture ; un refus tient aussi longtemps (l'agent ne peut plus redemander). La réponse revient à l'agent par un message d'Escouade dans sa conversation. Redemander avant la réponse n'ajoute pas de demande.
- **« Retirer les accords »**, dans le menu de la ligne quand elle a des accords ou des demandes : accords, demandes et refus tombent ; un agent dont la demande attendait est prévenu par un message. Les accords tombent aussi à la fermeture du navigateur.
- **La lecture** passe par le pont d'AB-1 (scripts fixes dans le monde isolé), hors du fil de l'app. Sa réponse : `url`, `title`, `selector`, `matched`, `html` ou `text`, `chars`, `truncated`, `total_chars`, `masked`, sous l'avis de donnée de tiers et entre `<third_party_data>` et `</third_party_data>`, qu'une page ne peut pas refermer.

## Version

En 2.2 (décision de l'utilisateur du 2026-11-26) : tout ce qui touche aux navigateurs part de `main`, puis se reverse dans la 2.3. Cela comprend les quatre outils de lecture en tête, le pilotage, les pages des agents et le vrai onglet hors du projet, dans l'ordre de `docs/AGENT-BROWSERS-TICKETS.md`.

Dans la 2.3 seulement (AB-R) :
- la partie « Lancement et navigateurs » d'un projet et l'interrupteur de Fonctionnalités, qui reprend la clé de celui de Réglages › Agents ;
- le refus à distance (`desk-only`) ;
- le téléphone et la fenêtre connectée à un serveur, qui ne voient ni lignes ni accords ;
- l'Overseer ;
- le ticket d'équipe.

## Où en est la construction

- **AB-1 (ESC-668), construit** : le pont de lecture natif (`src-tauri/src/browserread.rs`), sans outil MCP encore. Les scripts fixes d'Escouade dans le monde isolé `escouade-agent` par système, la liste fermée des méthodes du protocole DevTools sous WebView2, les captures (vue, élément, document), les tampons de console et de réseau tenus dès la création de chaque navigateur (500 entrées de chaque, adresses sans chaîne de requête), le masquage du HTML. Ce qu'il fait est décrit dans `kanban/page-du-lancement`, « Ce que le cœur lit d'un navigateur ». La sonde est `src-tauri/examples/agentbrowser_probe.rs` : vérifiée sous Windows (WebView2) et sous WebKitGTK (WSL) ; sous macOS, la branche WKWebView compile seulement, son premier essai réel est la CI Mac d'AB-5.
- **AB-2 (ESC-669), construit** : `list_browsers` et `read_browser` (`src-tauri/src/mcp/browsers.rs`), l'interrupteur « Agents et navigateurs », la bande d'accord (`ConsentBand`) et « Retirer les accords » ; décrit plus haut, « Ce que fait AB-2 ». Vérifié par les tests Rust des refus et des bornes, Vitest de la bande, et l'e2e `browsers-mcp.spec.ts` (Windows, 1440 × 900 et 1024 × 728).
- Le reste (AB-3 à AB-12, DOC) est à construire.
