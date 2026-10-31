---
title: "Barre de statut"
subject: "Fenêtre"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

La ligne au bas de la fenêtre. Elle dit d'un coup d'œil combien d'agents travaillent ou attendent, où en est le quota du compte en cours et ce qu'a coûté la journée. Elle donne aussi la branche du projet affiché, l'état de la mise à jour, le son et les réglages.

## Comportement

De gauche à droite : actifs · en attente · terminés │ quotas │ coût du jour │ ⎇ branche ↑ à pousser ↓ à tirer │ mise à jour │ ♪ · ⚙.

### Quotas

- Ce sont ceux du compte en cours : celui où partiraient les nouveaux agents. C'est le premier compte actif « bon » au plus bas des seuils « Pause au-delà du quota » et « Changer de compte à » : une fenêtre de 5 h au seuil l'écarte jusqu'à sa remise à zéro, une semaine, 30 jours ou un mois de crédits se juge à sa projection (voir *Comptes*, « Un compte « bon », fenêtre par fenêtre »). Sinon le premier dont seul le rythme est mauvais, sinon le premier compte actif. Comment ils sont lus : voir *Comptes*.
- Affichage : `5h <barre> reset 3h01 → 64 %` · `7j <barre> reset 4j 12h → 85 %` ; en anglais `5h … reset 3h01 → 64%` · `W … reset 4d 12h → 85%`. Pour la fenêtre de 5 h puis la fenêtre hebdomadaire : une barre, « reset » et le temps restant, puis la projection (ci-dessous).
- Tous les harness à quota l'ont : les fenêtres de Claude Code et de Codex (5 h, 7 jours, 30 jours avec l'offre gratuite), et les crédits du mois de Copilot CLI, pris comme une fenêtre d'un mois qui finit à leur remise à zéro, le 1er à 00:00 UTC.
- Le pourcentage n'est pas écrit. Il est dans l'infobulle, avec la remise à zéro (« 42 % · remise à zéro le 10/10 à 18:00 »), au survol de la barre et à son focus clavier.
- Chaque barre est une jauge que les lecteurs d'écran lisent. Elle passe à la couleur d'alerte au-delà de 80 % (pas à 80 %).
- Quota non lu : « Quota indisponible ».
- Libellés, infobulles et panneau suivent la langue de l'interface.

### Projection

- La projection dit où en sera la fenêtre à sa remise à zéro, au rythme actuel : le pourcentage utilisé rapporté au temps écoulé de la fenêtre.
- Couleur : vert jusqu'à 80 %, orange au-delà, rouge au-delà de 100 % (la fenêtre serait épuisée avant sa fin). Arrondie comme elle est écrite ; « 999+ % » au-delà.
- Aucune projection trop tôt :
  - pour une fenêtre d'un jour ou moins, avant son dixième (30 min sur 5 h), où un tour ou deux la projetteraient bien au-delà de 100 % ;
  - pour une fenêtre plus longue (7 jours, un mois), avant une journée entière, pour que les nuits comptent : sinon le travail d'une matinée, étalé sur la semaine comme s'il continuait jour et nuit, rendrait rouge chaque début de semaine ;
  - pour une fenêtre sans remise à zéro connue.
- La même projection, calculée par le cœur avec les mêmes chiffres, juge les fenêtres longues dans le choix du compte (voir *Comptes*) : une semaine projetée à 100 % ou moins reste un bon choix, une projetée au-delà passe derrière les autres.
- Pour un harness sans quota (OpenCode), le coût du jour est projeté à minuit au rythme depuis minuit (« 0,75 $ / 2,00 $ → 1,00 $ »), avec les mêmes couleurs rapportées au « Budget par jour ($) ». Seulement s'il a un budget, et pas avant 2 h 24 (le dixième de la journée).
- Où elle est écrite, selon « Projection des quotas » :
  - « Au-delà de 80 % » (défaut) : seulement les projections orange et rouges ;
  - « Toujours » : toutes, les vertes aussi ;
  - « Jamais » : aucune, nulle part.
