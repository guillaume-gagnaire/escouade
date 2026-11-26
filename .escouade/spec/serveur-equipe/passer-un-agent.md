---
title: "Passer un agent à un collègue"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md §4.2, §11, §13, §15.2 (C16), §16 (TS-17, TS-17b)","docs/TEAM-SESSION-COPY.md (ESC-581, TS-0c : l'essai de reprise d'une session copiée, la recommandation)","Réponse de l'utilisateur du 2026-11-21 (demande a-23) : « C16 : a »","crates/escouade-team/src/handoff.rs","crates/escouade-team-server/src/db/mod.rs","src-tauri/src/teamhandoff.rs","src-tauri/src/sessioncopy.rs","src-tauri/src/handover.rs","Revue de sécurité de la 2.3 (ESC-626), constats SR3-05 et SR3-06 : corrigés par ESC-649"]
tickets: ["ESC-548","ESC-581","ESC-588","ESC-607","ESC-649","ESC-638"]
decisions: [274,340,347]
compared: null
---

# Passer un agent à un collègue

L'agent d'un ticket de l'équipe se passe à un collègue, ou à une machine de l'équipe en mode serveur, avec son ticket, sa branche, son travail en cours et sa conversation (TS-17, §11 de `docs/TEAM-SERVER.md`). Chez qui le prend : **sur Claude Code des deux côtés, sa propre session reprise** (C16-B, TS-17b ; voir « La reprise native de la session ») ; **sinon, ou à tout échec de cette reprise**, une nouvelle session sur son harness et son compte, dont le premier message est le dossier de passation d'ESC-88 (C16-A).

## Le geste

- Le menu de la carte d'un ticket de l'équipe **que ce poste tient, avec son agent**, offre « Passer à un collègue… ». Jamais un agent : le serveur MCP n'a aucun outil pour passer un agent.
- **La fenêtre dit ce qui passe avant le geste** :
  - « À qui » : les collègues du projet (une pastille, leur nom ; « ton autre poste » pour un autre appareil de son membre), puis « Machines de l'équipe » (« En mode serveur : elle le fait tourner jour et nuit. »). Seuls les appareils du projet qui ont ouvert son Kanban y sont ; personne d'autre : « Personne d'autre n'a ouvert ce projet de l'équipe… ».
  - « La branche ticket/dem-41 sera poussée sur origin. Rien n'est poussé avant « Passer ». »
  - « L'agent travaille : son tour sera interrompu avant de passer. », ou « L'agent est arrêté entre deux tours. »
  - La conversation : « chiffrée par la clé du projet, déposée sur le serveur le temps du passage, effacée une fois reprise ou le passage annulé. »
  - Pour un agent de Claude Code dont la session est trouvée : « Sa session de Claude Code part avec lui, chiffrée ; s'il ne peut pas la reprendre, il repart avec la passation. »
  - Le travail non commité (« 2 fichiers non commités », les premiers nommés) : **« Un patch chiffré »** (coché d'office : chiffré par la clé du projet, appliqué tel quel chez qui reprend, rien ne s'ajoute à la branche) ou **« Un commit « en cours » sur la branche »** (poussé avec elle, message `wip: travail en cours de l'agent, passé à Bob [DEM-41]`). Sans travail non commité : « Aucun travail non commité. ».
- **« Passer »**, dans cet ordre : le tour en cours interrompu ; la session de Claude Code lue ; le travail mis en commit « en cours » ou en patch ; la branche poussée ; le dépôt chiffré et l'offre écrits ; la prise passée par le serveur. « Passé à Bob Durand ».
- **Sur ce poste ensuite** : la carte dit « Passé à Bob Durand · en attente qu'il le prenne », avec « Annuler le passage » ; l'agent reste, arrêté, sa conversation le dit (« Passé à Bob Durand : la branche ticket/dem-41 est poussée sur origin… »). Son dossier est détaché de la branche (ses fichiers et son travail y restent), pour qu'elle se reprenne dans un autre worktree, même plus tard sur ce poste.
- **« Annuler le passage »**, tant que le collègue ne l'a pas pris : la prise revient à ce poste sous un nouveau jeton, l'offre et le dépôt sont effacés, l'agent retrouve son ticket et sa branche (« Passage annulé : l'agent et son ticket restent sur ce poste. »). Pris entre-temps : « Bob Durand l'a déjà pris : le passage est fait. ».

