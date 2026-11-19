---
number: 221
id: "ta-relecture-une-relecture-en-attente-su"
title: "« Ta relecture » : une relecture en attente sur le poste, publiée en une fois par forge/review, et renvoyable à l'agent du ticket même en commentaire"
date: "2026-11-17"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-494"
pages: ["git-fichiers/pull-requests","git-fichiers/relecture-par-un-agent","git-fichiers/vue-pull-requests","kanban/en-revue","git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-494"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/pull-requests","added":21,"removed":2},{"page":"git-fichiers/relecture-par-un-agent","added":3,"removed":0},{"page":"git-fichiers/vue-pull-requests","added":1,"removed":0},{"page":"kanban/en-revue","added":4,"removed":0},{"page":"git-fichiers/hebergeurs-de-code","added":7,"removed":0}]
accepted_at: "2026-11-19T19:57:28+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

PR-7 des PR dans Escouade (ESC-494, direction A de design/pr-2-3) : la relecture de l'utilisateur dans la vue d'une PR, avec un texte, ses remarques en attente et un verdict, les changements demandés revenant à l'agent du ticket. Plusieurs choix n'étaient pas dits : où vit la relecture en attente, par quelle opération elle part, ce que devient une remarque dont un commit retire la ligne, ce qu'on fait d'un 422 de GitHub qui ne dit pas quelle remarque, et comment l'agent d'un ticket la reçoit alors que la PR d'un ticket est ouverte sous le compte de l'utilisateur, où GitHub refuse « Demander des changements ».

## Décision

1. La relecture en attente (texte et remarques) est gardée par PR sur le poste, dans l'état durable (`PersistedState::pending_reviews`, chemin atomique d'ESC-469) ; elle ne part qu'au clic sur « Publier », par la seule commande `pr_review_submit`.
2. Elle part en une seule relecture de l'hébergeur par `forge/review` (l'opération que publiait la relecture d'un agent avant PR-5), étendue sans casser les plugins : `ReviewLine.startLine` et `side`, `ReviewDraft.commit` (le commit lu), protocole et kit 0.1.3. Les capacités `Writing::review`, `approve` et `requestChanges` disent ce que l'hébergeur offre : GitHub, Gitea/Forgejo, Bitbucket et Azure DevOps les trois verdicts, GitLab sans `requestChanges`.
3. Sur sa propre PR (son auteur est le compte avec lequel l'hébergeur est lu, la règle de « Les miennes »), seul « Commenter » est offert, la raison dite, et le cœur refuse le reste.
4. Une remarque suit son code (le texte de ses lignes gardé à l'écriture) ; quand un commit retire sa ligne, elle est « dépassée » et part dans le texte de la relecture, sa place devant (« `fichier:ligne` : … »), pour rester une seule relecture ; ou l'utilisateur la retire.
5. Un 422 d'une ligne que GitHub n'a plus (`Line could not be resolved`, sans dire laquelle) relit le diff : la remarque qui a perdu sa ligne est dite dépassée ; tout reste en attente. Le kit lit désormais une liste de textes dans `errors`.
6. « Renvoyer aussi à l'agent du ticket » passe par `ticket_pr_fix_reviews` (ESC-61), le texte et les remarques de l'utilisateur donnés comme ses mots, hors des balises de tiers, et pas redonnés parmi les retours de l'hébergeur. La case est offerte pour tout verdict qui n'approuve pas, cochée d'office pour « Demander des changements » et, sur sa propre PR, pour « Commenter » : la PR d'un ticket étant ouverte sous le compte de l'utilisateur, c'est le seul moyen d'y renvoyer des changements.
7. Ctrl+Entrée dans la saisie d'une ligne commente aussitôt tant qu'aucune remarque n'attend, puis ajoute à la relecture (comme GitHub).
8. Une remarque de l'agent relecteur ne rejoint pas « Ta relecture » : elle se publie seule (PR-5).

## Conséquences

La relecture en attente n'est que sur ce poste (pas celle commencée sur GitHub, pas partagée). Les autres hébergeurs n'ont qu'un texte et un verdict tant qu'ils ne commentent pas une ligne depuis Escouade, et leurs verdicts restent à essayer en vrai (PR-9). Le parcours de parité de GitHub compte `forge/review` parmi ses changements depuis le portage. Le libellé « Commenter » de la saisie d'une ligne devient « Commenter maintenant » quand l'hébergeur publie une relecture.
