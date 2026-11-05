---
title: "La revue « Vérifié par Escouade » : la file, la signature, la révocation"
subject: "store"
sources: ["docs/STORE.md","design/store/index.html","design/app-redesign/UX-NOTES.md","store/server/utils/review.ts","store/server/utils/discord.ts","store/scripts/plugin-review/","docs/PLUGINS.md §5.6","docs/PLUGINS.md §8.5.5, §8.20","Consignes du chef au lancement d'ESC-324 (2026-11-01)"]
tickets: ["ESC-235","ESC-295","ESC-302","ESC-324","ESC-343","ESC-337"]
decisions: [46,76,89]
compared: "2026-10-30T03:23:39+01:00"
---

# La revue « Vérifié par Escouade » : la file, la signature, la révocation

Détail et procédure : `docs/STORE.md` §4.1, §4.2, §4.5, §4.6 (il fait foi).

## La file `/review`
- Visible des seuls comptes de `STORE_ADMINS` ; pour les autres, la page « n'existe pas ». Le lien « File de revue » et son nombre dans l'en-tête.
- Onglets : « À relire » (versions publiées dont l'auteur a demandé la revue, les plus installées puis les plus anciennes), « Signalements », « Noms proches », « Faites ».
- Le détail d'une version : la commande `npm run plugin-review -- <id>@<version>` à copier, ce qui est nouveau depuis la dernière version vérifiée, la taille du diff, le rapport déposé par le script (lu comme du texte), « Demander des changements » et « Refuser » avec un message à l'auteur. **Pas de bouton « Vérifier »** : on signe sur le poste.
- Les signalements : ceux que plusieurs disent « malveillant » en tête, l'adresse de qui a signalé n'est jamais affichée ; « Retenu » ou « Classer sans suite », avec une réponse envoyée à qui a signalé.

## Le jeton d'administration
- Créé dans le compte (« Jetons d'administration »), montré une fois, gardé haché, cinq au plus, révocable ; il n'agit que tant que son compte est dans `STORE_ADMINS`.
- Il permet de lire la file, de déposer un rapport, une signature, une liste de révocation ; ni publier ni supprimer.

## La revue locale d'un plugin `run.js` (ESC-324)
- La revue télécharge aussi le module `.wasm` que le store a compilé et vérifie sa taille et son SHA-256 contre `version.json` ; un écart arrête tout.
- Elle **recompile** le bundle de l'archive relue par l'`escouade-plugin` du poste (`ESCOUADE_PLUGIN`, sinon celui du `PATH`, sinon celui construit dans le dépôt), comme le store le compile : un dossier neuf, un environnement à lui. Cet `escouade-plugin` doit compiler avec les versions que nomme `version.json` (`compiler` : Javy, escouade-js, escouade-plugin).
- **La même empreinte** : le rapport dit « même empreinte, ce qui a été lu est ce qui tourne », et le résumé public « module WASM recompilé, même empreinte ». **Une autre empreinte** : la revue s'arrête là, code 1, verdict « Refuser », comme pour une archive changée. **Pas de recompilation** (pas d'`escouade-plugin` sur le poste, un autre compilateur que celui du store, un bundle qu'il refuse) : la revue continue, le rapport dit lequel installer, et le verdict proposé est « Demander des changements » tant qu'elle n'est pas relancée.
- **Le banc** d'un plugin `run.js` joue le module publié dans l'hôte WASM d'Escouade, hors ligne (`escouade-plugin test --wasm <module> --offline` : tout `fetch` refusé, même vers les hôtes de `permissions.network`), sans conteneur, dans un environnement à lui (ni le jeton d'administration ni rien de la revue). Le rapport dit « le module publié joué dans l'hôte WASM d'Escouade, sans conteneur ni réseau ». Le conteneur sans réseau reste pour un plugin à binaire.
- La lecture (`/review-plugin`) part du bundle (ce qui tourne), puis `src/`, et vérifie que le bundle est le build de `src/` quand le plugin dit comment le faire. Les programmes lancés, `net` et les `import` autres que `node:fs` et `node:readline` ne sont plus à chercher (la compilation les refuse) ; restent les hôtes appelés, comparés à `permissions.network`, et ce que le code fait de ses secrets.

## La signature
- Deux paires minisign créées par l'utilisateur seul (tous les jours + secours), chiffrées, sauvegardées hors ligne ; les deux clés publiques dans `store/keys/` et dans l'app. Sans elles, l'app refuse tout badge et le store ne prend aucune signature.
- `npm run plugin-review -- sign <id>@<version>` signe le `version.json` relu (commentaire `escouade-verified-v1 <id>@<version> <date>`), vérifie localement, dépose avec le résumé public fr/en.
- `sign` refuse une version dont la revue a relevé un bloqueur ; pour un plugin `run.js`, une recompilation qui n'a pas été faite ou n'a pas donné la même empreinte en est un. `sign` relit aussi le `wasm` du `version.json` servi et refuse si la revue n'a pas recompilé ce module-là : la signature couvre `version.json`, donc le module.
- Le store vérifie avec ses clés publiques avant d'accepter ; refuse une version avec un binaire non attesté ou désactivée. La version devient `verified` au catalogue, sa page montre « Relu par Escouade le … » et le résumé, l'auteur est prévenu.

## La révocation et le retrait d'urgence
- `npm run plugin-review -- revoke|lift|revoke-key` : liste signée sur le poste, `serial` + 1, vérifiée avant dépôt.
- `disable` sort la version du catalogue (fichiers en 410) ; `unverify` retire le sceau ; l'auteur est prévenu par e-mail sauf `--quiet`.
- **`--quiet` ne coupe que l'e-mail à l'auteur** (quand le prévenir aiderait un attaquant) : l'entrée part quand même dans le salon des admins et dans le salon public fr et en (ESC-302). La liste de révocation est publique de toute façon (`/api/v1/revocations.json`).

## Les annonces
- Chaque version à revoir, signalement, nom trop proche, étiquette déplacée, retrait et révocation part aussi dans le salon privé des admins sur Discord. Pour une entrée de la liste, ce salon dit si l'auteur a été prévenu (« Auteur : prévenu » ou « non prévenu (--quiet) »).
- Les badges et chaque nouvelle entrée de la liste partent dans le salon public fr et en, avec la raison publique de l'entrée dans la langue du salon : « Plugin retiré » / « Plugin taken off » pour `disable`, « Badge retiré » / « Badge withdrawn » pour `unverify`, avec ou sans `--quiet`. Une entrée déjà dans la liste précédente n'est pas annoncée de nouveau. Voir « Les annonces du store sur Discord », `docs/STORE.md` §4.7.

## La première main avant sa release (ESC-343)
- `escouade-plugin validate`, `test` et `dev` prennent `--escouade <version>` : le plugin est joué comme cette version d'Escouade. La plage `escouade` du manifeste se contrôle contre elle, et c'est elle que le plugin voit dans `initialize`. Sans l'option, c'est la version de l'outil. `dev` contrôle désormais la plage, comme `test`. Le rapport JSON du banc nomme la version jouée (`escouade`).
- La revue locale joue la première main (`--first-party`) comme la version en développement : `NEXT` de `website/app/data/site.ts`, complété en `x.y.0`. Elle peut ainsi passer le banc et être signée avant que cette version existe. Un tiers est toujours joué comme l'outil du poste : `--escouade` ne part jamais sans `--first-party`.
- Le rapport dit la version jouée : « Joué comme Escouade 2.2.0, la version en développement » ou « la version de l'outil du poste ».
- La recompilation ne change pas : elle se fait avec le compilateur que nomme `version.json`, sinon `sign` refuse.
- Un `escouade-plugin` construit sans `escouade-js`, ou avec un autre que celui de `version.json` : la revue récupère le bon de la CI de main (artefact `escouade-js`, gardé 7 jours, par `gh run download`) dans `src-tauri/escouade-js/target/`. Elle ne garde que le fichier à l'empreinte de `version.json`. Le rapport dit ce qui reste à faire : `cargo build`, puis relancer la revue. Sans `gh` ou sans artefact, il dit comment l'obtenir à la main.

## La revue locale d'un plugin `run.wasm` (ESC-337)
- La revue télécharge le module et le vérifie comme pour un `run.js`, puis **recompile les sources de l'archive** : `escouade-plugin fetch` puis `build`, avec le `compiler` de `version.json` (`rust`, `target`, `canon`, `lock`).
- **La première main** (`dev.escouade.*` du dépôt de l'app) compile sur le poste, par l'`escouade-plugin` du poste.
- **Un tiers ne compile jamais sur le poste nu.** Son `build.rs` et ses macros sont du code natif, et un `include_bytes!` d'un chemin absolu ferait entrer un fichier du poste dans le module. Il compile dans des conteneurs neufs qui ne voient ni `HOME`, ni `~/.escouade-signing`, ni le dépôt : seulement les fichiers réguliers de l'archive et l'`escouade-plugin` Linux (`ESCOUADE_PLUGIN_LINUX`), en lecture seule. Aucun dossier du poste ne leur est ouvert en écriture.
- Les étapes sont celles du store, en quatre conteneurs : préparer les volumes (sans le plugin), `fetch` avec réseau (rien du plugin ne s'exécute), `build` sans réseau (les outils en lecture seule), lire le module. Les outils et les crates restent dans un volume d'une revue à l'autre ; le reste est retiré.
- **La même empreinte** : le rapport dit « Recompilé dans un conteneur sans réseau », ou « ici » pour la première main. **Une autre empreinte, ou un autre `lock`** : code 1, verdict « Refuser ». **Pas de recompilation** (pas de moteur de conteneur, pas de binaire Linux, un autre `rust`, `target` ou `canon`, des sources refusées, un conteneur tué) : la revue continue, et le verdict proposé est « Demander des changements ».
- `sign` traite un `run.wasm` comme un `run.js` : sans recompilation à la même empreinte, il refuse.
- **Le banc** joue le module publié dans l'hôte WASM, hors ligne, sans conteneur, comme pour un `run.js`.
- **Les pistes de Rust** s'ajoutent aux autres : `build.rs` et macros procédurales ; `unsafe`, `extern`, `#[link]`, `#[no_mangle]`, `asm!`, `transmute` ; `include_bytes!`, `include_str!`, `include!` et `#[path]` vers un chemin absolu, qui sort du dossier ou calculé ; `env!` ; les imports du module, hors de `wasi_snapshot_preview1` et `escouade.call`, et les méthodes `net/*`, `ws/*`, `fs/*`, `proc/*` écrites dans le module, comparées à `permissions.network`, `files` et `commands` ; le `Cargo.lock` paquet par paquet (source, somme, liste fermée `rust_native.json`) et son diff avec la version d'avant ; le code illisible.
- La lecture (`/review-plugin`) part de `Cargo.toml` et de `src/` : la recompilation à la même empreinte prouve que ces sources sont ce qui tourne.
