# Gitea / Forgejo

Gitea, Forgejo et Codeberg comme hébergeurs de code d'Escouade, un compte par instance. Le plugin ouvre les pull requests de tes agents et de tes tickets, suit leur CI (statuts et Actions), leurs relectures et leurs fils, merge, et programme l'auto-merge sur une branche protégée. Il n'a pas de source de tickets dans Escouade : il sert aux PR et à la CI seulement.

Il a été écrit d'après les documents d'API de Forgejo et de Gitea et testé contre un faux serveur ; il n'a pas encore tourné sur une vraie instance : commence sur un dépôt sans enjeu.

## Pour qui

Pour toi si tes projets ont leur dépôt sur Codeberg, gitea.com, ou ta propre instance Gitea ou Forgejo.

## Ce qu'il te demande

- Un compte sur l'instance, et un jeton d'accès avec les droits `write:repository`, `write:issue` et `read:user`, créé dans les réglages de ton compte sur l'instance.
- Un compte par instance. Rien à installer sur ta machine.

## Ce qu'il peut faire

- Joindre `codeberg.org`, `gitea.com` et les instances que tu ajoutes.
- Recevoir de toi le jeton de chacun de tes comptes, qu'Escouade garde dans le trousseau du système.
- Lire les PR de tes dépôts, leurs statuts, leurs runs et jobs d'Actions et leurs journaux, leurs relectures et leurs commentaires ; ouvrir une PR, la merger ou programmer son auto-merge ; relancer les jobs en échec ; publier une relecture, répondre dans un fil et le résoudre, là où l'instance le permet.
- Rien d'autre : il ne lance aucun programme (c'est Escouade qui pousse les branches), ne lit aucun fichier et ne reçoit rien de tes agents en dehors de ce qu'Escouade lui demande de publier.

## En savoir plus

Sa documentation a cinq onglets : **Installer** (le connecter pas à pas), **Utiliser** (ce qu'il fait, écran par écran), **Permissions** (ce qu'il lit, écrit et envoie), **Dépannage** (ses messages) et **Développer** (pour qui veut lire ou modifier son code).
