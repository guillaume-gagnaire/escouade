# Utiliser

## Importer des tâches

Dans le Kanban, « Importer » (⤓) ouvre la fenêtre « Importer des tickets ». L'onglet ClickUp montre les tâches ouvertes de la liste liée, sous-tâches comprises, les dernières modifiées d'abord, 100 à la fois. Une tâche dont le statut est de type fini (`done`) ou fermé (`closed`) n'y est pas.

- **La recherche** (« Rechercher par id (#86abx3, ABC-12) ou par titre… ») :
  - l'id d'une tâche, avec ou sans `#` (`#86abx3`, ou `86abx3` quand il mêle lettres et chiffres et fait au moins six caractères), trouve cette tâche si elle est dans la liste ;
  - un id personnalisé (`ABC-12`), quand ton espace en a, aussi ;
  - un autre texte garde les tâches dont le nom le contient. ClickUp n'ayant pas de recherche par titre, le plugin lit la liste 100 tâches à la fois et s'arrête dès qu'une page en trouve, ou après 5 pages ; « Afficher plus » continue.
- **Les puces** : « Mes tâches » garde celles qui te sont assignées ; puis une puce par statut de la liste qui n'est ni fini ni fermé.
- **Chaque ligne** montre l'id personnalisé de la tâche (sinon `#` et son id), son nom, le type « Tâche », son statut, sa priorité, les personnes assignées et ses tags, et les critères trouvés.

Coche les tâches, choisis « Boucles max », puis « Importer ». Chaque ticket importé garde sur sa carte du Kanban la pastille ClickUp et la clé de la tâche : un clic l'ouvre dans ClickUp. Sa description est celle de la tâche, en Markdown.

## Les critères d'acceptation

Avec « Extraire les critères d'acceptation » activé (Réglages › Services connectés, groupe « Synchronisation »), le plugin prend comme critères, dans cet ordre :

1. les éléments de la checklist qui porte le nom des critères (« Critères d'acceptation », « Acceptance criteria », « Definition of done », « Critères », « AC », « DoD »), dans leur ordre ;
2. sinon les éléments de toutes les checklists de la tâche ;
3. sinon, dans sa description, les éléments de liste sous un titre ou une ligne qui nomme les critères, ou à défaut ses cases à cocher (`- [ ]`).

Il en garde vingt au plus. Sans critère trouvé, le ticket reçoit les critères par défaut d'Escouade.

## La tâche suit son ticket

Chaque liste ClickUp a ses propres statuts : la « Correspondance des statuts » de Réglages › Projets › Tickets importés se règle donc sur ceux de la liste liée. Choisir la liste la pré-remplit :

- d'abord par les noms : le statut qui ressemble à « en cours » (`In Progress`, `Doing`, `WIP`…) pour « En cours » et « À tester », celui qui ressemble à « à tester » (`Review`, `QA`, `Testing`…) pour « Terminé » ;
- sinon par leur type : le premier statut personnalisé (entre l'ouvert et les fermés) pour « En cours » et « À tester », le suivant pour « Terminé », à défaut le premier statut de type fini ;
- une liste qui n'a qu'un statut ouvert et un fermé ne déplace rien par défaut. « À faire » reste inchangé.

Quand un ticket importé arrive dans une colonne :

- la tâche prend le statut choisi, si « Mettre à jour le statut externe » est activé. Si la liste n'a plus ce statut (renommé, retiré), la tâche reste où elle est et la carte le dit ;
- Escouade y écrit un commentaire si la colonne est cochée dans « Commenter » (par défaut « À tester » et « Terminé »). Le commentaire est posté sans prévenir tous les abonnés de la tâche.

Avec « Publier un résumé à chaque boucle », chaque nouvelle boucle d'un agent laisse aussi un commentaire : les critères atteints et les notes de l'agent.

## L'import automatique

Réglages › Services connectés › « Import automatique » importe seul les tâches ouvertes qui portent l'étiquette choisie (`claude-ready` par défaut). Pour ClickUp, c'est un tag de la tâche ; ClickUp garde ses tags en minuscules.

## Les limites

- Un seul compte ClickUp par Escouade, et une liste par projet.
- Quand ClickUp limite les requêtes (429), le plugin attend la fin de la limite qu'il annonce, une minute au plus, et redemande une fois.
- Une réponse de ClickUp ne peut pas dépasser 8 Mio ni prendre plus de 15 secondes.
- Escouade ne crée pas de tâche, ne la modifie pas (nom, description, personnes assignées) et ne la supprime pas : elle change son statut et la commente, rien de plus.
