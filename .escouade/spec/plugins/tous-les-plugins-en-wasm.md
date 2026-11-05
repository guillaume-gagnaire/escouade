---
title: "Tous les plugins en WASM (la cible de la 2.2)"
subject: "plugins"
sources: ["docs/PLUGINS.md §8 (ESC-330)","docs/PLUGINS.md §8.16 (ESC-341)","prototypes/wasm-all/","Décision de l'utilisateur du 2026-11-04 : « en 2.2, tous les plugins, rust ou TS, seront compilés en WASM, c'est acté »","Précision de l'utilisateur du 2026-11-04 : « tous les plugins builtins soient dans /plugins/*, tous publiés sur le store, vérifiés et signés »","Choix de l'utilisateur du 2026-11-05 (ESC-341) : MIT pour les trois crates, avec l'accord de Florian Gagnaire"]
tickets: ["ESC-330","ESC-331","ESC-332","ESC-333","ESC-334","ESC-338","ESC-341"]
decisions: [73,71,70,72,75,80]
compared: null
---

**Cette page dit la cible, et ce qui en tourne déjà.** ESC-330 a fait l'étude et le prototype, le portage suit le plan de `docs/PLUGINS.md` §8.10. Seul ce que dit « Ce qui tourne déjà » est dans l'app. Pour le reste, *Les plugins livrés avec Escouade* et *Les programmes JS/TS des plugins, en WASM sans Node* disent ce qui est vrai aujourd'hui.

## Ce qui tourne déjà (ESC-331, ESC-333)

- Un plugin déclare `"run": { "wasm": "Cargo.toml" }` : un programme est une commande, un bundle JS/TS ou un module Rust, un seul des trois, et `wasm` nomme le `Cargo.toml` du plugin, jamais un binaire.
- Son module est `plugin.wasm`, à la racine de son dossier : celui que le store a compilé, écrit à l'installation à la place de tout ce que l'archive aurait sous ce nom, ou celui de l'auteur pour un dossier local. Une version du store sans module n'installe rien. Sans module, le plugin s'arrête et son journal dit pourquoi.
- `run.js` et `run.wasm` ont le même hôte, les mêmes limites et le même journal. Un module Rust ne reçoit pas la première ligne d'un bundle. Il n'a pas de `_start`, il exporte `escouade-turn`, et son `_initialize` est appelé s'il en a un (rustc n'en exporte pas d'un cdylib).
- Le fil d'un plugin attend sur un seul canal, écrit tout ce qui est arrivé sur l'entrée du module, et lance un tour pour le tout.
- Pour `run.js` comme pour `run.wasm`, `net/fetch` est servi à côté du fil : un appel lent ne retient ni le suivant ni le reste du plugin. Seize appels au plus sont en cours à la fois par plugin, les suivants attendent leur tour sans le retenir. Le temps de cette attente ne compte pas dans les 10 s d'un appel.
- Le précompilé se garde par l'empreinte du module et la version de l'app, fait à l'installation.
- `escouade-plugin test` joue un `run.wasm`, son `plugin.wasm` ou le module de `--wasm`. `validate` accepte un `run.wasm` et juge `plugin.wasm` s'il est là.
- `escouade.call`, l'appel synchrone vers l'hôte, compte dans le temps du tour. Il sert les fichiers et les programmes à `run.wasm`. `run.js` y va par une ligne (`p.fs`, `p.runProgram` du SDK) : mêmes refus, même journal. Une réponse trop grande pour la place donnée n'est pas servie deux fois : une écriture n'a lieu qu'une fois. Un plugin lancé par une commande n'a ni l'un ni l'autre.

### Les fichiers d'un dossier déclaré

