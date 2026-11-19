---
title: "À découvrir"
subject: "fenetre"
sources: ["docs/ONBOARDING.md","design/onboarding-2-3/README.md","design/onboarding-2-3/maquette.html","design/onboarding-2-3/captures/c-decouverte.png","design/onboarding-2-3/captures/c-decouverte-clair-1024.png"]
tickets: ["ESC-503","ESC-505","ESC-506"]
decisions: [200,215]
compared: null
---

## Ce que c'est

Après le premier lancement, ce qu'Escouade sait faire se découvre **en le faisant pour de vrai**, une chose à la fois. Aide › À découvrir est une page de l'app : la ligne du premier lancement se prolonge en cinq voies du cycle, Planifier, Coder, Relire, Livrer, Piloter (direction C « La ligne » de `docs/ONBOARDING.md`, avec les bulles au bon moment de la direction B). Chaque station est une découverte : une phrase (ce que c'est, à quoi ça sert), ce qu'elle demande avant, et « Essayer », qui mène au vrai écran et montre le vrai contrôle. Elle se coche quand l'app a vu le geste fait, jamais par un clic sur sa carte.

## Comportement

### Les découvertes

| Voie | Découverte | « Essayer » mène à | Cochée quand |
| --- | --- | --- | --- |
| Planifier | Écrire un ticket | le Kanban, sur « Nouveau ticket » | un ticket existe ou a été créé |
| | Le pilote auto | le Kanban, sur son interrupteur | il a lancé un ticket de lui-même (il est allumé par défaut : l'interrupteur ne compte pas) |
| | La spec et les décisions | l'onglet Docs | l'onglet Docs ouvert |
| Coder | Dicter un message | la conversation d'un agent, sur le micro | une dictée a donné du texte |
| | Parler à un agent | la conversation d'un agent, sur le bouton de la conversation vocale | une conversation vocale a commencé |
| | Un terminal | la barre du projet, sur le « + » des terminaux | un terminal ouvert |
| | L'éditeur | le bouton « Éditeur » de la barre de titre | un fichier ouvert dans l'éditeur |
| | Un 2ᵉ agent | « Nouvel agent » | deux agents de l'utilisateur dans un projet (ni ceux des tickets, ni les discussions des Docs) |
| Relire | Lancer le projet | Réglages du projet › Lancement (jamais « Proposer des commandes » de lui-même) | une commande de lancement démarrée |
| | Connecter GitHub | Réglages › Services | un hébergeur de code connecté |
| | Une PR dans Escouade | la vue des PR du projet | une PR ouverte dans l'app |
| | Relecture par un agent | la vue des PR | une relecture demandée à un agent |
| Livrer | Valider un ticket | le Kanban, sur la colonne « À tester » | un ticket validé, ou un ticket dans « Terminé » |
| | Ouvrir une PR | la conversation d'un agent, le panneau « PR et CI » | une PR ouverte par Escouade, ou un ticket qui a la sienne |
| Piloter | Un plugin | la vue Plugins | un plugin installé depuis le store ou un dépôt (pas celui que l'amorce installe pour un harness choisi) |
| | Le serveur MCP | Réglages › Agents | le serveur MCP allumé |
| | Un 2ᵉ compte | Réglages › Comptes et harness | deux comptes de harness |
| | Ton équipe | « Rejoindre une équipe » | une équipe rejointe ou créée, ou un projet d'équipe |

- Une découverte qui en demande une autre d'abord le dit sous sa station (« après « Connecter GitHub » ») et son bouton devient « D'abord : Connecter GitHub » : le pilote auto et la validation après un ticket, la PR dans Escouade et l'ouverture d'une PR après un hébergeur, la relecture après une PR vue.
- Ce que dit chaque découverte avant le geste : le poids d'un téléchargement (Whisper « base », 148 Mo, et rien de ce qu'on dit ne quitte la machine), le quota qu'un message à un agent consomme (le pilote auto, la conversation vocale, un 2ᵉ agent, « Proposer des commandes », la relecture), le jeton gardé au trousseau et le plugin installé sans réseau d'un hébergeur, le réseau du store, le jeton du serveur MCP qui n'écoute que sur la machine.
- **Rien ne s'installe, ne se connecte, ne se télécharge ni ne s'envoie depuis la découverte** : « Essayer » ouvre le bon écran et pose une bulle sur le vrai contrôle ; le geste reste celui de l'utilisateur, sur ce contrôle, qui dit lui-même son poids ou son coût avant (le micro propose le téléchargement du modèle avec sa taille, la conversation vocale celui de sa voix).
- Sans projet, « Essayer » d'une découverte de projet ouvre « Nouveau projet ». Sans agent, la dictée, la voix et l'ouverture d'une PR montrent « Nouvel agent ».
- Ce que fait le premier lancement n'en coche aucune : le premier projet (un dossier, un clone, la démonstration), son premier agent et la commande que cet agent lance (ce n'est pas « Lancer le projet ») ; un agent par projet n'est pas « Un 2ᵉ agent ».

### Seulement ce qui est possible ici

- Une découverte qui ne peut pas se faire sur ce poste reste sur la carte, grisée, « pas sur ce poste », avec pourquoi : aucun micro (la dictée et la voix), pas de serveur d'équipe connu (l'équipe ; une invitation en apporte un), un réglage imposé qui l'interdit, dans ses mots (`voice.dictation` éteint, `mcp.externalClients` à `denied`, `plugins.allowedOrigins` sans le store). Elle ne compte pas, n'a ni « Essayer » ni bulle, et ne met pas de point sur Aide.
- Ce que cette version n'a pas encore ne se montre pas du tout : Escouade à distance (le téléphone) attend le réglage du mode serveur.
- Une découverte faite reste faite, même si un réglage imposé arrive ensuite ou si l'état change (le ticket supprimé).

