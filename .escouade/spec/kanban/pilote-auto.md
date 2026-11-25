---
title: "Pilote auto"
subject: "Kanban"
sources: ["docs/SPEC.md","src-tauri/src/mcp/act_tests.rs","website/content/docs/fr/autopilot.md"]
tickets: ["ESC-312","ESC-63","ESC-382","ESC-479","ESC-547","ESC-645"]
decisions: [20,24,123,225,315]
compared: "2026-10-31T17:27:51+01:00"
gaps: [{"text":"La page dit que les cartes « À faire » en attente d'un autre ticket affichent « ⏸ après … » et que ce texte passe avant « En attente : pilote auto en pause ». Le code affiche « Après {keys} » (en anglais « After {keys} »), sans le symbole ⏸.","code":["src/lib/i18n/fr/boardSettings.ts","src/lib/board.ts"]}]
---

## Ce que c'est

Le pilote auto lance seul les tickets « À faire », dans l'ordre, tant qu'il reste des places. Il se met en pause quand un quota approche de sa fin ou qu'une limite d'usage est atteinte, pour ne pas lancer des tickets voués à s'arrêter, et, si le Kanban le demande, tant que la CI de la branche cible est rouge. Ces pauses ne retiennent que lui : un ticket lancé à la main part quand même (sauf à une limite d'usage atteinte).

## Comportement

### Les places

- Tant que les tickets « En cours » non bloqués sont sous le maximum, le premier ticket « À faire » qui n'attend aucun ticket non terminé part. Sans pilote auto, seuls partent les tickets lancés à la main (« Lancer »).
- « Pris dès qu'une place se libère » va à la première carte qui n'attend rien.
- Un ticket bloqué libère sa place.
- Le pilote auto ne prend jamais le ticket partagé d'un autre (voir *Tickets partagés*).

### Ce qui retient un ticket lancé à la main

- Un ticket lancé à la main, par « Lancer » ou par `start_ticket` d'un agent, part même pendant une pause au quota ou sur une cible rouge.
- Seule une limite d'usage atteinte le retient, à la main comme par le pilote auto :
  - une fenêtre de quota épuisée (100 %) ;
  - un agent qui attend la fin de sa limite d'usage ;
  - la pause qui suit une limite d'usage sans reprise prévue (ci-dessous).
- Le refus dit pourquoi et jusqu'à quand (la fenêtre, son pourcentage, l'heure de reprise). Une passe déjà en cours revérifie avant chaque départ.

### La pause au quota

- Une fenêtre de quota (5 h ou hebdomadaire) au seuil « Pause du pilote auto au-delà du quota » ou au-delà, dont la fin est connue et pas encore passée, met le pilote auto en pause. Elle le retient jusqu'à sa fin plus 30 s, ou jusqu'à ce qu'une lecture des quotas la redonne sous le seuil.
- Un seuil changé vaut tout de suite : relevé au-dessus de la fenêtre, le pilote auto repart sans attendre ; abaissé sous elle, il se met en pause avant le ticket suivant.
- Elle ne retient que le pilote auto : un ticket lancé à la main part quand même, tant que la fenêtre n'est pas épuisée.
- Les agents déjà au travail continuent.
- Avec plusieurs comptes, la pause se calcule compte par compte (voir *Comptes*).
- Le plafond est celui de l'utilisateur, jugé en pourcentage fenêtre par fenêtre : il ne suit pas le rythme que le choix du compte juge (une semaine à 95 % qui se remet à zéro dans 3 h est un bon choix, mais sous un plafond de 90 % elle retient quand même le compte). Voir *Comptes*, « Pause du pilote auto, compte par compte ».
- La fenêtre propre à un modèle (« Fable 7 j ») ne retient que les tickets de ce modèle, rapproché par sa famille (voir *Comptes*, décision 24) : au seuil, un ticket Fable part sur un autre compte ou attend, un ticket Opus ou Sonnet part. Elle ne met pas tout le pilote auto en pause, et la ligne de pause n'en parle pas. Un agent arrêté par la limite d'un modèle ne retient lui aussi que les tickets de ce modèle.

### La pause à la limite d'usage

