# Réglages : Notifications

La page « Notifications » règle les alertes d’Escouade : pour quels événements elle prévient, si elle affiche une notification du système quand la fenêtre n’est pas au premier plan, et si elle joue un son.

## Comment Escouade te prévient

Trois signaux se combinent, et la page commande les deux derniers. Les alertes elles-mêmes sont décrites dans [les notifications](notifications).

- Les signaux de la fenêtre : la tuile du projet, la carte de l’agent et la barre des tâches (le Dock sous macOS) signalent toujours l’agent, quels que soient les réglages. La barre des tâches clignote quand la fenêtre n’est pas au premier plan.
- Le carillon : joué pour un type d’événement coché, quand « Son activé » l’est aussi, que la fenêtre soit au premier plan ou non.
- La notification du système : affichée pour un type d’événement coché, quand l’interrupteur du groupe « Canaux » est activé et que la fenêtre n’est pas au premier plan, c’est-à-dire cachée, réduite ou sans le focus.

## Me prévenir pour

Quatre interrupteurs choisissent les types d’événements qui font sonner et qui montrent une notification du système. Un type désactivé ne fait ni l’un ni l’autre ; les signaux de la fenêtre, eux, restent.

| Réglage | Prévient quand | Par défaut |
|---|---|---|
| « Questions et autorisations » | Un agent pose une question ou attend une autorisation. | Activé |
| « Tâches terminées » | Un agent a terminé son tour. Pas quand tu as interrompu le tour, ni quand un message attend dans la file d’attente de l’agent. | Activé |
| « Erreurs » | Un agent s’arrête sur une erreur. | Activé |
| « Tickets (prêt à tester, bloqué) » | Un ticket du Kanban est prêt à tester, ou bloqué. | Activé |

> [!NOTE]
> Un agent dont le ticket est « En cours » ne prévient que pour ses questions et ses autorisations. Pour le reste, c’est le [Kanban](board) qui prévient, par le type « Tickets », quand le ticket est prêt à tester ou bloqué.

### Pull requests et déploiements

Huit interrupteurs de plus, tous activés par défaut, pour ce qu’un [hébergeur de code](code-hosts#les-nouvelles-des-hebergeurs) a de nouveau : « CI en échec », « Changements demandés », « PR approuvée », « PR mergée », « Conflit », « Relecture demandée », « Déploiement en production réussi » et « Déploiement en échec ». Escouade les signale quand elle relit ce qu’elle suit. Un type désactivé ne dit plus rien.

## Canaux

| Réglage | Effet | Par défaut |
|---|---|---|
| « Notifications Windows quand l’app n’est pas au premier plan » | Affiche une notification du système quand un événement prévu par « Me prévenir pour » arrive et que la fenêtre n’est pas au premier plan. Le libellé nomme le système : « Notifications macOS … » sous macOS, « Notifications Windows … » partout ailleurs. | Activé |

La notification a pour titre « projet · agent » et, pour corps, la question, la réponse ou l’erreur de l’agent. Pour un ticket, le titre est le projet et le corps dit, par exemple, « ATL-42 prêt à tester ». Sous Windows, un clic dessus ramène la fenêtre et affiche l’agent ou le Kanban concerné. Sous macOS, il ne fait que ramener l’app au premier plan. Sous Linux, l’interrupteur est affiché (avec le libellé « Windows »), mais Escouade n’affiche pas de notification du système : le son et les signaux de la fenêtre restent.

## Son

| Réglage | Effet | Par défaut |
|---|---|---|
| « Son activé » | Joue un carillon pour les événements prévus par « Me prévenir pour » (« Question de Claude, fin de tour »). Le même carillon pour tous les types. Sous Linux, il est joué par `paplay`, qui doit être installé. | Activé |
| « Tester le son » | Le bouton « ▶ Tester » joue le carillon tout de suite. | — |

Le bouton ♪ de la barre d’état (info-bulle « Son des notifications », texte « On » ou « Off ») bascule le même réglage, et l’enregistre aussitôt, sans ouvrir la fenêtre des réglages.
