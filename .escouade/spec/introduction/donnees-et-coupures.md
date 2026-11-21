---
title: "Données locales et coupures"
subject: "introduction"
sources: ["Ticket ESC-469 (perte de state.json le 2026-11-15)","Ticket ESC-487 (l'écran des copies, après la reconstruction à la main du Kanban le 2026-11-15)","crates/escouade-plugin-cli/src/paths.rs","crates/escouade-model/src/backups.rs","crates/escouade-model/src/backup_view.rs","src-tauri/src/copies.rs","src-tauri/src/mcp/read.rs","src-tauri/src/conv.rs","src-tauri/src/core.rs","src/components/RecoveryBanner.svelte","src/components/settings/BackupsSection.svelte","website/content/docs/fr/data.md","website/content/docs/fr/troubleshooting.md","Ticket ESC-535 (app.log du poste de l'utilisateur, « cannot save state … os error 2 » à la sortie pour une mise à jour, 2026-11-17 et 2026-11-18)","src-tauri/src/save_tests.rs"]
tickets: ["ESC-469","ESC-487","ESC-499","ESC-535"]
decisions: [180,191,243]
compared: null
---

## Ce que c'est

Escouade garde tout ce qu'elle ne peut pas refaire dans son dossier de données (`~/.escouade`, ou `ESCOUADE_DATA_DIR`). Une coupure du poste (courant, arrêt forcé, plantage du système) ne doit jamais lui faire perdre son état : ses fichiers sont écrits jusque sur le disque, elle en garde des copies, un fichier illisible au démarrage est remplacé par sa dernière bonne copie, en le disant, et l'utilisateur voit, compare et reprend une copie depuis les réglages.

Origine : le 2026-11-15, une coupure a laissé `state.json` (8,7 Mo) à sa taille, rempli de zéros. Escouade 2.2.0 l'avait écrit par un fichier temporaire renommé, sans forcer les données sur le disque (NTFS journalise le renommage avant les données), puis, ne pouvant le lire, était repartie d'un état vide sans rien dire et l'avait écrit par-dessus : 468 tickets, 490 agents et les projets perdus. Le Kanban (461 tickets) a dû être reconstruit à la main depuis les journaux, les historiques des projets fermés n'ayant aucune copie (ESC-487).

## Comportement

### L'écriture

- **Ce qui ne se refait pas** est écrit pour tenir à une coupure : un fichier temporaire à côté, propre à cette écriture (`<fichier>.<process>-<n>.tmp`, par exemple `state.json.4242-17.tmp`), ses données forcées sur le disque (`sync_all`), puis mis à sa place ; le dossier est forcé aussi (Unix : `sync_all` du dossier ; Windows : remplacement « write-through », `MoveFileExW` avec `MOVEFILE_WRITE_THROUGH`, et le renommage ordinaire si Windows le refuse, les données étant déjà sur le disque). Après une coupure, le fichier a son contenu d'avant ou le nouveau, entier. Le temporaire est effacé quand l'écriture échoue, et l'erreur est rendue à l'appelant.
- **Deux écritures d'un même fichier au même moment** (deux fils, ou l'app et `escouade-plugin`) ne partagent jamais de temporaire : aucune n'échoue, et le fichier a le contenu entier de l'une d'elles, celle mise en place la dernière (ESC-535). Avec un seul nom pour toutes (`<nom>.tmp` jusqu'en 2.2.3), la seconde trouvait le sien déjà renommé par la première (« os error 2 »), ou en renommait un que l'autre remplissait encore.
- **Un fichier dont le contenu doit suivre l'ordre de ses changements** prend son instantané et l'écrit sous un même verrou, pour que le dernier instantané pris soit le dernier écrit :
  - l'état (`state.json`) : toutes ses sauvegardes passent par un seul chemin sérialisé (l'arrêt, la sauvegarde périodique de 400 ms, la fermeture ou le retour d'un projet, la suppression d'un ticket qui a des images) ;
  - les réglages : la sauvegarde de la fenêtre, le port du serveur MCP, l'identifiant de l'utilisateur et les changements propres au cœur (refus du Remote Control, mode Bypass) ;
  - la liste des plugins installés, l'index de la mémoire partagée (réécrit d'après la liste de ses souvenirs), les vérifications des pages de la spec.
