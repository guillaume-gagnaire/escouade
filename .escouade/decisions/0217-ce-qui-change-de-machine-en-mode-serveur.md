---
number: 217
id: "ce-qui-change-de-machine-en-mode-serveur"
title: "Ce qui change de machine en mode serveur : les ports relayés par SSH, les fichiers reçus téléchargés à l'ouverture, la connexion d'un compte relayée au clic"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-538"
pages: ["a-distance/ce-qui-change-de-machine","a-distance/mode-serveur","projets/nouveau-projet","fenetre/notifications","harness/comptes","conversation/conversation"]
tickets: ["ESC-538"]
documents: ["docs/SERVER-UX.md","docs/REMOTE.md"]
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/ce-qui-change-de-machine","added":67,"removed":0},{"page":"a-distance/mode-serveur","added":5,"removed":5},{"page":"projets/nouveau-projet","added":6,"removed":0},{"page":"fenetre/notifications","added":7,"removed":0},{"page":"harness/comptes","added":4,"removed":0},{"page":"conversation/conversation","added":6,"removed":0}]
accepted_at: "2026-11-19T11:31:26+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Avec « Se connecter à un serveur » (ESC-537), la fenêtre pilote le cœur d'un serveur ; mais tout ce qui changeait de machine (table « Ce qui change de machine » de REMOTE.md) restait refusé ou faux : le sélecteur de dossier montrait ceux du poste, un fichier envoyé par un agent s'ouvrait sur le serveur, un serveur de test écoutait sur le 127.0.0.1 du serveur, les notifications n'étaient rendues nulle part, la connexion d'un compte ouvrait un navigateur que le serveur n'a pas. Les choix S12 et S13 de SERVER-UX.md gardent leur réponse par défaut (S1 = B).

## Décision

1. « Nouveau projet » sur un serveur : « Cloner un dépôt » (https ou ssh, cloné par le git du serveur) en premier, puis « Un dossier du serveur », un navigateur servi par le cœur (fs_browse : des dossiers seulement, dépôt et branche, déjà un projet, place du disque, 500 au plus) ; jamais le sélecteur du poste.
2. Les fichiers vers le serveur partent en octets (logo, joints, image collée), jamais un chemin du poste ; « Joindre le fichier d'origine » ne vaut pas sur un serveur.
3. Les fichiers des agents sont téléchargés quand on les ouvre (S12), dans Téléchargements/Escouade/<espace> ; un programme ou un script (par ses octets ou par son nom) n'est jamais ouvert, seulement téléchargé et montré dans son dossier ; le type lu dans les octets fixe l'extension ; la copie n'est jamais exécutable et, sous Windows, marquée comme téléchargée (Zone.Identifier).
4. Les ports du serveur (un serveur de test, le retour d'une connexion) sont relayés par une redirection SSH de plus (ssh -N -L 127.0.0.1:poste:127.0.0.1:serveur, les options sûres du tunnel), pas dans la connexion de l'API comme REMOTE.md le prévoyait : rien de nouveau côté serveur, tout TCP porté (rechargement à chaud compris). Un tunnel sans SSH (téléphone, WireGuard) demandera ce relai dans l'API.
5. Ce que l'hôte du serveur publie est rendu par le poste (server_alert), la notification nommée par l'espace (« Travail · … »), son clic rendu au serveur qui garde l'action de ses 64 dernières ; ce que le serveur demanderait d'ouvrir ne l'est jamais. La reprise dit le nombre d'événements rattrapés.
6. La voix est celle du poste, réglages compris (local_voice_get, local_voice_set).
7. La connexion d'un compte (S13) : l'adresse lue dans le terminal s'ouvre au clic sur le poste ; un retour sur localhost est relayé sur le même port, sinon le code se colle (ou le code d'appareil se tape) ; les secrets en fichier sur le serveur sont dits là où ils se donnent.

## Conséquences

Chaque ligne de la table est faite côté client, sauf la mise à jour du serveur depuis le client (il faut sa release signée), le Claude Code du poste qui piloterait le serveur MCP du serveur, et les serveurs de langage à distance. Un relai de port coûte un process ssh par port, arrêté en quittant le serveur. Treize commandes nouvelles (six servies, sept du poste) classées dans list.rs et REMOTE.md ; des crochets de test en build de développement seulement (ESCOUADE_BROWSE_HOME, ESCOUADE_DOWNLOADS_DIR, ESCOUADE_OPEN_LOG, ESCOUADE_ALERT_LOG). Reste l'essai réel d'une connexion de compte sur le VPS d'essai, avec l'utilisateur.
