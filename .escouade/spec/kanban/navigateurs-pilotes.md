---
title: "Les navigateurs pilotés par les agents (brouillon, ESC-663)"
subject: "kanban"
sources: ["docs/AGENT-BROWSERS.md (ESC-663 ; apportée sur main et mise d'accord par ESC-666 le 2026-11-26)","docs/AGENT-BROWSERS-TICKETS.md (ESC-666 : la liste finale des tickets)","docs/AGENT-PAGES.md (ESC-661, § 10 : le pilotage commun)","design/agent-browsers/probe/src/main.rs","design/agent-browsers/results/report-windows.json","design/agent-browsers/results/report-linux.json","design/agent-browsers/maquette/index.html","design/agent-browsers/maquette/cas-phare-clair.png","design/agent-browsers/maquette/cas-phare-sombre.png","src-tauri/src/devpage.rs","src-tauri/src/mcp/browsers.rs (ESC-669, ESC-671)","src-tauri/src/browserread.rs (ESC-668, ESC-671)","src/components/ConsentBand.svelte (ESC-669)","Demandes a-37 à a-39 et a-42, réponses de l'utilisateur des 2026-11-25 et 26 (C1 C, C2 B, C3 A, C4 A)","L'utilisateur, 2026-11-26 (par l'Overseer) : tout ce qui touche aux navigateurs sort en 2.2","src-tauri/src/mcp/browsers.rs (ESC-669, ESC-670)","src-tauri/src/browserread.rs (ESC-668 ; fit, ESC-670)","src-tauri/src/mcp/browsers.rs (ESC-669)","src-tauri/src/mcp/browsers_open.rs (ESC-678)","src/components/OpenAskBand.svelte (ESC-678)","Le chef d'escouade, ticket ESC-678 (2026-11-26) : la boucle locale hors des ports déclarés est refusée, pas demandée","src-tauri/src/mcp/browsers.rs (ESC-669, ESC-670, ESC-671)","src-tauri/src/browserread.rs (ESC-668, ESC-670, ESC-671 ; les entrées de la liste fermée, ESC-679)","src-tauri/src/browserdrive.rs, browserdrive.js, src-tauri/src/mcp/browsers_drive.rs (ESC-679)","tests/e2e/agent-browsers-drive.spec.ts (ESC-679)"]
tickets: ["ESC-663","ESC-661","ESC-660","ESC-666","ESC-668","ESC-669","ESC-670","ESC-671","ESC-692","ESC-678","ESC-679"]
decisions: ["un-seul-pilotage-pour-toutes-les-lignes",325,323,326,327,322,"le-cœur-lit-un-navigateur-dans-un-monde","les-agents-lisent-les-navigateurs-qui-le","le-relais-de-console-de-webkit-ne-rend-s","un-agent-ouvre-un-onglet-a-cote-de-sa-co","drive-browser-pilote-les-onglets-et-les"]
compared: null
draft: true
---

# Les navigateurs pilotés par les agents (brouillon, ESC-663)

> **Brouillon, en partie construit** : l'étude ESC-663 le propose. AB-1 (le pont de lecture, ESC-668), AB-2 (`list_browsers` et `read_browser`, ESC-669), AB-3 (`capture_browser`, ESC-670), AB-4 (`browser_console`, ESC-671), AB-6 (`open_browser` et `close_browser`, ESC-678) et AB-7 (`drive_browser`, avec AP-6, ESC-679) sont construits ; le reste est à construire, d'abord ce que voit l'utilisateur du pilotage (AB-8). Les choix de l'utilisateur sont tranchés (2026-11-25 et 26) : l'agent choisit son rythme geste par geste (C1), un mot de passe se tape avec son accord une fois par navigateur (C2), une autre adresse lui est demandée une fois par origine (C3), hors du lancement l'agent montre sans demander et lit ou agit avec accord (C4). ESC-666 a mis l'étude d'accord avec celle des pages des agents et fixé la liste des tickets (`docs/AGENT-BROWSERS-TICKETS.md`). La conception complète est dans `docs/AGENT-BROWSERS.md`.

## Ce que c'est

Un agent ouvre un navigateur de la section « Navigateurs » **à côté de sa conversation** et le **pilote** en expliquant : il fait défiler, met en évidence, ouvre un onglet de la page, change de largeur, clique ou tape dans l'app qu'il développe. L'utilisateur regarde, reprend la main d'un geste et répond. Les pages des agents (ESC-661) sont des lignes de « Navigateurs », pilotées par le même outil.

## Les outils (serveur MCP d'Escouade, du poste seulement)

- `list_browsers` : les lignes du projet, si l'appelant peut les piloter (`drivable`), qui a la main (`hand`).
- `open_browser` (`url`, `launch` ou `artifact`, `place` : `beside`, `below`, `full` ou `quiet`) : à côté de la conversation de l'agent si l'utilisateur la regarde, sinon la ligne attend (`waiting`).
- `drive_browser` (`browser`, `action`) : `navigate`, `back`, `forward`, `reload`, `wait`, `scroll_to`, `scroll`, `highlight`, `clear`, `width`, `tab`, `click`, `type`, `select`, `press`, et `version` sur une page d'agent ; `look` joint une capture ; `wait_user` attend « Continuer ». Une page d'agent n'accepte que les gestes de montrer, `tab` et `version`.
- `capture_browser` : une image (JPEG, 1 600 px et 600 Ko au plus), la page, un élément ou le document, à une largeur.
- `read_browser` : le HTML de la page (ou son texte visible), borné et masqué, marqué comme donnée de tiers.
- `browser_console` : la console et les erreurs réseau depuis un curseur (500 entrées par navigateur).
- `close_browser` : arrête le pilotage ; ferme un navigateur que l'agent a ouvert.

