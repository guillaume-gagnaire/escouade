# GitLab

GitLab dans Escouade, sur gitlab.com et tes instances auto-hébergées, un compte par instance. Le plugin ouvre les merge requests (MR) de tes agents et de tes tickets, suit leurs pipelines, leurs discussions et leurs déploiements, merge quand c'est vert, publie les relectures que tu valides. Sur le même compte, il sert aussi GitLab Issues comme source du Kanban : les issues d'un projet importées en tickets, fermées, rouvertes ou étiquetées quand leur ticket avance.

Il a été écrit d'après la documentation de GitLab et essayé contre des réponses enregistrées sur gitlab.com, pas encore contre une vraie instance avec un jeton : commence sur un projet sans enjeu.

## Pour qui

Pour toi si tes projets ont leur dépôt sur gitlab.com ou sur une instance GitLab.

## Ce qu'il te demande

- Un compte GitLab, et soit un jeton d'accès personnel avec le droit `api`, soit la CLI `glab` installée et connectée (`glab auth login`), dont Escouade prend le jeton.
- Un compte par instance.

## Ce qu'il peut faire

- Joindre `gitlab.com` et les instances GitLab que tu ajoutes.
- Recevoir de toi le jeton de chacun de tes comptes GitLab, qu'Escouade garde dans le trousseau du système.
- Lire tes projets, leurs MR, pipelines, jobs, approbations, discussions, déploiements et issues ; ouvrir une MR, la merger ou demander son auto-merge ; relancer les jobs en échec ; publier une relecture, répondre dans une discussion, la résoudre ; commenter, fermer, rouvrir ou étiqueter une issue.
- Rien d'autre : il ne lance aucun programme (ni `git`, ni `glab` : c'est Escouade qui pousse les branches et demande son jeton à `glab`), ne lit aucun fichier et ne reçoit rien de tes agents en dehors de ce qu'Escouade lui demande de publier.

## En savoir plus

Sa documentation a cinq onglets : **Installer** (le connecter pas à pas), **Utiliser** (ce qu'il fait, écran par écran), **Permissions** (ce qu'il lit, écrit et envoie), **Dépannage** (ses messages) et **Développer** (pour qui veut lire ou modifier son code).
