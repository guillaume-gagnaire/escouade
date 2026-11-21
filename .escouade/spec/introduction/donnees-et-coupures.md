---
title: "Données locales et coupures"
subject: "introduction"
sources: ["Ticket ESC-469 (perte de state.json le 2026-11-15)","Ticket ESC-535 (app.log du poste de l'utilisateur, « cannot save state … os error 2 » à la sortie pour une mise à jour, 2026-11-17 et 2026-11-18)","crates/escouade-plugin-cli/src/paths.rs","crates/escouade-model/src/backups.rs","src-tauri/src/conv.rs","src-tauri/src/core.rs","src-tauri/src/save_tests.rs","src/components/RecoveryBanner.svelte","website/content/docs/fr/data.md"]
tickets: ["ESC-469","ESC-499","ESC-535"]
decisions: [243]
compared: null
---

## Ce que c'est

Escouade garde tout ce qu'elle ne peut pas refaire dans son dossier de données (`~/.escouade`, ou `ESCOUADE_DATA_DIR`). Une coupure du poste (courant, arrêt forcé, plantage du système) ne doit jamais lui faire perdre son état : ses fichiers sont écrits jusque sur le disque, elle en garde des copies, et un fichier illisible au démarrage est remplacé par sa dernière bonne copie, en le disant.

Origine : le 2026-11-15, une coupure a laissé `state.json` (8,7 Mo) à sa taille, rempli de zéros. Escouade 2.2.0 l'avait écrit par un fichier temporaire renommé, sans forcer les données sur le disque (NTFS journalise le renommage avant les données), puis, ne pouvant le lire, était repartie d'un état vide sans rien dire et l'avait écrit par-dessus : 468 tickets, 490 agents et les projets perdus.

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

- `state.json`, `settings.json`, `plugins.json` et `integrations.json` ont des copies dans `backups/<nom>/` du dossier de données, une au démarrage puis toutes les 10 minutes pour un fichier qui a changé (une copie identique à la plus récente n'est pas refaite).
- Une copie n'est faite que d'un contenu relu sur le disque et trouvé lisible (l'état et les réglages par leur type, les deux autres comme un objet JSON) ; un fichier illisible n'est pas copié et ses copies restent. Elle est compressée en gzip (`state-20261115T093600Z.json.gz`, l'heure en UTC), écrite pour tenir comme le reste, lisible par l'utilisateur seul (une copie des intégrations peut porter un jeton que le trousseau a refusé).
- La rotation garde les 6 plus récentes, la première de chaque heure sur 48 heures et la première de chaque jour (UTC) sur 30 jours, soit 84 copies au plus par fichier ; puis, des plus récentes aux plus anciennes, tant que leur taille totale ne passe pas 256 Mo (la plus récente toujours). Un état de 9 Mo fait environ 1 Mo compressé : environ 85 Mo pour lui au plus.
- La première d'une heure ou d'un jour, pas la dernière : une fois qu'une période a sa copie, aucune copie suivante ne la remplace. Un état vidé ou abîmé par erreur, copié encore et encore, ne chasse que les 6 dernières copies ; celles des heures et des jours d'avant restent (la rotation va par l'âge, pas par le nombre).

### Le démarrage

- Avant toute lecture, chacun des quatre fichiers est vérifié. Absent : une installation neuve, rien à dire. Lisible : rien à faire.
- Illisible (rien que des zéros ou vide, coupé avant la fin de son JSON, ou autre chose que ce qu'Escouade lit) :
  - il est gardé tel quel sous un nom daté à côté (`state.unreadable-20261115T103600Z.json`, puis `-2`, `-3`… dans la même seconde) : jamais un précédent écrasé ;
  - sa copie lisible la plus récente prend sa place ; une copie qui ne se lit pas est sautée et comptée ;
  - sans copie lisible, l'app démarre sans lui (les valeurs par défaut), le fichier gardé de même.
- Ce qui a été fait est noté (`recovered.json`) et dit dans la fenêtre par une annonce en haut, sous la barre de titre, qui reste jusqu'à ce que l'utilisateur la ferme, redémarrages compris : « Des données d'Escouade étaient illisibles au démarrage », puis pour chaque fichier ce qui lui est arrivé, la copie reprise et son heure (« Escouade a repris sa copie du … »), ce qui a pu se perdre (« Ce qui a changé entre le … et le … est perdu. », du moment de la copie à la dernière écriture du fichier abîmé), les copies sautées, où le fichier illisible est gardé, et où sont les copies, avec « Ouvrir le dossier des copies » et « Fermer ». Sans copie : « … et aucune copie lisible n'existe : Escouade est partie sans. ». Jamais en silence. `app.log` le dit aussi.
- Un état sauvé après une reprise ne chasse pas les copies d'avant (voir la rotation).

### Les conversations

- Le journal d'une conversation (`conversations/<agent>.jsonl`) est écrit ligne à ligne. Il est forcé sur le disque à la fin de chaque tour (un `fsync` de ce que le tour a ajouté, quelques millisecondes) : une coupure en perd au plus la fin du tour en cours.
- Un journal qui finit par des zéros ou par une ligne coupée se lit jusqu'à sa dernière ligne entière (les lignes qui ne se lisent pas sont sautées) ; l'écriture suivante repart sur une ligne à elle.

## Réglages

Aucun : l'écriture forcée, les copies et la reprise ne se coupent pas.

## Limites

- Pas d'écran pour choisir une copie à la main (prévu en 2.3) : l'annonce et la documentation disent où sont les copies ; on en reprend une en quittant Escouade et en mettant la copie décompressée à la place du fichier.
- Les autres fichiers (historiques des projets fermés, mémoire, workflows, file de synchronisation…) sont écrits pour tenir mais n'ont pas de copies.
- Le coût n'est mesuré que sur un disque NVMe ; un disque dur lent coûterait davantage par sauvegarde de l'état.
- Le remplacement write-through refusé par Windows (fichier tenu ouvert par un autre programme sans le droit de le remplacer) retombe sur le renommage ordinaire : les données sont sur le disque, le renommage attend la prochaine écriture du système.
- Ce qui change après la dernière sauvegarde de la sortie (une ligne tardive d'un agent arrêté) n'est sauvé que si la sauvegarde périodique passe avant la fin du process : un agent tué à la sortie ne change plus son état (son tour coupé est gardé « en cours », pour être repris au démarrage suivant).
- Un `<nom>.tmp` laissé par une version d'avant la 2.2.4 (nom unique pour toutes les écritures) n'est pas effacé : seuls les temporaires au nom propre à une écriture le sont.

- **`plugin-secrets.json`** (ESC-499) : les secrets des plugins que l'utilisateur a choisi de garder dans un fichier, sur un poste sans trousseau. Il est écrit par le même chemin atomique, lisible de son seul propriétaire dès sa création, et n'est jamais dans les copies (`backups::FILES` ne le liste pas). S'il est illisible au démarrage, il est mis de côté (`plugin-secrets.unreadable.json`, droits gardés), et le choix est à refaire.
