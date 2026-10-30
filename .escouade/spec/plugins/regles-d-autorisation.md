---
title: "Règles d'autorisation des plugins"
subject: "plugins"
sources: ["docs/PLUGINS.md §1.7","docs/TODO-2.1.md (PLG-8)","schemas/plugin-protocol-1.json (permissions/decide)","website/content/docs/fr/plugins/permission-rules.md"]
tickets: ["ESC-180"]
decisions: []
compared: "2026-10-30T01:50:33+01:00"
---

# Règles d'autorisation des plugins

Un plugin qui a la permission `permissions.decide` répond aux demandes d'autorisation des agents avant qu'elles n'attendent l'utilisateur.

## Quand

- Toute demande d'autorisation **nouvelle** d'un agent (`add_pending`), quel que soit son harness (Claude Code, Codex, Copilot CLI, OpenCode, Cursor CLI, Mistral Vibe, Antigravity CLI, Gemini CLI). Une demande redemandée garde sa place et n'est pas reposée.
- Jamais : un plan à approuver (`ExitPlanMode`), une question de l'agent.

## Qui

Les plugins chargés, avec un programme, dont `decide` n'a pas été retiré, et allumés pour le projet de l'agent (un projet privé : seulement un plugin allumé pour lui). La permission est montrée en rouge à l'installation ; aucun `.escouade.yml` n'allume un plugin ni ne donne une permission.

## Comment

- `permissions/decide` à tous en même temps : la demande (`id`, `kind` parmi `command`, `edit`, `read`, `fetch`, `mcp`, `tool` ; `tool` au nom de Claude Code ; `input` aux textes longs coupés), le projet (avec `private`), le ticket s'il y en a un, l'agent. Jamais la conversation.
- Délai : 500 ms par plugin ; un plugin dont le process ne tourne pas encore a 3 s, une fois. Escouade démarre les plugins qui décident à son démarrage.
- Réponse `allow`, `deny` (avec `message`) ou `ask`. Une erreur, un silence, un plantage ou une réponse autre valent `ask`.
- **Un refus l'emporte**, puis une autorisation (le premier plugin dans l'ordre de la liste est nommé) ; sinon la demande va à l'utilisateur comme sans plugin.

## Ce qui se voit

- La carte de la demande est dans la conversation dès qu'elle arrive ; la notification de l'utilisateur est retenue pendant la décision et n'est envoyée que si aucun plugin n'a tranché.
- Une demande tranchée : la carte dit « Autorisé par <plugin> » / « Refusé par <plugin> » suivi de « via Escouade », et le message d'un refus. L'agent reçoit « Refusé par <plugin> via Escouade : <message> ».
- L'utilisateur peut répondre sur la carte pendant la décision ; la première réponse est gardée.
- Chaque décision de chaque plugin au journal du plugin (type « Décision ») : l'outil et sa cible, la réponse, son délai, ou pourquoi il n'a pas décidé.
