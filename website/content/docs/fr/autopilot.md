# Le pilote auto

Le pilote auto prend les tickets « À faire » d’un projet et confie chacun à un agent, dans son worktree, qui boucle jusqu’à atteindre les critères d’acceptation. Cette page explique comment il choisit, boucle, se met en pause près du quota et reprend.

## L’activer ou l’éteindre

L’interrupteur « Pilote auto » est dans l’en-tête du [Kanban](board). Il est propre à chaque projet et allumé par défaut. C’est le même réglage que « Attribuer les tickets automatiquement » dans Réglages › Kanban (voir [Réglages du Kanban](settings-board)).

Éteint, l’en-tête affiche « Pilote auto · off », les cartes « À faire » disent « Pilote auto désactivé » et chaque ticket part quand tu cliques sur « Lancer » sur sa carte (voir plus bas).

Rien ne démarre, pilote auto allumé ou non, dans trois cas :

- Claude Code est introuvable : l’en-tête dit « Claude Code introuvable — aucun ticket ne démarre ».
- La branche cible du Kanban n’a encore aucun commit, ou n’existe plus : l’en-tête dit « main n’a encore aucun commit — aucun ticket ne démarre » ou « Branche cible main introuvable — aucun ticket ne démarre », et les cartes « À faire » disent « En attente de la branche cible ».
- Aucun compte Claude actif ne sert le projet (tous sont « Réservés aux projets » qui ne sont pas le sien) : l’en-tête dit « Aucun compte Claude pour ce projet — aucun ticket ne démarre ».

## Combien d’agents en parallèle

Le réglage « En parallèle » (Réglages › Kanban) fixe le nombre de tickets « En cours » en même temps : de 1 à 6, 2 par défaut. L’en-tête compte les places libres (« 1 place libre », « Toutes les places sont prises »).

Un ticket « En cours » occupe une place, y compris quand son agent attend sa remise à zéro de quota. Un ticket bloqué n’en occupe plus.

## Quel ticket part, et quand

Dès qu’une place se libère, le pilote auto prend le premier ticket « À faire » qui peut partir.

- **L’ordre** est celui de la colonne, du haut vers le bas. Un ticket se place en tête avec clic droit › « Passer en tête » ; les nouveaux arrivent en bas.
- **Les dépendances** : un ticket dont le champ « Après » nomme des tickets qui ne sont pas tous « Terminé » est laissé de côté, et le suivant passe devant. Sa carte dit « ⏸ après DEM-3 ». Il part dès que le dernier de ces tickets est terminé. Un ticket supprimé ne retient plus personne.
- **La carte du prochain ticket** dit « Pris dès qu’une place se libère » ; les autres disent « En attente d’une place (2/2) ».

### Lancer un ticket à la main

Avec le pilote auto éteint, une carte « À faire » a un bouton « Lancer » tant qu’une place est libre et que rien ne retient les tickets. Le ticket démarre aussitôt, sans attendre son tour (sa carte dit « Lancement demandé… » le temps du démarrage).

Si le ticket vient après des tickets qui ne sont pas terminés, Escouade demande d’abord « DEM-5 attend DEM-3, pas encore terminé. Le lancer quand même ? » ; « Lancer quand même » le démarre sans eux. Un ticket lancé à la main respecte la pause et les places libres comme les autres.

## Un ticket, un worktree, un agent

Quand un ticket démarre, Escouade :

1. le passe dans « En cours », à la boucle 1 ;
2. crée un agent, nommé d’après la clé et le titre du ticket (`dem-5-export-csv-des-factures`) ;
3. lui prépare un worktree sur la branche du ticket, `ticket/dem-5` (le préfixe est le « Préfixe des branches des tickets » du projet), créée à partir de la branche cible. Pour un ticket importé d’une source, la branche et l’agent portent la clé du ticket d’origine et son titre (`ticket/proj-123-corriger-la-connexion`). Si le ticket reprend une branche existante, l’agent travaille sur celle-ci ;
4. attend la préparation du worktree : fichiers `.env*` copiés, commandes de préparation du projet (la carte dit « Prépare le worktree · … »). Si une commande échoue, l’agent en est informé avec son premier message, pour y remédier ;
5. envoie à l’agent son premier message : la clé et le titre, la description, les critères numérotés, et la consigne de travailler jusqu’à les atteindre puis de terminer par le bilan.

