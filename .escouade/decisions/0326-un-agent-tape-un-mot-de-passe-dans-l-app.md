---
number: 326
id: "un-agent-tape-un-mot-de-passe-dans-l-app"
title: "Un agent tape un mot de passe dans l'app développée avec l'accord de l'utilisateur, une fois par navigateur"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-663"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-663"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":2,"removed":2}]
accepted_at: "2026-11-26T00:48:42+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-663 : un agent qui pilote un navigateur peut avoir à se connecter à l'app en développement, avec un compte de test. Taper dans un champ mot de passe touche à un secret. La question a été posée à l'utilisateur (a-38) : jamais, avec son accord, ou par un réglage du projet.

## Décision

Le choix B de l'utilisateur (2026-11-25) : avec son accord, une fois par navigateur.

La première frappe d'un agent dans un champ `type=password` ne tape rien. Une bande native demande « <agent> veut remplir un mot de passe · Permettre / Refuser ». L'accord vaut pour ce navigateur jusqu'à sa fermeture. Pendant l'attente, `drive_browser` répond qu'il attend l'accord.

Ce qui ne change pas : la valeur n'est jamais relue (`read_browser` rend `«masqué»`, l'arbre d'accessibilité ne la donne pas), et un texte qui contient un secret connu d'Escouade reste refusé.

## Conséquences

La connexion avec un compte de test se fait sans que l'utilisateur tape lui-même, et sans réglage oublié allumé. AB-7 (le refus et l'attente dans `drive_browser`) et AB-8 (la bande « Permettre / Refuser ») partent sur ce choix. L'e2e AB-12 vérifie le refus sans accord. Hors du lancement du projet, taper un mot de passe reste interdit, même avec l'accord.
