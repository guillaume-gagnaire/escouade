---
number: 325
id: "les-bornes-du-pilotage-des-navigateurs-p"
title: "Les bornes du pilotage des navigateurs par un agent"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-663"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-663"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-26T00:48:41+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un agent qui pilote un navigateur lit une page qu'Escouade ne tient pas pour digne de confiance (l'app développée, du contenu de tiers, une page d'agent) et agit sous les yeux de l'utilisateur. La sonde d'ESC-663 montre qu'un passe-plat vers le protocole DevTools donne les cookies (`Network.getCookies`) et le stockage, et que le monde isolé lit la valeur d'un champ mot de passe : les bornes doivent être dans le cœur et dans les scripts fixes.

## Décision

Les origines permises :
- celles des lancements du projet et de ses worktrees ;
- `localhost` à ses ports déclarés ;
- ses pages d'agent.

Une autre adresse `http(s)` est demandée à l'utilisateur, une fois par origine et par navigateur (choix C3). Jamais d'adresse d'Escouade, de `file:` ni d'autre schéma.

Ne sortent jamais vers l'agent : les cookies, le stockage, les en-têtes, le corps des réponses, la valeur d'un mot de passe (`«masqué»`). Le protocole DevTools n'est appelé que par une liste fermée de méthodes.

Sont refusés : taper dans un mot de passe (sauf accord, choix C2), envoyer un fichier, télécharger, taper un secret d'Escouade.

Qui pilote :
- l'agent, sur ses navigateurs et ceux que l'utilisateur lui donne ;
- l'Overseer lit et capture ceux des agents de son projet, sans les piloter ;
- Claude hors Escouade et un autre projet n'ont rien ;
- dans un projet privé, seuls ses agents voient ses navigateurs.

L'écran :
- un volet à côté par agent ;
- jamais ouvert quand l'utilisateur regarde ailleurs (la ligne attend), jamais rouvert après une fermeture ;
- pas de capture d'un navigateur caché ;
- 4 gestes par seconde et 600 par heure au plus.

Ce que lit l'agent est marqué comme donnée de tiers.

## Conséquences

L'agent ne peut ni sortir du projet sans l'utilisateur, ni lire ses sessions, ni agir sur ce qu'il ne voit pas. Une injection de consignes dans une page visible reste possible : c'est la limite commune à tout contenu lu, d'où le marquage. Le passage en largeur d'une capture se voit à l'écran, voulu.
