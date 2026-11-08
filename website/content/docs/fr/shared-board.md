# Partager le Kanban avec l’équipe

Un ticket partagé est un fichier du dépôt, que tu commites avec ton travail : tes coéquipiers le retrouvent dans leur Kanban à leur pull, avec ton nom, et son statut suit le travail de chacun.

## Partager un ticket

Clic droit sur la carte, « Partager avec l’équipe », ou l’interrupteur du même nom dans le formulaire du ticket. Escouade écrit alors un fichier `.escouade/tickets/<id>.yml` dans le dossier du projet, et une petite marque apparaît à côté de la clé du ticket ; son infobulle donne le fichier.

- Escouade réécrit le fichier à chaque changement qui compte : un agent prend le ticket, il passe « À tester », il est validé, renvoyé ou remis « À faire », son titre, sa description ou ses critères changent. Jamais à chaque boucle.
- **Escouade ne commite ni ne pousse jamais ce fichier.** Il apparaît dans les fichiers non commités du [panneau Git](git), sur « Tout le projet », dans un groupe « Tickets partagés » : chaque fichier y est dit par la clé et le titre de son ticket. Commite-le avec ton travail ; l’équipe le reçoit à son pull.
- Ces fichiers ne bloquent ni un merge ni la validation d’un ticket : un dossier dont seuls les fichiers de tickets ont changé compte comme propre.
- Les images du ticket sont partagées dans un dossier à côté du fichier, `.escouade/tickets/<id>/` ; les fichiers joints aussi, jusqu’à 5 Mo. Au-delà, un fichier joint reste chez toi, et le formulaire le dit.
- « Arrêter le partage » supprime le fichier (la suppression est à commiter) ; le ticket reste chez toi tel quel. Supprimer un ticket partagé supprime aussi son fichier.

Dans Réglages › Projets › Équipe, le groupe « Kanban partagé » dit combien de tickets le projet partage, et « Partager chaque nouveau ticket » partage d’office ceux que tu crées, à la main, par un import ou par MCP.

## Ton nom

Chaque fichier porte le nom de son auteur, et celui de qui a pris le ticket. Escouade te le demande au premier lancement, prérempli par `git config user.name` ; « Plus tard » reporte la question à ton premier partage.

- Tu le changes dans Réglages › Général › Toi, « Ton nom ».
- Un projet peut avoir le sien : Réglages › Projets › Équipe › Kanban partagé, « Ton nom dans ce projet ». Vide, c’est celui de l’application.
- Ton nom reste sur ton poste : il n’est jamais écrit dans [.escouade.yml](escouade-yml).
- À côté du nom, un identifiant tiré de ton e-mail git (l’e-mail lui-même n’est écrit nulle part) distingue tes tickets de ceux d’un homonyme. C’est lui qui dit si un ticket est à toi ; il est le même sur tes autres postes.

## Le ticket d’un autre

Après ton pull, un ticket qu’Alice a partagé apparaît dans ton Kanban, dans la colonne de son statut, avec son nom et une pastille à son initiale sur la carte.

- Dans « À faire », la carte dit « Ajouté par Alice · le pilote auto ne le lance pas » : ton [pilote auto](autopilot) ne le prend jamais, même en tête de colonne. Le bouton « Lancer » le lance à la main, pilote auto allumé ou non.
- Lancé chez toi, il se comporte comme un des tiens (worktree, boucles, validation), et son fichier dit « pris par » toi. Il garde son auteur.
- En cours chez quelqu’un d’autre, la carte dit « Pris par Alice » ; à tester, « À tester chez Alice » ; fini, « Fait par Alice ». Ces tickets ne prennent aucune place de ton Kanban et ne sont jamais repris par ton Escouade : seul celui qui l’a pris le valide.
- Son menu propose aussi « Modifier » (ce que tu changes va dans son fichier, pour toute l’équipe), « Masquer chez moi » (le fichier reste ; un bandeau du Kanban dit combien de tickets sont masqués et les affiche à la demande) et « Supprimer pour l’équipe… » (supprime son fichier, à commiter).

Escouade relit le dossier `.escouade/tickets/` dès qu’il change : après un pull, un changement de branche, une modification à la main. Un ticket qu’Alice a fini passe dans « Terminé » chez toi après ton pull.

## Le format du fichier

