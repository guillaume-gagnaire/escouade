---
number: 211
id: "un-agent-peut-demander-la-suppression-de"
title: "Un agent peut demander la suppression des agents qu'il a créés ; la demande attend sur la carte, et l'utilisateur confirme en voyant ce qu'il perd"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-511"
pages: ["harness/serveur-mcp","fenetre/barre-laterale","harness/comptes"]
tickets: ["ESC-511"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/serveur-mcp","added":8,"removed":7},{"page":"fenetre/barre-laterale","added":20,"removed":6},{"page":"harness/comptes","added":1,"removed":0}]
accepted_at: "2026-11-18T12:18:26+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un agent orchestrateur crée des agents (create_agent) pour des sous-tâches, puis doit les laisser à l'utilisateur, qui les supprime à la main un par un. La page « Serveur MCP » excluait tout outil qui supprime un agent : un agent poussé par un contenu piégé pourrait s'en servir pour monter en droits. La PR 44 (Metalzoid) a ajouté delete_agent avec une modale qui s'ouvrait d'elle-même : à la revue, cette modale prenait le focus par-dessus ce que faisait l'utilisateur, se perdait au rechargement, ne disait pas au créateur un refus, et proposait de supprimer le worktree sans dire ce qu'il contenait (ESC-511).

## Décision

Un outil MCP delete_agent, réservé aux agents d'Escouade, demande la suppression d'un agent que l'appelant a créé par create_agent (le créateur est gardé sur l'agent, AgentMeta.created_by). L'outil ne supprime rien et rien ne s'ouvre de soi-même : la demande (qui demande, depuis quand, le worktree voulu) est gardée sur l'agent visé dans l'état durable d'Escouade. Sa carte le dit (« « <créateur> » demande de le supprimer », avec « Voir » et « Refuser »), un toast l'annonce une fois, elle compte « en attente » (dossier, tuile du projet, barre d'état, Ctrl+J) ; « Voir » ouvre la modale « Supprimer l'agent ». Elle survit au rechargement et au redémarrage, part avec l'agent visé ou son créateur ; un agent a au plus 5 demandes ouvertes. Refusée, le créateur le lit en tête de son prochain message. La modale « Supprimer l'agent », demandée par un agent ou par l'utilisateur, dit ce qui serait perdu, lu par git à l'ouverture (3 s au plus, puis « état du worktree inconnu ») : fichiers non commités (nombre et premiers noms), commits non fusionnés dans la branche de départ, tour ou demande en cours, ticket lié. La case du worktree est décochée quand il y a du travail non commité ou non fusionné (ou un état inconnu), cochée sinon (comme l'agent l'a demandé). Le focus va au cadre : Entrée ne confirme rien. Le compte, le modèle, l'effort et le mode de create_agent passent par les juges d'un ticket. Refusé pour soi-même, pour un agent que l'appelant n'a pas créé, et à Claude hors Escouade.

## Conséquences

L'exclusion « aucun outil ne supprime un agent » reste vraie pour l'outil : seul l'utilisateur supprime. Un agent piégé peut au plus poser 5 demandes sur les cartes de ses propres agents, sans qu'aucune modale prenne ce que tape l'utilisateur. Les agents créés avant ce changement n'ont pas de créateur : aucun agent ne peut demander leur suppression. Le modèle de la demande (qui, quoi, pour quel agent, depuis quand) est simple à verser dans la boîte « À toi » de la 2.3. Ouvrir la modale lance git sur le worktree (status et rev-list), borné à 3 s.
