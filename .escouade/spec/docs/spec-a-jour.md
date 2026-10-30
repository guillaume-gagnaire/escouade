---
title: "Spec tenue à jour"
subject: "Docs"
sources: ["website/content/docs/fr/docs-space.md","docs/FACTORY.md","src-tauri/src/board.rs","src-tauri/src/docsticket.rs","src-tauri/src/docscompare.rs","src-tauri/src/docsimport.rs","src-tauri/src/docs.rs","src-tauri/src/tickets.rs","src/lib/i18n/fr/docs.ts"]
tickets: ["ESC-263","ESC-278","ESC-301"]
decisions: []
compared: "2026-10-30T03:21:04+01:00"
---

## Ce que c'est

La spec suit le code de deux façons : les agents des tickets la mettent à jour avec leur code, et un agent court la compare au code après chaque ticket fusionné, ou à la demande. Tout agent qu'Escouade lance sur un projet qui a une spec sait qu'elle est la seule à suivre.

## Comportement

### Les agents des tickets la tiennent

- La consigne de chaque ticket demande à son agent, si le ticket change le comportement du produit, de mettre à jour les pages qu'il touche (`search_spec`, `read_spec_page` puis `write_spec_page`) et de laisser une décision qui dit ce qui a changé et pourquoi (`propose_decision`), avant de rendre le ticket. Rien ne bloque la validation : c'est à la revue de le voir.
- Une page qu'écrit l'agent d'un ticket reçoit la clé du ticket dans ses `tickets`. Sa décision a pour origine le ticket et est acceptée à sa validation (voir *Décisions*).
- **Branche courante** : ces changements sont dans la branche du ticket et se fusionnent avec son code.
- **Branche fixe** : l'agent lit et écrit une copie de la spec et des décisions de la branche, tenue pour son ticket dans le dossier de données d'Escouade ; les documents restent ceux de la branche. La copie n'est versée sur la branche des docs **qu'à la validation du ticket**, jamais avant : la spec ne décrit jamais un comportement pas encore fusionné. Seul ce que le ticket a changé est versé : ce que l'équipe a écrit entre-temps sur la branche reste. Un ticket supprimé emporte sa copie.

### La comparaison au code

- Un agent court lit le code sans rien modifier et relève, page par page, où la page dit autre chose que le code : chaque écart dit ce qui diffère et les fichiers en cause. Les écarts et la date de la comparaison sont gardés dans le dossier de données d'Escouade, par projet et par page (`docs-checks.json`), **pas dans la page** : une comparaison ne laisse aucune modification à git, et la validation suivante n'est pas refusée pour « des modifications non commitées » qu'Escouade aurait faites elle-même. Escouade ne commite toujours rien à la place de l'utilisateur.
- Ce qui est gardé se lit par-dessus l'en-tête de la page, quelle que soit la branche : la carte des sujets, l'arbre et le haut de la page les montrent (voir *Spécification*), et les agents les lisent par `read_spec_page` et `get_spec_tree`. Une page dont l'en-tête a encore des `gaps` ou un `compared` d'avant se lit telle quelle jusqu'à sa prochaine comparaison. Écrire la page (un agent, « Modifier », une discussion versée) garde son en-tête comme il était ; un écart réglé par une discussion versée sort de ce qui est gardé.
- Ce qui est gardé reste sur ce poste : un coéquipier voit les écarts de ses propres comparaisons.
- **Après chaque ticket validé par un merge**, s'il a modifié quelque chose, en arrière-plan : les pages qu'il touche, 12 au plus, celles qu'il a écrites d'abord, puis celles qui citent sa clé et celles qui parlent de ses fichiers (par leurs symboles quand l'index du code est allumé, sinon par leurs chemins et leurs noms).
- **À la demande**, « Comparer au code » : en tête de la spécification pour toute la spec (40 pages au plus, les moins récemment comparées d'abord), ou en tête d'une page pour elle seule. Pendant : « Comparaison… (N pages) » ; ensuite : « N pages comparées », « aucun écart », « N pages pas comparées ».
- Une comparaison à la fois, 5 minutes au plus par page, sur le code du dossier du projet (de son premier dépôt). Chaque page comparée coûte des tokens, sur le harness et le compte de « Modèle des textes rédigés » (voir *Langues*).
- Un écart se règle de deux façons : « Mettre la spec à jour » (voir *Discussions*) quand le code a raison, « Ouvrir un ticket pour le code » (voir *Spécification*) quand la spec a raison.
- Une modification de l'utilisateur dans le dossier du projet, d'une page de spec comme du code, bloque toujours la validation par merge dans ce dossier (« Le dossier du projet a des modifications non commitées sur main »).

### Les agents suivent la spec d'Escouade

Dès que la spec d'un projet a une page dans Escouade, tout agent qu'Escouade lance sur ce projet, quel que soit son harness (ticket, discussion, planificateur…), est prévenu que la spec et les décisions sont dans Escouade et sont les seules à suivre. Les fichiers d'une spec importée lui sont nommés, avec leur date d'import : ils ne se lisent plus comme la spec et ne se mettent plus à jour. Escouade retient ces chemins dans `.escouade/spec/.imports.json`, partagé avec la spec.

## Limites

- Rien ne vérifie qu'un agent de ticket a tenu la spec.
- Pas de comparaison après une validation sans merge (« Valider + PR », « Valider et pousser », « Valider ») ni après un ticket sans modification : « Comparer au code » le fait à la demande.
- Les écarts ne se partagent pas par git : chaque poste a ceux de ses comparaisons.
