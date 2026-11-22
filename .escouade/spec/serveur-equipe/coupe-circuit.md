---
title: "Le coupe-circuit de l'administrateur"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md §9, §3.7, §3.9, §6.1, §6.2, §16 (TS-15)","crates/escouade-team/src/breaker.rs, crates/escouade-team/tests/breaker.rs","crates/escouade-team-server/src/api.rs (ADMIN_STREAMS, TEAM_STREAMS), src/live.rs (is_live)","src-tauri/src/team_breakers.rs, src-tauri/src/teams_breakers.rs, src-tauri/src/locks.rs (LockedBy::Breaker, Lock::denies, Locks::halt)","crates/escouade-policy/src/list.rs (team.maxOfflineHours)","src/components/team/BreakerBand.svelte, src/components/team/TeamBreakers.svelte, src/lib/team-breakers.ts","tests/e2e/team-breaker.spec.ts"]
tickets: ["ESC-546","ESC-579"]
decisions: [229]
compared: null
---

# Le coupe-circuit de l'administrateur

Un administrateur de l'équipe retire quelque chose de l'espace de l'équipe, tout de suite, sur chaque poste : un plugin, un harness, un modèle, ou tous les agents. C'est une entrée signée au journal de l'équipe. Elle dure jusqu'à ce qu'un administrateur la lève. Elle ne fait que restreindre : elle ne lance, n'installe et n'écrit rien. Un coupe-circuit forgé ne serait qu'un déni de service, jamais une prise de contrôle ; il est signé quand même.

## L'entrée

- **Ce qu'elle dit** : sa portée (toute l'équipe, ou un projet de l'équipe, nommé par l'identifiant du journal de son Kanban et par son nom), ce qu'elle retire, sa raison (500 caractères au plus, que chaque membre lit sur sa bande), le nom sous lequel l'administrateur signe (« Claire Martin »), son heure et un **`serial`**.
- **Où elle va** : un flux de l'équipe à part, comme la liste des projets et le fichier d'équipe. Chaque entrée est scellée par la clé d'équipe de son époque et signée par la feuille MLS de son auteur. Une **levée** est une entrée de même sorte, signée elle aussi, qui nomme le coupe-circuit qu'elle lève.
- **Qui l'écrit** : un administrateur, et lui seul.
  - L'app d'un membre refuse avant l'envoi.
  - Le serveur, qui connaît les rôles, ne garde pas l'entrée d'un membre (403).
  - Chaque poste juge à la lecture la signature et le rôle de l'auteur par le journal : une entrée qui n'est pas d'un administrateur de son époque est refusée. Pour une époque que le poste n'a pas vécue, il faut être administrateur aujourd'hui.
  - Une entrée dont le sceau ou la signature ne tient pas (forgée) est refusée.
- **Le `serial`** : chaque entrée porte un numéro plus haut que tous ceux que le poste a lus avant. Une entrée d'un `serial` qui n'est pas plus haut est refusée. Un serveur qui rejoue un vieux coupe-circuit, ou une vieille levée, ne change donc rien. Deux administrateurs qui écrivent en même temps : la seconde entrée lue est refusée, et l'app de son auteur la réécrit au-dessus, sans qu'il ait rien à faire.
- **Un administrateur parti** : un coupe-circuit qu'il a posé tient sur les postes qui ont vécu son époque. L'appareil d'un administrateur encore là le réécrit pour qu'un membre arrivé après le lise aussi.
- **Un serveur qui ment** : comme pour la liste des projets, le poste relit la dernière entrée lue avant de lire plus loin. Une entrée disparue ou changée arrête la lecture, l'écran le dit, « Réessayer » relit. **Ce qui était en vigueur le reste.**

## Ce qu'elle retire

