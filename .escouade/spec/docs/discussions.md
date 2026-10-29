---
title: "Discussions"
subject: "Docs"
sources: ["website/content/docs/fr/docs-space.md","docs/FACTORY.md","src-tauri/src/discussion.rs","src-tauri/src/mcp/docs.rs","src-tauri/src/engine/mod.rs","src/lib/i18n/fr/docs.ts","src/lib/shortcuts.ts","src/components/docs/SpecSpace.svelte"]
tickets: ["ESC-261","ESC-289"]
decisions: []
compared: null
---

## Ce que c'est

Parler de la spec avec un agent, dans l'onglet Docs : l'interroger, la mettre à jour, ou faire mûrir une idée. L'agent lit la spec, les décisions, les documents et le code. Il n'écrit que des propositions de pages et de décisions, que l'utilisateur verse dans la spec d'un clic.

## Comportement

### Les deux sortes

- **« Discussion »** : l'agent rédacteur de spec, qui connaît la doc, les décisions et le code. Pour interroger la spec ou la mettre à jour simplement (« Que dit la spec sur le paiement en plusieurs fois ? », « Ajoute que le reçu fiscal part par e-mail. »). Il répond court, en citant les pages, décisions, documents et code d'où il tient ce qu'il dit.
- **« Brainstorm »** : un agent produit, à la manière de BMAD (analyste et chef de produit), qui fait mûrir une idée (« On voudrait un tarif famille. ») : usages et parcours, découverte, conformité et RGPD, accessibilité, ce que font d'autres secteurs. Il propose des fonctionnalités, des pages (en brouillon tant que ce n'est pas tranché) et des décisions. Les tickets, il les dit sans les créer : « Planifier » les créera une fois les changements versés.
- Les deux posent leurs questions une à une, avec des choix.

### Commencer une discussion

- « Nouvelle discussion », en tête de l'onglet Docs (Ctrl+B quand l'onglet est ouvert) : la sorte, un « Sujet » et, au besoin, des « Fichiers à joindre ». L'agent a déjà tous les documents du projet ; un fichier joint est rangé et indexé seul dans les Documents (voir *Documents*), et l'agent en reçoit le chemin.
- Le pied de la fenêtre montre le harness, le compte et le modèle (ceux du projet par défaut), et la branche des docs ; il laisse les changer.
- « Demander à la spec », en tête de la spécification, ouvre une discussion avec l'agent rédacteur.
- « Mettre la spec à jour », sur un écart d'une page, ouvre une discussion « Mettre à jour « <page> » » : l'écart et ses fichiers sont dits à l'agent, qui lit le code et propose la page d'après lui. Versée, la page perd cet écart.
- « Écrire une première spec depuis le code », sur une spec vide : l'agent rédacteur lit le code et propose une première spec, rangée par sujet, une page par sujet ou par fonctionnalité, qui dit ce que fait le produit et non comment il est codé (cela coûte des tokens, d'où le geste).
- Si son premier message est refusé (compte à reconnecter, harness introuvable…), la discussion n'est pas créée : la fenêtre dit pourquoi et garde ce qui avait été choisi.

### Un agent qui ne touche pas au code

- C'est un agent d'Escouade comme les autres : la même conversation, et ses questions se répondent aussi depuis la liste des agents. Il travaille dans le dossier de la branche des docs, en mode « Demander » ou « Plan » seulement.
- Il lit le code (par l'index du code s'il est allumé) sans jamais le modifier, et n'écrit que les docs, par les outils d'Escouade. Escouade refuse d'office ses écritures de fichiers et ses commandes, en lui disant pourquoi. Ses outils d'Escouade qui lisent (Kanban, docs, mémoire, index) et ceux qui écrivent les docs passent sans demander ; tout autre (créer ou lancer un ticket, écrire à un agent, partager les docs…) reste à l'accord de l'utilisateur.
- Son harness l'en empêche lui-même, dans tous les modes : Claude Code perd ses outils qui écrivent, Codex lit seulement dans son bac à sable ; OpenCode, Copilot CLI, Antigravity CLI et Gemini CLI voient leurs écritures et leurs commandes refusées.

### « Ce qui ira dans la spec »

- Une discussion ouverte montre la conversation à gauche et, à droite, « Ce qui ira dans la spec » : les pages proposées, page par page (« nouvelle section », phrases retirées barrées, nouvelles en vert, « N lignes »), et les « Décisions proposées » (« remplace la n° N » pour un remplacement). Une croix retire une page ou une décision.
- Ce que l'agent écrit par les outils de docs n'entre pas dans les docs : c'est une proposition, qu'il relit telle qu'il l'a proposée. Plusieurs écritures d'une page font une seule proposition, la dernière ; une décision du même titre est révisée. Une décision proposée a pour origine « discussion » ou « brainstorm », avec le sujet.
- **« Verser dans la spec »** écrit chaque page proposée, entière, sur la branche des docs, et chaque décision au registre, proposée (voir *Décisions*), puis les retire de la discussion. Rien n'est commité : « Versé dans la spec : N pages. Pas encore partagé. », et la barre d'état compte les changements à partager (voir *Docs du projet*).

### La liste des discussions

En cartes sous « Nouvelle discussion », chacune marquée « Discussion » ou « Brainstorm », avec son état : « Question », « Répond », « Brouillon » (des propositions pas encore versées), « Versé » ou « En pause ». Une discussion versée propose « Planifier » (voir *Planifier*).

## Limites

- Pas de discussion sur Cursor CLI ni sur Mistral Vibe : ils modifient les fichiers de leur dossier sans passer par Escouade. La fenêtre le dit et part sur un autre harness.
- Une proposition n'est pas dans la spec avant d'être versée : `get_spec_tree` et `search_spec` ne la voient pas, seul `read_spec_page` de la discussion qui l'a faite la lit.
