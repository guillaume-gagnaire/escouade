---
title: "Nouveau projet"
subject: "Projets"
sources: ["docs/SPEC.md"]
tickets: ["ESC-94","ESC-167","ESC-168","ESC-142","ESC-163","ESC-89"]
decisions: []
compared: null
---

## Ce que c'est

L'assistant « Nouveau projet » ajoute un projet en réglant, étape par étape, tout ce qui fait marcher la méthode de travail d'Escouade dès le premier ticket : agents en worktree, tickets du Kanban, lancements, validation. Il sert aussi à reconfigurer un projet ouvert, et à rouvrir un projet fermé avec son historique. Sa forme et son contenu ont été choisis sur aperçu (`design/new-project-wizard/`).

## Comportement

### La fenêtre

- 840 px de large, hauteur fixe. Les étapes sont listées à gauche : leur numéro, ✓ une fois passée par « Suivant », – et « passé » par « Passer », « facultatif » sur Intégrations. Un clic sur une étape y va, une fois le dossier lu. « Étape n sur N » au-dessus de l'étape.
- Pied : « Créer sans l'assistant » à gauche (projet neuf seulement) ; « Annuler » à la première étape, sinon « Retour » ; « Passer » sur les étapes de Worktrees à Intégrations (les valeurs restent telles qu'elles sont : copiées, recommandées ou par défaut) ; « Suivant », puis « Créer le projet » au récapitulatif.
- Les étapes, dans l'ordre : Départ, Réglages copiés (seulement en copie), Worktrees, Lancement, Kanban, Compte et commit, Intégrations (facultatif), Récapitulatif.

### Départ

- Le dossier (et « Parcourir… »), la détection de git (sans dépôt, `git init` à la création), le nom, l'aperçu de la tuile, 14 couleurs, « Lettres de la tuile », « Logo » et « Libellé sous la tuile » (voir *Barre des projets*).
- Le point de départ :
  - « Réglages de .escouade.yml », quand le dossier en a un, choisi par défaut (voir *.escouade.yml*) ;
  - « Copier la configuration d'un projet » : la liste des projets ouverts, celui affiché d'abord ; absent s'il n'y en a pas. Jamais un projet privé : sous les choix, « Un projet privé n’est pas proposé : rien d’un projet privé ne passe à un autre. » (ou « N projets privés… »), même quand il ne reste rien à copier ;
  - « Configuration recommandée », choisie par défaut sans `.escouade.yml` : un worktree par agent, validation par merge squash dans la branche courante (cible vide), worktree supprimé une fois validé, commits conventionnels, l'agent résout les conflits, commits rédigés par l'agent ; le reste par défaut ;
  - « Partir de zéro » : les défauts d'Escouade, ceux d'un projet créé sans l'assistant.
- Les réglages sont faits d'après le point de départ en quittant Départ ou Réglages copiés, et seulement si lui (ou ce qui est coché) a changé depuis : un aller-retour n'écrase pas ce qui a été modifié dans les étapes suivantes.

### Réglages copiés

- Les réglages du projet copié, par groupe, chacun avec sa case, sa valeur et ce que la vérification a trouvé ; une case par groupe (cochée, mixte ou décochée) :
  - « Worktrees » : un worktree par agent, dossier des worktrees, préfixes des branches, fichiers copiés, chaque commande de préparation et de démontage ;
  - « Lancement » : chaque commande ;
  - « Kanban » : la validation (action, stratégie, PR en brouillon, branche cible), les tests avant la validation, le nettoyage, le message conventionnel, le conflit, en parallèle et pilote auto, modèle, effort et mode ;
  - « Git et compte » : mode de commit, compte préféré, remote control du projet.
