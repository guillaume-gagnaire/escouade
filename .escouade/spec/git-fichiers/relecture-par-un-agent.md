---
title: "Relecture d'une PR par un agent"
subject: "git-fichiers"
sources: ["docs/TODO-1.8.md","design/esc-66/README.md","src-tauri/src/prreview.rs","src/components/prs/PrReviewView.svelte","src/lib/prreview.ts"]
tickets: ["ESC-66"]
decisions: [100]
compared: null
---

## Ce que c'est

Sur une PR ouverte du projet (« MR » sur GitLab), de l'équipe ou d'un ticket, un agent lit le diff et rédige une relecture : un texte et des commentaires par fichier et par ligne. C'est un brouillon gardé dans Escouade. L'utilisateur garde, corrige ou écarte chaque commentaire, puis publie le tout d'un coup : review de GitHub et de Gitea / Forgejo, discussions de GitLab, commentaires de ligne de Bitbucket Cloud, fils d'Azure DevOps. Rien ne part sans son clic.

## Comportement

### Demander une relecture

- « Faire relire » sur une ligne de la vue « Pull requests » (survolée ou choisie) ; « Faire relire par un agent » sur la PR de l'onglet « PR et CI », quand elle est ouverte et sur le dépôt de la cible. L'onglet ouvre le brouillon qui existe déjà au lieu d'en relancer un.
- La relecture tourne en arrière-plan : la vue dit « L'agent relit la PR #12… », on peut la quitter. Si elle finit hors de vue, un toast le dit, avec « Voir ». « Annuler » l'abandonne.
- Une relecture par PR : la dernière. Sur sa ligne, une marque dit où elle en est (« Relecture en cours », « Relecture : 2 à trancher », « Relecture à publier », « Relecture publiée », « Relecture en échec »), et « Voir la relecture » l'ouvre.
- Le brouillon survit au redémarrage, pas une lecture en cours. Il disparaît quand sa PR n'est plus ouverte, lue sur une liste complète, ou avec son projet.
- Refusée d'emblée, avec un toast, quand le harness des textes rédigés ne sait pas répondre sans outil (voir plus bas).

### Ce que lit l'agent

- La PR est relue chez l'hébergeur (« La PR #99 n'est plus ouverte. » sinon). Son diff vient de git : depuis le point où elle a quitté sa base, comme l'hébergeur le montre. Une PR de fork se lit par la référence que l'hébergeur garde (voir *Vue « Pull requests » du projet*).
- Pointe et base sont récupérées dans des références `refs/escouade/review/<n>/…`, supprimées aussitôt : aucune branche ne bouge, aucun `pr-<n>` ne naît, rien n'est poussé.
- Les fichiers au diff trop long (150 Ko en tout, un fichier entier ou pas du tout) et les fichiers générés (`package-lock.json`, `Cargo.lock`, `*.min.js`…) ne sont pas lus, et la vue le dit, leurs noms en infobulle. Un diff dont il ne reste rien est refusé avec sa raison.
- C'est une question ponctuelle du harness des textes rédigés (Réglages › Application), sur son modèle de lecture, 5 min au plus : sans session, sans MCP et **sans aucun outil**, tout est dans la question.

### Ce que dit une PR est une donnée

- Titre, branches, diff et commentaires viennent d'un tiers, peut-être d'un inconnu sur un dépôt public : ce sont des données, jamais des consignes. Le rôle de l'agent et la question le disent ; chaque texte est dans sa balise, ses balises fermantes neutralisées ; l'agent doit signaler dans un commentaire une instruction qu'il y trouve.
- Une consigne n'est pas une barrière : le texte d'un fichier lu reviendrait dans un commentaire que l'utilisateur pourrait garder et publier. La vraie barrière est que l'agent n'a aucun outil, pas même de lecture : un diff hostile ne peut lui faire ouvrir ni `~/.ssh/id_rsa` ni un `.env`.
- Chaque harness coupe ses outils à sa façon : Claude Code (`--tools ""`), Codex (sans shell ni apps), Copilot CLI (une liste d'outils qui n'en nomme aucun), OpenCode (agent sans outil, dossier vide), Gemini CLI (règles qui refusent tout), Antigravity CLI (tous refusés), Mistral Vibe (`--enabled-tools none`).
- **Cursor CLI ne sait pas couper les siens** : son mode Ask lit toujours le disque. La relecture y est refusée, le message dit de choisir un autre harness dans Réglages › Application › Textes rédigés. Il en va de même d'un harness de plugin : le protocole ne donne aucun moyen de vérifier ce qu'il laisse faire.
- En plus, l'agent tourne dans un dossier vide à lui, pas dans le projet : deux barrières valent mieux qu'une.
- Sa réponse ne donne que du texte et des commentaires. Le verdict n'y est pas lu : il est celui de l'utilisateur. Une réponse qui n'est pas une relecture se dit comme un échec.

