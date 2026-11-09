# Utiliser

## Importer des cartes

Dans le Kanban, « Importer » (⤓) ouvre la fenêtre « Importer des tickets ». L'onglet Trello montre les cartes ouvertes du tableau lié, toutes ses listes ouvertes confondues.

- **La recherche** (« Rechercher une carte… ») garde les cartes dont le nom contient le texte, sans tenir compte des majuscules. Un numéro de carte, `#12` ou `12`, trouve la carte `#12`.
- **Les puces** : « Mes cartes » garde celles dont tu es membre ; puis une puce par liste du tableau, qui garde les cartes de cette liste (plusieurs puces : les cartes de l'une ou l'autre).
- **Chaque ligne** montre le numéro de la carte (`#12`), son nom, le type « Carte », sa liste et ses étiquettes, et les critères trouvés (« ✓ 2 critères détectés »).

Coche les cartes, choisis « Boucles max », puis « Importer ». Chaque ticket importé garde sur sa carte du Kanban la pastille Trello et le numéro de la carte : un clic ouvre la carte dans Trello. Sa description est celle de la carte.

## Les critères d'acceptation

Avec « Extraire les critères d'acceptation » activé (Réglages › Services connectés, groupe « Synchronisation »), le plugin prend comme critères, dans cet ordre :

1. les éléments de la checklist qui porte le nom des critères (« Critères d'acceptation », « Acceptance criteria », « Definition of done », « Critères », « AC », « DoD ») ;
2. sinon les éléments de toutes les checklists de la carte ;
3. sinon, dans sa description, les éléments de liste sous un titre ou une ligne qui nomme les critères, ou à défaut ses cases à cocher (`- [ ]`).

Il en garde vingt au plus. Sans critère trouvé, le ticket reçoit les critères par défaut d'Escouade.

## La carte suit son ticket

Dans Réglages › Projets › Tickets importés, la « Correspondance des statuts » donne, pour chaque colonne du Kanban, la liste Trello où ranger la carte. Choisir le tableau la pré-remplit : la liste qui ressemble à « en cours » pour « En cours » et « À tester », celle qui ressemble à « à tester » pour « Terminé », rien pour « À faire ».

Quand un ticket importé arrive dans une colonne :

- la carte est rangée dans la liste choisie, si « Mettre à jour le statut externe » est activé ;
- Escouade y écrit un commentaire si la colonne est cochée dans « Commenter » (par défaut « À tester » et « Terminé ») : le ticket est pris, prêt à tester avec ses critères, ou terminé avec le lien de sa pull request.

Avec « Publier un résumé à chaque boucle », chaque nouvelle boucle d'un agent laisse aussi un commentaire : les critères atteints et les notes de l'agent.

## L'import automatique

Réglages › Services connectés › « Import automatique » importe seul les cartes ouvertes qui portent l'étiquette choisie (`claude-ready` par défaut). Pour Trello, c'est une étiquette de la carte, dont le nom doit être le même, majuscules mises à part.

## Les limites

- Un seul compte Trello par Escouade, et un tableau par projet.
- La liste des tableaux est celle de tes tableaux ouverts. Un tableau archivé n'y est pas, ni une liste archivée parmi les statuts.
- Le tableau est lu en une fois : toutes ses cartes ouvertes et leurs checklists, sans pages. Une réponse de Trello ne peut pas dépasser 8 Mio ni prendre plus de 15 secondes.
- Escouade ne crée pas de carte, ne la modifie pas (titre, description, membres) et ne la supprime pas : elle la range et la commente, rien de plus.
