---
title: "Les navigateurs (la page du lancement)"
subject: "kanban"
sources: ["design/browser/INVENTAIRE.md","design/browser/a-onglet.png","design/browser/a-tablette.png","design/browser/a-mobile.png","design/browser/a-retreci.png","design/browser/etats.png","design/split-view/index.html","website/content/docs/fr/browser.md","src/lib/devpage.svelte.ts","src/components/BrowsersSection.svelte","src/components/DevPage.svelte","src-tauri/src/devpage.rs","src-tauri/src/browserread.rs","src-tauri/src/browserread.js","src-tauri/src/browserread-relay.js","src-tauri/src/agentbrowser_probe.rs","docs/AGENT-BROWSERS.md","docs/AGENT-BROWSERS-TICKETS.md"]
tickets: ["ESC-311","ESC-419","ESC-560","ESC-657","ESC-668"]
decisions: [26,149,294,310,"le-cœur-lit-un-navigateur-dans-un-monde"]
compared: "2026-10-31T19:02:21+01:00"
---

# Les navigateurs (la page du lancement)

## Ce que c'est

Des navigateurs dans Escouade, comme il a des terminaux (ESC-657) : la section « Navigateurs » de la barre latérale, sous « Terminaux », a une ligne par onglet. On y ouvre l'app développée à côté des agents qui la codent, avec ses outils de développement et trois largeurs, sans changer de fenêtre. Un navigateur s'ouvre seul, par le « + » de la section, ou de lui-même : la page du test d'un agent quand « ▶ Tester » est prêt (ESC-311), celle d'une commande de lancement qui a une adresse à ouvrir (ESC-560). La fenêtre « Pages » n'existe plus.

## Le modèle

- Un navigateur est un onglet d'un projet : son adresse, son nom (le titre du document, sinon son hôte ; pour un lancement « agent · processus »), l'état de sa page (charge, chargée, ne répond pas), et ce qui l'a ouvert : l'agent dont le test l'a ouvert, la commande de lancement, ou l'utilisateur.
- Sa clé : l'id de l'agent pour la page de son test (une par agent : relancer recharge la même page) ; `run:<projet>:<commande>` pour une commande lancée dans le projet (une par commande) ; `browser:<id>` (lettres, chiffres, `-`, `_`, 40 au plus) pour un navigateur ouvert seul.
- Il vit tant qu'Escouade tourne, comme un terminal : ni l'un ni l'autre ne revient après un redémarrage. La largeur choisie est gardée par nom de page, sur la machine.
- Le titre du document vient de la page, qui n'est pas digne de confiance : une ligne, sans caractère de contrôle ni invisible, 120 caractères au plus, affiché comme texte.

## La section « Navigateurs »

Sur le modèle exact de « Terminaux » (mêmes lignes, mêmes gestes) :

- En tête : « Navigateurs », leur nombre, « + » (« Nouveau navigateur »).
- Une ligne par navigateur, dans l'ordre où ils se sont ouverts : son point (ambre : charge ; vert : chargée ; rouge : ne répond pas, dit aussi aux lecteurs d'écran), son nom (l'adresse en infobulle), la marque du worktree pour la page d'un lancement, sa croix au survol (« Fermer le navigateur »).
- Un clic l'affiche seul dans la zone principale (plein écran), comme un terminal ; `Ctrl`+clic (`⌘`) l'ouvre à côté de ce qui est affiché (demi-écran) ; glissée sur la zone de travail, elle va dans un volet par la boussole (voir *Écran partagé*). Sélectionnée, ou entourée quand elle est dans un autre volet que l'actif.
- Son menu : « Ouvrir à côté », « Ouvrir en dessous », « Recharger », « Copier l'adresse », « Ouvrir dans le navigateur du système », « Fermer le navigateur ». « Ouvrir à côté » et « Ouvrir en dessous » ne sont pas offerts pour un navigateur ouvert seul affiché seul (il n'a rien à côté de quoi aller).
- « + » ouvre en tête des lignes un champ « Adresse du nouveau navigateur » (`localhost:3000`) : `Entrée` ouvre le navigateur et l'affiche, `Échap` renonce, une adresse refusée garde le champ et un toast dit pourquoi.
- Vide : « Aucun navigateur ouvert ». Dans une fenêtre basse, la section se replie sur son titre comme « Terminaux ».

