# Réponses de GitLab

Les tests du plugin « GitLab » (`src-tauri/src/forge/gitlab_tests.rs`) nourrissent le faux serveur de ces réponses.

- **Enregistrées sur gitlab.com** par `node record.mjs` (sans jeton), le 2026-11-01 : la MR !3101 de `gitlab-org/api/client-go` (un projet dans un sous-groupe), fusionnée, venue du fork `gitlab-community/gitlab-org/api/client-go` (trois niveaux de groupes), avec son auto-merge (`merge_when_pipeline_succeeds`), ses approbations, ses relecteurs, ses pipelines, les jobs de son pipeline de tête et le journal d'un job : `project.json`, `fork-project.json`, `merge-request.json`, `branch-mrs.json`, `approvals.json`, `reviewers.json`, `mr-pipelines.json`, `jobs.json`, `job-id.txt`, `job-log.txt`.
- **Écrites d'après la documentation de l'API** (gitlab.com répond 401 sans jeton) : `discussions.json` (un fil de diff résolu à deux notes, un fil ouvert sur une ancienne ligne, une note sans ligne, une note système) et `deployments.json` (les plus récents d'abord : un déploiement en cours sans URL externe, une revue en succès après un échec plus ancien du même environnement, la production d'une autre branche). Avec `GITLAB_TOKEN`, `record.mjs` les enregistre aussi, et lit le journal par `/jobs/:id/trace`.

**Sans les personnes** : publiques sur gitlab.com, leurs données ne sont pas recopiées ici. `anonymize.mjs`, que `record.mjs` passe sur tout ce qu'il écrit (les fichiers gardés compris), donne à chacun un alias fixe (alice, bob, carol… dans l'ordre des ids, qui restent), son nom (Alice…), son profil `https://gitlab.com/<alias>` et ses e-mails `<alias>@example.com`, et remplace logins, noms et adresses dans les textes et le journal. `node anonymize.mjs` le repasse sur les fichiers du dossier. Après un enregistrement, `grep -rniE '@[a-z0-9.-]+\.[a-z]{2,}' .` ne doit trouver que `example.com` et `git@gitlab.com`.

Ce qui reste à essayer sur une vraie instance est dans `docs/TODO-2.2.md`.