- **Un harness** : plus aucun agent d'un projet de l'équipe ne démarre dessus. Un agent au repos dont le process tourne s'arrête tout de suite. Un agent en plein tour finit son tour, puis s'arrête. Sa conversation dit pourquoi : « Le harness « codex » est retiré par le coupe-circuit de Claire Martin (équipe Acme) : fuite de clé chez le fournisseur X. L'agent s'arrête à la fin de son tour. »
- **Un modèle** : de même, pour les agents qui tournent sur ce modèle. Le modèle est reconnu aussi derrière un fournisseur (`anthropic/claude-opus-5-5`).
- **Tous les agents** des projets de l'équipe : ils s'arrêtent tout de suite, même en plein tour, comme à la sortie de l'app. Le tour en cours est perdu. Chaque agent garde sa session et la reprend à son prochain message, une fois le coupe-circuit levé. Sa conversation dit « Arrêt demandé par Claire Martin : fuite de clé chez le fournisseur X. Le tour en cours est perdu ; l'agent reprendra sa session une fois le coupe-circuit levé. »
- **Un plugin** : il est désactivé sur chaque poste. Sa fiche le dit (« Arrêt demandé par … »), et son interrupteur, celui du membre, n'est pas touché : levé, le plugin revient comme il était. Son installation est refusée tant que le coupe-circuit tient. Quand l'administrateur le demande, il est aussi désinstallé. **Un plugin est de l'app**, comme les autres clés de plugins d'une équipe : sa coupure vaut pour tout le poste. Pour un harness, l'écran conseille de retirer le harness plutôt que son plugin : seul l'espace de l'équipe est alors touché.

## Par le verrou des réglages imposés

Le coupe-circuit passe par le moteur des couches et le verrou des réglages imposés, comme une restriction de rang 5, sans second chemin :

- son `locked_by` le nomme : `{ kind: "breaker", team, by, reason }` ;
- ce qu'il retire est refusé partout où le verrou refuse déjà : chaque démarrage d'un agent (fenêtre, Kanban, pilote auto, MCP, reprise automatique, vue terminal), le harness d'un projet, d'un agent ou d'un ticket, l'installation d'un plugin ;
- dans les listes de la fenêtre, un harness retiré est grisé : « Codex · retiré par le coupe-circuit de Claire Martin (équipe Acme) » ;
- aucun réglage n'est réécrit : levé, tout revient comme avant.

## Sa portée