- Ce qui dépend du dossier est vérifié dans le nouveau dépôt : le dossier de chaque commande de lancement et de chaque étape (relatif au projet, ou au worktree, qui a la même arborescence) ; la branche cible, parmi les branches locales ; le dossier des worktrees, relatif (« relatif au projet ») ou écrit en entier, `~` compris (« chemin complet d'un autre projet : non repris ») ; le compte préféré, actif et réservé à aucun projet.
- Tout est coché par défaut, sauf ce qui ne marcherait pas : une commande ou une étape dont le dossier manque (« dossier « web » introuvable ici »), un dossier des worktrees en chemin complet, un compte inactif, réservé ou supprimé (« … : Automatique »). Une branche cible introuvable reste cochée et devient la branche courante (« branche introuvable : main à la place »).
- Les commandes et les étapes copiées sont de nouvelles commandes : les lancements de l'autre projet restent les siens. La valeur « Harness · modèle · effort · mode » dit le harness copié avec eux (ses modèles sont les siens).
- Jamais copiés : les liens des intégrations, les tickets, la clé des tickets et son numéro suivant, le nom, la couleur, le dossier, « Projet privé », « Comptes partagés à la limite » et les comptes réservés. Le projet copié n'est jamais privé et ses comptes réservés sont les siens : le nouveau projet suit « Nouveaux projets privés ».

### Worktrees, Lancement, Kanban

- **Worktrees** : un worktree par agent (« Les tickets travaillent toujours dans leur worktree ; ce réglage vaut pour les agents ouverts à la main. »), le dossier des worktrees (refusé comme dans les réglages), les préfixes des branches, les fichiers copiés, les commandes de préparation et de démontage. « ✦ Remplir automatiquement » : Claude lit le dossier, avec un compte réservé à aucun projet ; les commandes proposées remplacent celles de l'étape, à relire.
- **Lancement** : les commandes (nom, shell, ligne, sous-dossier, dit introuvable s'il manque) et « ✦ Remplir automatiquement ».
- **Kanban** : l'action à la validation, la branche cible (les branches du dossier ; une cible absente reste montrée en rouge), la stratégie ou la PR en brouillon, la suppression du worktree, le message conventionnel, le conflit, la commande de tests et « Relancer les tests avant », en parallèle, pilote auto, modèle, effort, mode.

### Compte et commit

- Le harness par défaut (dès que deux harness sont proposés ; en changer vide le compte préféré et les comptes à réserver), le compte préféré (« Automatique » et les comptes actifs qui peuvent servir le projet), le commit, le remote control du projet (grisé et allumé quand le réglage pour tous les agents l'est).
- Dans un cadre à part, « Projet privé » : allumé au départ quand « Nouveaux projets privés » l'est (sauf « Garder la configuration actuelle », qui garde celui du projet). Pour un projet privé :
  - « Comptes réservés à ce projet » : une case par compte actif du harness qui peut servir le projet, c'est-à-dire réservé à aucun autre projet ; ceux qu'un projet existant a déjà à lui seul sont cochés et grisés, « déjà réservé » ; sans compte libre : « Aucun compte {harness} libre : … » ;
  - « Comptes partagés à la limite ».
- Les comptes cochés sont réservés au projet seul à l'enregistrement, comme par « Réserver <compte> à ce projet » (voir *Comptes*). Un projet rendu public avant l'enregistrement n'en réserve aucun ; « Comptes partagés à la limite » n'est enregistré allumé que pour un projet privé.

### Intégrations

- Pour chaque service dont le compte est connecté, une source à lier (« Aucune » par défaut), avec ses états par défaut ; le reste se règle dans Réglages › Intégrations (voir *Intégrations*). Sans compte connecté, une phrase le dit.

### Contrôles

Chaque contrôle est dit avec sa correction dans son étape, et tous au récapitulatif (sinon « Harness trouvé · un compte sert le projet · dépôt prêt ») :

| Contrôle | Étape | Correction |
|---|---|---|
| « Le dépôt n'a aucun commit » (dossier sans dépôt compris) | Départ | « Faire un premier commit » : `git init` au besoin, puis `git commit --allow-empty`, aucun fichier ajouté |
| « La branche cible … est introuvable » (merge ou PR, dépôt avec un commit) | Kanban | « Prendre la branche courante » |
| « Les worktrees iraient dans un dossier suivi par git » (le dossier des worktrees existe dans le dépôt et git y suit des fichiers) | Worktrees | « Utiliser .claude/worktrees » |
| « Aucun compte ne sert ce projet » (aucun compte actif qui ne soit réservé à d'autres projets) | Compte et commit | un texte : Réglages › Comptes |
| « Aucun harness trouvé » | Compte et commit | un texte : l'installer, ou indiquer son chemin dans Réglages › Harness |

Le dossier est revérifié à la lecture du chemin, au changement de projet copié, à chaque étape, après une correction et, après une pause, quand le dossier des worktrees ou celui d'une commande change.

### Récapitulatif et création

- Le récapitulatif montre les contrôles, ce qui sera réglé (worktrees, préparation, lancement, validation, conflits, agents des tickets — en parallèle, modèle, effort, mode, « pilote auto » —, harness, projet privé — « non », ou « oui · 1 compte réservé · comptes partagés à la limite » —, commit, intégrations) et « Créer un premier agent », avec son modèle (celui du réglage par défaut coché).
- « Créer le projet » crée le projet, sans premier agent et sans reprendre d'historique, enregistre ses réglages, ses sources liées et son Kanban (la clé des tickets du projet gardée), puis crée le premier agent avec le modèle choisi : il naît avec les réglages du projet, son worktree préparé par eux. Un refus de l'enregistrement est dit par un toast ; le projet, créé, est montré quand même.
- « Créer sans l'assistant » : un projet avec « Un worktree par agent » éteint et un premier agent du modèle par défaut de son compte ; rien d'autre n'est enregistré (les défauts d'Escouade).

### Le dossier d'un projet fermé

- Il passe avant l'assistant. Quand le dossier est celui d'un projet fermé qui a gardé son historique (voir *Fermer un projet*), la fenêtre le dit (« Historique retrouvé : 12 agents, 34 tickets, dernière activité le 14 octobre 2026 »), préremplit le nom et la couleur avec ceux du projet fermé, cache le point de départ et grise les étapes. Son bouton devient « Rouvrir le projet » : la configuration revient telle quelle, « Un worktree par agent » compris.
- La bascule « Partir d'un projet vierge » (« L'historique reste dans Réglages › Projets, où l'effacer. ») rend l'assistant et crée un projet neuf.
- Ouverte par « Rouvrir » sur un projet fermé (Réglages › Projets › Projets fermés), la fenêtre part du dossier de ce projet, lu tout de suite, et de cet historique-là, même quand le dossier en a plusieurs. Changer le chemin saisi abandonne cet historique ; le remettre tel qu'au départ le rend. « Annuler » ferme la fenêtre sans revenir aux réglages.

### Reconfigurer un projet ouvert

- Réglages › Projets › « Assistant de configuration » › « Ouvrir l'assistant… » remplace les réglages par l'assistant sur ce projet, titré « Configurer <projet> ». Pas de dossier à choisir. Points de départ : « Garder la configuration actuelle » (par défaut), la copie (des autres projets ouverts), la configuration recommandée ou zéro. Ni premier agent, ni « Créer sans l'assistant ».
- « Enregistrer », au récapitulatif, enregistre les réglages et le Kanban, nom et couleur compris ; les sources liées du projet sont gardées ; le reste du brouillon des réglages est abandonné. « ✦ Remplir automatiquement » lit alors le dossier du projet.