- **Un temporaire laissé par un plantage** (celui d'un autre process, vieux d'une minute au moins) est effacé à la première écriture dans son dossier, une fois par dossier et par lancement ; un temporaire récent (une écriture en cours d'un autre process) et les fichiers de l'utilisateur restent.
- Cela vaut pour l'état (`state.json`), les réglages, `plugins.json`, les intégrations (écrites lisibles par l'utilisateur seul), les historiques des projets fermés, la file de synchronisation, les groupes de process des agents, les fichiers des workflows et leur suivi, la mémoire partagée, les tickets partagés, la planification des docs et les vérifications de ses pages, les choix gardés d'un plugin, la liste de révocation du store, une conversation récrite (compactée ou copiée) et les petits fichiers (identifiant des statistiques, note de mise à jour).
- **Les caches**, refaits quand ils manquent ou ne se lisent pas, gardent l'écriture légère (temporaire propre et renommage, rien de forcé) : modules des plugins compilés et précompilés, Javy et escouade-js du dossier des outils, catalogue du store, texte tiré des documents, lecture des workflows d'un projet (`workflows.json`), catalogue des voix en ligne.
- **Le coût**, mesuré sur le poste de l'utilisateur (Windows 11, NVMe) : 8,1 ms médian (10,3 ms au plus) pour un état de 9 Mo écrit forcé, contre 3,5 ms en écriture légère ; 2,8 ms pour 1 Mo, 2,2 ms pour 64 Ko. L'état est sauvé au plus toutes les 400 ms quand il a changé : la cadence n'est pas bornée davantage, le surcoût étant de quelques millisecondes par sauvegarde. La mesure se rejoue par le test ignoré `what_a_forced_write_costs` de `escouade-plugin-cli`.

### La sortie

