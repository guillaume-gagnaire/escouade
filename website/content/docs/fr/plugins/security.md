# La sécurité des plugins

Un plugin du store passe par trois gardes : les onze contrôles du store à la soumission, la revue « Vérifié par Escouade », faite par un agent d’IA puis signée, et les vérifications de l’app à chaque installation. Cette page dit ce que chacune tient, ce que la revue refuse, et ce que le sceau ne promet pas.

## Ce que l’app vérifie

À l’installation et à chaque mise à jour, Escouade télécharge `version.json`, l’archive et son module WASM, puis vérifie :

- **les empreintes** : `version.json` contre le catalogue, l’archive, le module et le manifeste contre `version.json` (SHA-256). Un octet changé, sur le store, en route ou dans un cache, arrête l’installation ;
- **la signature**, pour une version vérifiée : `version.json` signé par minisign avec une clé d’Escouade dont le rôle couvre ce plugin (voir [les clés du sceau](#les-cles-du-sceau)), et le commentaire de la signature qui nomme cet identifiant et ce numéro. Les clés publiques sont dans l’app ; les clés secrètes vivent sur un poste, jamais sur le serveur du store, jamais dans la CI ni dans un dépôt. Le store ne peut donc pas fabriquer un sceau ;
- **l’archive** lue en mémoire, sans lien symbolique ni chemin qui sorte du dossier du plugin ;
- **le dossier** que `version.json` nomme pour un plugin rangé dans un sous-dossier de son dépôt : un chemin relatif simple, sans `..`, ni chemin absolu, ni antislash ; sinon rien n’est installé. Le store, lui, refuse à la soumission un dossier qui passe par un lien symbolique.

Le sceau d’un plugin installé est revérifié sur ta machine, sans le store. Puis la fenêtre d’installation montre ses permissions en mots, avant que rien ne tourne ([installer un plugin](plugins/installer)).

## Ce que le sceau couvre

« Vérifié par Escouade » vaut pour **une version** : son code au commit publié, son archive et son module, recompilé par la revue, désignés par l’empreinte de son `version.json`. Une nouvelle version est « non vérifiée » jusqu’à sa propre revue, même si la précédente l’était : c’est dans une mise à jour qu’un plugin honnête peut changer de mains. **N’installer que des versions vérifiées**, dans la vue Plugins, refuse toute version sans sceau.

Le sceau dit qu’un agent d’IA d’Escouade a lu ce code au regard de ce qu’il déclare, que le banc l’a fait tourner sans réseau, puis que la version a été signée. Ce n’est pas une relecture humaine : la page du plugin, sur le store et dans la vue Plugins, le dit avec la clé qui a signé. Il ne promet pas l’absence de bogue, ni la qualité du service tiers que le plugin joint.

## Les clés du sceau

Chaque clé d’Escouade a un rôle, écrit dans le code de l’app, du store et de leurs outils : une signature hors de son rôle est refusée, avec sa raison.

| Clé | Ce qu’elle scelle | Qui signe |
| --- | --- | --- |
| **La clé des plugins d’Escouade** | les plugins `dev.escouade.*`, et rien d’autre | le chef d’escouade, l’agent qui a fait la revue ; la clé n’a pas de mot de passe |
| **La clé des plugins tiers** | les plugins des autres auteurs, et la liste de révocation | l’éditeur, qui tape le mot de passe de la clé à chaque signature |
| **La clé de secours** | tout, le jour où une autre clé est perdue ou volée | l’éditeur ; elle est gardée hors ligne |

Chaque sceau dit sa clé : son identifiant est dans le fichier de signature (`version.json.minisig`), dans le catalogue du store, sur la page du plugin et dans la vue Plugins de l’app (« Signé par la clé des plugins d’Escouade · E0516468FFDCBDD5 »). Ainsi, une clé sans mot de passe volée sur le poste de l’éditeur ne scelle aucun plugin d’un autre auteur, ni une liste de révocation.

## Ce que la revue refuse

La revue lit chaque version avec un script qui lui prépare des pistes, fichier et ligne, et un agent qui lit le code comme des données. Ses issues : « Vérifier », « Demander des changements » (la version reste publiée, non vérifiée, le message dit quoi corriger) ou « Refuser ».

**Refusé d’office**, sans lire plus loin :

- une archive ou un module dont l’empreinte n’est pas celle de `version.json` ;
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
- un module que la recompilation ne redonne pas, à l’octet près ;
- une dépendance Rust qu’on ne peut pas justifier : ce que `Cargo.lock` ajoute est du code qui tourne ; du code `unsafe`, un `extern` ou un `#[link]` sans raison ;
- `agentHome`, ou une entrée en `*` de `permissions.commands`, sans une raison relue ;
- une étiquette déplacée depuis la soumission, une entrée d’archive refusée (lien, `..`, chemin absolu) ;
- un banc de conformité qui échoue, ou qui tente le réseau ou une écriture que le bac à sable refuse.

**Retiré** : un comportement malveillant découvert (envoyer des données ailleurs, cacher ce que fait le code, mentir sur ses permissions) mène au retrait de la version, et du plugin s’il le faut.

## Le retrait et la liste de révocation

Escouade lit une **liste de révocation** signée par la clé des plugins tiers et de la révocation (ou la clé de secours), au démarrage puis toutes les 6 heures, et refuse une liste plus ancienne que la dernière lue. Elle a deux sortes d’entrées :

- **retirer le sceau** : le plugin continue, une bande dit pourquoi ;
- **retrait d’urgence** : l’app arrête le plugin, le désactive et dit pourquoi, avec « Désinstaller ». Elle ne supprime rien seule : tes données sont à toi.

La liste est aussi copiée sur escouade.dev, une heure au plus après le store : si le store tombe, l’app la lit là. La dernière liste lue s’applique hors ligne.

## Signaler un plugin

« Signaler », sur la page d’un plugin du store ou dans l’app, est ouvert à tous avec une adresse e-mail, sans compte GitHub : malveillant, ne fait pas ce qu’il dit, permissions trompeuses, licence, autre. Une personne lit chaque signalement et te répond. L’auteur voit qu’un signalement est ouvert, jamais qui l’a fait.

## Un plugin hors du store

Un plugin installé depuis un dossier ou un dépôt git n’a passé ni les contrôles ni la revue : il est **non vérifié**, et la fenêtre d’installation le dit en tête. Ses permissions s’appliquent de la même façon.
