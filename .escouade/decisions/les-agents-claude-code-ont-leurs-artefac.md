---
number: null
id: "les-agents-claude-code-ont-leurs-artefac"
title: "Les agents Claude Code ont leurs artefacts, comme dans un terminal"
date: "2026-11-09"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-400"
pages: ["harness/claude-code","conversation/conversation"]
tickets: ["ESC-400"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/claude-code","added":2,"removed":0},{"page":"conversation/conversation","added":1,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Retour de l'utilisateur (2026-11-09) : depuis Escouade, un agent Claude Code ne crée pas d'artefact (« Unknown skill: artifact-design »), alors que le même claude le fait dans un terminal. Cause prouvée (Claude Code 2.1.319, binaire lu et essais réels) : en stream-json, Claude Code se tient pour une session de l'Agent SDK (CLAUDE_CODE_ENTRYPOINT=sdk-cli) et coupe ses artefacts par défaut (« sdk_default_off ») sauf si CLAUDE_CODE_ARTIFACT est vraie. Même lancement qu'Escouade : sans la variable, ni outil Artifact ni skill ; avec, Artifact, ArtifactComments, ArtifactData et la skill artifact-design.

## Décision

Le plugin dev.escouade.claude-code (2.2.4) pose CLAUDE_CODE_ARTIFACT=1 à chaque lancement d'agent, discussion comprise. Ce qui coupe les artefacts pour l'utilisateur (CLAUDE_CODE_DISABLE_ARTIFACT, ses réglages de Claude Code, son organisation) les coupe toujours. La conversation nomme les outils en clair (Artefact, Commentaires d’artefact, Données d’artefact, Vérification d’artefact) et met sur la ligne d'une publication réussie le lien « Ouvrir l’artefact », ouvert dans le navigateur au clic seulement.

## Conséquences

Un agent publie des pages privées sur claude.ai comme dans un terminal ; leur suppression passe par une carte de permission. Une variable de plus dans le lancement du plugin (la seule qu'il pose). Une valeur CLAUDE_CODE_ARTIFACT=0 mise par l'utilisateur dans l'environnement d'Escouade est recouverte : la façon de couper reste CLAUDE_CODE_DISABLE_ARTIFACT ou le réglage de Claude Code. Un agent en stream-json ne suit pas les nouvelles versions d'un artefact publiées ailleurs (réservé par Claude Code aux sessions interactives).