- Un plugin déclare `"permissions": { "files": [{ "root": "agent", "access": "write" }] }`. Les racines sont une liste fermée, résolue par l'hôte : `plugin` (son dossier, en lecture seule), `agent` (le dossier de l'agent), `agentRepo` (son dépôt), `agentHome` (le dossier de son compte), `temp` (un dossier à lui). Aucun chemin libre, aucun `~`.
- `fs/read`, `fs/write`, `fs/stat`, `fs/list`, `fs/canon` prennent une racine et un chemin relatif, ou un chemin de l'hôte sous une racine de cet agent.
- Le chemin est jugé liens compris, avant de dire s'il existe : il doit être sous une racine de cet agent-là, sinon le refus est le même, qu'il existe ou non. La clôture du projet privé de l'agent est appliquée par l'hôte, quel que soit le code du plugin.
- Une lecture rend 256 Kio au plus ; `limits.fileBytes` peut aller jusqu'à 4 Mio, l'écriture aussi. Chaque écriture et chaque refus vont au journal du plugin.
- Un agent qui arrive reçoit ses racines par `agent/attach`, seulement celles que le plugin déclare. Seuls les plugins de harness reçoivent un agent. `agentHome` n'existe que si l'agent a un dossier de compte à part : le dossier par défaut de chaque harness viendra avec son portage.

### Les programmes

- Un plugin déclare `permissions.commands`. Le premier mot est un nom de programme, trouvé par le cœur sur le PATH, jamais un chemin.
- `proc/spawn` refuse ce qui n'est pas une entrée, mot pour mot ; la suite n'est permise que si l'entrée finit par `*`. Pas de shell, donc sous Windows ni `.cmd` ni `.bat`.
- `dir` est une racine (`agent`, `project`), jamais un chemin, et un projet privé la refuse si le plugin n'y est pas activé. Sans `dir`, le programme tourne dans `temp`.
- L'environnement est celui d'un agent, sans les variables d'Escouade, plus celles que le plugin nomme. Il ne peut nommer ni `ESCOUADE_*`, ni `PATH`, ni ce qui ferait tourner autre chose que le programme déclaré.
- Chaque programme meurt avec l'instance du plugin ou à `proc/kill`, avec ce qu'il a lancé. Quatre à la fois. Sa sortie arrive par `proc/out`, sa fin par `proc/exit` ; l'hôte ne lit plus sa sortie au-delà de 256 Kio qui n'ont pas atteint le module.
- Chaque lancement va au journal avec ses arguments, secrets masqués.
- Le banc joue ces méthodes par le même code, autour d'un agent à lui ; hors ligne, aucun programme ne tourne.

### Ce que l'utilisateur voit

La fenêtre d'installation et Réglages › Plugins disent une ligne par permission, que l'utilisateur peut retirer : « Lit les fichiers de tes agents », « Écrit dans les dossiers de tes agents » en orange, « Lit le dossier de compte de tes agents », et « Lance cette commande sur ton ordinateur » en rouge, suivie de la commande. Pour un plugin en WASM, chacune dit qu'Escouade l'applique ; pour un plugin lancé par une commande, qu'elle est seulement déclarée.

## L'amorce (ESC-338)

La mécanique est dans l'app, et `plugins/seed.json` reste vide tant que la première main n'est pas publiée (Z2) : d'ici là, rien ne change pour l'utilisateur. Le détail est dans *Les plugins livrés avec Escouade*.

## L'état de fin

- Chaque plugin est un module WASM, le même pour tous les systèmes, compilé par le store depuis ses sources et signé : en JS/TS par Javy (`run.js`), en Rust par `run.wasm`.
- Il ne reste aucun programme natif par système : ni `escouade-plugins`, ni `run.command`.
- La première main suit le même chemin qu'un tiers. Chaque plugin vit dans `plugins/<id-court>/` (`plugins/claude`, `plugins/codex`, `plugins/github`…), avec son manifeste, son README, sa LICENSE, son CHANGELOG et ses tests. Ses identifiants `dev.escouade.*` ne changent pas.
- Chacun passe par une étiquette `plugins/<id-court>/vX.Y.Z`, la soumission, la compilation, la revue, la signature du chef et la publication.

## Ce que l'utilisateur voit

