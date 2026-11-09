# Installer

Trois étapes : Claude Code sur ta machine, le plugin dans Escouade, puis un compte.

## 1. Installer Claude Code

Installe-le avec son script, comme pour t’en servir dans un terminal :

- Windows : `irm https://claude.ai/install.ps1 | iex`
- macOS et Linux : `curl -fsSL https://claude.ai/install.sh | bash`

Escouade trouve `claude` dans le `PATH` ou aux endroits où son script le pose, et aussi dans WSL. Réglages › Comptes et harness montre l’exécutable trouvé, sa version et son état. Le plugin a été essayé de Claude Code 2.1.283 à 2.1.301. `claude update` le met à jour.

## 2. Installer le plugin

1. Ouvre la vue Plugins : l’icône en forme de pièce de puzzle de la barre des projets, ou Réglages › Plugins.
2. S’il n’est pas déjà dans **Installés**, ouvre **Store** et cherche « Claude Code ».
3. Sur sa page, clique sur **Installer**. La fenêtre d’installation dit ce qu’il pourra faire : « Tourne dans Escouade, isolé ». Confirme.

Comme tout plugin, il se coupe par son interrupteur dans la vue Plugins et se désinstalle depuis sa page. Coupé ou désinstallé, aucun agent de Claude Code ne démarre ; leurs conversations restent.

## 3. Brancher un compte

Aucun compte n’existe au départ : un agent ne démarre que sur un compte que tu as ajouté.

1. Ouvre Réglages › Comptes et harness, puis clique sur **Ajouter un compte…** (ou **Ajouter un compte Claude Code…** dans son groupe).
2. Choisis :
   - **Le compte de la machine**, proposé quand Claude Code est déjà connecté dans son dossier habituel. **Ajouter ce compte** l’ajoute tel quel.
   - **Un nouveau compte**, avec son propre dossier. Donne-lui un nom, choisis ce qu’il reprend du compte de la machine (`settings.json`, `CLAUDE.md`, `skills`, `agents`, `commands`, `plugins`, `hooks`, `output-styles`), par lien ou par copie, puis **Créer et se connecter**.
3. Pour un nouveau compte, un terminal s’ouvre avec le `claude` du compte : connecte-toi, avec `/login` si Claude Code ne le propose pas. La fenêtre dit « Connecté au compte » suivi de l’adresse dès que la connexion est faite.
4. Clique sur **Terminé**.

Dans la fiche du compte, règle le modèle, l’effort et le mode des nouveaux agents.

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** (nom, ordre, réglages) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **Le dossier du compte** : pour le compte de la machine, celui de Claude Code (`~/.claude`, ou celui de `CLAUDE_CONFIG_DIR`) ; pour un nouveau compte, un dossier à lui sous `~/.escouade/claude/`, donné à `claude` par `CLAUDE_CONFIG_DIR`.
- **La connexion** reste celle de Claude Code : `.credentials.json` dans le dossier du compte, ou le trousseau sous macOS. Escouade la lit pour savoir si le compte est connecté et pour son quota. Le plugin ne la voit jamais.
- La connexion et `.claude.json` ne sont jamais partagés d’un compte à l’autre.

Supprimer un compte le retire de la liste : son dossier reste sur le disque, avec sa connexion et ses conversations.
