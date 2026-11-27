# Étude : le client navigateur servi par `escouade-server` (ESC-655, 2026-11-26)

**La question.** Un utilisateur, OshOEz, a écrit sur un fork de notre v2.2.7 (`https://github.com/OshOEz/escouade`, branche `feat/serveur`, 57 commits sur `f29711a5`) un mode serveur où `escouade-server` sert l'interface dans un navigateur. Notre `release/2.3` a la fondation (le serveur, l'API distante, l'appairage, le client de bureau par SSH, le téléphone, l'image Docker, `install.sh`), mais pas ce client navigateur. Que prend-on de son travail, que fait-on autrement, et pourquoi ?

**Comment cette étude a été faite.** Le fork a été lu comme une donnée, dans un clone nu sans checkout (`git show` de fichiers nommés), sans rien construire ni lancer : ni build, ni script, ni test. Rien de son code n'est repris ici. Les fichiers lus : `website/content/docs/fr/server-mode.md`, `vite.web.config.ts`, `src/lib/platform.ts`, `src/lib/transport/{index,device}.ts`, `src/lib/notifier.ts`, `src-tauri/src/remoteapi/{server,web,files}.rs`, `src-tauri/src/headless_host.rs`, `src-tauri/src/browse.rs`, `src-tauri/build.rs`, `src/components/plugins/PluginsView.web.test.ts`, et son audit (`docs/security/audit-mantis-2026-11-serveur/README.md`, `rapport-mantis.md`). Notre côté : `docs/REMOTE.md`, `docs/MOBILE.md`, `src/lib/transport.ts`, `src-tauri/src/remoteapi/`, `src-tauri/src/server/`, `deploy/`, et la spec (« Escouade à distance » : *Le mode serveur*, *L'API distante*, *Ce qui change de machine*).

**En bref.**
- Son client navigateur tient debout et son audit a confirmé que la surface réseau (Host, Origin, DNS rebinding, CORS) tenait. Mais il est écrit sur un autre découpage (`remoteapi/` refait, transport à lui, fichiers par des routes HTTP), sur la ligne 2.2 : il ne se fusionne pas, et ce n'est pas souhaitable.
- Chez nous, l'essentiel est déjà là : `RemoteTransport` (que l'app mobile réutilise déjà), l'appairage par la WebSocket, les corps bruts, la reprise, les bornes, les dossiers du serveur, les fichiers reçus, les notifications rendues par le client. **Le client navigateur est surtout un troisième « poste »** : ce que la fenêtre de bureau fait par ses commandes `desk` (enregistrer un fichier reçu, notifier, lire le presse-papiers, relayer un port), le navigateur le fait avec ses propres moyens, ou ne le fait pas et le dit.
- **On prend ses idées**, pas son code : une build web qui écarte Tauri par la configuration de Vite, la page servie par le serveur avec des en-têtes stricts, l'origine de la page acceptée quand elle est celle du `Host`, les notifications du navigateur demandées au premier geste, `noreferrer` partout, jamais « Ouvrir » un exécutable, la socket Unix dans un dossier `0700`, `RestartPreventExitStatus`.
- **On fait autrement** : la page embarquée dans le binaire (pas `--web <dossier>`), le jeton jamais dans un en-tête ni dans le `localStorage` (une clé WebCrypto non extractible, si l'utilisateur la choisit), l'appairage et les fichiers par la WebSocket (pas de routes HTTP en plus), la CSP de la fenêtre avec son nonce de styles, le client navigateur éteint tant qu'on ne l'allume pas.
- **9 tickets proposés, environ 14 h d'agent en tout (15 h 45 avec la clé WebCrypto), 9 h 30 à 11 h de bout en bout avec 6 agents en parallèle** : la chaîne BC-1, BC-2, BC-3, BC-7 ne se parallélise pas (§ 9). Quatre choix reviennent à l'utilisateur (§ 10), laissés dans sa boîte « À toi ».

## 1. La build web de l'interface, sans Tauri

**Chez lui.** Une façade `platform.ts` : ce que l'interface demande au système (dialogue de fichier, ouvrir un lien, la fenêtre, le zoom, les événements des fenêtres) passe par elle ; dans un navigateur, des bouchons (la fenêtre n'est jamais maximisée, le zoom est celui du navigateur, `open` ne choisit rien, `pickFile` par un `<input type=file>`). Un `transport/index.ts` choisit l'IPC de Tauri si `__TAURI_INTERNALS__` est là, sinon la WebSocket de la page. `vite.web.config.ts` fusionne la configuration du bureau et remplace, au chargement, ses deux modules Tauri (`transport/tauri.ts`, `platform-tauri.ts`) par `export const … = null`, dans `dist-web/`.

