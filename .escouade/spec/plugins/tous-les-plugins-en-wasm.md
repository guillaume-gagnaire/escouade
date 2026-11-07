---
title: "Tous les plugins en WASM (la cible de la 2.2)"
subject: "plugins"
sources: ["docs/PLUGINS.md §8 (ESC-330)","docs/PLUGINS.md §8.16 (ESC-341)","docs/PLUGINS.md §8.17 (ESC-335)","docs/PLUGINS.md §8.18 (ESC-336)","docs/PLUGINS.md §8.19 (ESC-344)","docs/PLUGINS.md §8.20 (ESC-337)","docs/PLUGINS.md §8.22 (ESC-351)","docs/PLUGINS.md §8.23 (ESC-349)","prototypes/wasm-all/","Décision de l'utilisateur du 2026-11-04 : « en 2.2, tous les plugins, rust ou TS, seront compilés en WASM, c'est acté »","Précision de l'utilisateur du 2026-11-04 : « tous les plugins builtins soient dans /plugins/*, tous publiés sur le store, vérifiés et signés »","Choix de l'utilisateur du 2026-11-05 (ESC-341) : MIT pour les trois crates, avec l'accord de Florian Gagnaire","docs/PLUGINS.md §8.24 (ESC-348)","docs/PLUGINS.md §8.25 (ESC-345)","docs/PLUGINS.md §8.27 (ESC-347)","docs/PLUGINS.md §8.28 (ESC-350)","docs/PLUGINS.md §8.29 (ESC-358)","Accord de l'utilisateur du 2026-11-06 : publication et MIT d'escouade-voice"]
tickets: ["ESC-330","ESC-331","ESC-332","ESC-333","ESC-334","ESC-338","ESC-341","ESC-335","ESC-336","ESC-344","ESC-337","ESC-342","ESC-351","ESC-352","ESC-349","ESC-348","ESC-345","ESC-346","ESC-347","ESC-350","ESC-358","ESC-353"]
decisions: [73,71,70,72,75,80,84,86,89,92,93,96,99,103]
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
- `escouade-plugin test` joue un `run.wasm`, compilé depuis ses sources ou le module de `--wasm`. `validate` accepte un `run.wasm`, juge ses sources et juge `plugin.wasm` s'il est là.
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

## Ce qui tourne déjà : `escouade-plugin build` pour le Rust (ESC-335)

- `escouade-plugin build` compile un plugin `run.wasm` comme un `run.js` : `packs/<id>-<version>.wasm`, son `.sha256`, et en `--json` `file`, `size`, `sha256` et `compiler` (`rust`, `target`, `canon`, `lock`). Pour les mêmes sources et le même `Cargo.lock`, l'empreinte est la même sous Windows et sous Linux, quels que soient le dossier du plugin et son `target`. macOS n'a pas été essayé.
- **La chaîne d'outils est privée.** rustc 1.98.0, cargo et la bibliothèque standard de `wasm32-wasip1` sont téléchargés au premier besoin dans `<données>/tools/rust-1.98.0/`, sans rustup. Une archive dont l'empreinte n'est pas celle épinglée est refusée, et chaque fichier est revérifié à chaque usage : une chaîne changée est téléchargée à nouveau. `ESCOUADE_RUST_TOOLCHAIN` nomme une chaîne déjà là : elle doit être de cette version, avoir la cible et ne pas avoir `rust-src`. `--version` dit la chaîne et l'empreinte de l'enveloppeur.
- **Ce qui entoure le plugin ne compte pas.** La compilation se fait sur une copie du dossier, avec un `CARGO_HOME` à part, `--locked`, et le profil imposé passe sur le `[profile]` de l'auteur. Ni un workspace, ni un `[patch]` à la racine d'un dépôt ne changent le module.
- **`build` refuse, avec la raison en français et en anglais**, et `validate` dit la même chose pour ce qui se juge sans rien lancer : pas de `Cargo.lock` ; `[patch]` ou `[replace]` ; un `path` qui sort du dossier ; une dépendance git ou d'un autre registre ; une cible autre que `cdylib` (une `rlib` à côté est permise) ; un `build.rs` ou une macro procédurale dans les crates du plugin ; `.cargo/config.toml` ; `rust-toolchain` ; `[profile.*.package]` ou `build-override`.
- **La liste fermée.** Tout crate qui tournerait en natif pendant la compilation, un script, une macro ou ce dont ils dépendent, doit être sur la liste d'`escouade-plugin`, par nom, version et somme de contrôle. Ce qui ne tourne qu'en WASM n'a pas besoin d'y être.
- `test` et `dev` compilent un `run.wasm` puis le jouent. Ils ne lisent plus le `plugin.wasm` du dossier. `--wasm` joue toujours le module donné. `validate` juge toujours la forme de `plugin.wasm` s'il est là.
- Le store ne compile pas encore un `run.wasm`, et aucun plugin livré n'est porté : c'est la suite du plan.

