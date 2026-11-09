# Installer Discord

## 1. Créer le webhook dans Discord

1. Dans Discord, ouvre les réglages du salon où les messages doivent arriver.
2. **Intégrations › Webhooks › Nouveau webhook**.
3. Donne-lui un nom et un avatar si tu veux : ce sont eux que le salon affichera.
4. **Copier l’URL du webhook**. Elle commence par `https://discord.com/api/webhooks/`.

Cette adresse suffit à poster dans le salon : garde-la comme un mot de passe.

## 2. Installer le plugin

1. Dans Escouade, ouvre la vue Plugins (l’icône en forme de pièce de puzzle de la barre des projets).
2. **Store**, cherche « Discord », puis **Installer**. Tu peux aussi partir de sa page sur <https://store.escouade.dev/plugins/dev.escouade.discord> et **Ouvrir dans Escouade**.
3. La fenêtre « Installer Discord ? » dit ce qu’il pourra faire : recevoir `ticket.column` et `agent.status`, se connecter à `discord.com`, garder un secret. **Installer**.

## 3. Coller l’adresse du webhook

1. Sur la page du plugin, onglet **Réglages**, section **Secrets** : « Adresse du webhook Discord ».
2. Colle l’URL dans le champ, puis **Garder**. La ligne dit alors « gardé dans le trousseau ».

**Oublier** efface l’adresse : plus rien ne part.

## 4. Choisir les projets

Onglet **Projets** : le plugin est actif dans chaque projet qui n’est pas privé. Un projet privé est coupé d’office : rien ne part pour lui tant que tu ne l’y actives pas. Coupe un projet dont tu ne veux pas les messages dans ce salon.

Tous les projets actifs postent dans le même salon : il n’y a qu’une adresse de webhook.

## Ce qu’Escouade garde, et où

- **L’adresse du webhook** : dans le trousseau du système, sous un nom propre au plugin. Elle n’est écrite dans aucun fichier.
- **Le plugin** : ses fichiers dans `~/.escouade/plugins/dev.escouade.discord`, et son installation (version, empreinte, tes choix par projet) dans `~/.escouade/plugins.json`.
- **Le journal de ses envois** : `~/.escouade/plugin-logs/dev.escouade.discord.log`. L’adresse du webhook y est masquée.

Désinstaller efface ses fichiers, ses réglages, son secret et son journal, sauf si tu coches « Garder ses réglages et ses secrets pour une prochaine installation ».
