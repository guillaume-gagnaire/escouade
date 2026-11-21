# La 2.3.0-beta.1 : notes, retour à la 2.2 et essais

Ticket ESC-586. Préparée sur `release/2.3`, **rien n'est publié** : l'étiquette `v2.3.0-beta.1`, le Release et les annonces sont le geste du chef, sur le feu vert de l'utilisateur. Ce document sert au chef à relire les notes avant l'étiquette, et à l'utilisateur à savoir quoi essayer.

## Ce que la bêta change dans la publication

- **Le serveur d'équipe est publié comme `escouade-server`**, par le même job de `release.yml` (« Servers (Linux x64/arm64) ») et les mêmes scripts (`server-package.sh`, `server-release.sh`) : archives `escouade-team-server-<version>-linux-{x64,arm64}.tar.gz` signées (`.sig`, clé des mises à jour) avec leur SHA-256, image `ghcr.io/guillaume-gagnaire/escouade/escouade-team-server`. Avant tout envoi, le binaire est essayé dans un conteneur Debian nu (`scripts/team-server-trial.sh`). L'image prend le numéro de la version, et `beta` pour une bêta ; `latest` seulement pour une version stable (`scripts/image-tags.mjs`, testé). Sa version est celle de l'étiquette (`scripts/version.mjs`).
- **La CI de `release/2.3` essaie aussi le serveur d'équipe** (job `server-linux`) : le conteneur nu, puis l'image faite de son archive et essayée comme un utilisateur la lance (`scripts/team-server-image-test.sh`).
- **Une bêta se dit bêta partout** : le titre du Release (« Escouade v2.3.0-beta.1 (beta) », un pré-release) et ses notes, qui commencent par un encadré (`scripts/beta-notes.mjs`) ; les deux salons Discord, qui l'annoncent désormais avec « (bêta) » / « (beta) » après sa version et le même encadré ; la fenêtre « Nouveautés » et « Escouade X est prête » de l'app, qui montrent l'encadré et un bouton vers la page des versions ; la doc du site (Mises à jour › Versions bêta, Revenir à la version stable).
- **Le déploiement du serveur d'équipe** est dans la doc du site (fr et en, `docs/team/install`) : l'archive, l'image, le compose avec Caddy, le compose de Coolify (`crates/escouade-team-server/deploy/coolify.yml`), systemd.

## Revenir à la 2.2

Quelqu'un qui essaie la bêta décoche « Recevoir les versions bêta » (il reste alors sur la bêta jusqu'à la 2.3.0), ou réinstalle la dernière 2.2 par-dessus, **après avoir décoché** (sinon la 2.2 lui repropose la bêta aussitôt).

### Ce que la 2.3 écrit et que la 2.2 ne connaît pas

