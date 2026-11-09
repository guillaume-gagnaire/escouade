---
number: null
id: "les-commandes-de-lancement-prennent-le-c"
title: "Les commandes de lancement prennent le clavier tant qu'elles tournent"
date: "2026-11-09"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-396"
pages: ["fenetre/barre-laterale","editeur/ouvrir-un-fichier"]
tickets: ["ESC-396"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/barre-laterale","added":4,"removed":2},{"page":"editeur/ouvrir-un-fichier","added":1,"removed":1}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Retour de l'utilisateur : « Les commandes de lancement ne sont pas interactives, corrige. » Leur vue était un journal en lecture seule : impossible de répondre à une invite (« Ok to proceed? »), d'utiliser les touches d'un serveur de dev ou d'arrêter par Ctrl+C. En plus, sous Windows, un Escouade lancé avec Ctrl+C ignoré (nouveau groupe de processus) transmettait ce réglage aux programmes de ses terminaux, que Ctrl+C n'arrêtait alors jamais.

## Décision

Tant qu'une commande de lancement ou une étape de « ▶ Tester » tourne, sa vue est un terminal interactif : frappe, collage, touches spéciales et Ctrl+C (sans sélection) vont au programme, avec les mêmes raccourcis d'app qu'un terminal de la barre latérale ; seul le volet actif de l'écran partagé reçoit le clavier. Seule la saisie de l'utilisateur part (Escouade continue de répondre lui-même à la question de position du curseur). Terminée, la vue redevient un journal en lecture seule ; la relance lui rend le clavier. Une fin dans les 10 s d'un Ctrl+C tapé dans la vue compte comme « arrêté », sans notification de plantage. Sous Windows, Escouade lève le drapeau « Ctrl+C ignoré » avant son premier terminal.

## Conséquences

On peut répondre aux invites et piloter un serveur de dev dans sa vue. Un journal fini reste en lecture seule (Ctrl+P y ouvre la palette ; en cours, il va au programme). Ctrl+C marche aussi dans les terminaux interactifs quand Escouade a été lancé par un lanceur qui ignore Ctrl+C.
