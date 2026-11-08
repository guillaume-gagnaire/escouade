# Installer

Trois étapes : Antigravity CLI sur ta machine, le plugin dans Escouade, puis un compte.

## 1. Installer Antigravity CLI

Installe-le avec son script :

- Windows : `irm https://antigravity.google/cli/install.ps1 | iex`
- macOS et Linux : `curl -fsSL https://antigravity.google/cli/install.sh | bash`

Escouade trouve `agy` dans le `PATH`, ou là où son script le pose. Réglages › Comptes et harness montre l’exécutable trouvé et sa version. Le plugin a été essayé avec Antigravity CLI 1.4.9. `agy update` le met à jour ; Escouade coupe ses mises à jour automatiques pour ses agents.

## 2. Installer le plugin

1. Ouvre la vue Plugins : l’icône en forme de pièce de puzzle de la barre des projets, ou Réglages › Plugins.
2. S’il n’est pas déjà dans **Installés**, ouvre **Store** et cherche « Antigravity CLI ».
3. Sur sa page, clique sur **Installer**. La fenêtre d’installation dit ce qu’il pourra faire : « Tourne dans Escouade, isolé » et « Lit le dossier de compte de tes agents ». Confirme.

Comme tout plugin, il se coupe par son interrupteur dans la vue Plugins et se désinstalle depuis sa page. Coupé ou désinstallé, aucun agent d’Antigravity ne démarre ; leurs conversations restent.

## 3. Brancher un compte

Aucun compte n’existe au départ : un agent ne démarre que sur un compte que tu as ajouté.

1. Ouvre Réglages › Comptes et harness, puis clique sur **Ajouter un compte…** ; dès que deux harness sont installés, choisis le harness « Antigravity CLI ».
2. Choisis :
   - **Le compte de la machine** : ta connexion Google, proposée connectée ou non. Pas encore connecté, **Ajouter ce compte** l’ajoute puis lance `agy` dans un terminal de la fenêtre, qui ouvre ton navigateur. « Se connecter… », sur sa fiche, refait la même chose.
   - **Un nouveau compte** : une clé d’API Gemini (sur aistudio.google.com, « Get API key »). Ses agents passent par l’API Gemini de cette clé, avec ses modèles seulement. « Changer la clé… » la redonne.
3. Dans la fiche du compte, règle le modèle, l’effort et le mode des nouveaux agents.

Un second compte Google n’est pas possible : la connexion d’`agy` est celle de la machine, quel que soit le dossier.

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** (nom, ordre, réglages) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **La connexion Google** reste dans le trousseau du système, où `agy` la garde. Escouade ne voit jamais tes identifiants, seulement que la connexion est là ; le plugin ne lit jamais le trousseau.
- **Une clé d’API** est gardée dans le trousseau du système, et donnée aux agents de son compte par `GEMINI_API_KEY` : jamais dans un fichier ni sur une ligne de commande. Une clé `GEMINI_API_KEY` de ta machine n’est pas donnée à un agent qui ne passe pas par elle.
- **Chaque agent** a son propre dossier, retiré avec lui, qui tient lieu de dossier personnel à `agy` : ses règles, ses conversations et le serveur MCP d’Escouade y sont gardés, jamais dans ton `~/.gemini`, que Gemini CLI partage. Escouade retire le jeton d’Escouade de ce dossier quand le process de l’agent s’arrête.
