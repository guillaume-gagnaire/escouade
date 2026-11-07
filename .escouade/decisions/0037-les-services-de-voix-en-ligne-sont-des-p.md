---
number: 37
id: "les-services-de-voix-en-ligne-sont-des-p"
title: "Les services de voix en ligne sont des plugins de première main ; le compte Mistral est celui du plugin « Mistral »"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-285"
pages: ["plugins/livres-avec-escouade"]
tickets: ["ESC-285"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/livres-avec-escouade","added":6,"removed":0}]
accepted_at: "2026-11-01T18:07:18+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les cinq services de voix en ligne (OpenAI, Groq, Mistral, ElevenLabs, Deepgram) étaient du code du cœur. La règle « tout l'externe en plugins, en process » et le contrat VoiceService (INT-12) demandaient de les sortir. Le compte Mistral sert aussi Mistral Vibe : un compte par plugin (INT-6) obligeait à trancher.

## Décision

Chaque service est un plugin livré (dev.escouade.<service>, process escouade-plugins voice-<service>) servi par le protocole voice/*, le cœur ne parlant qu'au contrat ; le registre accepte des services de plugins et suit leurs interrupteurs, le service du cœur restant en secours si un plugin ne démarre pas. La clé reste au trousseau de l'app (dictation-<service>) et part dans chaque appel au seul plugin du service. Le plugin « Mistral » porte la voix et, plus tard, Mistral Vibe (INT-10.6), comme « GitHub » porte Issues et PR : une clé pour tout le plugin ; à INT-10.6 l'entrée dictation-mistral devient un ancien nom lu d'abord. Le code des services reste dans la lib, lié à escouade-plugins pour ces cinq entrées (écart assumé avec l'exécutable léger).

## Conséquences

Le direct coûte moins de 1 ms de plus par le plugin (médiane et p99, son au rythme du micro). Cinq processes escouade-plugins de plus quand les services sont actifs. Wispr Flow s'écrira plus tard comme sixième plugin. L'extraction du code des services dans une crate à part reste à faire.