**Chez nous (release/2.3).**
- `src/lib/transport.ts` a déjà les deux transports aux signatures de Tauri (`invoke`, `Channel`) ; `transport-only.test.ts` vérifie qu'aucun autre module n'importe `@tauri-apps/api/core`. La WebSocket est choisie quand la page reçoit son lien (`window.__ESCOUADE_REMOTE__`, ou `server_link` du cœur du poste).
- `RemoteTransport` est partagé : l'app mobile (`apps/mobile`, Capacitor) le prend par l'alias `$shared` et a sa propre interface ; elle ne charge pas celle du bureau.
- Restent liés à Tauri, hors de `transport.ts`, quinze modules (les dialogues natifs, le navigateur du système, la fenêtre et sa WebView, les événements de `PagesWindow`) : la liste de `docs/REMOTE.md`, fondation 2b.
- Les commandes du poste (`desk`) sont faites par l'IPC de la fenêtre : un navigateur n'en a pas.

**Ce qu'on prend.**
- **La build web par la configuration de Vite**, un `vite.web.config.ts` à nous : les modules `@tauri-apps/*` remplacés par des bouchons qui **refusent** (une erreur nommée, jamais un `null` qui ferait croire que l'appel a marché), et un test qui vérifie que le paquet construit ne contient ni `__TAURI_INTERNALS__` ni `ipc.localhost`.
- **Une façade du système**, mais à partir de la nôtre : `transport.ts` reste le seul accès au cœur ; une façade `poste` réunit ce que font aujourd'hui les commandes `desk` et les quinze modules Tauri, avec trois réalisations (la fenêtre, le navigateur, les tests). Le test `transport-only` s'étend à `@tauri-apps/*` entier.

**Ce qu'on fait autrement, et pourquoi.**
- **Le transport ne change pas.** Le sien est réécrit (`transport/ws.ts`, ses propres trames) ; le nôtre a la session, la reprise des terminaux numérotée, les corps bruts, les bornes et les refus nommés, déjà éprouvés par l'e2e `remote-cut` et l'app mobile. Le navigateur prend `RemoteTransport` tel quel, son lien tiré de l'adresse de la page (`ws(s)://<location.host>/remote`).
- **Le choix du transport par une marque de la page servie** (une balise posée par le serveur dans `index.html`), pas par l'absence de `__TAURI_INTERNALS__` : une page de bureau dont l'IPC n'est pas encore là ne doit jamais se croire dans un navigateur.

## 2. Le jeton d'un appareil dans le navigateur

**Chez lui.** Le jeton est rendu par `POST /pair`, rangé dans le `localStorage` de l'origine (`escouade-device`), présenté par le sous-protocole de la WebSocket (`escouade, bearer.<jeton>` ; le serveur répond `escouade`, jamais le jeton) et par `Authorization: Bearer` sur ses routes HTTP. **Jamais un cookie**, à raison : un navigateur envoie les cookies de `127.0.0.1` à tous ses ports, donc à tout autre serveur local. Sa page le dit : « Garde ce port local à Escouade seul : tout autre programme servi plus tard sur ce port pourrait lire le jeton. » Le jeton ne s'oublie que quand le serveur le refuse (`GET /device` en 401). Son audit a écarté « pas de déconnexion depuis le navigateur » comme un choix documenté.

**Chez nous.** Le jeton est donné dans le premier message de la WebSocket (`{"t":"hello","token",…}`), jamais dans un en-tête : le navigateur n'a donc pas besoin du sous-protocole, et le jeton n'est jamais dans la tête de la requête HTTP, que voit et peut journaliser un proxy. Sur le bureau, il est au trousseau du poste (`server-token-<id>`).

**Ce que le `localStorage` expose.**
- Toute page servie **plus tard sur la même origine** (même schéma, même hôte, même port) : un autre `ssh -L` sur 8765, le serveur de dev d'un agent relayé sur ce port, un autre outil. Elle lit le jeton et l'emporte : il vaut un terminal sur le serveur, sans date de fin.
- Les extensions du navigateur qui ont accès aux pages, et une faille XSS dans l'interface (la même que sur le bureau, où elle aurait l'IPC ; mais là, le jeton part et sert ailleurs, longtemps).
- Le profil du navigateur sur le disque, en clair.

