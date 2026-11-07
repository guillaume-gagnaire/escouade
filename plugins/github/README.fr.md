# GitHub

GitHub dans Escouade, sur un seul compte par instance (github.com, GHE.com ou un GitHub Enterprise Server). Le plugin ouvre les pull requests de tes agents et de tes tickets, suit leur CI, leurs relectures et leurs déploiements, merge quand c'est vert, publie les relectures que tu valides. Sur le compte de github.com, il sert aussi GitHub Issues comme source du Kanban : les issues d'un dépôt importées en tickets, fermées, rouvertes ou étiquetées quand leur ticket avance.

## Pour qui

Pour toi si tes projets ont leur dépôt sur GitHub.

## Ce qu'il te demande

- Un compte GitHub, et soit un jeton d'accès personnel, soit la CLI `gh` installée et connectée (`gh auth login`), dont Escouade prend le jeton.
- Un compte par instance : github.com, une instance GHE.com, ou ton GitHub Enterprise Server.

## Ce qu'il peut faire

- Joindre l'API de GitHub (`api.github.com`, `api.<nom>.ghe.com`, ou ton Enterprise Server), et les hôtes où GitHub range les journaux d'Actions, qu'il joint sans ton jeton.
- Recevoir de toi le jeton de chacun de tes comptes GitHub, qu'Escouade garde dans le trousseau du système.
- Lire tes dépôts, leurs PR, checks, relectures, déploiements et issues ; ouvrir une PR, la merger ou demander son auto-merge ; relancer des jobs en échec ; publier une relecture ou une réponse ; commenter, fermer, rouvrir ou étiqueter une issue.
- Rien d'autre : il ne lance aucun programme (ni `git`, ni `gh` : c'est Escouade qui pousse les branches et demande son jeton à `gh`), ne lit aucun fichier et ne reçoit rien de tes agents en dehors de ce qu'Escouade lui demande de publier.

## En savoir plus

Sa documentation a cinq onglets : **Installer** (le connecter pas à pas), **Utiliser** (ce qu'il fait, écran par écran), **Permissions** (ce qu'il lit, écrit et envoie), **Dépannage** (ses messages) et **Développer** (pour qui veut lire ou modifier son code).