- L'app marche dès son installation et hors ligne : elle embarque une amorce, les versions publiées et signées de sa première main, installée sans réseau par le même chemin qu'une installation du store.
- Une version plus récente, du store ou de l'amorce d'une nouvelle app, est proposée, et s'installe au clic avec la fenêtre habituelle : aucune mise à jour automatique, la première main comprise. Une version du store plus récente reste en place à la mise à jour de l'app. On ne redescend jamais de version. Un agent en cours garde sa version jusqu'à son prochain lancement.
- Les plugins de la première main sont des plugins à part entière : ils se coupent et se désinstallent. L'amorce ne remet pas ce que l'utilisateur a désinstallé : elle le note, et la vue Plugins le propose. Un harness ou une source qui n'a plus son plugin le dit, et renvoie à cette proposition.
- Avant de lancer un plugin, l'app vérifie sa signature, les empreintes de son module et de son archive, sa révocation (la liste du store, sinon celle de l'amorce) et sa compatibilité. Un `dev.escouade.*` non signé est refusé.

## Les capacités

Ce qu'un plugin fait hors du protocole se déclare au manifeste, est appliqué par Escouade et se dit dans la fenêtre d'installation : le réseau, y compris les instances que l'utilisateur donne, les flux et le WebSocket ; le serveur local d'un harness lancé par le cœur ; les fichiers de racines nommées, appelés de façon synchrone ; les commandes nommées. Les minuteurs sont un droit de base. Détail : `docs/PLUGINS.md` §8.4.

## Le modèle

Une instance par plugin, sur un fil, réveillée par les événements de l'hôte, un tour à la fois. `wasm32-wasip1`, un module cœur en réacteur : le même hôte que `run.js`.

## Les binaires natifs

Le store refuse `run.command` à la soumission et l'app ne le lance plus, quelle que soit l'origine. Aucun n'est publié aujourd'hui.

## Ce qui tourne déjà : le réseau de l'hôte (ESC-332)

Pour `run.js` comme pour `run.wasm`, appliqué par Escouade et gardé au journal par la méthode et l'hôte, jamais le chemin, la requête ou les en-têtes. Un plugin lancé par une commande reçoit `methodNotFound` pour tout cela.

