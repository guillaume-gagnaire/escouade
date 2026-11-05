# Les permissions

Ce qu’un plugin demande dans son manifeste, ce que chaque demande lui ouvre, et ce qu’Escouade applique ou ne fait que montrer. La fenêtre d’installation les dit en mots, la plus forte d’abord.

## Chaque permission

| Dans le manifeste | Ce qu’elle ouvre | Ce qu’Escouade applique |
| --- | --- | --- |
| `run.js`, `run.wasm` | Un programme en JS, en TypeScript ou en Rust, dans l’app, en WASM. | Tout : 64 Mo de mémoire, des délais, et rien hors de ce qu’il déclare ici : ni réseau, ni fichier, ni programme. |
| `run.command` | Un programme lancé dans un process, avec tes droits : il peut lire tes fichiers, lancer d’autres programmes, se connecter où il veut. Le store n’en publie aucun : il ne vient que d’un dossier ou d’un dépôt git. | Rien au-delà de ses autres permissions : Escouade le lance, l’arrête avec tout ce qu’il a lancé, borne sa mémoire à 1 Go, mais ne l’enferme pas. |
| `permissions.decide` | Autoriser ou refuser ce que tes agents demandent, avant toi. | Une réponse en 500 ms, sinon la demande vient à toi ; un refus l’emporte ; chaque décision est au journal. Jamais donnée par un projet. |
| `permissions.commands` | Les programmes de ton ordinateur qu’il peut lancer, ligne par ligne. | Rien d’autre ne se lance, sans shell, et le lancement va au journal. Mais le programme lancé a tes droits. |
| `permissions.files` | Les dossiers qu’il lit ou écrit : le sien, un dossier à lui, ceux de tes agents. | Une liste fermée de racines, résolues par Escouade, liens suivis, jamais un projet privé. Chaque écriture va au journal. |
| `limits` | Plus de temps ou d’octets pour un appel. | Plafonnées par Escouade : 120 s, 32 Mo, 4 Mo pour un fichier. |
| `permissions.events` | Les faits qu’il reçoit : un ticket qui change de colonne, un agent qui attend… | Un fait hors de la liste ne lui est jamais envoyé. |
| `permissions.network` | Les hôtes où il se connecte, les instances que tu lui donnes (`{account}`), le serveur local d’un harness (`local:shared`). | Appliquée à un programme en WASM et aux webhooks : rien d’autre ne part. Seulement montrée pour un programme `run.command`, dont Escouade ne filtre pas le réseau. |
| `permissions.secrets` | Les secrets qu’il garde au trousseau du système, sous un nom à lui. | Il ne reçoit que ceux de sa liste, jamais ceux d’un autre plugin, d’un compte ou d’un harness ; ils sont masqués dans son journal. |

Pour chaque capacité, ce qu’elle permet, ce qu’Escouade refuse et ce que dit la fenêtre : [les capacités de l’hôte](plugins/capabilities).

Les points d’extension ajoutent leurs propres lignes à la fenêtre : un harness y dit le programme que ses agents lanceront, un thème où le trouver.

## Ce que tu gardes en main

- **Retirer une permission** : l’onglet Permissions de sa page, un interrupteur par permission. Son programme est relancé sans elle (voir [installer un plugin](plugins/installer#retirer-une-permission)).
- **Le couper pour un projet** : l’onglet Projets. Un projet privé est coupé d’office pour tout plugin que tu as installé.
- **Une mise à jour qui demande plus** redemande ton accord, et la fenêtre marque « nouveau » chaque permission en plus.

> [!WARNING]
> Un programme `run.command` a tes droits. Même coupé pour un projet privé, il pourrait lire ses fichiers sur le disque : Escouade ne lui envoie rien de ce projet, mais ne peut pas l’en empêcher. N’installe un tel programme que d’un auteur en qui tu as confiance, ou une version « Vérifié par Escouade ».

## Pour l’auteur

Demande le moins possible : chaque permission en plus est une ligne de plus dans la fenêtre, et une raison de plus pour la revue de la refuser ([la sécurité des plugins](plugins/security)). Un plugin s’écrit en `run.js` ou en `run.wasm` : le store n’accepte rien d’autre.
