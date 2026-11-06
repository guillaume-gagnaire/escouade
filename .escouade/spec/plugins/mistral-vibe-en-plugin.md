---
title: "Mistral Vibe en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.5, §4.6, §4.13 (INT-10.6, INT-10.6 bis), §8.28","docs/TODO-2.2.md, INT-10.6","Ticket ESC-281","Ticket ESC-327","Ticket ESC-350","plugins/mistral/README.md"]
tickets: ["ESC-281","ESC-327","ESC-350"]
decisions: [57]
compared: null
---

# Mistral Vibe en plugin

Mistral Vibe est le deuxième harness sorti du cœur (INT-10.6) : sa conversation passe par le plugin de première main `dev.escouade.mistral` (« Mistral », INT-10.6 bis), le même que sa voix. Depuis ESC-350 c'est un module (`run.wasm`) : une seule instance sert les deux côtés, sa crate `escouade-plugin-mistral` est dans `plugins/mistral/`, et il a sa version et sa page au store. Pour l'utilisateur, les agents, les textes et la vue terminal ne changent pas ; ses comptes partagent le compte Mistral (voir *Le compte Mistral*).

## Ce que fait le plugin

- Il sert le harness que son manifeste déclare (`contributes.harnesses[0]`, id `vibe`, objet `acp` avec l'extension `vibe`) par le client ACP de `crates/escouade-acp`, comme Cursor CLI ; ce qui n'est pas `voice/*` va à ce client, dans son ordre, et `voice/*` à la crate `escouade-voice`.
- Le cœur trouve `vibe-acp`, écrit dans le `VIBE_HOME` de l'agent la confiance de son dossier et, pour un projet privé, le `denylist` de ses outils de fichiers (à côté des règles de « Pour cet agent »), demande au plugin la description du lancement (télémétrie coupée), la vérifie et lance Vibe.
- Ce que Vibe écrit va au plugin et revient en éléments de la conversation, comme le faisait le cœur : session (nouvelle, reprise, ou copiée par `session/fork`), agent de Vibe pour le mode d'Escouade, modèle (laissé au défaut de Vibe s'il n'est pas offert) et réflexion pour l'effort, textes, raisonnements, outils sous les noms de Claude Code (`agents.spawn` en `Task`, outils MCP), permissions nommées par leur seul appel, questions en formulaire, tâches, fin de tour avec ce dont le coût de la session a grandi, limite de débit (« limité », reprise une minute après).
- **Le mode Plan** est joué par Escouade (Vibe n'en a pas) : chaque message dit d'étudier sans rien changer, ses écritures sont refusées, ses derniers mots arrivent en carte « Plan à approuver » ; approuvé, Vibe le met en œuvre dans le mode choisi.
- **Un refus coupe le tour** (`session/cancel`) ; le mot de l'utilisateur part ensuite, jamais le texte d'Escouade d'un refus sans mot. « Pour cet agent » est `allow_always_permanent`, gardé dans le `VIBE_HOME` de l'agent.
- **Un projet privé** : la clôture tient par le `denylist` écrit par le cœur et sur ses permissions (`fenceOnPermissions`), qui nomment un chemin clôturé et sont refusées d'office ; en Bypass, Vibe reste dans son agent `ask`.
- **Un compte par sa clé** : `keyVar` `MISTRAL_API_KEY`, donnée à Vibe par le cœur, jamais au plugin.
- Aucune discussion de l'onglet Docs ne part sur lui : son manifeste ne dit pas de mode qui ne change rien.

## Le compte Mistral

Le plugin déclare un compte (`contributes.accounts`, id `mistral`), dont les points sont sa voix et le harness `vibe`. Une clé unique aurait fait perdre une clé à qui a deux organisations Mistral : les clés restent, et se partagent.

- **Le compte Mistral** est la clé de Réglages › Voix : `plugin:dev.escouade.mistral:account:mistral`, ancien nom `dictation-mistral`.
- **Un compte de Vibe qui a sa clé** la garde : `plugin:dev.escouade.mistral:account:vibe-<id>`, ancien nom `vibe:<id>`. Deux organisations restent deux comptes, chacun facturé à part.
- **Personne ne ressaisit sa clé** : les anciens noms restent tenus à côté et lus d'abord, écrits et effacés avec les nouveaux (la migration réversible de *Les comptes partagés par plugin*).
- **Chaque côté se replie sur l'autre.** Dans Réglages › Comptes, un nouveau compte de Vibe peut laisser sa clé vide quand le compte Mistral en a une (le champ dit « Vide : la clé Mistral de Réglages › Voix (clé …wxyz) ») : il va par elle, et se dit connecté avec sa fin. Sans clé dans Réglages › Voix, la voix va par celle du premier compte de Vibe actif, et sa ligne le dit (« Celle du compte Vibe « Équipe » · clé …wxyz », « Donner une autre clé », sans « Effacer la clé » : elle n'est pas à la voix).
- **Le Principal de Vibe** reste la clé de la machine, gardée par `vibe --setup` : Escouade ne la lit jamais, ne la partage pas.
- Le plugin ne lit aucun secret : la clé part avec chaque appel de la voix, et à Vibe par `MISTRAL_API_KEY`.

## Ce qui reste au cœur

Le moteur `Vibe` du registre garde ce qui n'est pas la conversation : ses capacités et le sens de ses modes, les comptes (la clé de la machine gardée par `vibe --setup`, les comptes par clé), les modèles lus pour le menu d'un compte, les textes rédigés (`vibe -p`, sans outil, dans un prix), la vue terminal (son journal et ses hooks), les fichiers de son `VIBE_HOME`, le serveur MCP d'Escouade.

## Un arrêt se dit

Coupé pour l'app ou pas chargé, le plugin n'est plus branché derrière Vibe, et la voix Mistral s'en va avec lui (la dictée repasse en local) : un agent de Vibe dit au lancement « Le plugin Mistral (dev.escouade.mistral) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa session ; rallumé, il reprend.

## Les harness livrés, une table

Les harness sortis du cœur se branchent par une table (l'id du plugin, ce que le cœur écrit avant le lancement) : Cursor CLI et Mistral Vibe y ont chacun une ligne, et les suivants n'y ajoutent que la leur.

## Différence assumée

En « Accepter les modifications », un `git add` ou `git commit` du worktree que le cœur acceptait sans demander demande désormais, comme pour Cursor CLI depuis INT-10.1 : la règle lit le dépôt, ce qu'un plugin ne fait pas.
