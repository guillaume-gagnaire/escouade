# Bitbucket

Bitbucket Cloud (bitbucket.org) comme hébergeur de code d'Escouade. Le plugin ouvre les pull requests de tes agents et de tes tickets, suit leurs statuts, leurs pipelines, leurs commentaires et leurs déploiements, merge, et publie les relectures que tu valides. Bitbucket n'a pas de source de tickets dans Escouade : il sert aux PR et à la CI seulement.

Il a été écrit d'après la documentation de l'API de Bitbucket Cloud et testé contre un faux serveur ; personne ne l'a encore essayé sur bitbucket.org : commence sur un dépôt sans enjeu.

## Pour qui

Pour toi si tes projets ont leur dépôt sur bitbucket.org. Bitbucket Data Center, dont l'API diffère, n'est pas pris en charge.

## Ce qu'il te demande

- Ton compte Atlassian : son e-mail, et un jeton d'API Atlassian avec les droits de Bitbucket (lecture du compte, des dépôts et des pipelines, lecture et écriture des pull requests).
- Rien à installer sur ta machine.

## Ce qu'il peut faire

- Joindre `api.bitbucket.org`, et aucun autre hôte.
- Recevoir de toi le jeton de ton compte, qu'Escouade garde dans le trousseau du système.
- Lire les PR de tes dépôts, leurs statuts, leurs pipelines et leurs journaux, leurs commentaires et leurs déploiements ; ouvrir une PR, la merger ; publier une relecture, répondre sous un commentaire, résoudre un fil.
- Rien d'autre : il ne lance aucun programme (c'est Escouade qui pousse les branches), ne lit aucun fichier et ne reçoit rien de tes agents en dehors de ce qu'Escouade lui demande de publier.

## En savoir plus

Sa documentation a cinq onglets : **Installer** (le connecter pas à pas), **Utiliser** (ce qu'il fait, écran par écran), **Permissions** (ce qu'il lit, écrit et envoie), **Dépannage** (ses messages) et **Développer** (pour qui veut lire ou modifier son code).
