---
title: "Le store dans l'app"
subject: "store"
sources: ["docs/STORE.md","design/plugins/maquettes-b.html","design/app-redesign/UX-NOTES.md","website/content/docs/fr/plugins/installer.md"]
tickets: ["ESC-236"]
decisions: []
compared: null
---

# Le store dans l'app

Référence : `docs/STORE.md` §5 et §7 (il fait foi) ; aperçu `#b1` à `#b4` de `design/plugins/maquettes-b.html`, choix 160 à 181 et 213 à 223 d'`UX-NOTES.md`.

## Le catalogue
- Lu au démarrage puis toutes les 6 heures (`GET /api/v1/catalog.json`, `If-None-Match` : un 304 le garde), gardé sur la machine ; l'en-tête du Store dit « catalogue lu à HH:MM, la recherche reste sur ta machine ». Le store ne répond pas : le catalogue gardé est montré, et l'en-tête le dit.
- Tout passe par Réglages › Réseau (proxy, certificats du système, TLS non vérifié si l'utilisateur l'a coupé), comme les autres requêtes de l'app.
- **La recherche** est locale : nom, description (les deux langues), mots-clés, auteur, id, sans casse ni accents. « Vérifiés seulement », les puces de catégories et les catégories de la barre latérale (comptées dans le catalogue une fois lu) filtrent.
- **Une ligne** : tuile (les lettres : la fenêtre ne charge aucune image du store), nom, sceau de la version proposée ou « Non vérifié », auteur · installations, description sur une ligne, ce qu'il demande en une ligne ; à droite « Installer », « ✓ Installé » ou « Mettre à jour ».
- Sans recherche ni filtre : « Vérifiés par Escouade » (les plus installés) et « Nouveaux » côte à côte, quatre chacun, « Tout voir ».
- **La version proposée** est la dernière que cette Escouade peut faire tourner (`escouade.api`, `min`, `max`), jamais une version retirée par son auteur ; une version stable avant une préversion.

## « Pour <projet> »
- Jusqu'à trois plugins du catalogue, non installés, tirés de ce qu'Escouade voit, chacun avec sa raison : un programme d'un plugin de harness sur le PATH (« vibe est installé sur ce poste »), des clés de tickets dans les 200 derniers commits du projet pour une source du Kanban de type Jira, Linear ou YouTrack (« des clés DEM-… dans les commits de ce dépôt »). Rien n'est envoyé ; les vérifiés d'abord.
- « Pas maintenant » écarte une proposition une semaine, « Ne plus proposer » pour de bon (gardé sur la machine).

## L'installation
- « Installer » télécharge `version.json` (comparé à l'empreinte du catalogue), la signature si le catalogue dit la version vérifiée (vérifiée avec les clés embarquées, moins celles que la liste de révocation retire ; le commentaire nomme l'id et la version), l'archive (taille et SHA-256 de `version.json`), le programme du système de la machine (SHA-256), puis lit l'archive en mémoire (liens, chemins absolus ou qui remontent refusés, 50 Mo, 200 Mo décompressés, 20 000 entrées) et compare le manifeste à son empreinte. Un écart arrête tout et le dit ; rien n'est mis de côté.
- Le programme de la release est écrit là où le manifeste le nomme (`run.<système>`, sinon `run.command`), exécutable.
- La fenêtre de PLG-4 dit « Vérifié par Escouade », l'origine « store.escouade.dev », la version → le commit, l'empreinte « archive reçue identique », la signature « vérifiée ici, par l'app » ; une version non vérifiée a sa bande (« Personne n'a relu cette version… ») et « Installer quand même ».
- Une app qui n'embarque pas encore les clés du store installe sans sceau, sans refuser.
- **« N'installer que des versions vérifiées »** (en tête des installés, éteint d'office, gardé dans les réglages) refuse une version sans sceau, à l'installation comme à la mise à jour.
- Installé, le plugin est fixé par `store:<SHA-256 de version.json>` ; son `version.json` et sa signature sont gardés à part (`plugins/.store/<id>/`) : le sceau se revérifie à chaque lecture, hors ligne aussi, et part si le fichier change.

## Les mises à jour et le passage au store
- Un plugin du store est mis à jour depuis le catalogue (la dernière version compatible) : proposé, jamais installé seul, « v1.3.0 · non vérifiée » quand elle n'a pas de sceau ; l'accord redemandé si elle demande plus.
- Un plugin installé depuis un dossier ou un dépôt git dont l'id est au store le dit sur sa page, avec « Passer à la v… du store » ; ses choix (projets, permissions retirées, réglages) restent.

## La liste de révocation
- Lue au démarrage puis toutes les 6 heures, et par « Relire maintenant » en bas des installés ; vérifiée (signée par une clé de l'app, commentaire `escouade-revocations-v1 <serial>`, pas par une clé qu'elle révoque) ; un `serial` plus petit que le dernier lu est refusé et dit. Le store ne répond pas : sa copie sur escouade.dev. La dernière liste gardée s'applique hors ligne, revérifiée au démarrage.
- `disable` : la version est arrêtée et désactivée (« Retiré par Escouade le … »), une bande rouge en tête des installés dit laquelle, quand et pourquoi, que ses réglages et secrets restent ; elle ne se réinstalle pas. `unverify` : le sceau part, le plugin continue, sa page dit pourquoi.
- En bas des installés : « Liste de révocation lue à HH:MM · nº N, signée, relue toutes les 6 heures ».

## Ce qui part
- Des GET sans identifiant. Avec l'accord aux statistiques seulement : `POST /api/v1/installs { id, version, event }` et l'événement Aptabase `plugin_installed` (`from_store` oui ou non, sans le nom du plugin).

## Pas dans ce ticket
- L'enregistrement de `escouade://plugin/<id>` auprès des systèmes (STORE-11, ESC-287) : la vue sait déjà ouvrir la page d'un plugin du store par son id (`openStorePlugin`).
