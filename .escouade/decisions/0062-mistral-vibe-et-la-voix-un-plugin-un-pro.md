---
number: 62
id: "mistral-vibe-et-la-voix-un-plugin-un-pro"
title: "Mistral Vibe et la voix : un plugin, un process, un compte Mistral partagé"
date: "2026-11-03"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-327"
pages: ["plugins/mistral-vibe-en-plugin","plugins/livres-avec-escouade","plugins/comptes-partages"]
tickets: ["ESC-327","ESC-281"]
documents: []
supersedes: [57]
superseded_by: null
changes: [{"page":"plugins/mistral-vibe-en-plugin","added":11,"removed":5},{"page":"plugins/livres-avec-escouade","added":2,"removed":3},{"page":"plugins/comptes-partages","added":4,"removed":0}]
accepted_at: "2026-11-04T01:28:39+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La décision 57 sortait Mistral Vibe dans un plugin à lui pour garder ses comptes tels quels, en renvoyant le regroupement à un ticket d'avant la 2.2. La règle d'un compte par plugin (décision de l'utilisateur du 2026-10-26) et docs/PLUGINS.md §4.13 prévoyaient un seul plugin Mistral. Or Vibe a plusieurs comptes, chacun sa clé, et la voix une seule : une clé unique ferait perdre une clé à qui a deux organisations Mistral.

## Décision

Mistral Vibe est servi par dev.escouade.mistral, le même process que la voix (escouade-plugins mistral) ; ce qui n'est pas voice/* va au client ACP. Le plugin déclare le compte `mistral`, dont les points sont sa voix et le harness vibe. La clé de Réglages › Voix est ce compte (ancien nom dictation-mistral) ; un compte de Vibe qui a sa clé la garde (plugin:dev.escouade.mistral:account:vibe-<id>, ancien nom vibe:<id>). Les anciens noms restent tenus et lus d'abord, comme pour INT-6. Chaque côté se replie sur l'autre et le dit : un compte de Vibe peut laisser sa clé vide et aller par celle du compte Mistral ; la voix sans clé va par celle du premier compte de Vibe actif, ce que sa ligne affiche. Le Principal de Vibe reste la clé de vibe --setup, jamais lue. (Choix de l'utilisateur le 2026-11-03, sur captures : « Partage visible ».)

## Conséquences

Personne ne ressaisit sa clé, et une version d'avant relit les siennes. L'identifiant dev.escouade.mistral-vibe ne sort dans aucune version publiée. Couper le plugin Mistral retire à la fois la voix et Vibe, et un plantage du process touche les deux. Le plugin n'a pas de permissions.secrets : la clé part avec chaque appel de la voix et à Vibe par MISTRAL_API_KEY. Le reste de la décision 57 (Vibe en process, le moteur au cœur pour les comptes et la vue terminal, la table BUILTIN_HARNESSES) tient.
