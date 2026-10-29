---
title: "Hébergeurs de code"
subject: "Git & fichiers"
sources: ["docs/SPEC.md"]
tickets: ["ESC-54","ESC-55","ESC-58","ESC-70","ESC-241"]
decisions: []
compared: null
---

## Ce que c'est

Escouade reconnaît chez quel hébergeur est le dépôt distant d'un projet : GitHub (github.com, GHE.com, GitHub Enterprise Server), GitLab, Bitbucket Cloud, Gitea / Forgejo ou Azure DevOps. Elle s'y connecte par un compte, pour ouvrir et suivre les pull requests et leur CI (voir *Pull requests*, et *Validation* pour celles d'un ticket). Chaque hébergeur déclare ce qu'il sait faire (brouillon, auto-merge, checks, journaux de jobs, relectures, fils, déploiements, stratégies de merge) ; l'interface ne propose que cela. Une PR s'appelle « MR » sur GitLab, « PR » ailleurs.

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
- Bitbucket Cloud : l'e-mail du compte Atlassian et un jeton d'API à scopes. Azure DevOps : un jeton d'accès personnel, droits demandés `vso.code_write`, `vso.build_execute`, `vso.work_write`, créé sur `<organisation>/_usersSettings/tokens`.
- L'adresse est en https (http seulement vers cette machine). API : api.github.com, `api.<nom>.ghe.com`, `/api/v3` d'un GitHub Enterprise Server ; `/api/v4` de GitLab ; api.bitbucket.org/2.0 ; `/api/v1` de Gitea ; dev.azure.com. `ESCOUADE_GITHUB_API`, `ESCOUADE_BITBUCKET_API` et `ESCOUADE_AZURE_DEVOPS_API` remplacent l'adresse publique.
- La vérification d'un compte lit qui il est : sur Azure DevOps, l'e-mail du compte de l'organisation, sinon son nom (un utilisateur anonyme est un refus).

### Ce qu'Escouade fait sur GitHub

- **Ouvrir une PR** : en brouillon si demandé ; `Closes #n` est ajouté à la description s'il manque. Relecteurs et étiquettes sont demandés ensuite : si GitHub les refuse, la PR reste ouverte.
- **Lire les PR** : état, approbations, changements demandés, fils ouverts, conflit, auto-merge et checks de la dernière révision ; toutes les PR demandées en une seule requête. Aussi les PR d'une branche.
- **Merger** (merge, squash, rebase) et **activer l'auto-merge** ; quand le dépôt ne l'autorise pas, l'erreur de GitHub est rendue telle quelle.
- **Checks** : check runs et statuts ; le journal d'un job de GitHub Actions ; « relancer ce qui a échoué » relance une exécution entière, pas un job.
- **Relectures** (le dernier mot de chacun), **commentaires** (fils de lignes avec leur état résolu, et la conversation), commenter, répondre, publier une relecture, et le dernier statut de chaque déploiement.
- **Économie du quota** : les lectures sont conditionnelles (une réponse inchangée ne compte pas dans la limite) ; les listes suivent les pages, 10 au plus. Sous 10 % du quota restant, Escouade étale ce qui reste de la fenêtre sur les appels restants ; à zéro, elle attend la remise à zéro, jamais plus de 30 s d'un coup. Les requêtes passent par le proxy des réglages (voir *Réglages*).

### Ce qu'Escouade fait sur Azure DevOps

- **Ouvrir une PR** : brouillon, description coupée à 4 000 caractères, relecteurs trouvés par leur identité puis ajoutés un à un, étiquettes, et l'élément de travail d'un ticket venu d'Azure Boards fermé par la PR. Un relecteur ou une étiquette refusés ne reprennent pas la PR.
- **Lire les PR**, une par une (une PR disparue est laissée de côté), et celles d'une branche, la plus récente d'abord : état (active ; terminée = mergée ; abandonnée = fermée), brouillon, conflits, votes des personnes (pas des équipes : 10 et 5 approuvent, -5 et -10 demandent des changements), auto-complete, fork, fils ouverts sur des lignes.
- **Merger** et **auto-merge** (l'auto-complete, posé au nom du jeton) avec la stratégie demandée (merge sans fast-forward, squash, rebase, rebase puis merge). Une stratégie que la politique de la branche cible n'autorise pas est refusée avant tout envoi.
- **Checks** : les évaluations des politiques de build et de statut (celles des relecteurs, commentaires et éléments de travail sont la relecture), le dernier statut de la PR par contexte, puis les builds de la PR qu'aucune politique ne tient (le plus récent par pipeline). Journal : celui des tâches en échec du build (sinon de la dernière tâche). « Relancer ce qui a échoué » relance chaque build en échec.
- **Relectures** : les votes. **Commentaires** : ceux écrits dans les fils (pas les messages d'Azure : votes, pushes) ; un fil est résolu quand il est corrigé, fermé, « won't fix » ou « by design ». Commenter, répondre, résoudre un fil. Publier une relecture : un fil par commentaire de ligne, le texte, puis le vote du jeton (10, -5, aucun pour un simple commentaire).
- **Refus** : un 401, un 203 ou une page de connexion sont un jeton refusé. Une limite (429, ou 503 avec `Retry-After`) est attendue une fois, 30 s au plus, puis la requête renvoyée ; les en-têtes de limite ralentissent les appels comme sur GitHub.

## Réglages

Réglages › Intégrations › « Hébergeurs de code » :

- Chaque compte : hébergeur, hôte, état de sa dernière vérification (« Connecté » ; « Jeton expiré ou révoqué » sur un 401 ; « Droits manquants : … » quand l'hébergeur dit les droits du jeton ; « Injoignable »), « Jeton de gh », « Sert aussi à GitHub Issues » (ou à GitLab Issues ; sinon « Pas de tickets : PR et CI seulement »), les projets dont le dépôt est sur l'instance, « Déconnecter ». Les comptes sont vérifiés à l'ouverture de l'onglet et par « Vérifier ».
- Les hôtes des dépôts des projets sans compte : « Aucun compte, pour le projet … » et « Connecter <hôte> », qui pré-remplit l'adresse (celle du dépôt en https, port compris ; celle de l'organisation sur Azure DevOps) et devine le type d'instance (GitLab, Gitea, Forgejo) en interrogeant sa version.
- « Connecter un hébergeur… » : hébergeur, adresse (son instance publique si vide), e-mail (Bitbucket), jeton (vide : celui de la CLI), les droits à lui donner, « Créer un jeton ».

## Limites

- GitLab, Bitbucket Cloud et Gitea / Forgejo : Escouade ne fait que vérifier le compte. Elle n'y ouvre ni ne lit aucune PR, et « Ouvrir une PR » n'y est pas proposé.
- Les droits d'un jeton ne sont lus que pour un jeton GitHub classique et sur GitLab ; un jeton GitHub à droits fins, Bitbucket, Gitea et Azure DevOps ne les disent pas.
- GitHub : les écritures (créer, merger, auto-merge, commenter) n'ont pas encore été essayées pour de vrai (voir `docs/TODO-1.8.md`).
- Azure DevOps : pas de déploiements ; le jeton de `az` ne vaut que pour dev.azure.com ; un essai réel avec un jeton reste à faire (ouvrir une PR brouillon, la lire, commenter, répondre, résoudre, l'abandonner).
