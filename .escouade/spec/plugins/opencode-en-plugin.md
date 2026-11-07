---
title: "OpenCode en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4 (INT-10.2)","docs/PLUGINS.md §8.23 (ESC-349)","docs/TODO-2.2.md, INT-10.1 à 10.5","Ticket ESC-223","Ticket ESC-349","Ticket ESC-364","plugins/opencode/src/rules_verdicts.json"]
tickets: ["ESC-223","ESC-349","ESC-352","ESC-364"]
decisions: [93]
compared: null
---

# OpenCode en plugin

La conversation des agents d'OpenCode passe par le plugin de première main `dev.escouade.opencode` : un module WASM (`run.wasm`, sa crate `escouade-plugin-opencode` dans `plugins/opencode/`), qui a sa version et tourne dans l'hôte de l'app. Pour l'utilisateur, rien ne change : mêmes agents, même serveur par compte, mêmes textes, même vue terminal.

OpenCode est le premier harness sorti du cœur qui n'est pas un flux de lignes : un seul `opencode serve` sert les agents d'un compte, en HTTP et SSE, chacun dans sa session.

## Ce que fait le plugin

- Il décrit le lancement de `opencode serve` : ses arguments, sa config, un mot de passe tiré au hasard, et qu'il est **partagé**. Il ne le lance jamais.
- Il est le client du serveur : il ouvre, reprend ou copie la session d'un agent, donne au dossier de l'agent le serveur MCP d'Escouade avec le jeton de cet agent, pose les règles de son mode, envoie ses messages, répond à ses permissions et ses questions.
- Il garde pour lui un message envoyé pendant un tour jusqu'à la fin de ce tour (OpenCode perdrait la demande du premier).
- Lui seul lit le flux d'événements du serveur : il le traduit en éléments de la conversation et les envoie au cœur à mesure.
- **La clôture d'un projet privé et la lecture seule d'une discussion** sont des règles de la session de l'agent, posées par le plugin : OpenCode n'a pas de bac à sable, seules ces règles protègent. Seul un plugin livré avec l'app est cru sur ce point.

## Comment le module joint le serveur (ESC-349)

- **`local:shared`** : le module ne joint que le port que le cœur a lu dans la sortie du serveur qu'il a lancé pour lui, tant que ce serveur vit, en http sur `127.0.0.1`. Tout autre port local est refusé, et le proxy n'entre pas en jeu.
- **Le flux d'événements** passe par `net/stream`, déclaré `sse` : un silence ne le ferme pas. Coupé ou refusé, il se rouvre 200 ms plus tard, autant de fois qu'il faut, comme en process. OpenCode ne rejoue pas ce qu'il a envoyé : ce qui tombe pendant une coupure est perdu, comme en process.
- **Le flux est lâché avec le dernier agent du serveur**, ou quand le serveur change d'adresse. En process, il restait ouvert jusqu'à l'arrêt du serveur.
- **Une requête** dure 60 s au plus et lit 16 Mio au plus (`limits` du manifeste). Une session plus lourde que ça ne se copie pas : la copie échoue et le dit. En process, il n'y avait pas de limite de taille.
- **8 flux à la fois** par plugin : au-delà de 8 serveurs OpenCode lancés en même temps, l'agent du neuvième dit que le flux ne répond pas, au bout d'une minute.

## Ce qui reste au cœur

- Le lancement et l'arrêt du serveur : le cœur le lance dans son Job object, sous WSL au besoin, dans un dossier à lui (aucun de ceux des agents), une seule fois pour les agents du même compte et avec un agent par dossier sur un serveur, et l'arrête avec son dernier agent.
- Les capacités et le sens des modes, les comptes, les modèles lus sans serveur (`opencode models`), les textes rédigés (`opencode run`), la déclaration de « Les agents peuvent piloter Escouade », le PTY de la vue terminal (`opencode attach`, que `agent/open` décrit, le mot de passe en variable d'environnement seulement).
- La lecture des modèles d'un compte, dans sa propre copie de la règle (`engine/opencode/rules.rs`). La crate du plugin n'est plus liée (ESC-352). Sa vue n'a pas de fichier à lire : le flux d'événements passe déjà par le plugin, et `view/translate` ne la concerne pas.

## Un arrêt se dit

- **Coupé pour l'app ou pas chargé** : un agent qui tournait s'arrête en le disant ; au message suivant, un agent d'OpenCode dit « Le plugin OpenCode (dev.escouade.opencode) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa session. Rallumé, il reprend.
- **Il plante** : le serveur continue, le plugin redémarre, la conversation dit « Le plugin de OpenCode a redémarré. », la session est réécoutée et le tour continue. Un tour fini pendant ce temps est fermé d'après le serveur.
- **Après son dernier plantage permis** : ses agents s'arrêtent en disant pourquoi, leurs sessions gardées pour le message suivant, une fois le plugin relancé.

## Ce qui reste à la carte (ESC-364)

OpenCode n'a pas de bac à sable : ses règles sont sa seule protection, et elles ne lisent que du texte. Sauf en « Sans permissions » :

- **`git add` et `git commit` ne passent d'office en « Accepter les modifications » que sous leur forme simple** : `git add <chemins>`, `git add -A`, `-u`, `--all`, `--update`, `git commit -m <message>`, `git commit -am <message>`, et rien d'autre derrière. Une seconde option (`-F`, `--amend`, `--no-verify`…), une redirection, une option entre guillemets ou échappée, ou ce qu'un shell développerait (`$`, `` ` ``, `{`, `%`, `^`) vont à la carte. Un message qui contient ` -`, `>`, `<`, `$` ou `%` va donc à la carte lui aussi : c'est le prix d'un motif qui ne sait dire que `*` et `?` et ignore la casse.
- **Une écriture dans `.git`, `.opencode`, `opencode.json`, `opencode.jsonc`, `.husky` ou `.githooks`**, à toute profondeur, sous les noms courts et les flux de Windows, va à la carte, en « Demander » aussi, et pour un sous-agent (config du serveur). Un hook tourne au prochain `git commit`, un plugin de `.opencode/plugin` au prochain démarrage de l'instance du dossier, un `opencode.json` donne leurs règles aux sous-agents et peut déclarer des plugins et des serveurs MCP. `AGENTS.md`, `.gitignore`, `.github` restent libres.
- **`websearch`** demande comme `webfetch`.

Un dépôt dont `core.hooksPath` pointe ailleurs que dans `.husky` ou `.githooks` échappe à ces motifs : son dossier de hooks s'écrit sans carte.

## Les règles sont posées avant l'ouverture

Une session neuve les reçoit à sa création. Une reprise et une copie (qui n'en a aucune) les reçoivent par un `PATCH` attendu avant tout le reste : s'il échoue, la session n'ouvre pas, l'agent le dit, et une copie sans règles est supprimée. Un changement de mode attend aussi ce `PATCH` : refusé, le mode reste celui d'avant. Un plan approuvé ne part qu'une fois les règles de son mode en place.

## Le mot de passe du serveur

Il tient à l'écart les autres utilisateurs de la machine et les pages d'un navigateur, pas les agents : OpenCode donne tout son environnement à ses commandes, donc une commande lit `OPENCODE_SERVER_PASSWORD`, et peut changer ses règles, répondre à ses cartes et atteindre les sessions des autres agents du serveur. Seul un plugin en fichier d'OpenCode peut masquer la variable, ce qu'un module ne peut pas écrire.
