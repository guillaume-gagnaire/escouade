---
title: "Vue « Pull requests » du projet"
subject: "git-fichiers"
sources: ["docs/TODO-1.8.md","design/esc-65/README.md","src/components/prs/PrsView.svelte","src/lib/prlist.ts","src/lib/prnew.ts","src-tauri/src/pragent.rs","src-tauri/src/prview.rs","ESC-403 (demande de l'utilisateur du 2026-11-09, réponse de l'overseer sur la lecture de fond)"]
tickets: ["ESC-65","ESC-66","ESC-68","ESC-383","ESC-390","ESC-403"]
decisions: [60,129,139]
compared: null
---

## Ce que c'est

À côté du Kanban, la liste des pull requests ouvertes du dépôt du projet (« merge requests » sur GitLab), qu'elles soient de l'utilisateur ou d'autres : leur CI, leurs relectures, un conflit et leur âge, trois filtres, « Lancer un agent » sur l'une d'elles, même venue d'un fork, et « Faire relire » par un agent (voir *Relecture d'une PR par un agent*). La liste n'écrit rien chez l'hébergeur : seule la publication d'une relecture y écrit, au clic de l'utilisateur. Le détail d'une PR (checks, journaux, revues, déploiements) reste l'onglet « PR et CI » de l'agent (voir *Pull requests*). L'onglet porte une pastille quand de nouvelles PR arrivent (ci-dessous).

## Comportement

### Où

- Dans la barre latérale du projet, l'onglet « PR » de « Agents · Kanban · PR · Docs » (voir *Barre latérale*). « PR » est le mot de l'hébergeur : « MR » sur GitLab. Infobulle : « Les PR ouvertes du projet ».
- L'onglet n'existe que si le projet a un hébergeur de code : la remote de sa cible (sinon `origin`, sinon la seule) est reconnue par un hébergeur dont le plugin est actif, et cet hébergeur lit des pull requests. Sans lui, la barre a trois onglets, et rien n'ouvre la vue.
- L'hébergeur se relit à l'ouverture du projet, quand une remote apparaît ou disparaît (`git remote add` dans un terminal), quand la fenêtre revient au premier plan, quand un plugin change (activé, coupé, installé) et quand un compte d'hébergeur est connecté ou retiré. Parti pendant que la vue est ouverte, elle laisse place aux agents ; arrivé, l'onglet apparaît.
- La vue prend la place de travail comme le Kanban, et la quitte de même : un agent, un terminal, un lancement, l'éditeur, le Kanban ou Docs choisis.
- En-tête comme celui du Kanban : « Pull requests » (« Merge requests » sur GitLab), dessous le projet et « 30 ouvertes » ; à droite les trois filtres, « Lu à 14:02 » (masqué sous 1100 px) et « Rafraîchir ».

### La liste

- Les 30 PR ouvertes les plus récemment mises à jour du dépôt de la cible, comme l'onglet « PR et CI » du projet, lues via le plugin de l'hébergeur ; à l'affichage de la vue, puis chaque minute tant qu'elle est affichée et la fenêtre visible, et par « Rafraîchir ». Vue fermée, la lecture de fond de la pastille la lit (ci-dessous).
- Une ligne de 34 px par PR, colonnes alignées : le signe de sa CI (gris pour un brouillon), son numéro, son titre et ses marques (« Nouvelle » pour une PR nouvelle, ci-dessous ; « Brouillon », « fork marta » avec en infobulle le dépôt et la branche du fork, l'agent du projet qui y travaille, où en est sa relecture par un agent), puis ses pastilles, sa CI en mots (« CI verte », « CI en échec », « CI en cours », « CI ignorée », « Pas de CI »), son auteur et son âge (« 3 h », « 2 j »). Sous 1150 px, la CI en mots et l'auteur laissent leur place au titre.
- Les pastilles, la plus pressante d'abord : « Conflit », « Changements demandés », « 1 approbation », « 2 fils ouverts », « Ta relecture » (en ambre).
- Filtres, avec leur compte, le dernier choisi gardé sur la machine (« Toutes » au départ) :
  - « Les miennes » : ouvertes par le compte de l'hébergeur avec lequel elles sont lues ;
  - « Relecture demandée » : une relecture est demandée à ce compte et n'est pas encore rendue (GitHub : les personnes, pas les équipes ; GitLab : les relecteurs ; Gitea : les relecteurs demandés ; Bitbucket Cloud : les relecteurs qui n'ont rien dit ; Azure DevOps : ceux qui n'ont pas voté, par leur nom) ;
  - « Toutes ».
  Sans identifiant du compte, une note le dit : « Les miennes » et « Relecture demandée » restent vides.

### Pastille des PR nouvelles (ESC-403)

- **Une PR nouvelle** : ouverte, apparue dans la liste du projet depuis la dernière fois que l'onglet PR a été ouvert, et pas ouverte par Escouade (« Ouvrir une PR » d'un agent, la PR d'un ticket, toute PR d'un agent du projet) : l'utilisateur les a vues naître. Une PR d'Escouade le reste, même son agent ou son ticket parti.
  - Ce qui a été vu est le plus haut numéro lu : à la première lecture du projet, puis à chaque ouverture de l'onglet et pendant qu'il est affiché. Une PR plus ancienne qui remonte dans les 30 dernières mises à jour n'est donc jamais nouvelle ; une PR fermée avant d'être vue ne compte plus.
  - À la première lecture d'un projet (rien de vu encore), aucune ne compte : pas de pastille à 37 au premier lancement.
- **La pastille** : sur l'onglet « PR » (« MR »), le nombre de PR nouvelles, dans la pastille ambre de l'app (celle des tickets « À tester » sur « Kanban »), lisible dans les deux thèmes et à 1024 px ; rien quand il n'y en a pas. Un lecteur d'écran lit l'onglet « PR, 2 nouvelles » (« PR, 1 nouvelle »). Le projet dans la barre des projets n'en porte pas.
- **Ouvrir l'onglet** (ou la vue venue à l'écran par un autre chemin) remet la pastille à zéro. Les PR nouvelles portent alors « Nouvelle » dans la liste (infobulle « Arrivée depuis ta dernière visite »), comme celles qui arrivent pendant que la vue est affichée, jusqu'à la prochaine ouverture de l'onglet.
- **Survit au redémarrage**, par projet, dans l'état de l'interface : ce qui a été vu, la dernière liste lue, les PR d'Escouade et celles marquées « Nouvelle ».
- **Ce qui la nourrit** : toute lecture de la liste, quelle qu'en soit la source (la vue, l'onglet « PR et CI », la vue d'ensemble, les outils, la lecture de fond) ; une liste reprise sans être relue ne compte pas.
- **La lecture de fond** (décision de l'overseer du 2026-11-09, sur ESC-403) : la liste du projet à l'écran seulement, fenêtre visible seulement, onglet PR fermé compris ; toutes les 2 minutes, jamais moins de 2 minutes après une autre lecture de cette liste (la vue, l'onglet « PR et CI », la vue d'ensemble, les outils) ; pas quand la limite d'appels de l'hébergeur approche. Après un échec (limite de débit, hébergeur injoignable, jeton refusé), l'attente double à chaque échec de suite, jusqu'à 30 minutes, sans toast ni notification : la vue dit l'échec quand on l'ouvre. Une lecture réussie, d'où qu'elle vienne, remet l'attente à 2 minutes.

### Agir

- La ligne survolée ou choisie montre ses actions par-dessus sa fin : « Faire relire » (ou « Voir la relecture » quand elle en a une), « Lancer un agent » (ou « Ouvrir l'agent » quand un agent du projet y travaille déjà) et l'ouverture dans le navigateur.
- Au clavier : ↑ ↓, Début, Fin pour choisir la ligne ; Entrée lance l'agent (ou ouvre celui qui y est). Un double clic aussi.
- « Lancer un agent » : la PR est relue chez l'hébergeur (« La PR #99 n'est plus ouverte. » sinon), sa branche récupérée, puis un agent naît dans un worktree sur elle, comme « Nouvel agent sur une branche… », mergé dans la base de la PR quand cette branche est là, et il est ouvert :
  - PR du dépôt : `git fetch <remote> +refs/heads/<branche>:refs/remotes/<remote>/<branche>`, puis la branche locale du même nom si elle existe, sinon une branche locale qui suit celle du remote ;
  - PR d'un fork : la référence que l'hébergeur garde, dans une branche `pr-<n>` (`mr-<n>` sur GitLab) : `refs/pull/<n>/head` (GitHub, Gitea, Forgejo), `refs/merge-requests/<n>/head` (GitLab), le second parent de `refs/pull/<n>/merge` (Azure DevOps), la branche du fork lui-même sur la même instance (Bitbucket Cloud, qui ne garde pas de référence). Une branche `pr-<n>` déjà là ne fait qu'avancer : celle qui a des commits à elle (d'un agent d'avant, de l'utilisateur) est gardée telle quelle, jamais écrasée ; celle qu'un worktree a prise n'est pas récupérée à nouveau, et l'agent est refusé comme pour toute branche prise.
  - Les refus d'un agent sur une branche valent (branche du dossier du projet, branche d'un autre worktree), dits dans un toast. Rien n'est poussé ni publié.
