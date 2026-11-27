---
number: null
id: "ce-que-voit-l-utilisateur-du-pilotage-ba"
title: "Ce que voit l'utilisateur du pilotage : bande toujours là, overlay hors du DOM sous Windows, repères numérotés (AB-8)"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-680"
pages: ["kanban/navigateurs-pilotes","harness/pages-des-agents"]
tickets: ["ESC-680","ESC-679"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":19,"removed":6},{"page":"harness/pages-des-agents","added":12,"removed":5}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AB-7 (ESC-679) a construit drive_browser et la règle de la main dans le cœur, sans rien montrer : le contour d'un highlight était un élément d'Escouade dans le DOM de la page, sous Windows comme sous WebKit, et aucune bande ni rangée ne disait qu'un agent pilotait. Le ticket ESC-680 (AB-8 avec AP-6) devait montrer le pilotage dans la direction A choisie par l'utilisateur (a-36), avec trois exigences de sécurité : l'utilisateur voit toujours qu'un agent pilote, « Reprendre la main » arrête l'agent dans le cœur et pas seulement à l'écran, le curseur et le surlignage sont hors d'atteinte du script de la page.

## Décision

1. La bande de pilotage commune (DriveBand) est branchée sur toute ligne pilotée, onglet ou page : dès le premier geste d'un agent et jusqu'à la fermeture de la ligne, sans exception ; « Reprendre la main », « La rendre », « Continuer », « Tu ne laisses plus piloter ». Pendant le pilotage, la légende du geste passe avant la mention de confiance, qui se replie sur son bouclier dans un volet étroit.
2. « Reprendre la main » passe par le cœur (browser_drive_hand) : le geste suivant est refusé, un geste vérifié mais pas commencé ne se fait pas (la main relue juste avant), un wait s'arrête, un wait_user rend took_hand.
3. Sous Windows, le curseur fantôme (Overlay.highlightQuad, une flèche à la couleur du projet) et le surlignage (Overlay.highlightNode sur l'objet de l'élément trouvé dans le monde isolé) sont peints par l'overlay du navigateur, absents du DOM de la page. L'overlay ne dessine que des formes : pas d'initiales sur le curseur ni d'étiquette sur le surlignage, que la légende de la bande remplace. La liste fermée du protocole gagne DOM.enable, Overlay.enable et Overlay.highlightQuad, aux paramètres vérifiés ; rien d'autre du domaine DOM ni de l'overlay. Sous WebKit, des éléments d'Escouade (ombre racine fermée) avec initiales et étiquette, que la page peut voir ou retirer.
4. Chaque geste réussi est une rangée numérotée dans la conversation (le numéro du cœur, le même dans la bande) et, sur une page d'agent, un repère d'un rail natif de 26 px le long de la page, posé où l'agent avait mis la page ; un repère ou une rangée cliqués y ramènent la page, sans prendre la main. La ligne pilotée de « Navigateurs » porte les initiales de l'agent et son menu les gestes de la main.

## Conséquences

L'utilisateur voit et arrête le pilotage partout ; la page ne peut ni lire ni effacer ce que l'agent montre sous Windows. Sous Windows, l'étiquette d'un surlignage n'est plus dans la page mais dans la bande. La couleur du projet (oklch) est convertie en sRGB par le cœur. Le curseur et le surlignage de WebKit n'ont pas tourné sur un Mac ni un Linux : AB-12 et la CI les essaieront. AB-9 (l'écran : « Laisser piloter par… », un volet par agent) reste à faire.
