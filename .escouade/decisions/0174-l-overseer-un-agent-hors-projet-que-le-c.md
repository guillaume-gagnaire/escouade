---
number: 174
id: "l-overseer-un-agent-hors-projet-que-le-c"
title: "L'Overseer : un agent hors projet que le cœur tient à l'utilisateur seul (consignes prouvées par ses mots, actions qui sortent gardées, annulations bornées)"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-462"
pages: ["overseer/overseer","fenetre/reglages","harness/serveur-mcp"]
tickets: ["ESC-462"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"overseer/overseer","added":52,"removed":0},{"page":"fenetre/reglages","added":5,"removed":0},{"page":"harness/serveur-mcp","added":4,"removed":0}]
accepted_at: "2026-11-15T10:28:43+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

docs/OVERSEER.md laissait au constructeur la liste des actions annulables et leur délai, le regroupement des événements et le renouvellement de la session ; il fallait aussi que le cœur, et non la seule consigne de l'agent, garantisse qu'il n'obéit qu'à l'utilisateur, alors qu'il lit des tickets, des documents et des réponses d'agents qui peuvent dire n'importe quoi.

## Décision

L'Overseer est un agent d'Escouade hors de tout projet (projet « overseer », dossier et mémoire dans le dossier système), éteint par défaut, sur Haiku de Claude Code par défaut, en lecture seule ; il voit ce que voit un agent d'un projet non privé, et aucun autre appelant ne le voit. Une consigne ne naît que par add_directive, qui nomme un des 20 derniers messages de l'utilisateur (moins de 24 h) et en cite les mots exacts, qui doivent nommer chaque action (fusionner, pousser, tag, publier, valider) ; elle garde sa provenance et sa portée (projet, ticket, release, fin, une fois). Les actions qui sortent (approve_ticket, share_docs, l'autorisation d'une commande git push/tag/merge, gh pr merge, publication, et un message, un premier message ou une réponse qui en contient une) sont refusées sans consigne active qui les couvre. Ce qu'il lit lui est donné marqué comme données. Les événements (tours finis, questions, autorisations, colonnes À tester et Terminé, blocages, nouvelles des PR, échéances programmées) sont regroupés 3 s, et ceux qui arrivent pendant son tour partent à sa fin ; après 120 messages ou 70 % de contexte, une nouvelle session reçoit le fil. Annulables : un agent lancé (2 min, arrêté et archivé), un ticket créé (10 min, tant qu'il est À faire et jamais démarré), un message à un agent (retenu 15 s), une consigne (10 min), une tâche programmée créée (10 min) ; rien de ce qui est sorti ou a été lu. Dans le doute il laisse la demande à l'utilisateur (leave_to_user). Une discussion minimale (bouton de la barre de titre) et Réglages › Overseer précèdent l'écran d'OV-2.

## Conséquences

Un ticket piégé ne donne ni consigne ni fusion, prouvé par les tests Rust et l'e2e. Un « oui » seul ne donne aucun droit : l'utilisateur dit ce qu'il permet. Les messages de l'Overseer aux agents partent 15 s plus tard. La garde reconnaît les commandes par leurs mots : un script qui pousse sans les nommer n'est pas vu. L'Overseer consomme le quota de son compte à chaque réveil, d'où l'interrupteur éteint par défaut.
