---
title: "Pilote auto"
subject: "Kanban"
sources: ["docs/SPEC.md","src-tauri/src/mcp/act_tests.rs","website/content/docs/fr/autopilot.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

Le pilote auto lance seul les tickets « À faire », dans l'ordre, tant qu'il reste des places. Il se met en pause quand un quota approche de sa fin ou qu'une limite d'usage est atteinte, pour ne pas lancer des tickets voués à s'arrêter. La pause au quota ne retient que lui : un ticket lancé à la main part quand même.

## Comportement

### Les places

- Tant que les tickets « En cours » non bloqués sont sous le maximum, le premier ticket « À faire » qui n'attend aucun ticket non terminé part. Sans pilote auto, seuls partent les tickets lancés à la main (« Lancer »).
- « Pris dès qu'une place se libère » va à la première carte qui n'attend rien.
- Un ticket bloqué libère sa place.
- Le pilote auto ne prend jamais le ticket partagé d'un autre (voir *Tickets partagés*).

### Ce qui retient un ticket lancé à la main

- Un ticket lancé à la main, par « Lancer » ou par `start_ticket` d'un agent, part même pendant une pause au quota.
- Seule une limite d'usage atteinte le retient, à la main comme par le pilote auto :
  - une fenêtre de quota épuisée (100 %) ;
  - un agent qui attend la fin de sa limite d'usage ;
  - la pause qui suit une limite d'usage sans reprise prévue (ci-dessous).
- Le refus dit pourquoi et jusqu'à quand (la fenêtre, son pourcentage, l'heure de reprise). Une passe déjà en cours revérifie avant chaque départ.

### La pause au quota

- Une fenêtre de quota (5 h ou hebdomadaire) au seuil « Pause au-delà du quota » ou au-delà, dont la fin est connue et pas encore passée, met le pilote auto en pause. Elle le retient jusqu'à sa fin plus 30 s, ou jusqu'à ce qu'une lecture des quotas la redonne sous le seuil.
- Elle ne retient que le pilote auto : un ticket lancé à la main part quand même, tant que la fenêtre n'est pas épuisée.
- Les agents déjà au travail continuent.
- Avec plusieurs comptes, la pause se calcule compte par compte (voir *Comptes*).
- Le plafond est celui de l'utilisateur, jugé en pourcentage fenêtre par fenêtre : il ne suit pas le rythme que le choix du compte juge (une semaine à 95 % qui se remet à zéro dans 3 h est un bon choix, mais sous un plafond de 90 % elle retient quand même le compte). Voir *Comptes*, « Pause du pilote auto, compte par compte ».

### La pause à la limite d'usage

L'agent d'un ticket arrêté par la limite sans reprise prévue (reprise automatique coupée ou annulée, clé d'API, heure inconnue) bloque son ticket. Le pilote auto se met alors en pause 30 min avant que la place libérée serve, au lieu de lancer le ticket suivant sur la même limite. Cette pause retient aussi les tickets lancés à la main.

### Ce que montre la pause

- Une ligne sous l'en-tête du Kanban : « Pilote auto en pause : quota hebdo à 96 % (reprise à 14:00) », « … quota de 5 h à 100 % (reprise à 14:00) » ou « … limite d'usage atteinte (reprise vers 14:30) », avec le jour quand ce n'est pas aujourd'hui. Les places libres ne s'affichent pas pendant ce temps.
- Les cartes « À faire » disent « En attente : pilote auto en pause ». « En attente de la branche cible » et « ⏸ après … » passent avant.
- « Reprendre maintenant » lève la pause de limite, et chaque fenêtre au-delà du seuil jusqu'à la fin qu'elle avait. Une fenêtre suivante, ou qui repasse le seuil, retient de nouveau. Un agent qui attend sa propre reprise retient toujours.
- La pause, fenêtres lues et levées comprises, est gardée avec l'état de l'app et tient après un redémarrage.

## Réglages

- « Pilote auto », dans l'en-tête du Kanban.
- Le nombre de places, dans les réglages du Kanban : de 1 à 6, 2 par défaut.
- « Pause au-delà du quota » (Réglages › Kanban, pour tous les projets) : 80, 90, 95 ou 100 %, 100 % par défaut. Une autre valeur écrite à la main vaut 100 %.

## Limites

- Une fenêtre de quota dont la fin n'est pas connue ne met jamais en pause.
