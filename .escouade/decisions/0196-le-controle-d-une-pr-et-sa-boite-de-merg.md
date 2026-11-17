---
number: 196
id: "le-controle-d-une-pr-et-sa-boite-de-merg"
title: "Le contrôle d'une PR et sa boîte de merge dans sa vue, par capacité de l'hébergeur et au clic seulement"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-488"
pages: ["git-fichiers/pull-requests","git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-488"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/pull-requests","added":36,"removed":9},{"page":"git-fichiers/hebergeurs-de-code","added":8,"removed":0}]
accepted_at: "2026-11-17T13:40:35+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

PR-8 des PR dans Escouade (design/pr-2-3, direction A, la boîte « prête à merger ? » en bas à droite). Le merge n'existait que sur la carte d'un ticket « En revue » (ESC-62), avec la méthode du Kanban ; ni brouillon ↔ prête, ni relecteurs ou étiquettes après l'ouverture, ni mise à jour de la branche par l'hébergeur, et la relance de la CI relançait tous les échecs. La vue d'une PR (décisions 171, 177, 187, 190) gardait la place du merge et de la boîte pour PR-8. Au retour de test, l'en-tête restait « Ouverte » après un merge : une lecture partie avant le merge et finie après était gardée comme fraîche par le cœur.

## Décision

La vue d'une PR la contrôle jusqu'à son merge : la case Brouillon, demander ou retirer une relecture et poser ou retirer une étiquette parmi ce que l'hébergeur propose, « Mettre à jour » une branche en retard (fusion de la cible ou rebase, faits par l'hébergeur), « Relancer <contrôle> seul ». Au pied de la colonne, la boîte « Prête à merger ? » dit ce qui manque, ligne par ligne avec le geste qui débloque (brouillon, contrôles, relectures et approbation exigée, fils ouverts, conflit, retard, règles de la branche protégée) ; un conflit se confie à l'agent du ticket (ESC-61), celui d'une PR d'agent s'ouvre (le lui confier viendra avec PR-6). La méthode est choisie parmi celles que le dépôt permet, celle du Kanban proposée ; supprimer la branche (pas celle d'un fork) et le worktree de l'agent qui la tient sont cochés ; l'auto-merge de l'hébergeur s'active quand seuls des contrôles ou une approbation attendent, et se désactive. Le merge passe par une confirmation dans la boîte qui dit la méthode, la cible, ce qui sera supprimé et le ticket ; la PR d'un ticket le passe en Terminé comme ESC-62. Chaque opération est une capacité déclarée par le plugin de l'hébergeur (Writing : control, draft, reviewers, labels, updateMerge, updateRebase, rerunCheck, cancelAutoMerge, deleteBranch ; protocole forge/control, candidates, setDraft, requestReviewers, removeReviewer, addLabels, removeLabel, updateBranch, rerunCheck, cancelAutoMerge, deleteBranch) : GitHub les sert toutes, les autres hébergeurs gardent leur merge et leur auto-merge (PR-9). Le cœur n'écrit que par les commandes pr_write et pr_merge de la fenêtre (et le client de l'API distante de l'utilisateur), jamais par le serveur MCP ni pour un agent. Les refus sont dits dans la boîte (jeton, droit d'écrire, conflit apparu, branche protégée, contrôle requis, limite, hors ligne) ; les noms venus de l'hébergeur sont du texte, une couleur d'étiquette n'entre dans un style qu'en six chiffres hexadécimaux. Les lectures d'une PR sont ordonnées : le cœur ne garde pas comme fraîche une lecture partie avant une écriture, la vue ne montre que la réponse la plus récente et écarte celles d'avant une écriture, « Relire la PR » relit toujours, et une écriture ou un merge disent prChanged à la fenêtre (la liste « Pull requests » se relit).

## Conséquences

La vue d'une PR n'est plus en lecture seule pour son contrôle ; quand elle est étroite ou sur l'onglet Fichiers, « Contrôle de la PR » ouvre la colonne par-dessus. Une PR d'agent peut maintenant être mergée depuis Escouade (sans être comptée dans pr_merged, réservé aux tickets). Mergée, la PR se dit « Fusionnée » aussitôt et aucune réponse plus ancienne ne la ramène à « Ouverte » ; même règle pour les autres gestes et les remarques de l'agent publiées (ESC-490). Le plugin GitHub 2.3.0 et le kit 0.1.3 gagnent onze opérations ; un jeton à droits fins demande Pull requests et Contents en écriture, et Actions pour relancer un job. Restent pour PR-9 : ces opérations chez GitLab, Gitea / Forgejo, Bitbucket et Azure DevOps ; pour PR-6 : confier une mise à jour ou un conflit à l'agent d'une PR qui n'est pas d'un ticket ; pas d'équipe parmi les relecteurs proposés, ni le détail des règles d'une branche protégée.
