---
number: null
id: "hors-du-lancement-un-geste-ne-dit-a-l-ag"
title: "Hors du lancement, un geste ne dit à l'agent que l'origine de la page, sans son titre"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-685"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-685"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":13,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'e2e du cas phare en mode 2 (AB-12) a montré que, hors du lancement et sans accord, `drive_browser` rendait l'adresse complète et le titre de la page à chaque geste « montrer ». Or le palier « montrer » de C4 ne doit rien dire de la page. Sur claude.ai, par exemple, le titre et le chemin nomment une conversation ou un document de l'utilisateur, et `list_browsers` coupait déjà l'adresse à l'origine. L'e2e a aussi montré qu'après `close_browser` sur son lancement, la ligne restait « Pilotée par » : la fenêtre ne relisait pas le pilotage oublié par le cœur.

## Décision

Hors du lancement, sans l'accord de lire et d'agir sur l'origine, la réponse d'un geste ne donne que l'origine de la page (`url`), un titre vide, et « trouvé » ou non. Avec l'accord, sur le lancement et sur une page d'agent, elle donne l'adresse et le titre comme avant. Sur `undriven`, la fenêtre relit le pilotage : la bande disparaît et la ligne redevient ordinaire.

## Conséquences

Un agent qui montre un artefact ou un site tiers ne voit plus le titre ni le chemin de la page sans accord. S'il en a besoin, il le dit à l'utilisateur, qui lui donne l'accord dans la bande. Le changement tient en une fonction pure (`browsers_drive::told`), avec un test Rust et un Vitest. L'e2e `agent-browsers.spec.ts` le vérifie.