## Le kit sur crates.io (ESC-341)

Publié le 2026-11-05 : `escouade-plugin-protocol`, `escouade-plugin-kit` et `escouade-acp` sont sur crates.io en `0.1.0`, après l'accord écrit de Florian Gagnaire sur sa contribution. Un auteur tiers peut en dépendre par version.

- `escouade-plugin-protocol`, `escouade-plugin-kit` et `escouade-acp` sortent en `0.1.0`, sous licence MIT comme le SDK. Un plugin en Rust garde ainsi le choix de sa licence. Les plugins de la première main et l'app restent en GPL-3.0-or-later.
- Leur documentation, en anglais, se lira sur docs.rs pour le natif et pour `wasm32-wasip1`.
- Les plugins dépendent de ces crates par version. Dans le dépôt, `[patch.crates-io]` les ramène sur `crates/`. Le store, qui ne lit que le dossier du plugin, compilera ce qui est publié.
- Un plugin de la première main ne se compile pas encore seul dans son dossier : il reste membre du workspace de l'app et n'a pas de `Cargo.lock` à lui. C'est à régler avant que le store le compile (`docs/PLUGINS.md` §8.16).

## Ce qui tourne déjà : le store compile un `run.wasm` (ESC-336)

Ceci remplace la phrase « Le store ne compile pas encore un `run.wasm` » ci-dessus, et « Rien n'est publié » pour les trois crates : elles sont sur crates.io en `0.1.0` depuis le 2026-11-05.

- Le store compile un `run.wasm` comme un `run.js`, deux fois, et publie le même module : le détail est dans *Publier sur le store*. La fixture du dépôt y donne la même empreinte que sous Windows, dans son bac à sable.
- `escouade-plugin fetch <dossier>` télécharge les dépendances d'un `run.wasm` depuis crates.io sans rien lancer du plugin, après les mêmes refus que `validate`. Un `Cargo.lock` qui ne correspond pas est refusé.
- Aucun plugin de la première main n'est encore compilé par le store : il reste membre du workspace de l'app et n'a pas de `Cargo.lock`, et `escouade-plugin` le refuse donc.
- En production, le Docker par défaut refuse le bac à sable. Tant que l'hébergement ne change pas de réglage, une version `run.wasm` attend.

## Ce qui tourne déjà : Linear, Trello, Jira et ClickUp (ESC-344)

Ces quatre plugins sont les premiers portés. Ce qui précède dit qu'aucun plugin n'est porté et qu'un plugin de la première main ne se compile pas seul dans son dossier : ce n'est plus vrai pour eux.

