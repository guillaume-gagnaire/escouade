# Installer

Trois étapes : Copilot CLI sur ta machine, le plugin dans Escouade, puis un compte.

## 1. Installer Copilot CLI

```sh
npm install -g @github/copilot
```

Réglages › Comptes et harness montre l’exécutable trouvé, sa version et son état. Le plugin a été essayé avec Copilot CLI 1.0.98. `npm install -g @github/copilot@latest` le met à jour.

## 2. Installer le plugin

1. Ouvre la vue Plugins : l’icône en forme de pièce de puzzle de la barre des projets, ou Réglages › Plugins.
2. S’il n’est pas déjà dans **Installés**, ouvre **Store** et cherche « Copilot CLI ».
3. Sur sa page, clique sur **Installer**. La fenêtre d’installation dit ce qu’il pourra faire : « Tourne dans Escouade, isolé » et « Lit les fichiers de tes agents » (leur dépôt). Confirme.

Comme tout plugin, il se coupe par son interrupteur dans la vue Plugins et se désinstalle depuis sa page. Coupé ou désinstallé, aucun agent de Copilot ne démarre ; leurs conversations restent.

## 3. Brancher un compte

Aucun compte n’existe au départ : un agent ne démarre que sur un compte que tu as ajouté.

1. Ouvre Réglages › Comptes et harness, puis clique sur **Ajouter un compte…** ; dès que deux harness sont installés, choisis le harness « Copilot CLI ».
2. Choisis :
   - **Le compte de la machine**, ton Copilot habituel (`~/.copilot`). **Ajouter ce compte** l’ajoute tel quel.
   - **Un nouveau compte**, avec son propre dossier. Donne-lui un nom, choisis ce qu’il reprend du compte de la machine (`mcp-config.json`, `copilot-instructions.md`, `agents`, `skills`), par lien ou par copie, puis **Créer et se connecter**.
3. Pour un nouveau compte, un terminal s’ouvre avec `copilot login` dans le dossier du compte : il te donne un code à saisir sur github.com/login/device. La fenêtre dit « Connecté au compte » suivi de `@` et de ton nom d’utilisateur GitHub.
4. Clique sur **Terminé**.

Dans la fiche du compte, règle le modèle, l’effort et le mode des nouveaux agents.

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** (nom, ordre, réglages) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **Le dossier du compte** : `~/.copilot` pour le compte de la machine, lancé sans toucher à `COPILOT_HOME` ; pour un nouveau compte, un dossier à lui sous `~/.escouade/copilot/`, donné à Copilot par `COPILOT_HOME`.
- **La connexion** reste celle de Copilot : son jeton est dans le trousseau du système. `config.json` et `settings.json` ne sont jamais copiés d’un compte à l’autre. Escouade ne prend ni ne recopie aucun jeton, pas même celui du GitHub CLI.
- Un compte autre que celui de la machine ne reçoit jamais les jetons de l’environnement (`GH_TOKEN`, `GITHUB_TOKEN`, `COPILOT_GITHUB_TOKEN`) : Copilot les préférerait à sa connexion.

Les crédits d’IA du mois viennent de chaque appel de ses agents, sinon d’un `copilot` du compte lancé au plus toutes les 5 minutes le temps de les lire.
