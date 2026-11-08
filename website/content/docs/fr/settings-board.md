# Réglages : le Kanban d’un projet

La page « Kanban » d’un projet décide de ce que devient un ticket validé, des tests lancés avant la validation, du pilote auto et des agents qui prennent les tickets.

## Ouvrir la page

Ouvre les réglages avec `Ctrl+,` (`⌘,` sous macOS), puis, sous « Projets », le projet et sa page « Kanban ». Depuis le [Kanban](board), le bouton ⚙ « Après validation : … » de l’en-tête ouvre cette page directement, sur le projet affiché.

Ce que tu y changes ne vaut que pour ce projet et s’enregistre au geste. La pause du pilote auto au-delà du quota, qui vaut pour tous les projets, est dans [« Comptes et harness »](settings-accounts), groupe « Quand un quota est épuisé ».

## Quand je valide un ticket « À tester »

Ce premier groupe dit ce que fait la validation d’un ticket « À tester » (voir [le Kanban](board)). Le bouton de validation prend le nom du choix, et l’en-tête du Kanban le résume, par exemple « merge squash → main ».

| Choix | Ce que fait la validation | Bouton |
|---|---|---|
| « Merger dans une branche » (par défaut) | Fusionne le worktree de l’agent dans la branche cible, puis libère l’agent. | « Valider et merger » |
| « Ouvrir une pull request » | Pousse `ticket/<clé>` et ouvre une PR vers la branche cible pour relecture. | « Valider + PR » |
| « Pousser la branche du ticket » | Commit et push sur `ticket/<clé>`, sans merge ni PR. | « Valider et pousser » |
| « Laisser en l’état » | Les modifications restent non commitées dans le worktree. | « Valider » |

`ticket/` est le « Préfixe des branches des tickets » du projet, à régler dans sa page [« Git et worktrees »](settings-projects) (`ticket/` par défaut) ; `<clé>` est la clé du ticket, par exemple `ATL-42`.

Avec les trois premiers choix, Escouade commite d’abord ce que l’agent a laissé dans le worktree, puis fait l’action. Le ticket passe ensuite en « Terminé » et son agent est archivé. Si la branche n’apporte aucune modification, le ticket est terminé sans rien fusionner ni pousser. Avec « Laisser en l’état », le ticket passe aussi en « Terminé » ; l’agent et son worktree restent comme ils sont, sauf si le worktree ne garde rien de non sauvegardé (voir « Supprimer le worktree d’un ticket terminé ou supprimé », plus bas).

### Branche cible

Affichée avec « Merger dans une branche » et « Ouvrir une pull request ». Le bouton ⎇ ouvre le sélecteur de branche, avec sa recherche : il propose les branches locales du projet, sans celles des agents et des tickets (`escouade/…`, `ticket/…`). C’est la branche dans laquelle les tickets sont mergés, et aussi la base de la branche de chaque ticket et de sa PR. Par défaut, c’est la branche ouverte dans le dossier du projet quand le premier ticket démarre ; Escouade la retient ensuite. Si la branche n’existe pas, la validation s’arrête avec une erreur.

Le merge se fait dans le dossier du projet quand la branche cible y est ouverte (il est alors refusé s’il y a des modifications non commitées sur des fichiers suivis), sinon dans un worktree temporaire. Il est refusé si la branche cible est ouverte dans le worktree d’un agent. Voir [les worktrees](worktrees).

