---
number: 287
id: "les-docs-venues-de-l-equipe-jugees-avant"
title: "Les docs venues de l'équipe jugées avant le disque, refusées avec une alarme"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-644"
pages: ["serveur-equipe/docs-equipe","docs/decisions"]
tickets: ["ESC-644","ESC-626","ESC-643"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/docs-equipe","added":9,"removed":0},{"page":"docs/decisions","added":1,"removed":0}]
accepted_at: "2026-11-22T16:27:06+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (SR3-01, critique) a montré que le passage des docs d'un projet d'équipe prenait tels quels le chemin d'une page et l'id d'une décision lus au journal, et en faisait des noms de fichiers : un simple membre pouvait faire écrire chez un autre, sans geste de sa part, un fichier .md hors du dossier des docs (par exemple un skill que son harness charge dans toutes ses sessions). La résolution d'un conflit reprenait le même chemin. Et une seule page qui ne s'écrivait pas arrêtait tout le passage.

## Décision

Chaque poste juge ce qu'il lit du journal avant tout accès au disque, du même jugement que ce qu'il écrit lui-même : une page n'est prise que si son chemin est celui que docs::page_path garde ; une décision que si son id a la forme d'un id (un seul nom de fichier sûr, écrite dans la spec) ; la résolution d'un conflit n'écrit qu'à un chemin de page. Les fichiers de la spec et des décisions ne se lisent, s'écrivent et se retirent plus qu'à un chemin jugé avant (le type DocFile). Ce qui est refusé est écarté, la lecture des autres continue, et un bandeau rouge le dit en tête de l'onglet Docs aux administrateurs et à l'appareil qui l'a écrit (un autre membre ne le voit pas, le journal du poste le note toujours). Ce poste ne publie pas une page ou une décision que les autres écarteraient. Une page ou une décision qui ne s'écrit pas n'arrête plus le passage.

## Conséquences

Aucun fichier venu de l'équipe ne s'écrit hors de team-docs/.escouade/. Un fichier de spec nommé à la main hors des formes d'une page reste sur son poste au lieu de partir. L'alarme est locale à chaque poste qui lit (pas de message au journal) : un administrateur la voit parce que son propre poste refuse la même version. Le durcissement de Space::file lui-même vient d'ESC-643 (2.2.6), à reprendre par cherry-pick quand il est sur main.
