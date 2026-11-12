---
title: "Process des agents"
subject: "harness"
sources: ["https://github.com/guillaume-gagnaire/escouade/issues/26","src-tauri/src/agentgroups.rs","src-tauri/src/core.rs","src-tauri/src/engine/claude/mod.rs"]
tickets: ["ESC-417"]
decisions: ["le-remote-control-ne-garde-plus-un-proce"]
compared: null
---

## Ce que c'est

Ce qui borne ce que pèsent les process des agents sur la machine : leur arrêt quand ils sont inactifs, le plafond de ceux que garde le remote control, l'arrêt des process qu'un lancement d'Escouade tué a laissés, et les serveurs MCP que chacun charge. Né de l'issue #26 (macOS, 16 Go) : avec le remote control pour tous les agents, chaque démarrage relançait un process par agent non archivé, même terminé, soit 65 sessions en 4 minutes et 26 Go de mémoire.

## Comportement

### Arrêt des inactifs

- Toutes les minutes, le process d'un agent inactif depuis « Arrêter les processus des agents inactifs après » (30 minutes par défaut, 0 : jamais) s'arrête. Sa session et sa conversation restent : la prochaine action sur l'agent la reprend.
- Un agent relié à claude.ai par le remote control s'arrête de même ; sa session distante est gardée, et son prochain process la rattache (voir *Barre latérale*, « Remote control »).
- Jamais un agent actif (un tour en cours, une question qui attend), ni un agent qui tient un travail de fond : une commande, un moniteur, un sous-agent, un workflow, un job de sa session ou le réveil d'une boucle (voir *Avancée d'un plan*).

### Le plafond du remote control

- Le remote control garde au plus « Process gardés au plus » process d'agents non actifs (Réglages › Agents › Remote control, 8 par défaut). Au-delà, les moins récemment actifs s'arrêtent d'abord, leur remote control et leur session distante gardés. Un démarrage pour le remote control qui le dépasserait n'a pas lieu ; ceux en cours comptent.
- Vérifié après chaque démarrage pour le remote control et avec l'arrêt des inactifs. Un agent actif n'est ni compté ni arrêté. Les interfaces de la vue Claude Code ne sont pas comptées.

### Les process restés d'un lancement tué (macOS, Linux)

- Le process d'un agent mène un groupe de process à lui, que l'app arrête quand elle se ferme ou arrête l'agent. Tuée elle-même (Jetsam sous pression mémoire, plantage, arrêt forcé), elle n'arrête rien : sous macOS et Linux, rien n'y lie un groupe à son parent, et les agents continuaient avec le ppid 1, sans personne pour lire leur sortie.
- Escouade note donc, dans un fichier à lui (`agent-groups.json` du dossier de données), le groupe de chaque process d'agent qu'il lance (le flux d'un agent, le serveur partagé d'OpenCode) : le pid de son meneur, l'heure de début et le programme, tels que `ps` les donne. Un groupe fini est oublié.
- Au lancement suivant, avant qu'aucun agent démarre, un groupe noté dont le meneur vit encore avec la même heure de début et le même programme est arrêté : SIGTERM, puis SIGKILL à ce qui reste 3 secondes après. Une ligne du journal le dit par groupe. Un pid repris depuis par un autre programme, ou par le même programme lancé à une autre heure, n'est jamais touché.
- L'interface d'un agent dans un pseudo-terminal (vue Claude Code) n'est pas notée : le système raccroche sa session quand la fin de l'app ferme le terminal.
- Sous Windows, rien n'est noté : le Job de chaque agent est créé avec KILL_ON_JOB_CLOSE, et le système arrête tout l'arbre avec l'app, de quelque façon qu'elle finisse.

### Les serveurs MCP des agents

- Réglages › Agents › Avancé › « Serveurs MCP des agents » : « Tous les miens » (par défaut, comme avant) ou « Seulement Escouade ».
- Tous les miens : Claude Code ajoute le serveur d'Escouade (`--mcp-config`) à sa configuration ; chaque agent lance donc aussi les serveurs MCP stdio de l'utilisateur, ceux du `.mcp.json` du projet et les connecteurs du compte claude.ai.
- Seulement Escouade : Claude Code reçoit aussi `--strict-mcp-config`, après les arguments que décrit son plugin, et ne charge que le serveur d'Escouade : ni la configuration de l'utilisateur, ni le `.mcp.json` du projet, ni les connecteurs du compte. Si le serveur d'Escouade est refusé à l'agent, il n'a aucun serveur MCP. Cela vaut pour le flux de l'agent et pour son interface en vue Claude Code, à leur prochain lancement.
- Les autres harness n'ont pas d'équivalent : Codex lit les `mcp_servers` de son `config.toml`, OpenCode son `opencode.json`, Copilot CLI son `mcp-config.json`, sans option pour n'en garder qu'un. Ils gardent leur configuration, et la rangée ne se montre qu'avec Claude Code installé.

## Réglages

- « Arrêter les processus des agents inactifs après » : Réglages › Agents › Avancé.
- « Process gardés au plus » : Réglages › Agents › Remote control.
- « Serveurs MCP des agents » : Réglages › Agents › Avancé.

## Limites

- Un groupe dont le meneur a changé de programme après la note (un lanceur qui se remplace par un autre programme) n'est pas reconnu, donc pas arrêté : la prudence passe avant le nettoyage.
- Les process d'un agent sortis de son groupe (un programme qui se détache exprès) ne sont pas suivis.
- « Seulement Escouade » ne vaut pas pour les harness autres que Claude Code.