Ce message, comme les suivants, est écrit dans « Langue des textes rédigés par les agents » (Réglages › Application, voir [Réglages de l’application](settings-application)).

À chaque démarrage, Escouade ajoute aussi à la consigne système de l’agent un protocole qui survit au compactage et aux reprises : travailler seul, vérifier soi-même chaque critère (tests, exécution) avant de le déclarer atteint, ne pas commiter les fichiers copiés du projet, poser ses questions par l’outil de question quand une décision manque, et terminer chaque réponse par le bilan des critères. Escouade réserve en plus au ticket un bloc de ports, et le protocole demande à l’agent comment lancer son travail pour le tester (voir [Tester et valider un ticket](testing)).

### Modèle, effort et permissions

Dans Réglages › Kanban, le groupe « Agents » règle les agents des tickets du projet : « Modèle », « Effort » et « Mode » (de permission). Laissés à « Par défaut », ils suivent les valeurs par défaut de Réglages › Harness (voir [Réglages des harness](settings-claude-code)).

Les agents des tickets restent dans la vue d’Escouade. Si l’un d’eux demande une autorisation ou pose une question, sa carte passe en ambre avec « Question en attente de ta réponse », tu es prévenu, et tu réponds dans sa conversation (voir [Le chat](chat)).

### Sur quel compte il part

