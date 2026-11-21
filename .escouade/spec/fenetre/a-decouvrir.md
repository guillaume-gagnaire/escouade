---
title: "À découvrir"
subject: "fenetre"
sources: ["docs/ONBOARDING.md","design/onboarding-2-3/README.md","design/onboarding-2-3/maquette.html","design/onboarding-2-3/captures/c-decouverte.png","design/onboarding-2-3/captures/c-decouverte-clair-1024.png"]
tickets: ["ESC-503","ESC-505","ESC-506","ESC-512","ESC-522","ESC-550","ESC-507","ESC-558"]
decisions: [200,215,223,276]
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
| | Importer des tickets | Réglages › Services connectés | un compte d'une source de tickets connecté (le sien, ou celui de l'hébergeur qui sert GitLab Issues, Azure Boards) |
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

- **Les cartes passées au premier lancement** (« Ce que tu veux brancher », ESC-512) s'y retrouvent, à essayer tant qu'elles ne sont pas faites : la source de tickets dans « Importer des tickets », la voix dans « Dicter un message », l'hébergeur dans « Connecter GitHub ». Une carte branchée au premier lancement fait la découverte par l'état réel (le compte connecté).
- Une découverte qui en demande une autre d'abord le dit sous sa station (« après « Connecter GitHub » ») et son bouton devient « D'abord : Connecter GitHub » : le pilote auto et la validation après un ticket, la PR dans Escouade et l'ouverture d'une PR après un hébergeur, la relecture après une PR vue.
- Ce que dit chaque découverte avant le geste : le poids d'un téléchargement (Whisper « base », 148 Mo, et rien de ce qu'on dit ne quitte la machine), le quota qu'un message à un agent consomme (le pilote auto, la conversation vocale, un 2ᵉ agent, « Proposer des commandes », la relecture), le jeton gardé au trousseau et le plugin installé sans réseau d'un hébergeur ou d'une source, le réseau du store, le jeton du serveur MCP qui n'écoute que sur la machine.
- **Rien ne s'installe, ne se connecte, ne se télécharge ni ne s'envoie depuis la découverte** : « Essayer » ouvre le bon écran et pose une bulle sur le vrai contrôle ; le geste reste celui de l'utilisateur, sur ce contrôle, qui dit lui-même son poids ou son coût avant (le micro propose le téléchargement du modèle avec sa taille, la conversation vocale celui de sa voix).
- Sans projet, « Essayer » d'une découverte de projet ouvre « Nouveau projet ». Sans agent, la dictée, la voix et l'ouverture d'une PR montrent « Nouvel agent ».
- Ce que fait le premier lancement n'en coche aucune : le premier projet (un dossier, un clone, la démonstration), son premier agent et la commande que cet agent lance (ce n'est pas « Lancer le projet ») ; un agent par projet n'est pas « Un 2ᵉ agent ».

### Seulement ce qui est possible ici

- Une découverte qui ne peut pas se faire sur ce poste reste sur la carte, grisée, « pas sur ce poste », avec pourquoi : aucun micro (la dictée et la voix), pas de serveur d'équipe connu (l'équipe ; une invitation en apporte un), un réglage imposé qui l'interdit, dans ses mots (`voice.dictation` éteint, `mcp.externalClients` à `denied`, `plugins.allowedOrigins` sans le store). Elle ne compte pas, n'a ni « Essayer » ni bulle, et ne met pas de point sur Aide.
- Ce que cette version n'a pas encore ne se montre pas du tout : Escouade à distance (le téléphone) attend le réglage du mode serveur.
- Une découverte faite reste faite, même si un réglage imposé arrive ensuite ou si l'état change (le ticket supprimé).

### La carte

- En tête : « À découvrir », « 5 sur 18 · chaque station t'amène au vrai geste » (faites, sur ce que ce poste peut faire), « Tout masquer » et « Retour au projet ». Puis le tronc, « Installé · harness, compte, projet, premier agent », et les cinq voies, chacune avec son coude vers le tronc, aux couleurs des voies du git graph.
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

- Aide › À découvrir (« 5 sur 18 : la voix, le Kanban, les PR… », ou « Masqué : la carte reste ici »), et Réglages › Général › « À découvrir » › « Ouvrir la carte ».
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

### Les bulles et les fenêtres (ESC-522)

- Une bulle au bon moment ne s'affiche jamais par-dessus une fenêtre ouverte (un formulaire de ticket, les réglages, une confirmation) : le contrôle qu'elle montrerait est dessous, et Échap est à la fenêtre (il y demande d'abandonner ce qui a été tapé). Un moment qui arrive pendant qu'une fenêtre est ouverte attend qu'elle se ferme, six secondes au plus comme pour la frappe, sinon il passe.
- Une bulle au bon moment déjà à l'écran se ferme quand une fenêtre s'ouvre. Celle que pose « Essayer » reste : le contrôle qu'elle montre peut être dans cette fenêtre (une page des réglages).

## Une fonctionnalité coupée (ESC-550)

Les découvertes d'une fonctionnalité coupée (voir *Fonctionnalités*) quittent la carte, ses bulles et son compte : celles du Kanban (Écrire un ticket, Le pilote auto, Valider un ticket, Les sources de tickets), de l'Espace Docs (La spec et les décisions), de l'Éditeur (L'éditeur) et de la Voix (Dicter un message, Parler à un agent). Elles ne sont ni absentes ni écartées : elles ne sont pas là. Rallumée, la fonctionnalité les ramène ; une découverte faite revient cochée.

## Avec les propositions des fonctionnalités coupées (ESC-558)

Les propositions d'une fonctionnalité coupée au bon moment (voir *Fonctionnalités*) et les bulles au bon moment se partagent la session : **une seule chose au bon moment par session**, une bulle ou une proposition. **La bulle passe d'abord** : elle naît d'un geste fait à l'instant (le micro survolé, le premier ticket, une PR ouverte) et se perdrait sinon ; la proposition naît d'un signal gardé et peut attendre la session suivante. Une bulle à l'écran, ou déjà montrée dans la session : aucune proposition jusqu'au prochain démarrage. Une proposition montrée : plus aucune bulle au bon moment dans la session. La bulle posée par « Essayer » sur le vrai contrôle n'en est pas une : elle vient toujours.

## Ce que comptent les statistiques (ESC-507)

Avec l'accord seulement, comme les autres statistiques d'usage, et l'identifiant de la découverte seul (une liste fermée : `ticket`, `terminal`, `dictation`, `secondAgent`…) :

- `discover_done` : une découverte faite par un geste que l'app voit (un fait du cœur, ou ce que la fenêtre seule voit : l'onglet Docs, un fichier dans l'éditeur, une PR ouverte), avec `from` : `tip` quand une bulle au bon moment l'avait proposée, `other` sinon. Ce que la relecture de l'état réel coche d'elle-même n'en dit rien : sur un poste réglé avant la carte, c'est ce qui était fait avant elle.
- `discover_dismissed` : « Écarter ».
- `discover_hidden` : « Tout masquer » (pas « Réafficher »).