| Fichier | Ce que la 2.3 y ajoute | Ce qu'en fait la 2.2 |
|---|---|---|
| `state.json` | projets : `team`, `teamProject`, `memoryReviewOff` ; tickets : `team` (prise, détenteur, conflits, passation), `teamLocal`, `target`, `otherLine`, `formerKeys`, `scheduled` ; `scheduled` (tâches programmées), `onboarding`, `pendingReviews` (« Ta relecture »), `prViewed` ; remarques de relecture : `severity`, `suggestion`, `publishedAt`, **le choix `published`** | ignore les champs ; **refusait `published`** (corrigé, voir plus bas) |
| `settings.json` | `overseer`, `languageServers`, `languageServersTrusted`, `extensionsAllowed`, `features`, `featuresUsed` | ignore |
| `.escouade.yml` | `team.server`, `team.id`, `team.project`, `team.name`, `memory.review`, `editor.extensions_allowed`, le groupe des serveurs de langage | lit le reste ; chaque clé inconnue est un avertissement du fichier, jamais une erreur |
| `.escouade/tickets/*.yml` | rien de neuf ; un projet passé sur un serveur d'équipe met en tête de chaque fichier le commentaire « Frozen » et ne les écrit plus | lit le fichier (le commentaire est un commentaire) ; voit les tickets dans leur dernier état partagé par fichier |
| `.escouade/spec`, `.escouade/decisions` | une décision « acceptée sur le serveur d'équipe » sans numéro encore | lit les trois sortes (proposée, numérotée, acceptée sans numéro) |
| `.escouade/memory` | rien de neuf dans le format (la revue des souvenirs d'équipe vit sur le serveur) | lit |
| les fichiers des plugins (`plugins.json`, `plugins/`) | rien de neuf dans le format | lit |
| la boîte « À toi » (`asks` de `state.json`, `asks/`) | rien de neuf dans le format ; `inboxSnoozes` | lit |
| les équipes, leur journal, les espaces de serveurs, le reste | des fichiers nouveaux du dossier de données : `teams/` (les équipes et leurs journaux chiffrés), `team-costs.json`, `servers.json` (les espaces des serveurs), `overseer/`, `extensions/` (Open VSX) | ne les lit pas, et ne touche à aucun : un projet d'équipe redevient un projet de la machine, ses tickets ceux de son dernier état sur le poste ; revenu à la 2.3, tout est là |

### Le test

`sh scripts/compat-2.2.sh` (ESC-586) : le test `compat22_tests` de la 2.3 écrit, par le code de la 2.3, un dossier de données et un dépôt avec tout ce qui précède ; puis les sources de `origin/main` (`git archive`, jamais un worktree de cette branche) reçoivent `tests/compat-2.2/read_on_main.rs` comme un de leurs modules, et son test relit tout avec le code de la 2.2 : `recover_all` ne met rien de côté, chaque fichier se lit, et les projets, agents, tickets, remarques, demandes, réglages, clés de `.escouade.yml`, ticket partagé (normal et gelé), page de spec, décisions et souvenir écrits par la 2.3 y sont.

**Ce qui casse aujourd'hui sur `origin/main`** : un `state.json` qui contient une remarque publiée seule (`"choice": "published"`, PR-5 de la 2.3). La 2.2 ne lit plus l'état, `recover_all` le met de côté, les copies sont aussi de la 2.3, et l'app repart **vide**. Le correctif est sur la branche `fix/esc-586-2-2-lit-la-2-3`, partie de `origin/main` (un commit, `NoteChoice` lu avec indulgence : `published` → `dropped`, une valeur inconnue → `open`, et son test), à fusionner sur `main` et à publier en 2.2.x **avant** l'étiquette de la bêta : sinon, revenir à la 2.2.5 perd l'état de quiconque a publié une remarque seule.

Résultats du 2026-11-21 (`release/2.3` à 8db4399b2) :

- contre `origin/main` (a54f0e2a3) : **échec**, `state.json` mis de côté au démarrage (`Recovery { file: "state.json", damage: Invalid, … }`) ;
- contre `fix/esc-586-2-2-lit-la-2-3` (4c34c8f50) : **réussi**, « the 2.2 reads what the 2.3 wrote (7 warnings) ».

**Ce que la 2.3 perd à l'aller-retour** : ce que `main` a ajouté depuis le dernier reversement et que `release/2.3` n'a pas encore (ESC-557 : `inboxMarks` de la boîte ; ESC-561 : `gitFetch` des réglages et de chaque projet, `git.auto_fetch`, `git.fetch_every`, `git.fetch_timeout` du `.escouade.yml`). Le test les dit en avertissements (`warning: state.inboxMarks`, `warning: state.projects[0].gitFetch`, `warning: settings.gitFetch`) : la 2.3 ne les relit pas et les réécrit sans eux. Reverser `main` dans `release/2.3` avant l'étiquette les fait disparaître. Les trois autres avertissements sont ceux de la 2.2 sur les groupes `memory`, `team` et `editor` du `.escouade.yml` (« clé inconnue : team, ignorée ») : la 2.2 lit le reste du fichier et le dit dans ses avertissements, sans rien perdre.

## Les notes de la bêta, par chantier

Tirées de `## [Non publié]` du CHANGELOG (ce que la 2.3 contient depuis la 2.2.5). Une ou deux phrases par entrée ; le Release reprend `CHANGELOG.en.md`, la mise à jour de l'app et le salon français `CHANGELOG.md`, précédés de l'encadré « bêta ».

### Le serveur d'équipe

- Crée une équipe sur le serveur que ton équipe héberge, avec une clé de secours de 24 mots ; invite par un lien à usage unique, ajoute tes autres appareils par QR code ou par un code court. Tout est chiffré de bout en bout.
- Les projets d'une équipe se partagent chiffrés et lisibles hors ligne ; « Projets de l'équipe » les liste, « Cloner » passe par tes propres comptes git, et « Verser à l'équipe » y met, après un aperçu, les tickets et les docs d'avant.
- Le Kanban d'un projet se partage en direct : un ticket se prend une seule fois, chaque carte dit qui le tient, et deux changements simultanés se montrent ou se fusionnent.
- La spec, les décisions, les documents et la mémoire d'un projet d'équipe vivent sur le serveur, chiffrés ; un souvenir écrit par un agent attend qu'un humain le garde.
- Passe l'agent d'un ticket à un collègue ou à la machine de l'équipe en mode serveur, sa branche poussée à ton geste et sa conversation chiffrée.
- Les coûts et les quotas de l'équipe, en totaux par jour pour ses administrateurs, avec des budgets du mois et une alerte dans « À toi ».
- Une équipe impose ou recommande des réglages par son `.escouade.yml` d'équipe signé ; rien qui touche le réseau ou lance une commande ne s'applique sans ton accord.
- Publié avec chaque version : archives signées pour Linux x64 et arm64, image `ghcr.io/…/escouade-team-server`, compose avec Caddy et recette Coolify (ESC-586).

### Le mode serveur

- `escouade-server` fait tourner Escouade sans fenêtre sur un serveur Linux (x64, arm64), son API sur 127.0.0.1, appairé par un code à usage unique.
- « Se connecter à un serveur » le pilote d'ici par un tunnel SSH ; plusieurs serveurs, chacun son espace, et la vue « Partout » répond sur chaque machine.
- Sur un serveur, ce qui doit rester sur ton poste y reste (fichiers téléchargés, serveur de test, notifications, voix, connexion d'un compte) ; les profils et réglages des plugins s'y règlent comme sur le poste.
- Prêt à déployer : image Docker sans root, Compose d'exemple, recette Coolify, `install.sh` et `escouade-server update`, signature vérifiée.

### Les plugins et l'éditeur

- Compiler un plugin en Rust ne remplit plus le disque (copies de plus de 7 jours retirées, cache sous 5 Go, `escouade-plugin cache`).
- L'éditeur d'un projet est celui de VS Code : extensions d'Open VSX vérifiées et sans code exécuté, aperçus (images, PDF, Word, Markdown), thème et langue d'Escouade, lignes changées, raccourcis qui ne se marchent plus dessus.

### L'onboarding

- Un premier lancement qui accompagne, une question à la fois : le harness trouvé ou la commande qui l'installe, son compte, le mode des agents, les statistiques ; tout se passe et se reprend.
- Ce que tu veux brancher (source de tickets, voix, hébergeur de code), rien de coché d'avance ; le premier projet (dossier, dépôt ou démonstration) et le premier agent sur un message proposé, expliqué là où il arrive.
- Aide › À découvrir : la carte de ce qu'Escouade sait faire, chaque chose menant au vrai geste.

### Les fonctionnalités

- Réglages › Fonctionnalités coupe ce que tu n'utilises pas (Kanban, Docs, éditeur, coût, statistiques, workflows, tâches programmées, Overseer, voix) : elle disparaît vraiment et ses données reviennent quand tu la rallumes.
- Les tâches programmées : un ticket d'agent qui revient à heure fixe, projet par projet.
- L'Overseer, un agent qui pilote tous tes projets (sauf les privés), avec son écran et la voix ; il ne fusionne, ne pousse et ne publie que sur ta consigne.

### Les PR

- Une PR se lit dans Escouade : description, commits, relectures, fils, contrôles ; l'onglet Fichiers et ses fils sous leur ligne ; « Ta relecture » ; le contrôle de la PR (brouillon, relecture, étiquette, merge).
- L'agent relecteur commente comme un humain, ses remarques publiables une à une sous ton compte.

### La boîte « À toi »

- Livrée dans la 2.2. La 2.3 y ajoute la branche cible propre d'un ticket (dite dans la boîte) et l'alerte de budget de l'équipe.

### Sécurité, stratégies et le reste

- Un poste géré impose des réglages par ses stratégies (registre, MDM, `/etc`), ADMX et profil MDM livrés ; les stratégies jugent le mode où le harness fait vraiment tourner l'agent.
- Un ticket peut viser sa propre branche cible ; Réglages › Avancé › « Copies des données » ; projets privés plus sûrs ; sept constats faibles de l'audit de la 2.2.0 corrigés.

### L'app mobile (en cours, pas livrée)

- La direction B est validée et ses fondations avancent (la crate `escouade-mobile` et son plugin Capacitor, construits par la CI, ESC-568), mais il n'y a pas encore d'app à installer : la bêta n'en parle pas comme d'une fonctionnalité.

## Ce qui attend l'utilisateur pour essayer

Rien n'est installé ni déployé par le ticket. Dans l'ordre :

1. **Donner le feu vert** : au correctif 2.2.x (branche `fix/esc-586-2-2-lit-la-2-3`, à fusionner et publier d'abord), au reversement de `main` dans `release/2.3`, puis à l'étiquette `v2.3.0-beta.1` (le chef la pose et suit le Release).
2. **Déployer le serveur d'équipe sur le VPS d'essai** (Coolify) : une ressource « Docker Compose » avec `crates/escouade-team-server/deploy/coolify.yml`, `ESCOUADE_TEAM_TAG=beta`, un domaine ; puis `escouade-team-server init` dans son terminal, et garder le jeton.
3. **Cocher « Recevoir les versions bêta »** (Réglages › Général) sur un poste, et laisser la bêta s'installer ; sur un second poste (ou un second compte) pour l'équipe.
4. **Observer** :
   - la fenêtre « Escouade 2.3.0-beta.1 (bêta) est prête » puis « Nouveautés » : l'encadré bêta et « Page des versions » ;
   - le Release (pré-release, titre « (beta) », l'encadré en tête) et les deux salons Discord (« (bêta) », le retour à la 2.2) ;
   - créer l'équipe avec le jeton, inviter le second poste, partager un projet et son Kanban, prendre un ticket des deux côtés ;
   - `/healthz` du serveur, `docker logs` (aucun jeton ni identifiant), un redéploiement de Coolify qui garde le volume ;
   - revenir à la 2.2 sur un poste (décocher, réinstaller la 2.2.x corrigée) : les projets, tickets et réglages sont là.
5. **Retirer le VPS d'essai** une fois les essais finis (il était prévu de le démonter).