- Un agent de la vue se reconnaît dans la liste : une PR qu'il a lue, sa branche, ou pour un fork sa branche `pr-<n>`.
- « Faire relire » : la vue laisse place à la relecture de la PR, jusqu'à « Revenir aux PR » (voir *Relecture d'une PR par un agent*).

### États

- Chargement : « Lecture des PR ouvertes du projet… ».
- Vide, par filtre : « Aucune PR ouverte. », « Aucune PR ouverte par toi. », « Personne ne t'a demandé de relecture. », avec « Toutes les PR » pour en sortir.
- Erreur : le cadre rouge et les mots de l'onglet « PR et CI » (jeton refusé, limite d'API, hébergeur injoignable, sinon ceux de l'hébergeur), avec « Réessayer » et, quand c'est là que ça se règle, « Réglages › Services connectés », qui ouvre cette page des réglages (voir *Réglages*). Une relecture qui échoue garde la liste affichée et dit pourquoi au-dessus.

## Limites

- Essai réel sur GitHub à faire avec le compte de l'utilisateur ; GitLab, Bitbucket Cloud, Gitea/Forgejo et Azure DevOps suivent leur documentation et restent à essayer en vrai.
- La carte d'un agent lancé sur une PR de fork ne montre pas cette PR : sa branche `pr-<n>` n'est pas celle du fork.
- 30 PR au plus, les plus récemment mises à jour.
- Une remote dont seule l'adresse change hors d'Escouade (`git remote set-url`) se voit au retour de la fenêtre au premier plan, pas avant.
- La pastille ne suit que le projet à l'écran : les PR arrivées sur un autre projet se comptent à la prochaine lecture de sa liste (en l'affichant, ou par la vue d'ensemble).
- La lecture de fond n'est pas conditionnelle (ETag / `If-None-Match`) : la requête chez l'hébergeur est faite par son plugin. Le plugin GitHub lit la liste en une requête GraphQL, que GitHub ne sert pas en conditionnel ; ceux de GitLab, Gitea/Forgejo, Bitbucket Cloud et Azure DevOps ne gardent pas d'ETag pour la liste aujourd'hui (une version de chacun le pourrait).

- Chaque lecture de la liste est gardée : la vue d'ensemble et `list_pull_requests` la reprennent pendant une minute, et une relecture nouvellement demandée à l'utilisateur se notifie (voir *Notifications*).