Un ticket part sur le « Compte préféré » du projet s’il est actif. Sinon, sur le premier compte actif, dans l’ordre de Réglages › Comptes, qui est « bon » (voir [Les comptes Claude](accounts#le-choix-automatique)) et sous le seuil de « Pause au-delà du quota », sans pause due à la limite d’usage et sans agent qui attende sa remise à zéro. Un compte « Réservé aux projets » ne sert que ces projets. L’agent reste ensuite sur son compte, sauf reprise sur un autre compte après la limite d’usage (voir plus bas). Voir [Les comptes Claude](accounts).

## La boucle

Une boucle, c’est un passage de l’agent sur le ticket : il travaille, répond, et termine sa réponse par le bilan des critères. Escouade lit ce bilan à la fin de chaque tour (voir [Le Kanban](board#le-bilan-des-criteres)).

| À la fin du tour | Ce que fait Escouade |
|---|---|
| Tous les critères sont atteints | Le ticket passe dans « À tester » et tu es prévenu (« prêt à tester »). |
| Des critères manquent, et il reste des boucles | La boucle suivante commence : l’agent reçoit la liste des critères non atteints, avec ses propres notes, et le compteur de la carte passe à « Boucle 2/5 ». |
| Des critères manquent à la dernière boucle | Le ticket passe dans « À tester » avec la mention « Objectif partiel ». À toi de le valider tel quel ou de le renvoyer. |
| Pas de bilan | L’agent est relancé une fois. S’il n’en donne pas non plus, le ticket se bloque (« Bilan des critères manquant »). |
| Tour interrompu (`Échap`, bouton stop) | Le ticket se bloque (« Interrompu »). |
| Erreur de Claude Code | Le ticket se bloque (« Erreur : … »). |

Le nombre maximal de boucles est le « Boucles max » du ticket : 3, 5 ou 8. Un critère que le bilan oublie compte comme non atteint. À chaque boucle, le bilan remplace le précédent : un critère peut repasser de ✓ à ○.

Un ticket bloqué te prévient (« bloqué »), ne tient plus de place et affiche son motif sur sa carte. « Reprendre » renvoie l’agent au travail (« Reprends le ticket DEM-5 là où tu en étais, puis termine par le bilan »). Si l’agent n’existe plus, le ticket repart à zéro. Pendant que l’agent attend son quota, « Reprendre » répond « En attente du quota — reprise à 14:30 ».

Quand un ticket revient chez son agent après un « Renvoyer », des tests en échec ou un conflit (voir [Tester et valider un ticket](testing)), le compteur repart à « Boucle 1/5 ».

## La pause près de la limite du quota

Le réglage « Pause au-delà du quota » (Réglages › Kanban, « pour tous les projets ») prend 80 %, 90 %, 95 % ou 100 % (par défaut). Dès que la fenêtre de 5 h ou la fenêtre hebdomadaire d’un compte atteint ce seuil, le pilote auto ne démarre aucun ticket sur lui avant la remise à zéro de la fenêtre. Ce seuil ne retient que le pilote auto : un ticket lancé à la main (« Lancer », ou `start_ticket` d’un agent) part quand même, et seule une limite d’usage atteinte le retient. La file repart d’elle-même 30 secondes après cette remise à zéro, ou dès que les quotas lus repassent sous le seuil.

L’en-tête du Kanban le dit sur une ligne à part, par exemple « Pilote auto en pause : quota de 5 h à 95 % (reprise à 14:30) » ou « Pilote auto en pause : quota hebdo à 96 % (reprise le vendredi 2 octobre à 09:30) », avec un bouton « Reprendre maintenant ». Les cartes « À faire » disent « En attente : pilote auto en pause ». Les agents déjà au travail continuent.

- Avec plusieurs comptes, la pause ne tombe que si tous les comptes que le projet peut utiliser sont retenus : un seul compte libre suffit, le ticket part dessus. La ligne ajoute alors les comptes qui ont dépassé le seuil (« Les comptes Principal et Pro ont passé le seuil. »), et la pause dure jusqu’à ce que le premier se libère.
- Un projet qui a un « Compte préféré », ou des comptes « Réservés aux projets », est en pause selon ses comptes à lui.
- « Reprendre maintenant » lève la pause : les fenêtres qui dépassent le seuil ne retiennent plus rien jusqu’à leur fin, et ce qui peut partir part.

Pendant une pause, rien ne démarre, même un ticket lancé à la main.

## Limite d’usage et reprise

Quand un agent de ticket atteint la limite d’usage de son compte, son tour s’arrête. Ce qui suit dépend de tes réglages (Réglages › Harness) :

- **« Reprise automatique après la limite d’usage »** (activée par défaut) : Escouade attend la remise à zéro du quota, puis envoie « continue » à l’agent. Le ticket reste « En cours » et garde sa place ; sa carte dit « Reprise à 14:30 ». Quand tous les comptes que le projet peut utiliser ont ainsi un agent en attente, l’en-tête dit « Quota atteint — reprise à 14:30 » et rien ne démarre avant. Tant qu’un compte n’en a pas, un ticket peut partir dessus.
- **« Reprendre sur un autre compte un agent de ticket arrêté par la limite »** (avec plusieurs comptes, désactivé par défaut) : l’agent repart tout de suite sur un autre compte utilisable, avec sa session et sa conversation. Un compte est utilisable s’il est connecté, sous le seuil, sans pause due à la limite d’usage et sans agent en attente de sa remise à zéro. Une session ne passe pas d’un compte Windows à un compte WSL. Le compte quitté reste retenu jusqu’à sa remise à zéro (30 minutes si elle n’est pas connue). Sans ce réglage, l’agent attend la remise à zéro de son compte.
- **Aucune reprise prévue** (réglage éteint, clé d’API, remise à zéro inconnue) : le ticket se bloque (« Erreur : limite d’usage atteinte »), et le pilote auto se met en pause pendant 30 minutes sur ce compte, car le ticket suivant rencontrerait la même limite (« Pilote auto en pause : limite d’usage atteinte (reprise vers 14:30) »). « Reprendre maintenant » lève cette pause.

## Quand l’app redémarre

Fermer la fenêtre ne coupe pas les agents. Si l’app s’arrête puis redémarre, chaque ticket « En cours » qui n’est pas bloqué reprend : son agent reçoit un message pour continuer là où il en était. Un ticket dont le démarrage a été coupé avant le premier message de l’agent revient « À faire » et repart, même pilote auto éteint.

## Voir aussi

- [Le Kanban](board) : les colonnes, les cartes et le bilan des critères.
- [Tester et valider un ticket](testing) : ce qui arrive après « À tester ».
- [Réglages du Kanban](settings-board) : « En parallèle », « Pause au-delà du quota », modèle, effort et mode.
- [Le serveur MCP](mcp) : lancer des tickets depuis Claude, sous les mêmes garde-fous.
