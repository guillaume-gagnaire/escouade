# Installer

Trois étapes : Cursor CLI sur ta machine, le plugin dans Escouade, puis un compte.

## 1. Installer Cursor CLI

Installe-le avec son script :

- Windows : `irm 'https://cursor.com/install?win32=true' | iex`
- macOS et Linux : `curl https://cursor.com/install -fsS | bash`

Puis connecte-le dans un terminal : `agent login`. Escouade le trouve dans le `PATH`, et sous Windows dans `%LOCALAPPDATA%\cursor-agent` même absent du `PATH`. Réglages › Comptes et harness montre l’exécutable trouvé et sa version. Le plugin a été essayé avec Cursor CLI 2026.10.22. `agent update` le met à jour.

## 2. Installer le plugin

1. Ouvre la vue Plugins : l’icône en forme de pièce de puzzle de la barre des projets, ou Réglages › Plugins.
2. S’il n’est pas déjà dans **Installés**, ouvre **Store** et cherche « Cursor CLI ».
3. Sur sa page, clique sur **Installer**. La fenêtre d’installation dit ce qu’il pourra faire : « Tourne dans Escouade, isolé » et « Lance pour ses agents ce programme de ton ordinateur, avec tes droits », avec `agent acp`. Confirme.

Comme tout plugin, il se coupe par son interrupteur dans la vue Plugins et se désinstalle depuis sa page. Coupé ou désinstallé, aucun agent de Cursor ne démarre ; leurs conversations restent.

## 3. Brancher un compte

Aucun compte n’existe au départ : un agent ne démarre que sur un compte que tu as ajouté.

1. Ouvre Réglages › Comptes et harness, puis clique sur **Ajouter un compte…** ; dès que deux harness sont installés, choisis le harness « Cursor CLI ».
2. Choisis :
   - **Le compte de la machine** : la connexion de `agent login`. **Ajouter ce compte** l’ajoute tel quel.
   - **Un nouveau compte** : un autre compte Cursor, par sa clé d’API (sur cursor.com, Settings › API Keys). Donne-lui un nom et colle la clé.
3. Dans la fiche du compte, règle le modèle, l’effort et le mode des nouveaux agents. « Changer la clé… » redonne la clé d’un compte.

La connexion de `agent login` est celle de la machine : un dossier de configuration ne la déplace pas. C’est pourquoi un second compte passe par sa clé.

## Ce qu’Escouade garde d’un compte

- **La liste des comptes** (nom, ordre, réglages) est dans les réglages d’Escouade, `~/.escouade/settings.json`.
- **La clé d’API** d’un compte ajouté est gardée dans le trousseau du système, et donnée à ses agents par `CURSOR_API_KEY` : jamais dans un fichier ni sur une ligne de commande. Le plugin ne la voit jamais ; il sait seulement que le compte passe par une clé.
- **La connexion de la machine** reste celle de Cursor. Escouade ne la lit pas.
- **Chaque agent** a son propre dossier de configuration de Cursor (`CURSOR_CONFIG_DIR`), retiré avec lui : ses règles, son modèle et ses sessions y sont gardés, jamais dans ta config de Cursor.

Cursor ne dit pas son usage du mois : « Usage sur cursor.com », dans Réglages › Comptes et harness, et dans le panneau des quotas, ouvre son tableau de bord.
