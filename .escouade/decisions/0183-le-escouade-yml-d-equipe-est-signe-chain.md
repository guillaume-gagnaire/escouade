---
number: 183
id: "le-escouade-yml-d-equipe-est-signe-chain"
title: "Le .escouade.yml d'équipe est signé, chaîné, au quorum pour le sensible, et ne vaut que pour les projets de l'équipe"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-471"
pages: ["serveur-equipe/fichier-equipe","fenetre/reglages-imposes","serveur-equipe/client"]
tickets: ["ESC-471"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/fichier-equipe","added":72,"removed":0},{"page":"serveur-equipe/client","added":3,"removed":0},{"page":"fenetre/reglages-imposes","added":12,"removed":9}]
accepted_at: "2026-11-15T20:23:39+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-13 (docs/TEAM-SERVER.md §6.5, §6.6, C12-B) : une configuration centrale piratée qui pousse un hook ou un proxy sur toute la flotte est le premier défaut à fermer. Il faut un format, une signature que le serveur ne peut forger, une règle de quorum, des sortes de clés tenues par le cœur, et dire où le rang 5 s'applique alors que les espaces (TS-4) n'existent pas encore.

## Décision

- Un fichier par équipe (defaults, enforced, proposed, reasons, quorum, signers), dans un flux du serveur à part, chaque version scellée sous la clé d'équipe et signée par la feuille MLS d'un administrateur, chaînée à la précédente par son empreinte ; les cosignatures nomment l'empreinte.
- Refusée : non signée ou forgée, signée par qui n'était pas administrateur à l'époque où elle a été scellée ou ne l'est plus aujourd'hui (un administrateur retiré depuis), hors de signers, rejouée, qui recule, qui ne suit pas la chaîne. La chaîne choisie n'est jamais défaite ; les administrateurs restants cosignent d'eux-mêmes les versions dont le signataire est parti.
- Quorum C12-B : deux administrateurs distincts (deux appareils d'un même membre comptent pour un) pour une clé sensible, la liste des signataires ou le quorum ; en option par équipe, allumé d'office dès deux administrateurs. Un seul administrateur : une signature suffit, et l'écran le dit.
- Les sortes sont celles de la liste unique et du .escouade.yml du dépôt : restreindre et valeur appliqués ; sensible en attente de l'accord de chaque poste à l'empreinte de la valeur ; exécutable jamais imposé, seulement proposé et accepté à son empreinte, appliqué comme rang 2.
- Une liste en dur de ce qui n'entre jamais (bypassPermissions, commande sans accord, plugin sans geste, dev.escouade.* signé par l'équipe, projet rendu public) refuse la version entière à la lecture.
- Le rang 5 et le rang 2 valent pour les projets de l'équipe (un champ team du projet, coché dans l'écran de l'équipe) ; les clés de l'app sans projet (réseau, télémétrie, voix, mises à jour, plugins, MCP extérieurs) valent pour l'app tant que le poste est dans l'équipe, jusqu'aux espaces.
- Pas encore de fichier par projet d'équipe par-dessus : il attend les groupes des projets gardés (TS-5) ; l'écran vit dans la fenêtre des équipes en attendant Réglages › Équipe (TS-4).

## Conséquences

Un serveur piraté ne peut rien pousser ; un administrateur pris seul ne pousse rien de sensible dès qu'il y a deux administrateurs ; rien ne s'exécute sans l'accord de chaque poste. Le verrou a désormais des verrous par équipe (locks_for), et une clé sensible venue de l'équipe s'applique une fois acceptée. Un serveur qui cache une version à un appareil qui ne l'a jamais vue ne se voit pas de cet appareil seul (à croiser avec les balises de tête plus tard).
