---
title: "Rechercher dans les conversations"
subject: "Conversation"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

« Rechercher dans les conversations » trouve un message dans les conversations des agents, du projet affiché ou de tous les projets, archivés compris, et ouvre la conversation à cet endroit.

## Comportement

- `Ctrl+K` (Cmd+K sous macOS ; dans un terminal sous Windows, Ctrl+K reste au shell) l'ouvre en haut de la fenêtre : un champ, « Ce projet » (par défaut ; indisponible sans projet) ou « Tous les projets », et « Agents archivés » (coché par défaut).
- La recherche part à partir de 2 caractères, 150 ms après la frappe ; seule la réponse de la dernière compte, et une nouvelle recherche arrête la précédente.
- Elle lit les journaux des conversations, en lecture seule, sans tenir compte de la casse ni des accents : lettres latines accentuées, accents écrits à part, `œ` `æ` `ß`, apostrophes typographiques, `\` comme `/`, blancs successifs.
- Ce qui est cherché : les textes de l'utilisateur (et les noms de ses fichiers joints) et de l'agent, les notices, les questions et leurs options, les autorisations, les plans, les erreurs de tour et, pour les outils, leur résumé (commande, fichier depuis le dossier de l'agent, motif, adresse…) et leur sortie. Ni la réflexion, ni le contenu écrit par un Edit / Write.
- Un résultat par message ; les agents par dernière activité, les messages récents d'abord. 200 résultats au plus (« Les 200 premiers résultats : précise ta recherche. »), 5 s au plus (« N résultats, recherche arrêtée au bout de 5 s : précise ta recherche. »).
- Les résultats sont groupés par agent (nom, projet, « archivé »), chaque ligne avec un extrait sur une ligne (le nom de l'outil devant, la correspondance surlignée vers le début) et sa date relative ; « Aucun message ne correspond. » sinon. ↑ ↓ choisissent, Entrée ouvre, Échap ferme.
- Ouvrir un résultat ferme l'éditeur, affiche l'agent (un agent archivé s'ouvre en lecture, la liste « Archivés » dépliée) et fait défiler sa conversation jusqu'au message (celui d'un sous-agent : son appel), au milieu de la vue, surligné 2 s. Un message plus ancien que ceux dessinés est dessiné avec les 10 qui le précèdent. Ce point devient la position de lecture (voir *Conversation*). Un agent disparu : « Cet agent n'existe plus. ».

## Limites

- Le texte encore en cours de streaming n'est trouvé qu'une fois écrit dans le journal.
- Les accents des autres alphabets (grec, cyrillique) ne sont pas ignorés.