## Ce qui ouvre un navigateur

- Le « + » de la section.
- Le test d'un agent prêt : sa ligne. Si l'utilisateur regardait l'agent (sans terminal ni journal par-dessus) ou sa fenêtre « Tester », Escouade ferme cette fenêtre et affiche la ligne ; sinon la ligne attend. Elle se ferme avec l'agent et quand son ticket passe « Terminé ».
- Une commande de lancement qui a une adresse (`url`, `open_in: page`) : lancée dans le worktree d'un agent, la ligne de l'agent ; lancée dans le projet, sa ligne à elle (« <commande> »), rechargée à chaque lancement.
- Réglage par projet, gardé sur la machine : « Ouvrir les pages des lancements » = « Dans Escouade » (la ligne, par défaut) ou « Dans le navigateur du système ». La valeur d'avant « Dans une fenêtre à part » (`window`) est lue comme « Dans Escouade » : elle ouvre la ligne. `open_in: browser` d'une commande l'envoie au navigateur du système.
- « Page », dans l'en-tête de la conversation d'un agent dont le test a ouvert une page : affiche sa ligne ; glissé sur la zone de travail, la met dans un volet à côté de la conversation. La vue « Page » dans la conversation n'existe plus : la ligne la remplace.
- « Rouvrir » de la fenêtre « Tester » la rouvre là où elle s'était ouverte ; la ligne dit « Ouvert dans Escouade, sa ligne de Navigateurs : … » ou « Ouvert dans le navigateur : … ».

## La vue d'un navigateur

