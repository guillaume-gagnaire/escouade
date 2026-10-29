---
title: "Ouvrir un fichier"
subject: "Éditeur"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
order: 28
---

## Ce que c'est

La palette « Ouvrir un fichier » trouve un fichier par son nom et l'ouvre dans l'éditeur (voir *Éditeur*), à une ligne précise au besoin.

## Comportement

### La palette

- `Ctrl+P` (Cmd+P sous macOS) l'ouvre sur la source de l'éditeur s'il est ouvert, sinon sur le worktree de l'agent sélectionné, sinon sur le projet. La source est rappelée à droite du champ. L'arborescence est relue à chaque ouverture.
- La saisie trie les fichiers : nom qui commence par elle, nom qui la contient, chemin qui la contient, puis lettres dans l'ordre dans le nom, puis dans le chemin. Le début d'un mot est favorisé, les chemins courts viennent d'abord. 50 résultats au plus.
- `nom:42` ouvre à la ligne 42 (« · ouvre à la ligne 42 ») ; `nom:42:7` aussi à la colonne.
- Champ vide : les fichiers ouverts récemment (30 par source, en mémoire), sauf celui affiché, puis les autres.
- Chaque ligne : le nom en gras, le dossier en gris, « ignoré par git » pour un fichier copié dans les worktrees.
- Statut : « Chargement des fichiers… », « N fichiers · liste tronquée », « Aucun fichier ne correspond. », « Les 50 premiers résultats : précise ta recherche. ».
- ↑ ↓ (en boucle) ; Entrée ou clic ouvrent ; Échap ou un clic dehors ferment.

### Les touches

- `Ctrl+P` et `Ctrl+Maj+P` n'impriment jamais la page. Ils sont retenus avant tout champ, même derrière une boîte de dialogue, où `Ctrl+P` n'ouvre rien.
- La touche P est reconnue à sa place sur un clavier d'un autre alphabet (cyrillique, grec).
- Dans un terminal interactif sous Windows, `Ctrl+P` reste au shell (commande précédente ; voir *Raccourcis*). Dans le journal en lecture seule d'une commande de lancement, il ouvre la palette.

## Limites

- Sur Dvorak, Colemak ou Workman, la touche P n'est pas reconnue à sa place : cette touche y tape une autre lettre.
