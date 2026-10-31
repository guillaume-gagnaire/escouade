---
title: "La page du lancement"
subject: "kanban"
sources: ["design/browser/INVENTAIRE.md","design/browser/a-onglet.png","design/browser/a-tablette.png","design/browser/a-mobile.png","design/browser/a-retreci.png","design/browser/etats.png","website/content/docs/fr/browser.md"]
tickets: ["ESC-311"]
decisions: [26]
compared: null
---

# La page du lancement

## Ce que c'est

Quand le lancement de test d'un agent est prêt, la page de l'app développée s'ouvre toute seule dans Escouade, avec ses outils de développement, pour tester sans changer de fenêtre. Une page par worktree.

## Comportement

### Où elle s'ouvre

- Réglage par projet, gardé sur la machine (jamais dans `.escouade.yml`) : « Ouvrir les pages des lancements » = « Dans l'agent » (par défaut), « Dans une fenêtre à part » ou « Dans le navigateur du système » (le comportement d'avant).
- Dans l'agent : vue « Page » du sélecteur de vue de l'en-tête (à côté de « Escouade » et de la vue terminal), montrée aussi pour un agent de ticket dès qu'il a une page. Si l'utilisateur regardait l'agent ou sa fenêtre « Tester », Escouade ferme cette fenêtre et montre la page ; sinon la page attend.
- Dans une fenêtre à part : fenêtre « <projet> · Pages » du projet, un onglet par page (« worktree · processus »), la même barre. Fermer la fenêtre ferme ses pages.
- Une page par agent (worktree) : relancer recharge la même page ; deux worktrees, deux pages. La page se ferme avec son agent et quand son ticket passe « Terminé ».
- « Rouvrir » de la fenêtre « Tester » la rouvre là où elle s'était ouverte ; la ligne dit « Ouvert dans Escouade, vue « Page » de l'agent : … », « Ouvert dans la fenêtre « Pages » du projet : … » ou « Ouvert dans le navigateur : … ».

### La barre

- Retour, Avancer, Recharger, adresse modifiable (sans schéma : celui de la page), « worktree · processus », largeur (pleine, tablette, mobile), Devtools, « Ouvrir dans une fenêtre à part » / « Remettre dans l'agent » (sans recharger la page), « Ouvrir dans le navigateur du système ».
- Vue étroite : Avancer, le propriétaire, les largeurs, la fenêtre à part et le navigateur du système passent dans « … » ; l'étiquette de largeur reste.
- États : chargement (point ambre, filet), lancement qui redémarre (« Le lancement démarre… »), page qui ne répond pas (chargement non fini en 20 s : « <hôte> ne répond pas », Recharger, Voir les logs), lancement arrêté (« Le lancement s'est arrêté », Relancer, Voir les logs). Textes en français et en anglais.
- La page est une webview native posée sur la fenêtre : elle se masque tant qu'un menu, une liste, un dialogue ou la palette d'Escouade la croise, et quand sa vue n'est pas affichée.

### Devtools

- Bouton, `F12`, `Ctrl+Maj+I` (`⌥⌘I` sous macOS), que le focus soit dans la page ou dans Escouade : les outils de développement de cette page seulement. Windows (WebView2) et macOS (inspecteur WebKit, détaché) : dans leur propre fenêtre ; Linux (inspecteur WebKit) : attaché à la page. Ceux de la fenêtre d'Escouade restent coupés dans une version publiée.

### Largeurs

- Pleine largeur ; Tablette 768 px ; Mobile 390 px, centrée sur fond neutre avec un filet, hauteur de la zone ; largeur affichée (« 768 px »). Zone plus étroite : la page rétrécit à la zone et le dit (« Rétréci à 692 px »).
- `Ctrl+Maj+M` (`⇧⌘M`) passe à la suivante, depuis la page ou depuis Escouade.
- Gardée par page (« worktree · processus ») d'un lancement à l'autre, dans l'agent comme dans la fenêtre à part.
- Seule la largeur change (les media queries). User agent, `devicePixelRatio` et tactile ne sont pas émulés ; les Devtools gardent leur propre mode appareil.

### Isolation, sans exception

- Webview à part : aucune commande d'Escouade ni d'un plugin ne lui répond (appel d'une webview `devpage-*` refusé ; la capacité ne nomme que les webviews `main` et `pagesbar-*`).
- Stockage du projet, jamais celui de la fenêtre : dossier `pages/<projet>` des données (Windows, Linux), magasin par projet (macOS 14+), session privée avant.
- Elle reste sur l'origine de son lancement : autre origine → navigateur du système, avec un message ; adresses d'Escouade, fichiers et autres schémas → nulle part.

### Vérifié par système

- `devpage_probe` (vraie webview) en CI sous Linux (Xvfb) et macOS, et à la main sous Windows et Linux (WSLg) ; l'e2e `devpage.spec.ts` sous Windows.