## Ce qui passe

- **Le ticket** : le serveur passe la prise à l'appareil choisi, sous un nouveau jeton (le jeton croît, §4.2) : l'ancien jeton de ce poste n'écrit plus rien. Une prise passée a un bail de **14 jours** tant qu'elle n'est pas prise ; la prendre donne encore un nouveau jeton. Le serveur refuse une prise passée à un appareil révoqué, sans accès au projet, ou à soi.
- **La branche** : poussée chez l'hébergeur par le geste de qui passe la main, jamais avant.
- **Le travail non commité** : commit « en cours » ou patch, au choix.
- **La conversation** : le journal d'Escouade de l'agent, dans le **dépôt**, un fichier du serveur chiffré par la clé du projet et rattaché au projet (au-delà de 20 Mo, les plus anciens messages sont laissés de côté). **Son auteur l'efface** : le poste qui passe la main, dès qu'il voit l'offre finie (reprise par le collègue, ou annulée). Le serveur ne laisse que l'appareil qui l'a déposé ou un administrateur l'effacer (SR2-03), jamais qui le reprend. Le serveur voit un fichier d'une taille, jamais ce qu'il contient.
- **La session de Claude Code** (TS-17b) : seulement pour un agent de Claude Code, entre deux tours, quand son fichier est trouvé dans le dossier de son compte (`projects/<worktree encodé>/<id>.jsonl`, sinon dans un autre dossier de `projects`) : ce fichier tel quel, le chemin du worktree où il a tourné et la version de `claude` qui l'a écrit (la plus récente de ses lignes, sinon celle du `claude` du compte). Dans le **même dépôt chiffré**, sous la même borne de 20 Mo : au-delà, la session ne part pas (la passation chez qui reprend). L'offre dit si elle est dans le dépôt. Effacée avec le dépôt, par son auteur.
- **Jamais un compte** : ni identifiant, ni jeton, ni réglage de compte ne passe.

## Chez le collègue

- **La proposition** : « Alice Martin te passe DEM-41 » sur la carte du ticket, avec « Prendre », et dans sa boîte « À toi » (« Agent passé », « Alice Martin te passe DEM-41 · La passation », même geste « Prendre », source `handoff` des vues de la boîte). Elle ne se croit qu'avec la prise : celle que le serveur dit passée à cet appareil par celui qu'elle nomme.
- **« Prendre »** : sur le harness de l'agent quand ce poste l'a pour le projet (écrit, trouvé, allumé, un compte qui sert le projet) ; sinon « Choisis ton harness » (« Claude Code n'est pas sur ce poste pour ce projet : l'agent repart sur le tien, avec la passation. ») liste ceux de ce poste ; aucun : « Aucun harness ne sert ce projet sur ce poste… ».
- Escouade récupère la branche de son hébergeur, prend le ticket (un nouveau jeton), efface l'offre (le dépôt, son auteur l'efface en la voyant finie), puis lance l'agent du ticket comme « Lancer » : son worktree **sur la branche**, son compte à lui, le protocole du ticket. Le patch y est appliqué (s'il ne s'applique pas, la conversation le dit et le patch est gardé dans un fichier nommé).
- **La branche et l'hébergeur, vérifiés (SR3-06)** : la branche de l'offre doit être un nom de branche que git prend (`git check-ref-format --branch` : ni option `-x`, ni `@{-1}`, ni `..`), et **n'écrase jamais une branche locale** de ce poste autre que celle du ticket : « La branche release/2.3 de ce passage existe déjà sur ce poste : elle n'est pas écrasée. Le passage attend. ». Elle se récupère d'**un remote du dépôt** (celui que nomme l'offre s'il existe, sinon le premier), jamais d'une adresse que donne l'offre ; sans remote : « Ce dépôt n'a aucun remote : la branche de ce passage ne se récupère pas. Le passage attend. ». `git fetch` reçoit `--` avant le remote et la branche.
- **La conversation, en passation** : la conversation d'avant, à lire, puis le séparateur « Passé par Alice Martin · passation, suite sur Claude Code », puis le **dossier de passation** replié (« Dossier de passation »), premier message de la nouvelle session : « Passation : suite d'un travail que Alice Martin t'a passé », ce qui a été demandé, fait, les fichiers, l'état git de son worktree, ce qui attendait.
- Chez qui l'a passé, la carte dit ensuite « Pris par Bob Durand ».