L'espace de l'équipe seulement. **Il n'arrête jamais un agent d'un projet personnel** : un projet qui n'est d'aucune équipe a les verrous de l'app, qu'aucun coupe-circuit d'équipe n'atteint (sauf un plugin, qui est de l'app). Seule une stratégie du poste géré pourrait le faire, et ce n'est pas le coupe-circuit. Un coupe-circuit d'un projet de l'équipe ne vaut que pour ce projet : les autres projets de l'équipe continuent.

## L'application

- **En ligne : dans la seconde.** Le serveur pousse l'entrée à chaque poste par le direct ; le poste lit les coupe-circuits en premier, avant le reste de l'équipe, et les applique aussitôt. Mesuré dans les tests du cœur, contre le vrai serveur : environ 110 ms entre l'envoi de l'entrée et l'arrêt du process de l'agent de l'autre poste, borné à une seconde.
- **Hors ligne : au retour, avant tout nouveau lancement.** Ce qu'un poste a lu reste appliqué hors ligne, et revient au démarrage depuis son disque. Une équipe dont la dernière lecture a échoué est « à relire » : avant tout démarrage d'un agent de l'un de ses projets, le poste relit ses coupe-circuits (trois secondes au plus). Le serveur revenu, un coupe-circuit posé pendant l'absence s'applique donc avant que quoi que ce soit démarre. Le serveur toujours absent, le démarrage continue avec ce que le poste sait, sauf la stratégie ci-dessous.
- **« Pas de nouvel agent d'équipe sans contact avec le serveur depuis N heures »** : une stratégie du poste géré, `team.maxOfflineHours` (de 1 à 720), dans la liste unique des stratégies (ADMX, profil MDM, `/etc`). Passé ce délai sans réponse du serveur de l'équipe, aucun agent d'un projet de l'équipe ne démarre : « Pas de nouvel agent de l'équipe Acme : son serveur n'a pas répondu depuis le 12/11 à 14:05, et la stratégie de ce poste demande un contact dans les 24 dernières heures. » Les agents qui tournent continuent, et les projets personnels démarrent. Une réponse du serveur lève le refus. L'heure du dernier contact est gardée sur le poste.

## L'écran

- **La bande**, en travers de la fenêtre de chaque poste, dans le style des bandes des certificats et des réglages imposés, tant qu'un coupe-circuit tient : « **Arrêt demandé par Claire Martin : fuite de clé chez le fournisseur X** · Retiré : le harness Codex · toute l'équipe Acme » (et « et 2 autres coupe-circuits »). « Voir » ouvre le coupe-circuit de l'équipe. Sous la stratégie « sans contact », passé le délai, la bande dit « Pas de nouvel agent de l'équipe Acme : son serveur n'a pas répondu depuis plus de 24 h (stratégie de ce poste) · Dernier contact : 12/11/2026 14:05 ».
- **Le coupe-circuit de l'équipe**, un bouton de chaque équipe dans la fenêtre des équipes (où sont ses réglages tant que Réglages › Équipe n'existe pas, TS-4) :
  - **en cours** : chaque coupe-circuit, qui l'a posé, quand, pourquoi, ce qu'il retire et sa portée ; l'heure du dernier contact avec le serveur ; le nombre d'entrées refusées par ce poste ;
  - **levés** : les vingt derniers, avec qui les a levés et quand ;
  - **pour un administrateur** : « Poser un coupe-circuit » (portée : toute l'équipe ou un projet de l'équipe qui suit son Kanban sur ce poste ; ce qui est retiré : un harness de la liste des harness, un modèle, un plugin par son id, avec « Le désinstaller aussi », ou tous les agents ; la raison ; le nom sous lequel il signe). « Continuer » montre la confirmation : ce qui va s'arrêter, la portée, et **sur combien de postes en ligne** (« Sur 3 postes en ligne, dans la seconde ; les autres à leur retour »), puis « Couper ». « Lever » sur chaque coupe-circuit en cours ;
  - un membre voit tout, et ne pose ni ne lève rien.
- **Le MCP** : aucun outil ne pose, ne lève ni ne lit un coupe-circuit. Un outil qui démarrerait un agent sur ce qu'il retire (`create_agent`, `start_ticket`, `update_ticket` avec un compte de ce harness…) le refuse avec la raison.

## Ce que voit le serveur

Qu'une entrée est écrite, par quel appareil, sa taille et son heure ; et qui, parmi les appareils, a une connexion en direct ouverte (ce qu'il sait déjà, et que la confirmation compte). Jamais ce qui est retiré, ni la portée, ni la raison, ni le nom de l'administrateur.

## La liste de révocation de l'équipe

Le coupe-circuit lit aussi la liste de révocation du store privé de l'équipe (page « Le store de plugins privés ») : signée par la clé de l'équipe, son `serial` ne recule jamais. Chaque poste la vérifie à chaque lecture de l'équipe.

- **Un plugin qu'elle retire dans toutes ses versions** (`disable`, versions `*`) est un retrait du coupe-circuit de plus : un refus du verrou que son `locked_by` nomme (`{ kind: "withdrawn", team, reason }`, « Retiré par la liste de révocation de l'équipe Acme : fuite d'un jeton »). Il est désactivé sur le poste, l'interrupteur du membre intact, et son installation est refusée tant que la liste le retire.
- **Un retrait de quelques versions** reste celui de la liste : ces versions-là sont désactivées, les autres non.
- **Quand la liste le reprend**, le plugin revient comme il était.
- **L'écran du coupe-circuit** montre ces plugins sous « Retirés par la liste de révocation de l'équipe », avec la raison de la liste. Elle se change dans « Plugins » de l'équipe, par un administrateur qui tient la clé de l'équipe.
