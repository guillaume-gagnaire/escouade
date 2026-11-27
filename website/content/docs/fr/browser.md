# Les navigateurs

Escouade a des navigateurs comme il a des terminaux : une ligne par onglet dans la section « Navigateurs » de la barre latérale. Tu y ouvres l’app que tu développes à côté des agents qui la codent, avec ses devtools et trois largeurs, par « + » ou de lui-même pour un test ou une commande de lancement.

![Un navigateur en plein écran : la barre (adresse, largeurs, Devtools) et la page de l’app développée ; à gauche, la section « Navigateurs » et ses trois lignes](/images/browser.jpg)

## La section « Navigateurs »

Comme celle des terminaux :

- **Une ligne par navigateur**, dans l’ordre où ils se sont ouverts, avec leur nombre en tête. La ligne dit le titre de la page (« Tableau — demo »), sinon son hôte ; celle d’un lancement dit l’agent et son processus (« pagination-users · web »), avec la marque de son worktree.
- **Son point** dit l’état de la page : ambre pendant qu’elle charge, vert une fois chargée, rouge quand elle ne répond pas (un chargement qui ne finit pas en 20 s).
- **Un clic** l’affiche dans la zone principale, en plein écran, comme un terminal ; `Ctrl`+clic (`⌘`+clic sous macOS) l’ouvre à côté de ce qui est affiché, en demi-écran ; tu peux aussi la glisser sur la zone de travail (voir [l’écran partagé](split-screen)).
- **Son menu** (clic droit) : « Ouvrir à côté », « Ouvrir en dessous », « Recharger », « Copier l’adresse », « Ouvrir dans le navigateur du système », « Fermer le navigateur ».
- **Sa croix** ferme le navigateur.
- **« + »** demande une adresse, dans la section même : tape `localhost:3000` (sans `http://`, c’est `http://`) et `Entrée`. `Échap` renonce.

Dans une fenêtre basse, la section se replie sur son titre, comme « Terminaux » ; un clic sur le titre la déplie.

Un navigateur vit tant qu’Escouade tourne, comme un terminal : ni l’un ni l’autre ne revient après un redémarrage. La largeur choisie, elle, est gardée pour la page.

## Ce qui ouvre un navigateur

- **Toi**, par le « + » de la section.
- **Le test d’un agent** : dès que les processus de « ▶ Tester » répondent (voir [Tester et valider](testing)), l’adresse qui montre la fonctionnalité s’ouvre dans la ligne de l’agent. Si tu regardais l’agent ou sa fenêtre « Tester », Escouade te la montre ; sinon la ligne t’attend. Il y a une ligne par agent : relancer le test recharge la même page. Elle se ferme avec l’agent et quand son ticket passe « Terminé ».
- **Une commande de lancement** qui a une adresse à ouvrir (voir [Lancer le projet](launch)) : sa page s’ouvre dans sa ligne, une par commande, rechargée à chaque lancement plutôt que doublée.

Réglages › Projets › Lancement, « Ouvrir les pages des lancements », choisit entre les navigateurs d’Escouade (par défaut) et le navigateur du système. L’ancien choix « Dans une fenêtre à part » ouvre désormais la ligne du navigateur : la fenêtre « Pages » n’existe plus.

Dans l’en-tête de la conversation d’un agent, « Page » montre la ligne de son lancement ; glissé sur la zone de travail, il la met à côté de la conversation.

## La barre d’un navigateur

| Bouton | Ce qu’il fait |
|---|---|
| Retour, Avancer, Recharger | Comme dans un navigateur. |
| L’adresse | Modifiable : tape un chemin ou une adresse et `Entrée`. Sans `http://`, l’adresse garde celui de la page. Le point devant dit l’état de la page. |
| `pagination-users · web` | Pour la page d’un lancement : l’agent et le processus qui la sert. |
| Pleine largeur, Tablette, Mobile | La largeur de la page (voir plus bas), à côté de l’adresse. |
| Devtools | Les outils de développement de **cette page** : console, réseau, éléments. |
| Ouvrir dans le navigateur du système | La même adresse, dans ton navigateur. |

« … » ouvre la page à côté ou en dessous de ce qui est affiché (à côté de la conversation de son agent quand c’est la page d’un lancement affichée seule) et copie son adresse. Quand la vue est étroite, Avancer et le navigateur du système y passent aussi, puis les largeurs quand elle l’est plus encore (un volet étroit).

La page d’un lancement dit aussi quand le lancement redémarre (« Le lancement démarre… ») ou s’est arrêté (« Le lancement s’est arrêté », avec « Relancer » et « Voir les logs »). Quand un menu, une liste, la palette ou une fenêtre d’Escouade passe sur la page, elle s’efface le temps qu’ils soient ouverts, puis revient.

## Les outils de développement

Le bouton « Devtools », ou `F12` et `Ctrl+Maj+I` (`⌥⌘I` sous macOS) quand le focus est dans Escouade, ouvrent les outils de développement de la page. Depuis la page elle-même, le clic droit « Inspecter » le fait partout, et `F12` sous Windows. Ils ne s’ouvrent que sur ton geste : la page ne peut pas les ouvrir d’elle-même. Ce sont ceux de la webview du système, et chacun se loge où il peut :

