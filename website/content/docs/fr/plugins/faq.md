# FAQ des plugins

Les questions qui reviennent sur les plugins, pour ceux qui les installent comme pour ceux qui les écrivent, avec un renvoi vers la page qui détaille chaque réponse.

## Pour l’utilisateur

### Faut-il Node pour faire tourner un plugin ?

Non : un plugin du store tourne dans l’app, en WASM, qu’il soit écrit en TypeScript (`run.js`) ou en Rust (`run.wasm`). Seul un plugin installé depuis un dossier ou un dépôt git dont la commande nomme `node` en a besoin ; le store refuse cette forme. Slack et Règles de l’équipe, deux des [plugins d’exemple](plugins/examples), tournent en `run.js` depuis leur 2.0.0.

### « Vérifié par Escouade », « Non vérifié » : quelle différence ?

Une version vérifiée a été relue par Escouade et signée ; l’app vérifie la signature elle-même, sans faire confiance au store. Une version non vérifiée a passé les contrôles automatiques du store, mais personne n’a lu son code ; un plugin installé depuis un dossier ou un dépôt git l’est toujours. Voir [la sécurité des plugins](plugins/security).

### Un plugin peut-il voir mes projets privés ?

Pas d’office : un plugin que tu installes est coupé dans chaque projet privé, et rien de ce projet ne lui est envoyé tant que tu ne l’y allumes pas. Un programme `run.command` a cependant tes droits sur le disque (voir [les permissions](plugins/permissions)). Les plugins livrés avec Escouade, eux, y sont actifs, comme le reste de l’app.

### Comment arrêter un plugin qui fait quelque chose que je ne veux pas ?

Son interrupteur, dans la vue Plugins, l’arrête pour toute l’app ; son onglet Projets, pour un projet ; son onglet Permissions retire une permission seule. Son journal dit ce qu’il a fait. Un plugin livré avec Escouade se coupe de la même façon.

### Un projet peut-il installer un plugin à ma place ?

Non. Son `.escouade.yml` peut en proposer (`plugins.recommended`) : Réglages › Projets les montre, et l’installation passe toujours par la fenêtre qui dit ses permissions (voir [`.escouade.yml`](escouade-yml#plugins)).

### Où sont ses données ?

Sa copie dans `~/.escouade/plugins/<id>/`, la liste des plugins dans `~/.escouade/plugins.json`, son journal dans `~/.escouade/plugin-logs/<id>.log`, ses secrets dans le trousseau du système (voir [les données locales](data)).

## Pour l’auteur

### Dans quel langage écrire un plugin ?

Sans code, quand le manifeste suffit : un thème, des webhooks, un harness ACP. En JS ou en TypeScript avec [le SDK](plugins/sdk), publié en `run.js`. En Rust avec [le kit](plugins/rust), publié en `run.wasm`. Le store n’accepte pas d’autre langage : le protocole reste ouvert à tout programme, mais il ne se publie pas.

### Comment le tester sans lancer Escouade ?

`escouade-plugin test` joue le banc de conformité, `escouade-plugin dev` le faux Escouade, à la main ; le SDK donne un faux en mémoire pour les tests unitaires, et le kit Rust `serve_on` (voir [tester un plugin](plugins/testing)).

### Puis-je utiliser `dev.escouade.*` comme identifiant ?

Non : il est réservé aux plugins d’Escouade. Prends un nom de domaine à toi, à l’envers (`io.github.<ton-login>.mon-plugin`).

### Peut-il ajouter un panneau ou une page à l’interface ?

Pas encore : un plugin ajoute ce que son manifeste déclare (thèmes, harness, réglages…), rendu par Escouade.

### Faut-il un compte pour publier ?

Un compte GitHub et un dépôt public : le store lit la version à son étiquette (voir [publier sur le store](plugins/publishing)). Un agent peut aussi le faire pour toi, par [le serveur MCP du store](plugins/store-mcp).
