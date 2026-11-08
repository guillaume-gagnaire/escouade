# Installer

Trois étapes : Gemini CLI sur ta machine, le plugin dans Escouade, puis un compte.

## 1. Installer Gemini CLI

Par npm (Node 20 ou plus) ou par Homebrew :

```sh
npm install -g @google/gemini-cli
brew install gemini-cli
```

Escouade trouve `gemini` dans le `PATH`, ou dans le dossier global de npm. Réglages › Comptes et harness montre l’exécutable trouvé et sa version. Le plugin a été essayé avec Gemini CLI 0.66.0, contre un faux serveur : pas encore avec un vrai compte. `npm install -g @google/gemini-cli` le met à jour.

## 2. Installer le plugin

1. Ouvre la vue Plugins : l’icône en forme de pièce de puzzle de la barre des projets, ou Réglages › Plugins.
2. S’il n’est pas déjà dans **Installés**, ouvre **Store** et cherche « Gemini CLI ».
3. Sur sa page, clique sur **Installer**. La fenêtre d’installation dit ce qu’il pourra faire : « Tourne dans Escouade, isolé », « Lance pour ses agents ce programme de ton ordinateur, avec tes droits », avec `GEMINI_CLI_TRUST_WORKSPACE=true gemini --acp`, et « Lit le dossier de compte de tes agents ». Confirme.

Comme tout plugin, il se coupe par son interrupteur dans la vue Plugins et se désinstalle depuis sa page. Coupé ou désinstallé, aucun agent de Gemini ne démarre ; leurs conversations restent.

## 3. Brancher un compte

Aucun compte n’existe au départ : un agent ne démarre que sur un compte que tu as ajouté.

1. Ouvre Réglages › Comptes et harness, puis clique sur **Ajouter un compte…** ; dès que deux harness sont installés, choisis le harness « Gemini CLI ».
2. Choisis **Un nouveau compte**, puis l’un ou l’autre :
   - **Un compte Google** : Escouade lance `gemini` dans un terminal de la fenêtre, avec le dossier du compte pour `GEMINI_CLI_HOME`. Il ouvre ton navigateur sur ton compte Google et garde sa connexion dans ce dossier. « Se reconnecter… » la refait.
   - **Une clé d’API Gemini** (sur aistudio.google.com, « Get API key »). « Changer la clé… » la redonne.
3. Dans la fiche du compte, règle le modèle et le mode des nouveaux agents.

**Le compte de la machine** n’est proposé que si la machine a une clé (`GEMINI_API_KEY`, `GOOGLE_API_KEY`, Vertex AI) : la connexion Google de ton `~/.gemini` n’est jamais donnée aux agents. Une clé ou Vertex AI de la machine ne passe jamais à un compte ajouté.

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** (nom, ordre, réglages) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **Un compte Google** a son dossier sous `~/.escouade/gemini/`, où Gemini garde sa connexion. Escouade ne la lit jamais : il voit seulement qu’elle est là, avec l’adresse du compte. Ses agents la reçoivent par `GOOGLE_APPLICATION_CREDENTIALS`.
- **Une clé d’API** est gardée dans le trousseau du système, et donnée à ses agents par `GEMINI_API_KEY` : jamais dans un fichier ni sur une ligne de commande.
- **Chaque agent** a son propre dossier de configuration (`GEMINI_CLI_HOME`), retiré avec lui : ses réglages, ses sessions et ses règles y sont gardés, jamais dans ton `~/.gemini`, qu’Antigravity CLI partage. Escouade y coupe ses mises à jour et ses statistiques d’usage.

Le plugin ne voit ni la clé ni la connexion d’un compte.