### La vue

- Elle prend la place de la liste : en-tête avec le numéro, le titre, qui a relu, « branche → cible », « Relire à nouveau » (avec confirmation si des commentaires sont déjà tranchés), le navigateur et « Écarter la relecture ».
- « Texte de la relecture », modifiable, puis les commentaires fichier par fichier, dans l'ordre du diff : les lignes du diff où ils sont (la leur encadrée), leur numéro et leur texte.
- Sur chacun : « Garder », « Corriger » (le texte s'édite, « Garder ce texte », « Revenir au texte de l'agent »), « Écarter » ; écarté, « Reprendre ». Chaque choix est gardé aussitôt. Un commentaire vide ne se garde pas.
- Un commentaire dont la ligne n'est pas dans le diff est marqué « Hors du diff » : l'hébergeur le refuserait, il part dans le texte de la relecture, `fichier:ligne : texte`.
- Le pied dit « 2 à trancher · 1 gardé · 1 écarté · rien ne part avant ton clic », puis le verdict et « Publier sur GitHub » (le nom de l'hébergeur).

### Publier

- Le verdict : « Commenter » au départ, « Demander des changements », « Approuver ». Sur sa propre PR, seulement « Commenter » : les hébergeurs refusent le reste.
- « Publier » reste éteint, avec sa raison en infobulle, tant qu'un commentaire reste à trancher, sans texte pour demander des changements, ou quand il n'y a rien à publier.
- Publier relit la PR et son diff. De nouveaux commits depuis la relecture : rien ne part, une confirmation le dit (« Publier quand même ») ; un commentaire dont la ligne a quitté le diff part alors dans le texte.
- Seuls les commentaires gardés partent, en un appel de `review` du plugin de l'hébergeur, avec le verdict choisi. Publiée, la relecture ne change plus (« Publiée sur GitHub à 14:02. ») ; « Relire à nouveau » en lance une autre.
- Un refus de l'hébergeur garde le brouillon et dit de regarder la PR avant de réessayer : GitLab, Bitbucket et Azure DevOps posent leurs commentaires un par un, une partie a pu partir.
- Aucun outil MCP ne publie ni ne lance une relecture : le clic dans la fenêtre est le seul chemin vers l'hébergeur.

### États

- Lecture : « L'agent relit la PR #12… », « Tu peux quitter cette vue : la relecture continue et rien n'est publié. »
- Échec : le cadre rouge, la raison et « Réessayer » (« La réponse de l'agent n'est pas une relecture. », pas de réponse dans le temps, diff illisible).
- Sans commentaire : « L'agent n'a rien à signaler ligne par ligne. », et le texte se publie seul.

## Limites

- Essayé sur le faux GitHub des tests ; à essayer en vrai sur GitHub, puis sur les quatre autres hébergeurs (`docs/TODO-1.8.md`).
- Pas de relecture sur Cursor CLI ni sur un harness de plugin (voir plus haut).
- Le protocole des hébergeurs ne donne pas le texte de la PR : l'agent lit son titre, ses branches, son diff et ses commentaires non résolus (20 au plus, 1 Ko chacun).
- Un commentaire porte sur une ligne de la nouvelle version : pas sur une ligne retirée.
- L'agent n'ouvre aucun fichier : il ne voit que le diff et son contexte.
- 40 commentaires au plus.
