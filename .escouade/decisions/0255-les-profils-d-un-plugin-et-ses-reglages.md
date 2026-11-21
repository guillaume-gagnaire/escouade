---
number: 255
id: "les-profils-d-un-plugin-et-ses-reglages"
title: "Les profils d'un plugin et ses réglages d'un projet sont servis ; la page Plugins d'un projet écrit plugins.<id>.<clé> clé par clé"
date: "2026-11-19"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-530"
pages: ["plugins/profils-et-reglages-d-un-projet","plugins/contrat-du-manifeste","projets/escouade-yml","fenetre/reglages"]
tickets: ["ESC-530"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/profils-et-reglages-d-un-projet","added":62,"removed":0},{"page":"plugins/contrat-du-manifeste","added":11,"removed":10},{"page":"projets/escouade-yml","added":10,"removed":6},{"page":"fenetre/reglages","added":14,"removed":5}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Issues #38 et #39 d'un auteur de plugin tiers (un VPN par projet) : un plugin ne pouvait garder ni des identifiants ni des réglages propres à un projet, et rien dans la fenêtre ne réglait plugins.<id> d'un projet (le fichier seul, à la main). L'étude ESC-501 a proposé deux directions ; l'utilisateur a tranché le 2026-11-17 : n° 1 B, les profils, avec une portée « project » pour les réglages simples ; n° 2 un réglage simple peut venir de .escouade.yml sauf "repo": false, jamais un secret, un profil ni une valeur importée ; n° 3 une page Plugins dans Réglages › Projets › projet ; n° 4 « Écrire dans .escouade.yml » clé par clé. La spec disait des clés plugins.* qu'elles n'étaient « jamais marquées dans la fenêtre, jamais écrites par l'export » : la règle change pour l'écriture d'une clé.

## Décision

- Un plugin déclare des profils (contributes.profiles) : l'utilisateur les crée, modifie et supprime sur la page du plugin ; chaque projet choisit le sien (un réglage type "profile" de portée project), un réglage profile de portée app sert de profil par défaut. Supprimer un profil efface ses secrets et met les projets qui le prenaient à « Aucun profil » (null).
- Un réglage simple de portée project a, par rang : la valeur de cette machine pour le projet, puis celle de .escouade.yml (sauf "repo": false ou import), puis le défaut du manifeste ; seulement pour un projet où le plugin est actif. Les rangs de l'équipe (2, 5) et du poste géré (6) viennent avec la 2.3.
- Le protocole reste en version 1, par ajouts : capabilities.profiles, profiles/list, profiles/get, secrets/get { key, profile }, settings/get { project } qui rend l'id du profil. Une clé qui n'est qu'un champ secret d'un profil, demandée sans profile, est refusée (-32602).
- Sécurité (§2.3) : nom au trousseau plugin:<id>:profile:<genre>:<profil>:<clé> avec un identifiant de profil tiré par Escouade ; le plugin seul lit ses secrets (-32001 sans profile:<genre>) ; .escouade.yml ne porte aucun secret et ne choisit jamais un profil, il en propose un par son nom (une bande, l'utilisateur choisit) ; le journal nomme le profil, jamais la valeur ; Escouade tient la liste des noms gardés au trousseau par plugin et les efface à la désinstallation.
- Réglages › Projets › projet gagne une page Plugins, présente seulement si un plugin installé et actif dans ce projet déclare un réglage de portée project, avec les marques des autres réglages d'un projet. Son « Écrire dans .escouade.yml » écrit plugins.<id>.<clé>, cette clé seule, en gardant les commentaires ; jamais un secret, un profil, une valeur importée ni un réglage "repo": false. L'export de tout le projet continue de ne pas écrire plugins.*.
- Un secret de portée project (direction A, non retenue) reste lu et coupe le plugin.

## Conséquences

Un plugin d'aujourd'hui marche pareil. Un plugin à profils doit dire escouade.min (2.2.5) : une app plus ancienne le coupe (ESC-525) ou ignorerait profile. Le kit (0.1.3) et le protocole (0.1.4), non publiés depuis la 2.2.4, portent les ajouts ; le SDK JavaScript, le banc de conformité et la documentation du site restent à ESC-531 (T9) ; un réglage de plugin recommandé ou imposé par une équipe à ESC-532 (T8, 2.3). Le store doit être redéployé pour lire les nouveaux manifestes (geste du chef).
