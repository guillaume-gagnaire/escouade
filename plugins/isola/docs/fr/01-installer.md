# Installer isola

Le plugin vient avec Escouade, tu n’as pas à l’installer depuis le store. Dans la vue Plugins, **Installés**, il est rangé :

- sous **Livrés avec Escouade** tant que l’app le donne elle-même : signé avec elle, mis à jour avec elle, sa version est celle d’Escouade. Tu peux le couper, pas le désinstaller ;
- sous **Proposés par Escouade** quand l’installeur porte sa version du store : Escouade l’installe sans réseau au premier lancement, puis c’est un plugin comme un autre, que tu peux couper, désinstaller et mettre à jour.

Son module WASM n’est pas encore dans les installeurs d’Escouade : il y arrive avec sa version du store. Si sa page dit qu’il est arrêté faute de module, voir le Dépannage.

Pour qu’il serve, il lui faut deux choses : le programme `isola` sur la machine, et un `.isola.toml` dans le projet.

## 1. Installer le programme isola

Installe [isola](https://github.com/cyucelen/isola) en suivant sa documentation, et vérifie qu’il est sur ton PATH : `isola --help` doit répondre dans un terminal.

- Escouade cherche `isola` dans les dossiers absolus du PATH, au plus toutes les 30 secondes : installé pendant qu’Escouade tourne, il est trouvé peu après, sans relancer l’app.
- Sous Windows, Escouade ne lance qu’un `isola.exe` ou un `isola.com`, jamais un `isola.cmd` ni un `isola.bat` (ils ne tournent qu’à travers un shell).

## 2. Ajouter un `.isola.toml` au projet

Le fichier se place à la racine du projet et se commite : c’est sa présence dans un worktree qui dit que ses services sont ceux d’isola. Sa forme est celle de la documentation d’isola.

Un worktree créé depuis une branche qui a le fichier commité est celui d’isola dès sa création : l’agent ne reçoit pas de bloc de ports réservés, et il n’a pas de recette de lancement à écrire. Il donne seulement l’adresse qui montre la fonctionnalité.

## 3. Choisir les projets

Dans la vue Plugins, ouvre **isola**, onglet **Projets** :

- il est actif dans chaque projet qui n’est pas privé ;
- un projet privé est coupé d’office : active-le pour lui si tu veux isola là aussi ;
- l’interrupteur de sa ligne, dans **Installés**, le coupe pour toute l’app.

Coupé pour l’app ou pour le projet, sans le programme `isola` ou sans le fichier, un worktree n’est à personne : le ticket part comme sans isola, avec une recette sur les ports réservés.

## Ce qu’Escouade garde, et où

- Tes choix (actif ou non pour l’app, pour chaque projet) : dans `~/.escouade/plugins.json`.
- Le `.isola.toml` que tu as approuvé et l’adresse à ouvrir : avec l’agent, dans les données d’Escouade (`~/.escouade`). Un fichier modifié ensuite t’est montré de nouveau.
- Le journal du plugin : `~/.escouade/plugin-logs/dev.escouade.isola.log`, et ses dernières lignes dans l’onglet **Journal**.

isola garde ses propres données (ses services, leurs bases) là où il les met : Escouade n’y touche pas, sinon par `isola destroy`.
