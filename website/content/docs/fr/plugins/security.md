# La sécurité des plugins

Un plugin du store passe par trois gardes : les dix contrôles du store à la soumission, la revue « Vérifié par Escouade » faite à la main, et les vérifications de l’app à chaque installation. Cette page dit ce que chacune tient, ce que la revue refuse, et ce que le sceau ne promet pas.

## Ce que l’app vérifie

À l’installation et à chaque mise à jour, Escouade télécharge `version.json`, l’archive et le programme de ton système, puis vérifie :

- **les empreintes** : `version.json` contre le catalogue, l’archive, le programme et le manifeste contre `version.json` (SHA-256). Un octet changé, sur le store, en route ou dans un cache, arrête l’installation ;
- **la signature**, pour une version vérifiée : `version.json` signé par minisign avec la clé d’Escouade, et le commentaire de la signature qui nomme cet identifiant et ce numéro. Les clés publiques sont dans l’app ; la clé secrète vit sur un poste, chiffrée, jamais sur le serveur du store, jamais dans la CI ni dans un dépôt. Le store ne peut donc pas fabriquer un sceau ;
- **l’archive** lue en mémoire, sans lien symbolique ni chemin qui sorte du dossier du plugin.

Le sceau d’un plugin installé est revérifié sur ta machine, sans le store. Puis la fenêtre d’installation montre ses permissions en mots, avant que rien ne tourne ([installer un plugin](plugins/installer)).

## Ce que le sceau couvre

« Vérifié par Escouade » vaut pour **une version** : son code au commit publié, son archive et ses binaires attestés, désignés par l’empreinte de son `version.json`. Une nouvelle version est « non vérifiée » jusqu’à sa propre revue, même si la précédente l’était : c’est dans une mise à jour qu’un plugin honnête peut changer de mains. **N’installer que des versions vérifiées**, dans la vue Plugins, refuse toute version sans sceau.

Le sceau dit qu’une personne a lu ce code au regard de ce qu’il déclare, et que le banc l’a fait tourner sans réseau. Il ne promet pas l’absence de bogue, ni la qualité du service tiers que le plugin joint.

## Ce que la revue refuse

La revue lit chaque version avec un script qui lui prépare des pistes, fichier et ligne, et un agent qui lit le code comme des données. Ses issues : « Vérifier », « Demander des changements » (la version reste publiée, non vérifiée, le message dit quoi corriger) ou « Refuser ».

**Refusé d’office**, sans lire plus loin :

- une archive ou un binaire dont l’empreinte n’est pas celle de `version.json` ;
- une version qui n’est plus celle que le store a figée.

**Pas de sceau tant que ce n’est pas corrigé** :

- un hôte joint qui n’est pas dans `permissions.network` (une adresse en dur, un `fetch`, un `WebSocket`) ;
- la lecture de secrets ou de dossiers personnels hors de ce que le plugin déclare : variables d’environnement, `~/.ssh`, `~/.claude`, le trousseau hors de `secrets/get` ;
- des fichiers lus ou écrits hors du dossier du plugin et de ce que ses permissions ouvrent ;
- un programme lancé que les permissions n’annoncent pas (`child_process`, `spawn`, `exec`) ;
- du code dynamique : `eval`, `new Function`, un `import()` d’une adresse ;
- du code illisible : minifié sans sa source, du texte caché en base64, des blobs binaires ;
- des dépendances non épinglées, un script `postinstall`, une dépendance tirée d’un dépôt git ;
- des consignes ajoutées aux agents qui les détournent de leur tâche (une injection de prompt) ;
- un binaire sans attestation de provenance qui le lie au commit relu ;
- une étiquette déplacée depuis la soumission, une entrée d’archive refusée (lien, `..`, chemin absolu) ;
- un banc de conformité qui échoue, ou qui tente le réseau ou une écriture que le bac à sable refuse.

**Retiré** : un comportement malveillant découvert (envoyer des données ailleurs, cacher ce que fait le code, mentir sur ses permissions) mène au retrait de la version, et du plugin s’il le faut.

## Le retrait et la liste de révocation

Escouade lit une **liste de révocation** signée par la même clé, au démarrage puis toutes les 6 heures, et refuse une liste plus ancienne que la dernière lue. Elle a deux sortes d’entrées :

- **retirer le sceau** : le plugin continue, une bande dit pourquoi ;
- **retrait d’urgence** : l’app arrête le plugin, le désactive et dit pourquoi, avec « Désinstaller ». Elle ne supprime rien seule : tes données sont à toi.

La liste est aussi copiée sur escouade.dev, une heure au plus après le store : si le store tombe, l’app la lit là. La dernière liste lue s’applique hors ligne.

## Signaler un plugin

« Signaler », sur la page d’un plugin du store ou dans l’app, est ouvert à tous avec une adresse e-mail, sans compte GitHub : malveillant, ne fait pas ce qu’il dit, permissions trompeuses, licence, autre. Une personne lit chaque signalement et te répond. L’auteur voit qu’un signalement est ouvert, jamais qui l’a fait.

## Un plugin hors du store

Un plugin installé depuis un dossier ou un dépôt git n’a passé ni les contrôles ni la revue : il est **non vérifié**, et la fenêtre d’installation le dit en tête. Ses permissions s’appliquent de la même façon.