### La carte

- En tête : « À découvrir », « 5 sur 17 · chaque station t'amène au vrai geste » (faites, sur ce que ce poste peut faire), « Tout masquer » et « Retour au projet ». Puis le tronc, « Installé · harness, compte, projet, premier agent », et les cinq voies, chacune avec son coude vers le tronc, aux couleurs des voies du git graph.
- Une station : pleine faite (« fait »), creuse à essayer (ce qu'elle demande : « 148 Mo, local », « un jeton, au trousseau »), en pointillés écartée, grise pas sur ce poste. Celle choisie a un anneau ; par défaut, la première à essayer qui ne demande rien d'abord.
- À droite (en 1024 × 728, en bas), le détail de la station choisie : son nom et son icône, sa phrase, « Avant le geste » et ce qu'elle demande, puis « Essayer » et « Écarter » ; « Y retourner » une fois faite ; « Remettre » une fois écartée ; pourquoi elle n'est pas là.
- Les stations sont des boutons, au clavier ; le détail est annoncé poliment au lecteur d'écran. En 1024, les lignes sous les stations et la légende se cachent, et toutes les stations restent visibles au-dessus du détail. Thèmes clair et sombre.
- Elle s'ouvre aussi depuis la fin du premier lancement : « Voir ce qu'il y a à découvrir » à côté de « Commencer » ; jamais d'elle-même.

### Écarter, tout masquer

- « Écarter » (sur la carte ou dans une bulle) : la découverte ne sera plus jamais proposée d'elle-même, ni bulle ni point ; elle reste sur la carte, « écarté », et « Remettre » la replace. Une découverte faite ne s'écarte pas.
- « Tout masquer » : plus aucune bulle, plus de point sur Aide ; la carte reste dans Aide › À découvrir, avec « Masqué : plus de bulle ni de point sur Aide. La carte reste ici, dans Aide › À découvrir. » et « Réafficher ».

### Les bulles au bon moment

- Trois moments seulement (choix O11) : le micro survolé (la dictée, sur le micro, quand le pointeur s'y pose sept dixièmes de seconde sans appuyer : un appui ouvre son propre panneau, que la bulle ne couvre pas), le premier ticket créé dans un projet (le pilote auto, sur son interrupteur, une fois le formulaire fermé) et une PR ouverte dans l'app (la relecture par un agent, sur « Relire avec un agent »).
- **Une seule bulle à la fois, au plus une par session, jamais deux fois la même**, jamais pendant que l'utilisateur tape (une touche dans la seconde et demie), jamais pendant qu'un agent attend une permission, jamais tant que le premier lancement est à l'écran (le micro du compositeur de son premier agent compris), jamais sur une découverte faite, écartée, pas sur ce poste, ni après « Tout masquer ».
- Un moment qui arrive pendant la frappe, une permission ou le premier lancement attend qu'ils cessent (six secondes au plus), sinon il passe.
- Une bulle dit la phrase de la découverte et ce qu'elle demande avant, avec « Compris », « Écarter » et « Tout ce qui reste à découvrir » (la carte). Un halo teal entoure le vrai contrôle. C'est un dialogue non modal, annoncé poliment ; Échap la ferme. Elle suit le contrôle et part avec lui.
- **Elle ne gêne jamais** : un appui ailleurs que sur ses boutons la ferme et va à ce qu'il visait (elle laisse passer les clics hors de ses boutons) ; appuyer sur le contrôle qu'elle montre, ou se mettre à taper, la ferme aussi.
- La bulle posée par « Essayer » sur le vrai contrôle n'a que « Compris ».

### Où elle se retrouve

- Aide › À découvrir (« 5 sur 17 : la voix, le Kanban, les PR… », ou « Masqué : la carte reste ici »), et Réglages › Général › « À découvrir » › « Ouvrir la carte ».
- Un point teal sur l'icône Aide du rail tant qu'une découverte possible ici n'est ni faite ni écartée, sauf après « Tout masquer » ; le lecteur d'écran l'entend (« Il reste des choses à découvrir »).

### Nouveau dans la version

- Un poste réglé avant la 2.3 (des projets, pas une nouvelle installation) voit une fois, en haut de la place de travail, « Nouveau dans la 2.3 : les PR dans Escouade, l'éditeur VS Code, ton équipe. », « Voir » et ×. « Voir » ouvre la carte sur ce qui est nouveau (« Nouveau dans la 2.3 · Tout voir ») ; l'un ou l'autre la ferme pour cette version. Une version mineure n'affiche rien ; une nouvelle installation ne la voit jamais. Elle apparaît quand la version de l'app est une 2.3.

### L'état gardé

- Par installation, dans `state.json` (avec le premier lancement), jamais dans un projet : chaque découverte faite ou écartée, « Tout masquer », les bulles déjà montrées, la dernière version dont la bande a été vue.
- Le cœur coche une découverte quand il voit le geste (les mêmes moments que les statistiques d'usage comptent, accord ou non), et d'après l'état réel à chaque lecture ; la fenêtre ne lui dit que ce qu'elle seule voit (l'onglet Docs, un fichier dans l'éditeur, une PR ouverte). Un client de l'API distante ne voit ni point, ni bande.

## Réglages

- Aucun à elle. Réglages › Général › « À découvrir » ouvre la carte.

## Limites

- Les événements de statistiques de la découverte (`discover_done`, `discover_dismissed`, `discover_hidden`) viennent avec ON-4 (ESC-507), avec la page de confidentialité du site.
- Escouade à distance n'est pas une découverte tant que le mode serveur n'a pas son réglage.
- L'absence de sortie son n'est pas lue : seule l'absence de micro retire la voix.
