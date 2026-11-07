---
title: "Antigravity CLI en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4 (INT-10.8)","docs/PLUGINS.md §8.24 (ESC-348)","docs/PROTOCOL.md, Antigravity CLI, « Règles et jugement »","docs/TODO-2.2.md, INT-10.8","Ticket ESC-283","Ticket ESC-348","Ticket ESC-371","Revue de dev.escouade.antigravity-cli@2.2.0 (~/.escouade-review)","plugins/antigravity/README.en.md"]
tickets: ["ESC-283","ESC-348","ESC-352","ESC-369","ESC-371"]
decisions: [68,94,"sur-antigravity-cli-aucune-regle-d-agy-n"]
compared: null
---

# Antigravity CLI en plugin

La conversation des agents d'Antigravity CLI passe par le plugin de première main `dev.escouade.antigravity-cli` (INT-10.8) : un module WASM (`run.wasm`, sa crate `escouade-plugin-agy` dans `plugins/antigravity/`, version `2.2.0`), qui tourne dans l'hôte de l'app, sans process à lui (ESC-348). Il reste au registre des plugins livrés jusqu'à l'amorce, et sa page du store pointe vers ce dossier. Pour l'utilisateur : mêmes agents, mêmes modes, mêmes comptes, même vue terminal.

Antigravity CLI n'est pas un harness ACP : `agy -p` parle en stream-json, une ligne par message.

## Ce que fait le plugin

