---
title: "Antigravity CLI en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4 (INT-10.8)","docs/PLUGINS.md §8.24 (ESC-348)","docs/PROTOCOL.md, Antigravity CLI","docs/TODO-2.2.md, INT-10.8","Ticket ESC-283","Ticket ESC-348","plugins/antigravity/README.md"]
tickets: ["ESC-283","ESC-348"]
decisions: [68,94]
compared: null
---

# Antigravity CLI en plugin

La conversation des agents d'Antigravity CLI passe par le plugin de première main `dev.escouade.antigravity-cli` (INT-10.8) : un module WASM (`run.wasm`, sa crate `escouade-plugin-agy` dans `plugins/antigravity/`, version `2.2.0`), qui tourne dans l'hôte de l'app, sans process à lui (ESC-348). Il reste au registre des plugins livrés jusqu'à l'amorce, et sa page du store pointe vers ce dossier. Pour l'utilisateur, rien ne change : mêmes agents, mêmes modes, mêmes comptes, même vue terminal.

Antigravity CLI n'est pas un harness ACP : `agy -p` parle en stream-json, une ligne par message.

## Ce que fait le plugin

- Il décrit le lancement (`-p=`, stream-json dans les deux sens, le modèle et l'effort, le mode, la conversation, le dossier de l'agent en `USERPROFILE` et `HOME`, sa mise à jour automatique coupée) et les fichiers de ce dossier (ses règles de permission, le serveur d'Escouade et son jeton) ; le cœur trouve `agy`, vérifie la description et le lance.
- Il relance `agy` quand il le faut, sur la même conversation : un modèle, un effort ou un mode changé, une permission accordée, un tour arrêté (`agy` n'a pas de message d'arrêt : le programme est arrêté, et le message suivant en démarre un autre). Chaque relance est vérifiée par le cœur comme le premier lancement.
- L'agent est prêt quand `agy` a dit sa conversation ; une conversation à reprendre disparue en ouvre une neuve, et la conversation le dit.
- Ce qu'`agy` écrit lui arrive et repart en éléments de la conversation : textes, raisonnements, outils sous les noms de Claude Code avec leur diff, sous-agents, tokens, fin de tour. Ce que ses événements taisent est lu dans le transcript de la conversation.
- `-p` ne demande rien : à la fin d'un tour, le plugin montre les cartes de ce qui a été refusé et des questions sautées, écrit à chaud les règles accordées, et le tour continue par un message.
- Une discussion de l'onglet Docs est tenue en lecture seule par `write_file(*)` et `command(*)` refusés.
- Un mode Plan finit sur « Plan à approuver », avec le plan écrit dans la conversation.

## Le transcript

`agy` écrit parfois le transcript juste après l'événement qui dit l'étape finie. Rien n'attend alors dans un tour du module, et aucun autre agent n'est retenu :

- ce qu'`agy` a dit à partir de cette étape est gardé, dans l'ordre ;
- le transcript est relu sur un minuteur de l'hôte, toutes les 40 ms, 600 ms au plus par lot ;
- ce qui était gardé part ensuite vers le cœur, dans le même ordre. Si le transcript n'est toujours pas là, ces lignes passent sans lui ;
- un tour arrêté lit aussitôt ce qui est gardé.

## Ce qu'il lit

Il déclare une permission : le dossier de compte de ses agents, en lecture (`agentHome`), que la fenêtre d'installation dit (« Lit le dossier de compte de tes agents ») et que l'hôte applique. Chaque agent d'Antigravity a son propre dossier, jamais le `~/.gemini` de l'utilisateur. Il n'y écrit rien lui-même : il décrit ses fichiers, le cœur les vérifie et les écrit.

Du transcript, il ne lit que les 4 derniers Mio : une étape plus ancienne n'y est plus cherchée.

## Ce qui reste au cœur

Le moteur `Agy` du registre garde ce qui n'est pas la conversation : ses capacités et le sens de ses modes, les comptes (celui de la machine connecté par `agy` dans un terminal, les autres par une clé Gemini), `agy models`, les textes rédigés (`agy -p`), la vue terminal (son `--conversation`, son transcript, ses hooks), le serveur MCP d'Escouade, le retrait de son jeton du dossier de l'agent à la fin du process, et ce qui garde les outils de l'utilisateur à leur place (sa configuration git, Rust) : un module ne connaît pas son dossier.

**La vue lit les mots du plugin, pris comme une bibliothèque**, pour les mêmes règles et les mêmes appels des deux côtés, jusqu'à ce que les « Vue X » sortent du cœur (P8).

## Un arrêt se dit

Coupé pour l'app ou pas chargé, un agent d'Antigravity CLI dit au lancement « Le plugin Antigravity CLI (dev.escouade.antigravity-cli) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa conversation ; rallumé, il reprend. Un plugin qui plante pendant un tour est relancé comme tout plugin de harness ; une carte montrée avant ne peut plus faire continuer le tour : sa réponse le finit. Ce que le module gardait en attendant le transcript est perdu avec lui.
