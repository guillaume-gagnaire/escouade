---
title: "Réglages imposés"
subject: "fenetre"
sources: ["docs/TEAM-SERVER.md §6.1, §6.2, §6.3, §6.4, §6.5, §16 (TS-11, TS-12, TS-13)","crates/escouade-policy/README.md (TS-0b ESC-454, TS-12 ESC-465)","crates/escouade-policy/src/list.rs (la liste unique)","src-tauri/src/team_settings.rs (TS-13)"]
tickets: ["ESC-455","ESC-465","ESC-471","ESC-478","ESC-481","ESC-497"]
decisions: [169,175,183,189,201,198,197]
compared: null
---

# Réglages imposés

Un service informatique, ou une équipe, peut **imposer** un réglage : il ne se change plus sur le poste. Ça sert même sans serveur d'équipe, à une entreprise qui gère sa flotte par ses propres outils (stratégies de groupe, Intune, un MDM, `/etc`). L'équipe impose par son `.escouade.yml` signé (voir « Le .escouade.yml d'équipe »).

## Les six rangs

Un réglage prend sa valeur du rang le plus fort qui la donne :

| Rang | Source | Peut |
| --- | --- | --- |
| 1 | Les valeurs par défaut d'Escouade | tout |
| 2 | L'équipe, recommandé (`defaults` du fichier d'équipe) | proposer une valeur, que le dépôt et le poste changent |
| 3 | Le dépôt (`.escouade.yml`) | comme avant |
| 4 | Le poste (Réglages, « Réglage local ») | passer devant le fichier, comme avant |
| 5 | L'équipe, imposé (`enforced` du fichier d'équipe) | verrouiller une valeur ou une borne, pour les projets de l'équipe |
| 6 | Le poste géré (stratégies du poste) | verrouiller pour toute l'app |

