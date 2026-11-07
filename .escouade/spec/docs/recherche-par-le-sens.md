---
title: "Recherche par le sens"
subject: "docs"
sources: ["src-tauri/src/vectors/mod.rs","src/components/settings/VectorsModelRow.svelte","src/components/settings/VectorsSection.svelte","website/content/docs/fr/settings-projects.md"]
tickets: ["ESC-252","ESC-254","ESC-310"]
decisions: []
compared: "2026-10-31T16:08:13+01:00"
gaps: [{"text":"La page dit que la recherche par le sens couvre la spec, les décisions, les documents et la mémoire. Dans le code, seuls les documents du drive (docsdrive.rs) et la mémoire (memory.rs) sont indexés et cherchés avec les vecteurs. `Source::Spec` et `Source::Decision` existent dans l'énumération, mais aucun appelant ne les indexe ni ne les cherche. Il n'y a donc pas de recherche par le sens dans la spec ni dans les décisions.","code":["src-tauri/src/vectors/index.rs","src-tauri/src/docsdrive.rs","src-tauri/src/memory.rs"]},{"text":"La page dit que la recherche ne reçoit que des textes autorisés par la clôture du projet : un fichier interdit n'entre dans aucun index ni dans aucune réponse. Je n'ai trouvé aucune référence à la clôture dans le code d'indexation et de recherche (vectors/*, docsdrive.rs, memory.rs, mcp/memory.rs, mcp/docs.rs). Le contrôle annoncé n'y est pas appliqué.","code":["src-tauri/src/vectors/index.rs","src-tauri/src/docsdrive.rs","src-tauri/src/memory.rs"]},{"text":"Point secondaire : la page dit que la recherche ne sert pas au code. Le code garde pourtant `Source::Code` dans `Source::ALL` et un format de découpage `Format::Code`. Aucun appelant ne les utilise pour l'instant.","code":["src-tauri/src/vectors/index.rs","src-tauri/src/vectors/chunk.rs"]}]
---

## Ce que c'est

En plus des mots, la mémoire partagée et les docs (la spec, les décisions, les documents) se cherchent par le sens, avec un petit modèle multilingue qui tourne sur la machine. Un seul modèle pour tous les projets ; rien ne quitte la machine, sauf son téléchargement. Elle ne sert pas au code : chaque harness trouve le code par ses propres outils (voir *Index du code*, retiré).

## Comportement

- **Le modèle** : multilingual-e5-base de Microsoft (8 bits, 768 dimensions, licence MIT, 295 Mo : 278 Mo de modèle et 17 Mo de tokenizer). Un seul pour tous les projets, lu sur la machine par le même moteur que les voix. Les textes portent le préfixe `query: ` ou `passage: ` du modèle.
- **Le téléchargement**, comme celui d'une voix : par le proxy des réglages, avec une jauge, et annulable. Chaque fichier n'est gardé qu'entier, de la bonne taille et de la bonne empreinte SHA-256 ; un fichier qui n'est pas le bon est supprimé. Après une coupure ou une annulation, le téléchargement reprend là où il en était, sauf avec un serveur qui ne sait pas reprendre.
- **Le découpage** : le Markdown par titres (un morceau ne traverse pas un titre et se lit avec le chemin des titres au-dessus, `A › B`), le texte par paragraphes, le code par blocs séparés d'une ligne vide. Les petits voisins sont joints jusqu'à 900 caractères. Aucun morceau ne dépasse 1 400 caractères (environ 300 jetons ; le modèle en lit 512). Un bloc de code entre ``` n'est coupé ni par un titre ni par une ligne vide.
- **L'index** : une base par projet dans le dossier de données d'Escouade (`projects/<id>/vectors.db`), jamais dans le dépôt. Un texte inchangé ne coûte rien. D'un texte modifié, seuls les morceaux nouveaux passent par le modèle. Un index fait par un autre modèle est vidé.
- **La recherche** : les plus proches par le sens, sur tous les morceaux, sans approximation. Elle est fusionnée avec la recherche par les mots, pour qu'un mot rare (un nom, un identifiant) retrouve son morceau. Elle sait chercher dans la spec, les décisions, les documents et la mémoire. Les documents s'en servent (voir *Documents*), la mémoire aussi (voir *Mémoire partagée*).
- Elle ne reçoit que des textes que la clôture du projet autorise : un fichier interdit n'entre dans aucun index ni dans aucune réponse.

## Réglages

- **Réglages › Projets › Tous les projets › « Recherche par le sens »** (visible même sans projet ouvert) : le modèle, sa licence et sa taille, « Télécharger (N Mo) » avec sa jauge, « Annuler », « Reprendre (N %) » et « Effacer ». Sans le modèle, la recherche se fait par les mots seuls.

## Mesures

- Build de debug, Windows. Chargement du modèle : 1,2 s. 195 passages courts par seconde, et 27 morceaux par seconde sur de longs documents. Ajouter une section à un document indexé relit 1 morceau sur 30, en 11 ms. Une recherche prend 8 ms sur 506 morceaux, 17 ms sur 1 000, 173 ms sur 10 000 et 860 ms sur 50 000.