Pour que les nouveaux agents partent de la branche cible, règle « Branche de départ des nouveaux worktrees » sur « Branche cible du Kanban » dans [les réglages du projet](settings-projects) : ce réglage a remplacé « Les agents partent de la branche cible » de l’ancien onglet « Kanban » (voir [la branche de départ](worktrees#la-branche-de-depart)).

### Stratégie

Affichée avec « Merger dans une branche » seulement. Défaut : « Squash ».

| Stratégie | Ce qui arrive à la branche cible |
|---|---|
| « Merge commit » | Un commit de merge, avec le message de git ; les commits du ticket restent dans l’historique. |
| « Squash » | Un seul commit, avec le message généré (voir « Message de commit généré » plus bas). |
| « Rebase » | La branche du ticket est rebasée sur la branche cible, puis la branche cible avance en fast-forward. |

Un conflit annule le merge : voir « En cas de conflit » plus bas.

### PR en brouillon

Affiché avec « Ouvrir une pull request » seulement. Désactivé par défaut. Activé, la PR est ouverte en brouillon.

Escouade pousse la branche, puis ouvre la PR par l’API de l’[hébergeur de code](code-hosts) du dépôt, avec le compte connecté pour son instance. Sans compte, ou quand l’hébergeur refuse, elle ouvre dans ton navigateur sa page de création de PR, remplie ; ce réglage ne s’applique alors pas. Chez un hébergeur inconnu, la branche est seulement poussée.

Le ticket attend ensuite son merge dans [la colonne « En revue »](board#la-colonne-en-revue). Deux réglages de plus accompagnent cette action : « Répondre sous les fils corrigés » (éteint par défaut : Escouade ne publie rien sous les fils sans lui, et ne résout jamais un fil) et « Mise à jour de la branche » d’une PR en conflit avec sa cible (merge par défaut, ou rebase).

## Avant et après

Ce groupe règle ce qui se passe avant l’action choisie plus haut, et après elle.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Commande de tests » | La commande lancée dans le worktree du ticket, par exemple `npm test`. Elle s’arrête au bout de 20 minutes. | Vide |
| « Relancer les tests avant » | Lance la commande de tests avant l’action. Si un test échoue, l’action est bloquée et le ticket revient à l’agent avec la fin de la sortie des tests. Le réglage est grisé tant que la commande de tests est vide. | Désactivé |
| « Supprimer le worktree d’un ticket terminé ou supprimé » | Libère l’espace disque : le worktree d’un ticket terminé, quelle que soit la validation, et celui d’un ticket supprimé, dans n’importe quelle colonne, sont supprimés après leurs commandes de démontage, avec ceux des agents que le ticket a eus avant. Un worktree qui a des changements non commités, ou des commits ni mergés ni poussés, est gardé, et c’est dit (voir ci-dessous). Mergée ou vide, la branche du ticket part aussi ; poussée ou proposée en PR, elle reste, comme toute branche que le ticket reprenait. Désactivé, les worktrees restent. | Activé |
| « Message de commit généré » | Activé, Haiku écrit le message au format Conventional Commits ; si sa réponse n’est pas utilisable, le message est `feat: <titre> [<clé>]`. Désactivé, le message est `<clé> <titre>`. Un exemple s’affiche sous le réglage, avec la clé de ton projet. | Activé |
| « En cas de conflit » | Ce que fait un merge qui tombe sur un conflit. Voir plus bas. | « Me demander » |

Escouade ne perd jamais de travail en nettoyant : un worktree n’est supprimé que si ses fichiers sont tous commités et si chaque commit de sa branche est dans la branche cible ou dans une autre branche, locale ou distante. Sinon il reste :

- avec « Laisser en l’état », l’issue du ticket le dit, par exemple « ◇ Laissé dans le worktree · worktree gardé : 2 fichier(s) non commité(s) », et l’agent reste ;
- pour un ticket supprimé, la carte de son agent archivé le dit, sous son nom, dans la liste « Archivés ».

Restaurer cet agent le remet au travail dans le même worktree. Pour ranger plus tard ce qui reste, utilise « Nettoyer » dans sa page [« Git et worktrees »](settings-projects).

Le message de commit sert à commiter ce que l’agent a laissé dans le worktree, et de message au squash ; un merge commit garde le message de git. Sa langue suit « Langue des textes rédigés par les agents » (page [« Général »](settings-application)).

### En cas de conflit

Un conflit ne peut survenir que pendant un merge. Dans tous les cas, le merge est annulé d’abord.

| Choix | Ce qui se passe |
|---|---|
| « Me demander » (par défaut) | Le ticket est bloqué avec la liste des fichiers en conflit, et sa carte propose « L’agent résout » et « Annuler ». |
| « L’agent résout » | Escouade passe tout de suite la main à l’agent : la branche cible est mergée dans la branche du ticket (ou, avec la stratégie « Rebase », l’agent est invité à rebaser), et le ticket repart « En cours » à la boucle 1. |
| « Annuler » | Le ticket est bloqué avec la liste des fichiers en conflit, sans bouton pour résoudre. |

## Pilote auto

Voir [le pilote auto](autopilot).

| Réglage | Effet | Par défaut |
|---|---|---|
| « Attribuer les tickets automatiquement » | Un agent libre prend le prochain ticket « À faire ». C’est le même interrupteur que « Pilote auto » dans l’en-tête du Kanban, qui s’y enregistre aussitôt. Désactivé, un ticket ne part que si tu cliques sur « Lancer ». | Activé |
| « Pause au-delà du quota » | Choix de 80 %, 90 %, 95 % ou 100 %. Aucun ticket ne démarre tant que la fenêtre de 5 h ou la fenêtre hebdomadaire a atteint ce seuil. Commun à tous les projets (« pour tous les projets »). | 100 % |

Quand la pause joue, une bande sous l’en-tête du Kanban dit pourquoi rien ne démarre et propose « Reprendre maintenant », qui lève la pause jusqu’à la fin de la fenêtre concernée.

Avec plusieurs comptes Claude, la pause ne tient que si tous les comptes que le projet peut utiliser sont retenus, et le même seuil sert à choisir le compte sur lequel partent les nouveaux agents. Voir [les comptes](accounts) et [le suivi du quota](usage).

> [!NOTE]
> Le réglage « Reprendre sur un autre compte un agent de ticket arrêté par la limite » n’est pas dans cette page. Il est dans [« Comptes et harness »](settings-accounts), et n’apparaît qu’avec plusieurs comptes. Il est désactivé par défaut.

## Agents

Les agents qui prennent les tickets de ce projet. Voir [le Kanban](board).

| Réglage | Effet | Par défaut |
|---|---|---|
| « En parallèle » | Le nombre de tickets « En cours » en même temps : 1, 2, 3, 4, 5 ou 6. Au-delà, les tickets « À faire » attendent une place libre. | 2 |
| « Modèle » | Le modèle des agents de ticket : « Par défaut (…) », qui suit le modèle du compte qui le lance, ou Fable, Opus, Sonnet, Haiku. | « Par défaut » |
| « Effort » | « Par défaut », ou Bas, Moyen, Élevé, Très élevé, Max. | « Par défaut » |
| « Mode » | « Par défaut », ou le mode de permission : Auto, Demander, Plan, Édits auto, Bypass. | « Par défaut » |

« Par défaut » veut dire le choix correspondant du compte qui prend le ticket, dans sa fiche de [Réglages › Comptes et harness](settings-accounts) (« Nouveaux agents ») : le menu le nomme d’après le compte où partirait un nouvel agent du projet. Tant que tu n’y touches pas, ce sont ceux de son harness : Sonnet, Moyen et Auto pour Claude Code.

Les modes de permission sont expliqués dans [les agents](managing-agents). En bref :

- « Auto » : un classifieur approuve les actions sûres et demande pour le reste.
- « Demander » : Claude demande ton accord avant chaque action sensible.
- « Plan » : Claude analyse et propose un plan sans rien modifier.
- « Édits auto » : les modifications de fichiers sont acceptées sans demander.
- « Bypass » : aucune demande de permission, à réserver aux environnements sûrs.
