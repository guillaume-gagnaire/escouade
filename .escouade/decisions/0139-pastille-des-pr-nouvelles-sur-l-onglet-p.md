---
number: 139
id: "pastille-des-pr-nouvelles-sur-l-onglet-p"
title: "Pastille des PR nouvelles sur l'onglet PR, nourrie par une lecture de fond de la liste du projet à l'écran"
date: "2026-11-09"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-403"
pages: ["git-fichiers/vue-pull-requests","fenetre/barre-laterale"]
tickets: ["ESC-403"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/vue-pull-requests","added":14,"removed":3},{"page":"fenetre/barre-laterale","added":1,"removed":0}]
accepted_at: "2026-11-09T16:38:10+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur veut une pastille sur l'onglet PR dès qu'une nouvelle PR arrive (ESC-403, 2026-11-09). La liste des PR d'un projet n'était lue chez l'hébergeur que quand quelque chose la montrait (la vue, l'onglet « PR et CI », la vue d'ensemble, les outils) : onglet fermé, la pastille n'aurait pas bougé. Le ticket demandait « aucune lecture de plus » ; l'overseer a tranché pour une lecture de fond encadrée.

## Décision

L'onglet PR porte le nombre de PR nouvelles : ouvertes, de numéro plus haut que ce qui a été vu (première lecture, ouverture de l'onglet, vue affichée), pas ouvertes par Escouade (ses agents, ses tickets). La première lecture d'un projet ne compte rien ; ouvrir l'onglet remet à zéro et marque les nouvelles « Nouvelle » jusqu'à la prochaine ouverture ; ce qui a été vu est gardé par projet dans l'état de l'interface. Toute lecture de la liste la nourrit. En plus, la liste du projet à l'écran est relue en fond, fenêtre visible, toutes les 2 minutes, jamais moins de 2 minutes après une autre lecture, pas près de la limite d'appels ; après un échec l'attente double jusqu'à 30 minutes, sans rien dire.

## Conséquences

Une requête de liste au plus toutes les 2 minutes par fenêtre visible, pour le seul projet à l'écran (aucune quand la vue PR est ouverte : elle lit déjà chaque minute). La requête conditionnelle (ETag) n'est pas faite : elle relève des plugins d'hébergeur, et la liste de GitHub est une requête GraphQL qui ne se sert pas en conditionnel ; une version des plugins REST (GitLab, Gitea, Bitbucket, Azure DevOps) pourrait l'ajouter. Les PR d'un autre projet ne se comptent qu'à la prochaine lecture de sa liste.
