---
title: "Spécification"
subject: "Docs"
sources: ["docs/SPEC.md"]
tickets: ["ESC-256","ESC-258","ESC-261","ESC-262","ESC-263"]
decisions: []
compared: null
---

## Ce que c'est

La spécification d'un projet : des pages Markdown rangées par sujet, tenues à jour avec le code, et le registre des décisions qui expliquent ses choix (voir *Décisions*). Elle se lit et s'écrit dans l'onglet Docs, avec les mêmes écrans pour tout le monde, et par les outils MCP des agents. Ses fichiers vivent dans la branche des docs (voir *Docs du projet*).

## Comportement

### L'onglet Docs

Troisième onglet de la barre latérale : « Agents · Kanban · Docs ». Il ne s'ouvre pas en même temps que le Kanban. Dessous, à la place des agents : « Nouvelle discussion » et les discussions (voir *Discussions*), puis trois espaces : Spécification (le nombre de ses écarts, sinon de ses pages), Décisions et Documents (leur nombre ; voir *Décisions* et *Documents*). Les docs se relisent à l'ouverture, au clic sur un espace et à chaque changement de « N changements à partager ».

### Les pages

- Une page est un fichier `.escouade/spec/<sujet>/<page>.md` : un en-tête, puis le texte en Markdown. Les pages sont rangées en dossiers par sujet, sur 4 niveaux au plus. Un chemin est un *slug* par segment (accents ôtés, minuscules, tirets).
- L'en-tête a une ligne `clé: valeur` par clé, la valeur en JSON : `title` ; `subject`, le nom affiché de son sujet ; `sources`, d'où elle vient (documents, fichiers, liens) ; `tickets` ; `decisions` ; `compared`, sa dernière comparaison au code (`null` si jamais). Trois champs ne sont écrits que quand ils servent : `gaps`, les écarts avec le code que la comparaison laisse (un texte et les fichiers en cause) ; `draft`, un brouillon ; `order`, sa place dans son sujet.
- Un fichier écrit à la main, sans en-tête, est une page quand même. Les fichiers sont lus et réécrits à l'octet près.
- L'ordre : les pages par `order`, puis par nom ; 0 range par nom, après les autres. Un sujet prend le plus petit `order` de ses pages. Son titre est le `subject` de ses pages, sinon le nom de son dossier.

### La carte des sujets

C'est l'accueil de l'espace Spécification. En tête : les pages à jour, les écarts, les brouillons et la dernière comparaison au code, avec « Demander à la spec » (voir *Discussions*), « Comparer au code » (voir *Spec tenue à jour*) et, tant que des changements n'ont pas de tickets, « N changements sans tickets · Planifier » (voir *Planifier*). Puis une carte par sujet : son état (« N écarts » si une page en a, « Brouillon » si toutes le sont, sinon « À jour »), ses pages, ses tickets et ses décisions (comptés une fois), et ses pages en résumé. Enfin « Ce qui a bougé » : les pages modifiées ici et pas encore partagées, et les trois derniers commits de la spec (auteur, date, sujets touchés).

### L'arbre

À gauche, les pages par sujet. Un sujet se replie : tous sont dépliés jusqu'à 12 pages ; au-delà, seuls ceux qui ont un écart ou la page ouverte. Une page y dit « écart », « brouillon » ou « à partager ».

### La page

- En haut, ses écarts, chacun avec « Mettre la spec à jour » (une discussion avec l'agent rédacteur, voir *Discussions*), « Ouvrir un ticket pour le code » (la fenêtre d'un ticket, titre et description écrits depuis l'écart) et « Voir le code » (le premier fichier de l'écart, dans l'éditeur).
- Le texte, rendu comme les messages de la conversation. À côté : ses sources, ses tickets, ses décisions et sa dernière comparaison, avec « Comparer au code » pour elle seule (voir *Spec tenue à jour*).
- « Modifier » : le titre et le texte dans un éditeur simple, l'en-tête gardé. « Ouvrir dans l'éditeur » : sur la branche courante seulement, où le fichier est dans le dossier du projet.
- « Nouvelle page » : un sujet et un titre, puis la page s'ouvre, prête à écrire.

### Importer une spec existante

- Tant que la spec n'a aucune page, Escouade cherche ce qui en tient lieu : `SPEC.md` (et ses variantes) à la racine ou dans `docs/`, un dossier `spec` ou `specs`, et `docs/` s'il a des fichiers Markdown. Pour chacun : son nombre de fichiers et sa taille. Les liens symboliques ne comptent pas.
- L'espace vide propose chaque candidat. « Voir le découpage » montre les pages sans rien écrire :
  - dans un fichier, le niveau de titre qui revient plus d'une fois donne les sujets, le suivant les pages. Un titre seul en tête est le nom du document et le titre de l'introduction. Ce qui précède le premier sujet devient « Introduction ». Un sujet sans sous-titre est une page. Les titres dans un bloc de code ne comptent pas. L'en-tête YAML est laissé ;
  - dans un dossier, un sujet par fichier (son titre `# `, sinon son nom).
- On décoche des pages, puis « Importer ». Chaque page cite son fichier dans `sources` et garde son ordre. Une page déjà là reste telle quelle. Seul un candidat proposé est lu, sans lien symbolique ni sortie du projet ; le fichier d'origine n'est pas touché. Ce que les agents en sont ensuite prévenus est dans *Spec tenue à jour*.
- L'espace vide propose aussi « Écrire une première spec depuis le code » (voir *Discussions*) et « Écrire une première page ».

### Pour les agents (MCP)

Tout ce que les agents écrivent dans la spec passe par ces outils. Ils écrivent dans la branche des docs, sans rien commiter ; ceux d'une discussion n'écrivent que des propositions (voir *Discussions*). Les formats sont dans `docs/PROTOCOL.md`.

- `get_spec_tree` (l'arbre ; tant que la spec est vide, les candidats à l'import, sans rien importer), `read_spec_page`, `search_spec`, `write_spec_page`. Ce dernier crée ou modifie une page : un chemin sans dossier va sous `subject`, sinon sous `general`. Sa réponse donne les lignes ajoutées et retirées.
- Les décisions ont leurs outils (voir *Décisions*).

## Limites

- « Ouvrir dans l'éditeur » n'existe que si la branche des docs est la branche courante.
- Un import prend 500 pages au plus.
