---
number: 34
id: "le-contrat-des-harness-en-process-lancem"
title: "Le contrat des harness en process : lancement vérifié par le cœur, lots sans attente, mémoire déclarée"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-221"
pages: ["plugins/harness-en-process","plugins/livres-avec-escouade"]
tickets: ["ESC-221"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-en-process","added":17,"removed":0},{"page":"plugins/livres-avec-escouade","added":2,"removed":2}]
accepted_at: "2026-11-01T15:42:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-9 (ESC-221) pose le contrat des harness en process (docs/PLUGINS.md §4.5 et §4.6, revue du 2026-10-29) et le prouve sur le faux moteur minimal devenu un plugin. Il faut trancher plusieurs points que le plan laissait ouverts ou que la mesure a contredits.

## Décision

1. Le plugin décrit le lancement (engine/launch, terminal/launch) sans jamais donner le programme ; le cœur lance celui qu'il a trouvé, et refuse une description qui met le jeton d'Escouade dans les arguments, qui n'applique pas la clôture (fenceApplied, et chaque chemin refusé présent), qui ne tient pas read_only quand il est demandé, ou qui écrit hors du dossier de l'agent.
2. Un lot d'agent/frames est ce qui a été lu pendant que le précédent était chez le plugin, jusqu'à 64 Kio, sans attente : l'attente de 8 ms prévue ajoutait jusqu'à 8 ms par ligne au rythme réel, au-delà du seuil de 5 ms.
3. Ce que le plugin écrit au harness vient dans ses réponses (writes, et l'élément write), joué dans l'ordre, plutôt que par une requête agent/write du plugin.
4. La reprise de la mémoire du harness est déclarée dans le manifeste (memory.dir), lue sans lancer le plugin, comme le reste de ce qui sert avant tout agent (§4.6, point 4).
5. terminal/hook porte la réponse du cœur (un contexte pour le modèle, un refus), que le plugin met dans la forme de son harness ; le branchement à TerminalView vient avec le premier harness en process qui a une vue (INT-10.1).
6. Le faux en process (engine/minimal/) reste pour les tests de la passation ; le harness « Minimal » du plugin n'est pas au registre de l'app tant qu'aucun vrai harness n'y passe.

## Conséquences

Rien ne change pour l'utilisateur d'une release. En développement, la fiche du plugin de démonstration dit qu'il ajoute un harness. Les mesures sur Minimal (build de debug) : +0,66 ms au 99e centile au rythme réel, 41 119 lignes/s, 14 Mo pour le plugin. INT-10.x s'appuie sur ce contrat et refait les mesures.