- Sa barre : Retour, Avancer, Recharger (boutons d'icône), l'adresse (champ de l'app, modifiable ; sans schéma, celui de la page ; le point de l'état devant), l'étiquette « agent · processus » pour la page d'un lancement (glissée, elle met la page dans un volet), puis à côté de l'adresse les trois largeurs (segments) et Devtools, puis « Ouvrir dans le navigateur du système » et « … ».
- « … » : « Ouvrir à côté », « Ouvrir en dessous » (à côté de la conversation de son agent quand la page d'un lancement est affichée seule ; rien pour un navigateur ouvert seul affiché seul), « Fermer le volet » dans un volet, « Copier l'adresse ». La barre se replie sur sa largeur : étroite, Avancer, l'étiquette et les largeurs passent dans « … », Devtools garde son icône ; l'étiquette de largeur reste.
- États : chargement (point ambre, filet) ; page qui ne répond pas (chargement non fini en 20 s, compté dès l'ouverture : « <hôte> ne répond pas », Recharger ; « Voir les logs » pour la page d'un lancement) ; pour la page d'un test, lancement qui redémarre (« Le lancement démarre… ») et lancement arrêté (« Le lancement s'est arrêté », Relancer, Voir les logs). Textes en français et en anglais.
- La page est une webview native posée sur la fenêtre : elle se masque tant qu'un menu, une liste, un dialogue ou la palette la croise, tant qu'un contenu est glissé sur la zone de travail, et quand sa vue n'est pas affichée. Dans un volet, elle suit sa place et sa taille ; avec deux pages à l'écran, F12 et Ctrl+Maj+M vont à celle du volet actif.

## Devtools

- Ils ne s'ouvrent que sur un geste de l'utilisateur : le bouton Devtools ; `F12`, `Ctrl+Maj+I` (`⌥⌘I` sous macOS) quand le focus est dans Escouade ; dans la page, les gestes du système (le menu « Inspecter l'élément », `F12` de WebView2). La page ne peut pas les ouvrir elle-même : le raccourci injecté dans la page qui naviguait vers `escouade-devtools://open/` est retiré (un script de la page pouvait y aller seul) ; cette adresse est refusée comme tout autre schéma. Ceux de la fenêtre d'Escouade restent coupés dans une version publiée.
- Où ils se logent, vérifié par système (ESC-657) :

| Système | Où | Pourquoi |
|---|---|---|
| Windows (WebView2) | Fenêtre à part | `OpenDevToolsWindow` est la seule API ; les loger dans l'app demanderait un port de débogage distant (`--remote-debugging-port`), ouvert à tout programme de la machine : refusé, la sécurité ne s'élargit pas. Vérifié dans l'app (e2e `devpage.spec.ts`, capture). |
| macOS (WKWebView) | Fenêtre à part | L'inspecteur de WebKit sait s'attacher (en bas, à droite), mais attaché il recadre lui-même la vue de la page et lui rend toute la largeur de la fenêtre (vu par `devpage_probe` en CI) : les largeurs tablette et mobile et la place dans un volet ne tiennent plus. Il est détaché dans sa fenêtre (`_WKInspector detach`). Non revérifié sur un Mac dans ce ticket. |
| Linux (WebKitGTK) | Dans la page, en bas ; à droite ou à part par les boutons de l'inspecteur | `inspector.show()` attache l'inspecteur à la vue de la page quand il le peut ; le côté se choisit par les boutons de l'inspecteur (pas d'API publique pour le côté). Non revérifié sous Linux dans ce ticket. |

- La vue le dit : l'infobulle du bouton Devtools, et sa description pour les lecteurs d'écran (« Dans leur propre fenêtre : WebView2 ne les loge pas dans la page. », etc.).

## Largeurs

- Pleine largeur ; Tablette 768 px ; Mobile 390 px, centrée sur fond neutre avec un filet, hauteur de la zone ; largeur affichée (« 768 px »). Zone plus étroite : la page rétrécit à la zone et le dit (« Rétréci à 692 px »).
- `Ctrl+Maj+M` (`⇧⌘M`) passe à la suivante, depuis la page ou depuis Escouade.
- Gardée par nom de page d'un lancement à l'autre.
- Seule la largeur change (les media queries). User agent, `devicePixelRatio` et tactile ne sont pas émulés ; les Devtools gardent leur propre mode appareil.

## Isolation, sans exception

- Webview à part : aucune commande d'Escouade ni d'un plugin ne lui répond (appel d'une webview `devpage-*` refusé ; la capacité ne nomme que les webviews de la fenêtre).
- Stockage du projet, jamais celui de la fenêtre : dossier `pages/<projet>` des données (Windows, Linux), magasin par projet (macOS 14+), session privée avant.
- Adresses, les mêmes règles qu'une page de lancement, rien de plus large : seule une adresse `http` ou `https` avec un hôte s'ouvre (tapée dans « + », venue d'une commande ou d'un test), jamais une adresse d'Escouade, un fichier ou un autre schéma (`launch_url`, et son miroir dans la fenêtre) ; la page reste sur l'origine de son adresse de départ : une autre origine (tapée dans la barre ou suivie par un lien) s'ouvre dans le navigateur du système, avec un message ; adresses d'Escouade, fichiers et autres schémas → nulle part.

## Ce que le cœur lit d'un navigateur (ESC-668)

Le pont de lecture natif (`browserread`) prépare les outils des agents qui liront les navigateurs (`list_browsers`, `read_browser`, `capture_browser`, `browser_console`, à venir : voir `docs/AGENT-BROWSERS.md`). Il ne change rien à ce que voit l'utilisateur, et la page n'a toujours aucun canal vers Escouade.

- **Un monde à Escouade** : ce que le cœur lit d'une page passe par la poignée native de sa webview, dans un monde isolé `escouade-agent` du cadre principal, que les scripts de la page ne touchent pas (une page qui réécrit `JSON.stringify` ou `querySelector` ne fausse pas la lecture). Seuls des scripts fixes d'Escouade, compilés dans l'app et paramétrés en JSON, y tournent ; jamais le monde de la page (`eval`), jamais un script d'un agent. WebView2 : `Page.createIsolatedWorld` et `Runtime.evaluate` ; WebKitGTK : `run_javascript_in_world` ; WKWebView : `WKContentWorld`.
- **Sous Windows, une liste fermée** des méthodes du protocole DevTools, appelé dans le process, sans port de débogage : les captures, le monde isolé, et pour le pilotage à venir les entrées, l'overlay, l'émulation de largeur, l'arbre d'accessibilité, la navigation vers une adresse web. Toute autre méthode est refusée (ni cookies, ni stockage, ni en-têtes, ni corps de réponse), et `Runtime.evaluate` ne vaut que pour le monde d'Escouade et ses scripts.
- **La console et le réseau, tenus dès la création** de chaque navigateur, qu'un agent les lise ou non, rouvert ou rechargé compris : 500 entrées de console (texte de 500 caractères au plus) et 500 requêtes en échec (statut 400 ou plus, ou sans réponse), les plus anciennes perdues d'abord et comptées. Les adresses y sont gardées sans chaîne de requête, fragment ni identifiants (`data:`, `blob:` réduits à leur schéma). Fermé, le navigateur emporte ses tampons.
  - Windows : les événements du protocole (`Runtime`, `Log`, `Network`), activés à la création de la webview ; la page ne peut ni les taire ni les inventer (« rapporté par le navigateur »).
  - Linux et macOS : WebKit n'en donne rien. Un relais posé au début de chaque document, dans le monde de la page, enveloppe `console.*`, `onerror`, `unhandledrejection`, `fetch` et XHR ; il garde ce qui précède la première relève et le dit au monde isolé, où le cœur le relève chaque seconde. La page peut le taire ou le tromper : marqué « rapporté par la page ». C'est, avec le raccourci des largeurs, le seul script ajouté à la page.
- **Une lecture du HTML est masquée** : la valeur des champs mot de passe et cachés, le `content` d'un `<meta>` nommé pour un jeton (`csrf`, `xsrf`, `token`, `nonce`), le texte des scripts en ligne, et les jetons visibles (clés GitHub, GitLab, OpenAI, Anthropic, Slack, AWS, un JWT, un `Bearer`, un bloc de clé privée) deviennent `«masqué»` et sont comptés. Elle se coupe à une borne (50 000 caractères par défaut, 200 000 au plus), après la dernière balise entière. Le texte visible (`innerText`) ne contient pas ce que la page cache. Un sélecteur fait 200 caractères au plus, sans `:has()` ni plus de quatre niveaux.
- **Les captures**, par système : la vue, un élément (ce qui en est visible), le document entier (8 000 px de haut au plus), 1 600 px au plus sur le grand côté. WebView2 : JPEG de qualité 70, et une autre largeur émulée le temps de la capture. WebKitGTK : PNG, le document entier compris ; une autre largeur passe par les largeurs de la barre. WKWebView : PNG de la vue seule (le document entier est refusé et dit).

## Vérifié par système

- `devpage_probe` (vraie webview) en CI sous Linux (Xvfb) et macOS ; les e2e `devpage.spec.ts`, `devpage-pane.spec.ts` et `browsers.spec.ts` (un navigateur ouvert seul, plein écran, demi-écran, largeur mobile vue par la page, fermeture ; une adresse refusée) sous Windows.
- `agentbrowser_probe` (vraie webview, ESC-668), contre une page qui se défend (globales réécrites, consigne cachée, cookie, jeton, champs mot de passe et caché, clé dans le texte) : lecture du HTML et du texte, sélecteur, captures, console, exception, 404, port refusé et navigation, lancée sous Windows (WebView2) et sous WebKitGTK (WSL). Sous macOS, la branche WKWebView compile (clippy pour `x86_64-apple-darwin`) mais n'a tourné sur aucun Mac : son premier essai réel est la CI Mac d'AB-5.
