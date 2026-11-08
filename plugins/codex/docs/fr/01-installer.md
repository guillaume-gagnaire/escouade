# Installer

Trois étapes : Codex sur ta machine, le plugin dans Escouade, puis un compte.

## 1. Installer Codex

```sh
npm install -g @openai/codex
```

Escouade lance le binaire natif du paquet npm (sous Windows, le `codex.exe` du paquet, pas son lanceur). L’app de bureau de Codex installe souvent une version plus ancienne, que le `PATH` peut trouver d’abord : Réglages › Comptes et harness montre l’exécutable trouvé et sa version. Le plugin a été essayé avec Codex 0.163.0. `npm install -g @openai/codex@latest` le met à jour.

## 2. Installer le plugin

1. Ouvre la vue Plugins : l’icône en forme de pièce de puzzle de la barre des projets, ou Réglages › Plugins.
2. S’il n’est pas déjà dans **Installés**, ouvre **Store** et cherche « Codex ».
3. Sur sa page, clique sur **Installer**. La fenêtre d’installation dit ce qu’il pourra faire : « Tourne dans Escouade, isolé », « Lit les fichiers de tes agents » (leur dépôt) et « Lit le dossier de compte de tes agents ». Confirme.

Comme tout plugin, il se coupe par son interrupteur dans la vue Plugins et se désinstalle depuis sa page. Coupé ou désinstallé, aucun agent de Codex ne démarre ; leurs conversations restent.

## 3. Brancher un compte

Aucun compte n’existe au départ : un agent ne démarre que sur un compte que tu as ajouté.

1. Ouvre Réglages › Comptes et harness, puis clique sur **Ajouter un compte…** ; dès que deux harness sont installés, choisis le harness « Codex ».
2. Choisis :
   - **Le compte de la machine**, ton Codex habituel (`~/.codex`), proposé quand Codex y est connecté. **Ajouter ce compte** l’ajoute tel quel.
   - **Un nouveau compte**, avec son propre dossier. Donne-lui un nom, choisis ce qu’il reprend du compte de la machine (`config.toml`, `AGENTS.md`, `skills`, `prompts`, `rules`), par lien ou par copie, puis **Créer et se connecter**.
3. Pour un nouveau compte, un terminal s’ouvre avec `codex login` dans le dossier du compte : ton navigateur s’ouvre sur la connexion de ChatGPT. La fenêtre dit « Connecté au compte » suivi de l’adresse dès qu’`auth.json` est écrit.
4. Clique sur **Terminé**.

Dans la fiche du compte, règle le modèle, l’effort et le mode des nouveaux agents.

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** (nom, ordre, réglages) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **Le dossier du compte** : `~/.codex` pour le compte de la machine, lancé sans toucher à `CODEX_HOME` ; pour un nouveau compte, un dossier à lui sous `~/.escouade/codex/`, donné à Codex par `CODEX_HOME`.
- **La connexion** reste celle de Codex : `auth.json` dans ce dossier. Elle n’est jamais copiée d’un compte à l’autre : son jeton change à chaque renouvellement, et deux dossiers qui s’en serviraient se déconnecteraient l’un l’autre.
- Le plugin ne lit jamais `auth.json`. Il ne lit de ce dossier que `config.toml`, et seulement pour un agent à qui le serveur d’Escouade est refusé.

Le quota du compte vient de Codex lui-même : de ses agents pendant qu’ils tournent, sinon d’un `codex app-server` du compte lancé toutes les 5 minutes le temps de le lire.