- Sous 1240 px de large, la barre de statut n'écrit aucune projection, quel que soit le choix.
- Sauf sur « Jamais », le panneau des comptes les écrit toutes, et l'infobulle de chaque jauge (et la description du groupe des quotas) la dit après la remise à zéro : « 42 % · remise à zéro le 10/10 à 18:00 · projection à la remise à zéro : 64 % » ; pour le coût du jour, « Projection à minuit : 1,00 $. »
- Un réglage d'avant 2.0.2, quand c'était un interrupteur, se lit sans perte : allumé donne « Au-delà de 80 % », éteint « Jamais ». Une valeur inconnue donne le défaut.

### Plusieurs comptes

- Le nom du compte en cours précède les quotas (« Pro · 5h … »), coupé à 160 px, entier dans l'infobulle.
- Tout le groupe est alors un bouton, nommé « Quotas par compte (compte en cours : Pro) » ; les deux fenêtres sont dites aux lecteurs d'écran. Son infobulle ne s'affiche qu'au focus clavier, et Échap la retire.
- Il ouvre le panneau « Quotas des comptes », au-dessus de la barre :
  - le compte en cours en tête, marqué « en cours », puis les autres dans l'ordre des réglages ;
  - pour chacun, ses deux fenêtres (barre, pourcentage écrit, remise à zéro) et « Aujourd'hui : 1,20 $ » (« ≈ » tant qu'un tour tourne) ;
  - « au-delà du seuil de pause ou de changement de compte » pour un compte que le choix du compte écarte : une fenêtre de 5 h au seuil de « Pause au-delà du quota » ou de « Changer de compte à », une fenêtre longue à sa limite (ou au seuil sans projection). Une semaine au-dessus du seuil mais projetée à 100 % ou moins n'est pas marquée ;
  - « rythme trop rapide pour sa fenêtre » pour un compte que seule la projection d'une fenêtre longue met derrière les autres (au-delà de 100 %), sous le seuil comme au-dessus ; la marque d'écart l'emporte quand les deux valent ;
  - un compte inactif, pas connecté (« Pas connecté ») ou à la connexion expirée (« Connexion expirée : relance Claude Code pour ce compte. ») est grisé, avec sa raison.
- Échap, un clic dehors ou un nouveau clic sur le groupe ferment le panneau. Le focus va dans le panneau à l'ouverture et revient au bouton à la fermeture.
- Avec un seul compte, le groupe n'est pas un bouton et n'écrit pas de nom.

### Coût du jour

- Le total de tous les comptes : la somme des « Aujourd'hui » du panneau.
- C'est un équivalent API (le coût qu'auraient les mêmes tokens facturés à l'usage, même sur un abonnement Max).
- Le coût exact d'un tour vient de Claude Code à la fin du tour. Pendant le tour, tokens et coût montent en direct, préfixés « ≈ », estimés d'après les tarifs publics, puis remplacés par le chiffre exact. Un modèle dont le tarif n'est pas connu n'a pas d'estimation. Les statistiques n'enregistrent que les coûts exacts. Le coût des autres harness, exact ou estimé : voir *Harness*.

### Branche et synchro git

- Pour le projet affiché, si son dossier est un dépôt git. Une HEAD détachée s'écrit « HEAD détachée ».
- Avec un dépôt distant : l'avance et le retard à côté du nom (« ↑2 ↓5 ») ; ↓ en couleur d'attente quand il y a des commits à tirer. Infobulle : la branche suivie et le dernier fetch. « non publiée » ou « distante supprimée » quand il n'y a pas de branche suivie.
- Clic : le sélecteur de branche, au-dessus de la barre (voir *Branches*).

### Mise à jour

- « Mise à jour X… » pendant le téléchargement, puis « Mise à jour X prête · Redémarrer », qui ouvre la fenêtre de la mise à jour.
- Pendant les 30 s qui précèdent un redémarrage automatique : « Redémarrage dans N s », avec « Plus tard ». Voir *Livraison et mises à jour*.

### Son et réglages

- ♪ coupe ou rend le carillon (voir *Notifications*). ⚙ ouvre les réglages (voir *Réglages*).

## Réglages

- Réglages › Comptes › « Quotas » › « Projection des quotas » : « Au-delà de 80 % » (défaut), « Toujours », « Jamais ». Enregistré tout de suite, comme le reste de l'onglet.
- Les seuils « Pause au-delà du quota » et « Changer de compte à » : voir *Comptes*.

## Limites

- Les harness sans quota lisible (Cursor CLI, Mistral Vibe, Antigravity CLI, Gemini CLI) n'ont pas de jauge (voir *Harness*).
