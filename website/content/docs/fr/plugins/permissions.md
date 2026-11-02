# Les permissions

Ce qu’un plugin demande dans son manifeste, ce que chaque demande lui ouvre, et ce qu’Escouade applique ou ne fait que montrer. La fenêtre d’installation les dit en mots, la plus forte d’abord.

## Chaque permission

| Dans le manifeste | Ce qu’elle ouvre | Ce qu’Escouade applique |
| --- | --- | --- |
| `run.command` | Un programme lancé dans un process, avec tes droits : il peut lire tes fichiers, lancer d’autres programmes, se connecter où il veut. | Rien au-delà de ses autres permissions : Escouade le lance, l’arrête avec tout ce qu’il a lancé, borne sa mémoire à 1 Go, mais ne l’enferme pas. |
| `run.js` | Un programme en JS ou en TypeScript, dans l’app, en WASM. | Tout : 64 Mo de mémoire, des délais, aucun programme lancé, aucun fichier lu hors de son dossier, le réseau par Escouade seulement. |
| `permissions.decide` | Autoriser ou refuser ce que tes agents demandent, avant toi. | Une réponse en 500 ms, sinon la demande vient à toi ; un refus l’emporte ; chaque décision est au journal. Jamais donnée par un projet. |
| `permissions.commands` | Des lignes de commande lancées sur un fait, dans ton shell. | Rien ne tourne avant ton accord sur ces commandes telles qu’elles sont ; une commande changée le redemande. |
| `permissions.events` | Les faits qu’il reçoit : un ticket qui change de colonne, un agent qui attend… | Un fait hors de la liste ne lui est jamais envoyé. |
| `permissions.network` | Les hôtes où il se connecte. | Appliquée à un programme `run.js` et aux webhooks : rien d’autre ne part. Seulement montrée pour un programme `run.command`, dont Escouade ne filtre pas le réseau. |
| `permissions.secrets` | Les secrets qu’il garde au trousseau du système, sous un nom à lui. | Il ne reçoit que ceux de sa liste, jamais ceux d’un autre plugin, d’un compte ou d’un harness ; ils sont masqués dans son journal. |

Les points d’extension ajoutent leurs propres lignes à la fenêtre : un harness y dit le programme que ses agents lanceront, un thème où le trouver.

## Ce que tu gardes en main

- **Retirer une permission** : l’onglet Permissions de sa page, un interrupteur par permission. Son programme est relancé sans elle (voir [installer un plugin](plugins/installer#retirer-une-permission)).
- **Le couper pour un projet** : l’onglet Projets. Un projet privé est coupé d’office pour tout plugin que tu as installé.
- **Une mise à jour qui demande plus** redemande ton accord, et la fenêtre marque « nouveau » chaque permission en plus.

> [!WARNING]
> Un programme `run.command` a tes droits. Même coupé pour un projet privé, il pourrait lire ses fichiers sur le disque : Escouade ne lui envoie rien de ce projet, mais ne peut pas l’en empêcher. N’installe un tel programme que d’un auteur en qui tu as confiance, ou une version « Vérifié par Escouade ».

## Pour l’auteur

Demande le moins possible : chaque permission en plus est une ligne de plus dans la fenêtre, et une raison de plus pour la revue de la refuser ([la sécurité des plugins](plugins/security)). Un plugin qui n’a besoin que du réseau et de ses fichiers s’écrit en `run.js` plutôt qu’en `run.command`.
