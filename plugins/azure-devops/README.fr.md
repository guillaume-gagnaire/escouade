# Azure DevOps

Azure DevOps dans Escouade, sur le compte de ton organisation (dev.azure.com, ou une collection d'Azure DevOps Server). Le plugin sert deux choses : l'hébergeur de code (les pull requests d'Azure Repos de tes agents et de tes tickets, leurs stratégies de branche, leurs pipelines, leurs relectures et leur merge) et la source Azure Boards du Kanban (les éléments de travail d'un projet importés en tickets, qui changent d'état quand leur ticket avance).

Il a été écrit d'après la documentation de Microsoft et testé contre un faux serveur ; il n'a pas encore tourné sur une vraie organisation : commence sur un projet sans enjeu.

## Pour qui

Pour toi si tes projets ont leur dépôt dans Azure Repos, ou si ton équipe suit son travail dans Azure Boards.

## Ce qu'il te demande

- Un compte de ton organisation, et soit un jeton d'accès personnel (PAT) avec les droits Code (lecture et écriture), Build (lecture et exécution) et Work Items (lecture et écriture), soit, sur dev.azure.com, la CLI `az` installée et connectée (`az login`), dont Escouade prend le jeton.
- Un compte par organisation ou collection.

## Ce qu'il peut faire

- Joindre `dev.azure.com` et ses sous-domaines, `*.visualstudio.com`, et l'Azure DevOps Server que tu ajoutes.
- Recevoir de toi le jeton de chacun de tes comptes, qu'Escouade garde dans le trousseau du système.
- Lire les PR de tes dépôts, leurs stratégies, statuts, builds et journaux, leurs fils de commentaires ; ouvrir une PR, la merger ou activer son auto-complete ; relancer un build ; publier une relecture et son vote, répondre dans un fil et le résoudre. Pour Azure Boards : lire les projets, leurs types et leurs états, leurs éléments de travail ; changer l'état d'un élément ; le commenter.
- Rien d'autre : il ne lance aucun programme (ni `git`, ni `az` : c'est Escouade qui pousse les branches et demande son jeton à `az`), ne lit aucun fichier et ne reçoit rien de tes agents en dehors de ce qu'Escouade lui demande de publier.

## En savoir plus

Sa documentation a cinq onglets : **Installer** (le connecter pas à pas), **Utiliser** (ce qu'il fait, écran par écran), **Permissions** (ce qu'il lit, écrit et envoie), **Dépannage** (ses messages) et **Développer** (pour qui veut lire ou modifier son code).