- Chacun est en `run.wasm`, en version `2.2.0`, `escouade.min` `2.2.0`, ses identifiants inchangés. Il n'est plus dans `escouade-plugins`.
- **Deux compilations.** Dans le dépôt, un second workspace, `plugins/Cargo.toml`, tient les plugins portés et les compile contre `crates/` par son `[patch.crates-io]` : tests de l'app et build de développement (`scripts/plugins-wasm.sh`). Dans son dossier, chaque plugin porté a son propre `Cargo.lock`, qui fixe les crates publiées sur crates.io. Cargo l'ignore dans le workspace ; `escouade-plugin build` et le store le lisent, dans le dossier seul. Le module que le store publie est celui-là. `scripts/plugin-lock.sh` écrit ce lock.
- Pour les mêmes sources, chacun a la même empreinte sous Windows et sous Linux (`docs/PLUGINS.md` §8.19).
- Chacun passe le banc avec `--first-party`, et `plugins/ci.mjs` le compile et le joue.
- Chacun répond comme son natif : mêmes réponses et mêmes requêtes au service, sur neuf appels enregistrés avant le portage (`tests/fixtures/plugins/parity/`).
- **Jira déclare `{account}`** : il joint le site que l'utilisateur a tapé, comme il le faisait en process. C'est la seule permission qui change.
- **Jira sans site** répond comme à un jeton refusé (`-32401`), au lieu d'une erreur non typée.
- **Avant Z2**, l'installeur n'a pas ces modules : l'app ne les trouve que dans un build de développement ou les tests, dans `<target>/plugins-wasm/<id-court>/`. Une release faite avant Z2 n'aurait pas ces quatre plugins.
- Un build de développement et les tests laissent un plugin en WASM joindre un faux service par son adresse (`ESCOUADE_TRELLO_API`, `ESCOUADE_LINEAR_API`, `ESCOUADE_CLICKUP_API`). Une release n'a pas ce chemin.

## La revue locale d'un `run.wasm` (ESC-337)
- La revue recompile les sources et compare l'empreinte au module publié, avant toute signature : sur le poste pour la première main, dans des conteneurs pour un tiers. Voir « La revue "Vérifié par Escouade" ».

## La documentation pour écrire un plugin, et le modèle Rust (ESC-342)

Un développeur externe écrit un plugin en TypeScript ou en Rust à partir du site seul, en français et en anglais.

- `npm create escouade-plugin -- --kind rust` écrit un plugin en Rust qui dit la même chose que le gabarit TypeScript. Il dépend du kit par version, depuis crates.io, avec son propre `Cargo.lock`, et se compile seul dans son dossier. Il passe `cargo test`, `escouade-plugin validate` et le banc. La commande demande le genre parmi `program`, `rust` et `declarative`.
- Le site gagne trois pages : *Les capacités de l'hôte* (ce que chaque capacité permet, ce qu'Escouade refuse, ce que dit la fenêtre d'installation), *Les appels* (chaque méthode, avec un exemple en TS et en Rust) et *Le kit Rust* (les trois crates, le modèle par tours, une source, un hébergeur, un harness, et les refus de la compilation).
- La page de démarrage montre les deux langages côte à côte. Celle de publication dit les onze contrôles, la compilation de Rust, la revue, le sceau et les mises à jour proposées.
- Aucun exemple de code n'est écrit dans une page. Un bloc de TypeScript ou de Rust est fait de lignes d'un fichier compilé, cité sous son chemin : un modèle, un plugin de `plugins/`, ou un des deux plugins de `sdk/doc-examples`, qui font la même chose dans chaque langage. Chaque morceau de manifeste d'une page passe `validate`. Les tests le tiennent.
- Le banc ne sert pas `settings/get`, qui y répond `-32601`. La page de tests le dit.

## Ce qui tourne déjà : les hébergeurs (ESC-345)

GitHub, GitLab, Bitbucket, Gitea et Azure DevOps sont portés comme les quatre sources d'ESC-344 : `run.wasm`, version `2.2.0`, `escouade.min` `2.2.0`, identifiants inchangés, hors d'`escouade-plugins`, un `Cargo.lock` à chacun, le banc `--first-party` vert. Ce qui les distingue :

