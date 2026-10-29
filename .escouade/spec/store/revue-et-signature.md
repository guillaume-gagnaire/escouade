---
title: "La revue « Vérifié par Escouade » : la file, la signature, la révocation"
subject: "store"
sources: ["docs/STORE.md","design/store/index.html","design/app-redesign/UX-NOTES.md"]
tickets: ["ESC-235","ESC-295"]
decisions: []
compared: null
---

# La revue « Vérifié par Escouade » : la file, la signature, la révocation

Détail et procédure : `docs/STORE.md` §4.1, §4.5, §4.6 (il fait foi).

## La file `/review`
- Visible des seuls comptes de `STORE_ADMINS` ; pour les autres, la page « n'existe pas ». Le lien « File de revue » et son nombre dans l'en-tête.
- Onglets : « À relire » (versions publiées dont l'auteur a demandé la revue, les plus installées puis les plus anciennes), « Signalements », « Noms proches », « Faites ».
- Le détail d'une version : la commande `npm run plugin-review -- <id>@<version>` à copier, ce qui est nouveau depuis la dernière version vérifiée, la taille du diff, le rapport déposé par le script (lu comme du texte), « Demander des changements » et « Refuser » avec un message à l'auteur. **Pas de bouton « Vérifier »** : on signe sur le poste.
- Les signalements : ceux que plusieurs disent « malveillant » en tête, l'adresse de qui a signalé n'est jamais affichée ; « Retenu » ou « Classer sans suite », avec une réponse envoyée à qui a signalé.

## Le jeton d'administration
- Créé dans le compte (« Jetons d'administration »), montré une fois, gardé haché, cinq au plus, révocable ; il n'agit que tant que son compte est dans `STORE_ADMINS`.
- Il permet de lire la file, de déposer un rapport, une signature, une liste de révocation ; ni publier ni supprimer.

## La signature
- Deux paires minisign créées par l'utilisateur seul (tous les jours + secours), chiffrées, sauvegardées hors ligne ; les deux clés publiques dans `store/keys/` et dans l'app. Sans elles, l'app refuse tout badge et le store ne prend aucune signature.
- `npm run plugin-review -- sign <id>@<version>` signe le `version.json` relu (commentaire `escouade-verified-v1 <id>@<version> <date>`), vérifie localement, dépose avec le résumé public fr/en.
- Le store vérifie avec ses clés publiques avant d'accepter ; refuse une version avec un binaire non attesté ou désactivée. La version devient `verified` au catalogue, sa page montre « Relu par Escouade le … » et le résumé, l'auteur est prévenu.

## La révocation et le retrait d'urgence
- `npm run plugin-review -- revoke|lift|revoke-key` : liste signée sur le poste, `serial` + 1, vérifiée avant dépôt.
- `disable` sort la version du catalogue (fichiers en 410) ; `unverify` retire le sceau ; l'auteur est prévenu sauf `--quiet`.

## Les annonces
- Chaque version à revoir, signalement, nom trop proche, étiquette déplacée, retrait et révocation part aussi dans le salon privé des admins sur Discord ; les badges et les retraits dans le salon public fr et en (voir « Les annonces du store sur Discord », `docs/STORE.md` §4.7).