- À la sortie (« Quitter », la fermeture du système, l'arrêt pour installer une mise à jour), l'arrêt (`Core::shutdown`) arrête les agents, les terminaux et les plugins, puis sauve l'état une dernière fois. Pendant ce temps, la sauvegarde périodique peut écrire aussi (ce que les agents qui s'arrêtent et les plugins ont changé) : les deux passent l'une après l'autre (voir l'écriture).
- **Une dernière sauvegarde ratée est réessayée une fois** avant que l'app se ferme. Le journal (`app.log`) le dit : l'échec et la nouvelle tentative (« the last save as the app stops failed, tried again »), puis la réussite (« state saved as the app stops, at the second try ») ou la perte (« the state is not saved as the app stops, its last changes are lost »).

### Les copies

- Ont des copies, dans `backups/<chemin du fichier sans .json>/` du dossier de données, une au démarrage puis toutes les 10 minutes pour un fichier qui a changé (une copie identique à la plus récente n'est pas refaite) :

  | Fichier | Pourquoi |
  |---|---|
  | `state.json` | Les projets, tickets et agents. |
  | `settings.json`, `plugins.json`, `integrations.json` | Les réglages, les plugins installés, les comptes de tickets et d'hébergeurs. |
  | `ticket-accounts.json` | Les comptes de tickets après le premier de chaque service. |
  | `sync-queue.json` | Les synchronisations de tickets qui attendent leur service. |
  | `closed-projects/<id>.json` | L'historique d'un projet fermé : ses tickets et ses agents n'existent nulle part ailleurs. |
  | `projects/<id>/workflow-tracks.json` | Où en est chaque agent dans ses workflows. |

- N'ont pas de copies, et pourquoi : les conversations (`conversations/<agent>.jsonl`, écrites ligne à ligne et forcées à chaque tour, trop grosses pour une copie toutes les 10 minutes) ; la mémoire partagée (dans le dossier du projet, `.escouade/memory/`, partagée par git) ; `stats.db` (les statistiques, une base SQLite à journal propre) ; les caches (voir plus haut), qui se refont.
- Une copie n'est faite que d'un contenu relu sur le disque et trouvé lisible (l'état et les réglages par leur type ; un historique de projet fermé comme un objet dont le projet a l'id que son nom dit ; la file de synchronisation comme une liste ; le suivi des workflows comme un objet ou une liste ; les autres comme un objet JSON) ; un fichier illisible n'est pas copié et ses copies restent. Elle est compressée en gzip (`state-20261115T093600Z.json.gz`, `<id>-20261116T100000Z.json.gz` pour un projet fermé, l'heure en UTC), écrite pour tenir comme le reste, lisible par l'utilisateur seul (une copie des intégrations ou des comptes de tickets peut porter un jeton que le trousseau a refusé). Un fichier mis de côté (`.unreadable-`, `.before-restore-`) n'est jamais copié.
- La rotation garde les 6 plus récentes, la première de chaque heure sur 48 heures et la première de chaque jour (UTC) sur 30 jours, soit 84 copies au plus par fichier ; puis, des plus récentes aux plus anciennes, tant que leur taille totale ne passe pas 256 Mo pour les quatre premiers fichiers, 32 Mo pour chacun des autres (la plus récente toujours). Un état de 9 Mo fait environ 1 Mo compressé : environ 85 Mo pour lui au plus.
- La première d'une heure ou d'un jour, pas la dernière : une fois qu'une période a sa copie, aucune copie suivante ne la remplace. Un état vidé ou abîmé par erreur, copié encore et encore, ne chasse que les 6 dernières copies ; celles des heures et des jours d'avant restent (la rotation va par l'âge, pas par le nombre).
- **Toutes les copies ensemble** tiennent en 512 Mo au plus : au-delà, les plus anciennes partent d'abord, jamais la plus récente d'un fichier. L'écran des copies dit le total et la borne.
- Les copies d'un fichier qui n'est plus là (un projet fermé rouvert, effacé ou perdu) restent 30 jours après la plus récente, puis partent avec leur dossier ; celles des quatre premiers fichiers restent toujours.

### Le démarrage

- En tout premier, la copie choisie dans l'écran des copies pour le prochain démarrage (`restore-pending.json`) est posée : relue et trouvée lisible, le fichier d'aujourd'hui mis de côté sous un nom daté à côté (`state.before-restore-20261116T100000Z.json`, puis `-2`… dans la même seconde), puis la copie écrite comme le reste. Le choix est oublié qu'elle ait été posée ou non ; une copie devenue illisible ou disparue entre-temps n'est pas posée et le fichier reste tel quel. Ce qui s'est passé est noté (`restored.json`).
- Puis, avant toute lecture, chacun des quatre fichiers est vérifié. Absent : une installation neuve, rien à dire. Lisible : rien à faire.
- Illisible (rien que des zéros ou vide, coupé avant la fin de son JSON, ou autre chose que ce qu'Escouade lit) :
  - il est gardé tel quel sous un nom daté à côté (`state.unreadable-20261115T103600Z.json`, puis `-2`, `-3`… dans la même seconde) : jamais un précédent écrasé ;
  - sa copie lisible la plus récente prend sa place ; une copie qui ne se lit pas est sautée et comptée ;
  - sans copie lisible, l'app démarre sans lui (les valeurs par défaut), le fichier gardé de même.
- Ce qui a été fait est noté (`recovered.json`) et dit dans la fenêtre par une annonce en haut, sous la barre de titre, qui reste jusqu'à ce que l'utilisateur la ferme, redémarrages compris : « Des données d'Escouade étaient illisibles au démarrage », puis pour chaque fichier ce qui lui est arrivé, la copie reprise et son heure (« Escouade a repris sa copie du … »), ce qui a pu se perdre (« Ce qui a changé entre le … et le … est perdu. », du moment de la copie à la dernière écriture du fichier abîmé), les copies sautées, où le fichier illisible est gardé, et où sont les copies, avec « Ouvrir le dossier des copies » et « Fermer ». Sans copie : « … et aucune copie lisible n'existe : Escouade est partie sans. ». Jamais en silence. `app.log` le dit aussi.
- La même annonce dit une copie reprise à la main : « Une copie de tes données a été reprise », « Escouade a repris la copie du … de state.json (projets, tickets et agents), comme tu l'as demandé. Le fichier d'avant est gardé sous … » (ou pourquoi elle n'a pas pu l'être, le fichier resté tel quel).
- Un état sauvé après une reprise ne chasse pas les copies d'avant (voir la rotation).
- Un état repris d'une copie où un projet fermé depuis était encore ouvert : au démarrage, l'historique du projet fermé (plus récent) donne au projet ouvert les agents et les tickets qui lui manquent, puis son fichier est oublié une fois l'état sauvé ; rien ne se perd.

### L'écran des copies

Dans Réglages › Avancé, sous « Données locales », le groupe « Copies des données ».

- **En tête** : le dossier des copies (son chemin), ce que prennent toutes les copies et leur borne (« 3 Mo de copies, 512 Mo au plus »), « Faire une copie maintenant » (une copie de chaque fichier qui a changé depuis sa plus récente) et « Ouvrir le dossier des copies ».
- **Chaque fichier** : ce qu'il est (« Projets, tickets et agents », « Réglages et comptes des harness », « Plugins installés », « Comptes de tickets et d'hébergeurs », « Autres comptes de tickets », « Synchronisations de tickets en attente », « Projet fermé « … » », « Suivi des workflows d'un projet »), son nom dans le dossier de données, ce qu'il contient aujourd'hui (ou « Absent aujourd'hui ») et son nombre de copies ; « Voir les copies » les déplie, de la plus récente à la plus ancienne : la date, la taille et **ce que la copie contient en clair** (« 2 projets, 5 agents, 12 tickets » pour l'état ; « 3 comptes » pour les réglages et les comptes ; « 4 plugins » ; « 7 éléments » pour une liste), ou « Illisible ». Un fichier absent et sans copie (une fonction jamais servie) n'est pas montré. Les historiques des projets fermés sont repliés sous une ligne à eux (« Historiques de N projets fermés »).
- **Comparer** une copie à aujourd'hui dit, par nom et par sorte (projets, tickets « CLÉ titre », agents non archivés, comptes, plugins), ce que la copie ramènerait (« Reviendraient : … ») et ce qui serait perdu (« Seraient perdus : … »), huit noms au plus par ligne puis « … et N autres ». Rien n'est écrit.
- **Reprendre** une copie (grisé pour une copie illisible) :
  - une confirmation d'abord, qui dit ce qui sera remplacé, ce que contiennent le fichier d'aujourd'hui et la copie, que le fichier d'aujourd'hui sera mis de côté sous un nom daté (rien de perdu), et ce qui serait perdu ou reviendrait ;
  - la copie est relue et doit se lire : sinon refusée, rien n'est touché ;
  - l'historique d'un projet fermé est repris aussitôt (rien ne le tient en mémoire), le fichier d'aujourd'hui mis de côté s'il était là, et l'historique remis dans la liste des projets fermés ; refusé quand ce projet est ouvert ;
  - tout autre fichier est tenu en mémoire par Escouade, qui l'écrirait par-dessus : la copie est choisie pour le prochain démarrage, qui la pose (voir le démarrage). Une bande le dit (« La copie du … de state.json sera reprise au prochain démarrage »), avec « Redémarrer maintenant » (Escouade s'arrête comme à « Quitter », son état sauvé, et redémarre, sans installer de mise à jour) et « Annuler la reprise ». Une seule copie choisie à la fois : un nouveau choix remplace le précédent ;
  - **tant qu'un agent travaille** (un tour en cours, une question ou une permission qui attend, un worktree qui se prépare, un ticket en validation, un travail en cours hors d'un tour, une tâche de fond dans son process), « Reprendre » est grisé pour ces fichiers et le cœur refuse (« N agent(s) au travail : une reprise redémarre Escouade et couperait leur tour… »), et « Redémarrer maintenant » de même : l'écran dit combien d'agents travaillent. Rien n'arrive donc aux agents ; un historique de projet fermé se reprend toujours.
- **Projets fermés à retrouver** : un historique dont le fichier n'est plus sur le disque, que l'index des projets fermés ne cite plus, dont le projet n'est pas ouvert et dont une copie lisible reste, est proposé avec son nom, ses agents, ses tickets et la date de sa copie. « Retrouver » le reprend (comme ci-dessus), puis « Rouvrir » ouvre la fenêtre « Nouveau projet » sur son dossier et cet historique, comme « Rouvrir » des projets fermés.
- **Fichiers mis de côté** : ceux du dossier de données et du dossier des projets fermés, par une reprise (« state.json, avant une reprise ») ou par un démarrage qui les a trouvés illisibles, avec leur chemin, leur taille et leur date. Escouade ne les efface jamais.
- « Ce qui n'a pas de copies » rappelle en une ligne les conversations, la mémoire partagée et les caches.
- **Rien de secret** : l'écran ne reçoit du cœur que des comptes et des noms (noms de projets, de tickets, d'agents, des comptes des réglages, des services des intégrations et des fournisseurs des hébergeurs, ids des plugins), jamais la valeur d'un champ : ni jeton, ni clé, ni adresse, ni e-mail, ni proxy. `app.log` nomme les fichiers et les copies par leur chemin dans le dossier de données, jamais par un chemin du disque.

