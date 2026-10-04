# Product

<!-- impeccable:product-schema 1 -->

Ce que les skills Impeccable doivent savoir d'Escouade, quel que soit l'écran : l'app comme le site.

## Platform

web

L'app est une fenêtre desktop Tauri 2 (Windows et macOS) dessinée en technologies web (Svelte 5) ; le site de présentation est un site statique Nuxt 4, dans `website/`.

## Users

Des développeurs qui font travailler plusieurs agents Claude Code en parallèle sur leurs projets, sous Windows 10 ou 11 ou macOS 11 et plus, avec un abonnement Claude ou une clé API. Ils lancent le travail, puis reviennent quand un agent a besoin d'eux : une question, une autorisation, une tâche finie, un ticket à tester.

## Product Purpose

Escouade est le poste de pilotage de ces agents : toutes les sessions Claude Code de tous les projets dans une seule fenêtre native, pour garder la vue d'ensemble et n'intervenir que quand il le faut. Réussi : plusieurs agents avancent en même temps sans qu'on surveille des terminaux, et rien n'attend sans qu'on le sache.

## Positioning

Une fenêtre qui pilote une escouade d'agents Claude Code : un onglet par projet, autant d'agents que nécessaire, un worktree git par agent, un vrai chat au lieu d'un terminal, un signal dès qu'un agent attend, quotas et coût en direct, et un Kanban de tickets que des agents prennent seuls et sur lesquels ils bouclent jusqu'à atteindre leurs critères d'acceptation. Gratuit, local et open source (MIT), par-dessus le Claude Code de l'utilisateur.

## Operating Context

- Chaque agent est un process `claude` local, dans le dossier du projet ou dans son propre worktree ; conversations, projets et statistiques restent dans `~/.escouade/`.
- Le réseau ne sert qu'à Claude Code lui-même, à la lecture des quotas et aux mises à jour de l'app.
- Statuts d'un agent : Prêt, En cours, Question, Terminé (et erreur). Un ticket passe par À faire, En cours (avec sa boucle, « boucle 2/5 »), À tester, Terminé.
- Les raccourcis portent l'usage quotidien : Ctrl+N nouvel agent, Ctrl+J prochain agent qui attend, Ctrl+1…9 projets.

## Capabilities and Constraints

- Projets en onglets, agents en parallèle avec leur modèle (Fable, Opus, Sonnet, Haiku), leur effort et leur mode de permission.
- Chat natif : markdown, appels d'outils compacts, questions et autorisations en cartes cliquables.
- Notifications : onglet et carte qui clignotent, carillon, notification système, barre des tâches ou Dock.
- Git : fichiers non commités, diffs, git graph, commit rédigé par l'agent, merge ou squash d'un worktree.
- Éditeur intégré et vrais terminaux (PowerShell, Git Bash, WSL ; zsh, bash, fish).
- Commandes de lancement du projet, et lancements de test sur des ports réservés pour les agents de ticket.
- Barre de statut et statistiques : tokens et coût en direct, quotas de session de 5 h et hebdomadaire.
- Kanban (1.3) : des tickets avec critères d'acceptation, pris par des agents dans leur propre worktree, qui bouclent jusqu'à les atteindre puis passent « À tester » ; validation par merge, pull request ou push.
- Remote control depuis claude.ai et l'app Claude sur mobile.
- Pas encore de Linux. Pas encore signé avec un certificat Apple Developer ID (le premier lancement sous macOS passe par « Ouvrir »).

## Brand Commitments

- Nom : Escouade (anciennement « Claude Code Manager » ; l'ancien nom et « CCM » n'apparaissent plus).
- Logo : trois fenêtres arrondies empilées, celle de devant portant une étincelle à huit rayons, en #35C1BD (`public/logo.svg`, `website/public/logo.svg`).
- Voix : en français, au tutoiement, simple et concrète, sans emphase publicitaire.
- Projet indépendant, non affilié à Anthropic ; Claude et Claude Code sont des marques d'Anthropic, et le site le dit.

## Evidence on Hand

- Captures et vidéo de présentation, rendues depuis les composants de l'app dans `video/` : `website/public/images/*.jpg`, `website/public/escouade.mp4`.
- La version, lue dans `src-tauri/tauri.conf.json` ; les releases sur GitHub.
- Aucun témoignage, nombre d'utilisateurs, benchmark ni article de presse : rien de tout cela ne s'invente. Les données de démonstration (noms d'agents, tokens, coûts) sont fictives, comme sur les captures.

## Product Principles

- Le développeur garde la vue d'ensemble ; les agents font le travail et disent quand ils ont besoin de lui.
- Montrer la vraie chose : l'app au travail plutôt que des promesses sur elle.
- Local et transparent : le Claude Code de l'utilisateur, sa machine, du code ouvert.
- Des mots qui disent exactement ce qui se passe, en français.

## Accessibility & Inclusion

Les couleurs de texte respectent le WCAG AA (4,5:1) sur le fond (testé dans `website/tests/tokens.test.ts`) ; les animations respectent `prefers-reduced-motion` ; tout se fait au clavier, avec un focus visible.