- **`"network": ["{account}"]`** : l'hôte que l'utilisateur a saisi pour un compte que ce plugin sert, sur le port saisi (sans port saisi, celui du schéma), et lui seul, sans ses sous-domaines. Pour cet hôte, http et le réseau local sont permis. La liste est lue à chaque appel : un compte retiré ferme son hôte. La fenêtre d'installation dit « Se connecte aux instances que tu lui donnes ».
- **`"network": ["local:shared"]`** : http vers `127.0.0.1:<port>`, seulement sur un port que le cœur a lu dans la sortie d'un process partagé qu'il a lancé pour ce plugin (`Launch::shared.listens`, dont l'hôte ne peut être que `127.0.0.1`), tant que ce process vit. Ce trafic ne passe jamais par le proxy. La fenêtre dit « Parle au serveur local du harness, lancé par Escouade ». Ni `{account}` ni `local:shared` n'est un hôte : le store les accepte tels quels.
- **Réglages › Réseau** : le proxy, ses exclusions et les certificats non vérifiés sont suivis, et un changement atteint l'appel suivant, sans relancer le plugin. Le proxy du système n'est pas suivi. Par un proxy, seul le nom est jugé, et la ligne du journal le dit. Le nom du proxy lui-même peut résoudre vers le réseau local. `localhost` et ses sous-domaines sont refusés quand un proxy les emmènerait. Un proxy réglé qui n'est pas une adresse arrête l'appel au lieu de sortir sans lui.
- **`bodyBase64`** dans les deux sens : une réponse UTF-8 vient dans `body`, une autre, entière, dans `bodyBase64`. Rien n'est remplacé.
- **`limits`**, au manifeste : `fetchSeconds` et `fetchBytes`, 10 s et 1 Mio sans eux, plafonnés par l'hôte à 120 s et 32 Mio.
- **`net/stream`**, avec `net/chunk`, `net/end`, `net/credit` et `net/cancel` : les règles de `net/fetch` à l'ouverture. Le plugin n'a jamais plus de 256 Kio non consommés : au-delà, l'hôte ne lit plus la socket. 8 flux à la fois. 120 s sans octet ferment un flux, sauf s'il est déclaré `"sse": true`. Un morceau vient dans `data` s'il est en UTF-8, un caractère coupé attendant la suite, sinon dans `dataBase64`.
- **Le WebSocket** (`ws/open`, `ws/send`, `ws/close`, `ws/message`, `ws/closed`) : `wss` seulement, vers les mêmes hôtes, jamais `local:shared`. 4 sockets, 1 Mio par message. `ws/send` est refusé si ce qui attend dépasserait 256 Kio ; un message plus long passe quand rien n'attend. Un message reçu attend d'être remis au module avant que le suivant soit lu.
- **Les minuteurs** (`timer/set`, `timer/cancel`, `timer/fired`) : un droit de base, 10 ms au moins (ce qui est dessous est ramené à 10 ms), 64 à la fois. `setTimeout` et `clearTimeout` existent en JS.
- Tout ce qui tourne ainsi s'arrête avec l'instance.
- Le SDK donne `openStream` et `openSocket`. `escouade-plugin test --account <adresse>` donne une adresse à `{account}` au banc, qui sert toutes ces méthodes.
- OpenCode n'utilise pas encore `local:shared`, et aucun plugin n'est porté : c'est la suite du plan.

## Ce qui tourne déjà : le kit Rust (ESC-334)

- `escouade-plugin-protocol`, `escouade-plugin-kit` et `escouade-acp` compilent en natif et pour `wasm32-wasip1`, sans tokio ni reqwest sur wasm. Un plugin écrit sur eux garde la même source sur les deux cibles. Aucun plugin livré n'a encore changé de manifeste : ils restent natifs jusqu'à leur portage.
- `initialize` dit à tout plugin la langue de l'interface (`lang`) en plus de la version d'Escouade. C'est par là qu'un module la connaît : il n'a pas d'environnement. Un plugin de la première main y lit sa langue, en module comme en process.
- En module, le réseau d'un plugin du kit passe par `net/fetch` : le proxy, les certificats et le délai restent à l'hôte, et ce que le plugin en dit n'a pas d'effet.
- En module, `sleep` et `timeout` du kit reposent sur les minuteurs de l'hôte. Si l'hôte refuse un minuteur, le journal le dit et l'attente finit aussitôt, pour qu'un appel ne reste pas retenu pour toujours.
- Linear et Codex, compilés depuis leurs crates, répondent dans l'hôte de l'app comme leur natif : Linear à ses appels, Codex ligne à ligne sur les flux enregistrés.
- Les trois crates sont prêtes pour crates.io, avec licence, dépôt et README. Seul le chef les publie.

## Le kit sur crates.io (ESC-341, publication à venir)

Rien n'est publié : seul le chef publie, après l'accord écrit de Florian Gagnaire sur sa contribution. D'ici là, un auteur tiers ne peut pas encore tirer ces crates.

- `escouade-plugin-protocol`, `escouade-plugin-kit` et `escouade-acp` sortent en `0.1.0`, sous licence MIT comme le SDK. Un plugin en Rust garde ainsi le choix de sa licence. Les plugins de la première main et l'app restent en GPL-3.0-or-later.
- Leur documentation, en anglais, se lira sur docs.rs pour le natif et pour `wasm32-wasip1`.
- Les plugins dépendent de ces crates par version. Dans le dépôt, `[patch.crates-io]` les ramène sur `crates/`. Le store, qui ne lit que le dossier du plugin, compilera ce qui est publié.
- Un plugin de la première main ne se compile pas encore seul dans son dossier : il reste membre du workspace de l'app et n'a pas de `Cargo.lock` à lui. C'est à régler avant que le store le compile (`docs/PLUGINS.md` §8.16).
