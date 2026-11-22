---
title: "Les plugins : vue d'ensemble"
subject: "plugins"
sources: ["docs/PLUGINS.md","docs/PLUGINS-DESIGN.md §8.8, §8.33","docs/STORE.md","schemas/plugin-1.json","website/content/docs/fr/plugins/introduction.md","src-tauri/src/harness_scope.rs","issue GitHub #37","crates/escouade-forge/src/integrations/kept.rs","src-tauri/src/plugin_secrets.rs","docs/PLUGINS-CONTRACT-STUDY.md §1.4, §3.3 (release/2.3)","website/content/docs/fr/plugins/manifest.md","docs/PLUGINS-CONTRACT-STUDY.md §4.4, §7 (release/2.3, ESC-501)","crates/escouade-plugins/src/secrets.rs"]
tickets: ["ESC-184","ESC-339","ESC-404","ESC-355","ESC-499","ESC-520","ESC-524","ESC-525","ESC-527","ESC-582","ESC-579"]
decisions: [142,204,247,259]
compared: null
---

# Les plugins : vue d'ensemble

Ce qui suit est la carte du sujet. La référence est `docs/PLUGINS.md`, la place de marché `docs/STORE.md`. Chaque point a sa page dans ce sujet ou dans `store/`.

## Ce qu'est un plugin

Un dossier avec un manifeste (`escouade-plugin.json`), de trois genres :

- **déclaratif** : rien ne s'exécute ; ses contributions sont lues et rendues par Escouade ;
- **programme en JS ou en TS** (`run.js`) : le store le compile en WASM, l'app le fait tourner dans son process, sans Node, le réseau et les fichiers appliqués (*Les programmes JS/TS des plugins, en WASM sans Node*) ;
- **programme en Rust** (`run.wasm`) : le store compile son crate en module, l'app le fait tourner de même (*Tous les plugins en WASM*).

Un **programme natif** (`run.command`) ne démarre plus, quelle que soit l'origine du plugin (ESC-355) : le manifeste se lit, la fiche dit pourquoi, et le store le refuse. Tous parlent le même protocole, JSON-RPC 2.0, version 1. Aucun HTML de plugin n'entre dans la fenêtre.

## Ce qu'un plugin peut apporter aujourd'hui

- réagir aux 39 faits d'Escouade, par son programme, un webhook ou une commande déclarés ;
- décider des autorisations des agents (*Règles d'autorisation des plugins*) ;
- un thème ;
- un harness : décrit par ACP sans code (*Les harness ACP décrits*), ou, pour la première main, par un module (*Le contrat des harness en process*) ;
- un genre de compte, une source du Kanban, un hébergeur de code, un service de voix, un environnement de lancement ;
- des réglages et des secrets, rendus par Escouade. Un réglage a `key`, `type` (`string`, `number`, `boolean`, `select`) et `title`, et peut avoir `description` (montrée sous son titre), `default` (sa valeur tant que l'utilisateur n'en a pas choisi) et, pour un `select`, `options` ; la page du plugin les montre dans l'ordre du manifeste, les réglages puis les secrets.

Le lecteur de workflow a son contrat, mais rien de l'app ne l'appelle encore. Les outils MCP apportés par un plugin, les textes rédigés, les langues, les statistiques, Ctrl+K, les canaux de notification déclarés et les vues libres ne sont pas construits.

## Les points réservés (ESC-524)

Le manifeste réserve `menus`, `statusBar`, `badges` et `projectTemplates` (et lit puis ignore `escouadeYml`, que la documentation ne cite plus). Ils restent acceptés, de la forme d'aujourd'hui : aucun manifeste publié n'est refusé pour eux. Mais rien ne les sert, donc :

- **la fenêtre ne les annonce pas** : ni dans « Ce qu'il ajoute » (fenêtre d'installation, depuis un dossier ou le store, et page du plugin), ni dans la ligne de Réglages › Plugins, et ils ne rangent pas un plugin dans « Outils ». Un plugin qui ne déclare qu'eux n'ajoute rien, et le dit comme un plugin sans contribution ;
- **`escouade-plugin validate` et le contrôle du manifeste du store** (le lecteur partagé) disent chacun d'eux « réservé, sans effet dans cette version », un avertissement : la version passe.

Concevoir `statusBar` (le plus demandé) est laissé à une étude à part.

## D'où ils viennent