L'agent d'un ticket arrêté par la limite sans reprise prévue (reprise automatique coupée ou annulée, clé d'API, heure inconnue) bloque son ticket. Le pilote auto se met alors en pause 30 min avant que la place libérée serve, au lieu de lancer le ticket suivant sur la même limite. Cette pause retient aussi les tickets lancés à la main. La limite d'un modèle seul (la semaine de Fable) ne la déclenche pas : la fenêtre de ce modèle retient déjà ses tickets.

### La pause sur une cible rouge

- « Pause quand la CI de la cible est rouge », éteint par défaut. Allumé, le pilote auto ne démarre aucun ticket « À faire » du projet tant que la CI de sa branche cible est en échec, d'après la dernière lecture (voir *CI de la cible et des branches poussées*).
- Un ticket lancé à la main part quand même, et « Lancer » reste sur les cartes. Les agents au travail continuent, et la correction de la CI d'un ticket « En revue » aussi.
- Elle se lève dès qu'une lecture voit la cible verte ou en cours, ou que ses jobs en échec sont relancés.

### Ce que montre la pause

- Une ligne sous l'en-tête du Kanban : « Pilote auto en pause : quota hebdo à 96 % (reprise à 14:00) », « … quota de 5 h à 100 % (reprise à 14:00) » ou « … limite d'usage atteinte (reprise vers 14:30) », avec le jour quand ce n'est pas aujourd'hui. Les places libres ne s'affichent pas pendant ce temps.
- Sur une cible rouge : « Pilote auto en pause : la CI de main est rouge depuis 2 h », avec « Voir les jobs ».
- Les cartes « À faire » disent « En attente : pilote auto en pause », ou « En attente : CI de la cible rouge ». « En attente de la branche cible » et « Après … » passent avant.
- « Reprendre maintenant » lève la pause de limite, et chaque fenêtre au-delà du seuil jusqu'à la fin qu'elle avait. Une fenêtre suivante, ou qui repasse le seuil, retient de nouveau. Un agent qui attend sa propre reprise retient toujours.
- La pause, fenêtres lues et levées comprises, est gardée avec l'état de l'app et tient après un redémarrage. La pause sur une cible rouge, non : elle attend la première lecture de la CI après le démarrage.

## Réglages

- « Pilote auto », dans l'en-tête du Kanban.
- Le nombre de places, dans les réglages du Kanban : de 1 à 16 sur un poste, à 64 sur un serveur, sous un plafond imposé, 2 par défaut (voir *Kanban*, « Les tickets en parallèle », ESC-620).
- « Pause du pilote auto au-delà du quota », « pour tous les projets » : 80, 90, 95 ou 100 %, 100 % par défaut. Une autre valeur écrite à la main vaut 100 %. C'est un réglage de l'app, dans Réglages › Comptes et harness › « Quand un quota est épuisé », à côté de « Changer de compte à » (avant la 2.2, dans la page Kanban d'un projet ; sa clé, `quotaPause`, n'a pas changé).
- « Pause quand la CI de la cible est rouge » (Réglages › *projet* › Kanban, par projet) : éteint par défaut.

## Limites

- Une fenêtre de quota dont la fin n'est pas connue ne met jamais en pause.
- Un ticket « À faire » d'un modèle retenu par sa fenêtre propre dit « En attente : pilote auto en pause » seulement quand tout le pilote auto l'est : sinon sa carte ne dit pas encore pourquoi il attend.

## Les tickets d'un Kanban d'équipe (TS-6, C11-A)

- Le pilote auto ne prend que les tickets de l'équipe qu'on lui confie : ceux assignés au membre de ce poste (« Me l'assigner »), et ceux de « À faire » quand « Prendre aussi « À faire » de l'équipe » est coché (Réglages › *projet* › Kanban, éteint par défaut, de ce poste seul, jamais dans `.escouade.yml`).
- Il ne prend jamais un ticket qu'un collègue tient, ni un ticket interrompu chez un collègue (son bail expiré) : « Reprendre » le fait à la main. Il ne prend aucun ticket de l'équipe sans le serveur : sa prise échoue en silence et il le retente à la passe suivante.
- Un ticket de l'équipe qu'il ne prendra pas garde « Lancer » sur sa carte, pilote auto allumé. Voir *Le Kanban d'équipe*.

## L'arrêt au-delà d'un budget de l'équipe (TS-16)

- Quand le fichier de l'équipe dit `costs.stopAutopilot: true` et qu'un budget du mois est dépassé (le projet, l'équipe, ou ce membre), le pilote auto ne démarre plus les tickets « À faire » des projets concernés. Une ligne sous l'en-tête le dit : « Pilote auto arrêté : Le projet api a dépensé 104 % de son budget du mois (imposé par l'équipe Acme). »
- Un ticket lancé à la main part, après la question « Le projet api a dépensé 104 % de son budget du mois : le pilote auto de ce projet est arrêté par l'équipe Acme. Le lancer quand même ? ». La question vient dès 80 % d'un budget, sans arrêt.
- Le budget de l'équipe ou de ce membre grise aussi l'interrupteur « Pilote auto » (le verrou, « Imposé par l'équipe Acme »). Voir *Les coûts et les quotas de l'équipe*.

## Un ticket de l'équipe à accepter (ESC-645)

- Un ticket de l'équipe assigné au membre de ce poste, dont le titre, la description ou les critères ont été écrits par un autre membre, attend « Accepter » (sa carte, ou sa demande dans « À toi ») : le pilote auto ne le prend pas, ni l'Overseer, ni `start_ticket` d'un agent, ni « Lancer ». Sa carte dit « À accepter : rien ne le lance avant ». Un texte changé par un autre après l'accord attend de nouveau. Voir *Le Kanban d'équipe*, « Un ticket qu'un collègue écrit et m'assigne ».