**Ce qu'on fait autrement.**
- **Recommandé : une clé d'appareil WebCrypto non extractible** (ECDSA P-256, `extractable: false`, gardée dans IndexedDB). À l'appairage, le navigateur envoie sa clé publique avec le code ; à chaque connexion, le serveur envoie un défi et le navigateur le signe. Une page de la même origine peut **se servir** de la clé tant qu'elle est ouverte, mais ne peut **ni la lire ni l'emporter** : une fuite ne dure pas plus que la page. C'est le modèle des appareils du serveur d'équipe (une clé par appareil, Ed25519 ; WebCrypto n'a Ed25519 que dans les navigateurs récents, d'où P-256).
- Un bouton « Oublier ce serveur » dans le navigateur (la clé effacée et l'appareil révoqué, comme `unpair` du bureau, S15).
- L'appareil du navigateur nommé comme tel dans `escouade-server devices` (« Navigateur · Firefox · 127.0.0.1:8765 »), pour qu'on sache lequel révoquer.
- Le choix entre la clé et le jeton du fork revient à l'utilisateur (§ 10, B2).

## 3. La page servie, le service systemd, l'image Docker

**Chez lui.** `escouade-server --web <dossier>` sert `dist-web/` (des noms simples, sans point en tête, aucun lien qui sort du dossier ; `assets/` gardé un an, le reste `no-cache`), avec une CSP (`script-src 'self' 'wasm-unsafe-eval'`, `style-src 'self' 'unsafe-inline'`, `connect-src` de la page et des origines déclarées, `frame-ancestors 'none'`), `nosniff` et `Referrer-Policy: no-referrer`. `escouade-server install` écrit l'unité systemd de l'utilisateur depuis le binaire et dit les commandes à lancer (`daemon-reload`, `enable --now`, `loginctl enable-linger`) sans rien lancer ; `KillMode=mixed`, `Restart=on-failure`, `RestartPreventExitStatus=2 3` (des arguments faux, ou le dossier de données d'un autre serveur : redémarrer n'y changerait rien) ; un seul serveur par dossier de données (verrou). La graine des plugins est embarquée dans le binaire par `build.rs`, parce que son serveur se construit depuis les sources, sans archive.

**Chez nous.**
- `install.sh` (dans chaque release) télécharge l'archive, vérifie sa SHA-256 et sa signature, pose un service de l'utilisateur (avec linger) ou du système (`--system`, utilisateur `escouade`), `Restart=always`, `KillMode=mixed`, `TimeoutStopSec=90`, `UMask=0077`, `NoNewPrivileges` (et `ProtectSystem=full`, `PrivateTmp` pour le système) ; idempotent, `--uninstall`, `--purge`. `escouade-server update` met à jour par la même signature. L'image Docker tourne en lecture seule, sans capacité, `tini` en PID 1. La graine des plugins est dans l'archive et l'image, à côté du programme. Le verrou du dossier de données existe déjà (`headless::InstanceLock`). L'arrêt par SIGTERM (`Core::shutdown`) aussi.
- L'API ne sert **aucune page** ; elle refuse toute `Origin` hormis celles de la fenêtre et du téléphone.

**Ce qu'on prend.**
- **`RestartPreventExitStatus`** : un code de sortie à lui pour « le dossier est tenu par un autre serveur » et pour « arguments faux », que l'unité d'`install.sh` ne relance pas. Aujourd'hui, ces deux cas sortent en 1 ou 2 et `Restart=always` les relance toutes les 3 s jusqu'à la limite de démarrage de systemd.
- **Les en-têtes de la page** : `nosniff`, `Referrer-Policy: no-referrer`, `frame-ancestors 'none'`, `Cache-Control` immuable pour `assets/` (noms tirés du contenu) et `no-cache` pour `index.html`. On ajoute `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-origin` et une `Permissions-Policy` qui ferme le micro, la caméra et la géolocalisation.

**Ce qu'on fait autrement, et pourquoi.**
- **La page dans le binaire**, pas dans un dossier : `build.rs` embarque `dist-web/` compressé (la technique de sa graine, appliquée à la page). Un seul fichier signé, mis à jour d'un coup par `update` et par l'image : la page et le serveur ont toujours la même version ; aucun chemin du disque n'est servi, donc rien à confiner (ses règles de `web.rs` deviennent sans objet). Environ 10 à 15 Mo de plus au binaire, à mesurer.
- **La CSP de la fenêtre**, pas une plus faible : `style-src 'self' 'nonce-…'` et `style-src-attr 'unsafe-inline'` (SEC-05, ESC-593). Le serveur tire un nonce par réponse et le pose sur la balise `<style id="escouade-style-nonce">` d'`index.html`, comme Tauri le fait pour la fenêtre (`lib/style-nonce.ts` le partage ensuite). Son audit a noté la CSP plus faible de sa page (écarté : `script-src` reste strict) ; nous n'avons pas de raison de l'affaiblir.
- **Pas de `install` dans le binaire** : `install.sh` le fait déjà, avec la signature vérifiée avant de rien poser ; un serveur construit depuis les sources prend `install.sh --archive <fichier>` (à ajouter, petit). La graine reste dans l'archive.
- **Le client navigateur éteint par défaut**, allumé par `--browser` (`ESCOUADE_SERVER_BROWSER=1`, `install.sh --browser`) : il ouvre une surface de plus (une page, une origine acceptée), que la plupart des serveurs, pilotés par le bureau ou le téléphone, n'ont pas besoin d'offrir. À trancher (§ 10, B1).
- **L'image Docker** : rien ne change ; avec `ESCOUADE_SERVER_BROWSER=1`, la page est servie sur le même port, derrière le tunnel ou le proxy.

## 4. La socket Unix, `ssh -L` vers elle, `--origin` pour un proxy HTTPS

**Chez lui.** `--socket <chemin>` à la place du port : le dossier créé en `0700`, refusé s'il n'est pas à l'utilisateur ou s'il est ouvert à d'autres, **chaque dossier au-dessus vérifié** (à l'utilisateur ou à root, jamais modifiable par d'autres sauf un dossier collant de root), un verrou `<socket>.lock`, une socket morte remplacée, une vivante jamais, la socket en `0600`. Par défaut, l'unité écoute sur `%t/escouade/server.sock` (`$XDG_RUNTIME_DIR`). `pair` donne la ligne `ssh -N -L 8765:<socket> <hôte>`. Sur une socket, tout port de la boucle locale est celui d'un tunnel. `--origin https://nom` déclare une origine dont le `Host` et l'`Origin` sont acceptés ; `http://` hors de la boucle locale est permis avec un avertissement. Sa page de doc traite les échecs réels du tunnel (VS Code Remote-SSH qui prend le port, les plages réservées de Windows, `AllowStreamLocalForwarding no`).

**Chez nous.** L'API écoute sur `127.0.0.1:46480` (`--port`), ou `--listen` pour Docker et un proxy ; `--allow-host <nom>` ajoute un nom au contrôle de `Host` ; `--trust-proxy` déclare un proxy et lit sa dernière adresse de `X-Forwarded-For`. Le client de bureau ouvre `ssh -N -L 127.0.0.1:<port libre>:127.0.0.1:<port du serveur>` et classe les échecs de `ssh`.

**Pourquoi la socket compte.** Sur une machine où d'autres comptes existent, le port de la boucle locale est joignable par tous : ils peuvent tenter des codes (bornés, § *L'appairage*), et surtout **prendre le port pendant que le serveur redémarre** (une mise à jour, une panne) : le client de bureau, revenu par son tunnel, enverrait alors son jeton à l'usurpateur. Une socket dans un dossier `0700` ferme les deux.