- **Du store** : store.escouade.dev, revue en local et sceau « Vérifié par Escouade », vérifié par l'app avec ses clés embarquées (*Le store dans l'app*). Les plugins de première main (`dev.escouade.*`, `plugins/<id-court>/`) y passent comme ceux d'un tiers, signés seulement, et l'installeur en porte une version, l'amorce (*Les plugins d'Escouade*). L'app n'en donne plus aucun : l'origine `builtin` a disparu (ESC-355).
- **D'un dossier ou d'un dépôt git** : « non vérifié », jamais un id `dev.escouade.*`.
- **En développement** : un dossier chargé en place, relu à chaque changement.
- **Une build de développement** lance aussi les modules du dépôt (`scripts/plugins-wasm.sh`) que l'amorce n'a pas, origine `workspace`, jamais dans une release.

## Ce que garde l'utilisateur

Installé n'est pas activé : par défaut actif partout sauf dans les projets privés, où il s'active un par un. Cela vaut pour tous les plugins, ceux de la première main compris. Avant l'installation, une fenêtre dit d'où il vient et ce qu'il pourra faire. Après : désactiver, retirer une permission, désinstaller, lire son journal. Une mise à jour est proposée, jamais installée seule. Un projet propose des plugins (`plugins.recommended`), il n'en installe aucun.

Le réseau déclaré est appliqué par Escouade pour un programme en WASM et pour les webhooks d'un plugin déclaratif.

## Le bouton projet par projet d'un plugin de harness (ESC-404)

Le choix d'un plugin pour un projet (`plugins::active_in`) vaut pour ses faits, ses dossiers, ses autorisations, et pour son harness : le module d'un harness reçoit tout ce que fait l'agent (ce qu'il écrit et lit, ses commandes, son transcript). Le cœur en décide, pour les harness de la première main comme pour un harness ACP décrit (son plugin est celui qui le décrit).

- **Coupé pour un projet**, le harness n'y est plus proposé : nouvel agent, ticket, réglages du projet, nouvelle discussion. Le harness déjà choisi reste affiché, avec « (coupé pour ce projet) ». Le cœur refuse d'y créer un agent sur lui, de démarrer, de reprendre ou d'envoyer un message à un de ses agents, et d'y ouvrir sa vue terminal ; la relève ne va pas vers lui. Le rédacteur des textes d'un projet le choisit parmi les harness allumés pour lui.
- **Couper un harness pour un projet** arrête ses agents de ce projet (tour interrompu, process et vue terminal fermés), comme le couper pour l'app arrête tous les siens ; le ticket d'un de ces agents est bloqué, avec la raison.
- **Dans un projet privé**, coupé d'office, le harness reste proposé : le premier agent sur lui demande une fois « Activer <harness> pour ce projet privé ? ». La fenêtre dit ce que le plugin recevra et liste ses permissions comme la fenêtre d'installation. « Activer » allume le plugin pour ce projet seulement, puis l'agent part ; « Annuler » ne fait rien partir.
- **Un agent existant** dont le harness est coupé pour son projet (ou pas encore activé dans un projet privé) ne démarre pas et ne reçoit rien. Sa conversation le dit sur une carte, avec « Activer » : le même accord. Son process qui tourne encore (le projet devenu privé depuis) est jugé comme un démarrage : au repos il s'arrête aussitôt, au travail à la fin de son tour, et sa conversation dit pourquoi (ESC-582, voir *Projets privés*).
- **Le Kanban, le pilote auto et les outils MCP** ne posent pas la question à la place de l'utilisateur. Un ticket dont le harness est coupé pour son projet est bloqué avant que rien ne soit créé (ni branche, ni agent), avec la raison ; sa carte porte « Activer », qui allume le plugin puis reprend le ticket. `create_agent` et `start_ticket` refusent, avec la raison. Aucun outil MCP n'allume un plugin pour un projet : c'est le geste de l'utilisateur.
- **À la mise à jour** qui applique cette règle, une fois : un projet privé qui a déjà des agents non archivés sur un harness a ce plugin allumé pour lui, là où l'utilisateur n'avait rien choisi. Les autres restent coupés d'office, et un choix fait ensuite n'est plus touché.

## L'essai réel

Fait le 2026-11-04 sur Windows, depuis store.escouade.dev (ESC-184) : voir `docs/PLUGINS-DESIGN.md` §7. macOS et Linux restent à faire à la release.

## Les secrets d'un plugin, et un poste sans trousseau (ESC-499)

