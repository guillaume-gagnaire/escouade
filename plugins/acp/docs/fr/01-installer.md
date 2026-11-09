# Installer

Le plugin ACP ne fait rien seul : il sert les harness que d’autres plugins décrivent.

## 1. Installer le plugin

1. Ouvre la vue Plugins : l’icône en forme de pièce de puzzle de la barre des projets, ou Réglages › Plugins.
2. S’il n’est pas déjà dans **Installés**, ouvre **Store** et cherche « ACP ».
3. Sur sa page, clique sur **Installer**. La fenêtre d’installation dit ce qu’il pourra faire : « Tourne dans Escouade, isolé », « Écrit dans les dossiers de tes agents » et « Lit les fichiers de tes agents » (leur dépôt). Confirme.

## 2. Installer un harness décrit

1. Installe le programme du harness sur ta machine, comme sa page le dit (pour Goose : son script, puis `goose configure`).
2. Installe le plugin qui le décrit, depuis le **Store**, ou depuis un dossier pour un plugin en développement (**Depuis un dossier…**). Avant l’installation, la fenêtre montre le programme que ses agents lanceront, avec ses arguments : « Lance pour ses agents ce programme de ton ordinateur, avec tes droits ».
3. Choisis son harness dans le menu « Harness » à côté de « Nouvel agent ».

## 3. Brancher un compte

Le plugin ACP n’a pas de compte. Chaque harness décrit a les siens : Réglages › Comptes et harness, **Ajouter un compte…**, son harness.

- **Le compte de la machine** : la configuration habituelle du harness, telle quelle.
- **Un nouveau compte** : un dossier de configuration à lui, sous `~/.escouade/<harness>/`, donné au harness par la variable que son manifeste nomme (`accountVar`, `GOOSE_PATH_ROOT` pour Goose).

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** (nom, ordre, réglages) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **La configuration et les clés du harness** restent les siennes, dans son dossier. Escouade et le plugin n’en lisent aucune. ACP ne dit ni le quota, ni le coût, ni les comptes d’un agent.