**Ce qu'on prend.**
- **`--socket`** avec ses vérifications (le dossier, ses ancêtres, le verrou, `0600`), et `pair`, `devices`, `health` qui le suivent par `server-status.json`.
- **La page de dépannage du tunnel** : VS Code Remote-SSH, les plages réservées de Windows (`netsh interface ipv4 show excludedportrange`), `AllowStreamLocalForwarding no` (le repli sur le port). Notre client de bureau classe déjà « port local pris » ; il apprend « redirection vers une socket refusée » et propose le repli.

**Ce qu'on fait autrement, et pourquoi.**
- **Le client de bureau suit la socket** : `servers.json` garde le chemin de la socket à côté du port, `ssh -L 127.0.0.1:<port>:<socket>` ; le chemin est lu par SSH dans `server-status.json` (ou demandé par `escouade-server status --json`), jamais deviné (`%t` dépend de l'uid du serveur).
- **Le port reste celui de l'image Docker et du proxy** : une socket ne traverse pas la frontière d'un conteneur sans monter un dossier de l'hôte, et un proxy parle TCP.
- **Pas de `--origin` à part** : notre `--allow-host <nom>` accepte déjà le `Host` ; quand la page est servie, il accepte aussi `Origin: https://<nom>`, et **jamais `http://`** hors de la boucle locale (le jeton, ou la signature, passerait en clair : refusé, pas seulement averti).
- Le défaut (la socket ou le port pour le service de l'utilisateur) revient à l'utilisateur (§ 10, B3), comme le proxy pour un navigateur (§ 10, B4).

## 5. Le contrôle de `Host` et d'`Origin` pour une page

**Chez lui.** `mine` : un seul `Host`, de la boucle locale (`127.0.0.1` ou `localhost`, un port numérique valide) ou d'une origine déclarée, et l'autorité de l'URI d'accord avec lui ; un en-tête donné deux fois est refusé ; s'il y a une `Origin`, elle doit être l'une des siennes, ou `http://<Host>` quand le `Host` est de la boucle locale (la page d'un tunnel, sur le port local que l'utilisateur a choisi) ; aucun `X-Forwarded-*` lu.

**Chez nous.** Le `Host` de la boucle locale (tout port), les noms de `--allow-host`, et **aucune `Origin`** hormis la fenêtre (`tauri://localhost`, `http://tauri.localhost`) et, par le tunnel ou Tailscale, l'app du téléphone.

**Ce qu'on prend.**
- **`Origin` = `http://<Host>`**, seulement quand le `Host` est de la boucle locale, et seulement quand le client navigateur est allumé. Une page d'un autre port (`http://127.0.0.1:5173`, le serveur de dev d'un agent) ouvre sa WebSocket avec `Host: 127.0.0.1:46480` et `Origin: http://127.0.0.1:5173` : refusée. Une page d'un autre site qui fait pointer son nom vers 127.0.0.1 (DNS rebinding) a un `Host` qui n'est pas de la boucle : refusée.
- **Un en-tête donné deux fois refusé** (`Host`, `Origin`), et l'autorité de l'URI d'accord avec le `Host` : deux règles que notre `guard` (`remoteapi/server.rs`) n'écrit pas aujourd'hui (il lit le premier `Host` et ignore l'autorité), à ajouter avec leurs tests.

