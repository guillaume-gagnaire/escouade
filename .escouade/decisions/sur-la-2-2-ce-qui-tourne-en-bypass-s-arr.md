---
number: null
id: "sur-la-2-2-ce-qui-tourne-en-bypass-s-arr"
title: "Sur la 2.2, ce qui tourne en Bypass s'arrête quand « Autoriser le mode Bypass » s'éteint"
date: "2026-11-17"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-521"
pages: ["harness/comptes"]
tickets: ["ESC-521","ESC-497","ESC-489"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/comptes","added":1,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

ESC-489 (2.2.2) refusait d'envoyer, de reprendre ou d'ouvrir la vue d'un agent en Bypass une fois « Autoriser le mode Bypass » éteint, mais le process lancé d'avance à la création de l'agent (warm), ou l'interface gardée pour Remote Control, restait en vie dans un mode qui n'était plus permis. ESC-497 l'a corrigé sur release/2.3 avec les stratégies imposées ; la ligne 2.2 (main) n'a pas de stratégies, seul le réglage de l'app retire Bypass à chaud.

## Décision

Sur main, la part d'ESC-497 qui ne dépend pas des stratégies est portée sous la même forme et les mêmes noms (refusal_of, stopped, run_refusal, stop_out_of_rule, judge_start, settle_agents, settle_locked_mode) : quand le réglage s'éteint, chaque process et chaque interface en Bypass s'arrête, au repos tout de suite, en plein tour à la fin du tour, avec une notice qui dit pourquoi ; son mode ne change pas. Tout démarrage refusé (envoi, reprise, « Relancer », vue terminal, démarrage d'avance, interface en fond) arrête aussi ce qu'il trouve en vie.

## Conséquences

Le prochain reversement de main dans release/2.3 garde la version de la 2.3 de ces fonctions (avec les stratégies) ; sur main elles vivent dans modes.rs, sur la 2.3 settle_agents et settle_locked_mode sont dans locks.rs : la version de main est à retirer au reversement. La notice de main est de niveau « warn » (la 2.3 écrit « warning », qui n'a pas de style).
