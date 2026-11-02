---
title: "Environnements et déploiements"
subject: "kanban"
sources: ["docs/TODO-1.8.md (chapitre 2, « Les déploiements »)","design/esc-67/README.md"]
tickets: ["ESC-67"]
decisions: [61]
compared: null
---

## Ce que c'est

Après le merge, Escouade suit les déploiements du dépôt chez son hébergeur : les déploiements de GitHub, les environnements de GitLab, les déploiements de Bitbucket Pipelines. Gitea, Forgejo et Azure DevOps n'ont pas d'API de déploiements : Escouade n'y suit que la CI. La carte d'un ticket terminé dit dans quels environnements il tourne, la vue « Environnements » du Kanban dit ce qui tourne où, et un aperçu posé sur une PR s'ouvre depuis la carte « En revue ». Rien n'est publié chez l'hébergeur : Escouade ne fait que lire.

## Comportement

### Un ticket dans un environnement

- Un ticket terminé et mergé a le commit de son merge : celui qu'Escouade a fait (« Valider et merger »), ou celui de sa PR mergée chez l'hébergeur (commit de merge, de squash, ou dernier commit rebasé), quand l'hébergeur le dit (GitHub, GitLab, Bitbucket).
- Ce qui tourne dans un environnement est son dernier déploiement réussi. Le ticket y est quand ce commit contient celui de son merge (`git merge-base --is-ancestor`). Un commit déployé absent du dépôt local est d'abord rapporté par un fetch ; sans fetch possible, le ticket n'y est pas compté. Un commit qu'un fetch n'a pas rapporté (sa branche supprimée depuis) ne déclenche plus de fetch : seul un nouveau commit déployé en relance un.
- Un ticket est suivi 30 jours après être passé « Terminé ». Au-delà, sa carte garde ce qui a été lu en dernier.
- Les aperçus de PR et les review apps ne sont pas des environnements : « Preview » de Vercel, `review/<branche>` de GitLab, `deploy-preview-12`, `pr-12`, et chez GitHub tout environnement transitoire.
- Production se reconnaît à son nom : `production`, `prod`, `prod-…` (« Production – web » compris). Elle vient en dernier ; les autres environnements sont rangés par leur nom.

### La carte d'un ticket terminé

- Sous ce que la validation a fait : « staging ✓ · production ⏳ », vert pour un environnement où il tourne, ambre pour un où il n'est pas encore.
- Chaque nom ouvre l'adresse de l'environnement dans le navigateur quand l'hébergeur la donne (GitHub, GitLab ; Bitbucket n'en a pas) ; son infobulle la montre.
- Rien n'est affiché tant qu'aucun environnement n'est connu.

### La vue « Environnements »

- Elle s'ouvre par « Tickets · Environnements » dans l'en-tête du Kanban et prend la place des colonnes ; son titre devient « Environnements ».
- Une ligne : le projet, l'hébergeur, « lu il y a … » ; « N tickets pas encore en production » quand le dépôt a une production ; « Rafraîchir », qui relit tout de suite.
- Un couloir par environnement, dans l'ordre où le code avance : d'abord « Mergés, pas encore déployés », puis chaque environnement, production en dernier (direction A, choisie sur aperçu le 2026-11-02).
- En tête de chaque environnement : ce qui y tourne (commit court, il y a combien de temps, par qui), son adresse cliquable, et un déploiement en cours (« Déploiement en cours · <commit> »).
- Sous la tête, les tickets qui y sont arrivés, chacun dans le dernier environnement qu'il a atteint, le plus récent en haut ; huit au plus, puis « + N autres ». Un clic ouvre sa PR ou son commit.
- Le dépôt n'a aucun déploiement : la vue le dit. L'hébergeur n'en a pas d'API : « Gitea n'a pas d'API de déploiements : Escouade ne suit que la CI de la branche cible. » Une lecture qui échoue est dite, la dernière lue reste affichée.

### Un déploiement en échec

- Le dernier déploiement d'un environnement a échoué : son couloir est bordé de rouge, avec « Dernier déploiement en échec · <commit> » et « Voir le job », qui ouvre son journal chez l'hébergeur. Ce qui y tourne reste le dernier déploiement réussi.
- Une notification le dit une fois par déploiement (« Déploiement en échec : production (<commit>) ») ; un clic ouvre le Kanban du projet. Ce qui avait déjà échoué à la première lecture n'est pas notifié. Elle suit le réglage des notifications des tickets.

### Les aperçus

- Vercel, Netlify, Cloudflare Pages… posent sur la PR un déploiement ou un check avec une adresse. Sur la carte « En revue », « Ouvrir l'aperçu » l'ouvre dans le navigateur.
- L'aperçu est un déploiement réussi de la branche de la PR qui a une adresse (un aperçu d'abord, production en dernier), sinon un check réussi d'un hébergeur d'aperçus (Vercel, Netlify, Cloudflare, Pages, Render, Surge, Amplify, « preview ») qui a une adresse. Une PR venue d'un fork n'en a pas.

### Le rafraîchissement

- Pas de webhook. Les environnements se lisent dans la même boucle que les PR des tickets « En revue » : toutes les minutes fenêtre devant, toutes les 5 minutes sinon, et tout de suite quand la fenêtre revient. L'aperçu se lit avec la PR, d'un appel de plus.
- Seuls les projets suivis sont lus : ceux qui ont un ticket mergé depuis moins de 30 jours, et celui dont la vue est ouverte (pendant 15 minutes après qu'elle s'est ouverte, tenue tant qu'elle reste ouverte).
- GitHub : les 30 derniers déploiements, puis le dernier statut de chacun jusqu'au dernier succès de chaque environnement, 12 statuts au plus par lecture. GitLab : les 50 derniers déploiements du projet, en un appel. Bitbucket : les environnements, puis les déploiements, 3 pages au plus.
- La réponse « ce commit contient-il celui-là » est gardée : un environnement qui ne change pas ne relance pas git.

## Ce qui n'est pas fait

- Les essais réels : seulement contre les faux serveurs des tests. GitHub est à essayer avec le compte de l'utilisateur ; GitLab et Bitbucket Cloud d'après leur documentation (voir `docs/TODO-1.8.md`).
- Gitea et Forgejo : la CI de la cible n'a pas encore de vue à elle.
- Relancer ou approuver un déploiement depuis Escouade.
