---
title: "Décisions"
subject: "Docs"
sources: ["website/content/docs/fr/docs-space.md","docs/FACTORY.md","src-tauri/src/docs.rs","src-tauri/src/mcp/docs.rs","src-tauri/src/tickets.rs","src/lib/i18n/fr/docs.ts"]
tickets: ["ESC-259","ESC-263"]
decisions: []
compared: null
---

## Ce que c'est

Le registre des décisions sur le produit : une décision par fichier, numérotée et datée, qui dit son contexte, ce qui est décidé, ce qui en découle et ce qu'elle a changé dans la spec. Les agents et l'équipe en proposent ; seul un humain les accepte ; une décision acceptée ne se réécrit plus, elle se remplace.

## Comportement

### Le fichier d'une décision

- `.escouade/decisions/<NNNN>-<titre>.md` (par exemple `0014-tarif-famille.md`), sur la branche des docs, partagé avec les autres docs (voir *Docs du projet*).
- Son en-tête : son numéro, sa date, son état (`proposed`, `accepted`, `superseded`), son origine (`brainstorm`, `discussion`, `ticket` ou `document`) et sa référence, ses liens (`pages`, `tickets`, `documents`), ce qu'elle remplace et ce qui l'a remplacée (`supersedes`, `superseded_by`), ce qu'elle a changé dans la spec (par page, lignes ajoutées et retirées), et quand et par qui elle a été acceptée.
- Puis trois parties : Contexte, Décision et Conséquences (titres lus en français et en anglais).

### Ses états

- **Proposée** : un agent ou quelqu'un de l'équipe la propose ; elle ne vaut décision qu'une fois acceptée. Une proposition encore ouverte se révise sous son numéro. « Accepter », sur sa ligne ou dans son détail, l'accepte d'un clic, au nom de l'utilisateur de la machine.
- **Acceptée** : elle ne se réécrit plus. « La remplacer » propose une nouvelle décision qui la nomme dans `supersedes` ; l'ancienne tient jusqu'à ce que la nouvelle soit acceptée.
- **Remplacée** : la nouvelle a été acceptée ; l'ancienne reste dans le registre telle quelle, et dit par laquelle.

### L'espace Décisions

- Un registre, pas un document : une ligne par décision, la plus récente en tête, avec son numéro, son titre, son origine (« Brainstorm « … » », « Discussion « … » », « Ticket ESC-12 », « Un document »), son état, ce qu'elle a changé dans la spec et sa date. En tête : combien sont acceptées, proposées et remplacées. Le menu « Toutes » filtre par état (« Proposées », « Acceptées », « Remplacées »).
- Un clic sur une ligne ouvre son détail à droite : « ADR-n · proposée … », « … acceptée … par … » ou « … remplacée par … », le contexte, la décision, les conséquences, « A changé dans la spec » (chaque page avec ses lignes ; un clic ouvre la page), ses liens (tickets, origine, documents), ce qu'elle remplace ou ce qui la remplace, et la « Proposition qui la remplacerait » s'il y en a une.
- « Proposer une décision » en écrit une à la main : titre, contexte, décision, conséquences, et « Remplace » (une décision acceptée, ou « Aucune décision »).

### D'où elles viennent

- **D'une discussion ou d'un brainstorm** : proposées dans « Ce qui ira dans la spec », elles entrent au registre, proposées, par « Verser dans la spec » (voir *Discussions*).
- **De l'agent d'un ticket** : il en laisse une quand son ticket change le comportement du produit (voir *Spec tenue à jour*). Son origine est le ticket, d'office. Son détail dit « Acceptée d'office quand le ticket ESC-12 sera validé. » Elle est **acceptée à la validation du ticket**, après les tests et avant le commit : avec la branche courante, elle part dans le même commit que son code ; avec une branche fixe, elle y est versée et attend « Partager ». Une décision qui ne peut être écrite n'arrête pas la validation (voir *Validation*).
- **À la main**, dans l'espace Décisions.
- Les décisions proposées depuis la dernière planification sont à planifier (voir *Planifier*).

### Pour les agents (MCP)

`list_decisions` (filtrées par état au besoin), `read_decision`, `propose_decision` (une nouvelle proposition, ou la révision d'une proposition ouverte), `replace_decision` (une proposition qui remplace une décision acceptée). Ils écrivent dans la branche des docs, sans rien commiter ; les formats sont dans `docs/PROTOCOL.md`.

## Limites

- Aucun outil MCP n'accepte une décision, et le mot « accepter » est interdit aux noms d'outils : seuls la fenêtre et la validation d'un ticket l'acceptent.
