---
number: 367
id: "une-beta-s-annonce-comme-beta-partout-di"
title: "Une bêta s'annonce comme bêta partout, Discord compris, et le serveur d'équipe se publie avec chaque version"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-586"
pages: ["introduction/livraison","serveur-equipe/serveur"]
tickets: ["ESC-586"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"introduction/livraison","added":12,"removed":2},{"page":"serveur-equipe/serveur","added":7,"removed":2}]
accepted_at: "2026-11-21T22:15:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur veut essayer en conditions réelles le serveur d'équipe et le reste de la 2.3 par une bêta (2026-11-21). Jusqu'ici une pré-version n'était pas annoncée sur Discord, le Release ne la distinguait que par sa case « pré-version », la fenêtre « Nouveautés » ne disait rien de plus que « (bêta) » après le numéro, et le serveur d'équipe n'était publié nulle part. Un essayeur doit aussi pouvoir revenir à la 2.2 sans rien perdre.

## Décision

1. Une bêta n'est jamais annoncée comme la version stable : le titre de son Release porte « (beta) », ses notes commencent par un encadré (version d'essai, qui la reçoit, comment revenir à la version stable d'avant), le job `announce` la poste désormais sur les deux salons Discord avec « (bêta) »/« (beta) » et le même encadré, et les modales « est prête » et « Nouveautés » d'une bêta montrent cet encadré avec un bouton vers la page des versions.
2. `escouade-team-server` est publié par le même job et les mêmes scripts que `escouade-server` : archives Linux x64 et arm64 signées avec leur SHA-256, image ghcr.io étiquetée de la version, `latest` pour une version stable seulement, `beta` pour une bêta ; un essai en conteneur Debian nu avant tout envoi ; sa version est celle de l'app.
3. Le retour à la 2.2 se vérifie par `scripts/compat-2.2.sh` : ce que la 2.3 écrit, relu par le code de `main`. Ce qui casse se corrige sur `main` (correctif 2.2.x), jamais dans `release/2.3`.

## Conséquences

Chaque bêta fait un message par salon Discord (la 2.2 en avait fait 15 sans les annoncer) : si c'est trop, `announce` peut se limiter à la première bêta d'une ligne. Le job « Servers » du Release construit deux programmes (quelques minutes de plus par architecture) et le job des images tourne une fois par programme. Le premier essai de bout en bout n'aura lieu qu'à l'étiquette `v2.3.0-beta.1`. Avant elle : publier le correctif 2.2.x de `NoteChoice` (sinon revenir à la 2.2.5 remet l'app à zéro pour qui a publié une remarque seule) et reverser `main` dans `release/2.3` (sinon les réglages de fetch d'ESC-561 et les marques de la boîte d'ESC-557 se perdent à l'aller-retour).
