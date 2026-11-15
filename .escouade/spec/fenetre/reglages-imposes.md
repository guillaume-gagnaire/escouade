---
title: "Réglages imposés"
subject: "fenetre"
sources: ["docs/TEAM-SERVER.md §6.1, §6.2, §6.4, §6.5, §16 (TS-11)","crates/escouade-policy/README.md (TS-0b, ESC-454)"]
tickets: ["ESC-455"]
decisions: [169]
compared: null
---

# Réglages imposés

Un service informatique, ou plus tard une équipe, peut **imposer** un réglage : il ne se change plus sur le poste. Ça sert même sans serveur d'équipe, à une entreprise qui gère sa flotte par ses propres outils (stratégies de groupe, Intune, un MDM, `/etc`).

## Les six rangs

Un réglage prend sa valeur du rang le plus fort qui la donne :

| Rang | Source | Peut |
| --- | --- | --- |
| 1 | Les valeurs par défaut d'Escouade | tout |
| 2 | L'équipe, recommandé | proposer une valeur, que le dépôt et le poste changent |
| 3 | Le dépôt (`.escouade.yml`) | comme avant |
| 4 | Le poste (Réglages, « Réglage local ») | passer devant le fichier, comme avant |
| 5 | L'équipe, imposé | verrouiller une valeur ou une borne, dans l'espace de l'équipe |
| 6 | Le poste géré (stratégies du poste) | verrouiller pour toute l'app |

- Les rangs 2 à 4 gardent l'ordre d'avant. Les rangs 5 et 6 passent devant tout.
- **Entre l'équipe et le poste géré**, pour une même clé :
  - une **borne** (les modes permis, les harness permis) se croise : il reste ce que les deux permettent ;
  - une **valeur** : celle du poste géré gagne.
- **Chaque clé a sa sorte** : restreindre, valeur, sensible ou exécutable.
  - Une clé qui *restreint* ne s'impose que dans son sens : « Statistiques d'usage » éteintes, jamais allumées ; le canal stable, jamais la bêta. Une autre valeur est ignorée et dite.
  - Une clé *exécutable* (commande, hook, lancement) n'est **jamais** imposée par l'équipe, même signée ; seul le poste géré le peut.
  - Une clé *sensible* (adresse d'un proxy, autorités) venue de l'équipe attend l'accord du poste (à venir avec le fichier d'équipe).
- **Aujourd'hui** : le rang 6 est lu au démarrage de l'app (le registre `Policies\Escouade` sous Windows, le domaine géré `dev.gagnaire.escouade` sous macOS, `/etc/escouade/policies` sous Linux). Une stratégie changée pendant que l'app tourne s'applique au prochain démarrage (la relecture à chaud viendra). Le rang 5 n'a pas encore de source : aucune équipe n'impose rien.

## Les clés qu'on peut imposer

| Clé | Sorte | Ce qu'elle verrouille |
| --- | --- | --- |
| `permissions.allowBypass` | restreindre (éteint seulement) | Réglages › Agents › « Autoriser le mode Bypass » |
| `permissions.modes` | restreindre, une borne (liste de `plan`, `default`, `acceptEdits`, `auto`, `bypassPermissions`) | les modes des agents, le « Mode par défaut des agents », le mode des agents du Kanban |
| `permissions.defaultMode` | valeur (dans la borne des modes) | Réglages › Agents › « Mode par défaut des agents » |
| `harnesses.allowed` | restreindre, une borne (ids des harness) | le harness d'un projet, d'un agent, d'un ticket |
| `telemetry.enabled` | restreindre (éteint seulement) | Réglages › Général › « Statistiques d'usage » |
| `updates.channel` | restreindre (`stable` seulement) | Réglages › Général › « Recevoir les versions bêta » |
| `remoteControl.enabled` | restreindre (éteint seulement) | le Remote Control pour tous les agents, d'un projet, d'un agent |

- Un booléen se donne `true`/`false`, ou `0`/`1` (un `REG_DWORD` d'ADMX). Une borne est une liste (une liste ADMX, un tableau JSON) ; une chaîne seule vaut une liste d'un élément.
- Une clé inconnue, ou une valeur mal formée, est ignorée. Ce que la lecture a dit va au journal et dans **Réglages › Général › « Stratégies de ce poste »** (le groupe n'apparaît que s'il y a quelque chose à dire).

## Dans la fenêtre

- Un réglage imposé est **grisé**, son contrôle inerte, avec un cadenas et la ligne « **Imposé par la stratégie de ce poste** » (ou « Imposé par l'équipe Acme » pour le rang 5) sous ce qu'il fait.
- Une **borne** ne grise pas la ligne mais ses choix interdits : un mode non permis est grisé, sa bulle dit « Auto · non permis par la stratégie de ce poste » ; dans une liste déroulante (le mode du Kanban), le choix porte la même raison.
- Chaque ligne du registre des réglages a son `locked_by` : qui impose sa valeur, ou rien.

## C'est le cœur qui refuse

L'interface ne fait que montrer. Une écriture de ce qui est imposé est refusée par le cœur, et dite avec qui l'impose (« Ce réglage est imposé par la stratégie de ce poste (telemetry.enabled) : il ne se change pas sur ce poste. », « Le mode « auto » n'est pas permis par la stratégie de ce poste. ») :

- **par la fenêtre** : l'enregistrement des réglages de l'app, les réglages d'un projet (harness, Remote Control), le Kanban (son mode), le mode d'un agent, son Remote Control ;
- **par le MCP** : `set_board` avec un mode interdit, `create_agent` dans un projet dont le harness n'est pas permis, `update_ticket` qui met un compte sur un harness interdit ;
- **par le `.escouade.yml` du dépôt** : `agents.engine` ou `board.mode` interdits ne sont pas appliqués, et un avertissement le dit dans Réglages › Équipe, le reste du fichier lu comme avant ;
- **par « Exporter vers .escouade.yml »** : `board.mode` et `agents.engine`, bornés sur ce poste, ne sont pas écrits (un collègue n'a pas forcément la même stratégie) ; demandés un à un, l'écriture est refusée et dite.
- Un agent ne démarre pas sur un harness ou dans un mode non permis.

## Ce qui est déjà en cours

- Au démarrage, les réglages sont ramenés dans ce qui est imposé et réécrits : la télémétrie éteinte, la bêta coupée, le mode par défaut ramené au **mode permis le plus proche** (celui qui ne permet jamais plus), le mode du Kanban et des comptes de même.
- **Un agent dans un mode devenu interdit** finit son tour, puis passe au mode permis le plus proche, et le dit dans sa conversation : « Le mode « auto » n'est pas permis par la stratégie de ce poste. L'agent passe en mode « acceptEdits », le plus proche permis. » Un agent restauré au démarrage, qui ne travaille pas, y passe aussitôt.

## Pour les essais

Un build de développement (jamais une version publiée) lit, à la place des stratégies du système, le fichier JSON que nomme `ESCOUADE_TEST_POLICIES` (`{"telemetry.enabled": false}`). C'est la source de l'e2e `tests/e2e/policies.spec.ts`. Un test vérifie que ce code n'existe que sous `cfg(debug_assertions)` : une variable d'environnement ne doit ni lever ni poser une stratégie dans une version publiée (constat SEC-13 de l'audit de la 2.2.0).