## La reprise native de la session (C16-B, Claude Code)

Décidée par l'utilisateur le 2026-11-21, d'après l'essai d'ESC-581 (`docs/TEAM-SESSION-COPY.md`), construite par TS-17b (ESC-607).

- **Seulement Claude Code, des deux côtés** : l'agent passé tournait sur Claude Code, sa session est dans le dépôt, et le collègue le prend avec Claude Code. Codex, Copilot, Cursor, OpenCode et les autres gardent la passation : leur session ne part jamais.
- **Les contrôles, chez qui reprend**, avant que l'agent ne démarre :
  - **la version** : son `claude` (celui du compte choisi) égal ou plus récent que celui de la session ; plus ancien, ou une version inconnue : la passation ;
  - **la réécriture par chaîne JSON** : chaque chaîne (et chaque clé) de chaque objet de chaque ligne, jamais le texte brut (le `\` de Windows y est doublé) ; le worktree de qui passe la main devient le sien, sous la forme de son système (`\` sous Windows), sous l'autre forme (`/`), et tel que l'écrit Git Bash (`/c/…`), sans tenir compte de la casse sous Windows. Un dossier voisin (`dem-41-old`) n'est pas touché, ni les chemins hors du worktree (dossier temporaire de la session, `--add-dir`, dossiers de l'utilisateur). Une ligne sans le worktree reste telle quelle ; une ligne qui n'est pas du JSON fait refuser ;
  - **la vérification** : plus aucune occurrence de l'ancien worktree, sous aucune forme, doublée ou non, dans aucune chaîne, clé ni ligne ; sinon la passation, sans copie déposée ;
  - **le dépôt** : dans `<dossier de configuration du compte choisi>/projects/<worktree encodé>/<id>.jsonl` (chaque caractère qui n'est ni lettre ni chiffre devient `-`), jamais par-dessus une session du même identifiant, ni ailleurs ; un identifiant qui n'est pas celui d'une session (lettres, chiffres, `-`, `_`) fait refuser.
- **Le lancement** : `claude --resume <id> --fork-session` (un nouvel identifiant : jamais deux histoires sous le même). Un process démarré d'avance est arrêté d'abord. La conversation montre la conversation d'avant, le séparateur « **Passé par Alice Martin · reprise de la session, suite sur Claude Code** », puis le premier message replié « Reprise de la session » (« Écrit par Escouade : la session continue ici, sur Claude Code, avec tout son contexte. ») : ta session continue ici, dans tel dossier, sur la même branche, ses chemins réécrits, ce qui vivait hors du dépôt n'est pas là, vérifie les fichiers avant de les modifier ; le patch qui ne s'applique pas y est dit.
- **Le repli** : un contrôle qui refuse le dit dans la conversation (« La session de Claude Code de Alice Martin ne se reprend pas ici (sa version de claude (2.1.330) est plus récente que celle de ce poste (2.1.300)) : la suite part de la passation. ») et la suite est la passation ci-dessus. **Le premier tour qui échoue** (session non trouvée, erreur de l'API, sortie en erreur, refus d'un garde-fou) : son process s'arrête, la copie est retirée du dossier du compte, la conversation le dit (« … ne s'est pas reprise ici (…) : la suite part de la passation. »), puis le séparateur de passation et le dossier de passation, sur une nouvelle session ; ce tour ne bloque pas le ticket. Un premier tour fini, interrompu, arrêté par la limite ou par la connexion garde la session reprise.
- **Reste à essayer par l'utilisateur** : la reprise d'un compte ou d'une organisation à l'autre (demande « À toi » a-24) : ce que font les lignes `session-owner` et `hybrid-organization`, le cache des prompts, les blocs de réflexion signés.

## Vers la machine de l'équipe

- Une machine en **mode serveur** (`escouade-server`, membre de l'équipe) le dit dans chaque projet de l'équipe qu'elle ouvre : la fenêtre du passage la range sous « Machines de l'équipe ».
- Le même geste lui passe l'agent. **Elle ne le prend d'elle-même que si la règle de l'équipe le permet (SR3-05)**, sur son harness (celui de l'agent s'il y est, sinon le premier qui sert le projet), et le fait tourner jour et nuit ; sur Claude Code, sa session reprise comme chez un collègue. La règle est la clé `handoff.machineTakes` du fichier de l'équipe (une valeur, imposée ou recommandée, page « Le .escouade.yml d'équipe ») :
  - `admins` (**d'office**, et pour toute autre valeur) : seuls les agents passés par un **administrateur** de l'équipe ;
  - `members` : ceux de tout membre du projet ;
  - `nobody` : aucun.
- Qui a passé l'agent est l'appareil que le serveur dit avoir passé la prise (celui que nomme l'offre, vérifié par la prise), et son rôle est celui que l'équipe lui donne maintenant.
- **Le reste attend un humain de la machine** : l'agent passé reste proposé sur la carte du ticket et dans la boîte « À toi » de la machine (« Alice Martin te passe DEM-41 · Prendre »), jusqu'à ce que son administrateur le prenne par « Prendre » (depuis un poste connecté à la machine), comme chez un collègue.

## Les refus

Dits avec leur raison, rien n'est passé ni perdu :

- un **agent d'un projet personnel** : « Cet agent est d'un projet personnel : seul l'agent d'un ticket d'un projet de l'équipe se passe à un collègue. » ; un ticket « Ne pas partager », aussi ;
- **quelqu'un hors du projet** : « Cet appareil n'est pas du projet demo… », ou, refusé par le serveur, « Bob n'a pas accès à ce projet sur le serveur de l'équipe : rien n'est passé. » ;
- **une branche dont l'hébergeur ne répond pas** : « L'hébergeur de la branche ticket/dem-41 ne répond pas : rien n'est passé, l'agent et son travail restent ici. (…) » — le commit « en cours » est défait ; chez qui prend, « La branche … ne se récupère pas de son hébergeur : le passage attend, rien n'est perdu. » ;
- chez qui prend, **une branche qui écraserait une branche locale**, un nom qui n'est pas une branche, un dépôt sans remote : le passage attend (ci-dessus) ;
- sur la machine de l'équipe, **un passage que la règle ne lui laisse pas prendre seule** : « Ce passage attend qu'un humain de cette machine le prenne. » ;
- sans dépôt distant : « La branche … n'a pas d'hébergeur où la pousser » ; un agent hors de son worktree, un projet de plusieurs dépôts, un ticket que ce poste ne tient pas, un passage déjà en cours ; hors ligne.

## Le serveur MCP

- Aucun outil ne passe un agent.
- `get_ticket` et `list_tickets` disent, dans `team`, `handoff` : `from`, `to`, `machine`, `branch`, `toHere`, `fromHere`, `since`.

## Ce que voit le serveur

La prise (son id opaque, l'appareil qui la tient, celui qui l'a passée tant qu'elle n'est pas prise, le jeton, les heures), un fichier chiffré d'une taille, rattaché au projet, une opération chiffrée de plus au journal du projet. Jamais le ticket, la branche, le code, la conversation, la session ni les noms.

## Limites

- Ce que passe un administrateur, la machine le prend et l'exécute : sa branche, son patch, sa conversation et sa session viennent de lui (un `.claude/settings.json` de la branche compris). Un compte d'administrateur volé le peut aussi ; `handoff.machineTakes: nobody` remet chaque passage au geste d'un humain de la machine.
- La session copiée garde mot pour mot ce que ses outils ont lu, le prompt de qui passe la main et les identifiants de son compte et de son organisation (`session-owner`) : elle ne va qu'aux membres du projet, chiffrée de bout en bout, et seulement dans le dossier du compte que choisit qui reprend.
- La réécriture change aussi l'histoire : un tour d'avant dit avoir lu un fichier du nouveau worktree, qui peut différer (le travail resté hors du dépôt). Le premier message le dit à l'agent.
- La session d'origine reste chez qui passe la main, sous son identifiant ; la reprise en fork en donne un autre chez le collègue.
