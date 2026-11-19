---
title: "Les plugins : vue d'ensemble"
subject: "plugins"
sources: ["docs/PLUGINS.md","docs/PLUGINS-DESIGN.md §8.8, §8.33","docs/STORE.md","schemas/plugin-1.json","website/content/docs/fr/plugins/introduction.md","src-tauri/src/harness_scope.rs","issue GitHub #37","crates/escouade-forge/src/integrations/kept.rs","src-tauri/src/plugin_secrets.rs","docs/PLUGINS-CONTRACT-STUDY.md §1.4, §3.3 (release/2.3)","website/content/docs/fr/plugins/manifest.md"]
tickets: ["ESC-184","ESC-339","ESC-404","ESC-355","ESC-499","ESC-524","ESC-525"]
decisions: [142]
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
- **Un agent existant** dont le harness est coupé pour son projet (ou pas encore activé dans un projet privé) ne démarre pas et ne reçoit rien. Sa conversation le dit sur une carte, avec « Activer » : le même accord.
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

## Le contrat du manifeste qui vient (ESC-525)

Les deux lecteurs du manifeste lisent déjà, et refusent hors de leur grammaire, ce que l'étude ESC-501 ajoute au contrat : portée et `repo` des réglages et secrets, groupes, champs sur plusieurs lignes, import d'un fichier, profils, jokers typés des commandes, limite de programmes (*Le contrat du manifeste*). Un plugin qui en demande un point que l'app ne sert pas encore est chargé coupé, la raison dite.