```yaml
# A ticket of the Kanban shared by Escouade: https://escouade.dev/en/docs/shared-board
# Escouade writes it again at each status; commit it with your work, Escouade never does.
# yaml-language-server: $schema=https://escouade.dev/schemas/escouade-ticket.schema.json
version: 1
id: q4w7c2na
key: DEM-7
title: Journal d’audit des connexions
status: in-progress
author: { name: Alice Martin, id: a91f03c2 }
taken_by: { name: Bob Durand, id: 5d0e77b1 }
updated_by: { name: Bob Durand, id: 5d0e77b1 }
created: 2026-10-26T09:12:04+01:00
updated: 2026-10-26T15:02:40+01:00
after: [p2v8c4ma]
description: |-
  Chaque tentative de connexion laisse une ligne : qui, quand, d’où.
criteria:
  - Chaque connexion, réussie ou non, est journalisée
  - Tests verts
images:
  - { name: maquette.png, file: 3f2a9c01b7de.png }
files: []
external: null
outcome: null
```

| Clé | Ce qu’elle dit |
| --- | --- |
| `id` | L’identifiant stable du ticket, le nom de son fichier ; jamais sa clé. |
| `key` | Sa clé, telle que le Kanban de son auteur l’a donnée. |
| `status` | `todo`, `in-progress`, `review` ou `done`. |
| `author`, `taken_by`, `updated_by` | Qui l’a partagé, qui l’a lancé, qui a changé le fichier en dernier : `{ name, id }`. |
| `after` | Les `id` des tickets partagés qu’il attend (« Après »). |
| `images`, `files` | Ses images et ses fichiers joints, dans le dossier de son `id`. |
| `external` | Le ticket Jira, Trello, GitHub, Linear, ClickUp, GitLab ou Azure Boards d’où il a été importé. |
| `outcome` | Ce qu’a fait sa validation, une fois terminé. |

Restent sur ton poste : les boucles, le coût, l’avancement de l’agent, les critères cochés, le compte, le modèle, l’agent et sa branche. Le schéma JSON du format est publié à `https://escouade.dev/schemas/escouade-ticket.schema.json`.

Pour un projet de plusieurs dépôts dont le dossier n’est pas lui-même un dépôt, le dossier `.escouade/tickets/` va dans le premier dépôt.

## Les clés

Un ticket partagé garde partout la clé que lui a donnée son auteur : le DEM-12 d’Alice est DEM-12 chez toi. Tes nouveaux tickets ne reprennent jamais la clé d’un ticket partagé que tu as reçu. Si Alice et toi avez chacun créé un DEM-12 avant de tirer, le tien garde sa clé et le sien porte le nom d’Alice sur sa carte ; pour le [serveur MCP](mcp), il se nomme `DEM-12@alice`, ou par son identifiant.

## Quand ça coince

- **Deux personnes sur le même ticket.** Si le fichier reçu dit qu’Alice l’a pris alors que ton agent y travaille, ton agent continue et la carte le dit ; ton prochain statut réécrit le fichier.
- **Un fichier en conflit git.** Escouade ne le lit pas : la carte garde son dernier état et propose « Garder le mien » (Escouade réécrit le fichier avec ta version, ce qui lève le conflit) ou « Prendre le sien » (le côté reçu). Rien n’est commité.
- **Un fichier illisible.** Il devient une carte « Ticket partagé illisible », avec la ligne en cause ; Escouade ne le réécrit jamais et ne le lance pas.
- **Un ticket que son auteur ne partage plus.** Il quitte ton Kanban à ton pull, sauf si tu l’avais lancé : il reste alors chez toi, comme un des tiens.
- **Un import déjà partagé.** Importer un ticket Jira, Trello, GitHub, Linear, ClickUp, GitLab ou Azure Boards qu’un coéquipier a déjà partagé ne crée pas de doublon : ton import, s’il n’a pas commencé, devient le ticket partagé.
- **Un projet privé.** Il se partage aussi : « privé » dit ce que voient les autres projets d’Escouade, pas ton équipe sur le dépôt. Ses tickets partagés restent invisibles des agents des autres projets.

## Avec le serveur MCP

Les outils de tickets du [serveur MCP](mcp) disent si un ticket est partagé (`shared` : par qui, si c’est toi, qui l’a pris, son fichier). `create_ticket` et `update_ticket` prennent `shared: true` pour partager, `false` pour arrêter. Un agent ne lance jamais un ticket partagé par un autre : `start_ticket` le refuse, et `delete_ticket` aussi ; seul toi le lances, le masques ou le supprimes, depuis le Kanban.
