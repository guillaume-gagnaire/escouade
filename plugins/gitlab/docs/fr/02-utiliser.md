# Utiliser

Le plugin sert deux choses d'Escouade sur le même compte : l'hébergeur de code GitLab (les merge requests, leurs pipelines, leurs discussions, leurs déploiements) et la source GitLab Issues du Kanban. Partout dans l'app, une pull request s'appelle « MR » sur GitLab.

## Les merge requests

**Ouvrir une MR.** « Ouvrir une MR » sur un agent, ou la validation d'un ticket avec « Ouvrir une pull request » comme action, fait pousser la branche par Escouade puis demande au plugin d'ouvrir la MR : son titre, sa description, sa branche et sa cible, ses relecteurs (leurs logins, que le plugin traduit en identifiants GitLab) et ses étiquettes. Une MR en brouillon a un titre qui commence par `Draft:`. Quand elle ferme une issue, la description finit par « Closes #12 ».

**La suivre.** La carte de l'agent et l'onglet de sa MR montrent son état, brouillon ou non, ses conflits, ses approbations, ses fils ouverts, son auto-merge et l'état de son pipeline. Les projets sont nommés par leur chemin, groupes et sous-groupes compris (`gitlab-org/api/client-go`).

**Les pipelines.** Les jobs du pipeline de tête de la MR (un job autorisé à échouer ne fait pas échouer le pipeline), le journal d'un job, que « L'agent corrige » donne à l'agent, et la relance des jobs en échec du pipeline.

**Merger.** Merge commit ou squash, selon ce que le réglage de squash du projet permet ; c'est le réglage du projet qui dit si GitLab rebase d'abord. Le rebase et le merge semi-linéaire ne se demandent pas depuis Escouade. « Merger quand c'est vert » demande l'auto-merge de GitLab, qui merge quand le pipeline passe.

**Les relectures.** Le plugin lit les approbations, l'avis des relecteurs et les discussions (sur une ligne du diff ou non, sans les notes système). Il répond dans une discussion, la résout, commente, et publie la relecture que tu as validée : ses lignes comme discussions sur le diff, son texte, et ton approbation.

**Les déploiements.** Pour une branche, le dernier déploiement de chaque environnement : la carte d'un ticket terminé dit où il tourne, et la vue « Environnements » montre un couloir par environnement.

**La vue « Pull requests ».** Elle montre les 30 MR ouvertes du projet les plus récemment mises à jour, de toi ou d'autres.

## GitLab Issues

Chaque compte GitLab est aussi une source du Kanban.

**Importer.** Dans la fenêtre « Importer des tickets », l'onglet GitLab montre les issues ouvertes du projet lié, 50 à la fois, avec leur nombre total :

- « Rechercher une issue (#12) ou par texte… » : un numéro (`12` ou `#12`) trouve l'issue ; un autre texte la cherche dans le titre et la description ;
- « Assignées à moi », puis une puce par étiquette du projet (douze au plus) ;
- chaque ligne montre le type (Issue, Incident, « Tâche »), l'auteur (« par @ada »), la personne assignée (« Non assignée » sinon) et les étiquettes.

**Les critères.** Avec « Extraire les critères d'acceptation », ce sont les éléments de liste sous un titre ou une ligne qui nomme les critères, sinon les cases à cocher (`- [ ]`) de la description.

**L'issue suit son ticket.** Dans la « Correspondance des statuts », les états d'une issue sont « Ouverte », « Fermée », puis les étiquettes des listes des tableaux du projet (« Doing », « To review »…), puis ses autres étiquettes. « Fermée » ferme l'issue ; une étiquette la rouvre si besoin, l'ajoute et retire les autres étiquettes de la correspondance, en un seul appel. Le commentaire est une note de l'issue, en Markdown.

**L'import automatique** prend les issues ouvertes qui portent l'étiquette choisie (`claude-ready` par défaut).

## Les limites

- Écrit d'après la documentation de GitLab et essayé contre des réponses enregistrées sur gitlab.com : pas encore contre une vraie instance avec un jeton.
- La liste des projets de GitLab Issues propose les 100 projets dont tu es membre, les derniers actifs d'abord, sans les archivés.
- Le plugin ménage la limite de l'API : il ne lit les fils ouverts que si GitLab dit qu'il y en a, ralentit quand la limite approche, et attend ce que GitLab demande après un refus (429), 30 secondes au plus, avant de redemander une fois.
- Il ne relance pas un déploiement et n'en approuve pas.
- Une réponse de GitLab ne peut pas dépasser 8 Mio ni prendre plus de 15 secondes.