### Le serveur MCP

- `list_backups`, en lecture seule (aucun argument) : pour `state.json`, `settings.json`, `plugins.json`, `integrations.json`, `ticket-accounts.json` et `sync-queue.json`, s'il est là et ses copies (heure, taille, lisible ou non, comptes de ce qu'elles contiennent) ; les historiques des projets fermés et le suivi des workflows résumés (fichiers, copies, taille), jamais nommés, un projet privé restant le sien ; le nombre d'historiques à retrouver ; la copie choisie pour le prochain démarrage ; le total et la borne. Ni valeur ni chemin.
- **Aucun outil ne reprend une copie** : reprendre reste un geste de l'utilisateur dans la fenêtre.

### Les conversations

- Le journal d'une conversation (`conversations/<agent>.jsonl`) est écrit ligne à ligne. Il est forcé sur le disque à la fin de chaque tour (un `fsync` de ce que le tour a ajouté, quelques millisecondes) : une coupure en perd au plus la fin du tour en cours.
- Un journal qui finit par des zéros ou par une ligne coupée se lit jusqu'à sa dernière ligne entière (les lignes qui ne se lisent pas sont sautées) ; l'écriture suivante repart sur une ligne à elle.

## Réglages

Aucun : l'écriture forcée, les copies et la reprise ne se coupent pas.

