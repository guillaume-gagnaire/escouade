# Installer

Quatre étapes : Goose sur ta machine, le plugin ACP, ce plugin, puis un compte.

## 1. Installer Goose

- Windows : `Invoke-WebRequest -Uri https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1 -OutFile download_cli.ps1; ./download_cli.ps1`
- macOS et Linux : `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash`

Puis configure son fournisseur dans un terminal : `goose configure`. Escouade trouve `goose` dans le `PATH`. Le plugin a été essayé avec Goose 1.57.0. `goose update` le met à jour.

## 2. Le plugin ACP

Goose est servi par le plugin ACP (`dev.escouade.acp`). Dans la vue Plugins, vérifie qu’il est dans **Installés** et allumé ; sinon, installe-le depuis **Store**. Coupé, les agents Goose restent, marqués « harness absent », avec leurs conversations, et reviennent avec lui.

## 3. Installer le plugin

1. Dans la vue Plugins, ouvre **Store** et cherche « Goose » (ou, pour essayer un dossier, **Depuis un dossier…**).
2. Sur sa page, clique sur **Installer**. Avant l’installation, la fenêtre montre le programme que ses agents lanceront : « Lance pour ses agents ce programme de ton ordinateur, avec tes droits », `goose acp`. Confirme.
3. Choisis « Goose » dans le menu « Harness » à côté de « Nouvel agent ».

Comme tout plugin, il se coupe par son interrupteur et se désinstalle depuis sa page. Coupé ou désinstallé, ses agents restent, marqués « harness absent », avec leurs conversations.

## 4. Brancher un compte

Aucun compte n’existe au départ : un agent ne démarre que sur un compte que tu as ajouté. Ouvre Réglages › Comptes et harness, **Ajouter un compte…**, harness « Goose » :

- **Le compte de la machine** : la configuration habituelle de Goose, telle quelle.
- **Un nouveau compte** : un dossier de configuration à lui, sous `~/.escouade/goose/`, donné à Goose par `GOOSE_PATH_ROOT`. Configure son fournisseur pour ce dossier.

Dans la fiche du compte, règle le mode des nouveaux agents.

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** (nom, ordre, réglages) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **La configuration et les clés de Goose** restent les siennes, dans son dossier. Escouade n’en lit aucune.