En 2.2, l'interrupteur « Agents et navigateurs » de Réglages › Agents, sous « Les agents peuvent piloter Escouade », les coupe tous (pour `drive_browser`, les onglets ; « Pages des agents » coupe ses lignes `page:`). En 2.3, il devient l'interrupteur de Réglages › Fonctionnalités (la même clé) et les outils appartiennent à la partie « Lancement et navigateurs » du projet.

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

Ces éléments viennent avec AB-8 ; le cœur qu'ils montrent est construit (AB-7, plus bas).

## Les bornes

- Origines : celles des lancements du projet et de ses worktrees, `localhost` à ses ports déclarés, ses pages d'agent, les artefacts qu'il a publiés. Une autre adresse `http(s)` d'internet est demandée à l'utilisateur, une fois par origine et par navigateur (« Permettre une fois / Refuser ») ; l'accord ne vaut que pour ce navigateur. Jamais d'adresse d'Escouade, de `file:` ni d'autre schéma. Ce poste (`localhost`, `127.0.0.1`, `[::1]`) à un port qu'aucun lancement du projet ne déclare est **refusé, pas demandé** : ce peut être un service du poste ou d'Escouade (décision du chef d'escouade, plus stricte que l'étude, ESC-678).
- Jamais vers l'agent : cookies, stockage, en-têtes, corps des réponses, valeur d'un mot de passe (`«masqué»`).
- Pas de passe-plat vers le protocole DevTools : une liste fermée de méthodes, dont les paramètres des entrées sont vérifiés.
- Refusés : taper dans un mot de passe sans l'accord de l'utilisateur sur le lancement, et toujours hors de lui ; envoyer un fichier ; télécharger ; un champ d'un formulaire qui s'envoie vers une autre origine ; taper un secret d'Escouade.
- Qui pilote : l'agent sur ses navigateurs (la page de son test, ceux qu'il a ouverts, les lancements qu'il pilote) et sur ceux que l'utilisateur lui a donnés (AB-9). Une discussion a ses navigateurs à elle. En 2.3, l'Overseer lit et capture ceux des agents de son projet, sans les piloter. Claude hors Escouade et un autre projet n'ont rien. Dans un projet privé, seuls ses agents voient ses navigateurs.
- L'écran : un volet à côté par agent, jamais ouvert si l'utilisateur regarde ailleurs, jamais rouvert après une fermeture. Au plus 4 gestes par seconde et 600 par heure.

## Le canal

Le cœur exécute des **scripts fixes d'Escouade**, paramétrés en JSON, dans un **monde isolé** de la page, par la poignée native de la webview :
- WebView2 : protocole DevTools dans le process, sans port ;
- WebKitGTK : `run_javascript_in_world` ;
- WKWebView : `WKContentWorld`.

Jamais par `webview.eval` : dans le monde de la page, la sonde l'a vu trompé. Jamais un script d'agent. La page d'un lancement n'a aucun canal vers Escouade. Sous WebKit, le monde isolé ne garde rien d'un appel à l'autre (ESC-692) : chaque script fixe fait tout dans son appel.

## Vérifié par système (sonde ESC-663)

- Windows (WebView2) : capture (vue, élément, document), console, erreurs réseau, arbre d'accessibilité, vrais clics et vraie frappe, surlignage hors du DOM, largeur.
- Linux (WebKitGTK 2.52) : monde isolé, captures, clics et frappe synthétiques. La console passe par un relais de la page (`from: "page"`), que le cœur interroge à chaque relève (ESC-692).
- macOS (WKWebView) : le pont de lecture tourne en CI (ESC-672) : lecture, captures de la vue et d'un élément, et la console avec l'exception et les requêtes en échec depuis ESC-692.

## Deux modes d'ouverture

