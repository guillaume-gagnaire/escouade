---
number: null
id: "la-boite-a-toi-se-filtre-par-type-de-dem"
title: "La boîte « À toi » se filtre par type de demande, sur ce poste, sans changer ses comptes"
date: "2026-11-21"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-578"
pages: ["harness/boite-a-toi"]
tickets: ["ESC-578"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/boite-a-toi","added":16,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La boîte mêle les demandes des agents (choix, oui ou non, feu vert, réponse, tâche), les vues qu'Escouade y verse (autorisations, questions, plans, tickets à valider ou bloqués, décisions, docs, relectures) et les agents arrêtés. Avec beaucoup de demandes, l'utilisateur veut n'en voir qu'une sorte à la fois (demande du 2026-11-21, ESC-578).

## Décision

Un filtre par type en haut de la file : « Tout », puis chaque type qui attend avec son nombre, plusieurs à la fois, gardé sur ce poste (stockage local), effacé d'un geste. Dix types, regroupés là où l'utilisateur ne les distingue pas : Autorisations (permission + suppression demandée), Questions (question bloquante + `answer` + `confirm` sans `gate`), Tickets (à valider + bloqué) ; Feux verts (`gate`) reste à part. Les groupes et l'ordre restent ; la tuile, l'en-tête et la jauge comptent toujours tout ; ce qui est caché se dit d'une ligne discrète (région status). La logique vit dans `src/lib/inbox/filter.ts` pour l'app mobile ; rien ne passe par le cœur.

## Conséquences

Un choix propre à chaque poste : le téléphone (MO-5, ESC-571) garde le sien avec le même module. Aucune commande ni API distante nouvelle. Un nouveau type de vue devra recevoir sa place dans `INBOX_TYPES` (le typage le force).
