# Utiliser

## Importer des issues

Dans le Kanban, « Importer » (⤓) ouvre la fenêtre « Importer des tickets ». L'onglet Linear montre les issues de l'équipe liée qui ne sont ni terminées ni annulées (états de type `completed` et `canceled`), les dernières modifiées d'abord, 50 à la fois ; « Afficher plus » charge les suivantes.

- **La recherche** (« Rechercher par identifiant (ENG-42) ou par titre… ») : `ENG-42`, `42` ou `#42` trouve l'issue numéro 42 de l'équipe ; un autre texte garde les issues dont le titre le contient, sans tenir compte des majuscules.
- **Les puces** : « Mes issues » garde celles qui te sont assignées ; puis une puce par état de l'équipe qui n'est ni terminé ni annulé (plusieurs puces : les issues de l'un ou l'autre).
- **Chaque ligne** montre l'identifiant (`ENG-42`), le titre, le type « Issue », l'état, la priorité (sauf « No priority »), la personne assignée et les étiquettes, et les critères trouvés.

Coche les issues, choisis « Boucles max », puis « Importer ». Chaque ticket importé garde sur sa carte du Kanban la pastille Linear et l'identifiant de l'issue : un clic l'ouvre dans Linear. Sa description est celle de l'issue.

## Les critères d'acceptation

Avec « Extraire les critères d'acceptation » activé (Réglages › Services connectés, groupe « Synchronisation »), le plugin cherche les critères dans la description de l'issue : les éléments de liste sous un titre ou une ligne qui les nomme (« Critères d'acceptation », « Acceptance criteria », « Definition of done », « Critères », « AC », « DoD »), sinon ses cases à cocher (`- [ ]`). Il en garde vingt au plus, de 300 caractères au plus. Sans critère trouvé, le ticket reçoit les critères par défaut d'Escouade.

## L'issue suit son ticket

Dans Réglages › Projets › Tickets importés, la « Correspondance des statuts » donne, pour chaque colonne du Kanban, l'état Linear de l'issue. Les états de l'équipe y sont rangés dans l'ordre qu'une issue suit : triage, backlog, à faire, en cours, terminé, annulé. Choisir l'équipe pré-remplit la correspondance : l'état qui ressemble à « en cours » pour « En cours » et « À tester », celui qui ressemble à « à tester » pour « Terminé », rien pour « À faire ».

Quand un ticket importé arrive dans une colonne :

- l'issue passe à l'état choisi, si « Mettre à jour le statut externe » est activé ;
- Escouade y écrit un commentaire, en Markdown, si la colonne est cochée dans « Commenter » (par défaut « À tester » et « Terminé »).

Avec « Publier un résumé à chaque boucle », chaque nouvelle boucle d'un agent laisse aussi un commentaire : les critères atteints et les notes de l'agent.

## L'import automatique

Réglages › Services connectés › « Import automatique » importe seul les issues qui portent l'étiquette choisie (`claude-ready` par défaut). Pour Linear, c'est une étiquette de l'issue, dont le nom doit être le même, majuscules mises à part.

## Les limites

- Un seul compte Linear par Escouade, et une équipe par projet.
- La liste propose tes 100 premières équipes.
- La fenêtre d'import ne dit pas combien d'issues correspondent en tout : Linear ne le donne pas.
- Une réponse de Linear ne peut pas dépasser 8 Mio ni prendre plus de 15 secondes.
- Escouade ne crée pas d'issue, ne la modifie pas (titre, description, personne assignée) et ne la supprime pas : elle change son état et la commente, rien de plus.