- Les rangs 2 à 4 gardent l'ordre d'avant. Les rangs 5 et 6 passent devant tout.
- **Entre l'équipe et le poste géré**, pour une même clé :
  - une **borne** (les modes permis, les harness permis, les serveurs permis) se croise : il reste ce que les deux permettent ;
  - un **plafond** (l'effort au plus, ce que peuvent les clients MCP extérieurs) : le plus bas tient ;
  - une **liste d'interdits** (plugins interdits, motifs de fichiers) : les deux s'ajoutent ;
  - une **valeur** : celle du poste géré gagne.
- **Ce qu'impose une équipe ne vaut que pour ses projets** : les modes, les harness, les modèles, l'effort, la clôture et le Remote Control d'un projet de l'équipe ; un projet perso garde les seuls verrous du poste géré. Les clés de l'app (réseau, télémétrie, voix, mises à jour, plugins, clients MCP extérieurs) valent pour l'app tant que le poste est dans l'équipe, en attendant les espaces (TS-4).
- **Chaque clé a sa sorte** : restreindre, valeur, sensible ou exécutable.
  - Une clé qui *restreint* ne s'impose que dans son sens : « Statistiques d'usage » éteintes, jamais allumées ; le canal stable, jamais la bêta. Une autre valeur est ignorée et dite.
  - Une clé *exécutable* (commande, hook, lancement) n'est **jamais** imposée par l'équipe, même signée ; seul le poste géré le peut. L'équipe peut la proposer : chaque poste la relit et l'accepte à son empreinte.
  - Une clé *sensible* (adresse du proxy, exclusions, fichier d'autorités) s'applique quand elle vient du poste géré ; venue de l'équipe, elle attend l'accord de ce poste, puis s'applique comme une valeur (celle du poste géré gagne).

## La liste unique des stratégies

Une seule liste, dans `crates/escouade-policy/src/list.rs`, dit pour chaque clé son **type** (booléen, liste, chaîne, nombre, JSON), sa **sorte**, sa forme (interrupteur, choix, plafond, borne, liste d'interdits, texte, JSON), les lignes des Réglages qu'elle verrouille, et ses textes en français et en anglais. C'est la table du cœur (`locks::KEYS`) : rien d'autre ne dit quelles clés existent.

En sont générés, et un test vérifie que les fichiers du dépôt sont ce qu'elle génère :

- `policies/windows/escouade.admx`, `fr-FR/escouade.adml`, `en-US/escouade.adml` (stratégies de groupe, Intune) ;
- `policies/macos/dev.gagnaire.escouade.plist` (manifeste au format ProfileManifests), `dev.gagnaire.escouade.json` (schéma de Jamf Pro), `escouade-example.mobileconfig` (profil d'exemple) ;
- `policies/linux/escouade-policies.schema.json` et `example.json` (`/etc/escouade/policies`) ;
- la page du site « Stratégies du poste » (`/docs/managed-policies`, en anglais « Device policies ») : ce que fait chaque stratégie, comment la déployer, ce que voit l'utilisateur.

Un `REG_DWORD` 0 ou 1 devient un booléen quand la liste le dit ; une chaîne seule vaut une liste d'un élément ; un réglage JSON s'écrit dans le registre comme une chaîne JSON.

## Les clés qu'on peut imposer

| Clé | Sorte | Ce qu'elle verrouille ou refuse |
| --- | --- | --- |
| `permissions.allowBypass` | restreindre (éteint seulement) | Réglages › Agents › « Autoriser le mode Bypass » |
| `permissions.modes` | restreindre, une borne (`plan`, `default`, `acceptEdits`, `auto`, `bypassPermissions`) | les modes des agents, le « Mode par défaut des agents », le mode des agents du Kanban |
| `permissions.defaultMode` | valeur (dans la borne des modes) | Réglages › Agents › « Mode par défaut des agents » |
| `harnesses.allowed` | restreindre, une borne (ids des harness) | le harness d'un projet, d'un agent, d'un ticket |
| `harnesses.allowedModels` | restreindre, JSON `{harness: [modèles]}` | un agent ou un ticket sur un modèle non permis, refusé à son démarrage et à son changement de modèle |
| `harnesses.maxEffort` | restreindre, un plafond (`low` à `max`) | l'effort du Kanban et d'un agent : ramené au plafond pour un nouvel agent, refusé quand on le demande plus haut |
| `plugins.allowedOrigins` | restreindre, une borne (`store-verified`, `store`, `team-store`, `git`, `folder`, `dev`) | l'installation d'un plugin d'une autre origine, refusée |
| `plugins.allowed`, `plugins.denied` | restreindre (borne ; liste d'interdits) | l'installation d'un plugin hors de la liste, ou de la liste des interdits, refusée |
| `plugins.verifiedOnly` | restreindre (allumé seulement) | Plugins › « N'installer que des versions vérifiées » |
| `extensions.allowed` | restreindre, JSON comme `extensions.allowed` de VS Code (`{"*": false, "dracula-theme": true}`) ; le poste géré et l'équipe croisés avec les réglages du poste et les `.escouade.yml` : chacun doit permettre l'extension | Réglages › Général › Avancé › « Extensions permises de l'éditeur », grisé ; une extension de l'éditeur hors de la liste refusée avant tout téléchargement, une extension installée qu'elle exclut plus chargée (voir *Extensions*) |
| `projects.forcePrivate` | restreindre (allumé seulement) | « Nouveaux projets privés » et « Projet privé » de chaque projet |
| `projects.deniedFiles` | restreindre, liste d'interdits | lu et gardé ; appliqué avec la clôture des fichiers (à venir) |
| `proxy.url`, `proxy.exclusions`, `proxy.authorities` | sensible | Réglages › Avancé › Réseau : le proxy, ses exclusions, les autorités supplémentaires |
| `proxy.denyInsecureTls` | restreindre (allumé seulement) | « Ignorer la vérification des certificats TLS », éteint |
| `telemetry.enabled` | restreindre (éteint seulement) | Réglages › Général › « Statistiques d'usage » |
| `voice.onlineAllowed` | restreindre (éteint seulement) | la dictée et la voix de la conversation ramenées au local, « En ligne » refusé |
| `voice.dictation` | restreindre (éteint seulement) | la dictée, refusée ; Réglages › Voix › Transcription grisé |
| `remoteControl.enabled` | restreindre (éteint seulement) | le Remote Control pour tous les agents, d'un projet, d'un agent |
| `updates.channel` | restreindre (`stable` seulement) | Réglages › Général › « Recevoir les versions bêta » |
| `updates.automatic` | restreindre (éteint seulement) | Réglages › Général › « Installer les mises à jour automatiquement » |
| `servers.allowed` | restreindre, une borne (adresses) | un serveur hors de la liste, refusé |
| `servers.personalSpace`, `servers.aggregateView`, `servers.contentlessNotifications` | restreindre | lus et gardés ; appliqués avec les espaces et le serveur d'équipe (à venir) |
| `servers.remoteApi` | restreindre (éteint seulement) | l'API distante du cœur (« Escouade à distance › L'API distante ») : refusée à son démarrage et à chaque nouvelle connexion |
| `mcp.externalClients` | restreindre, un plafond (`denied`, `readOnly`, `allowed`) | Claude hors d'Escouade : refusé à la connexion (`denied`), ou ses outils qui écrivent refusés (`readOnly` : seuls `get_*`, `list_*`, `read_*`, `search_*`, `wait_*` passent) |
| `team.server`, `team.fingerprint`, `team.onlyThisServer` | valeur ; restreindre | l'enrôlement (ci-dessous) |

- Une clé inconnue, ou une valeur mal formée, est ignorée. Ce que la lecture a dit va au journal et dans **Réglages › Général › « Stratégies de ce poste »** (le groupe n'apparaît que s'il y a quelque chose à dire).
- L'équipe impose aussi, sur ses projets, les clés du `.escouade.yml` d'un projet qui ne s'exécutent pas (`board.on_approve`, `git.agent_branch_prefix`…) : appliquées par-dessus le fichier du dépôt, refusées quand on les change dans la fenêtre (« Ce réglage est imposé par l'équipe Acme (git.agent_branch_prefix) »).

## L'enrôlement

- Le poste géré peut poser l'adresse du serveur d'équipe de l'entreprise (`team.server`) et l'empreinte de son équipe (`team.fingerprint`).
- Avec `team.onlyThisServer`, tout autre serveur d'équipe est refusé, avec la raison (« Le serveur … est refusé : la stratégie de ce poste ne permet que … ») ; sans serveur posé, tous le sont. Avec `servers.allowed`, tout serveur hors de la liste est refusé. Une équipe du bon serveur qui n'a pas l'empreinte posée est refusée.
- Les adresses se comparent sans la casse du schéma et de l'hôte, ni la barre finale.
- **Aujourd'hui**, le cœur garde ces valeurs (elles partent à la fenêtre avec les verrous) et tient le refus. L'écran d'entrée dans l'équipe, qui les préremplira, vient avec TS-4.

## Dans la fenêtre

- Un réglage imposé est **grisé**, son contrôle inerte, avec un cadenas et la ligne « **Imposé par la stratégie de ce poste** » (ou « Imposé par l'équipe Acme » pour le rang 5) sous ce qu'il fait.
- Une **borne** ne grise pas la ligne mais ses choix interdits : un mode non permis est grisé, sa bulle dit « Auto · non permis par la stratégie de ce poste » ; dans une liste déroulante (le mode du Kanban), le choix porte la même raison. Dans les réglages d'un projet de l'équipe, la raison est son équipe : « Auto · non permis par l'équipe Acme ».
- Chaque ligne du registre des réglages a son `locked_by` : qui impose sa valeur, ou rien. La fenêtre reçoit les verrous de l'app et ceux des projets de chaque équipe.

## C'est le cœur qui refuse

L'interface ne fait que montrer. Une écriture de ce qui est imposé est refusée par le cœur, et dite avec qui l'impose (« Ce réglage est imposé par la stratégie de ce poste (telemetry.enabled) : il ne se change pas sur ce poste. », « Le mode « auto » n'est pas permis par la stratégie de ce poste. ») :

- **par la fenêtre** : l'enregistrement des réglages de l'app, les réglages d'un projet (harness, Remote Control, projet privé), le Kanban (son mode, son effort), le mode, le modèle et l'effort d'un agent, son Remote Control, l'installation d'un plugin, l'installation ou la mise à jour d'une extension de l'éditeur, la dictée ;
- **par le MCP** : `set_board` avec un mode ou un effort interdits, `create_agent` dans un projet dont le harness n'est pas permis, `update_ticket` qui met un compte sur un harness interdit ; un client extérieur selon `mcp.externalClients` ;
- **par l'API distante** : son démarrage et chaque nouvelle connexion, selon `servers.remoteApi` ;
- **par le `.escouade.yml` du dépôt** : `agents.engine`, `board.mode` ou `board.effort` interdits ne sont pas appliqués, et un avertissement le dit dans Réglages › Équipe, le reste du fichier lu comme avant ;
- **par « Exporter vers .escouade.yml »** : `board.mode`, `agents.engine` et `board.effort`, bornés sur ce poste, ne sont pas écrits ; demandés un à un, l'écriture est refusée et dite.
- Un agent ne démarre pas sur un harness, dans un mode, sur un modèle ou à un effort non permis.
- Pour un projet de l'équipe, chacun de ces refus lit les verrous de son équipe (rangs 5 et 6) ; pour un autre projet, ceux du poste.

## Relu à chaud

- Le rang 6 est lu au démarrage, puis **surveillé pendant que l'app tourne** : une stratégie changée s'applique en quelques secondes, sans redémarrer.
  - Windows : `RegNotifyChangeKeyValue` sur `HKLM\SOFTWARE\Policies` et `HKCU\SOFTWARE\Policies` (tout leur sous-arbre : la clé `Escouade` peut ne pas exister encore) ;
  - macOS : les fichiers de `/Library/Managed Preferences` ; Linux : ceux de `/etc/escouade` (noms, tailles et dates, regardés chaque seconde) ;
  - partout, tout est relu toutes les 30 secondes en plus.
- Le rang 5 suit le fichier d'équipe : une version qui entre en vigueur, un accord donné ou retiré refait les verrous de l'équipe et relit le `.escouade.yml` de ses projets.
- Un changement refait les verrous et les applique comme au démarrage : les réglages sont ramenés dans ce qui est imposé et réécrits, la fenêtre reçoit les verrous **et les réglages ramenés** (le formulaire des Réglages ouvert les reprend, sans rien réenregistrer par-dessus) ; une stratégie retirée libère le réglage, qui garde la valeur où elle l'avait laissé.

## Ce qui est déjà en cours

- Au démarrage comme à chaud, les réglages sont ramenés dans ce qui est imposé : la télémétrie éteinte, la bêta coupée, le mode par défaut ramené au **mode permis le plus proche** (celui qui ne permet jamais plus), le mode du Kanban et des comptes de même, l'effort du Kanban ramené au plafond, chaque projet rendu privé quand c'est imposé.
- **Un agent dans un mode devenu interdit** finit son tour, puis passe au mode permis le plus proche, et le dit dans sa conversation : « Le mode « auto » n'est pas permis par la stratégie de ce poste. L'agent passe en mode « acceptEdits », le plus proche permis. » Un agent qui ne travaille pas y passe aussitôt.
- **Un agent d'un harness devenu interdit** s'arrête entre deux tours : son process est fermé à la fin de son tour (aussitôt s'il ne travaille pas), il le dit (« Le harness « claude » n'est pas permis par la stratégie de ce poste. L'agent s'arrête à la fin de son tour. »), et son message suivant est refusé.
- **Une connexion de l'API distante déjà ouverte** quand `servers.remoteApi` passe à `false` continue ; la suivante est refusée.

## Pour les essais

Un build de développement (jamais une version publiée) lit, à la place des stratégies du système, le fichier JSON que nomme `ESCOUADE_TEST_POLICIES` (`{"telemetry.enabled": false}`), et le **surveille** comme le système. C'est la source des e2e `tests/e2e/policies.spec.ts` et `tests/e2e/policies-live.spec.ts` (une stratégie posée puis retirée pendant que l'app tourne), et de `tests/e2e/vscode-extensions.spec.ts` (la liste permise des extensions imposée). Un test vérifie que ce code n'existe que sous `cfg(debug_assertions)` : une variable d'environnement ne doit ni lever ni poser une stratégie dans une version publiée (constat SEC-13 de l'audit de la 2.2.0). Le rang 5 s'essaie dans `tests/e2e/team-settings.spec.ts`, contre un vrai serveur d'équipe et deux clients.

## Le mode jugé est celui où tourne le harness (ESC-497)

Ceci précise « C'est le cœur qui refuse » et « Ce qui est déjà en cours » pour `permissions.allowBypass`, `permissions.modes` et `permissions.defaultMode`, du poste comme d'une équipe. La règle du mode est dans *Comptes*, « Le mode où tourne vraiment le harness ».

- **Les stratégies jugent le mode où le harness fait tourner l'agent**, pas seulement celui qu'on lui a demandé. Un harness qui ne tourne qu'en Bypass est refusé quand `permissions.allowBypass` est éteint ou que `permissions.modes` n'a pas Bypass, quel que soit le mode demandé ; un harness sans « Demander » qui ferait tourner l'agent en « Plan » est refusé quand la borne n'a pas « Plan ». Le refus dit le mode où il tournerait et qui l'interdit : « Unharnessed ne tourne qu’en « Bypass » : Ce réglage est imposé par la stratégie de ce poste (permissions.allowBypass) : il ne se change pas sur ce poste. Choisis un autre harness. », « X n’a pas le mode « Demander » et tournerait en « Plan » : Le mode « plan » n’est pas permis par l’équipe Acme. Choisis un autre mode ou un autre harness. ».
- **Sur chaque chemin** : nouvel agent (fenêtre, outil `create_agent`, y compris l'agent qu'un agent lance, Kanban), ticket (« Lancer », pilote auto, `start_ticket`, Kanban d'équipe, tâche programmée), démarrage et reprise après un redémarrage, reprise programmée après la limite d'usage, vue terminal, changement de mode, passation et « Continuer sur » (le harness d'arrivée aussi), Overseer. Pour un projet d'une équipe, ce sont les verrous de l'équipe (rangs 5 et 6) ; pour un autre projet et pour l'Overseer, ceux du poste.
- **Un nouvel agent dont la stratégie refuse le mode où il tournerait n'est pas créé**, qu'il vienne de la fenêtre, du Kanban ou d'un outil ; la raison est dite. Quand il part du mode par défaut (le sien n'est pas demandé), il va d'abord au mode le plus proche **parmi ceux de son harness** que la stratégie permet, jamais un qui permet plus ; aucun : refusé.
- **Le mode par défaut imposé par une équipe** (`permissions.defaultMode` de son `enforced`) vaut pour les nouveaux agents, les tickets et `start_ticket` de ses projets, ramené dans la borne des modes, puis au mode où le harness le fait tourner. Celui du poste géré ramène le réglage de l'app, comme avant.
- **« Autoriser le mode Bypass » coupé par une équipe** ne vaut que pour ses projets : le réglage de l'app reste allumé pour les autres, mais les agents de ses projets ne sont jamais lancés avec les arguments de Bypass d'un plugin (lancement, relance, vue terminal), comme si le réglage était éteint. Une borne des modes sans Bypass fait de même.
- **Un agent dont le mode devient interdit** passe, à la fin de son tour, au mode permis le plus proche **parmi ceux de son harness** (jamais un mode que son harness n'a pas) ; s'ils permettent tous plus, au moins permissif d'entre eux, jamais Bypass. Un harness dont aucun mode n'est permis s'arrête entre deux tours et le dit : « Le mode « bypassPermissions » n’est pas permis par la stratégie de ce poste. Sans Filet n’a aucun autre mode permis. L’agent s’arrête à la fin de son tour. ».
- **Ce qui tourne encore ne reste pas en vie.** Le process d'un agent, ou son interface (la vue terminal, celle gardée pour Remote Control), qui tourne dans un harness ou un mode que la stratégie ne permet plus, ou en Bypass quand « Autoriser le mode Bypass » vient d'être éteint dans l'app : au repos, il s'arrête aussitôt ; en plein tour, il finit son tour puis s'arrête, sans couper le travail en cours. La conversation dit pourquoi, avec la raison du refus suivie de « L’agent s’arrête à la fin de son tour. » quand la règle change, ou de « L’agent s’arrête. » quand un démarrage est refusé. « Autoriser le mode Bypass » éteint ne change pas le mode de l'agent ; une stratégie le déplace comme ci-dessus quand son harness a un mode permis.
- **Rien ne le relance** : son message, « Relancer », sa vue terminal, la reprise programmée, le démarrage d'avance d'un nouvel agent (préchauffage) et l'interface gardée en fond pour Remote Control passent par le même jugement ; refusé, ce qui tournait encore s'arrête, et la conversation le dit avec la même raison.
