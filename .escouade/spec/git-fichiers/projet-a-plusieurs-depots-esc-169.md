---
title: "Projet à plusieurs dépôts"
subject: "Git & fichiers"
sources: ["docs/SPEC.md","design/app-redesign/UX-NOTES.md"]
tickets: ["ESC-169","ESC-160","ESC-156"]
decisions: []
compared: null
order: 24
---

## Ce que c'est

Un projet peut tenir plusieurs dépôts git : un dossier qui n'est pas un dépôt et en contient au moins un, ou un dépôt qui en contient d'autres que ses sous-modules (la racine compte alors comme un dépôt, « racine », en tête). Escouade le voit comme un seul projet : un agent y a un worktree de chaque dépôt sous une même branche, et le panneau Git, les merges, les PR et la validation d'un ticket vont dans chaque dépôt changé. Les dépôts se nomment par leur chemin depuis le dossier du projet (`api`, `services/billing`). Un projet à un seul dépôt ne change en rien : ni groupe, ni menu, ni « · 1 dépôt ».

## Comportement

### Trouver les dépôts

- Escouade cherche dans les sous-dossiers directs et ceux d'un niveau plus bas. Jamais dans un dossier caché (`.git`, `.claude`…), `node_modules`, `target`, `vendor`, `dist`, `build`, `__pycache__`, le dossier des worktrees du projet, un lien, ni, si la racine est un dépôt, un dossier qu'elle ignore (sauf s'il est lui-même un dépôt). Il ne cherche pas dans un dépôt trouvé : un sous-module (lu dans `.gitmodules`) reste à son dépôt.
- Les dépôts sont cherchés à l'ajout du projet, au démarrage, quand la fenêtre revient au premier plan, et par « Rechercher à nouveau ». Un dossier de dépôts n'est jamais fait dépôt par `git init`.
- Un dépôt apparu est annoncé (« Nouveau dépôt git dans acme : services/billing ») ; les prochains agents en ont un worktree, ceux qui ont déjà le leur n'en reçoivent pas.
- Un dépôt disparu reste listé « retiré » jusqu'à « Oublier » ; les worktrees qu'en ont des agents restent jusqu'au nettoyage.
- L'assistant « Nouveau projet » dit « 4 dépôts git : api, docs, front, infra » (ce qui a été passé, au survol) et « infra n'a aucun commit » (un worktree part d'un commit) ; ses vérifications prennent les branches du premier dépôt qui a un commit.
- `.escouade.yml` est celui de la racine du dossier : elle n'est pas un dépôt, il n'est donc versionné nulle part tant que l'utilisateur ne l'y met pas. Celui d'un dépôt n'est pas lu.

### Barre d'état et branches

- L'état git du projet additionne celui de ses dépôts (branche, avance et retard, dépôt distant, commit, changements) ; sa branche est la leur quand ils la partagent.
- La barre d'état dit « main · 4 dépôts », sinon « 3 branches · 4 dépôts », avec les commits à pousser et à tirer additionnés. Un clic ouvre le menu des dépôts (branche, « 2 à tirer, 1 à pousser » ou « à jour », « HEAD détaché », « aucun commit », « introuvable »), puis le sélecteur de branche du dépôt choisi, le même que pour un projet à un dépôt (voir *Branches*).
- Le Δ de la barre de titre et du pied de la barre latérale donne la somme, et le détail par dépôt en infobulle (« api ~2 +1 −0 · front ~3 +0 −1 · docs sans changement »).

### Panneau Git

- « Non commités » est groupé par dépôt : un en-tête repliable par dépôt (nom, branche, +/−), les chemins depuis le dépôt, « Sans changement : docs ». Hors de cette liste, les chemins partent du dossier du projet (`api/src/users.ts`), et le diff les écrit ainsi.
- « Commit… » direct ouvre une fenêtre avec un message par dépôt changé, chacun proposé à part dans le style de ce dépôt, puis « Commiter les 2 dépôts ». Un dépôt dont le commit échoue le dit ; les autres sont commités.
- « Historique » montre un dépôt à la fois, choisi par un menu en tête (touches 1 à 9), par défaut celui où l'agent a touché des fichiers. Le menu d'un commit et ses comparaisons agissent dans ce dépôt.

### Agents et worktrees

- Sans worktree, un agent travaille à la racine et voit tous les dépôts.
- Avec, il a un dossier à lui sous le dossier des worktrees du projet, avec un worktree de chaque dépôt qui a un commit et une branche, à son chemin, tous sous la même branche ; chaque dépôt garde sa base. Les fichiers de la racine qui ne sont dans aucun dépôt y sont copiés, ainsi que les « Fichiers copiés » de chaque dépôt : un motif `api/.env.local` ne vaut que pour `api`, `.env*` pour tous. Un dépôt laissé de côté est dit dans sa conversation.
- La branche de départ vaut par dépôt : la branche nommée ou la cible là où elle existe, sinon la branche du dépôt (voir *Git & fichiers*).
- Avec le worktree au premier message, tous ses worktrees se font une fois l'agent nommé ; sa carte dit « Worktrees · 2 sur 3 dépôts ». Renommé d'après son premier message, sa branche l'est dans chaque dépôt ou dans aucun.
- Un worktree qui échoue défait les autres.
- Les commandes de lancement et de préparation gardent leur sous-dossier (un dépôt) dans le dossier de l'agent.

### Nettoyage et suppression

Chaque dépôt est jugé à part : un worktree avec des changements non commités, ou des commits ni mergés ni poussés, reste (« pagination (api) : 2 commits… ») ; les autres partent avec leur branche (mergée). Le dossier de l'agent part quand il est vide. Un dépôt sans changement n'a ni merge, ni PR.

### Merger

La fenêtre « Merger <agent> » vérifie d'abord chaque dépôt (merge possible sans conflit, dossier propre, base présente). Rien n'est mergé tant qu'un dépôt changé ne peut pas l'être (« Rien n'est mergé tant que front ne peut pas l'être ») ; « Merger api seul » le fait à la demande, et « L'agent résout le conflit » lui passe le conflit (« Intégrer » de chaque base). Chaque dépôt va dans sa propre base ; une erreur après la vérification dit ce qui est déjà mergé.

### Pull requests

Une PR par dépôt changé, ouvertes depuis une seule fenêtre : titre et description communs, une ligne par dépôt avec son dépôt distant et sa base. Chaque description nomme les autres (« Va avec la branche … de acme/front : à merger ensemble. »). L'en-tête offre « Ouvrir 2 PR » ; la carte et l'en-tête montrent les numéros (« api #43 · front #12 »). Voir *Pull requests*.

### Validation d'un ticket

Ce qui reste est commité dans chaque dépôt, avec un même message, puis l'action du Kanban (merge, PR, push) va dans chaque dépôt que la branche a changé. Pour un merge, tout est vérifié d'abord : un conflit n'importe où bloque le ticket « À tester » (« Conflit avec main dans front : rien n'est mergé ») avec « L'agent résout ». Résultat : « ⤵ api : Mergé dans main · squash · front : Mergé dans main · squash ». Un projet sans branche à lui ne bloque pas le Kanban : chaque dépôt part de la sienne. La carte « À tester » dit « Changés : api, front ». Voir *Validation*.

### Chemins et vues

La carte d'un agent, la vue d'ensemble, la recherche et « Ouvrir un fichier » disent les chemins depuis la racine du projet (`front/src/…`). L'en-tête de la conversation ajoute à la branche les dépôts changés (« · api, front »). L'arbre de l'éditeur montre la branche de chaque dossier de dépôt, et « Voir les changements » compare un fichier à la base de son dépôt.

### Serveur MCP

`list_projects` donne les dépôts d'un projet (chemin et branche) ; `get_agent_summary` et le ticket de `get_ticket`, les dépôts de son worktree (dépôt, branche, base). `approve_ticket` et `resolve_conflict` font la validation et la résolution ci-dessus. Rien ne change pour un projet à un dépôt.

## Réglages

Réglages › Projets, groupe « Dépôts » : la liste des dépôts avec la branche de chacun, les dépôts retirés avec « Oublier », « Rechercher à nouveau » (aussi dans le menu des branches de la barre d'état), et où est `.escouade.yml`.

## Limites

- Un dépôt sans commit n'a pas de worktree.
- Les dépôts ne sont cherchés que sur deux niveaux de sous-dossiers.
