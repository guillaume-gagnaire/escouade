---
title: "Hébergeurs de code"
subject: "Git & fichiers"
sources: ["docs/SPEC.md","plugins/gitlab/README.fr.md","plugins/gitlab/docs/fr/","tests/fixtures/forges/gitlab/README.md","docs/TODO-2.2.md (Hébergeurs de code)","plugins/gitea/README.fr.md","plugins/gitea/docs/fr/","tests/fixtures/forges/gitea/README.md","docs/TODO-1.8.md","src-tauri/src/telemetry.rs"]
tickets: ["ESC-54","ESC-55","ESC-56","ESC-58","ESC-70","ESC-241","ESC-212","ESC-57","ESC-67","ESC-345","ESC-69","ESC-367","ESC-383"]
decisions: [104]
compared: null
---

## Ce que c'est

Escouade reconnaît chez quel hébergeur est le dépôt distant d'un projet : GitHub (github.com, GHE.com, GitHub Enterprise Server), GitLab, Bitbucket Cloud, Gitea / Forgejo (Codeberg compris) ou Azure DevOps. Elle s'y connecte par un compte, pour ouvrir et suivre les pull requests et leur CI (voir *Pull requests*, et *Validation* pour celles d'un ticket). Chaque hébergeur déclare ce qu'il sait faire (brouillon, auto-merge, checks, journaux de jobs, relectures, fils, déploiements, stratégies de merge) ; l'interface ne propose que cela. Une PR s'appelle « MR » sur GitLab, « PR » ailleurs.

## Comportement

### Reconnaître le dépôt d'un projet

- Le dépôt distant d'un projet est celui que suit sa branche cible, sinon `origin`, sinon le seul.
- Adresses lues : https, http, ssh (`ssh://`, `git+ssh://`), `git://` et la forme scp (`git@hôte:chemin`), avec utilisateur ou jeton, port, `.git` ou non.
- L'hôte donne l'hébergeur : github.com et `ssh.github.com`, `*.ghe.com` ; gitlab.com et `altssh.gitlab.com` ; bitbucket.org et `altssh.bitbucket.org` ; codeberg.org et gitea.com. Sinon, le compte connecté pour cet hôte (son port https ou ssh à part).
- Le chemin donne propriétaire et dépôt (GitHub, Gitea), workspace et dépôt (Bitbucket), ou groupes, sous-groupes et projet (GitLab).
- Azure DevOps : `https://[<org>@]dev.azure.com/<org>/<projet>/_git/<dépôt>` (`<org>/_git/<dépôt>` quand le projet porte le nom du dépôt), `https://<org>.visualstudio.com/[DefaultCollection/]<projet>/_git/<dépôt>` (gardé comme `https://dev.azure.com/<org>`), ssh `git@ssh.dev.azure.com:v3/<org>/<projet>/<dépôt>` (et `vs-ssh.visualstudio.com`), et ceux d'un Azure DevOps Server (`https://serveur/tfs/<collection>`, `[tfs/]<collection>/<projet>/_git/<dépôt>`, https ou ssh). Les espaces encodés (`%20`) sont décodés.

### Comptes

- Un compte par instance (son hôte, port compris), plusieurs par hébergeur. Sur Azure DevOps, un compte par organisation (ou collection), un par hôte.
- Celui de github.com est aussi le compte de GitHub Issues : le connecter ou le déconnecter d'un côté le fait de l'autre (voir *Intégrations*).
- Le jeton est gardé dans le trousseau du système, sous `forge:<hôte>`, avec les mêmes règles que les jetons des intégrations (voir *Intégrations*).
- Jeton vide : celui de la CLI de l'hébergeur, demandé quand il sert (`gh auth token --hostname <hôte>`, `glab auth status --show-token --hostname <hôte>`). Sur dev.azure.com, celui du CLI `az` (`az account get-access-token --resource 499b84ac-1321-427f-aa17-267ca6975798`), redemandé cinq minutes avant qu'il expire.
- Bitbucket Cloud : l'e-mail du compte Atlassian et un jeton d'API à scopes. Azure DevOps : un jeton d'accès personnel, droits demandés `vso.code_write`, `vso.build_execute`, `vso.work_write`, créé sur `<organisation>/_usersSettings/tokens`. Gitea / Forgejo : un jeton d'accès à scopes (`write:repository`, `write:issue`, `read:user`), créé sur `/user/settings/applications` de l'instance.
- L'adresse est en https (http seulement vers cette machine). API : api.github.com, `api.<nom>.ghe.com`, `/api/v3` d'un GitHub Enterprise Server ; `/api/v4` de GitLab (gitlab.com ou l'adresse de l'instance, chemin compris : `https://acme.dev/gitlab/api/v4`) ; api.bitbucket.org/2.0 ; `/api/v1` de Gitea et de Forgejo ; dev.azure.com. `ESCOUADE_GITHUB_API`, `ESCOUADE_BITBUCKET_API` et `ESCOUADE_AZURE_DEVOPS_API` remplacent l'adresse publique.
- La vérification d'un compte lit qui il est : sur Azure DevOps, l'e-mail du compte de l'organisation, sinon son nom (un utilisateur anonyme est un refus).

### Ce qu'Escouade fait sur GitHub

- **Ouvrir une PR** : en brouillon si demandé ; `Closes #n` est ajouté à la description s'il manque. Relecteurs et étiquettes sont demandés ensuite : si GitHub les refuse, la PR reste ouverte.
- **Lire les PR** : état, approbations, changements demandés, fils ouverts, conflit, auto-merge et checks de la dernière révision ; toutes les PR demandées en une seule requête. Aussi les PR d'une branche.
- **Merger** (merge, squash, rebase) et **activer l'auto-merge** ; quand le dépôt ne l'autorise pas, l'erreur de GitHub est rendue telle quelle.
- **Checks** : check runs et statuts ; le journal d'un job de GitHub Actions ; « relancer ce qui a échoué » relance une exécution entière, pas un job.
- **Relectures** (le dernier mot de chacun), **commentaires** (fils de lignes avec leur état résolu, et la conversation), commenter, répondre, publier une relecture, et le dernier statut de chaque déploiement.
- **Économie du quota** : les lectures sont conditionnelles (une réponse inchangée ne compte pas dans la limite) ; les listes suivent les pages, 10 au plus. Sous 10 % du quota restant, Escouade étale ce qui reste de la fenêtre sur les appels restants ; à zéro, elle attend la remise à zéro, jamais plus de 30 s d'un coup. Les requêtes passent par le proxy des réglages (voir *Réglages*).

### Ce qu'Escouade fait sur GitLab
Sur gitlab.com et les instances auto-hébergées, par le plugin livré « GitLab » (voir *Les plugins livrés avec Escouade*). Le projet est nommé par son chemin entier, groupes et sous-groupes compris.
- **Ouvrir une MR** : en brouillon par le préfixe `Draft:` du titre ; `Closes #n` ajouté à la description s'il manque ; étiquettes ; relecteurs trouvés par leur identifiant (un relecteur inconnu est laissé de côté, la MR s'ouvre quand même).
- **Lire les MR**, une par une (une MR disparue est laissée de côté), celles d'une branche (la plus récente d'abord) et les 30 ouvertes du projet : état (ouverte, mergée, fermée), brouillon (le préfixe ôté du titre), conflit (rien n'est dit tant que GitLab vérifie, ni d'une MR mergée ou fermée), approbations, changements demandés, fils ouverts (lus seulement quand GitLab dit qu'il en reste), auto-merge, état du pipeline de tête, fork (son chemin, lu une fois). Les MR ouvertes du projet sont lues en un appel, sans approbations ni pipeline.
- **Merger** (merge ou squash ; le rebase et le fast-forward sont un réglage du projet, pas de l'appel : refusés avant tout envoi) et **merger quand le pipeline passe** (`auto_merge`, et `merge_when_pipeline_succeeds` des versions d'avant la 17.11, envoyés ensemble) ; GitLab qui n'active rien est une erreur.
- **Checks** : les jobs du pipeline de tête de la MR ; un job autorisé à échouer ne fait pas échouer (l'avertissement de GitLab) ; un job manuel facultatif est ignoré, un manuel bloquant en attente. Le journal d'un job par `/jobs/:id/trace` (dans le projet du pipeline). « Relancer ce qui a échoué » relance le pipeline (ses jobs en échec, en un appel).
- **Relectures** : les approbations, et ce que disent les relecteurs quand l'instance le donne (`/reviewers` : changements demandés, relu). **Commentaires** : les notes des discussions, sur une ligne du diff ou non, sans les notes système, avec leur état résolu. Commenter, répondre dans une discussion, la résoudre, publier une relecture (chaque ligne en discussion sur le diff, le texte en note, puis l'approbation ; l'API n'a pas de « changements demandés »).
- **Déploiements** de la branche : le dernier de chaque environnement parmi les 50 plus récents du projet (GitLab ne les filtre pas par branche), son état et l'adresse de l'environnement, sinon celle de son job.
- **Limites d'API** : les en-têtes `RateLimit-*` ralentissent les appels comme sur GitHub ; un 429 est attendu (`Retry-After`, 30 s au plus) puis la requête renvoyée une fois ; les listes suivent les pages (`Link`), 10 au plus. Chaque réponse dit à Escouade ce qui reste au compte.

### Ce qu'Escouade fait sur Gitea et Forgejo (Codeberg compris, ESC-58)
Le client est le plugin livré `dev.escouade.gitea`. Gitea et Forgejo partagent leur API pour ce dont Escouade a besoin ; le client lit la version de l'instance (`/api/v1/version`, une fois par compte ; celle de Forgejo porte `+gitea-<version>`) pour ce qui les distingue.
- **Ouvrir une PR** : un brouillon est un titre préfixé `WIP:` (ni l'un ni l'autre n'a de champ brouillon à écrire) ; `Closes #n` est ajouté à la description s'il manque ; les étiquettes sont données par leur identifiant (une étiquette inconnue est laissée de côté) ; les relecteurs sont demandés ensuite, et un refus ne reprend pas la PR.
- **Lire les PR** : état, brouillon, conflit (d'une PR ouverte), fork, approbations et changements demandés (le dernier mot de chacun ; un commentaire seul ne reprend pas une approbation, une relecture en attente ou écartée ne compte pas), fils ouverts (commentaires de relecture sans résolution) et état combiné des statuts de la dernière révision. Aussi les PR d'une branche, la plus récente d'abord. L'auto-merge déjà programmé n'est pas dit par l'API : il n'est pas affiché.
- **Merger** (merge, squash, rebase ; le rebase suivi d'un commit de merge existe aussi) et **auto-merge** (`merge_when_checks_succeed`) : l'instance ne le programme que sur une branche protégée ; le client lit la branche avant et, si elle ne l'est pas, refuse sans rien envoyer en disant laquelle. Une erreur de l'instance (contrôles non verts…) est rendue telle quelle.
- **Checks** : les statuts de la dernière révision (le plus récent de chaque contexte) ; ceux qu'Actions poste sont reliés à leur job par leur adresse, avec ses heures chez Gitea. Une CI tierce n'a qu'un statut et une adresse. Le **journal** d'un job se télécharge, et « relancer ce qui a échoué » relance chaque exécution en échec.
- **Relectures et fils** : le dernier mot de chacun, les commentaires de lignes avec leur état résolu et la conversation, commenter, publier une relecture avec ses lignes ; répondre dans un fil et le résoudre.
- **Capacités absentes, déclarées comme telles** : les **déploiements** n'existent ni chez Gitea ni chez Forgejo (aucune API) : la capacité est absente, rien n'est demandé. Ce qui dépend de la version est dit appel par appel (« Cet hébergeur ne sait pas faire »), jamais une erreur ni une réponse fausse, et l'interface n'affiche rien pour cela : le **journal** et les jobs des Actions (Gitea 1.25 et plus, Forgejo 12 et plus ; en dessous, des statuts sans job), la **relance** (Gitea 1.26 et plus ; aucune chez Forgejo), **répondre dans un fil et le résoudre** (Gitea 1.26 et plus ; aucun chez Forgejo). Ces versions viennent des sources et des notes de version, pas d'un essai.
- **Écarts relevés entre les deux** (d'après les swagger de Forgejo 16 et de la branche principale de Gitea) : le corps du merge (`do` / `Do`), les relecteurs à la création (Gitea seul), les exécutions et leurs jobs (`run_number`, `conclusion`, `{ jobs }` chez Gitea ; `index_in_repo`, un statut, une liste chez Forgejo).

### Ce qu'Escouade fait sur Azure DevOps

- **Ouvrir une PR** : brouillon, description coupée à 4 000 caractères, relecteurs trouvés par leur identité puis ajoutés un à un, étiquettes, et l'élément de travail d'un ticket venu d'Azure Boards fermé par la PR. Un relecteur ou une étiquette refusés ne reprennent pas la PR.
- **Lire les PR**, une par une (une PR disparue est laissée de côté), et celles d'une branche, la plus récente d'abord : état (active ; terminée = mergée ; abandonnée = fermée), brouillon, conflits, votes des personnes (pas des équipes : 10 et 5 approuvent, -5 et -10 demandent des changements), auto-complete, fork, fils ouverts sur des lignes.
- **Merger** et **auto-merge** (l'auto-complete, posé au nom du jeton) avec la stratégie demandée (merge sans fast-forward, squash, rebase, rebase puis merge). Une stratégie que la politique de la branche cible n'autorise pas est refusée avant tout envoi.
- **Checks** : les évaluations des politiques de build et de statut (celles des relecteurs, commentaires et éléments de travail sont la relecture), le dernier statut de la PR par contexte, puis les builds de la PR qu'aucune politique ne tient (le plus récent par pipeline). Journal : celui des tâches en échec du build (sinon de la dernière tâche). « Relancer ce qui a échoué » relance chaque build en échec.
- **Relectures** : les votes. **Commentaires** : ceux écrits dans les fils (pas les messages d'Azure : votes, pushes) ; un fil est résolu quand il est corrigé, fermé, « won't fix » ou « by design ». Commenter, répondre, résoudre un fil. Publier une relecture : un fil par commentaire de ligne, le texte, puis le vote du jeton (10, -5, aucun pour un simple commentaire).
- **Refus** : un 401, un 203 ou une page de connexion sont un jeton refusé. Une limite (429, ou 503 avec `Retry-After`) est attendue une fois, 30 s au plus, puis la requête renvoyée ; les en-têtes de limite ralentissent les appels comme sur GitHub.

## Réglages

Réglages › Services connectés › « Hébergeurs de code », une page d'Escouade (voir *Réglages*) :

- Chaque compte : hébergeur, hôte, état de sa dernière vérification (« Connecté » ; « Jeton expiré ou révoqué » sur un 401 ; « Droits manquants : … » quand l'hébergeur dit les droits du jeton ; « Injoignable »), « Jeton de gh », « Sert aussi à GitHub Issues » (ou à GitLab Issues ; sinon « Pas de tickets : PR et CI seulement »), les projets dont le dépôt est sur l'instance, « Déconnecter ». Les comptes sont vérifiés à l'ouverture de la page et par « Vérifier ».
- Les hôtes des dépôts des projets sans compte : « Aucun compte, pour le projet … » et « Connecter <hôte> », qui pré-remplit l'adresse (celle du dépôt en https, port compris ; celle de l'organisation sur Azure DevOps) et devine le type d'instance (GitLab, Gitea, Forgejo) en interrogeant sa version.
- « Connecter un hébergeur… » : hébergeur, adresse (son instance publique si vide), e-mail (Bitbucket), jeton (vide : celui de la CLI), les droits à lui donner, « Créer un jeton ».
- Les messages du cœur qui renvoient à un compte manquant (« Hébergeur inconnu pour <hôte> : connecte-le dans Réglages › Services connectés. ») nomment cette page ; ceux des plugins livrés disent encore « Réglages › Intégrations » jusqu'à leur prochaine version au store.

## Limites

- Bitbucket Cloud : rien n'a été essayé pour de vrai (écrit d'après la documentation de l'API, testé sur un faux serveur ; la liste des essais à faire est dans `docs/TODO-2.2.md`) ; pas d'auto-merge (l'API n'en a pas), pas d'étiquettes, pas de relance de jobs.
- Les droits d'un jeton ne sont lus que pour un jeton GitHub classique et sur GitLab ; un jeton GitHub à droits fins, Bitbucket, Gitea / Forgejo et Azure DevOps ne les disent pas.
- GitHub : les écritures (créer, merger, auto-merge, commenter) n'ont pas encore été essayées pour de vrai (voir `docs/TODO-1.8.md`).
- GitLab : écrit d'après sa documentation et ses sources, essayé sur des réponses enregistrées sur gitlab.com, jamais sur une vraie instance avec un jeton (décision du 2026-10-16 ; ce qui reste à essayer : `docs/TODO-2.2.md`). Les jobs d'un pipeline déclenché (`bridges`) et les statuts externes d'un commit ne sont pas des checks.
- Gitea / Forgejo : aucun essai réel (décidé le 2026-10-16) ; le client est écrit d'après les swagger des deux et testé sur un faux serveur nourri de réponses écrites d'après eux. Restent à essayer sur Codeberg et un Gitea, les routes des Actions d'abord (`docs/TODO-2.2.md`).
- Azure DevOps : pas de déploiements ; le jeton de `az` ne vaut que pour dev.azure.com ; un essai réel avec un jeton reste à faire (ouvrir une PR brouillon, la lire, commenter, répondre, résoudre, l'abandonner).

## Les comptes servis par un plugin (ESC-212)

- Le jeton d'une instance est aussi sous `plugin:<id>:account:<hôte>` ; `forge:<hôte>` reste tenu à côté pour une version d'avant (voir *Les comptes partagés par plugin*).
- Quand un plugin installé sert le même hébergeur, le formulaire demande « Servi par » ; ses PR, sa CI et sa source de tickets passent alors par ce plugin. « Sert aussi à … » suit les points du compte.

## Ce qu'Escouade fait sur Bitbucket Cloud (ESC-57)

Servi par le plugin livré `dev.escouade.bitbucket` ; compte : e-mail et jeton d'API Atlassian (les mots de passe d'application sont retirés).

- **Ouvrir une PR** : brouillon si demandé (le champ `draft`), `Closes #n` ajouté à la description s'il manque, relecteurs par identifiant de compte ou `{uuid}` ; si Bitbucket refuse un relecteur (un pseudo n'en est pas un), la PR s'ouvre sans eux. Pas d'étiquettes sur Bitbucket.
- **Lire les PR**, une par une (une PR disparue est laissée de côté), celles d'une branche (tous états) et les ouvertes d'un dépôt : état (fusionnée, refusée ou remplacée = fermée), brouillon, approbations et changements demandés (les participants), fils de lignes ouverts, fork. Les fils et les checks ne sont lus que pour une PR ouverte. Bitbucket ne dit pas d'avance si une PR a des conflits : « inconnu ».
- **Merger** (merge commit, squash, rebase fast-forward, rebase puis merge) ; un merge que Bitbucket fait en tâche de fond est attendu. **Pas d'auto-merge** : l'API n'en a pas, la capacité est déclarée absente ; Escouade attend la CI et merge elle-même, et le dit.
- **Checks** : les statuts de commit de la PR, un par clé (le dernier mot) ; un run de Bitbucket Pipelines a un journal, lu étape après étape sous le nom de l'étape (la fin de chacune, 256 Ko au plus). Une autre CI ne donne qu'un statut et une adresse. Pas de relance.
- **Relectures** (approuvé, changements demandés), **commentaires** (fils de lignes et leur état résolu, la conversation, les supprimés écartés), commenter, répondre, résoudre un fil, publier une relecture (commentaires de ligne, texte, puis approbation ou changements demandés), et les déploiements d'un commit par environnement.
- **Limite horaire** : 1000 appels par heure (plus pour un gros espace de travail) sans compteur dans les réponses : Escouade compte ses appels, croit `X-RateLimit-Limit`, prend `X-RateLimit-NearLimit` pour « un cinquième restant », étale ce qui reste de l'heure (30 s au plus d'un coup) ; un 429 est attendu (`Retry-After`) puis renvoyé une fois.

## Les environnements d'un dépôt (ESC-67)

Une opération de plus des hébergeurs, `environments` (`forge/environments` des plugins) : les déploiements récents du dépôt, toutes branches, le plus récent d'abord, et de chaque environnement au moins le dernier et le dernier réussi, avec leur commit, leur date, leur auteur, l'adresse de l'environnement et celle du job. Un hébergeur qui ne la sert pas répond « Cet hébergeur ne sait pas faire » (voir *Environnements et déploiements*).

- **GitHub** : `/deployments` (30), puis le dernier statut de chacun jusqu'au dernier succès de chaque environnement (12 statuts au plus) ; les environnements transitoires (aperçus) sont laissés de côté. Adresse : `environment_url` ; job : `log_url`, sinon `target_url`.
- **GitLab** : `/deployments` du projet (50, les plus récemment mis à jour d'abord) ; adresse : `external_url` de l'environnement ; job : son `deployable`.
- **Bitbucket Cloud** : les environnements, puis les déploiements (3 pages), rangés par leur date ; pas d'adresse d'environnement ni d'auteur ; job : l'exécution du pipeline.
- Une PR lue mergée dit aussi le commit de son merge (`mergeCommit` de GitHub, `squash_commit_sha` / `merge_commit_sha` / `sha` de GitLab, `merge_commit` de Bitbucket).
- Un déploiement lu dit aussi son commit, sa date, son auteur et son job, dans `deployments` comme dans `environments`.

## Les hébergeurs en WASM (ESC-345)

Les cinq hébergeurs ne sont plus des process `escouade-plugins <nom>` : chacun est un module WASM, dans l'hôte de l'app (voir *Tous les plugins en WASM*). Tout ce qui précède reste vrai, à ceci près :

- Le proxy et la vérification TLS sont ceux de l'hôte : un changement de Réglages › Réseau atteint l'appel suivant, sans relancer le plugin.
- Un journal de CI de plus de 8 Mio se dit trop long. En process, il se lisait.
- Sur une GitHub Enterprise Server dont le stockage des journaux est sur un autre hôte que l'instance, le journal d'un job ne se lit pas : le plugin dit le refus. Le reste marche.

## Statistiques d'usage

Quatre événements disent, si l'utilisateur a accepté les statistiques, que les hébergeurs servent. Rien de plus : ni dépôt, ni numéro, ni titre, ni adresse d'une instance.

- `pr_opened` : Escouade ouvre une PR (une par dépôt d'un projet à plusieurs dépôts). `from` : `agent` ou `ticket` ; `forge`.
- `pr_merged` : la PR d'un ticket est mergée, depuis Escouade ou chez l'hébergeur. `forge`. Une PR d'agent ordinaire n'est pas comptée : Escouade ne la merge pas.
- `ci_fix_started` : une CI en échec est donnée à un agent. `from` : `agent` ou `ticket` ; `by` : `user` ou `autopilot`. Un refus n'est pas compté.
- `pr_review_started` : une PR part en relecture chez un agent (voir *Relecture d'une PR par un agent*). `forge`. Ce qu'il en dit n'est pas envoyé.

`forge` vient d'une liste fermée : `github`, `gitlab`, `bitbucket`, `gitea` (Forgejo compris), `azure`, sinon `other` pour l'hébergeur d'un plugin d'un tiers, jamais nommé.

## Ce qui reste chez l'utilisateur

Rien ne part vers les serveurs d'Escouade : les jetons restent sur le poste (trousseau du système, sinon `integrations.json`), et les appels vont de l'app à l'hébergeur, par le proxy des réglages réseau. La page Confidentialité du site le dit, avec ce que voient l'hébergeur et le service du harness (une relecture et les textes proposés lui donnent le diff), et elle liste chaque événement : un test du site la compare à `src-tauri/src/telemetry.rs`.

Le site dit aussi ce qui a été essayé (page « Les hébergeurs de code ») : GitHub en lecture sur le vrai service ; ses écritures contre le faux GitHub des tests ; les autres d'après leur documentation, contre de faux serveurs. Une page ne promet pas plus que ce qui a tourné.
