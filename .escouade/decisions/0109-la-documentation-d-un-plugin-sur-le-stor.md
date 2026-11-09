---
number: 109
id: "la-documentation-d-un-plugin-sur-le-stor"
title: "La documentation d'un plugin sur le store a ses pages par langue, docs/fr et docs/en"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-366"
pages: ["store/publier-et-securite"]
tickets: ["ESC-366"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/publier-et-securite","added":8,"removed":0}]
accepted_at: "2026-11-07T11:25:13+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le README d'un plugin existe en README.fr.md et README.en.md et le store montre celui du visiteur. Les pages de store.docs, elles, n'avaient pas de langue : toutes sortaient, dans l'ordre de leur nom. Une documentation écrite dans les deux langues aurait montré chaque onglet en double. Chaque plugin doit avoir sa documentation sur le store (demande du 2026-11-07).

## Décision

Sous le dossier de store.docs, les pages de fr/ et en/ ont leur langue : la page du plugin montre celles du visiteur, sinon celles de l'autre. Un dossier sans fr/ ni en/ montre toutes ses pages, comme avant ; avec l'un des deux, le reste du dossier n'est pas lu. Les pages suivent l'ordre de leurs noms, 20 au plus par langue. Le titre d'un onglet est le premier titre de la page, sinon le nom du fichier sans son préfixe d'ordre. Un lien vers une autre page ou vers le README reste sur le store (?tab=docs&page=…) au lieu de mener à GitHub. Un store.docs sans page markdown est un avertissement du contrôle README, pas un refus.

## Conséquences

Les plugins (première main d'abord) peuvent écrire leur documentation en deux langues sans doublon. Les versions déjà rendues gardent leurs pages sans langue, montrées comme avant. Un contrôle qui passe peut maintenant porter un avertissement, montré à la soumission. L'app ne montre toujours pas la documentation : elle renvoie à la page du store, et la montrer dans l'app reste un chantier à part.