Un plugin déclare ses secrets (`contributes.secrets`) ; l'utilisateur les saisit dans l'onglet Réglages de sa page, et ils vont au trousseau du système sous `plugin:<id>:<clé>`. Le plugin les demande par `secrets/get`.

- **« Garder » n'échoue jamais en silence** (issue #37). Le secret est écrit puis relu. S'il n'est pas gardé, la cause est dite sous le champ, en français et en anglais. Sans trousseau (Linux, WSL sans Secret Service) : « Pas de trousseau système sur ce poste : le secret n’a pas été gardé. » Si un trousseau refuse : « Le trousseau système a refusé le secret (<sa raison>) : il n’a pas été gardé. », la valeur jamais dans la raison. La ligne dit toujours « aucun ». Sur un poste sans trousseau, la note des secrets le dit aussi.
- **Le repli est choisi, jamais pris tout seul.** Quand le trousseau a refusé, la ligne offre « Garder dans un fichier lisible de toi seul », avec ce que cela veut dire : le secret est écrit en clair sur le disque, dans `plugin-secrets.json` du dossier de données d'Escouade, avec les droits du seul propriétaire. Le choix vaut pour ce poste (le fichier est dans le dossier de données, jamais dans un projet ni dans `.escouade.yml`). Ensuite, un secret gardé là se garde de nouveau là, et la ligne dit « gardé dans un fichier lisible de toi seul ». Sous Windows et macOS, le trousseau existe : le fichier n'y est offert que si le trousseau refuse.
- **Le fichier** est créé avec ses droits dès l'ouverture (Unix `0600` sur le fichier temporaire, puis mis en place) et écrit par le chemin atomique de l'état (voir *Données locales et coupures*). Il n'est jamais copié dans les copies de l'état (`backups/`), ni écrit dans un journal ou un message d'erreur. Il ne reçoit que les secrets des plugins : le jeton d'un compte, une clé d'API ou une clé de voix gardent leurs règles.
- **Le plugin lit son secret où qu'il soit gardé.** Le fichier choisi est lu avant le trousseau, par `secrets/get` comme par les gabarits des webhooks (`{{secrets.<clé>}}`). Le protocole ne change pas : `secrets/get` rend `null` pour un secret absent, qu'il n'ait jamais été saisi ou que le trousseau l'ait refusé. Rien n'est ajouté pour distinguer ces deux cas, car c'est l'utilisateur qui peut y remédier et la fenêtre le lui dit au moment du geste (voir `docs/PLUGINS.md`).
- **Un trousseau qui arrive ensuite** ne déplace rien tout seul : le fichier reste lu, et la ligne d'un secret qui y est offre « Déplacer vers le trousseau ». Le secret est écrit au trousseau, relu, puis retiré du fichier.
- **« Oublier »** retire le secret du fichier ou du trousseau. Désinstaller un plugin sans garder ses secrets les retire aussi.
- **Réglages › Plugins** montre le choix et le retire (voir *Réglages*).
- Une stratégie de poste géré qui interdirait ce repli n'est pas dans la 2.2.

### Le fichier des secrets interdit par une stratégie (ESC-520)

Un poste géré ou une équipe peut interdire ce fichier (`secrets.fileFallback`, voir *Réglages imposés*). Dans ce cas, « Garder dans un fichier lisible de toi seul » n'est plus offert : le cadenas dit qui l'interdit. Le cœur refuse aussi d'y écrire. Un fichier déjà choisi n'est plus lu, donc `secrets/get` rend `null` pour ses secrets. Il reste dans Réglages › Plugins, sous cadenas, avec ses secrets : on peut les déplacer vers le trousseau ou les effacer avec « Retirer le fichier ».

## Le contrat du manifeste qui vient (ESC-525)

Les deux lecteurs du manifeste lisent déjà, et refusent hors de leur grammaire, ce que l'étude ESC-501 ajoute au contrat : portée et `repo` des réglages et secrets, groupes, champs sur plusieurs lignes, import d'un fichier, profils, jokers typés des commandes, limite de programmes (*Le contrat du manifeste*). Un plugin qui en demande un point que l'app ne sert pas encore est chargé coupé, la raison dite.

## Un secret long, en morceaux au trousseau (ESC-527)