## Limites

- Les conversations, la mémoire partagée et les statistiques n'ont pas de copies (voir plus haut pourquoi).
- Reprendre l'état, les réglages, les plugins ou les comptes demande un redémarrage, et attend qu'aucun agent ne travaille.
- Les fichiers mis de côté par une reprise ou un démarrage ne sont jamais effacés par Escouade : l'utilisateur les supprime à la main.
- Le coût n'est mesuré que sur un disque NVMe ; un disque dur lent coûterait davantage par sauvegarde de l'état.
- Le remplacement write-through refusé par Windows (fichier tenu ouvert par un autre programme sans le droit de le remplacer) retombe sur le renommage ordinaire : les données sont sur le disque, le renommage attend la prochaine écriture du système.
- Ce qui change après la dernière sauvegarde de la sortie (une ligne tardive d'un agent arrêté) n'est sauvé que si la sauvegarde périodique passe avant la fin du process : un agent tué à la sortie ne change plus son état (son tour coupé est gardé « en cours », pour être repris au démarrage suivant).
- Un `<nom>.tmp` laissé par une version d'avant la 2.2.4 (nom unique pour toutes les écritures) n'est pas effacé : seuls les temporaires au nom propre à une écriture le sont.

- **`plugin-secrets.json`** (ESC-499) : les secrets des plugins que l'utilisateur a choisi de garder dans un fichier, sur un poste sans trousseau. Il est écrit par le même chemin atomique, lisible de son seul propriétaire dès sa création, et n'est jamais dans les copies (`backups::FILES` ne le liste pas). S'il est illisible au démarrage, il est mis de côté (`plugin-secrets.unreadable.json`, droits gardés), et le choix est à refaire.