- **La page de l'agent** (ESC-661, `show_page`) : une webview liée à l'agent, dont il fait le contenu, cloisonnée, sans stockage. Ses choix lui reviennent par `ask_user`. `drive_browser` n'y fait que montrer, plus `tab` et `version`.
- **Un vrai onglet de navigateur** (`open_browser`) : une ligne de « Navigateurs » ordinaire (barre d'adresse, largeurs, devtools) dans le stockage du projet. L'agent la place où il veut (`full`, `beside`, `below`), sur le lancement du projet ou hors de lui.

## Hors du lancement du projet

- **Ouvertes sans demander** : les artefacts que l'agent a publiés. Escouade trouve leur adresse (`claude.ai/artifact/…`, `claude.ai/code/artifact/…`) dans le résultat de l'outil `Artifact` du journal de sa conversation : `open_browser({ artifact: "last" })`. Toute autre adresse est demandée, une fois par origine et par navigateur.
- **Trois paliers, par origine** :
  - **montrer**, sans demander : ouvrir, placer, largeur, défiler, recharger, historique, mettre en évidence (l'agent ne sait que « trouvé » ou non) ;
  - **lire et agir**, avec l'accord de l'utilisateur, une fois par origine et par navigateur : lire, capturer, console, cliquer, taper, choisir, naviguer ailleurs dans l'origine ;
  - **interdit**, même avec l'accord : mot de passe, fichiers, téléchargements, formulaire vers une autre origine, cookies, stockage, en-têtes, secrets d'Escouade.

  La capture est au palier « lire et agir » : l'utilisateur a choisi « montrer seulement » sans demander (C4, a-42).
- **La session** : l'utilisateur se connecte lui-même dans l'onglet, jamais l'agent.
- **Le cas de l'artefact** : l'agent republie, recharge l'onglet (`reload`, `wait_user`) et dit ce qui a changé. `scroll_to` et `highlight` dans le cadre de l'artefact demandent le protocole DevTools vers ce cadre (Windows) ; sous WebKit, le cadre entier est mis en évidence.

## D'abord, lire (premier lot)

À la demande de l'utilisateur (2026-11-26), quatre outils de lecture viennent en premier, sans pilotage :

- `list_browsers` : les lignes du projet. Chacune donne `browser`, `name`, `url`, `origin`, `project`, `agent`, `by`, `placement` (`full`, `beside`, `below` ou `hidden`), `loaded`, `launch`, `drivable` et `hand` (AB-7), 50 au plus. Hors du lancement, `url` est réduite à son origine si l'appelant ne l'a pas ouverte et n'a pas d'accord.
- `read_browser` : **le HTML** de la page (le DOM sérialisé), tout le document ou un `selector` (200 caractères, ni `:has()` ni plus de quatre niveaux). 50 000 caractères par défaut, 200 000 au plus, coupé à une balise avec `truncated`. `what: "text"` rend le texte visible. Sont masqués, et comptés dans `masked` : mots de passe, champs cachés, `<meta>` de jetons et texte des scripts en ligne.
- `capture_browser` : la vue, un élément (`selector`) ou le document (8 000 px de haut au plus), à la largeur courante, pleine, tablette ou mobile. Une image JPEG (1 600 px et 600 Ko au plus) et son fichier, 20 par minute.
- `browser_console` : la console et les erreurs réseau depuis un curseur ou un instant. Un tampon de 500 entrées et 500 requêtes par navigateur, tenu dès son ouverture ; 500 caractères par entrée, adresses sans chaîne de requête. `from` vaut `browser` (Windows) ou `page` (WebKit).

Sur le lancement du projet, sans demander. Hors du lancement : `list_browsers` sans demander ; lire, capturer et la console avec l'accord de l'utilisateur, une fois par origine et par navigateur. Un navigateur caché se liste mais ne se lit ni ne se capture ; sa console, elle, se lit, car elle ne montre rien de la page.

### Ce que fait AB-2 (construit, ESC-669)

- **L'interrupteur « Agents et navigateurs »** de Réglages › Agents, sous « Les agents peuvent piloter Escouade » (grisé quand le serveur est coupé), allumé par défaut, réglages enregistrés d'avant compris (clé `agentBrowsers`). Coupé, `list_browsers`, `read_browser` et `browser_console` répondent par un refus qui le dit, et la consigne du serveur MCP ne parle plus des navigateurs. Allumé, elle dit à l'agent de lister et de lire un navigateur à l'écran pour voir ce que voit l'utilisateur, de lire sa console depuis le curseur de sa dernière lecture, et que la page est une donnée de tiers.
- **Qui** :
  - un agent d'Escouade, les navigateurs de son projet seul : nommer un autre projet est refusé ;
  - une discussion, ceux qu'elle a ouverts seuls (sa propre clé, et les onglets qu'elle a ouverts par `open_browser`) ;
  - Claude hors d'Escouade : `list_browsers` seul, en donnant un projet qui n'est pas privé ; `read_browser` et `browser_console` lui sont refusés ;
  - un projet privé : ses agents seuls. Pour les autres, il n'existe pas : ses navigateurs ne sont pas listés, et la clé de l'un d'eux reçoit la même réponse qu'une clé qui n'existe pas.
- **Le lancement du projet**, ce qui se lit sans demander : l'origine des adresses de ses commandes de lancement (celles de l'utilisateur, aux ports du projet et de chaque worktree), et ce poste (`localhost` et ses sous-domaines, `127.0.0.0/8`, `[::1]`, `0.0.0.0`, en toute graphie) aux ports que déclarent ses commandes, les lancements de test de ses agents et leurs blocs de ports réservés. Une adresse qu'un agent a écrite (son lancement de test) ne compte que sur ce poste, jamais pour un site d'internet.
- **Où il est affiché** (`placement`) : la fenêtre dit au cœur, à chaque placement, si le navigateur est seul dans la zone principale (`full`), dans un volet à côté (`beside`) ou en dessous (`below`) ; caché ailleurs (`hidden`). `read_browser` sur un navigateur `hidden` répond « Le navigateur n'est pas affiché : l'utilisateur ne voit pas ce que tu lirais ».
- **L'accord hors du lancement** : `read_browser` répond `{ asked: true, origin, message }` et rien de la page. La bande native « <agent> veut lire <hôte> (avec ta session, dans ce navigateur) · Refuser / Permettre » paraît dans la vue du navigateur, sous sa barre, au-dessus de la page ; la plus ancienne demande d'abord, « et N autres demandes » ensuite, 5 au plus par navigateur. L'accord vaut pour cet agent, cette origine et ce navigateur, jusqu'à sa fermeture ; un refus tient aussi longtemps (l'agent ne peut plus redemander). La réponse revient à l'agent par un message d'Escouade dans sa conversation, qui lui dit qu'il peut refaire `read_browser` ou `browser_console`. Redemander avant la réponse n'ajoute pas de demande.
- **« Retirer les accords »**, dans le menu de la ligne quand elle a des accords ou des demandes : accords, demandes et refus tombent ; un agent dont la demande attendait est prévenu par un message. Les accords tombent aussi à la fermeture du navigateur.
- **La lecture** passe par le pont d'AB-1 (scripts fixes dans le monde isolé), hors du fil de l'app. Sa réponse : `url`, `title`, `selector`, `matched`, `html` ou `text`, `chars`, `truncated`, `total_chars`, `masked`, sous l'avis de donnée de tiers et entre `<third_party_data>` et `</third_party_data>`, qu'une page ne peut pas refermer.

### Ce que fait AB-3 (construit, ESC-670)

- **`capture_browser`** (`browser`, `selector`, `width` : `current` par défaut, `full`, `tablet` ou `mobile`, `full_page`) : la vue, le premier élément visible d'un sélecteur (ce qu'il en montre dans la vue ; rien ne défile) ou le document entier (8 000 px de haut au plus ; refusé sous macOS ; pas avec un sélecteur). Mêmes règles de sélecteur que `read_browser`. Un élément absent : « Aucun élément visible pour « <sélecteur> » ».
- **La réponse** : une **image MCP** JPEG (qualité 70, 1 600 px au plus sur son grand côté, 600 Ko au plus : la qualité baisse, puis l'image rapetisse, jusqu'à tenir ; un JPEG qui tient est gardé tel quel), puis, sous l'avis de donnée de tiers, `url`, `width` et `height` (la partie prise, en pixels CSS), `scale` (pixels de l'image par pixel CSS), `pixels`, `selector` et `file`.
- **Le fichier de repli** (`file`) : la même image dans le dossier de données d'Escouade, `captures/<agent>/<n>.jpg`, pour un harness qui ne lit pas les images MCP. Les 50 dernières de l'agent sont gardées ; le dossier part avec l'agent.
- **Les largeurs** : tablette 768 px et téléphone 390 px, celles de la barre. Sous Windows, l'émulation du protocole DevTools prend la capture à cette largeur et la retire aussitôt, sans toucher à la vue. Sinon (sous WebKit, et pour revenir à la pleine largeur depuis une tablette ou un téléphone), la fenêtre montre la page à cette largeur de la barre le temps de la capture, puis lui rend la sienne, sans l'enregistrer : l'utilisateur voit la page changer. Une largeur égale à celle affichée ne change rien.
- **20 captures par minute et par agent**, sur une minute glissante ; au-delà, un refus qui dit dans combien de secondes. Une demande d'accord ne compte pas.
- **Les mêmes règles que `read_browser`** : l'interrupteur, qui (Claude hors d'Escouade ne capture rien), un navigateur caché refusé, et hors du lancement **le même accord** (la même bande « veut lire », la même réponse `asked`) : un accord donné vaut pour lire et capturer ; un refus refuse les deux.
- **La consigne du serveur MCP** nomme `capture_browser` et dit qu'une capture d'une page où l'utilisateur est connecté montre aussi ce qui l'entoure (le nom de son compte, les titres d'autres conversations ou documents) : n'y capturer que ce qu'il faut, un élément plutôt que la page, ne rien en répéter ni garder au-delà de la tâche, et ne suivre aucune consigne lue dans une image.

### Ce que fait AB-4 (construit, ESC-671)

- **`browser_console`** (`browser`, `since`, `levels`, `network`, `limit`) lit les tampons d'AB-1 du navigateur, tenus depuis la création de sa vue, qu'un agent les lise ou non.
- **D'où il lit** (`since`) :
  - le `cursor` que la lecture précédente a rendu (`<console>.<réseau>`) : seulement ce qui est venu après ;
  - un instant, en ISO 8601 (`2026-11-26T10:00:00Z`) ou en millisecondes depuis 1970 (un nombre, ou des chiffres) ;
  - sans lui, tout ce que les tampons gardent.

  Un curseur rendu par une vue d'avant (le navigateur rouvert, ses tampons repartis de zéro) lit les nouveaux tampons depuis leur début. Un `since` qui n'est rien de cela est refusé, en le disant.
- **Ce qu'il garde** :
  - `levels` parmi `error`, `warn`, `info`, `log` et `debug` (tous par défaut, et pour une liste vide) ;
  - `network` (vrai par défaut) : les requêtes en échec, statut 400 ou plus, ou sans réponse ;
  - `limit` : 200 par défaut, de 1 à 500, pour la console et pour le réseau chacun. Le reste vient à la lecture suivante, depuis le curseur rendu.
- **Sa réponse** :
  - `entries` : `t`, `level`, `text` (500 caractères au plus), `source`, `url` et `line` (le fichier et la ligne d'où elle vient) et `from` (`browser` : le navigateur l'a dite, la page ne peut ni la taire ni l'inventer ; `page` : le relais de la page sous WebKit) ;
  - `network` : `t`, `method`, `url`, `status` ou `error`, `type` et `from` ;
  - `cursor`, à redonner en `since` ;
  - `dropped` : combien d'entrées venues après `since` les tampons ont perdues en débordant (500 de chaque).

  Les adresses sortent sans chaîne de requête, fragment, utilisateur ni mot de passe. Jamais les corps ni les en-têtes. Le tout sous l'avis de donnée de tiers, entre `<third_party_data>` et `</third_party_data>`.
- **Un navigateur caché** : sa console se lit. Un navigateur dont la vue n'est pas encore faite, ou qui vient de se fermer, répond qu'il n'a pas de console.
- **Hors du lancement** : le même accord que `read_browser`, par agent, origine et navigateur, la même bande et la même demande. Avant l'accord, l'outil répond `{ asked: true }` et rien de la console ; un refus est refusé.
- **Qui** : comme `read_browser` (AB-2), et coupé par le même interrupteur.

## Ouvrir et fermer un onglet

### Ce que fait AB-6 (construit, ESC-678)

- **`open_browser`** ouvre un vrai onglet, une ligne de « Navigateurs » du projet de l'agent :
  - `url` : une adresse `http(s)` ;
  - `launch: true` : la page de son lancement. Celle de son test si elle est ouverte ; sinon, avec un worktree, l'adresse de la première commande de lancement du projet à ses ports (puis celle de sa recette), sous la clé de l'agent ; sans worktree, la ligne de la commande de lancement du projet (`run:<projet>:<commande>`). Avec `url`, une adresse de la même origine ;
  - `artifact` : `"last"` ou l'adresse d'un artefact que l'agent a publié. Escouade la prend dans les résultats de son outil `Artifact` (une publication réussie, jamais une lecture ni le résultat d'un autre outil), dans le journal de sa conversation : `https://claude.ai/artifact/<id>` ou `https://claude.ai/code/artifact/<id>`, sans rien d'autre de `claude.ai`. Sans artefact : « Aucun artefact publié dans ta conversation ». Claude Code seulement ;
  - `place` : `beside` (le défaut), `below`, `full` ou `quiet`.
- **La réponse** : `{ browser, url, place }`. `place` vaut `beside`, `below` ou `full` quand l'utilisateur regarde la conversation de l'agent ; la fenêtre dit au cœur, à chaque changement, quelles conversations sont à l'écran. Sinon `place` vaut `waiting`, avec `why` : la ligne attend, son nom en ambre et « esc-412 veut te montrer la page » en infobulle, et s'ouvre où l'agent la voulait dès que l'utilisateur revient sur sa conversation (ou quand il clique la ligne). `quiet` : la ligne seule, rien d'affiché.
- **Où** : `full` montre l'onglet seul dans la zone principale ; `beside` et `below` l'ouvrent dans un volet à droite ou sous la conversation de l'agent. Un volet par agent : un autre navigateur qu'il pilote quitte son volet (rouvrir le déplace, n'en ajoute pas). Ouvrir une origine où l'agent a déjà un onglet va dans cet onglet.
- **Les adresses** :
  - sans demander : les origines des commandes de lancement du projet, et ce poste aux ports que déclarent les lancements du projet (ses commandes, les tests de ses agents et leurs blocs de ports), jamais au port du serveur MCP d'Escouade ;
  - refusées, jamais demandées, même avec l'accord : un autre schéma que `http(s)` (`file:`, `data:`, `javascript:`, `blob:`, `about:`…), une adresse avec un nom d'utilisateur ou un mot de passe, une adresse d'Escouade (`navigation::destination`), **ce poste à un autre port**, une adresse de plus de 2 048 caractères ;
  - demandées (C3) : toute autre adresse d'internet. L'outil répond `{ place: "asked", origin, message }` et n'ouvre rien. Une bande de la conversation de l'agent, au-dessus du compositeur : « esc-412 veut ouvrir docs.stripe.com (hors du lancement du projet, dans un navigateur d'Escouade) · Refuser / Permettre une fois », la plus ancienne demande d'abord, 5 au plus par agent. « Permettre une fois » ouvre l'adresse à la place voulue, et l'origine est permise dans ce navigateur jusqu'à sa fermeture. « Refuser » tient : l'agent ne peut plus la redemander. La réponse revient à l'agent par un message d'Escouade, avec la clé du navigateur. Redemander avant la réponse n'ajoute pas de demande.
- **Les bornes** : 3 navigateurs pilotés à la fois par agent, 10 ouvertures par heure (les demandes comprises).
- **Le stockage** : l'onglet est dans le stockage du projet (`pages/<projet>`), jamais celui de la webview de la fenêtre : il n'hérite d'aucun de ses cookies.
- **`close_browser`** (`browser`) : un onglet que l'agent a ouvert se ferme et sa ligne part. Sur la page de son lancement, ou une ligne de lancement qu'il pilote, il cesse de la piloter (son état de pilotage tombe) et la ligne reste. Jamais un navigateur ouvert par l'utilisateur ni celui d'un autre agent ; une page d'agent (`page:…`) se ferme par `close_page`.
- **Qui** : les agents d'Escouade, dans leur propre projet ; Claude hors d'Escouade n'ouvre ni ne ferme rien. Un projet privé : ses navigateurs restent inconnus des agents des autres projets (une clé de l'un d'eux reçoit la même réponse qu'une clé qui n'existe pas). « Agents et navigateurs » coupé, les deux outils le disent ; allumé, la consigne du serveur MCP dit d'ouvrir son travail à côté de sa conversation et de le fermer ensuite. `list_browsers` donne `by: "agent"` à un onglet qu'un agent a ouvert.

## Piloter

### Ce que fait AB-7 (construit, ESC-679, avec AP-6)

`src-tauri/src/browserdrive.rs` (les règles, le registre du pilotage, les entrées par système), `browserdrive.js` (la bibliothèque fixe du monde isolé), `src-tauri/src/mcp/browsers_drive.rs` (l'outil).

- **`drive_browser`** (`browser`, `action`, et selon l'action `target`, `url`, `for`, `timeout`, `by`, `label`, `width`, `text`, `submit`, `value`, `key`, `n` ; `wait_user`, `look`), une action par appel, sur toute ligne : un onglet (la page du test de l'agent, `run:…`, `browser:…`) ou une page d'agent (`page:p-<n>`, décrite dans *Les pages des agents*, « Le pilotage d'une page »).
- **Qui** : un agent d'Escouade ; sur un onglet, la page de son test, un onglet qu'il a ouvert ou un lancement qu'il pilote (`open_browser`), jamais celui d'un autre agent (« Ce navigateur n'est pas le tien ») ni un onglet que l'utilisateur a ouvert seul (« L'utilisateur a ouvert ce navigateur : tu ne le pilotes pas », jusqu'à « Laisser piloter par… », AB-9). Claude hors d'Escouade ne pilote rien. Une ligne cachée répond « Le navigateur n'est pas affiché : l'utilisateur ne le voit pas ». `list_browsers` dit `drivable` pour l'appelant, et `hand` (`agent`, `user`, ou rien tant que personne ne l'a pilotée).
- **Les actions et leurs paliers** :

  | Palier | Actions | Lancement du projet | Hors du lancement | Page d'agent |
  | --- | --- | --- | --- | --- |
  | Montrer | `scroll_to`, `scroll` (±5 000 px), `highlight` (`label` de 60 caractères, un contour à la couleur du projet, un seul à la fois), `clear`, `width` (`full`, `tablet`, `mobile` : la barre suit et la largeur est gardée comme si l'utilisateur l'avait choisie), `wait` (`for` : `load` ou `target`, 30 s au plus, 10 par défaut), `reload`, `back`, `forward` | Sans demander | Sans demander | Sans demander, sans historique |
  | Agir | `tab` (un onglet déclaré, `role="tab"`), `click`, `type` (2 000 caractères, ce que le champ tenait remplacé, `submit` presse Entrée), `select` (`value` : la valeur ou le libellé de l'option), `press` (Entrée, Échap, Tab, les flèches, Page↑↓, Début, Fin), `navigate` | Sans demander | Avec l'accord (C4) | `tab` seul |
  | Page d'agent | `version` (`n`) | Refusé | Refusé | Sans demander |

- **Hors du lancement, agir** demande l'accord de lire et d'agir sur l'origine dans ce navigateur, **le même que celui de `read_browser`** (une fois par agent, origine et navigateur, la même bande, retiré par « Retirer les accords ») : l'outil répond `{ asked: true, origin, message }` sans rien faire, l'accord ou le refus revient par un message, un refus tient. Montrer passe sans demander ; l'agent ne reçoit que « trouvé » ou non, l'adresse et le titre, jamais le texte de la page.
- **`navigate`** : une adresse de l'origine du navigateur, ou du lancement du projet (une origine que l'utilisateur a laissé ouvrir dans ce navigateur aussi) ; une autre origine s'ouvre par `open_browser`, qui la demande ; jamais ce que refuse `open_browser` (autres schémas, adresses d'Escouade, ce poste hors des ports déclarés).
- **Les cibles** (`target`) : `#id`, un sélecteur CSS de 200 caractères au plus, sans `:has()` ni plus de quatre niveaux, refermé ; `text=Envoyer`, le premier élément cliquable visible de ce texte (le texte exact, sinon le premier qui le contient) ; sur une page d'agent, une clé de son modèle. Le premier élément visible ; aucun : « Aucun élément visible pour « … » ».
- **Les refus, avant tout geste** :
  - un fichier : cliquer ou taper dans un `<input type="file">` (ou son `<label>`) ;
  - un téléchargement : cliquer un lien `download`, ou vers `blob:`, `data:`, `filesystem:` ;
  - un formulaire vers une autre origine : taper, choisir, cliquer ou presser Entrée dans un champ d'un formulaire dont l'`action` (ou le `formaction` du bouton) va vers une autre origine ;
  - un mot de passe : sur le lancement, sans l'accord de l'utilisateur (C2), l'outil refuse et la bande le lui demande (« veut remplir un mot de passe · Permettre / Refuser »), une fois par navigateur et par agent ; la réponse revient par un message ; un refus tient ; la valeur n'est jamais relue. Hors du lancement, toujours refusé, même avec l'accord : l'utilisateur s'y connecte lui-même ;
  - un secret d'Escouade, quel que soit le champ : une valeur de ses secrets (les jetons du serveur MCP, ceux des comptes des systèmes de tickets et des hébergeurs de code, le mot de passe du proxy), ou ce qui a la forme d'une clé ou d'un jeton ;
  - `type` ailleurs que dans un champ de texte, `select` ailleurs que dans une liste, `tab` ailleurs que sur un onglet déclaré.
- **Les entrées** : sous Windows, de vraies entrées du protocole DevTools, que la page entend comme celles de l'utilisateur (`isTrusted`) : le clic au centre de l'élément (amené dans la vue), `Input.insertText` après avoir donné le focus au champ et sélectionné son texte, `Input.dispatchKeyEvent` pour une touche. Sous macOS et Linux, celles du monde isolé : un clic synthétique, la valeur posée par le *setter* natif avec ses événements `input` et `change`, une touche synthétique ; la réponse porte `synthetic: true`. `select`, `scroll_to`, `scroll`, `highlight` passent partout par le monde isolé.
- **La liste fermée du protocole DevTools** reste celle d'AB-1 ; ses entrées sont vérifiées : `Input.dispatchMouseEvent` pour bouger, presser, relâcher et la molette, en un point de la vue ; `Input.dispatchKeyEvent` pour une touche de la liste, sans modificateur ; `Input.insertText` pour un texte de 2 000 caractères ; `Emulation.setDeviceMetricsOverride` pour une largeur de 200 à 4 000 px. `Runtime.evaluate` n'exécute que les deux bibliothèques d'Escouade (lire, piloter), dans son monde.
- **La règle de la main** : chaque ligne a son état de pilotage, qui tombe avec elle. Les gestes de l'utilisateur dans la page (pointeur pressé, molette, touche, toucher : un événement `isTrusted`) sont comptés : sous Windows par le monde isolé, qui reste d'un appel à l'autre, hors de la fenêtre d'un geste de l'agent et de ses 400 ms ; sous WebKit, dont le monde isolé ne garde rien d'un appel à l'autre (ESC-692), par un relais posé au début de chaque document dans le monde de la page (`browserdrive-gestures.js`, dans les onglets comme dans les pages d'agent), que le monde isolé relève par un événement auquel il répond dans le même appel, comme le relais de console. La page peut fausser ce relais : un geste inventé ne fait que donner la main à l'utilisateur, un geste tu lui laisse « Reprendre la main » ; le cœur les relève à chaque appel et toutes les 500 ms tant que l'agent a la main. Au premier, la main passe à l'utilisateur : chaque appel répond « L'utilisateur a repris la main : il te la rendra par « La rendre », ou écris-lui » sans rien faire. La fenêtre, et AB-8 avec sa bande, a les gestes « Reprendre la main », « La rendre » (l'agent prévenu par un message ; ce que l'utilisateur a fait pendant qu'il avait la main ne la lui reprend pas), « Ne plus laisser piloter » (jusqu'à la fermeture de la ligne, l'agent prévenu) et « Continuer » (commandes `browser_drive_state`, `browser_drive_hand`, `browser_drive_password`). Aucun message ne monte de la page.
- **Les bornes** : 4 gestes par seconde et 600 par heure, par ligne. Dans la seconde, un geste attend son tour (1 s au plus) ; une heure pleine est refusée en disant quand reprendre.
- **La réponse** : `{ done, step, what, url, title, hand }`, plus `scrollY`, `innerWidth`, `found` et `ms` (`wait`) ou `version` selon le geste, sous l'avis de donnée de tiers. `what` est la légende du geste (« Met en évidence « Offre Pro » », jamais le texte tapé) ; chaque geste rend la main quand la page s'est posée (chargée, son défilement immobile), 1,5 s au plus. La fenêtre est prévenue de chaque geste (`devPage`, phase `drive`) et d'une largeur (phase `mode`).
- **`look: true`** joint une image de la vue après le geste, aux bornes de `capture_browser` (20 par minute) ; hors du lancement, seulement avec l'accord de lire, sinon `look` dit pourquoi il n'y a pas d'image.
- **`wait_user: true`** (C1) : après le geste, « Continuer » attend l'utilisateur, 5 minutes au plus ; `user` vaut `continued`, `took_hand` (son geste, « Reprendre la main », la ligne fermée) ou `timeout`.
- **La consigne du serveur MCP** (avec « Agents et navigateurs ») dit de piloter ce qu'on montre par `drive_browser`, un geste par appel en le disant à l'utilisateur, ce qui passe sans demander et ce qui attend son accord, de s'arrêter quand il reprend la main, et de ne jamais taper un secret ni, hors du lancement, un mot de passe.
- **Les interrupteurs** : « Agents et navigateurs » coupé, les onglets sont refusés ; « Pages des agents » coupé, les lignes `page:`. Tous deux coupés, `drive_browser` quitte la liste des outils.

## Version

En 2.2 (décision de l'utilisateur du 2026-11-26) : tout ce qui touche aux navigateurs part de `main`, puis se reverse dans la 2.3. Cela comprend les quatre outils de lecture en tête, le pilotage, les pages des agents et le vrai onglet hors du projet, dans l'ordre de `docs/AGENT-BROWSERS-TICKETS.md`.

Dans la 2.3 seulement (AB-R) :
- la partie « Lancement et navigateurs » d'un projet et l'interrupteur de Fonctionnalités, qui reprend la clé de celui de Réglages › Agents ;
- le refus à distance (`desk-only`) ;
- le téléphone et la fenêtre connectée à un serveur, qui ne voient ni lignes ni accords ;
- l'Overseer ;
- le ticket d'équipe.

## Où en est la construction

- **AB-1 (ESC-668), construit** : le pont de lecture natif (`src-tauri/src/browserread.rs`), sans outil MCP encore. Les scripts fixes d'Escouade dans le monde isolé `escouade-agent` par système, la liste fermée des méthodes du protocole DevTools sous WebView2, les captures (vue, élément, document), les tampons de console et de réseau tenus dès la création de chaque navigateur (500 entrées de chaque, adresses sans chaîne de requête), le masquage du HTML. Ce qu'il fait est décrit dans `kanban/page-du-lancement`, « Ce que le cœur lit d'un navigateur ». La sonde est `src-tauri/examples/agentbrowser_probe.rs` : vérifiée sous Windows (WebView2) et sous WebKitGTK (WSL), et en CI sous Linux et macOS depuis AB-5 (ESC-672) ; sous macOS, sa console est complète depuis ESC-692 (le relais rend son lot à chaque relève du cœur).
- **AB-2 (ESC-669), construit** : `list_browsers` et `read_browser` (`src-tauri/src/mcp/browsers.rs`), l'interrupteur « Agents et navigateurs », la bande d'accord (`ConsentBand`) et « Retirer les accords » ; décrit plus haut, « Ce que fait AB-2 ». Vérifié par les tests Rust des refus et des bornes, Vitest de la bande, et l'e2e `browsers-mcp.spec.ts` (Windows, 1440 × 900 et 1024 × 728).
- **AB-3 (ESC-670), construit** : `capture_browser` ; décrit plus haut, « Ce que fait AB-3 ». Vérifié par les tests Rust (bornes de l'image, débit, largeurs, fichiers, refus sans accord) et l'e2e `browsers-capture.spec.ts` (Windows, 1440 × 900 et 1024 × 728). La largeur par la barre sous WebKit n'a tourné sur aucun Linux ni Mac : AB-5 l'essaie.
- **AB-4 (ESC-671), construit** : `browser_console` (`src-tauri/src/mcp/browsers.rs`) ; décrit plus haut, « Ce que fait AB-4 ». Vérifié par les tests Rust (curseur, instant, niveaux, `limit`, `dropped`, adresses nettoyées, navigateur caché, accord, refus) et l'e2e `browsers-console.spec.ts` (Windows, 1440 × 900 et 1024 × 728).
- **AB-6 (ESC-678), construit** : `open_browser` et `close_browser` (`src-tauri/src/mcp/browsers_open.rs`), la bande « veut ouvrir » de la conversation (`OpenAskBand`), la ligne qui attend dans « Navigateurs » ; décrit plus haut, « Ce que fait AB-6 ». Vérifié par les tests Rust des adresses, des artefacts, des bornes, de qui et de la fermeture, Vitest de la bande et de la ligne, et l'e2e `browsers-open.spec.ts` (Windows, 1440 × 900 et 1024 × 728).
- **AB-7 (ESC-679, avec AP-6), construit** : `drive_browser` ; décrit plus haut, « Ce que fait AB-7 ». Vérifié par les tests Rust (`browserdrive_tests.rs` : paliers, refus, cibles, bornes, la main, `wait_user`, le mot de passe, la liste fermée et ses entrées ; `mcp/browsers_drive_tests.rs` : qui, paliers par mode, accord hors du lancement, secrets, interrupteurs, page d'agent par ses clés), Vitest de la largeur gardée, et l'e2e `agent-browsers-drive.spec.ts` (Windows, 1440 × 900 et 1024 × 728) : un faux agent pilote son lancement (vrai clic entendu `isTrusted`, frappe, liste, onglet, largeur mobile, refus du mot de passe sans accord puis frappe après lui, fichier, téléchargement, formulaire vers une autre origine, secret), `look`, `wait_user` et « Continuer », la main prise par une vraie molette de Windows au-dessus de la page puis rendue, « Ne plus laisser piloter », puis une page d'agent par ses clés. Les entrées synthétiques de WebKit n'ont tourné sur aucun Linux ni Mac : AB-5 et AB-12. Ce que voit l'utilisateur (bande, curseur, surlignage par `Overlay` sous Windows, rangées) vient avec AB-8 ; sous Windows comme sous WebKit, le contour est aujourd'hui un élément d'Escouade posé par le monde isolé, placé dans les coordonnées du document (il défile avec la page sans écouteur) et retrouvé par sa marque : rien n'attend dans le monde isolé d'un appel à l'autre. Vitest `browserDriveGestures.test.ts` rejoue le relais des gestes et sa relève, chaque relève dans un monde neuf.
- Le reste (AB-5, AB-8 à AB-12, DOC) est à construire.
