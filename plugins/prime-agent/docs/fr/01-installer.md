# Installer

Cinq étapes : Prime Agent sur ta machine, le plugin ACP, ce plugin, le mode Bypass, puis un compte.

Le plugin demande **Escouade 2.2.2 ou plus** : avant, l’app ne refusait pas un agent Prime Agent quand le mode Bypass n’est pas permis (voir l’onglet Permissions). Sur une version plus ancienne, il ne s’installe pas.

## 1. Installer Prime Agent

Ses installeurs officiels, tels que son README les donne :

- macOS et Linux (GNU, glibc 2.35 au moins ; pas musl) : `curl -fsSL https://app.primeintellect.ai/prime-agent/install.sh | sh`
- Windows x64 : `irm https://app.primeintellect.ai/prime-agent/install.ps1 | iex`

Le script de macOS et Linux pose son lanceur dans `~/.local/bin/prime-agent` et le programme dans `~/.local/share/prime-agent/`. Il installe aussi `uv` (d’astral.sh) et prépare le noyau Python de Prime Agent dans `~/.prime/agent/kernel-venv`. Il lit la version du canal stable, télécharge l’archive et son `SHA256SUMS` depuis le site de téléchargement de Prime Intellect, et vérifie la somme avant d’extraire. Le script de Windows fait de même dans `%USERPROFILE%\.local\` et ajoute son dossier `bin` au `PATH` de l’utilisateur. Lis un script avant de le lancer.

Ajoute `~/.local/bin` au `PATH` si le script te le dit, puis connecte Prime Agent à un fournisseur : lance `prime-agent` dans un terminal et tape `/login` (un abonnement ou une clé), ou décris un fournisseur à toi dans `~/.prime/agent/models.json` (un modèle local compatible OpenAI, par exemple). Escouade trouve `prime-agent` dans le `PATH`. Le plugin a été essayé avec Prime Agent 0.10.3. `prime-agent update` le met à jour.

Prime Agent envoie par défaut des statistiques d’usage à Prime Intellect. `prime-agent telemetry` dit où il en est, `prime-agent telemetry off` les coupe. Escouade n’y touche pas.

## 2. Le plugin ACP

Prime Agent est servi par le plugin ACP (`dev.escouade.acp`). Dans la vue Plugins, vérifie qu’il est dans **Installés** et allumé ; sinon, installe-le depuis **Store**. Coupé, les agents Prime Agent restent, marqués « harness absent », avec leurs conversations, et reviennent avec lui.

## 3. Installer le plugin

1. Dans la vue Plugins, ouvre **Store** et cherche « Prime Agent » (ou, pour essayer un dossier, **Depuis un dossier…**).
2. Sur sa page, clique sur **Installer**. Avant l’installation, la fenêtre montre le programme que ses agents lanceront : « Lance pour ses agents ce programme de ton ordinateur, avec tes droits », `prime-agent --mode acp --no-session`. Confirme.
3. Choisis « Prime Agent » dans le menu « Harness » à côté de « Nouvel agent ».

Comme tout plugin, il se coupe par son interrupteur et se désinstalle depuis sa page. Coupé ou désinstallé, ses agents restent, marqués « harness absent », avec leurs conversations.

## 4. Permettre le mode Bypass

Prime Agent n’a qu’un mode, Bypass. Allume « Autoriser le mode Bypass » dans Réglages › Agents : sans lui, un agent en Bypass ne démarre pas. Lis d’abord l’onglet Permissions.

## 5. Brancher un compte

Aucun compte n’existe au départ : un agent ne démarre que sur un compte que tu as ajouté. Ouvre Réglages › Comptes et harness, **Ajouter un compte…**, harness « Prime Agent ». Dans la fiche du compte, mets le mode des nouveaux agents sur **Bypass**.

Prime Agent n’a qu’un compte dans Escouade : celui de la machine, `~/.prime/agent`. Un second compte n’aurait pas de dossier à lui. Prime Agent sait déplacer son dossier (`PRIME_AGENT_CODING_AGENT_DIR`), mais son démon, un pour toutes les sessions de l’utilisateur, garde le dossier avec lequel il a démarré : deux comptes se mêleraient.

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** (nom, ordre, réglages) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **La configuration, les sessions et les identifiants de Prime Agent** restent les siens, dans `~/.prime/agent`. Escouade n’en lit aucun.
