---
title: "Les navigateurs pilotés par les agents (brouillon, ESC-663)"
subject: "kanban"
sources: ["docs/AGENT-BROWSERS.md","design/agent-browsers/probe/src/main.rs","design/agent-browsers/results/report-windows.json","design/agent-browsers/results/report-linux.json","design/agent-browsers/maquette/index.html","design/agent-browsers/maquette/cas-phare-clair.png","design/agent-browsers/maquette/cas-phare-sombre.png","src-tauri/src/devpage.rs","docs/AGENT-PAGES.md (ticket/esc-661)"]
tickets: ["ESC-663","ESC-661","ESC-657","ESC-658"]
decisions: [324,325,323,326,327,322]
compared: null
draft: true
---

# Les navigateurs pilotés par les agents (brouillon, ESC-663)

> **Brouillon** : l'étude ESC-663 le propose, rien n'est construit. Les choix de l'utilisateur sont tranchés (2026-11-25) : l'agent choisit son rythme geste par geste (C1), un mot de passe se tape avec son accord une fois par navigateur (C2), une autre adresse lui est demandée une fois par origine (C3). La conception complète est dans `docs/AGENT-BROWSERS.md`.

## Ce que c'est

Un agent ouvre un navigateur de la section « Navigateurs » **à côté de sa conversation** et le **pilote** en expliquant : il fait défiler, met en évidence, ouvre un onglet de la page, change de largeur, clique ou tape dans l'app qu'il développe. L'utilisateur regarde, reprend la main d'un geste et répond. Les pages des agents (ESC-661) sont des lignes de « Navigateurs », pilotées par le même outil.

## Les outils (serveur MCP d'Escouade, du poste seulement)

- `list_browsers` : les lignes du projet, qui les pilote, qui a la main.
- `open_browser` (`url` ou `launch`, `place` : `beside`, `below`, `full` ou `quiet`) : à côté de la conversation de l'agent si l'utilisateur la regarde, sinon la ligne attend (`waiting`).
- `drive_browser` (`browser`, `action`) : `navigate`, `back`, `forward`, `reload`, `wait`, `scroll_to`, `scroll`, `highlight`, `clear`, `width`, `tab`, `click`, `type`, `select`, `press` ; `look` joint une capture ; `wait_user` attend « Continuer ». Une page d'agent n'accepte que `scroll_to`, `highlight`, `clear`, `width` et `tab`.
- `capture_browser` : une image (JPEG, 1 600 px et 600 Ko au plus), la page, un élément ou le document, à une largeur.
- `read_browser` : le texte visible, l'arbre d'accessibilité ou un extrait du DOM, bornés (20 000 caractères), marqués comme donnée de tiers.
- `browser_console` : la console et les erreurs réseau depuis un curseur (500 entrées par navigateur).
- `close_browser` : arrête le pilotage ; ferme un navigateur que l'agent a ouvert.

Ils appartiennent à la partie « Lancement et navigateurs » du projet. Un interrupteur de Fonctionnalités les coupe tous : « Agents qui pilotent les navigateurs ».

## Ce que voit l'utilisateur

- La ligne pilotée porte l'avatar de l'agent et « piloté ». Son menu propose « Reprendre la main » et « Ne plus laisser piloter ».
- Une bande native, sous la barre du navigateur, porte « Piloté par <agent> », la légende du dernier geste, « Reprendre la main » et « Continuer ».
- Un curseur fantôme et un contour avec son étiquette montrent où l'agent agit.
- Une rangée numérotée par geste apparaît dans la conversation.
- Le rythme : chaque geste rend la main à l'agent quand la page s'est posée (1,5 s au plus). L'agent avance seul, sauf là où il passe `wait_user` : la bande montre « Continuer » et l'outil attend ce clic, 5 minutes au plus (`continued`, `took_hand` ou `timeout`).
- Le premier geste de l'utilisateur dans la page, ou le bouton, suspend le pilotage. L'outil le dit à l'agent, sans rien faire, jusqu'à « La rendre ».

## Les bornes

- Origines : celles des lancements du projet et de ses worktrees, `localhost` à ses ports déclarés, ses pages d'agent. Une autre adresse `http(s)` est demandée à l'utilisateur, une fois par origine et par navigateur (« Permettre une fois / Refuser ») ; l'accord ne vaut que pour ce navigateur. Jamais d'adresse d'Escouade, de `file:` ni d'autre schéma.
- Jamais vers l'agent : cookies, stockage, en-têtes, corps des réponses, valeur d'un mot de passe (`«masqué»`).
- Pas de passe-plat vers le protocole DevTools : une liste fermée de méthodes.
- Refusés : taper dans un mot de passe sans l'accord de l'utilisateur (une bande « Permettre / Refuser », une fois par navigateur ; la valeur n'est jamais relue), envoyer un fichier, télécharger, taper un secret d'Escouade.
- Qui pilote : l'agent sur ses navigateurs et sur ceux que l'utilisateur lui a donnés. L'Overseer lit et capture ceux des agents de son projet, sans les piloter. Une discussion a ses navigateurs à elle. Claude hors Escouade et un autre projet n'ont rien. Dans un projet privé, seuls ses agents voient ses navigateurs.
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

- **La page de l'agent** (ESC-661, `show_page`) : une webview liée à l'agent, dont il fait le contenu, cloisonnée, sans stockage. Ses choix lui reviennent par `ask_user`. `drive_browser` n'y fait que montrer.
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

## Version

En 2.2 (décision de l'utilisateur du 2026-11-26) : tout ce qui touche aux navigateurs part de `main`, puis se reverse dans la 2.3. Cela comprend les quatre outils de lecture en tête, le pilotage, les pages des agents et le vrai onglet hors du projet. En 2.2, un réglage « Agents et navigateurs » des navigateurs coupe les outils.

Dans la 2.3 seulement :
- la partie « Lancement et navigateurs » d'un projet et l'interrupteur de Fonctionnalités, qui reprend la clé du réglage ;
- le refus à distance (`desk-only`) ;
- le téléphone et la fenêtre connectée à un serveur, qui ne voient ni lignes ni accords ;
- le ticket d'équipe.
