---
number: 6
title: "Les écarts et la date de comparaison au code sont gardés hors des pages de spec"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-301"
pages: ["docs/spec-a-jour","docs/specification"]
tickets: ["ESC-301"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"docs/spec-a-jour","added":5,"removed":1},{"page":"docs/specification","added":3,"removed":2}]
accepted_at: "2026-10-30T03:19:57+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La comparaison de la spec au code (ESC-263) écrivait `compared` et parfois `gaps` dans l’en-tête des pages, suivies par git. Comme Escouade ne commite jamais seule les fichiers partagés de `.escouade/` (ESC-190), chaque validation laissait main sale et la suivante était refusée (« Le dossier du projet a des modifications non commitées sur main ») ; le chef d’escouade a dû commiter à la main (e31f8d40, b9fe4dcf). Deux pistes : ignorer au contrôle les modifications d’Escouade sous `.escouade/`, ou garder ces champs hors des fichiers suivis.

## Décision

Ce que trouve la comparaison (date et écarts) est gardé dans le dossier de données d’Escouade, par projet et par page (`docs-checks.json`), jamais dans la page. Il se lit par-dessus l’en-tête de la page partout où Escouade lit la spec (onglet Docs, `read_spec_page`, `get_spec_tree`, discussions). Écrire une page garde son en-tête tel qu’il était ; une discussion versée qui règle un écart le retire de ce qui est gardé. Le contrôle d’un main non commité ne change pas : toute modification de l’utilisateur, page de spec comprise, bloque la validation.

## Conséquences

Une comparaison ne laisse plus rien à commiter, et aucun commit n’est fait à la place de l’utilisateur. Le contrôle reste simple et sûr : pas besoin de distinguer qui a modifié un fichier de `.escouade/spec/`, ni de fusionner un en-tête changé des deux côtés. En contrepartie, les écarts ne se partagent plus par git : chaque poste voit ceux de ses comparaisons. Les `compared`/`gaps` déjà commités dans des en-têtes se lisent tant que la page n’est pas comparée de nouveau.