| | Windows (WebView2) | macOS (WKWebView) | Linux (WebKitGTK) |
|---|---|---|---|
| Où ils s’ouvrent | Dans leur propre fenêtre | Dans leur propre fenêtre | Dans la page, en bas |
| En bas ou à droite de la page | Non : WebView2 ne les loge pas dans l’app (il faudrait ouvrir un port de débogage à tout programme de la machine) | Non : logé dans la page, l’inspecteur de WebKit en change lui-même la largeur, et les largeurs Tablette et Mobile ne tiennent plus | Oui : en bas par défaut, à droite ou à part par les boutons de l’inspecteur |

L’infobulle du bouton « Devtools » le dit pour ton système. Les outils de développement d’Escouade elle-même restent fermés.

## Pleine largeur, tablette, mobile

Trois boutons, à côté de l’adresse, changent la largeur de la page :

- **Pleine largeur** : la page prend toute sa zone.
- **Tablette** : 768 px, centrée sur un fond neutre, la hauteur de la zone.
- **Mobile** : 390 px, de même.

La largeur s’affiche à côté (« 768 px »). Si la zone est plus étroite, la page rétrécit à la zone et le dit (« Rétréci à 692 px »). `Ctrl+Maj+M` (`⇧⌘M` sous macOS) passe à la largeur suivante, depuis la page ou depuis Escouade. Le choix est gardé pour la page : une relance garde sa largeur.

Seule la largeur change : c’est elle qui déclenche les media queries de la page. Ce que le site voit d’autre ne bouge pas :

| | Windows (WebView2) | macOS (WKWebView) | Linux (WebKitGTK) |
|---|---|---|---|
| User agent mobile | Possible | Possible | Possible |
| `devicePixelRatio` | Possible, par le protocole des devtools | Non | Non |
| Écran tactile | Possible, par le protocole des devtools | Non | Non |

Ce n’est pas encore fait : pour émuler un appareil complet, utilise le mode appareil des outils de développement là où ils en ont un (celui d’Edge sous Windows).

## Une page à part

La page est l’app que tu développes, ou un site que tu as tapé : Escouade ne lui fait pas confiance, sans exception.

- Elle tourne dans une webview **à part**, qui n’a accès à **aucune** commande d’Escouade : un appel à `__TAURI__` ou `invoke` depuis la page est refusé.
- Ses cookies et son stockage (localStorage, IndexedDB…) sont ceux du **projet**, jamais ceux de la fenêtre d’Escouade : un dossier `pages/<projet>` dans les données d’Escouade sous Windows et Linux, un magasin à part sous macOS 14 et plus (une session privée avant).
- Seule une adresse `http://` ou `https://` s’ouvre dans un navigateur, jamais une adresse d’Escouade, un fichier ou un autre schéma.
- C’est un vrai navigateur : les liens, les redirections, les formulaires et ce que tu tapes dans la barre restent dans l’onglet, vers n’importe quel site `http://` ou `https://`, et la barre dit toujours l’adresse où tu es. Jamais : une adresse d’Escouade, un fichier, `data:`, `javascript:`, `blob:`, un autre schéma, ni ce poste (`localhost`, `127.0.0.1`) à un port que ton projet ne déclare pas (ses commandes de lancement, les ports de ses agents), où un site joindrait un service de ta machine : Escouade le refuse et le dit. Les adresses `mailto:` et `tel:` s’ouvrent dans l’application du système.
- Ni Escouade ni un agent ne lit les cookies ou le stockage d’un onglet. Un agent ne lit, ne capture et ne pilote un site hors du lancement du projet qu’avec ton accord (voir [les agents et les navigateurs](agent-browsers)), et rien de ce qu’il lit ne vient d’une page où il n’a pas cet accord, même quand l’onglet y est passé puis revenu.

## Se connecter à un site

Tu te connectes à un site dans un onglet d’Escouade comme dans ton navigateur, et la session reste celle du projet :

- **La page de connexion** et ses détours (le fournisseur d’identité, « Continuer avec Google », le retour vers le site) restent dans l’onglet.
- **La fenêtre de connexion** qu’ouvre la page (`window.open`, un lien `target="_blank"`, celle de Google ou d’un OAuth) s’ouvre dans Escouade, avec la même session que l’onglet, et garde son lien avec la page : elle lui répond, puis se ferme quand la page la ferme. Son titre commence par l’origine où elle est (`https://accounts.google.com — …`), qu’aucune page ne choisit. Elle s’ouvre au-dessus de la fenêtre d’Escouade, jamais au-dessus d’une autre application ; trois au plus à la fois par onglet, et elles se ferment avec lui.
- **Un lien de connexion reçu par courriel** : copie-le et colle-le dans la barre de l’onglet, puis `Entrée` : la connexion se fait dans l’onglet, et pas dans ton navigateur.
- **« Ouvrir dans le navigateur du système »** reste dans la barre (ou dans « … ») pour qui préfère s’y connecter.

Une page d’agent (`show_page`, voir [les pages des agents](agent-pages)) n’ouvre aucune fenêtre : elle n’a pas de réseau.

## À distance

Une page ne se montre qu’à l’écran de la machine où elle tourne : les navigateurs sont ceux de la fenêtre. Une fenêtre connectée à un [serveur](connect-server) a les siens, sur ta machine (l’adresse d’un test du serveur y est relayée). Le téléphone et l’API distante ne les voient pas.
