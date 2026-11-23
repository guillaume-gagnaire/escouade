---
number: 293
id: "le-fetch-automatique-se-regle-se-voit-et"
title: "Le fetch automatique se règle, se voit et aboutit sur un gros dépôt"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-561"
pages: ["git-fichiers/fetch-automatique","git-fichiers/git-fichiers","fenetre/reglages","projets/escouade-yml"]
tickets: ["ESC-561"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/fetch-automatique","added":47,"removed":0},{"page":"git-fichiers/git-fichiers","added":3,"removed":2},{"page":"fenetre/reglages","added":5,"removed":0},{"page":"projets/escouade-yml","added":9,"removed":0}]
accepted_at: "2026-11-23T20:47:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le fetch de fond existait (15 s après le démarrage puis toutes les 5 min, délai de 60 s, sans identifiants), sans réglage ni rien de visible : ses échecs n'allaient qu'au journal. Chez l'utilisateur, il échouait sur le dépôt d'Escouade (« toujours pas terminé après 60 s », 15 fois de suite le 2026-11-13), et un dépôt qui demandait des identifiants était retenté toutes les 5 min (546 fois en trois jours). L'utilisateur a demandé le 2026-11-20 de pouvoir l'activer et d'en régler le délai. Mesure du 2026-11-20 sur un clone en retard de 8 jours (9 700 objets, 120 Mo) : avec le ssh de Git for Windows 2.47.0.windows.1 (OpenSSH 9.9p1), le transfert cale après quelques Ko jusqu'à ce que GitHub coupe à 600 s (git-for-windows/git#5199, corrigé dans 2.47.0(2)) ; en HTTPS 6,6 s, avec le ssh de Windows 5 à 10 s. Les étiquettes et les autres branches ne coûtent presque rien.

## Décision

1. Réglages › Tous les projets › Git : « Récupérer automatiquement (git fetch) » (allumé), « Intervalle » (valeur libre de 1 min à 24 h, valeurs proposées 2, 5, 15, 30 min, 1 h ; 5 min) et « Délai d'attente » du fetch de fond (de 30 s à 30 min ; 5 min au lieu de 60 s). Le délai d'un geste ne change pas (3 min).
2. Par projet (Git et worktrees) : « Comme le réglage général », « Toujours », « Jamais », avec un intervalle et un délai propres ; partagés dans .escouade.yml (git.auto_fetch, git.fetch_every, git.fetch_timeout), et dans le registre des réglages pour les rangs d'équipe et de poste géré à venir. Un dépôt de plusieurs projets suit le plus pressé et le plus patient.
3. Hors bornes, une valeur est ramenée à la borne, et la fenêtre (ou le fichier) le dit. Les réglages sont relus à chaque passage : un changement vaut sans redémarrer.
4. Le sélecteur de branche dit « Récupéré il y a N min » (l'heure au survol) et, discrètement, la raison du dernier échec de fond (identifiants, délai dépassé avec « Régler », réseau ou remote injoignable, refus de git).
5. Un dépôt qui demande des identifiants le dit une fois et ne réessaie plus en fond jusqu'au prochain fetch, pull ou push réussi de l'utilisateur.
6. Le fetch de fond passe par la configuration de git de l'utilisateur telle qu'elle est, sans le proxy d'Escouade et sans rien y écrire. Seule exception, choisie par l'utilisateur le 2026-11-20 (« Contourner, et guider ») : avec Git for Windows 2.47.0.windows.1, un remote ssh et aucun ssh choisi par l'utilisateur, il passe par le ssh de Windows (BatchMode), pour ce seul processus, et le panneau dit de mettre Git à jour.
7. Toujours toutes les branches et les étiquettes : la mesure ne les met pas en cause.

## Conséquences

Le fetch de fond du dépôt d'Escouade, en retard de 8 jours, aboutit en 12,3 s (au lieu de ne jamais aboutir). Un fetch de fond peut tenir jusqu'à 5 min par défaut (un dépôt à la fois, sauté pendant un geste de l'utilisateur : rien n'attend après lui). L'état du fetch de fond n'est gardé que pendant que l'app tourne. Le geste « Fetch » à la main garde le ssh de Git et cale toujours avec 2.47.0(1) : seule la mise à jour de Git le règle. Aucune commande nouvelle : l'état passe par l'état git du projet, les réglages par ceux de l'app et du projet.
