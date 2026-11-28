---
number: null
id: "la-sensibilite-du-micro-se-regle-avec-un"
title: "La sensibilité du micro se règle, avec un testeur en direct"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-665"
pages: ["voix/conversation-vocale","voix/dictee"]
tickets: ["ESC-665"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"voix/conversation-vocale","added":7,"removed":0},{"page":"voix/dictee","added":2,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur, le 2026-11-26 : « pour la détection vocale, il faudrait pouvoir configurer le seuil de détection, avec un petit testeur pour voir quand la voix est détectée ou pas ». Jusqu'à la 2.2.11, le détecteur de la conversation vocale (vad.rs) avait deux constantes : 3,2 fois le bruit de la pièce (+10 dB) et un plancher de -46 dBFS. Une voix basse ou un micro lointain n'était pas entendu, une pièce bruyante coupait l'agent, et rien ne permettait de voir pourquoi.

## Décision

« Sensibilité du micro » dans Réglages › Voix › Conversation : 11 crans de -5 à +5, 0 par défaut, qui est exactement le détecteur d'avant. Chaque cran déplace de 1,5 dB le seuil au-dessus du bruit et le plancher ensemble (le plancher seul décide dans une pièce calme) ; le seuil ne descend jamais sous 2 fois le bruit (+6 dB). Il agit sur la conversation vocale seulement, quelle que soit la transcription : début et fin de phrase, et coupure de l'agent (le seuil sur la pièce, jamais ce qui est appris de l'écho). La dictée n'en dépend pas : elle n'a pas de détecteur. Gardé dans les réglages du poste, jamais dans .escouade.yml. À côté, « Tester » écoute le micro choisi avec le même détecteur et montre le niveau, le bruit appris, la ligne du seuil (que le curseur déplace sur-le-champ) et « Voix détectée » ; il ne garde ni n'envoie rien, seuls des niveaux vont à la fenêtre.

## Conséquences

Un nouveau champ speech.sensitivity (0 par défaut, donc rien ne change pour les réglages existants) ; trois commandes voice_meter_start/set/stop et deux événements meter/meterEnded ; la formule des crans existe en Rust (vad::Levels) et en TypeScript (mic-meter.svelte.ts), tenues égales par leurs tests. Le testeur prend le micro : une dictée ou une conversation qui commence l'arrête, et il ne s'ouvre pas pendant elles.
