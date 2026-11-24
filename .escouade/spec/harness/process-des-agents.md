---
title: "Process des agents"
subject: "harness"
sources: ["https://github.com/guillaume-gagnaire/escouade/issues/26","src-tauri/src/agentgroups.rs","src-tauri/src/core.rs","src-tauri/src/engine/claude/mod.rs","crates/escouade-model/src/job.rs"]
tickets: ["ESC-417","ESC-431","ESC-544","ESC-656"]
decisions: [147,254,"sous-macos-et-linux-l-arbre-d-un-agent-c"]
compared: null
---

## Ce que c'est

Ce qui borne ce que pèsent les process des agents sur la machine : leur arrêt quand ils sont inactifs, le plafond de ceux que garde le remote control, l'arrêt des process qu'un lancement d'Escouade tué a laissés, et les serveurs MCP que chacun charge. Né de l'issue #26 (macOS, 16 Go) : avec le remote control pour tous les agents, chaque démarrage relançait un process par agent non archivé, même terminé, soit 65 sessions en 4 minutes et 26 Go de mémoire.

## Comportement

### L'arbre d'un agent : ce qu'il pèse, ce que son arrêt emporte

- La mémoire et le CPU d'un agent (sa carte, le total de la barre) comptent tout ce qu'il a lancé, et son arrêt arrête tout cela, sur les trois systèmes.
- Sous Windows, c'est le Job de l'agent, qui tient toute sa descendance.
- Sous macOS et Linux (ESC-656), l'arbre de l'agent est : le groupe de process que mène son process ; les descendants de ses process par leur parent, même partis dans une session ou un groupe à eux (un `spawn` détaché de Node, une commande de l'outil Bash de Claude Code, un serveur MCP, un sous-agent hors process) ; les groupes que ces descendants mènent ; et ce qui en a été vu et vit encore, même quand son parent a fini et qu'il est passé à un autre (comme il reste dans le Job sous Windows). Avant, seul le groupe comptait : ce qui l'avait quitté n'était ni compté ni arrêté.
- Un process est reconnu par son pid **et** son heure de début, lue du système (`/proc` sous Linux, `proc_pidinfo` sous macOS) : un pid repris par un autre process n'est jamais compté ni tué, ni le groupe d'un meneur dont le pid a été repris.
- Un échantillon des ressources (toutes les 2 s tant qu'un agent tourne) lit les process de la machine une seule fois (`ps`) pour tous les agents ; aucun sans agent.
- L'arrêt d'un agent relit l'arbre juste avant, tue chaque process (SIGKILL, comme avant) et chaque groupe qu'un descendant mène, après avoir vérifié qu'il est toujours le même process, puis relit, quelques fois au plus, tant qu'il en trouve un nouveau (un process lancé pendant l'arrêt). Un process qui n'en descend pas n'est jamais touché.
- La mémoire : sous macOS, l'empreinte physique de chaque process (ce que montre la colonne « Mémoire » du Moniteur d'activité : sa mémoire propre, compressée comprise, sans les pages des bibliothèques partagées), sinon sa mémoire résidente ; sous Linux, sa mémoire résidente (`rss`) ; sous Windows, son working set. La somme sous Linux (et sous Windows) compte une fois par process les pages des bibliothèques qu'ils partagent : elle dit plus que ce que l'arbre coûte vraiment à la machine.

### Arrêt des inactifs

- Toutes les minutes, le process d'un agent inactif depuis « Arrêter les processus des agents inactifs après » (30 minutes par défaut, 0 : jamais) s'arrête. Sa session et sa conversation restent : la prochaine action sur l'agent la reprend.
- Un agent relié à claude.ai par un réglage du remote control (« pour tous les agents », ou celui de son projet) s'arrête de même ; sa session distante est gardée, et son prochain process la rattache (voir *Barre latérale*, « Remote control »).
- Jamais un agent dont le remote control a été allumé à la main (son menu, ESC-544) : il reste joignable depuis claude.ai, dans le plafond ci-dessous.
- Jamais un agent actif (un tour en cours, une question qui attend), ni un agent qui tient un travail de fond : une commande, un moniteur, un sous-agent, un workflow, un job de sa session ou le réveil d'une boucle (voir *Avancée d'un plan*).

### Le plafond du remote control

- Le remote control garde au plus « Process gardés au plus » process d'agents non actifs (Réglages › Agents › Remote control, 8 par défaut), qu'il les tienne d'un réglage ou qu'ils aient été allumés à la main. Au-delà, les moins récemment actifs s'arrêtent d'abord, quelle que soit la source, leur remote control et leur session distante gardés. Un démarrage pour le remote control qui le dépasserait n'a pas lieu ; ceux en cours comptent.
- Au lancement de l'app (et à la réouverture d'un projet), démarrent les agents allumés à la main, même terminés ou inactifs depuis longtemps, et ceux d'un réglage actifs ou actifs depuis moins que le délai d'arrêt des inactifs ; les autres attendent leur prochain message. Les places sous le plafond vont aux agents dans l'ordre de leur dernière activité, la plus récente d'abord, toujours : chacun ne demande la sienne qu'une fois que le précédent dans cet ordre a eu sa réponse, quel que soit le temps qu'a pris pour chacun l'application des réglages du remote control. Un agent actif démarre hors plafond, comme ailleurs.
- Vérifié après chaque démarrage pour le remote control et avec l'arrêt des inactifs. Un agent actif n'est ni compté ni arrêté. Les interfaces de la vue Claude Code ne sont pas comptées.
- La mémoire reste donc bornée quel que soit le nombre d'agents allumés à la main : au plus « Process gardés au plus » process non actifs.

### Les process restés d'un lancement tué (macOS, Linux)

- Le process d'un agent mène un groupe de process à lui, que l'app arrête quand elle se ferme ou arrête l'agent (avec le reste de son arbre, ci-dessus). Tuée elle-même (Jetsam sous pression mémoire, plantage, arrêt forcé), elle n'arrête rien : sous macOS et Linux, rien n'y lie un groupe à son parent, et les agents continuaient avec le ppid 1, sans personne pour lire leur sortie.
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
- Pour les tests de bout en bout seulement, deux variables d'environnement raccourcissent l'arrêt des inactifs : `ESCOUADE_IDLE_STOP_MS` (le délai, en ms, à la place du réglage) et `ESCOUADE_IDLE_CHECK_EVERY_MS` (le pas de la vérification, une minute sinon).

## Limites

- Un groupe dont le meneur a changé de programme après la note (un lanceur qui se remplace par un autre programme) n'est pas reconnu, donc pas arrêté : la prudence passe avant le nettoyage.
- Sous macOS et Linux, un process lancé à part dont le parent a fini avant qu'un échantillon ou un arrêt l'ait vu est passé à un autre parent : il n'est plus rattaché à l'agent. Un lancement après une app tuée n'arrête que les groupes notés, pas ce que leurs process avaient lancé à part.
- Le CPU d'un process lancé à part ne compte que tant qu'il tourne (sous Windows, le Job garde aussi celui des process finis).
- « Seulement Escouade » ne vaut pas pour les harness autres que Claude Code.
- Au-delà du plafond, même un agent allumé à la main s'arrête et n'est plus joignable jusqu'à son prochain process : sa carte le dit « en veille » (voir *Barre latérale*).