**Ce qu'on fait autrement.** Rien de plus : pas de CORS à ouvrir (la page et l'API ont la même origine, aucune réponse ne porte `Access-Control-Allow-Origin`), pas de cookie, donc pas de CSRF. Son audit dit que cette surface a tenu sur trois tours ; on garde nos tests de refus et on ajoute ceux du navigateur (une `Origin` d'un autre port, un `Host` rebondi, un `Host` en double, `http://` d'un nom déclaré).

## 6. Ce qui change de machine, vu d'un navigateur

| Quoi | Chez lui | Chez nous (bureau sur un serveur, ESC-538) | Dans le navigateur, proposé |
| --- | --- | --- | --- |
| Choisir un dossier | `browse.rs` : dossiers sous le dossier personnel du service, fichiers privés et cachés laissés, un lien qui sort ni montré ni suivi | `fs_browse` et `clone_repo` (servies), `ServerFolders.svelte`, `CloneRepo.svelte` | **Les nôtres, telles quelles** ; on prend l'idée de borner au dossier personnel du service, à trancher dans le ticket (aujourd'hui `fs_browse` prend un chemin absolu) |
| Joindre un fichier | `POST /files/attach` (HTTP, à côté de la WebSocket) | Corps bruts dans la WebSocket (`attach_*`, `Frame::Body`) | `<input type=file>` et glisser-déposer du navigateur, puis **nos corps bruts** : une seule surface, fermée avec la connexion (son audit a dû couper à part ses transferts à la révocation ; les nôtres le sont par construction, ESC-654) |
| Fichiers envoyés par un agent | `GET /files/sent`, toujours `attachment` ; « Ouvrir » proposé même pour un exécutable (constat de son audit, corrigé) | `sent_file_bytes`, `received_save` sur le poste : type lu dans les octets, jamais un programme ouvert | Les octets par `sent_file_bytes`, un `Blob` de type `application/octet-stream` et un téléchargement (`download`), **jamais une navigation** vers un `blob:` de l'agent (un HTML ou un SVG ouvert dans l'origine de la page y ferait tourner son script) ; « Télécharger » seul pour un programme ou un script, avec la mise en garde ; les images en aperçu par `<img>` seulement |
| Notifications | `notifier.ts` : la permission demandée au premier geste, rien si l'onglet est regardé, une par alerte (`tag`), le clic rendu au serveur | `server_alert` du poste, `host_notification_click` | **Son approche**, écrite chez nous sur les événements de l'hôte (`HostEvent::Notify`) ; le son par `<audio>` après un geste ; le compte de ce qui attend dans le titre de l'onglet |
| Liens | `window.open(…, 'noopener,noreferrer')`, `http`, `https`, `mailto`, `tel` seuls | `open_url` du poste (navigateur du système, sans Referer), `<meta name="referrer" content="no-referrer">` | Les mêmes règles : `noopener,noreferrer`, l'en-tête `Referrer-Policy` en plus de la balise, l'adresse de bouclage du serveur jamais réécrite (pas de relais) |
| Presse-papiers | — | `clipboard_image` du poste | `navigator.clipboard.read()` au geste (127.0.0.1 et HTTPS sont des contextes sûrs), sinon le collage (`paste`) |
| Ports relayés (serveur de test d'un agent) | Absents (« pas relayés vers ton poste ») | `server_relay` : une redirection SSH de plus | **Absents au premier jet**, et dit : « Ouvre un tunnel vers le port 5173 du serveur ». Un relais dans l'API (déjà prévu pour un tunnel sans SSH) viendrait ensuite, pour le navigateur et le téléphone |
| Connexion d'un compte | Le code à coller (Claude Code) | Retour relayé sur le même port, sinon le code (S13) | Le code quand le harness en donne un ; pour un retour par `localhost` (Codex 1455, Gemini), la ligne `ssh -L` à lancer, dite par `ServerSignIn` |
| Voix | Absente | Celle du poste (commandes `desk`) | Absente, et dit (la dictée du navigateur, `getUserMedia` et un Whisper en WASM, serait un autre chantier) |
| Mise à jour | Absente (remplacer le programme et la page) | `escouade-server update` | Rien à faire : la page est dans le binaire ; au refus `version`, la page se recharge |
| Secrets | En clair dans `~/.escouade`, dit | `secrets.json` `0600`, `secretsInFile` montré | Pareil, `SecretsOnServer.svelte` |

## 7. Les plugins et le store dans le navigateur

**Chez lui.** La vue Plugins est sur le rail ; installer depuis le store et depuis un dépôt Git marche ; « Depuis un dossier… » (le dialogue du système) et les liens `escouade://` restent du bureau. La graine des plugins est embarquée (`build.rs`, `place_seed` l'écrit dans `plugin-seed` du dossier de données quand son empreinte change).

**Chez nous.** Les commandes du store et des plugins sont servies ; `plugin_inspect_folder` et `plugin_inspect_dev` prennent un chemin de la machine du cœur ; la graine est dans l'archive et l'image.

**Ce qu'on prend.** Le même partage : le store et les dépôts Git dans le navigateur ; « Depuis un dossier » devient **un dossier du serveur** (`fs_browse`), ce qui marche chez nous puisque le chemin est celui du cœur. Les liens `escouade://` (installer depuis le site) restent du bureau : un navigateur ne les reçoit pas. Les liens de la page d'un plugin et du store s'ouvrent par la règle des liens (§ 6). **La graine embarquée n'est pas reprise** : l'archive la porte déjà.

## 8. Ce que son audit a trouvé sur la surface d'un navigateur

Le rapport (Mantis, 23 novembre 2026, trois tours, 19 constats retenus, aucun critique ni élevé d'après son compte rendu) est une donnée d'un tiers ; ESC-654 a déjà vérifié chez nous ses constats sur le serveur.

- **Le Referer (LOW)** : `window.open(…, 'noopener')` sans `noreferrer`, et une page sans `Referrer-Policy` : un lien écrit par un agent et cliqué donnait l'origine du serveur au site visité. Avec son compteur de codes faux, cela permettait de bloquer l'appairage de l'extérieur, derrière un proxy. Chez nous : les liens passent par `open_url` (sans Referer), le Markdown met `noreferrer`, la page a la balise `no-referrer`, et un code faux n'annule plus rien (ESC-654). **Pour le navigateur, on prend les deux correctifs** : `noopener,noreferrer` à chaque ouverture et l'en-tête `Referrer-Policy: no-referrer` sur chaque réponse ; un test Vitest et un e2e qui vérifient l'absence de Referer.
- **« Ouvrir » un exécutable (LOW)** : la branche navigateur de `SentFiles` proposait « Ouvrir » (un téléchargement) même pour un programme, contre la règle « jamais exécuté ». Chez nous, la règle est au cœur et au poste (`received_save`) ; **dans le navigateur, on la garde côté page** (§ 6) avec un test qui la tient.
- **Ce qu'il dit de solide** : Host, Origin, DNS rebinding, CORS, l'appairage par code, le jeton d'appareil, la page statique, le confinement des fichiers envoyés, sans contournement en trois tours. On en retient la forme de ses contrôles (§ 5), pas leur code.
- **Ce qu'il a écarté et qu'on traite quand même** : la CSP plus faible de la page (on sert celle de la fenêtre, § 3), l'absence de déconnexion dans le navigateur (« Oublier ce serveur », § 2).
- **Ses limites** : aucun serveur complet n'a tourné pendant l'audit (les reproductions recopient le fragment fautif), et six correctifs sont « proposés » sans contre-attaque. D'où le ticket BC-9 : une revue de sécurité du client navigateur chez nous, avant la bêta.

## 9. Les tickets proposés

Estimés au rythme de Claude Code (Opus, un agent par ticket), en heures d'agent : **environ 14 h en tout** (15 h 45 si B2 retient la clé WebCrypto). Avec 6 agents en parallèle, BC-4, BC-5 et BC-6 se font pendant la chaîne BC-1 → BC-2 → BC-3 → BC-7 → BC-8 et BC-9, qui ne se parallélise pas : **9 h 30 de bout en bout** (11 h avec la clé). Ce qui attend l'utilisateur est à part (§ 10 et dernière ligne).

| Ticket | Ce qu'il fait | Après | Heures d'agent |
| --- | --- | --- | --- |
| **BC-1** La page servie | `vite.web.config.ts` (bouchons qui refusent, test du paquet sans Tauri) ; `dist-web/` embarqué par `build.rs` ; `GET /` et `/assets/*` sur le port de l'API, avec la CSP de la fenêtre et son nonce par réponse, `nosniff`, `Referrer-Policy`, COOP, CORP, `Permissions-Policy`, le cache ; `--browser` (`ESCOUADE_SERVER_BROWSER`) ; `Origin` = `http://<Host>` de la boucle, `https://<allow-host>` ; `Host` et `Origin` en double refusés ; tests Rust des refus | — (B1) | 2 h |
| **BC-2** Le navigateur comme client | La marque de la page servie, `RemoteTransport` sur l'adresse de la page ; l'écran d'appairage dans la page (le code et le nom, par `{"t":"pair"}`) ; l'identité de l'appareil selon B2 ; la reprise, le refus `version` qui recharge, la révocation qui ramène à l'appairage ; « Oublier ce serveur » ; la façade `poste` (fenêtre, navigateur, tests) et `transport-only` étendu à `@tauri-apps/*` | BC-1 | 2 h 30 (avec la clé WebCrypto de B2 : `hello` par défi signé, la clé publique dans `remote-devices.json`, le cœur et ses tests : + 1 h 30) |
| **BC-3** Ce qui change de machine, dans le navigateur | Les fichiers joints (`<input>`, glisser-déposer, corps bruts) ; les fichiers reçus (Blob téléchargé, jamais navigué, programmes en « Télécharger » seul) ; les notifications et le son ; les liens ; le presse-papiers ; « Nouveau projet » par `ServerFolders` et `CloneRepo` ; ce qui est absent dit (voix, ports relayés, retour par `localhost` d'un compte) | BC-2 | 2 h 30 |
| **BC-4** Plugins et store dans le navigateur | La vue Plugins sur le rail ; le store et les dépôts Git ; « Depuis un dossier » par un dossier du serveur ; `escouade://` absent ; les liens des pages des plugins | BC-2 | 1 h |
| **BC-5** La socket Unix | `--socket` (dossier `0700`, ancêtres, verrou, `0600`) ; `server-status.json` la dit ; `pair`, `devices`, `health` la suivent ; `install.sh --socket` (défaut selon B3) ; le client de bureau : `ssh -L` vers la socket, le chemin lu par SSH, « redirection vers une socket refusée » classé avec le repli sur le port ; e2e `server-connect` sur une socket (Linux) | — (B3) | 2 h |
| **BC-6** systemd | Des codes de sortie à eux (dossier tenu, arguments faux) et `RestartPreventExitStatus` dans les deux unités d'`install.sh` ; `install.sh --browser` et `--archive <fichier>` ; `scripts/server-install-test.sh` étendu | — | 45 min |
| **BC-7** L'e2e du navigateur | Playwright (Chromium et Firefox) contre un `escouade-server --browser` de chaque test, par une redirection locale : l'appairage, un agent du faux `claude`, la coupure et la reprise d'un terminal, un fichier joint, un fichier reçu et un script jamais ouvert, une notification et son clic, un lien sans Referer, la révocation qui ramène à l'appairage ; aux deux tailles | BC-3, BC-4 | 1 h 30 |
| **BC-8** La documentation | La page « Mode serveur » du site (fr, en) : le navigateur, la socket, le dépannage du tunnel (§ 4) ; la spec (*Le mode serveur*, *L'API distante*, *Ce qui change de machine*, une page *Le client navigateur*) et sa décision ; `docs/REMOTE.md` | BC-7 | 1 h |
| **BC-9** La revue de sécurité du client navigateur | Comme ESC-624 : la surface de la page (CSP, nonce, Origin, Host, rebinding, Referer, téléchargements, `blob:`), l'identité de l'appareil, la socket ; un rapport privé, les correctifs en tickets | BC-7 | 1 h |

**Ce qui attend l'utilisateur** (hors de ces heures) : les quatre choix du § 10 avant BC-1, BC-2 et BC-5 ; puis un essai réel, sur le VPS d'essai, d'un harness connecté depuis le navigateur (un compte à lui), une demi-heure.

**Dans le Kanban** (créés le 2026-11-27, aucun lancé) : BC-1 = ESC-704, BC-2 = ESC-711, BC-3 = ESC-714, BC-4 = ESC-715, BC-5 = ESC-706 (après ESC-702), BC-6 = ESC-707 (après ESC-702 et BC-1), BC-7 = ESC-717, BC-8 = ESC-719, BC-9 = ESC-720.

**L'ordre.** BC-1, BC-5 et BC-6 partent ensemble ; BC-2 après BC-1 ; BC-3 et BC-4 ensemble après BC-2 ; puis BC-7, BC-8 et BC-9.

## 10. Les choix qui reviennent à l'utilisateur

Laissés dans sa boîte « À toi » le 2026-11-26 (B1 : a-45, B2 : a-46, B3 : a-47, B4 : a-48), chacun avec ma recommandation. **L'utilisateur a choisi (a) pour les quatre**, d'où quatre décisions proposées (ids `le-client-navigateur-d-escouade-server-a`, `l-identite-d-un-appareil-dans-le-navigat`, `escouade-server-ecoute-sur-une-socket-un`, `le-client-navigateur-est-permis-derriere`), acceptées à la validation du ticket. Les tickets BC-1 à BC-9 s'écrivent avec ces choix, BC-2 avec la clé WebCrypto (environ 15 h 45 d'agent en tout, 11 h de bout en bout).

- **B1. Faire le client navigateur, et comment l'allumer.** (a) Oui, éteint par défaut, allumé à l'installation (`--browser`) ; (b) oui, allumé par défaut ; (c) non : le bureau et le téléphone suffisent. **Recommandé : (a)** : il sert qui ne peut pas installer Escouade (un poste de travail verrouillé, une tablette, un Chromebook), sans ouvrir de surface sur les serveurs qui n'en ont pas besoin.
- **B2. L'identité d'un appareil dans le navigateur.** (a) Une clé WebCrypto non extractible et un défi signé à chaque connexion ; (b) le jeton dans le `localStorage`, comme le fork. Le cookie est écarté (envoyé à tous les ports de `127.0.0.1`). **Recommandé : (a)** : une page de la même origine, une extension ou un XSS peuvent se servir de la clé tant que la page est ouverte, jamais l'emporter ; avec (b), le jeton part et vaut un terminal sur le serveur jusqu'à sa révocation. Coût : 1 h 30 d'agent de plus.
- **B3. La socket Unix.** (a) La socket par défaut pour le service de l'utilisateur d'`install.sh`, le port pour l'image Docker et le proxy ; (b) la socket en option (`--socket`), le port reste le défaut ; (c) pas de socket. **Recommandé : (a)** : sur une machine où d'autres comptes existent, un port de la boucle locale peut être pris pendant un redémarrage et recevoir le jeton du client de bureau ; la socket dans un dossier `0700` l'empêche. Le prix : un serveur SSH réglé avec `AllowStreamLocalForwarding no` demande le repli sur le port, que le client propose.
- **B4. Un navigateur derrière un proxy HTTPS.** (a) Permis : `--allow-host <nom>` accepte la page de `https://<nom>`, jamais `http://` ; (b) le navigateur par le tunnel SSH seulement. **Recommandé : (a)** : c'est le cas d'usage d'un client navigateur (un appareil sans `ssh`) ; la spec dit déjà les risques d'un proxy (l'API sur Internet, à filtrer par adresse), et B2 (a) borne une fuite.

## 11. Ce qui n'est pas repris, et pourquoi

- **Son code**, aucun fichier ni fragment : autre découpage de `remoteapi/` (`table.rs`, `wire.rs`, `files.rs`, `web.rs`, routes HTTP), autre ligne (2.2.7), un transport à lui. Ses idées sont réécrites sur nos modules, avec nos tests.
- **Les routes HTTP à côté de la WebSocket** (`POST /pair`, `GET /device`, `/files/*`, `/admin/*`) : chez nous tout passe par la WebSocket et `server-status.json` ; une seule surface à borner et à fermer.
- **Le jeton par le sous-protocole et `Authorization`** : le nôtre est dans le premier message, hors de la tête HTTP.
- **`escouade-server install` et la graine embarquée** : `install.sh` et l'archive signée les couvrent.
- **`--web <dossier>`** : la page est dans le binaire.
- **`http://` d'une origine déclarée hors de la boucle locale** : refusé, pas seulement averti.
