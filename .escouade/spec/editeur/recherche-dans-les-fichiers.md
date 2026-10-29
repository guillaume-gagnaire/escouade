---
title: "Recherche dans les fichiers"
subject: "Éditeur"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
order: 27
---

## Ce que c'est

Chercher un texte ou une expression régulière dans tous les fichiers de la source affichée, depuis la colonne de gauche de l'éditeur (voir *Éditeur*). La même recherche trouve les références d'un nom.

## Comportement

### Ouvrir la recherche

- `Ctrl+Maj+F`, l'éditeur affiché seulement (dans un terminal, la touche reste au shell), ouvre la vue « Rechercher dans les fichiers », titrée « Recherche », le curseur dans le champ « Rechercher ».
- Le champ prend le texte sélectionné dans le code s'il tient sur une ligne et que le focus est dans le code ; il est échappé quand « Expression régulière » est activée.
- `Maj+F12` (« Trouver les références ») cherche le nom sous le curseur, où qu'il soit, en « Mot entier » et « Respecter la casse », « Expression régulière » désactivée.
- Options, dans le champ : « Respecter la casse », « Mot entier », « Expression régulière ».

### La recherche

- Elle part 250 ms après la frappe ; Entrée la lance tout de suite, et de nouveau.
- Elle cherche dans la source affichée, et repart au changement de source. Seule la réponse de la dernière compte ; les résultats précédents restent, atténués, jusqu'à elle.
- Elle est gardée par projet tant que l'app tourne, même l'éditeur fermé.
- Ce qu'elle lit : les fichiers suivis et nouveaux de la source, ignorés exclus. Dans un dossier sans git : ses fichiers, sans ceux que l'arborescence saute (`node_modules`, `target`…). Jamais les worktrees des agents (`.claude/worktrees` à toute profondeur, et le « Dossier des worktrees » du projet).
- Bornes : 2 000 lignes par défaut (5 000 au plus), 10 s au plus ; au-delà, la recherche est arrêtée, avec tout ce qu'elle a lancé.
- Une expression s'écrit comme en JavaScript. Quand le git du poste ne lit pas ces expressions, elle est traduite (`\d`, `\s`, `\w`…). Ce qui ne se traduit pas est refusé : « Cette expression n'est pas prise en charge par git sur ce poste : \b ». Une expression invalide aussi (« Expression régulière invalide. », sous le champ), comme un motif sur plusieurs lignes.
- Chaque ligne trouvée vient avec chacune de ses correspondances, dans une fenêtre de 300 caractères autour de la première.

### Les résultats

- Un groupe par fichier (icône, nom, dossier, nombre, repliable), puis ses lignes, chaque correspondance surlignée.
- En bas : « Recherche… », « N résultats dans M fichiers » (des lignes), « Aucun résultat. », puis « Résultats limités aux 2 000 premiers. » ou « Recherche arrêtée après 10 s : résultats partiels. ».
- Clic ou Entrée sur une ligne : le fichier à la ligne et à la colonne, noté dans l'historique (voir *Navigation de code*).
- Clavier : ↓ depuis le champ ; ↑ ↓ Début Fin entre les lignes ; ↑ depuis la première revient au champ ; → déplie un fichier ou entre dans ses lignes ; ← le replie ou remonte au fichier.

## Limites

- La recherche lit le disque, pas les onglets non enregistrés.
- Les résultats ne suivent pas les fichiers modifiés, renommés ou supprimés depuis : ils gardent les anciens chemins jusqu'à la recherche suivante (Entrée relance).
- « Mot entier » suit les lettres ASCII de git : `caf` trouve `café`.
- Une correspondance au-delà des 300 caractères montrés n'est pas surlignée.
