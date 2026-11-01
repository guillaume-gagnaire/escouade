---
number: 44
id: "les-harness-acp-decrits-par-un-plugin-se"
title: "Les harness ACP décrits par un plugin, servis par dev.escouade.acp"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-178"
pages: ["plugins/harness-acp-decrits"]
tickets: ["ESC-178"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-acp-decrits","added":19,"removed":0}]
accepted_at: "2026-11-01T23:12:32+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

PLG-6 : un agent qui parle ACP doit pouvoir devenir un harness sans code, et le client ACP de Cursor CLI, que Vibe et Gemini appellent déjà, doit devenir générique avant la sortie de ces harness en process (revue du 2026-10-29, ESC-203 : tout l'externe en plugins, sur le contrat d'INT-9).

## Décision

Le client ACP est une crate à part (`escouade-acp`), ce qui est propre à Cursor dans son module `cursor`. Un plugin déclaratif décrit son harness (`contributes.harnesses[].acp`) ; le plugin livré `dev.escouade.acp` (dans toutes les versions) le sert sur le contrat des harness en process, la description donnée avec chaque agent. L'id est `plugin:<plugin>:<harness>` (nu pour `dev.escouade.*`) ; plugin désinstallé ou coupé : harness absent, données gardées. La clôture des projets privés est tenue par le client sur `fs/*` (`fenceByClient`) et dite manquante sinon ; `"fs": false` ferme les projets privés. Un plugin tiers ne donne jamais de chemin de programme, et l'installation montre le programme lancé (permission `harness:`). Le contrat gagne les éléments du texte en continu, de l'entrée d'un appel, du diff, des tâches, du mode, du modèle d'un sous-agent et du tour en erreur. Goose est le premier plugin (`dev.escouade.goose`).

## Conséquences

INT-10.1 (Cursor), 10.6 (Vibe), 10.7 (Gemini) décrivent leur harness sur ce client au lieu du code du cœur ; Vibe et Gemini appellent déjà les aides de la crate. Un harness décrit n'a pas encore de vue terminal ni de textes rédigés. Un plugin de première main de plus dans la liste des plugins livrés.
