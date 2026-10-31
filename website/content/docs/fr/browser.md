# La page du lancement

Quand « ▶ Tester » est prêt, la page de l’app que l’agent vient de développer s’ouvre toute seule dans Escouade, dans la vue « Page » de l’agent, avec ses outils de développement. Tu testes sans changer de fenêtre, et chaque page reste rattachée à son worktree.

![La vue « Page » de l’agent pagination-users : la barre d’adresse, le sélecteur de largeur, le bouton Devtools, et la page de l’app développée](/images/browser.jpg)

## Où s’ouvre la page

Dès que les processus du test répondent (voir [Tester et valider](testing)), l’adresse qui montre la fonctionnalité s’ouvre là où le projet le dit, dans Réglages › Projets › Lancement, « Ouvrir les pages des lancements » :

- **Dans l’agent** (par défaut) : la page s’ouvre dans la vue « Page » de l’agent, à côté de « Escouade » et de la vue terminal dans l’en-tête de sa conversation. Si tu regardais cet agent ou sa fenêtre « Tester », Escouade te montre la page ; sinon elle t’attend, et la ligne du processus dans « Lancement » le dit.
- **Dans une fenêtre à part** : la page s’ouvre dans la fenêtre « demo-api · Pages » du projet, un onglet par worktree, que tu peux mettre sur un second écran.
- **Dans le navigateur du système** : comme avant, ton navigateur s’ouvre sur l’adresse.

Il y a une page par worktree : relancer le test recharge la même page, deux worktrees ont deux pages. Le réglage reste sur ta machine, il n’est jamais écrit dans le `.escouade.yml` du projet.

## La barre de la page

| Bouton | Ce qu’il fait |
|---|---|
| Retour, Avancer, Recharger | Comme dans un navigateur. |
| L’adresse | Modifiable : tape un chemin ou une adresse et `Entrée`. Sans `http://`, l’adresse garde celui de la page. |
| `pagination-users · web` | À qui est la page : le worktree et le processus qui la sert. |
| Pleine largeur, Tablette, Mobile | La largeur de la page (voir plus bas). |
| Devtools | Les outils de développement de **cette page** : console, réseau, éléments. |
| Ouvrir dans une fenêtre à part | La page passe dans la fenêtre « Pages » du projet, sans être rechargée. Le même bouton l’y reprend. |
| Ouvrir dans le navigateur du système | La même adresse, dans ton navigateur. |

Quand la vue est étroite, Avancer, les largeurs, la fenêtre à part et le navigateur du système passent dans « … ».

Le point devant l’adresse dit l’état de la page :

- **ambre** : elle charge, ou le lancement redémarre (« Le lancement démarre… La page s’ouvre ici dès que son serveur répond. ») ;
- **rouge** : elle ne répond pas (« localhost:4101 ne répond pas », après 20 s d’un chargement qui ne finit pas), avec « Recharger » et « Voir les logs » ; ou le lancement s’est arrêté (« Le lancement s’est arrêté »), avec « Relancer » et « Voir les logs » ;
- **vert** : elle est chargée.

Quand un menu, une liste, la palette ou une fenêtre d’Escouade passe sur la page, elle s’efface le temps qu’ils soient ouverts, puis revient.

## Les outils de développement

Le bouton « Devtools », `F12` ou `Ctrl+Maj+I` (`⌥⌘I` sous macOS) ouvrent les outils de développement de la page, que le focus soit dans la page ou dans Escouade. Ce sont ceux de la webview du système : ceux d’Edge (WebView2) sous Windows et l’inspecteur de WebKit sous macOS, chacun dans sa propre fenêtre ; l’inspecteur de WebKit sous Linux, attaché à la page. Les outils de développement d’Escouade elle-même restent fermés.

## Pleine largeur, tablette, mobile

Trois boutons, à côté de « Devtools », changent la largeur de la page :

- **Pleine largeur** : la page prend toute sa zone.
- **Tablette** : 768 px, centrée sur un fond neutre, la hauteur de la zone.
- **Mobile** : 390 px, de même.

La largeur s’affiche à côté (« 768 px »). Si la zone est plus étroite, la page rétrécit à la zone et le dit (« Rétréci à 692 px »). `Ctrl+Maj+M` (`⇧⌘M` sous macOS) passe à la largeur suivante, depuis la page ou depuis Escouade. Le choix est gardé pour la page (le worktree et son processus) : une relance garde sa largeur, dans l’agent comme dans la fenêtre à part.

Seule la largeur change : c’est elle qui déclenche les media queries de la page. Ce que le site voit d’autre ne bouge pas :

| | Windows (WebView2) | macOS (WKWebView) | Linux (WebKitGTK) |
|---|---|---|---|
| User agent mobile | Possible | Possible | Possible |
| `devicePixelRatio` | Possible, par le protocole des devtools | Non | Non |
| Écran tactile | Possible, par le protocole des devtools | Non | Non |

Ce n’est pas encore fait : pour émuler un appareil complet, utilise le mode appareil des outils de développement là où ils en ont un (celui d’Edge sous Windows).

## Une page à part

La page est l’app que tu développes : Escouade ne lui fait pas confiance, sans exception.

- Elle tourne dans une webview **à part**, qui n’a accès à **aucune** commande d’Escouade : un appel à `__TAURI__` ou `invoke` depuis la page est refusé.
- Ses cookies et son stockage (localStorage, IndexedDB…) sont ceux du **projet**, jamais ceux de la fenêtre d’Escouade : un dossier `pages/<projet>` dans les données d’Escouade sous Windows et Linux, un magasin à part sous macOS 14 et plus (une session privée avant).
- Elle reste sur l’origine de son lancement : un lien vers une autre origine (un autre port, `127.0.0.1` au lieu de `localhost`, un autre site) s’ouvre dans ton navigateur, et Escouade le dit (« github.com s’est ouvert dans ton navigateur : ici, seul localhost:4101 s’affiche. ») ; les adresses d’Escouade, les fichiers et les autres schémas ne s’ouvrent nulle part.

## La fenêtre « Pages »

La fenêtre « demo-api · Pages » d’un projet a un onglet par page détachée (« pagination-users · web », « refacto-auth · Front »…), et sous les onglets la même barre que dans l’agent. Fermer la fenêtre ferme ses pages ; « Remettre dans l’agent » reprend une page dans la vue de son agent sans la recharger.
