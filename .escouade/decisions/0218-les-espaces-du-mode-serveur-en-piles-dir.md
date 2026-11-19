---
number: 218
id: "les-espaces-du-mode-serveur-en-piles-dir"
title: "Les espaces du mode serveur en piles (direction B), « Partout » qui répond sans changer d'espace, et ses trois garde-fous"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-539"
pages: ["a-distance/les-espaces","a-distance/se-connecter-a-un-serveur","a-distance/mode-serveur","fenetre/barre-des-projets","fenetre/raccourcis","fenetre/reglages-imposes","fenetre/vue-d-ensemble"]
tickets: ["ESC-539"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/les-espaces","added":41,"removed":0},{"page":"a-distance/se-connecter-a-un-serveur","added":6,"removed":6},{"page":"a-distance/mode-serveur","added":1,"removed":2},{"page":"fenetre/barre-des-projets","added":12,"removed":5},{"page":"fenetre/raccourcis","added":9,"removed":1},{"page":"fenetre/reglages-imposes","added":6,"removed":3},{"page":"fenetre/vue-d-ensemble","added":5,"removed":1}]
accepted_at: "2026-11-19T14:24:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le client du mode serveur (ESC-537) gérait un serveur à la fois. L'utilisateur veut plusieurs serveurs dans un même Escouade (« Escouade pour le taf, et en perso »). docs/SERVER-UX.md proposait trois directions pour ranger les espaces ; la recommandation était A (l'aiguillage). Le 2026-11-18, l'utilisateur a choisi S1 = B, « les piles » : répondre partout sans changer d'espace compte pour lui plus que le cloisonnement strict. B mêle dans une vue ce que tout le reste cloisonne : il fallait des garde-fous pour qu'on n'autorise jamais une commande sur le mauvais serveur et qu'un poste géré puisse refuser la vue agrégée.

## Décision

1. Direction B : l'espace ouvert en tête de ses projets dans la barre des projets, chaque autre espace en pile sous le « + » (deux de ses projets en cartes, ce qui y attend en pastille) ; la vue d'ensemble devient « Partout » (onglet par défaut dès deux espaces, un onglet par espace pour filtrer), d'où l'on accorde une autorisation lisible en entier sans changer d'espace ; le reste s'ouvre dans sa conversation. Un seul espace : la fenêtre d'avant, inchangée.
2. Chaque serveur connu garde son tunnel et sa connexion, chacun ses essais : une coupure n'en touche aucun autre. La fenêtre lit chaque autre espace par une commande servie légère (`space_glance`, toutes les 3 s), jamais sa conversation.
3. Les trois garde-fous : (a) la machine est écrite sur chaque demande de « Partout » et sur le geste qui y répond (« Autoriser sur Travail »), et un espace coupé grise ses demandes, sans aucun geste ; (b) `servers.aggregateView` à `false` ne laisse d'un serveur de travail (de `servers.allowed`, ou tout serveur sans liste) que ses nombres, ni texte ni geste, les textes ne venant même pas à la fenêtre ; posée sur le serveur, elle vaut pour lui ; `servers.contentlessNotifications` retire le texte de ses notifications ; (c) les piles se tassent en plaques au-delà de trois espaces ou à 1024 px, pour que la barre ne défile pas.
4. `servers.personalSpace` à `false` (S16 par défaut) : seuls les serveurs de `servers.allowed` s'ajoutent, aucun sans liste ; ce poste reste un espace. Les trois clés `servers.*` sortent de « à venir » dans la liste unique et se montrent verrouillées dans Réglages › Serveurs.
5. Les autres choix gardent leur réponse par défaut : `Alt+1`…`Alt+9` (`⌃` sous macOS) et `Ctrl+²` (S3), une question à l'écran gardant `Alt+n` pour ses options ; un espace dans sa fenêtre, une au plus (S14) ; Réglages de l'espace › Appareils (lister, retirer, créer un code, S5) ; retirer un serveur (S15) ; notifications nommées par leur espace.

## Conséquences

Un tunnel `ssh` et une WebSocket par serveur restent ouverts tant que l'app tourne (mesuré par l'e2e `spaces` : la mémoire de trois connexions au bilan d'ESC-539). « Partout » a 3 s de retard au plus sur un autre espace. Une autorisation accordée de « Partout » part par la connexion de son espace, jamais celle de la fenêtre. `Alt+n` change d'espace dans un terminal et dans la vue VS Code (qui perd ces touches et `Ctrl+²`). Avec ESC-538 : les ports relayés appartiennent au serveur où est la fenêtre et s'arrêtent quand elle en change ; ce que publie ce serveur vient de la connexion de la fenêtre, et les demandes des autres serveurs sont notifiées par la lecture de « Partout », chacune nommée par son espace, jamais deux fois.
