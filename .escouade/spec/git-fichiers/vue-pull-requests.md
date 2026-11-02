---
title: "Vue « Pull requests » du projet"
subject: "git-fichiers"
sources: ["docs/TODO-1.8.md","design/esc-65/README.md","src/components/prs/PrsView.svelte","src/lib/prlist.ts","src-tauri/src/pragent.rs","src-tauri/src/prview.rs"]
tickets: ["ESC-65"]
decisions: [60]
compared: null
---

## Ce que c'est

À côté du Kanban, la liste des pull requests ouvertes du dépôt du projet (« merge requests » sur GitLab), qu'elles soient de l'utilisateur ou d'autres : leur CI, leurs relectures, un conflit et leur âge, trois filtres, et « Lancer un agent » sur l'une d'elles, même venue d'un fork. En lecture : Escouade n'écrit rien chez l'hébergeur. Le détail d'une PR (checks, journaux, revues, déploiements) reste l'onglet « PR et CI » de l'agent (voir *Pull requests*).

## Comportement

### Où

- Dans la barre latérale du projet, le sélecteur devient « Agents | Kanban | PR | Docs ». « PR » est le mot de l'hébergeur : « MR » sur GitLab, une fois la liste lue ou d'après un agent du projet. Infobulle : « Les PR ouvertes du projet ».
- La vue prend la place de travail comme le Kanban, et la quitte de même : un agent, un terminal, un lancement, l'éditeur, le Kanban ou Docs choisis.
- En-tête comme celui du Kanban : « Pull requests » (« Merge requests » sur GitLab), dessous le projet et « 30 ouvertes » ; à droite les trois filtres, « Lu à 14:02 » (masqué sous 1100 px) et « Rafraîchir ».

### La liste

- Les 30 PR ouvertes les plus récemment mises à jour du dépôt de la cible, comme l'onglet « PR et CI » du projet, lues via le plugin de l'hébergeur ; à l'affichage de la vue, puis chaque minute tant qu'elle est affichée et la fenêtre visible, et par « Rafraîchir ».
- Une ligne de 34 px par PR, colonnes alignées : le signe de sa CI (gris pour un brouillon), son numéro, son titre et ses marques (« Brouillon », « fork marta » avec en infobulle le dépôt et la branche du fork, l'agent du projet qui y travaille), puis ses pastilles, sa CI en mots (« CI verte », « CI en échec », « CI en cours », « CI ignorée », « Pas de CI »), son auteur et son âge (« 3 h », « 2 j »). Sous 1150 px, la CI en mots et l'auteur laissent leur place au titre.
- Les pastilles, la plus pressante d'abord : « Conflit », « Changements demandés », « 1 approbation », « 2 fils ouverts », « Ta relecture » (en ambre).
- Filtres, avec leur compte, le dernier choisi gardé sur la machine (« Toutes » au départ) :
  - « Les miennes » : ouvertes par le compte de l'hébergeur avec lequel elles sont lues ;
  - « Relecture demandée » : une relecture est demandée à ce compte et n'est pas encore rendue (GitHub : les personnes, pas les équipes ; GitLab : les relecteurs ; Gitea : les relecteurs demandés ; Bitbucket Cloud : les relecteurs qui n'ont rien dit ; Azure DevOps : ceux qui n'ont pas voté, par leur nom) ;
  - « Toutes ».
  Sans identifiant du compte, une note le dit : « Les miennes » et « Relecture demandée » restent vides.

### Agir

- La ligne survolée ou choisie montre ses actions par-dessus sa fin : « Lancer un agent » (ou « Ouvrir l'agent » quand un agent du projet y travaille déjà) et l'ouverture dans le navigateur.
- Au clavier : ↑ ↓, Début, Fin pour choisir la ligne ; Entrée lance l'agent (ou ouvre celui qui y est). Un double clic aussi.
- « Lancer un agent » : la PR est relue chez l'hébergeur (« La PR #99 n'est plus ouverte. » sinon), sa branche récupérée, puis un agent naît dans un worktree sur elle, comme « Nouvel agent sur une branche… », mergé dans la base de la PR quand cette branche est là, et il est ouvert :
  - PR du dépôt : `git fetch <remote> +refs/heads/<branche>:refs/remotes/<remote>/<branche>`, puis la branche locale du même nom si elle existe, sinon une branche locale qui suit celle du remote ;
  - PR d'un fork : la référence que l'hébergeur garde, dans une branche `pr-<n>` (`mr-<n>` sur GitLab) : `refs/pull/<n>/head` (GitHub, Gitea, Forgejo), `refs/merge-requests/<n>/head` (GitLab), le second parent de `refs/pull/<n>/merge` (Azure DevOps), la branche du fork lui-même sur la même instance (Bitbucket Cloud, qui ne garde pas de référence). Une branche `pr-<n>` déjà là ne fait qu'avancer : celle qui a des commits à elle (d'un agent d'avant, de l'utilisateur) est gardée telle quelle, jamais écrasée ; celle qu'un worktree a prise n'est pas récupérée à nouveau, et l'agent est refusé comme pour toute branche prise.
  - Les refus d'un agent sur une branche valent (branche du dossier du projet, branche d'un autre worktree), dits dans un toast. Rien n'est poussé ni publié.
- Un agent de la vue se reconnaît dans la liste : une PR qu'il a lue, sa branche, ou pour un fork sa branche `pr-<n>`.

### États

- Chargement : « Lecture des PR ouvertes du projet… ».
- Vide, par filtre : « Aucune PR ouverte. », « Aucune PR ouverte par toi. », « Personne ne t'a demandé de relecture. », avec « Toutes les PR » pour en sortir.
- Erreur : le cadre rouge et les mots de l'onglet « PR et CI » (jeton refusé, limite d'API, hébergeur injoignable, sinon ceux de l'hébergeur), avec « Réessayer » et, quand c'est là que ça se règle, « Réglages › Intégrations ». Une relecture qui échoue garde la liste affichée et dit pourquoi au-dessus.

## Limites

- Essai réel sur GitHub à faire avec le compte de l'utilisateur ; GitLab, Bitbucket Cloud, Gitea/Forgejo et Azure DevOps suivent leur documentation et restent à essayer en vrai.
- La carte d'un agent lancé sur une PR de fork ne montre pas cette PR : sa branche `pr-<n>` n'est pas celle du fork.
- « Faire relire par un agent » n'est pas fait (ticket suivant).
- 30 PR au plus, les plus récemment mises à jour.
