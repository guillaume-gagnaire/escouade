---
number: 15
title: "Les plugins proposés par un projet : le store pour seule source, une bande qui n'installe rien, des réglages filtrés"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-288"
pages: ["projets/escouade-yml"]
tickets: ["ESC-288"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"projets/escouade-yml","added":16,"removed":5}]
accepted_at: "2026-10-30T16:16:11+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

PLUGINS.md §3.3 prévoyait `plugins.recommended` (« des identifiants et une source ») et `plugins.<id>.*` dans `.escouade.yml`, sans ticket jusqu'à PLG-14 (ESC-288). Il fallait trancher la source, la forme des réglages, où et comment la bande se tait, et comment un plugin reçoit les réglages d'un projet, sans qu'un dépôt cloné puisse lancer un programme ni faire fuir un secret.

## Décision

- `plugins.recommended` est une liste d'identifiants du store : la source est store.escouade.dev, un dépôt git ou un dossier ne se propose pas.
- `plugins.<id>` est un groupe de valeurs simples (texte, nombre, booléen). Ces clés sont celles du fichier seul : jamais appliquées au projet, jamais marquées dans la fenêtre, jamais écrites par l'export, qui les garde telles qu'elles sont.
- À l'ouverture, un toast dit combien de plugins proposés manquent. En tête de Réglages › Projets, une bande les nomme avec ce qu'ils demandent et « Voir dans le store », qui ouvre leur page dans la vue Plugins. L'installation reste le geste de l'utilisateur, par la fenêtre qui montre les permissions.
- « Pas pour ce projet » fait taire la bande, sur cette machine, pour les plugins qu'elle nommait. Un plugin proposé plus tard la fait revenir pour lui seul.
- `settings/get` prend un `project` facultatif. Les réglages du fichier passent alors devant ceux de l'utilisateur, mais seulement les clés de `contributes.settings`, d'une valeur de leur type, jamais un secret, et seulement si le plugin est actif dans ce projet : un projet privé ne donne rien à un plugin qui n'y est pas activé.

## Conséquences

Le schéma de `.escouade.yml` gagne le groupe `plugins` (`patternProperties` par identifiant). Le protocole des plugins gagne `settings/get` avec `project`, et le SDK `settings(project?)`. Un plugin hors du store ne peut pas être proposé par un projet : on l'installe à la main. Le silence de « Pas pour ce projet » vit sur chaque poste et ne se partage pas.
