---
title: "Agent d'un ticket"
subject: "Kanban"
sources: ["docs/SPEC.md"]
tickets: ["ESC-123"]
decisions: []
compared: null
---

## Ce que c'est

Chaque ticket lancé a son agent, dans un worktree à lui. L'agent boucle : à chaque fin de tour, il fait le bilan de ses critères, et Escouade le relance sur ceux qui manquent, jusqu'à les atteindre tous ou épuiser ses boucles. Les cartes du Kanban suivent son travail en direct.

## Comportement

### Le départ

- L'agent s'appelle `<clé>-<titre>`. Pour un ticket importé d'une source, la clé est celle du ticket d'origine : `proj-123-corriger-la-connexion`, `45-…` pour `#45`. De même pour sa branche (`<préfixe>proj-123-corriger-la-connexion` au lieu de `<préfixe>dem-12`), avec `-2`… si le nom est pris ; sa PR reprend cette clé dans son titre. Un ticket déjà lancé garde ses noms.
- Modèle et effort : ceux du ticket, sinon du Kanban, sinon de l'app. Mode : celui du Kanban, sinon de l'app. Avec Haiku, qui n'a pas d'effort, l'agent part sans effort.
- Avec un modèle sans mode Auto d'après le Claude Code de son compte, comme dans la zone de saisie, l'agent part en mode « Demander » si son mode serait Auto. C'est le cas de Haiku 4.5, que l'alias `haiku` fait tourner jusqu'à Claude Code 2.1.292, et non de Haiku 5.5, qu'il fait tourner depuis la 2.1.293.
- Son worktree est sur la branche `ticket/<clé>` (« Préfixe des branches des tickets » du projet), créée depuis la branche cible.
- Avec « Reprendre une branche existante… » dans le champ « Branche » du ticket, le worktree est fait sur cette branche. Elle doit exister, être libre, et n'être ni celle du dossier du projet à ce moment-là ni la branche cible du Kanban ; sinon le ticket est bloqué avec la raison. L'entrée dit au survol « Escouade ne supprime jamais cette branche, même une fois le ticket validé. »
- Si l'agent précédent du ticket, archivé, a encore la branche dans son worktree : ce dossier est supprimé avant le départ quand il n'a aucun changement non commité. La branche reste, et restaurer l'agent recrée le dossier si la branche est libre. Sinon le ticket est bloqué : « L’agent <nom>, archivé, garde la branche « <branche> » dans son worktree avec des changements non commités : commite-les ou supprime l’agent, puis reprends le ticket. »
- Un bloc de 10 ports lui est donné, à partir de 4100 ; aucun quand isola gère le projet (voir *Lancement de test*).
- Les fichiers ignorés par git sont copiés dans le worktree (`.env*` par défaut), puis vient sa préparation (voir *Git & fichiers*).
- Le protocole du ticket s'ajoute au prompt système de l'agent (voir *Langues*). La préparation finie, son premier message donne les critères numérotés, suivis de l'échec de la préparation s'il y en a un.

### La boucle

- Chaque fin de tour est lue : le dernier bloc `escouade` du texte de l'agent.
  - Tous les critères atteints : le ticket passe « À tester ».
  - Limite de boucles atteinte : « À tester » avec « Objectif partiel ».
  - Sinon : boucle suivante, avec les critères manquants et leurs notes.
- Sans bilan : un rappel, puis « Bilan des critères manquant ».
- Stop : « Interrompu ». Erreur ou process mort : « Erreur : … ». Limite d'usage : la reprise automatique s'en charge (voir *Comptes*). Connexion que Claude Code n'a pas pu rafraîchir : la relance du process ou la nouvelle connexion s'en chargent, sans bloquer le ticket.
- Un ticket bloqué libère sa place ; « Reprendre » relance son agent.
- Au démarrage de l'app, un tour coupé par l'arrêt est repris. Une validation coupée laisse « Validation interrompue ».

### Le bilan et l'avancement

- Le bilan est un bloc `escouade` ; ses clés sont les mêmes dans les deux langues (voir *Langues*).
- À chaque bilan, l'agent donne la liste succincte de ce qu'il a fait dans le ticket jusque-là : 1 à 8 éléments courts, de ses mots.
- L'exemple du protocole n'a que des places à remplir (`<ce que tu as fait>`…), qui ne peuvent passer pour un travail. Rendues telles quelles, elles sont écartées de la liste.
- Le dernier bilan remplace le précédent ; un bilan sans liste garde la précédente.
- Dans la conversation, le bloc `escouade` s'affiche en carte « Bilan des critères » (✓ / ○, critère, note ; puis l'avancement), suivie de « Lancement de test » s'il y a une recette.
- La liste nourrit aussi le corps de la PR et le message de commit écrit à la validation.

### Les cartes

- « En cours » : « Boucle n/max », critères ✓ / ○, barre de progression, les 3 derniers éléments de l'avancement (« +n » pour les autres), et l'activité en direct de l'agent (« Lit src/db.ts », « Lance npm test », « Réfléchit »…) ou « Question en attente de ta réponse ». Au-dessus, la ligne d'état que l'agent a donnée par le serveur MCP, s'il en a donné une (voir *Serveur MCP*).
- « À tester » : toute la liste, sous « Ce qui a été fait ».
- « En cours » et « À tester » portent le coût cumulé du ticket : tous ses agents, archivés compris ; rien tant que rien n'a été dépensé. Exact pour les tours finis, « ≈ » quand un tour tourne, comme la barre de statut.
- « Terminé » : « n boucles · coût ». « n boucles » compte toutes les boucles de son agent, renvois compris. Le coût de tous ses agents est figé à la validation, jamais moins que celui de l'agent qui valide.
- Un agent supprimé ne compte plus dans le coût d'un ticket.
- Dans une colonne étroite, le pied de la carte garde le nom de l'agent (une douzaine de caractères au moins) et passe dessous le coût et « n/N critères » (ou « n boucles · coût »). « ≈ », le montant et « $ » ne se séparent jamais d'une ligne à l'autre.
- Seules les cartes du Kanban montrent l'activité. Dans la barre latérale, la carte de l'agent porte « ▸ ATL-42 · boucle 2/5 ».

### Archiver ou supprimer son agent

- Archiver ou supprimer l'agent d'un ticket « En cours » ou « À tester » renvoie le ticket « À faire ».
- Depuis la barre latérale, archiver demande d'abord « Archiver <agent> ? » (« Son ticket <clé> repartira « À faire ». »).
