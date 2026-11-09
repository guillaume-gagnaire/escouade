# Installer Slack

## 1. Créer le bot dans Slack

1. Sur <https://api.slack.com/apps>, crée une app (**Create New App › From scratch**) dans ton espace de travail.
2. **OAuth & Permissions › Bot Token Scopes** : ajoute `chat:write`. Le plugin n’a besoin de rien d’autre.
3. **Install to Workspace**, puis copie le **Bot User OAuth Token**. Il commence par `xoxb-`.
4. Crée, si besoin, le salon de chaque projet (voir plus bas son nom), et invites-y le bot : `/invite @le-nom-de-ton-app` dans le salon.

Ce jeton permet de poster au nom du bot : garde-le comme un mot de passe.

## 2. Installer le plugin

1. Dans Escouade, ouvre la vue Plugins (l’icône en forme de pièce de puzzle de la barre des projets).
2. **Store**, cherche « Slack », puis **Installer**. Tu peux aussi partir de sa page sur <https://store.escouade.dev/plugins/dev.escouade.slack> et **Ouvrir dans Escouade**.
3. La fenêtre « Installer Slack ? » dit ce qu’il pourra faire : tourner dans Escouade, isolé ; recevoir `ticket.column` et `agent.status` ; se connecter à `slack.com` ; garder un secret. **Installer**.

Il lui faut Escouade 2.2.0 ou plus récent.

## 3. Coller le jeton

1. Sur la page du plugin, onglet **Réglages**, section **Secrets** : « Jeton du bot Slack (xoxb-…) ».
2. Colle le jeton, puis **Garder**. La ligne dit alors « gardé dans le trousseau ».

**Oublier** efface le jeton : plus rien ne part.

## 4. Le nom du salon de chaque projet

Chaque projet poste dans `escouade-<nom du projet>`, écrit comme Slack écrit les noms de salon : en minuscules, sans accents, chaque suite de caractères autres que lettres, chiffres, `_` et `-` remplacée par un `-`, 80 caractères au plus.

| Projet | Salon |
| --- | --- |
| `Boutique` | `#escouade-boutique` |
| `Équipe Web` | `#escouade-equipe-web` |
| `API v2` | `#escouade-api-v2` |

Le fichier `channels.json` du plugin peut donner un autre salon à un projet, par son nom. Celui qui est publié ne nomme que le projet d’Escouade elle-même (`Escouade` → `#escouade-dev`). Ne le modifie pas dans le dossier installé : Escouade verrait que ses fichiers ont changé et le désactiverait.

## 5. Choisir les projets

Onglet **Projets** : le plugin est actif dans chaque projet qui n’est pas privé. Un projet privé est coupé d’office : rien ne part pour lui tant que tu ne l’y actives pas.

## Ce qu’Escouade garde, et où

- **Le jeton** : dans le trousseau du système, sous un nom propre au plugin. Il n’est écrit dans aucun fichier.
- **Le plugin** : ses fichiers dans `~/.escouade/plugins/dev.escouade.slack`, et son installation (version, empreinte, tes choix par projet) dans `~/.escouade/plugins.json`. Son module WASM compilé est gardé dans le cache d’Escouade, et refait s’il disparaît.
- **Son journal** : dans l’onglet **Journal** de sa page, et en entier dans `~/.escouade/plugin-logs/dev.escouade.slack.log`. Le jeton y est masqué.

Désinstaller efface ses fichiers, ses réglages, son secret et son journal, sauf si tu coches « Garder ses réglages et ses secrets pour une prochaine installation ».
