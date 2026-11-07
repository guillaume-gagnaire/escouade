---
number: 17
title: "L'index du code n'annonce plus d'économie qu'on ne mesure pas"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-307"
pages: ["docs/index-du-code"]
tickets: ["ESC-307"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"docs/index-du-code","added":13,"removed":12}]
accepted_at: "2026-10-30T21:03:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'« économie » de l'index (67 % en moyenne) est la définition de tokensave : contre la lecture des fichiers entiers. Face à ce qu'un agent fait vraiment, elle est fausse : sur les sondes, l'index coûte 63 % de plus que le grep informé ; avec de vrais agents Haiku (IDX-4, ESC-299), index + hook font dépenser 8 % de moins que grep seul (−13 % sur les questions, +14 % sur les petits changements), dans le bruit.</context>
<parameter name="decision">Escouade ne promet plus ce qu'il ne mesure pas. Les réponses MCP de l'index n'ont plus de ligne d'économie (le compte interne reste pour les Stats) ; le hook et les consignes disent ce que l'index trouve (symbole, appelants justes malgré les homonymes, appelés, impact), sans chiffre ni promesse de tokens ; la vue Stats devient « Index du code » (réponses, tokens des réponses, « Lecture de fichiers évitée (estimation haute) » expliquée, plus de dollars) ; le site dit l'effet mesuré avec de vrais agents et que IDX-6 est en cours.</decision>
<parameter name="consequences">Moins de tokens envoyés à chaque appel ; les statistiques d'avant gardent leurs valeurs mais ne montrent plus de dollars. Le banc des sondes compte ce que l'agent reçoit, sans la ligne. Le résultat d'IDX-6 pourra changer le discours, pas avant.

## Décision



## Conséquences
