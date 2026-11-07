---
title: "Décisions"
subject: "Docs"
sources: ["website/content/docs/fr/docs-space.md","docs/FACTORY.md","src-tauri/src/docs.rs","src-tauri/src/docsbranch.rs","src-tauri/src/docsticket.rs","src-tauri/src/mcp/docs.rs","src-tauri/src/tickets.rs","src/lib/decisions.ts","src/lib/i18n/fr/docs.ts"]
tickets: ["ESC-259","ESC-263","ESC-303","ESC-377"]
decisions: [115]
compared: "2026-10-30T05:29:15+01:00"
---

## Ce que c'est

Le registre des décisions sur le produit : une décision par fichier, datée, qui dit son contexte, ce qui est décidé, ce qui en découle et ce qu'elle a changé dans la spec. Les agents et l'équipe en proposent ; seul un humain les accepte ; une décision acceptée ne se réécrit plus, elle se remplace. Une décision reçoit son numéro quand elle est acceptée, pas avant.

## Comportement

### Le fichier d'une décision

- Une proposition : `.escouade/decisions/<id>.md` (par exemple `tarif-famille.md`), où l'id est le slug de son titre (40 caractères au plus), rendu unique dans le registre (`tarif-famille-2` pour un second titre pareil) ; un id n'est jamais fait de chiffres seuls. Elle n'a pas de numéro (`number: null`).
- Une décision acceptée : `.escouade/decisions/<NNNN>-<id>.md` (par exemple `0014-tarif-famille.md`), sur la branche des docs, partagée avec les autres docs (voir *Docs du projet*). Elle garde son id.
- Son en-tête : son numéro (une fois acceptée), son id, sa date, son état (`proposed`, `accepted`, `superseded`), son origine (`brainstorm`, `discussion`, `ticket` ou `document`) et sa référence, ses liens (`pages`, `tickets`, `documents`), ce qu'elle remplace et ce qui l'a remplacée (`supersedes`, `superseded_by`), ce qu'elle a changé dans la spec (par page, lignes ajoutées et retirées), et quand et par qui elle a été acceptée.
- Puis trois parties : Contexte, Décision et Conséquences (titres lus en français et en anglais).
- Les décisions écrites avant (numérotées dès leur proposition, sans id dans l'en-tête) se lisent telles quelles : leur id vient du nom de leur fichier, sans le numéro. Elles ne sont pas réécrites.

### Comment on la cite

- Une proposition par son id, une décision acceptée par son numéro : dans les pages de spec (`decisions: [3, "tarif-famille"]`), les tickets que « Planifier » fait, ce qui est à planifier, les outils des agents. Son id la retrouve encore une fois acceptée.

### Ses états

- **Proposée** : un agent ou quelqu'un de l'équipe la propose ; elle ne vaut décision qu'une fois acceptée. Une proposition encore ouverte se révise sous son id. « Accepter », sur sa ligne ou dans son détail, l'accepte d'un clic, au nom de l'utilisateur de la machine.
- **Acceptée** : à l'acceptation, elle reçoit le numéro qui suit le plus haut du registre de la branche où elle va ; son fichier prend ce numéro, et les pages de spec et les tickets qui la citaient par son id citent son numéro ; la validation d'un ticket ajoute ce ticket à ses `tickets`. Elle ne se réécrit plus. « La remplacer » propose une nouvelle décision qui la nomme dans `supersedes` ; l'ancienne tient jusqu'à ce que la nouvelle soit acceptée.
- **Remplacée** : la nouvelle a été acceptée ; l'ancienne reste dans le registre telle quelle, et dit par laquelle (`superseded_by`, le numéro que la nouvelle a reçu).
- Une proposition écrite avant, qui a déjà un numéro, le garde à l'acceptation tant qu'aucune autre décision ne l'a, ici ni sur la branche où elle va ; sinon elle prend le suivant, et les pages qu'elle nomme suivent.

### L'espace Décisions

- Un registre, pas un document : une ligne par décision, les propositions en tête (la plus récente d'abord), puis les décisions numérotées, la plus haute d'abord ; avec son numéro (« — » pour une proposition, qui n'en a pas encore), son titre, son origine (« Brainstorm « … » », « Discussion « … » », « Ticket ESC-12 », « Un document »), son état, ce qu'elle a changé dans la spec et sa date. En tête : combien sont acceptées, proposées et remplacées. Le menu « Toutes » filtre par état (« Proposées », « Acceptées », « Remplacées »).
- Deux fichiers du même numéro (laissés par deux validations simultanées avant ESC-377, ou par une fusion à la main) sont deux lignes du registre, chacune avec son détail : l'app n'en renumérote aucune d'elle-même, à l'équipe de le faire.
- Un clic sur une ligne ouvre son détail à droite : « <id> · proposée …, numérotée à son acceptation », « ADR-n · acceptée … par … » ou « … remplacée par … », le contexte, la décision, les conséquences, « A changé dans la spec » (chaque page avec ses lignes ; un clic ouvre la page), ses liens (tickets, origine, documents), ce qu'elle remplace ou ce qui la remplace, et la « Proposition qui la remplacerait » s'il y en a une.
- « Proposer une décision » en écrit une à la main : titre, contexte, décision, conséquences, et « Remplace » (une décision acceptée, ou « Aucune décision »).

### D'où elles viennent

- **D'une discussion ou d'un brainstorm** : proposées dans « Ce qui ira dans la spec », elles entrent au registre, proposées, par « Verser dans la spec » (voir *Discussions*).
- **De l'agent d'un ticket** : il en laisse une quand son ticket change le comportement du produit (voir *Spec tenue à jour*). Son origine est le ticket, d'office. Son détail dit « Acceptée d'office quand le ticket ESC-12 sera validé. » Elle est **acceptée à la validation du ticket**, après les tests et avant le commit, et numérotée face au registre de la branche où le ticket est mergé (main, tel que git l'a au moment de la validation), à celui du dossier du projet et aux numéros que les autres validations en cours du projet ont déjà donnés sans être encore mergées : deux tickets menés en même temps, dans deux worktrees, puis validés l'un après l'autre ou au même moment, reçoivent deux numéros qui se suivent. Les validations d'un projet numérotent l'une après l'autre ; un numéro donné reste retenu jusqu'à la fin de sa validation (mergé, il est alors sur la branche ; arrêtée, il redevient libre). « Accepter » dans la fenêtre tient compte de ces numéros retenus aussi. La validation retrouve ses propositions par leur id (origine `ticket`, sa clé), jamais par un numéro qu'une autre décision pourrait avoir. Avec la branche courante, elle part dans le même commit que son code ; avec une branche fixe, elle est numérotée face au registre de cette branche, y est versée (le fichier sans numéro de la proposition en est retiré) et attend « Partager ». Une décision qui ne peut être écrite n'arrête pas la validation (voir *Validation*).
- **À la main**, dans l'espace Décisions.
- Les décisions proposées depuis la dernière planification sont à planifier (voir *Planifier*).

### Pour les agents (MCP)

`list_decisions` (filtrées par état au besoin ; chaque décision avec son `number`, `null` pour une proposition, et son `id`), `read_decision` (par `number` ou par `id`), `propose_decision` (une nouvelle proposition, sans numéro, ou la révision d'une proposition ouverte par son `id` ; la réponse rappelle de la citer par son id), `replace_decision` (une proposition qui remplace une décision acceptée, nommée par son numéro). Ils écrivent dans la branche des docs, sans rien commiter ; les formats sont dans `docs/PROTOCOL.md`. Les événements des plugins `decision.proposed` portent son `id`, `decision.accepted` son `number` et son `id`.

## Limites

- Aucun outil MCP n'accepte une décision, et le mot « accepter » est interdit aux noms d'outils : seuls la fenêtre et la validation d'un ticket l'acceptent.
- Deux instances d'Escouade (deux machines) qui valident en même temps ne se voient pas : chacune numérote face au registre de la branche tel que git l'a chez elle.
- Une validation par pull request (sans merge) libère son numéro à sa fin, avant que la PR soit fusionnée : une validation suivante peut le reprendre.