Le trousseau de Windows refuse un secret de plus de 1 280 caractères (`keyring` 3.6.3, `CRED_MAX_CREDENTIAL_BLOB_SIZE`). Une configuration OpenVPN avec ses certificats fait 4 à 8 Ko. Le magasin des secrets (`Chunked`, dans `escouade-plugins::secrets`) découpe donc un secret long. Il enveloppe le trousseau sur les trois systèmes, et le faux trousseau des tests. macOS et Linux n'ont pas la limite, mais lisent et écrivent le même format. Tous les secrets de l'app y passent : ceux des plugins, les jetons des comptes, les clés de voix, le jeton MCP.

- **Un secret court** (1 200 unités UTF-16 au plus) reste une seule entrée sous son nom, comme avant. Ce qui est déjà gardé se lit comme avant.
- **Un secret long** est coupé en morceaux de 1 200 unités UTF-16, jamais au milieu d'un caractère. Ils sont rangés sous `<nom>#a1`, `#a2`… ou `#b1`, `#b2`… Il y a deux emplacements pour que les nouveaux morceaux ne prennent jamais la place des anciens avant la bascule. L'entrée `<nom>` garde l'en-tête : `escouade-pieces:v1:<emplacement>:<nombre>:<SHA-256 du tout>`. Un secret qui commence comme un en-tête est lui aussi gardé en morceaux, pour qu'un en-tête ne soit jamais pris pour un secret.
- **La lecture** recolle les morceaux et vérifie l'empreinte. Un morceau manquant, un morceau en trop, une empreinte fausse ou un en-tête illisible rendent une erreur, jamais un secret tronqué. Un plugin reçoit alors l'erreur du trousseau (`-32002`).
- **L'écriture est entière ou n'est pas.** Les nouveaux morceaux sont écrits dans l'emplacement que l'en-tête ne nomme pas, et ceux qui traîneraient au-delà dans cet emplacement sont effacés. Puis l'en-tête bascule. Enfin, les anciens morceaux et les restes de l'autre emplacement sont effacés, du dernier au premier. Si un morceau est refusé, ce qui a été écrit est effacé, et l'ancien secret reste lisible. Après une coupure, on lit l'ancien secret tant que l'en-tête n'a pas basculé, le nouveau ensuite. Les restes sont effacés à l'écriture suivante ou à « Oublier ».
- **« Oublier »** efface les morceaux des deux emplacements, puis l'en-tête. Coupé entre les deux, l'en-tête dit qu'un morceau manque : c'est une erreur, et oublier de nouveau termine le travail. La désinstallation d'un plugin et « Déplacer vers le trousseau » traitent un secret long comme un seul secret. Le fichier lisible de toi seul (ESC-499) garde toujours un secret d'un seul tenant.
- **64 Kio au plus par secret**, au trousseau comme dans le fichier. Au-delà, rien n'est écrit, et la fenêtre dit « Ce secret dépasse 64 Kio : il n’a pas été gardé. »
- **`#` est réservé aux morceaux.** Une clé de `contributes.secrets` n'en contient jamais (le lecteur du manifeste ne prend que lettres, chiffres, `_`, `.` et `-`). Une clé de `permissions.secrets` qui en contient n'est ni montrée ni gardée. Pour `secrets/get`, ce n'est jamais une clé du plugin (`-32001`). Le magasin refuse tout nom de morceau (`…#a<n>`, `…#b<n>`).
- **Le journal d'un plugin** masque un secret entier, comme avant. Pour un secret de plusieurs lignes, il masque aussi chacune de ses lignes de plus de 8 caractères.
- **Essai réel** : fait le 2026-11-18 sous Windows 11, avec le vrai trousseau et un service à part (`…trial-esc527`). Une valeur factice de 6 Ko est refusée d'un seul tenant, puis gardée en 6 morceaux, relue, réécrite dans l'autre emplacement, raccourcie, oubliée. Aucune entrée de l'essai ne reste. C'est le test ignoré d'office `real_keychain` de `crates/escouade-forge/src/integrations/secrets.rs`.

## Le store d'une équipe (ESC-579)

Une origine de plus : **le store privé d'une équipe** (`team-store` pour les stratégies), voir *Le store de plugins privés*. Un plugin de l'équipe est signé par la clé de l'équipe, jamais un id `dev.escouade.*`, toujours dans l'espace de noms de l'équipe ; il s'installe depuis la vue Plugins, « Équipe », par le geste du membre, après la fenêtre de ses permissions. Sa page dit « Acme · privé ». Il ne prend jamais la place d'un plugin du même id venu d'ailleurs, ni l'inverse : l'identité d'un plugin installé est son origine plus son id.
