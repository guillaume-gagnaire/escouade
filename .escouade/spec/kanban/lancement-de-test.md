---
title: "Lancement de test"
subject: "Kanban"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

Tout agent qui a un worktree peut lancer l'app de sa branche pour qu'on l'essaie. L'agent écrit la recette de lancement ; l'utilisateur la lit et l'approuve ; Escouade prépare, lance les processus et ouvre le navigateur sur la fonctionnalité. Quand isola gère le projet, ce sont les services d'isola qui tournent.

## Comportement

### La recette

- « Préparer le lancement » demande la recette à l'agent, avec ses ports et, quand il en a, les commandes de lancement du projet (nom, commande, dossier) dont partir. Ce n'est pas proposé tant que son ticket est « En cours » : son bilan la donne déjà.
- « ▶ Tester » ouvre une fenêtre qui montre d'abord la recette :
  - « Préparation » : commandes et dossier ;
  - « Lancement » : nom, commande, dossier, variables et adresse de chaque processus ;
  - « Ouverture » : l'adresse ouverte ;
  - « Ces commandes ont été écrites par <agent>. Elles tournent dans ton shell, hors du mode de permission de Claude Code. »
- Rien ne tourne avant « Lancer » ; « Annuler » ferme la fenêtre.

### L'approbation

- L'approbation est gardée avec l'agent : la recette entière. Une recette identique se lance directement. Une recette modifiée (une commande, une variable, un dossier, l'adresse) redemande. Le cœur refuse de lancer une étape d'une recette non approuvée.
- La recette se lit telle que le shell la lira : les caractères invisibles ou de direction (retour chariot, séquence d'échappement, espace de largeur nulle, lettres de droite à gauche) sont écrits en clair, les suites de lignes vides ou d'espaces sont comptées (« ⟨12 lignes vides⟩ »), et le texte est montré dans l'ordre où le shell le lit.
- Une recette (ou un `.isola.toml`) qui change pendant qu'on la lit le dit (« La recette vient de changer : relis-la avant de lancer. ») et garde « Lancer » fermé une seconde.

### Le lancement

- Une fois la recette approuvée, Escouade prépare (une fois par recette), puis lance chaque processus dans un terminal en lecture seule, dans un groupe au nom de l'agent de la section « Lancement ».
- Elle attend que chacun réponde en HTTP (toutes les 500 ms, 3 min au plus), puis ouvre le navigateur sur la page de la fonctionnalité.
- Les lancements s'arrêtent à la validation, à l'archivage, à la suppression de l'agent, à la fermeture du projet, et quand l'agent donne une nouvelle recette, qui remplace l'ancienne.
- Quand son ticket passe « Terminé », ses terminaux de test quittent la section « Lancement » avec leurs logs, et la fenêtre « Tester » de l'agent se ferme.

### Avec isola

- isola (github.com/cyucelen/isola) est utilisé quand son CLI `isola` est dans le PATH (cherché au plus toutes les 30 s) et que le worktree a un `.isola.toml` (pour un ticket : commité sur la branche cible). isola lance alors ses services sur ses propres ports et ses adresses `*.localhost`.
- Dans ce cas : pas de bloc de ports pour un ticket ; le protocole dit d'utiliser `isola up`, `isola ls --json` et `isola down`, et de ne donner que `"lancement": {"ouvrir": …}` ; pas de « Préparer le lancement ».
- « ▶ Tester » montre d'abord le `.isola.toml` du worktree, sous « Configuration isola (.isola.toml) », avec l'adresse « ouvrir ». L'agent peut écrire ou modifier ce fichier, et `isola up` lance ses services et ses commandes `setup` dans ton shell, hors du mode de permission de Claude Code. Rien ne tourne avant « Lancer ».
- L'approbation garde avec l'agent le contenu entier du fichier et l'adresse. Un fichier modifié, d'une ligne ou d'un caractère, redemande. Le cœur refuse `isola up` quand le `.isola.toml` n'est plus celui qui a été approuvé.
- Approuvé, « ▶ Tester » lance `isola up` dans un terminal en lecture seule, dans le groupe de l'agent ; isola ne demande jamais de mot de passe. Escouade lit `isola ls --json` (les services de la branche du worktree), attend que chaque service réponde sur son propre port (le proxy d'isola répond avant lui), puis ouvre l'adresse « ouvrir » de l'agent, ou celle du premier service par le proxy. Relancé, il revérifie tout.
- « Tout arrêter », la validation, l'archivage et la fermeture du projet lancent `isola down`. La suppression du worktree lance `isola destroy`, après les commandes de démontage.
