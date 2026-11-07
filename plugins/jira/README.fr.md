# Jira

Les tickets d'un projet Jira Cloud dans le Kanban d'Escouade, ses statuts comme colonnes. Tu lies un projet Jira à un projet Escouade, tu importes ses tickets ouverts, et chaque ticket Jira suit le sien : quand le ticket change de colonne, Jira passe la transition qui mène au statut choisi, et Escouade peut y laisser un commentaire.

## Pour qui

Pour toi si ton équipe suit son travail dans Jira Cloud (un site `….atlassian.net`) et que tu veux confier ces tickets à des agents sans les recopier.

## Ce qu'il te demande

- Ton site Jira Cloud, l'e-mail de ton compte Atlassian et un jeton d'API Atlassian, créé sur la page de sécurité de ton compte Atlassian.
- Rien à installer sur ta machine.

## Ce qu'il peut faire

- Joindre les sites `*.atlassian.net`, et le site que tu lui donnes s'il est ailleurs ; aucun autre hôte.
- Recevoir de toi le jeton d'API de ton compte, qu'Escouade garde dans le trousseau du système. Le site et l'e-mail ne sont pas secrets : Escouade les lui donne avec le compte.
- Lire tes projets Jira, leurs statuts et leurs tickets ouverts ; faire passer à un ticket la transition de son workflow ; commenter un ticket.
- Rien d'autre : il ne lance aucun programme, ne lit aucun fichier et ne reçoit rien de ce que font tes agents, en dehors du texte des commentaires qu'Escouade lui demande de poster.

## En savoir plus

Sa documentation a cinq onglets : **Installer** (le connecter pas à pas), **Utiliser** (ce qu'il fait, écran par écran), **Permissions** (ce qu'il lit, écrit et envoie), **Dépannage** (ses messages) et **Développer** (pour qui veut lire ou modifier son code).