- Il décrit le lancement (`-p=`, stream-json dans les deux sens, le modèle et l'effort, `--mode plan` en Plan seulement, la conversation, le dossier de l'agent en `USERPROFILE` et `HOME`, sa mise à jour automatique coupée) et les fichiers de ce dossier (ses règles de permission, le seul serveur d'Escouade et son jeton) ; le cœur trouve `agy`, vérifie la description et le lance.
- Il relance `agy` quand il le faut, sur la même conversation : un modèle, un effort ou un mode changé, une permission accordée, une règle accordée retirée en fin de tour, un tour arrêté (`agy` n'a pas de message d'arrêt), un redémarrage du plugin. Chaque relance est vérifiée par le cœur comme le premier lancement.
- L'agent est prêt quand `agy` a dit sa conversation ; une conversation à reprendre disparue en ouvre une neuve, et la conversation le dit.
- Ce qu'`agy` écrit lui arrive et repart en éléments de la conversation : textes, raisonnements, outils sous les noms de Claude Code avec leur diff, sous-agents, tokens, fin de tour. Ce que ses événements taisent est lu dans le transcript de la conversation.
- `-p` ne demande rien : à la fin d'un tour, le plugin juge chaque refus (ci-dessous), montre les cartes de ce qui reste et des questions sautées, fait écrire les règles exactes accordées, et le tour continue par un message.
- Une discussion de l'onglet Docs est tenue en lecture seule par `write_file(*)` et `command(*)` refusés, et aucun de ses refus ne devient une carte.
- Un mode Plan finit sur « Plan à approuver », avec le plan écrit dans la conversation.

## Ses règles et le jugement des refus (ESC-371)

`agy` juge seul ses règles, comme un début du texte, et la façon dont il les lit n'a jamais été essayée. Aucune règle d'`agy` ne garde donc rien à elle seule : hors de « Tout accepter » sans projet privé ni discussion, le plugin ne fait écrire aucune règle `allow` à lui ; `agy` refuse, et le plugin juge le refus à la fin du tour.

- **Refusé sans carte** : une discussion ; un appel qui nomme un chemin d'un projet privé, par son texte ou par son chemin résolu (`..`, chemin relatif joint au dossier de l'agent).
- **Accepté sans carte** : tout en « Tout accepter » (seul cas où un refus revient : avec un projet privé, il n'a ni `command(*)` ni autre règle large) ; une lecture dans le dossier des fichiers joints ; en « Demander » et « Édits auto », une écriture dans le dossier de l'agent hors des dossiers gardés ; en « Édits auto », une commande qui n'est que `git add` et `git commit` du worktree, lue comme pour Codex et Copilot CLI (`accepts_script`, refus au moindre doute) quand l'hôte dit son git protégé ; exactement un appel accordé « Pour cet agent ».
- **Une carte** pour le reste. Les **dossiers gardés** sont `.git` (toute casse, `.git.`, flux `::$…`, le fichier `.git` d'un worktree), `.husky`, `.githooks`, `.agents` et `.gemini`, à tout niveau, et tout nom avec `~` ou `:` (nom court) : une écriture y a toujours sa carte, sans « Pour cet agent ». « Pour cet agent » n'est pas non plus offert à un agent d'un projet privé. La carte dit qu'« Autoriser » vaut pour le tour qui suit, ce qui commence pareil passant aussi pendant ce tour.
- Ce qui est accordé l'est par la règle exacte de l'appel, pour le tour qui suit seulement, puis retirée. Un texte avec `*`, `?`, une parenthèse ou un retour à la ligne ne devient jamais une règle : le refus tient, et la conversation le dit. Un appel refusé à nouveau pendant le tour que sa règle autorisait n'est ni accordé de nouveau ni redemandé.
- **« Pour cet agent »** est gardé par le plugin, en mémoire, tant qu'il tourne : ni règle d'`agy`, ni fichier du dossier de l'agent (que l'agent peut écrire). Un redémarrage du plugin l'oublie.
- **Ce que l'agent a pu écrire dans son dossier** ne lui donne aucun droit : les règles sont réécrites à chaque lancement, fin de tour, arrêt, changement de mode et re-attache ; `toolPermission` et `agentMode` sont retirés de `settings.json` ; `mcp_config.json` ne garde que le serveur d'Escouade.
- Un « Édits auto » ou une écriture refusés par `agy` coûtent une reprise du tour (relance d'`agy`, puis il refait l'appel).

L'essai réel `real_agy_rules` (ignoré) dit cas par cas ce qu'`agy` doit faire de ces règles ; il est à lancer quand `agy` sera remis sur le poste.

## Le transcript

`agy` écrit parfois le transcript juste après l'événement qui dit l'étape finie. Rien n'attend alors dans un tour du module, et aucun autre agent n'est retenu :

- ce qu'`agy` a dit à partir de cette étape est gardé, dans l'ordre ;
- le transcript est relu sur un minuteur de l'hôte, toutes les 40 ms, 600 ms au plus par lot ;
- ce qui était gardé part ensuite vers le cœur, dans le même ordre. Si le transcript n'est toujours pas là, ces lignes passent sans lui ;
- un tour arrêté lit aussitôt ce qui est gardé.

## Ce qu'il lit

Il déclare deux permissions : le dossier de compte de ses agents, en lecture (`agentHome`), et le dépôt de l'agent, en lecture (`agentRepo`), qui ne sert qu'à demander à l'hôte si le git du worktree est protégé (`fs/gitProtected`). Chaque agent d'Antigravity a son propre dossier, jamais le `~/.gemini` de l'utilisateur. Il n'y écrit rien lui-même : il décrit ses fichiers, le cœur les vérifie et les écrit. Le plan est lu au chemin d'écriture que le modèle a donné, borné à ce dossier par l'hôte.

Du transcript, il ne lit que les 4 derniers Mio : une étape plus ancienne n'y est plus cherchée.

## Ce qui reste au cœur

Le moteur `Agy` du registre garde ce qui n'est pas la conversation : ses capacités et le sens de ses modes, les comptes (celui de la machine connecté par `agy` dans un terminal, les autres par une clé Gemini), `agy models`, les textes rédigés (`agy -p`), la vue terminal (son `--conversation`, son transcript, ses hooks), le serveur MCP d'Escouade, le retrait de son jeton du dossier de l'agent à la fin du process, et ce qui garde les outils de l'utilisateur à leur place (sa configuration git, Rust) : un module ne connaît pas son dossier.

**La vue passe par le module** (`view/translate`, ESC-352) : le cœur lit le transcript et passe ses lignes au plugin. Plugin coupé, le terminal marche et la conversation dit qu'elle ne le suit plus. Le cœur garde sa copie des mots du lancement de l'interface (ses règles et son `--mode accept-edits`, où l'interface demande elle-même). Voir *Le contrat des harness en process*.

## Un arrêt se dit

Coupé pour l'app ou pas chargé, un agent d'Antigravity CLI dit au lancement « Le plugin Antigravity CLI (dev.escouade.antigravity-cli) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa conversation ; rallumé, il reprend. Un plugin qui plante pendant un tour est relancé comme tout plugin de harness : ses règles sont réécrites aussitôt, sans ce qui avait été accordé, et `agy` relancé avant le message suivant ; une carte montrée avant ne peut plus faire continuer le tour : sa réponse le finit. Ce que le module gardait en attendant le transcript est perdu avec lui.
