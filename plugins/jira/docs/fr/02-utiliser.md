# Utiliser

## Importer des tickets

Dans le Kanban, « Importer » (⤓) ouvre la fenêtre « Importer des tickets ». L'onglet Jira montre les tickets du projet lié qui ne sont pas dans la catégorie « Done », les derniers modifiés d'abord, 50 à la fois ; « Afficher plus » charge les suivants, et la ligne au-dessus de la liste dit combien Jira en compte en tout (« 50 affichés sur 312 ») quand la liste est coupée.

- **La recherche** (« Rechercher par clé ou par texte… ») : une clé, `ATL-12`, trouve ce ticket ; un autre texte est cherché par Jira dans le texte des tickets (`text ~`), les signes spéciaux de sa syntaxe de recherche retirés.
- **Les puces** : « Assignés à moi », « Sprint actif » (les tickets des sprints ouverts) et « À faire » (la catégorie « To Do »). Plusieurs puces se cumulent : un ticket doit répondre à chacune.
- **Chaque ligne** montre la clé, le titre, le type (« Tâche », « Sous-tâche », Bug, Story… tels que Jira les nomme), la priorité, la personne assignée (« Non assigné » sinon) et le statut, et les critères trouvés.

Coche les tickets, choisis « Boucles max », puis « Importer ». Chaque ticket importé garde sur sa carte du Kanban la pastille Jira et sa clé : un clic l'ouvre dans Jira (`<site>/browse/ATL-12`). Sa description est celle du ticket, convertie du format de document d'Atlassian en Markdown. S'il change de projet chez Jira, Escouade le suit encore : il est retenu par son id, pas par sa clé.

## Les critères d'acceptation

Avec « Extraire les critères d'acceptation » activé (Réglages › Services connectés, groupe « Synchronisation »), le plugin cherche les critères dans la description, une fois convertie en Markdown : les éléments de liste sous un titre ou une ligne qui les nomme (« Critères d'acceptation », « Acceptance criteria », « Definition of done », « Critères », « AC », « DoD »), sinon ses cases à cocher (`- [ ]`). Il en garde vingt au plus, de 300 caractères au plus. Sans critère trouvé, le ticket reçoit les critères par défaut d'Escouade.

## Le ticket Jira suit son ticket

Dans Réglages › Projets › Tickets importés, la « Correspondance des statuts » donne, pour chaque colonne du Kanban, un statut du projet Jira (ceux de tous ses types de tickets, chacun une fois). Choisir le projet la pré-remplit : le statut qui ressemble à « en cours » pour « En cours » et « À tester », celui qui ressemble à « à tester » pour « Terminé », rien pour « À faire ».

Quand un ticket importé arrive dans une colonne, si « Mettre à jour le statut externe » est activé :

- s'il est déjà dans ce statut, rien ne part ;
- sinon le plugin lit les transitions que son workflow permet depuis son statut actuel, et passe celle qui mène au statut choisi. S'il n'y en a aucune, il ne force rien : la carte le dit (voir **Dépannage**).

Escouade y écrit aussi un commentaire si la colonne est cochée dans « Commenter » (par défaut « À tester » et « Terminé »). Le commentaire est écrit en Markdown et converti au format de document d'Atlassian.

Avec « Publier un résumé à chaque boucle », chaque nouvelle boucle d'un agent laisse aussi un commentaire : les critères atteints et les notes de l'agent.

## L'import automatique

Réglages › Services connectés › « Import automatique » importe seul les tickets ouverts qui portent l'étiquette choisie (`claude-ready` par défaut). Pour Jira, c'est un label du ticket.

## Les limites

- Jira Cloud seulement : un Jira Data Center ou Server n'est pas pris en charge.
- Un seul compte Jira par Escouade, quel que soit son site, et un projet Jira par projet.
- La liste propose les 500 premiers projets du site, rangés par clé.
- Le statut suit le workflow : si aucune transition ne mène du statut actuel au statut choisi, il ne change pas.
- Une réponse de Jira ne peut pas dépasser 8 Mio ni prendre plus de 15 secondes.
- Escouade ne crée pas de ticket, ne le modifie pas (titre, description, personne assignée) et ne le supprime pas : elle le fait avancer dans son workflow et le commente, rien de plus.