- **`{account}`** : GitHub, GitLab, Gitea et Azure DevOps le déclarent, pour joindre l'instance que l'utilisateur a saisie, http et réseau local compris. Bitbucket Cloud n'a pas d'instance et ne le déclare pas. L'hôte d'une instance vient toujours du compte, et pendant qu'un compte se vérifie le cœur le prête au plugin.
- **Le plugin suit lui-même les redirections.** L'hôte rend un 3xx au module, pour juger l'adresse suivante comme toute autre. Chaque plugin fait donc ce que faisait son client natif : dix sauts au plus, un GET après 301, 302 et 303, le même appel après 307 et 308. Il garde ses identifiants sur le même hôte et de http vers https, et les retire sinon. Un dépôt renommé continue de marcher, en lecture et en écriture. Une adresse hors du manifeste et hors du compte est refusée et le plugin le dit.
- **Les journaux de GitHub Actions** sont rangés sur un stockage hors de l'API : le manifeste de GitHub déclare `*.actions.githubusercontent.com` et `*.blob.core.windows.net`, joints sans le jeton. Une Enterprise Server dont le stockage est sur un autre hôte que l'instance n'y arrive pas : rien ne peut nommer cet hôte, et le journal dit le refus. Le reste de ce qu'elle sert marche.
- **Azure DevOps et sa page de connexion** : la redirection vers elle se lit comme un jeton refusé, sans la suivre.
- **Les limites** : chaque hébergeur déclare `fetchSeconds: 15`, le délai qu'avait son process, et `fetchBytes: 8388608`. 8 Mio tiennent dans les 64 Mio d'un module, avec de la marge : 16 Mio le faisaient tomber. Un journal plus long, que le process lisait, se dit trop long.
- **Un compte vide** n'est plus une erreur non typée : Gitea part de Codeberg, Azure DevOps répond comme à un jeton refusé (`-32401`). Dans l'app, un compte a toujours son adresse.
- **La parité** couvre tout `forge/*`, plus les sources GitHub Issues, GitLab Issues et Azure Boards, nominal, 401, 404, 429, une lecture et une écriture redirigées : `tests/fixtures/plugins/parity/<nom>-host.json` et `<nom>-source.json`, enregistrés sur le programme d'avant le portage. Deux différences assumées : reqwest ajoutait un `Referer` en suivant une redirection, et Bitbucket compte un saut dans son budget de l'heure, puisque c'en est un.
- **Le fait qu'on puisse joindre un faux service** s'étend en développement et en test à `ESCOUADE_GITHUB_API` et `ESCOUADE_AZURE_DEVOPS_API`. `ESCOUADE_BITBUCKET_API` disparaît : un module n'a pas d'environnement, et rien ne s'en servait.
- **Tests déplacés** : ceux qui appelaient un client directement (son allure, ce qu'il attend) vivent dans la crate du plugin, un `cdylib` ne se liant plus à l'app. Tous les autres tests des hébergeurs passent par le module, sans changer d'assertion, sauf le journal redirigé : sa cible est maintenant un hôte du manifeste.
- **Un corps ne quitte pas le service.** Ceci précise « le même appel après 307 et 308 » ci-dessus. Si un saut sort de l'hôte du service, ou passe de https à http, et que l'appel a encore son corps, quel que soit le 3xx, le plugin s'arrête là : rien ne part vers l'autre hôte, et l'appel finit en erreur avec ce statut. Un appel sans corps, ou dont un 301, 302 ou 303 sur le service a retiré le corps, suit toujours, sans les identifiants. Le natif renvoyait le corps : un stockage comme `*.blob.core.windows.net` est ouvert à tout compte Azure, et un commentaire ou un merge n'a rien à y faire. Les lectures redirigées ne changent pas.

## Ce qui tourne déjà : isola, le premier plugin qui lance un programme (ESC-351)

- isola est en `run.wasm`, en version `2.2.0`, `escouade.min` `2.2.0`, son identifiant inchangé. Il n'est plus dans `escouade-plugins`. Il suit les *deux compilations* de ESC-344 : même module sous Windows et sous Linux (`docs/PLUGINS.md` §8.22).
- **`proc/*`.** L'hôte lance `isola ls --json`, `isola down` et `isola destroy`, les trois lignes de `permissions.commands`, montrées dans la fenêtre des permissions. Un autre programme, ou les mêmes avec un argument de plus, est refusé et va au journal. `isola up` n'y est pas : le cœur le lance dans un terminal, une fois le `.isola.toml` approuvé.
- **Le worktree est prêté le temps d'un appel.** Le plugin reçoit un nom (`agent`), jamais un chemin, et le donne à `proc/spawn`. Le prêt finit avec l'appel : le plugin ne garde aucun worktree. Un plugin qui n'est pas un harness n'a pas d'autre moyen de joindre un worktree.
- **Il répond comme son natif** : mêmes réponses et mêmes commandes sur quinze appels enregistrés avant le portage. Seul le texte d'un programme qui ne démarre pas vient maintenant de l'hôte.
- **Un programme introuvable pour l'hôte n'est pas installé.** Le cœur cherche `isola` comme l'hôte : sous Windows, un `.exe` ou un `.com`, jamais un `.cmd`. Sans lui, un worktree n'est à personne.
- **isola n'a pas de release pour Windows** à ce jour (0.5.0 : Linux et macOS). Le vrai programme n'a pas été essayé avec le module : voir `docs/PLUGINS.md` §8.22, c'est à faire avant Z1.
- **Avant Z2**, comme les quatre de ESC-344, son module n'est pas dans l'installeur.

## OpenCode, le premier harness porté à un serveur local (ESC-349)

OpenCode est en `run.wasm`, en version `2.2.0`, `escouade.min` `2.2.0`, comme les plugins de P1. Ce qui précède dit qu'il n'utilise pas encore `local:shared` : ce n'est plus vrai.

- **Sa crate reste liée au cœur** en bibliothèque (`crate-type = ["cdylib", "rlib"]`), pour la vue OpenCode et les comptes, jusqu'à P8. Elle est membre du workspace de `plugins/`, pas de celui de `src-tauri`, et n'a plus de `tokio` ni de `reqwest`, sur aucune cible.
- **Il n'utilise pas le kit**, seulement `escouade-plugin-protocol` : le réacteur du kit ne sait pas lancer une tâche qu'aucun appel du cœur n'a commencée, ce qu'est la lecture du flux. Son module se compile donc sur la version publiée sans changer le kit. Le détail est à la page « OpenCode en plugin ».

## Ce qui tourne déjà : Antigravity CLI (ESC-348)

- Antigravity CLI est le premier harness en `run.wasm` (voir *Antigravity CLI en plugin*), en version `2.2.0`, `escouade.min` `2.2.0`, ses identifiants inchangés. Il n'est plus dans `escouade-plugins`. Il dit ce que disait son natif sur les treize flux enregistrés d'`agy` (`tests/fixtures/plugins/parity/antigravity.json`).
- **Sa cible porte aussi une `rlib`** : la vue terminal du cœur lit ses mots jusqu'à P8. Le cœur le garde donc en dépendance, hors des membres de son workspace.
- **`agent/items` atteint tout agent d'un plugin de harness**, plus seulement ceux d'un process partagé. Pour un agent dont le programme est le sien, ce qu'un plugin dit sans qu'on lui demande passe derrière la réponse du lot d'avant.
- **`fs/read` prend `tail`** : un fichier au-delà de la limite rend sa fin et `cut`, à partir d'un caractère entier, au lieu d'échouer. Sans `tail`, rien ne change.
- Comme Linear et les autres, son module n'est pas dans l'installeur : avant Z2, seuls un build de développement et les tests le lancent. **Une release sortie avant Z2 n'aurait pas Antigravity CLI.**

## Les harness en modules : Codex, Copilot CLI, Cursor CLI et Gemini CLI (ESC-346)

Ces quatre plugins sont en `run.wasm`, en version `2.2.0`, `escouade.min` `2.2.0`, leurs identifiants inchangés. Ils ne sont plus dans `escouade-plugins`. Le cœur lance toujours leur harness et ne lie plus que Codex et Copilot, pour leurs vues, jusqu'à P8. Ce qui précède disait que le harness est servi par un process : ce n'est plus vrai pour eux.

- **Ce que l'installation dit.** Codex lit le dépôt de l'agent et le dossier de son compte ; Copilot CLI lit le dépôt ; Gemini CLI lit le dossier du compte ; Cursor CLI ne déclare aucun fichier.
- **`git add` et `git commit` restent acceptés sans carte** pour Codex et Copilot CLI, dans les modes qui le permettent. C'est maintenant Escouade qui regarde la configuration du dépôt et ses hooks, à chaque demande, par `fs/gitProtected` : un module ne lit pas `.git` et ne lance pas `git`. La méthode prend le dossier d'un agent de ce plugin et demande la racine `agentRepo`. Si la permission est retirée, ou si git ne répond pas en une seconde, la carte s'affiche : le pire cas est une question de plus, jamais une commande acceptée à tort.
- **Le plan de Gemini** se lit par l'hôte, dans le dossier du compte de l'agent : la carte « Plan à approuver » montre le plan comme avant. Sans cette permission, elle montre le nom du fichier.
- **La clôture sur les permissions** (Cursor, Gemini) reste insensible à la casse sous Windows : un module décide d'après le chemin.
- **Le dossier d'un compte par défaut.** Un agent sans dossier de compte a celui où son harness regarde : `~/.codex` (ou `CODEX_HOME`), `~/.copilot`, `~/.cursor`, `~/.gemini`. Pour Gemini, c'est ce dossier-là et jamais tout `~`.
- **Une adresse dans une permission de Copilot** dont l'hôte n'est pas en ASCII n'est permise que pour une fois, au lieu de la session.
- **Chacun répond comme son natif**, sur 2 256 pas enregistrés avant le portage : ses lancements, ses flux enregistrés, les appels du contrat (`tests/fixtures/plugins/parity/`).
- **Le service du protocole ACP** est dans la crate `escouade-acp`, plus dans `plugins/acp`. Cursor et Gemini en dépendent par version, comme tout plugin que le store compile seul. Cela demande `escouade-acp` `0.1.1` sur crates.io, publiée par le chef après la fusion d'ESC-346 et d'ESC-347. Tant qu'elle n'y est pas, ces quatre plugins n'ont ni `Cargo.lock` ni empreinte, ne se compilent que dans le dépôt, et `plugins/ci.mjs` les écarte de `validate` et de `build` par une liste que le second passage vide.

## Ce qui tourne déjà : ACP et Claude Code (ESC-347)

Ces deux plugins sont portés à leur tour. Goose, manifeste sans code servi par ACP, tourne donc sur le module.

- Chacun est en `run.wasm`, en version `2.2.0`, `escouade.min` `2.2.0`, ses identifiants inchangés. Il n'est plus dans `escouade-plugins`.
- Chacun répond comme son natif, à l'élément près, sur chaque ligne des flux enregistrés de ses tests et sur chaque méthode du contrat (`tests/fixtures/plugins/parity/`).
- **Claude Code ne déclare aucune permission.** Ce qu'il garde d'une connexion (`.credentials.json`, le trousseau de macOS) n'est lu que par le cœur : le plugin ne voit jamais une connexion. Il reste aussi une bibliothèque pour le cœur, qui lit sa traduction pour « Vue Claude Code », jusqu'à P8.
- **ACP déclare `agent` en écriture et `agentRepo` en lecture.** Les fichiers qu'un agent lui confie passent par `fs/*` de l'hôte, et la clôture du projet privé y est appliquée. Un agent ACP lit et écrit donc moins qu'avant par Escouade : voir *Les harness ACP décrits*.
- **`limits.memoryBytes` et `limits.turnSeconds`.** Un plugin en WASM peut demander plus que les 64 Mio de mémoire et les 2 s d'un tour par défaut, 256 Mio et 10 s au plus, jamais moins. Une ligne plus longue que le défaut n'est ni un droit ni une capacité : ce n'est pas dit dans la fenêtre d'installation. Un tour qui tient une décision garde ses 500 ms. Claude Code et ACP demandent les deux plafonds : un message peut porter 24 Mio en base64, et une instance sert tous les agents du harness. Une fois montée, la mémoire d'une instance ne redescend pas avant son arrêt.
- Dans les tests, le précompilé d'un module de `scripts/plugins-wasm.sh` se garde à côté de lui. Une release n'a pas ce chemin.
- **`escouade-acp` passe à `0.1.1`, publiée par le chef après la fusion.** Tant qu'elle n'est pas sur crates.io, le plugin ACP n'a ni `Cargo.lock` ni empreinte : il se compile dans le dépôt et passe le banc par `--wasm`, mais `escouade-plugin validate` et `build` le refusent seul dans son dossier, donc le store ne le compile pas. `plugins/ci.mjs` l'écarte de façon explicite (`AWAITING_PUBLICATION`), et un test refuse un lock posé sans retirer cet écart.
- **Avant Z2**, comme pour les plugins précédents, seuls un build de développement et les tests trouvent ces modules. Une release faite avant Z2 n'aurait pas Claude Code.

## La voix et Vibe (P6, ESC-350)

La voix est le cas le plus serré du chantier : du son binaire, un WebSocket, un flux, et un délai de l'ordre de la milliseconde. Elle sort dans la crate `crates/escouade-voice`, en MIT comme le kit : ce que chaque service demande et ce que ses réponses veulent dire en code pur, le réacteur d'OpenCode augmenté de `ws/*`, et le service de `voice/*`. Quatre plugins y tiennent en une ligne ; Mistral y ajoute Vibe, sur le client d'`escouade-acp`, dans la même instance. L'app garde la capture, Whisper, Piper, le trousseau et le choix du service, et lit la crate pour décrire les cinq services sans démarrer leur module.

- **La parité** : `tests/fixtures/plugins/parity/voice-<id>.json` et `mistral.jsonl` tiennent ce que le natif a répondu et demandé. Aucun écart, en dehors de ce que dit le §8.28 de `docs/PLUGINS.md`.
- **La dictée passe en PCM 16 bits** dans `voice/transcribe` : dix minutes de son en flottants ne tenaient pas dans la mémoire d'un module.
- **`ws/open` dit ce que le service a répondu**, sans les valeurs de ses en-têtes : la raison d'un refus reste visible.
- **Il n'y a plus de service dans le cœur.** Un plugin qui ne démarre pas se dit à son premier appel.
- **Second passage** : `escouade-voice` `0.1.0` et `escouade-acp` `0.1.1` à publier, puis les `Cargo.lock` et les empreintes des cinq. D'ici là, ils restent dans `AWAITING_PUBLICATION`.

## Le kit 0.1.1 sur crates.io (K1, ESC-358)

Publiés le 2026-11-06 par l'utilisateur : `escouade-plugin-kit` `0.1.1`, `escouade-acp` `0.1.1`, `escouade-voice` `0.1.0`, en MIT. `escouade-plugin-protocol` reste en `0.1.0`. Ceci remplace, plus haut, tout ce qui attendait ces versions : `AWAITING_PUBLICATION` n'existe plus, et les 23 plugins en `run.wasm` ont leur `Cargo.lock` sur ce qui est publié.

- **Une tâche réveillée repart dans le même tour.** En module, un `Waker` marche comme ailleurs : une tâche qui attend ce qu'une plus récente pose n'a plus besoin d'une ligne de plus. Un tour est borné : des tâches qui se réveillent sans fin passent au tour suivant par un minuteur vide, et les autres lignes passent.
- **Le client du kit suit les redirections.** Ceci remplace « Le plugin suit lui-même les redirections » : les règles sont celles d'ESC-345, et elles sont dans le kit. En module, `send` les applique à tout plugin du kit, donc aussi à Jira, Linear, Trello et ClickUp, qui finissaient en erreur sur un 3xx. Hors du service, seuls `Accept`, `Accept-Language` et `User-Agent` restent, et un appel qui a encore son corps s'arrête là. Un hébergeur garde la main à chaque saut par `http::redirect`.
- **`host::Host` donne les mêmes tours à un plugin qui sert ses lignes lui-même**, avec `net/stream`, `ws/*` et des tâches qu'aucun appel n'a commencées. OpenCode et la voix s'en servent : ceci remplace « Il n'utilise pas le kit » pour OpenCode. Un minuteur refusé va au journal du plugin.
- **Un corps qui n'est pas de l'UTF-8** arrive au client du kit avec ses caractères de remplacement, au lieu d'arriver vide.
- **`proc/*` reste sans API dans le kit** : seul isola lance un programme, et le réacteur réglé, son code n'a plus de contournement.

## Le cœur ne lie plus aucun plugin (P8, ESC-352)

Ce qui est dit plus haut des crates liées au cœur « jusqu'à P8 » est fini : `src-tauri/Cargo.toml` ne dépend d'aucun dossier de `plugins/`, et les crates de harness sont des `cdylib` seules. Les vues terminal de Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI, Mistral Vibe et Antigravity CLI passent par leur module (`view/translate`, voir *Le contrat des harness en process*). Le cœur garde `crates/escouade-acp`, `escouade-voice` et le kit, qui ne sont pas des plugins.

Le contrat a grandi : `escouade-plugin-protocol` 0.1.1 et `escouade-acp` 0.1.2, qui porte les vues de Cursor CLI, Gemini CLI et Mistral Vibe, sont publiées. Antigravity CLI, Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI et Mistral compilent depuis crates.io. Leurs empreintes ont changé, comme celle d'OpenCode, redevenu un `cdylib` seul ; les 15 autres plugins n'ont pas bougé.

## La publication de la première main (ESC-353)

- `npm run plugins:publish` enchaîne les gestes de chaque plugin ; le détail est dans *La revue « Vérifié par Escouade »*. Rien n'est publié tant que le chef ne l'a pas lancée.
- **Le store n'a plus de liste à lui.** Un plugin de la première main y est une vraie ligne du catalogue, avec ses versions, ses installations et son sceau, ou « En revue ». Il n'y est plus tant qu'il n'est pas publié : la liste « livrés avec Escouade » du store est vide, et *Les plugins* de l'app ne changent pas.
- Le préfixe `dev.escouade.*`, réservé au dépôt de l'app, donne sa marque : « Escouade » à la place du compte, le lien vers `/authors/escouade`, et sa tuile (la marque de GitHub, Gitea ou OpenCode, sinon le glyphe d'un harness, d'un hébergeur ou d'une source). Un plugin d'un autre genre garde ses lettres.
- Chaque manifeste qui n'a pas encore de version au store dit ses `store.categories`, comme un tiers : une première version sans catégorie arrête la commande.

- **Une exception à la phrase précédente** : Cursor CLI, Gemini CLI et Mistral lisent leur manifeste depuis leur module, qui l'embarque. Une clé `store` y changerait leur empreinte : leurs catégories restent dans la commande jusqu'à leur prochaine version. Les 21 autres modules, et Goose, les déclarent à leur manifeste, sans effet sur leur empreinte.
- **Ce qui est déjà au store ne repart pas.** La commande passe par les 23 modules et par `slack`, `team-rules`, `discord`, `high-contrast` et `goose`. Une version signée au catalogue n'est pas touchée. Une version publiée mais non signée va à sa revue, sans nouvelle soumission. Seul le numéro de version compte : un dossier changé sans nouvelle version ne repart pas.
