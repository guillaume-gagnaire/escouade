---
title: "Voir les changements"
subject: "Éditeur"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
order: 29
---

## Ce que c'est

L'éditeur (voir *Éditeur*) montre ce qu'un fichier a changé par rapport à sa version de référence : les lignes modifiées dans la marge, leur compte, et sur demande les changements dans le texte. La même vue compare le texte ouvert à une version du fichier changée sur le disque.

## Comportement

### La référence

- Pour la branche du projet, la référence est HEAD ; pour un worktree, la branche de départ.
- Les lignes modifiées par rapport à elle sont marquées, et le fil d'Ariane les compte : « N lignes modifiées vs main ».

### « Voir les changements »

- Le bouton est à droite de « N lignes modifiées vs main », quand la version de référence existe : pas pour un nouveau fichier ni hors d'un dépôt.
- Il reste enfoncé tant que les changements sont montrés dans le texte : les lignes supprimées au-dessus de chaque bloc, les lignes et caractères changés teintés. Le texte reste modifiable.
- Le choix vaut pour la vue de l'éditeur, d'onglet en onglet.
- « Annuler ce bloc » remet le bloc de la référence, sans enregistrer ; `Ctrl+Z` le reprend. Au clavier, Échap puis Tab atteignent les boutons des blocs.
- Les blocs suivent les lignes uniques communes aux deux versions, puis les caractères, chaque morceau borné : chaque changement d'un lockfile a son bloc, et taper dans un long bloc réécrit reste instantané.

### Comparer avec le disque

- « Comparer » vient du bandeau d'un fichier modifié qui a changé sur le disque (voir *Éditeur*) : « Comparaison avec la version du disque. », la même vue contre la version du disque, sans toucher au texte.
- « Prendre ce bloc » copie un bloc du disque. « Garder ma version » enregistre le texte, mais seulement par-dessus la version montrée. « Recharger » prend celle du disque. Le bandeau reste jusqu'au choix.
- Une version plus récente arrivée pendant la comparaison (l'agent écrit encore) la remplace dans la vue, sans toucher au texte : « Le fichier a encore changé sur le disque : la comparaison montre sa nouvelle version. », dans le bandeau et dans un toast (aussi en revenant sur l'onglet). « Garder ma version » attend alors une seconde.
- La même version trouvée par un enregistrement refusé : « Le fichier a encore changé sur le disque : rien n'est enregistré, la comparaison montre sa nouvelle version. ».
- Le disque revenu à la version ouverte : le même message que hors comparaison (voir *Éditeur*).
- Un échec : « Comparaison impossible : … ».

## Limites

- Dans un fichier fait de longues suites de lignes vides, un bloc vide ou sans effet peut rarement apparaître (de l'ordre d'une paire de textes sur 240 000, passager après une frappe).
- Au-delà d'environ 63 000 lignes distinctes, ou dans un fichier de lignes répétées, un passage entier peut faire un seul bloc.
- Une nouvelle version du disque qui n'est pas du texte laisse la comparaison sur la précédente.
- Tous les boutons des blocs portent le même nom.
