# La place de marché des plugins : store.escouade.dev (ESC-202)

Demandé par l'utilisateur le 2026-10-26 : « Pour le système de plugins, fais en même temps le système de marketplace qui sera sur store.escouade.dev. Chacun pourra soumettre des plugins via des repos publics, avec la documentation, etc. Une gestion de compte pour gérer ses plugins partagés. Un système de review de plugins que j'analyserai en local via un agent dans le repo d'escouade, pour avoir un flag "vérifié par escouade". »

Ce document est l'analyse, l'architecture, la sécurité et le plan. Il ne change ni l'app ni le site. Il remplace la décision 3 du 2026-10-25 (« aucun registre en 2.1.0 ; un dépôt public avec revue par PR et signature en 2.2 ») : la place de marché arrive **avec les plugins de la 2.1**. Ce qui reste vrai : une licence libre exigée, pas de vente par Escouade, l'installation depuis un dépôt git toujours permise et marquée « non vérifié ».

L'aperçu : `design/store/index.html` (le store, dans le style du site) et `#store*` de `design/app-redesign/index.html` (« Parcourir le store » dans l'app), leurs captures, et les choix 160 et suivants d'`design/app-redesign/UX-NOTES.md`.

- §1 : le service (où il vit, sa pile, son hébergement et l'application Coolify, ses sauvegardes, sa surveillance, son coût, la CI) ;
- §2 : les comptes et le RGPD ;
- §3 : la soumission depuis un dépôt public ;
- §4 : la revue « Vérifié par Escouade » ;
- §5 : dans l'app ;
- §6 : les pages du store ;
- §7 : la sécurité de bout en bout ;
- §8 : les décisions à poser ;
- §9 : le découpage en tickets.

## En bref

- **Un dossier `store/` de ce dépôt** (décidé), en **Nuxt 4 rendu par le serveur** (la pile du site, avec ses routes d'API Nitro), une base **PostgreSQL**, les archives sur un **volume** sauvegardé chaque nuit vers un stockage objet de Scaleway à Paris ; déployé par **Coolify** à chaque push sur `main` qui touche le store, sur le serveur d'escouade.dev et d'Aptabase ; la CI le vérifie avant la fusion. Coût en plus : moins d'un euro par mois.
- **On se connecte avec GitHub**, par une **GitHub App** « Escouade Store » : la connexion, la preuve qu'on administre le dépôt (l'App y est installée) et les webhooks des nouvelles étiquettes, d'un seul geste, en lecture seule. GitLab plus tard.
- **Une version = une étiquette = un commit = une archive figée** : le store télécharge l'archive du commit une fois, la garde, en calcule le SHA-256. Les contrôles automatiques (schéma, `escouade-plugin validate`, licence libre, README, captures, permissions, taille) disent à l'auteur, champ par champ, ce qui manque. **Le serveur n'exécute jamais le code d'un plugin.**
- **La revue se fait sur le poste de l'utilisateur** : `npm run plugin-review -- <id>@<version>` et la commande `/review-plugin` de Claude Code dans ce dépôt téléchargent l'archive dans un dossier à part, vérifient son empreinte, lisent le code au regard des permissions, passent le banc de conformité dans un conteneur sans réseau et rendent un rapport. **L'utilisateur décide** ; sur son accord, il **signe** avec une clé minisign qui ne quitte pas son poste. L'app vérifie la signature avec la clé publique qu'elle embarque : c'est le badge **« Vérifié par Escouade »**.
- **Le badge vaut pour une version** (une empreinte), pas pour un plugin. Une nouvelle version repasse en revue, sur le diff depuis la dernière vérifiée. Une **liste de révocation signée** retire un badge ou désactive une version en urgence ; l'app la lit au démarrage et toutes les 6 heures.
- **L'app cherche dans le catalogue sur la machine** : elle télécharge le catalogue entier (quelques centaines de Ko), la recherche ne part jamais au store. Le seul compte qu'elle envoie, celui des installations, part sans identifiant et seulement si l'utilisateur a accepté les statistiques.
- **Si le store tombe**, rien d'installé ne casse : les signatures se vérifient hors ligne, le catalogue et la liste de révocation restent en cache, l'installation depuis un dépôt git marche toujours.

## 1. Le service

### 1.1 Où il vit : un dossier `store/` de ce dépôt (décidé)

**Décidé par l'utilisateur le 2026-10-26** : « Le code de la marketplace vivra dans notre dépôt, le push sur main déclenchera le build sur Coolify pour la partie marketplace. » Le store est un dossier `store/`, à côté de `website/`, avec son `Dockerfile` ; une application Coolify le redéploie à chaque push sur `main` qui touche ses fichiers, et seulement eux (§1.3).

Ce que ça apporte, et qui allait dans le sens de l'analyse :
- **le schéma du manifeste** se lit dans `schemas/plugin-1.json`, la même source que l'app et le SDK : un champ ajouté l'est partout dans le même commit, et un plugin accepté par le store n'est pas refusé par l'app ;
- **la revue locale** (`/review-plugin`, `npm run plugin-review`) vit à côté du banc (`src-tauri/src/plugins/bench/`) et du SDK qu'elle appelle ;
- **les jetons et le logo du site** se partagent par un chemin relatif ;
- ce que ça coûte : le dépôt de l'app grossit d'un service web, et la CI doit garder `main` déployable pour lui aussi (§1.5).

### 1.2 Sa pile

- **Nuxt 4 en rendu serveur** (le site est en `nuxt generate`, statique ; le store a des pages qui changent et une connexion) : les pages en Vue, les routes d'API dans `store/server/api/` (Nitro), le même `main.css` d'Inter et des jetons du site, et un thème clair (le site n'en a pas ; le store suit le système, comme l'app).
- **PostgreSQL 17** (une base de Coolify), par **Drizzle** (schéma en TypeScript, migrations versionnées dans `store/server/db/migrations/`). Tables : `accounts`, `sessions`, `plugins`, `owners`, `versions` (id, numéro, commit, SHA-256, taille, manifeste figé, état), `checks` (le résultat de chaque contrôle), `signatures`, `reviews` (file et résumé public), `reports` (signalements), `revocations`, `installs_daily` (compteurs agrégés), `audit` (le journal des actions).
- **Les tâches longues** (lire un dépôt, télécharger une archive, la contrôler) passent par une file dans PostgreSQL (`pg-boss`) : pas de Redis à héberger. Un seul processus Node sert les pages et les tâches au départ ; le travailleur se sépare quand il le faut, la même image avec une autre commande.
- **Les archives** : un volume Docker monté (`/data/archives/<id>/<version>.tar.gz`), servi par Nitro avec un cache long (une archive ne change jamais). Raison de ne pas commencer par un stockage objet : 200 plugins × 10 versions × 2 Mo font 4 Go, que le disque du serveur tient ; la sauvegarde va au stockage objet (§1.4).
- **Le catalogue de l'app** : `GET /api/v1/catalog.json`, un JSON compressé régénéré à chaque changement, avec un `ETag` ; `GET /api/v1/revocations.json` et sa signature `revocations.json.minisig` (§4.6).
- **Le texte d'un README** se rend en HTML **sur le serveur**, assaini (`rehype-sanitize`, aucun script, aucun `style`, des liens `rel="nofollow ugc noopener"`), ses images copiées sur le store à la soumission (pas de pixel de suivi chez un tiers, ni sur le store ni dans l'app).

### 1.3 Son hébergement : Coolify (recommandé), et l'application à créer

Sur le Coolify de l'utilisateur, dans le projet « Escouade », à côté du site et d'Aptabase. Le serveur est le serveur dédié de Scaleway à Paris : aucune donnée hors de l'UE, sauf ce que GitHub sait déjà (§2.4). Le chef d'escouade ou l'utilisateur crée l'application d'après ce qui suit, une fois le plan validé ; **aucun secret ne va dans le dépôt**.

**L'application « store »** (projet « Escouade », environnement `production`) :

| Réglage | Valeur |
| --- | --- |
| Source | l'App GitHub « guillaume-gagnaire » de Coolify (celle du site), ce dépôt, branche `main` |
| Construction | Dockerfile, `store/Dockerfile`, contexte : la racine du dépôt (comme `website/Dockerfile` ; ce que le build voit est dans `store/Dockerfile.dockerignore`) |
| Déploiement automatique | oui, sur push ; **chemins surveillés** : `store/**`, `schemas/**`, `website/app/assets/main.css` (les jetons partagés), et `sdk/**` si le validateur du SDK (PLG-9) vit dans ce dépôt plutôt qu'en paquet npm épinglé |
| Domaine | `https://store.escouade.dev` (un enregistrement A ou CNAME vers le serveur chez Gandi ; le certificat par Traefik) |
| Port exposé | 3000 (Nitro) |
| Vérification de santé | `GET /api/health`, toutes les 30 s ; un nouveau conteneur qui ne répond pas n'est pas mis en ligne, l'ancien reste |
| Stockage persistant | un volume monté sur `/data/archives` |
| Sauvegarde du volume | le volume `/data/archives` chaque nuit à 3 h 30 vers le bucket des sauvegardes, par Coolify (§1.4 ; décidé par le chef d'escouade le 2026-10-29, à la place d'une tâche `restic`) |
| Notifications | échec de déploiement, conteneur arrêté, sauvegarde ratée : le salon Discord d'Escouade |

**Ses variables** (dans Coolify ; les secrets ne se relisent pas après saisie et ne servent pas au build) :

| Variable | Contenu | Secret |
| --- | --- | --- |
| `DATABASE_URL` | l'adresse interne de la base `store-db` (donnée par Coolify) | oui |
| `NUXT_SESSION_SECRET` | 32 octets aléatoires | oui |
| `GITHUB_APP_ID`, `GITHUB_APP_CLIENT_ID` | l'App « Escouade Store » | non |
| `GITHUB_APP_CLIENT_SECRET`, `GITHUB_APP_PRIVATE_KEY` | idem (la clé PEM telle quelle, sur plusieurs lignes, ou sur une ligne aux retours échappés `
` : les deux se lisent) | oui |
| `GITHUB_WEBHOOK_SECRET` | le secret des webhooks de l'App | oui |
| `STORE_ADMINS` | l'identifiant numérique GitHub de l'utilisateur (la file de revue) | non |
| `SMTP_URL`, `MAIL_FROM` | l'envoi des avis : Scaleway Transactional Email (Paris), `store@escouade.dev` | oui (`SMTP_URL`) |
| `ARCHIVE_DIR` | `/data/archives` | non |
| `NUXT_PUBLIC_SITE_URL` | `https://store.escouade.dev` | non |

Les **clés publiques** de signature des plugins sont publiques : elles sont dans le dépôt (`store/keys/*.pub`, les mêmes que celles que l'app embarque). **La clé privée de signature n'est jamais ni dans Coolify ni dans le dépôt** (§4.5).

**La base** : une ressource PostgreSQL 17 de Coolify, `store-db`, dans le même projet, non exposée ; sauvegarde planifiée chaque nuit à 3 h vers une destination S3 de Coolify (stockage objet de Scaleway, `fr-par`, bucket `escouade-store-backups`), 30 jours gardés.

**Les migrations** (Drizzle) s'appliquent au démarrage du conteneur, avant que la vérification de santé réponde. Un nouveau conteneur peut échouer après elles et laisser l'ancien en ligne : une migration ne fait donc qu'**ajouter** (colonnes, tables, index) ; on ne retire ou ne renomme qu'au déploiement suivant, quand plus aucun code ne lit l'ancien nom.

Écarté : un hébergeur géré (Vercel, Fly.io) — un coût, un sous-traitant de plus, souvent hors UE, pour un service qui tient sur le serveur existant ; Cloudflare devant le store — utile si le trafic des téléchargements grossit, à reprendre alors (une archive ne change jamais : elle se met en cache sans risque, et l'empreinte est vérifiée par l'app quel que soit le chemin).

### 1.4 Ses sauvegardes, sa surveillance, son coût

- **La base** : la sauvegarde planifiée de Coolify (`pg_dump` chaque nuit) vers un bucket du **stockage objet de Scaleway (fr-par)**, 30 jours gardés ; une restauration essayée à la mise en route, puis une fois par trimestre (notée dans `docs/STORE.md`).
  - Restaurations essayées : **2026-10-29**, en local (ESC-228 : PostgreSQL 16 dans WSL, `store/scripts/restore-try.sh` sur une base vide, le dump au format de Coolify et l'archive du volume, archives identiques à l'octet, le store reparti sur la base restaurée ; détail dans `store/README.md`) ; à refaire sur les vraies sauvegardes de Coolify à la mise en route.
- **Les archives** : la sauvegarde du volume par Coolify chaque nuit à 3 h 30 vers le même bucket, 30 jours gardés ; elles sont aussi reconstructibles depuis les dépôts tant qu'ils existent, mais pas à l'octet près : la sauvegarde fait foi (§3.4).
- **Ce qui ne se sauvegarde pas parce qu'il n'est pas là** : la clé de signature (sur le poste de l'utilisateur, §4.5).
- **La surveillance** : une route `GET /api/health` (base joignable, file qui avance, disque sous 80 %) vérifiée par Coolify (redémarrage) et par une sonde **extérieure** au serveur (un serveur qui tombe ne se signale pas lui-même) : Uptime Kuma sur une autre machine de l'utilisateur, ou le palier gratuit d'un service de sondes, qui prévient sur le salon Discord d'Escouade. Les échecs de déploiement et les conteneurs arrêtés par les notifications de Coolify (même salon). Les erreurs du serveur dans le journal du conteneur ; pas de service d'erreurs tiers au départ.
- **Le coût** : le serveur, le domaine et Coolify existent ; la GitHub App est gratuite. En plus : le stockage objet (quelques Go à environ 0,015 € le Go par mois, moins d'un euro), l'envoi d'e-mails (le palier gratuit de Scaleway suffit à quelques centaines d'avis par mois) et l'éventuelle sonde extérieure (gratuite). Le vrai coût est le temps de revue de l'utilisateur (§4.3).

### 1.5 Ce que la CI vérifie avant la fusion

Le push sur `main` déploie : ce qui entre dans `main` doit déjà avoir passé. Deux barrières :

1. **Un job `store` dans `.github/workflows/ci.yml`**, lancé sur chaque pull request et chaque push sur `main` qui touche les chemins surveillés (un filtre de chemins, `dorny/paths-filter`, pour qu'il ne tourne pas quand seule l'app change ; les autres jobs ne changent pas), sur Ubuntu :
   - `npm ci` dans `store/` ;
   - Prettier sur `store/` et `nuxi typecheck` ;
   - **Vitest** : les contrôles d'une soumission (§3.3) sur des fixtures de plugins valides et piégés, l'empreinte de `version.json` comparée à une valeur figée, l'assainissement du README, la vérification d'une signature minisign avec une clé de test ;
   - **les migrations** jouées depuis une base vide sur un service PostgreSQL 17 du job, puis les tests de l'API contre elle (connexion simulée, faux GitHub : un serveur local qui rejoue les réponses de l'API et signe des webhooks) ;
   - **Playwright** sur le serveur construit (`nuxi build` puis `node .output/server/index.mjs`) : les pages publiques, la soumission, la file de revue, dans les deux thèmes, comparées à l'aperçu `design/store/` ;
   - **`docker build -f store/Dockerfile .`** : l'image se construit (sans être poussée).
2. **Le Dockerfile rejoue les tests** au build (comme `website/Dockerfile` : `npm ci && npm run build && npm test`) : un test raté fait échouer le déploiement, et la version en ligne reste la dernière qui passait.

La protection de `main` exige le job `store` comme les autres vérifications ; un job qui saute faute de changement compte comme passé.

## 2. Les comptes

### 2.1 La connexion : une GitHub App (recommandé), GitLab plus tard

Les soumissions viennent de dépôts publics, et la plupart sont sur GitHub. Trois façons de s'y connecter :

| | OAuth App, scope vide | OAuth App, scope `public_repo` | **GitHub App « Escouade Store »** |
| --- | --- | --- | --- |
| Connexion | oui | oui | oui (« user-to-server ») |
| Preuve qu'on administre le dépôt | non (seulement s'il est à son nom) | oui, mais par un droit d'**écriture** sur tous ses dépôts publics | oui : l'App ne s'installe que par un administrateur du dépôt, sur les seuls dépôts choisis |
| Webhooks des nouvelles étiquettes | non | à poser à la main par dépôt | oui, d'office (`release`, `create`, `repository`) |
| Ce que le store peut faire | rien | écrire dans ses dépôts | **lire** (`Metadata`, `Contents` en lecture seule) |

**Recommandation : la GitHub App.** Raison : la moindre permission (lecture seule, sur les dépôts choisis), la preuve de propriété et les mises à jour automatiques en un seul geste de l'auteur ; un dépôt d'organisation marche de la même façon (un administrateur de l'organisation installe l'App). Le compte sur le store est l'identité GitHub (`id` numérique, pas le login, qui peut changer).

**GitLab** : pas au départ. Raison : peu de plugins viendront d'ailleurs que de GitHub au lancement, et chaque hébergeur a sa preuve de propriété et ses webhooks à écrire. Le modèle le prévoit (`accounts.provider`, `plugins.repository` qui garde l'hôte) ; un ticket à part l'ajoutera (gitlab.com puis une instance, avec un jeton de projet en lecture), quand un auteur le demandera. Un dépôt hors GitHub reste installable dans l'app par son URL git, « non vérifié ».

### 2.2 Ce qu'un compte gère

- **Ses plugins** : la liste, chacun avec son état (publié, en revue, vérifié jusqu'à quelle version, retiré de la liste, rejeté), ses installations, ses signalements ouverts.
- **Les versions d'un plugin** : publiées, en contrôle, rejetées (avec la raison, §3.5), vérifiées ; **retirer une version** (« yank ») : elle n'est plus proposée à l'installation ni comme mise à jour ; ceux qui l'ont la gardent, avec une bande « retirée par l'auteur » dans l'app. Une version publiée ne se modifie jamais ; son numéro ne se réutilise pas.
- **Ses statistiques** : les installations par jour et par version, et les mises à jour, agrégées sans identifiant, venues des seules installations dont l'utilisateur a accepté les statistiques (§5.4). Pas de pays, pas de système, pas de version d'Escouade au départ : rien qui ne serve à l'auteur pour décider.
- **Transférer** : à un autre compte du store, qui l'accepte ; ou en suivant le dépôt quand il change de propriétaire sur GitHub (le webhook `repository.transferred`, puis le nouveau propriétaire installe l'App). Les signatures restent valables (elles portent sur l'archive, pas sur l'auteur) ; la page dit le transfert et sa date. Un transfert gèle les publications 7 jours, le temps d'un signalement (§7).
- **Retirer de la liste** : le plugin n'apparaît plus dans la recherche ni dans le catalogue ; son adresse reste lisible (« retiré de la liste par son auteur ») ; les installations gardent leurs versions et leurs badges, sans mises à jour.
- **Supprimer** : la fiche et les archives partent après 30 jours (le temps de se raviser, et pour les sauvegardes) ; les installations gardent ce qu'elles ont, marqué « retiré du store » ; l'identifiant du plugin reste réservé à son compte (personne ne reprend le nom pour publier autre chose sous la même identité).
- **Son compte** : l'adresse e-mail facultative des avis (rejet, revue faite, signalement), l'export de ses données (JSON), la suppression du compte (ses plugins sont d'abord transférés ou supprimés).

Plusieurs mainteneurs : ceux qui administrent le dépôt sur GitHub (lu par l'App) gèrent le plugin ; pas de gestion d'équipe à part sur le store au départ.

**Fait le 2026-10-29** (ESC-232, STORE-5) : `/account` et `/en/account` comme l'aperçu `#account` (`server/utils/author.ts`, migration `0005_author_space`, après celle de STORE-4) : le bandeau d'une version en attente (« v1.3.0 trouvée sur GitHub, contrôles passés », les deux commits, les permissions en plus), « Publier » ou « Ignorer » ; la table des plugins et de leurs états, la raison d'un premier refus sous sa ligne ; le menu « … » qui ouvre la gestion d'un plugin (ses versions et leurs installations, retirer une version, les nouvelles étiquettes publiées seules ou non, retirer de la liste, transférer, supprimer) et, à droite, ses installations des 12 dernières semaines ; un avis par e-mail, en français et en anglais, à chaque action, à chaque version en attente ou publiée seule. Ce qui a été tranché en le construisant :
- **une version retirée** reste dans `catalog.json`, marquée `yanked`, ses fichiers toujours servis (ceux qui l'ont la gardent) ; elle n'est plus `latest`, qui devient `null` si toutes le sont ; elle ne revient pas ;
- **retiré de la liste ou supprimé** : le plugin sort du catalogue, de l'accueil, de la recherche et de la page de son auteur ; sa page reste lisible et le dit (« Retiré de la liste par son auteur », ou « Retiré du store par son auteur » avec la date où elle part) ; ses fichiers restent servis jusqu'à la purge ; une version retirée y est marquée « Retirée par l'auteur », et un plugin dont toutes les versions le sont quitte les listes ;
- **la suppression** demande l'identifiant tapé ; elle met de côté les versions en attente et refuse les nouvelles étiquettes ; « Annuler la suppression » pendant 30 jours ; puis la purge de chaque nuit efface ses versions et les fichiers qu'aucune autre version ne garde, et garde la ligne du plugin (son id au compte) et ses numéros (`retired_versions`), que les contrôles tiennent pour pris. Les signatures de STORE-8 devront survivre à cette purge (§2.3) ;
- **le transfert** se propose par le login d'un compte du store, qui l'accepte ou le refuse depuis le sien (l'offrant peut l'annuler d'ici là) ; **le dépôt transféré sur GitHub** (`repository.transferred`) : le plugin suit le nouveau nom, et un compte dont une installation de l'App atteint le dépôt peut le « Reprendre » ; dans les deux cas, gel de 7 jours (`frozen_until`) : la publication refusée, une étiquette attend un clic même avec la publication seule, `catalog.json` porte `transferred_at` et la page du plugin dit « Transféré par … le … » ;
- **le compte** ne se supprime plus tant qu'il a des plugins non supprimés ;
- **les statistiques** : installations par version et par semaine, mises à jour à part, depuis `installs_daily` ; aucune autre donnée.

### 2.3 Les données gardées

| Donnée | Pourquoi | Combien de temps |
| --- | --- | --- |
| Identifiant GitHub, login, nom affiché, adresse de l'avatar | le compte, la page de l'auteur | jusqu'à la suppression du compte, puis 30 jours dans les sauvegardes |
| E-mail (facultatif, saisi par l'auteur, vérifié par un lien) | les avis | idem ; effacé dès qu'il est retiré |
| Sessions (un jeton aléatoire, sa date) | rester connecté | 30 jours |
| Jeton d'accès GitHub de l'utilisateur | lire ses installations de l'App pendant la session | chiffré en base, effacé à la déconnexion, jamais plus de 8 h (durée de GitHub) |
| Plugins, versions, contrôles, signatures, résumés de revue | le service | tant que le plugin existe ; signatures et révocations gardées toujours (une signature publiée doit rester vérifiable, une révocation lisible) |
| Signalements (texte, compte qui signale) | la modération | 1 an après leur clôture |
| Journal des actions (qui a publié, transféré, retiré, signé) | la sécurité, les litiges | 1 an |
| Compteurs d'installations (plugin, version, jour, nombre) | les statistiques de l'auteur | sans limite : aucune donnée personnelle |
| Adresse IP, adresse demandée, navigateur | journaux du proxy Traefik (traitement 2 du registre) | 1 an au plus, déjà en place |

Un seul cookie, celui de la session, strictement nécessaire : exempté de consentement, pas de bandeau. Aucune mesure d'audience.

**Fait le 2026-10-29** (ESC-229, STORE-2) : la connexion par la GitHub App (l'autorisation demandée à l'installation ; un retour sans l'état que le navigateur a emporté relance la connexion au lieu de se fier au code), la session de 30 jours (`__Host-store_session`, la base ne garde que l'empreinte du jeton), le jeton GitHub chiffré, 8 h au plus, révoqué chez GitHub à la déconnexion et à la suppression, les installations et leurs dépôts suivis par les webhooks signés (rejoués une fois par livraison), l'e-mail vérifié par un lien de 24 h (5 liens par heure au plus ; sans `SMTP_URL`, la page dit que le lien ne part pas), l'export JSON, la suppression du compte, le journal des actions, la purge de chaque nuit ; `/account`, `/terms` (version 1), la section « Le store » de `/privacy/` et le traitement 3 du registre. Les en-têtes de sécurité de tout le site (HSTS, CSP à nonce, `nosniff`, `Referrer-Policy`, `Permissions-Policy` fermée, `X-Frame-Options`), demandés par le chef d'escouade le même jour. Les avatars sont les initiales de l'auteur, dessinées par le store, comme dans l'aperçu : aucune image chargée chez GitHub.

### 2.4 Le RGPD, la politique de confidentialité et les conditions des auteurs

- **Le registre** (`docs/registre-rgpd.md`) gagne le traitement 3, « Comptes des auteurs du store », et le traitement 4, « Compteurs d'installations des plugins » (fait dans ce ticket, en projet jusqu'à la mise en ligne). Base légale des comptes : l'exécution des conditions acceptées (art. 6.1.b) ; des signalements et du journal : l'intérêt légitime (art. 6.1.f) ; des compteurs : aucune donnée personnelle, et l'accord aux statistiques donné dans l'app.
- **GitHub** (GitHub, Inc., États-Unis) est le fournisseur d'identité : le store ne lui envoie rien qu'il ne sache déjà (le visiteur qui se connecte s'y connecte lui-même). Le transfert de l'identifiant vers le store se fait depuis les États-Unis vers l'UE ; la politique le dit, avec le cadre de protection des données UE–États-Unis dont GitHub se prévaut.
- **La politique de confidentialité** : une section « Le store » ajoutée à `/privacy/` d'escouade.dev (une seule politique pour le site, l'app et le store, comme aujourd'hui), avec le lien depuis le pied du store : ce qui est gardé (§2.3), pourquoi, combien de temps, les droits (accès et export dans le compte, rectification par GitHub, effacement par la suppression du compte, opposition par e-mail), le contact.
- **Les conditions des auteurs** (`/terms/` du store, acceptées à la première soumission, versionnées) : l'auteur a les droits sur ce qu'il publie ; une **licence libre** reconnue par l'OSI ou la FSF (identifiant SPDX) ; pas de vente, pas de paiement dans le plugin par Escouade ; interdits (code malveillant, collecte de données non déclarée dans ses permissions, minage, contournement des permissions, contenu illicite, usurpation d'un nom ou d'une marque) ; Escouade peut refuser, retirer de la liste ou révoquer à tout moment, en disant pourquoi sauf danger ; **« Vérifié par Escouade » veut dire qu'une version a été relue, pas qu'elle est garantie** ; pas de responsabilité d'Escouade sur un plugin tiers ; la loi française.
- **Les utilisateurs de l'app** n'ont pas de compte sur le store : ils ne donnent rien (§5.4).

## 3. La soumission depuis un dépôt public

### 3.1 Le parcours de l'auteur

1. **Se connecter** avec GitHub, accepter les conditions.
2. **Choisir le dépôt** : la liste des dépôts publics où l'App est installée, ou « Installer l'App sur un dépôt » (la page de GitHub, puis retour). Un dépôt privé est refusé (le code d'un plugin du store se lit par tous).
3. **Choisir la version** : les étiquettes et les releases du dépôt, la plus récente d'abord. Le store résout l'étiquette en commit (`git/ref/tags/<tag>`, l'objet annoté déroulé) et l'affiche : « v1.2.0 → 3f9c2e1 ».
4. **Les contrôles** (§3.3) tournent, en direct sur la page, une ligne par contrôle.
5. **Publier** : le plugin paraît, **non vérifié**, et entre dans la file de revue s'il le demande (case cochée d'office). Une première version demande aussi la catégorie et des mots-clés.

### 3.2 Ce que le store lit

- **Le manifeste** `escouade-plugin.json` (PLG-1, ESC-173), validé par `schemas/plugin-1.json`. Le manifeste gagne une clé facultative `store` : `{ "categories": ["notifications"], "keywords": [...], "screenshots": ["store/1.png", ...], "docs": "docs/" }`.
- **Le README** (`README.md`, ou `README.fr.md` et `README.en.md` pour les deux langues ; la page montre celui de la langue du visiteur, sinon l'autre) : obligatoire, rendu sur la page du plugin et dans l'app.
- **La documentation** : le dossier `store.docs` s'il est déclaré (des pages markdown, rendues comme le README, sur des onglets de la page) ; sinon le README seul.
- **La licence** : l'identifiant SPDX du manifeste, comparé au fichier `LICENSE` (ou `COPYING`) du commit, lu par la détection de licence de GitHub (`GET /repos/{o}/{r}/license?ref=<sha>`) ; refusée si absente, différente ou non libre.
- **L'icône** (`icon` du manifeste, PLG-13, ESC-292) : facultative, un PNG carré du dépôt, 256 px de côté au moins (512 conseillé), 1 Mo au plus ; ni SVG, ni image distante. Le store la décode, bornée à 4096 px de côté, et la **réencode sans métadonnées en 256 et 64 px**, gardées sous leur SHA-256 et servies par `/files/<sha256>.png` (`immutable`), comme les captures. Sans elle, la tuile garde ses deux lettres sur sa teinte.
- **Les captures** : jusqu'à 6 images PNG, JPEG ou WebP de `store.screenshots`, 1600 px de large au plus, 1 Mo chacune, réencodées par le store (les métadonnées retirées).
- **Les permissions déclarées** (`permissions`, §3.4 de `docs/PLUGINS.md`), montrées en mots, la plus forte d'abord, sur la page du plugin et dans l'app.
- **Les binaires** d'un plugin à programme compilé (`run` vers un exécutable) : les fichiers de la release attachés à l'étiquette (`escouade-plugin pack` en fait une archive par système), avec leur **attestation de provenance** GitHub (`actions/attest-build-provenance`) qui les lie au commit et au workflow public qui les a construits. Un exécutable sans attestation est accepté non vérifié, et ne peut pas être vérifié (§4.4).

### 3.3 Les contrôles automatiques

Sur l'archive figée (§3.4), dans un dossier temporaire du travailleur, **sans rien exécuter du plugin** :

1. Le manifeste est là, au bon schéma, et `escouade-plugin validate` passe (le validateur du SDK, PLG-9, ESC-181, appelé comme une bibliothèque : il lit, n'exécute pas). Son icône y passe aussi : l'en-tête lu par `schemas/plugin-manifest.mjs` (absente du dépôt, pas un PNG, SVG, pas carrée, moins de 256 px, plus de 1 Mo, chemin hors du dépôt ou distant), puis son décodage par le store (plus de 4096 px de côté, ou des pixels qui ne se décodent pas) ; une icône qui ne tient pas refuse la version, sous `icon`, avec la raison.
2. `version` du manifeste égale à l'étiquette (`v` facultatif) ; plus grande que la dernière publiée (semver).
3. `id` libre ou déjà à ce compte ; préfixe `dev.escouade.*` réservé ; un nom trop proche d'un plugin existant (distance d'édition, mêmes mots une fois les tirets retirés) part en revue manuelle avant publication (le typosquatting).
4. `repository` du manifeste égal au dépôt soumis.
5. La licence (§3.2).
6. Le README présent, non vide.
7. Les captures déclarées présentes, aux bons formats et tailles.
8. L'archive : moins de 50 Mo, pas de lien symbolique, pas de chemin absolu ni de `..`, pas de sous-module ni de fichier LFS non résolu.
9. Les permissions cohérentes avec les contributions (un plugin sans `run` qui demande `network` : refusé, il ne peut pas s'en servir).
10. Les binaires annoncés par `run` présents pour chaque système déclaré, et leurs attestations.

### 3.4 Chaque version figée

- **Un commit** : l'étiquette est résolue une fois, à la soumission ; le store garde le SHA du commit.
- **Une archive** : le store télécharge `GET /repos/{o}/{r}/tarball/<sha>` une fois, **la garde telle quelle** et en calcule le SHA-256. Raison de ne pas la redemander à GitHub plus tard : les archives générées par GitHub ne sont pas garanties identiques à l'octet dans le temps (la compression a déjà changé en 2023) ; l'empreinte doit porter sur des octets qui ne bougent plus. Les binaires de la release sont téléchargés, gardés et empreints de la même façon.
- **L'empreinte de la version** est le SHA-256 d'un petit fichier `version.json` canonique (clés triées, sans espaces) : `{ id, version, repository, commit, archive: { sha256, size }, binaries: { "windows-x64": { sha256 }, ... }, manifest_sha256 }`, et `icon: { "256": <sha256>, "64": <sha256> }` pour une version qui a une icône (PLG-13). C'est lui que l'app télécharge, vérifie et que la revue signe (§4.5) : une signature couvre ensemble l'identité, la version, le code et les binaires.
- **Une étiquette déplacée** après la soumission (le webhook `create`, ou une relecture hebdomadaire) : le store ne change rien à la version publiée, la marque « étiquette déplacée sur le dépôt » et prévient l'auteur et la file de revue. Une version ne se republie pas sous le même numéro.

### 3.5 Les mises à jour et les rejets

- **Une nouvelle étiquette** (webhook `release` ou `create`) : le store la lit et lance les contrôles ; si tout passe, une **version en attente** que l'auteur publie d'un clic (ou d'office, si l'auteur a coché « Publier les nouvelles étiquettes seul » pour ce plugin). Raison de ne pas tout publier d'office : une étiquette de test poussée par erreur paraîtrait chez tous les utilisateurs.
- **Un rejet automatique** dit, contrôle par contrôle : ce qui a échoué, où (le chemin du champ, `permissions.network[0]`), pourquoi, comment le corriger, et la page de la documentation (`/docs/plugins/publishing`). Exemple : « La licence du manifeste est MIT, le fichier LICENSE est sous GPL-3.0 : mets les deux d'accord, puis pousse une nouvelle étiquette. » Le même texte part par e-mail si l'auteur en a donné un.
- **Corriger** = pousser une nouvelle étiquette (ou relancer les contrôles si la panne venait du store).

**Fait le 2026-10-29** (ESC-230, STORE-3) : `/submit` et `/en/submit` comme l'aperçu `#submit` (les étapes, le dépôt et la version choisis, « v0.3.0 → a41c9d2 », les dix contrôles relus chaque seconde pendant qu'ils tournent, puis la publication avec ses catégories, ses mots-clés et la case de la revue) ; la file `store.check` qui télécharge l'archive une fois (50 Mo au plus, coupée en route), la lit en mémoire sans rien écrire ni exécuter (`server/utils/tar.ts` : liens, fichiers spéciaux, chemins absolus ou en `..` refusés entrée par entrée ; une bombe arrêtée à 200 Mo décompressés ; 20 000 entrées au plus), la garde telle quelle sous son SHA-256 (`<ARCHIVE_DIR>/sha256/ab/<empreinte>.tar.gz`, comme les binaires et les captures réencodées), et écrit `version.json` canonique et son empreinte (§3.4, avec `escouade` pour la compatibilité de §8.1) ; les dix contrôles (`server/utils/checks.ts`), chacun en français et en anglais, sur 30 plugins piégés et des valides (`tests/fake/plugins.mjs`) ; le rejet par e-mail si l'auteur a une adresse ; les nouvelles étiquettes par les webhooks `create`, `release` (publiée) et `push` d'une étiquette, l'étiquette déplacée notée et dite à l'auteur, et la relecture de chaque lundi à 5 h 20 ; 20 soumissions par heure et par compte. Ce qui a été tranché en le construisant :
- **les binaires** : un fichier de la release par système et architecture, nommé `…-<windows|macos|linux>-<x64|arm64>…` (ce que fera `escouade-plugin pack`, PLG-9) ; chaque système dont le programme de `run` (le sien, sinon `command`) n'est pas dans le dépôt doit avoir le sien ;
- **les attestations** : le store lit la déclaration in-toto de chaque attestation que GitHub liste pour l'empreinte du fichier (le dépôt et le commit de la construction, le workflow) ; la signature Sigstore du paquet est vérifiée par la revue sur le poste (`gh attestation verify`, STORE-7), seule à donner le badge. Une attestation d'un autre commit ou d'un autre dépôt refuse la version ;
- **les permissions** : sans `run` ni `contributes.webhooks`, ni `network`, ni `events`, ni `secrets` ; sans `run`, pas de `commands` ; un secret demandé est déclaré dans `contributes.secrets` ; un hôte s'écrit sans `https://` ni chemin ;
- **les noms trop proches** : le dernier mot de l'id ou le nom, identiques sans les tirets, ou à une lettre près (deux à partir de 8 lettres) d'un plugin d'un autre compte ; la version reste « en revue manuelle » (`held`) jusqu'à la file de STORE-8 ;
- **une archive dont une entrée est refusée** (un lien, un `..`) se lit quand même pour le reste : tous les contrôles disent ce qu'ils trouvent d'un coup ; une archive trop grosse, une bombe ou des octets illisibles font attendre les contrôles qui lisent les fichiers ;
- **une étiquette poussée de force** (`git push -f` d'une étiquette) n'envoie ni `create` ni `release` : l'App doit être abonnée à l'événement **Push** pour la voir tout de suite (à cocher dans ses réglages sur GitHub), sinon la relecture du lundi la trouve ;
- **le compte supprimé** garde ses plugins sans auteur et leurs ids réservés, en attendant le transfert et la suppression de STORE-5.

## 4. La revue « Vérifié par Escouade »

### 4.1 Côté store : la file

Une page `/review`, visible du seul administrateur (son identifiant GitHub dans la variable `STORE_ADMINS`) : les versions qui demandent la revue, les plus installées d'abord puis les plus anciennes ; pour chacune, l'auteur, les permissions (et ce qui a grandi depuis la dernière version vérifiée), la taille du diff, les signalements, et la commande à copier : `npm run plugin-review -- dev.exemple.slack@1.3.0`. La file mêle les signalements et les noms trop proches (§3.3) à revoir.

L'API de la file (`/api/v1/review/*`) demande un **jeton d'administration** que l'utilisateur crée dans son compte et garde dans le trousseau du système ; il permet de lire la file, de poser un rapport et de déposer une signature ; il ne permet ni de publier ni de supprimer un plugin.

### 4.2 Côté poste : un agent dans ce dépôt

Deux portes, la même mécanique :
- **`npm run plugin-review -- <id>@<version>`** (`scripts/plugin-review/`) : la partie déterministe, sans IA ;
- **`/review-plugin <id>@<version>`** (`.claude/commands/review-plugin.md`) : la commande de Claude Code qui lance le script, puis lit le code et rédige le rapport.

Ce qu'ils font, dans l'ordre :

1. **Télécharger** `version.json`, l'archive et les binaires depuis le store, dans un dossier neuf hors du dépôt (`%TEMP%/escouade-review/<id>-<version>-<aléa>/`, `$TMPDIR` ailleurs), et **vérifier les empreintes** contre `version.json` ; comparer aussi le commit à celui du dépôt sur GitHub (une étiquette déplacée se voit ici).
2. **Extraire** sans suivre de lien, sans chemin qui sorte du dossier ; **rien ne s'exécute** hors du bac à sable : ni `npm install`, ni script, ni binaire. Le dossier est passé en argument, jamais comme dossier courant d'un interpréteur.
3. **Lire au regard des permissions** : l'agent reçoit le manifeste et la liste des fichiers, et lit le code en cherchant ce que les permissions ne couvrent pas. Le script lui prépare des pistes : appels réseau (`fetch`, `http`, `net`, `dns`, `WebSocket`, `XMLHttpRequest`, URL en dur) comparés à `permissions.network` ; fichiers lus ou écrits hors du dossier du plugin ; secrets (`process.env`, chemins de `~/.ssh`, `~/.claude`, trousseau) ; exécution de programmes (`child_process`, `spawn`, `exec`, `Command`) ; code dynamique (`eval`, `new Function`, `import()` d'une URL) ; code illisible (minifié, encodé en base64, blobs binaires) ; dépendances (`package.json`, versions épinglées, scripts `postinstall`) ; le fichier de consignes ajouté aux agents (une injection de prompt est un comportement à risque).
4. **Passer le banc de conformité** (PLG-3, `escouade-plugin test`) **dans un conteneur sans réseau** (`docker` ou `podman run --network=none --read-only --tmpfs /tmp`, le dossier monté en lecture seule, une limite de mémoire et de temps) ; les tentatives de connexion et les écritures refusées vont au rapport. Sans moteur de conteneur, le banc n'est pas lancé et le rapport le dit en tête : l'utilisateur décide en connaissance.
5. **Pour une mise à jour** : le diff avec la dernière version vérifiée, et seulement lui à lire en détail ; les permissions qui grandissent en tête.
6. **Le rapport** : `~/.escouade-review/<id>/<version>.md` (hors du dépôt : il peut citer du code d'un tiers), avec le verdict proposé (« Vérifier », « Demander des changements », « Refuser »), chaque constat avec son fichier et sa ligne, ce qui n'a pas pu être vérifié, le résultat du banc. Un **résumé public** court (permissions relues, réseau vers quels hôtes, ce qui a été vérifié) est préparé à part.

### 4.3 La décision

L'utilisateur lit le rapport et décide. Trois issues :
- **Vérifier** : `npm run plugin-review -- sign <id>@<version>` (§4.5), qui dépose la signature et le résumé public ;
- **Demander des changements** : un message à l'auteur, la version reste publiée non vérifiée ;
- **Refuser** : la version reste non vérifiée, le message dit pourquoi ; un comportement malveillant mène au retrait (§4.6).

Le temps de revue est le vrai coût du store : la file trie par installations ; une mise à jour se relit sur son diff ; le banc et les pistes du script mâchent le travail ; rien n'oblige à tout vérifier (un plugin non vérifié reste installable, avec sa mention).

### 4.4 Ce que le badge couvre

- **Une version**, désignée par l'empreinte de son `version.json` (§3.4) : son code source au commit, son archive, ses binaires attestés.
- **Pas le plugin entier** : la v1.3.0 d'un plugin dont la v1.2.0 est vérifiée est « non vérifiée, en revue » jusqu'à sa signature. Raison : une mise à jour est l'endroit où un plugin honnête devient dangereux (compte repris, dépendance piégée) ; un badge pour le plugin le cacherait.
- **Une nouvelle version** demande une nouvelle revue, sur le diff ; l'app propose la mise à jour en le disant (« la v1.3.0 n'est pas encore vérifiée ; la v1.2.0 l'est ») et laisse l'utilisateur attendre la vérification (réglage « N'installer que des versions vérifiées », éteint d'office).
- **Un exécutable sans attestation de provenance** ne peut pas être vérifié : ce qu'on lit doit être ce qui tourne.

### 4.5 La signature

- **minisign** (Ed25519), comme `latest.json` de l'updater de Tauri ; l'app a déjà `minisign-verify` dans ses dépendances (`src-tauri/Cargo.lock`).
- **Ce qui est signé** : `version.json` (§3.4), auquel la signature ajoute dans son commentaire de confiance `escouade-verified-v1 <id>@<version> <date>`. Signer le seul SHA-256 de l'archive ne lierait pas la signature à l'identifiant et au numéro (une archive vérifiée resservie sous un autre nom).
- **La clé** : une paire **à part** de celle de l'updater (qui vit dans les secrets de la CI). Sa clé secrète est créée sur le poste de l'utilisateur (`minisign -G`), chiffrée par un mot de passe (scrypt, le format de minisign), gardée hors du dépôt (`~/.escouade-signing/store.key`), sauvegardée hors ligne (un gestionnaire de mots de passe et une clé USB rangée). **Jamais sur le serveur, jamais dans la CI, jamais dans un dépôt.**
- **Une clé de secours** : une seconde paire, créée en même temps, gardée seulement hors ligne. L'app embarque **les deux clés publiques** : la perte de la première n'oblige pas à attendre une version de l'app pour continuer à signer.
- **Envoyer** : le script signe en local (le mot de passe demandé dans le terminal), puis dépose la signature au store par l'API de la file. Le store la vérifie avec les clés publiques avant de l'accepter (une erreur de manipulation ne se publie pas), puis la sert à côté de la version.
- **Dans l'app** : la signature vérifiée avec les clés publiques embarquées, contre le `version.json` téléchargé, dont les empreintes sont comparées aux octets reçus. Tout concorde : « Vérifié par Escouade ». Le store ne peut pas fabriquer un badge : il n'a pas la clé.

### 4.6 La révocation, le retrait d'urgence et les signalements

- **La liste de révocation** : `revocations.json`, signée par la même clé, servie par le store et copiée sur escouade.dev (un second chemin si le store tombe) : `{ serial, issued_at, entries: [{ id, versions: "*" | ["1.2.0"], action: "unverify" | "disable", reason: { fr, en }, date }] }`.
  - `unverify` : le badge part, le plugin continue, une bande dit pourquoi ;
  - `disable` : le **retrait d'urgence** — l'app arrête le plugin, le désactive et dit pourquoi, avec « Désinstaller » ; elle ne supprime rien seule (les données de l'utilisateur sont à lui).
- **La copie sur escouade.dev** (décidée le 2026-10-29, ESC-233) : **qui la publie** : le workflow `.github/workflows/revocations.yml` de ce dépôt, sur les machines de GitHub, indépendant du serveur du store ; **à quel rythme** : toutes les 15 minutes ; il lit la liste du store, la vérifie avec les clés publiques du dépôt (signature, commentaire de confiance `escouade-revocations-v1 <serial>`, `serial` au-dessus de celui de la copie), l'écrit telle quelle avec sa signature dans `website/public/` et la commite sur `main`, ce qui redéploie le site par Coolify : **une heure de retard au plus**. Un store qui sert une liste fausse ou plus vieille ne change rien à la copie et fait échouer le workflow, ce que GitHub dit par e-mail ; un store tombé la laisse telle quelle. L'app lit la copie à `https://escouade.dev/revocations.json` et `.minisig` quand le store ne répond pas, et la vérifie comme l'autre.
- **Lue par l'app** au démarrage puis toutes les 6 heures ; gardée en cache ; un `serial` plus petit que le dernier vu est refusé (personne ne rejoue une vieille liste pour lever une révocation).
- **Le store** retire de la liste, d'office, une version désactivée ; l'auteur est prévenu (sauf si le prévenir aiderait un attaquant).
- **Les signalements** : « Signaler » sur la page d'un plugin, ouvert à tous par un formulaire avec une adresse e-mail, sans compte GitHub (le DSA ; décidé à la revue du 2026-10-29, §8.1, fait par STORE-4 : une déclaration de bonne foi, 5 par adresse et par heure, un accusé de réception par e-mail) et dans l'app (qui ouvre cette page) : malveillant, ne fait pas ce qu'il dit, permissions trompeuses, licence, autre. Ils vont dans la file de revue, avec le nombre par plugin ; l'auteur voit qu'un signalement est ouvert, pas qui l'a fait. Un plugin « malveillant » signalé par plusieurs comptes remonte en tête ; la décision reste humaine.

## 5. Dans l'app

### 5.1 Réglages › Plugins › « Parcourir le store »

Un bouton « Parcourir le store » dans l'en-tête de l'onglet Plugins (PLG-4, ESC-176), qui ouvre le store dans l'onglet même (pas une fenêtre de plus) :
- **la recherche**, sur le catalogue téléchargé (nom, description, mots-clés, auteur, sans tenir compte de la casse ni des accents, comme la recherche des conversations) ;
- **les catégories** : Harness, Thèmes, Notifications, Kanban, Autorisations, Outils, Langues (celles des points d'extension de la 2.1, puis les suivants) ;
- **le filtre** « Vérifiés seulement » ;
- **la liste** : nom, auteur, description, badge, installations, ce que le plugin demande en une ligne (« Lance un programme », « Se connecte à hooks.slack.com »), « Installé » ou « Mise à jour » sur ceux qu'on a ;
- **la page d'un plugin** : le README et la doc, les captures, les permissions en mots, les versions (chacune vérifiée ou non, retirée), l'auteur et son dépôt, le badge, le résumé public de la revue, « Installer » et « Signaler ».

### 5.2 L'installation et les mises à jour

- « Installer » passe par **la fenêtre des permissions de PLG-4** (ESC-176), qui gagne l'origine « store.escouade.dev » et le badge (ou « non vérifié »).
- L'app télécharge `version.json`, l'archive et le binaire de son système, **vérifie les empreintes**, puis la signature si la version est vérifiée ; un écart arrête tout et le dit (« L'archive reçue ne correspond pas à celle que le store a publiée »).
- **Les mises à jour viennent du store** pour un plugin installé du store (le catalogue dit la dernière version) ; jamais en silence (§3.2 de `docs/PLUGINS.md`) ; l'accord redemandé si les permissions grandissent ; « non vérifiée » dit sur la mise à jour.
- **Un plugin installé depuis un dépôt git** reste ce qu'il est : « non vérifié », mis à jour par ses étiquettes. Si le même identifiant est au store, l'app le dit et propose de passer à la version du store.

### 5.3 Ce que l'app ne fait pas

Pas de compte sur le store dans l'app, pas de note ni d'avis (un avis demande une modération que le store n'a pas au départ ; les installations et le badge suffisent), pas de recommandations personnalisées.

### 5.4 Ce que l'app envoie au store

- **Le catalogue, la liste de révocation, les archives** : de simples GET, sans identifiant, sans cookie. Le serveur voit l'adresse IP (journal du proxy, 1 an), comme pour le site.
- **La recherche** : rien, elle se fait sur la machine.
- **Le compte des installations** : seulement si l'utilisateur a accepté les statistiques (`Settings.telemetry`), un `POST /api/v1/installs { id, version, event: "install" | "update" }`, sans identifiant d'installation ; le store incrémente un compteur du jour et ne garde rien d'autre. Sans accord, rien ne part ; l'auteur voit des chiffres plus bas que la réalité, et sa page le dit.
- **Les statistiques d'Escouade** (Aptabase) gagnent « plugin installé depuis le store » (vrai ou faux), sans le nom du plugin.

**Fait le 2026-10-29** (ESC-233, STORE-6) : l'API de l'app sous `/api/v1/` (le détail des routes dans `store/README.md`, « L'API de l'app ») : `catalog.json` entier, compressé, avec son `ETag` ; `plugins/<id>/<version>/version.json` à l'octet près, `archive.tar.gz` et `binaries/<système>` (aussi en `HEAD`), cachés un an ; `revocations.json` et sa signature ; `POST installs`. Ce qui a été tranché en le construisant :
- **le catalogue** porte, par version, l'empreinte (`sha256` de `version.json`), la compatibilité (`escouade.api`, `min`, `max` du manifeste, §8.1 : l'app choisit la dernière version qu'elle peut faire tourner), ce que la version demande (`permissions`, `run`, la liste de `contributes`), ses systèmes et les chemins de ses fichiers, son icône (`icon`, les empreintes de ses PNG en 256 et 64 px comme dans `version.json`, et `files.icon` leurs chemins sous `/files/`, PLG-13) ; `verified` reste faux jusqu'aux signatures de STORE-8 ; le README n'y est pas (la page du plugin, STORE-4) ;
- **les installations du catalogue sont celles des jours d'avant** (`installs_until`) : le catalogue ne change pas à chaque installation, son `ETag` tient la journée, et l'heure d'une installation ne se lit pas dans les chiffres ; il est refait à chaque publication, et une minute au plus après un autre changement ;
- **le compteur** n'accepte que du JSON aux trois clés (un formulaire d'un autre site ne peut pas l'envoyer, l'`Origin` n'est donc pas demandée), compte une version publiée seulement, une fois par jour et par adresse ;
- **les limites de débit** sont tenues en mémoire, par une empreinte HMAC de l'adresse à clé tirée au démarrage, jamais écrite : 300 requêtes de l'API par minute, 300 téléchargements par heure, 60 `POST /installs` par heure, 60 soumissions, relances ou publications par heure et par adresse (plus les 20 soumissions par heure et par compte) ; larges, pour les proxys d'entreprise ; l'adresse est la dernière de `X-Forwarded-For`, celle que Traefik a vue ;
- **une version désactivée** par la liste de révocation sort du catalogue et ses fichiers répondent 410 ; `version.json` reste servi (il ne contient pas de code) ;
- **les clés publiques** sont lues dans `store/keys/*.pub` (copiées dans l'image) ; la vérification minisign est écrite sans dépendance (`server/utils/minisign.ts`), essayée sur les vecteurs de la crate `minisign-verify` de l'app, la forme préhachée seule ;
- **SPF, DKIM et DMARC** de `store@escouade.dev` : les enregistrements à poser chez Gandi et `store/scripts/check-mail-dns.ts` qui les vérifie sont dans `store/README.md` ; le 2026-10-29, le SPF d'escouade.dev ne connaît que Gandi, ni DKIM ni DMARC : à poser par l'utilisateur.

## 6. Les pages du store

Sur le design du site (les jetons de `website/app/assets/main.css` : fond presque noir, Inter, filets blancs translucides, le teal pour l'action principale et le focus), avec un thème clair qui suit le système ; en français (`/`) et en anglais (`/en/`), comme le site.

- **L'accueil** : la recherche en tête, les catégories, « Vérifiés par Escouade » (une rangée), « Nouveaux », « Les plus installés », et une bande « Publier un plugin » qui mène à la documentation et à la soumission.
- **La recherche** : les résultats en liste, les filtres (catégorie, vérifiés, ce que le plugin demande : « sans programme » pour les déclaratifs), le tri (pertinence, installations, récents).
- **La page d'un plugin** : en tête le nom, l'auteur, le badge, « Ouvrir dans Escouade » (un lien `escouade://plugin/<id>` qui ouvre l'app sur sa page) et la commande pour un dépôt ; puis la doc (onglets README, docs, versions), et à droite les permissions en mots, l'origine (dépôt, commit, empreinte), la licence, les installations, le résumé de la revue, « Signaler ».
- **Le compte de l'auteur** : ses plugins, leurs versions et états, les statistiques, les actions (retirer une version, retirer de la liste, transférer, supprimer), les avis.
- **La soumission** : dépôt, version, contrôles en direct, publier.
- **La file de revue** (l'utilisateur seul) : la file, les signalements, les noms proches.

**Fait le 2026-10-29** (ESC-231, STORE-4) : l'accueil, la recherche (`/search`), la page d'un plugin (`/plugins/<id>`, ses onglets Présentation, Documentation et Versions, et `/plugins/<id>/report`) et celle d'un auteur (`/authors/<login>`), en français et en anglais, comparées à l'aperçu `design/store/` dans les deux thèmes et à 1440, 1028 et 390 px (`tests/e2e/catalog.spec.ts`). Ce qui a été tranché en les construisant :
- **le README** est rendu à la vérification de la version (`server/utils/readme.ts`, rangé dans `versions.docs`), une fois ; une version publiée avant l'est à sa première visite, depuis son archive gardée. Le markdown (markdown-it) passe par une liste courte de balises et d'attributs (sanitize-html) : ni script, ni style, ni attribut `style`, `class` (sauf la langue d'un bloc de code), `id` ou d'événement, ni cadre, ni formulaire ; des liens `http`, `https` et `mailto` seulement, marqués `nofollow ugc` ; un lien vers un fichier du dépôt mène à GitHub au commit figé ;
- **le nonce** de la politique de sécurité ne marque plus que les morceaux de Nuxt (`head`, `bodyPrepend`, `bodyAppend`), jamais le corps de la page où est le README (`server/utils/headers.ts`, relecture de STORE-2) : une balise qui passerait l'assainissement ne s'exécuterait pas ;
- **les images** d'un README (relatives, ou par leur adresse sur GitHub dans le même dépôt) sont lues dans l'archive, réencodées sans métadonnées et gardées sous leur SHA-256 comme les captures, servies par `/files/<sha256>.<png|jpg|webp>` (`immutable`) ; les autres (badges, pixels d'un tiers) sont retirées ;
- **la recherche** se fait sur le serveur (décidé à la revue du 2026-10-29) : les mots dans le nom, les mots-clés, les catégories, l'auteur et la description, sans tenir compte des accents ni de la casse, chacun devant se trouver quelque part ; tri par pertinence, installations ou date ; chaque filtre est un lien, la page marche sans script ;
- **les plugins de première main** (`server/utils/builtin.ts`) : les huit harness, GitHub, GitLab, Azure DevOps, Bitbucket, Gitea et Forgejo, Jira, Trello, Linear et ClickUp, avec la marque « Livré avec Escouade » aux couleurs du sceau et leur page sous `/authors/escouade` ; une fois sortis du cœur (INT-4 et après), leur page pointe vers leur dossier `plugins/<id>/` du dépôt de l'app, à l'étiquette de la dernière version (décidé le 2026-10-29, `docs/PLUGINS.md` §4.3) ; ils remplissent la rangée « Vérifiés par Escouade » tant que STORE-8 n'a rien signé ;
- **l'icône** (PLG-13, ESC-292) : `PluginTile` montre l'icône de la dernière version publiée à la place des lettres partout où elle existe (accueil, recherche, page du plugin, page de l'auteur, `/account`), en 64 px pour les petites tuiles et 256 px sinon, servie par le store lui-même (même origine, CSP inchangée) ; les plugins de première main ont le logo de leur marque là où ses règles publiées le permettent sans rien demander (GitHub, OpenCode, Gitea), sinon le glyphe de leur catégorie (`docs/PLUGINS.md` §4.3, dessinés dans la page par `shared/marks.ts`) ;
- **le badge** : « En revue » si l'auteur l'a demandée, « Non vérifié » sinon, la signature de la chaîne d'origine « Pas encore signée » ; le sceau viendra des signatures de STORE-8. les installations sont celles du compteur de STORE-6 jusqu'à la veille, et une version que la liste de révocation désactive quitte les listes (« Retirée » dans l'onglet Versions) ;
- **« Signaler »** : la table `reports` (plugin, version, motif, texte, adresse, langue), l'accusé de réception par e-mail, le nombre de signalements ouverts sur la page, jamais qui l'a fait ; la file de STORE-8 les lira.

## 7. La sécurité de bout en bout

| Ce qu'un attaquant tente | Ce qui l'empêche |
| --- | --- |
| **Reprendre un dépôt** (compte GitHub de l'auteur volé, dépôt transféré) et pousser une étiquette piégée | la nouvelle version est **non vérifiée** (le badge vaut par version) ; elle attend un clic de publication de l'auteur (sauf réglage contraire) ; un transfert gèle les publications 7 jours et se dit sur la page ; l'app n'installe une mise à jour qu'après un clic, en disant « non vérifiée » ; le réglage « Vérifiées seulement » la bloque |
| **Déplacer une étiquette** après la revue | le store a figé le commit et l'archive à la soumission ; une étiquette déplacée est signalée ; la revue compare le commit à celui du dépôt |
| **Changer l'archive** après la revue (sur le store, sur le chemin, dans un cache) | l'app compare les octets aux empreintes de `version.json`, et `version.json` à la signature ; un octet changé arrête l'installation |
| **Compromettre le serveur du store** | il n'a pas la clé : il peut servir une archive piégée **non vérifiée**, pas un faux badge ; il peut cacher une révocation (d'où la copie sur escouade.dev et le `serial` qui ne recule pas) ; les archives vérifiées restent vérifiables par l'app |
| **Voler le compte du store de l'auteur** | il peut publier des versions non vérifiées, retirer, transférer (gelé 7 jours) ; tout passe au journal et prévient l'auteur par e-mail ; GitHub porte la double authentification |
| **Voler le compte d'administration** (le jeton de la file) | il peut lire la file et déposer un rapport ; une signature déposée est vérifiée par les clés publiques : sans la clé privée, rien ne passe |
| **Voler la clé de signature** | chiffrée par mot de passe, hors ligne en dehors des signatures ; si elle fuit : la révoquer par une version de l'app qui ne l'embarque plus (la clé de secours prend le relais tout de suite), resigner les versions vérifiées avec la clé de secours, et une liste de révocation de l'ancienne clé (`keys_revoked`) signée par la clé de secours |
| **Perdre la clé de signature** | la clé de secours continue ; l'app suivante embarque une nouvelle clé de secours |
| **Connecter un visiteur sous un autre compte** (un lien de retour de GitHub forgé) | l'état tiré au départ de la connexion, gardé dans un cookie de 10 minutes et comparé en temps constant au retour ; une installation commencée sur GitHub, qui revient sans lui, relance la connexion au lieu d'utiliser son code |
| **Faire agir un auteur connecté depuis un autre site** | le cookie `SameSite=Lax`, et l'en-tête `Origin` de chaque formulaire comparé à l'adresse du store |
| **Forger ou rejouer un webhook** | la signature `X-Hub-Signature-256` sur les octets reçus, comparée en temps constant ; une livraison n'est traitée qu'une fois |
| **Injecter un script dans une page du store**, ou la mettre dans un cadre | une politique de sécurité du contenu stricte : aucun script en ligne hors du nonce de la réponse, `frame-ancestors 'none'` ; HSTS |
| **Voler la base du store** | elle ne garde que l'empreinte des jetons de session et des liens, et le jeton GitHub chiffré par une clé qui n'est que dans les variables de Coolify |
| **Typosquatter** un plugin connu | les noms trop proches passent en revue manuelle avant publication ; le badge et l'auteur sont sur chaque ligne |
| **Mentir sur ses permissions** | l'app applique ce qu'elle peut (réseau des harness, secrets, événements, clôture) ; la revue lit le reste ; un mensonge découvert mène au retrait |
| **Injecter des consignes** dans les agents par un plugin | une permission montrée (« Ajoute des consignes à vos agents ») ; la revue lit ces consignes |
| **Faire exécuter du code au serveur** par une archive piégée (zip slip, bombe) | le serveur n'exécute jamais un plugin ; l'extraction refuse liens et chemins sortants ; tailles bornées ; le validateur lit sans exécuter |
| **Piéger le poste de l'utilisateur** pendant la revue | l'archive dans un dossier neuf hors du dépôt, rien d'exécuté hors du conteneur sans réseau, le dossier passé en argument, jamais comme dossier courant |
| **Suivre les utilisateurs** par les images d'un README | les images sont copiées sur le store à la soumission, servies depuis lui |

**Où vit la clé** : sur le poste de l'utilisateur, chiffrée, et en double hors ligne ; la clé de secours seulement hors ligne. Les clés publiques dans l'app (et sur escouade.dev, pour qui veut vérifier à la main).

**Si le store tombe** : l'app garde le catalogue en cache (la dernière version lue, son âge dit dans « Parcourir le store ») ; les plugins installés tournent, leurs badges se vérifient hors ligne (la signature et la clé sont sur la machine) ; la liste de révocation en cache s'applique, et se lit aussi sur escouade.dev ; l'installation depuis un dossier ou un dépôt git marche toujours ; seules les installations du store attendent son retour.

## 8. Les décisions

**Prises par l'utilisateur le 2026-10-26** : chaque recommandation ci-dessous est retenue (GitHub App, badge par version, plugins non vérifiés listés et marqués, PostgreSQL de Coolify avec archives sur le serveur et sauvegardes Scaleway, binaires attestés), et le design de l'aperçu (`design/store/`, `#store*` de l'app, choix 160 à 181) est validé. Construction prévue avec les plugins (« On développera les plugins et la marketplace jeudi prochain »).

Déjà tranché (2026-10-26) : le code vit dans ce dépôt, `store/`, et le push sur `main` le déploie par Coolify (§1.1, §1.3, §1.5).

1. **La connexion** : une GitHub App « Escouade Store » (lecture seule, preuve de propriété et webhooks d'un geste), GitLab plus tard. **Recommandé.** Autre choix : OAuth simple (pas de preuve de propriété) ou GitLab dès le départ (un ticket de plus avant le lancement).
2. **Ce que le badge couvre** : une version (son empreinte), le code source au commit et les binaires attestés ; une nouvelle version repasse en revue sur son diff. **Recommandé.** Autre choix : le plugin entier, toutes versions (moins de revues, mais un compte repris hérite du badge).
3. **Le store liste-t-il les plugins non vérifiés ?** Oui, marqués « non vérifié », avec un filtre « Vérifiés seulement » et un réglage de l'app « N'installer que des versions vérifiées » éteint d'office. **Recommandé** : la revue ne doit pas devenir le goulot qui empêche un plugin d'exister, et l'installation depuis un dépôt git le permet déjà. Autre choix : seulement les vérifiés (un catalogue plus sûr, plus petit, et une file qui décide du rythme).
4. **Le stockage et les sauvegardes** (Coolify étant décidé) : PostgreSQL de Coolify, archives sur un volume du serveur actuel (Scaleway, Paris), sauvegardes nocturnes vers le stockage objet de Scaleway, une sonde extérieure qui prévient sur Discord. **Recommandé** (moins d'un euro par mois de plus, aucune donnée hors UE). Autre choix : les archives directement dans le stockage objet (plus de place, un service de plus sur le chemin des téléchargements).
5. **Les binaires** (ajoutée par l'analyse) : un exécutable n'est vérifiable qu'avec une attestation de provenance GitHub qui le lie au commit relu. **Recommandé.** Autre choix : vérifier seulement les plugins sans binaire (déclaratifs, ou lancés par un runtime présent).

### 8.1 La revue du 2026-10-29

Le plan du store, revu avec celui des plugins (`docs/PLUGINS.md` §4.12), couvrait bien la pile, la CI, Coolify, les sauvegardes, la GitHub App, la soumission et ses contrôles, la revue, la signature, la révocation et le RGPD. Il lui manquait :
- **la compatibilité** : `escouade.api`, `min` et `max` dans le catalogue et `version.json`, et l'app qui choisit la dernière version compatible (STORE-6, STORE-9) ;
- **les limites de débit** de l'API, des soumissions et de `POST /installs`, pour que le compteur ne se gonfle pas (STORE-6) ;
- **la copie de `revocations.json` sur escouade.dev**, avec son mécanisme écrit : qui la publie, à quel rythme, une heure de retard au plus, un fichier signé (STORE-6) ;
- **le lien profond** `escouade://plugin/<id>` sur les trois systèmes et leurs installeurs, qui ouvre la page du plugin dans Réglages › Plugins et n'installe jamais rien (STORE-11, ESC-287) ;
- **le réseau** : le catalogue et les archives par Réglages › Réseau (proxy, autorité de certification), comme l'updater qui reçoit un 403 derrière Zscaler (STORE-9) ;
- **la télémétrie avec l'accord** de l'utilisateur (`POST /installs`, l'événement Aptabase), et **le passage d'une installation par git au store** pour le même id (STORE-9) ;
- **les plugins proposés par un projet** (`plugins.recommended` de `.escouade.yml`, PLG-14, ESC-288).

Décidé par le chef d'escouade, à confirmer par l'utilisateur s'il le veut autrement :
- les plugins de première main ont une page, avec la marque « Escouade », leurs ids `dev.escouade.*` réservés ;
- les catégories suivent le choix 214 (« Sources du Kanban », « Hébergeurs de code ») ;
- « Signaler » est ouvert à tous par un formulaire avec une adresse e-mail (DSA), publier demande un compte GitHub ;
- SPF et DKIM pour `store@escouade.dev` (STORE-4, STORE-6).

## 9. Le découpage en tickets

Les tickets s'appellent ici STORE-1, STORE-2… ; le chef d'escouade leur donne leurs clés. Les plugins (ESC-172 à ESC-184) restent le socle : le store lit leur manifeste, appelle leur validateur et leur banc, et l'app l'ouvre dans leur onglet.

| Plan | Ce qu'il fait | Modèle conseillé | Dépend de |
| --- | --- | --- | --- |
| STORE-1. Le socle du service | `store/` en Nuxt 4 rendu serveur, Drizzle et PostgreSQL, la file `pg-boss`, le Dockerfile depuis la racine (tests dans le build), le job `store` de la CI (§1.5), l'app Coolify et `store.escouade.dev` (§1.3, créée par le chef d'escouade ou l'utilisateur), `/api/health`, les sauvegardes (base et archives) et leur restauration essayée, la sonde extérieure, le français et l'anglais, les jetons du site et le thème clair ; pages légales liées | Sonnet medium (back) | — |
| STORE-2. Les comptes | La GitHub App (connexion, installations, webhooks signés), les sessions, l'e-mail vérifié facultatif, l'export et la suppression du compte, le journal des actions ; registre RGPD, `/privacy/` et `/terms/` mis en ligne | Sonnet medium | STORE-1 |
| STORE-3. La soumission et les contrôles | Dépôt et étiquette, résolution en commit, archive et binaires figés, `version.json` et son empreinte, les dix contrôles (§3.3) sur des fixtures de plugins valides et piégés, les rejets expliqués, les nouvelles étiquettes par webhook, l'étiquette déplacée | Sonnet medium | STORE-2, ESC-173 (PLG-1), ESC-181 (PLG-9 : `validate`) |
| STORE-4. Les pages publiques | Accueil, recherche, page d'un plugin (README assaini, images copiées, permissions en mots, versions, origine), page d'un auteur, « Ouvrir dans Escouade » ; comparées à l'aperçu `design/store/` dans les deux thèmes, à 1440, 1028 et 390 px | Opus medium (interface) | STORE-3 |
| STORE-5. L'espace de l'auteur | Ses plugins, versions et états, publier une version en attente, retirer une version, retirer de la liste, transférer (gel de 7 jours), supprimer (30 jours), statistiques, avis par e-mail | Opus medium (interface) | STORE-3 |
| STORE-6. L'API de l'app | `catalog.json` (ETag), `version.json`, archives et binaires (cache long), compteur d'installations sans identifiant, `revocations.json` et sa copie sur escouade.dev | Sonnet medium | STORE-3 |
| STORE-7. La revue locale | `npm run plugin-review` (télécharger, vérifier, extraire sans danger, pistes, banc dans un conteneur sans réseau, diff avec la dernière vérifiée, rapport hors du dépôt), `.claude/commands/review-plugin.md` ; essayé sur les plugins d'exemple et sur un plugin piégé exprès | Opus medium (prompt et sécurité) | STORE-6, ESC-175 (PLG-3), ESC-181 |
| STORE-8. La signature et la file | Les deux paires de clés (procédure écrite, sauvegarde hors ligne), `plugin-review sign`, le dépôt vérifié par le store, la file `/review`, le jeton d'administration, les signalements, le retrait d'urgence et la liste de révocation signée (`serial`) | Opus medium (sécurité) | STORE-7, STORE-5 |
| STORE-9. Le store dans l'app | « Parcourir le store » (catalogue en cache, recherche locale, catégories, filtre), la page d'un plugin, l'installation par la fenêtre de PLG-4 avec empreintes et signature vérifiées (clés embarquées), le badge, les mises à jour du store, « N'installer que des versions vérifiées », la liste de révocation (démarrage puis 6 h, `serial`), `escouade://plugin/<id>` ; faux store pour les e2e ; comparé à l'aperçu `#store*` | Opus medium (interface) | ESC-176 (PLG-4), STORE-6, STORE-8 |
| STORE-10. Les docs et l'essai réel | `website/content/docs/*/plugins/publishing.md` réécrit pour le store, `security.md` (ce que la revue refuse), `docs/SPEC.md`, `docs/PROTOCOL.md` si besoin, CHANGELOG fr/en ; un essai réel : publier les plugins d'exemple (ESC-182), les revoir, les signer, les installer dans l'app sur Windows, macOS et Linux, révoquer l'un | Sonnet medium | STORE-1 à STORE-9, ESC-182 (PLG-10) |

Ordre : STORE-1 → STORE-2 → STORE-3 → (STORE-4, STORE-5, STORE-6 en parallèle) → STORE-7 → STORE-8 → STORE-9 → STORE-10. Le service (STORE-1 à STORE-6) n'attend des plugins que le schéma et le validateur ; il peut avancer pendant que l'app construit PLG-2 à PLG-8. STORE-9 attend l'onglet Plugins (ESC-176).

## 10. Sources

- GitHub Apps, permissions et installation : <https://docs.github.com/en/apps/creating-github-apps/about-creating-github-apps/about-creating-github-apps> ; l'archive d'un commit : <https://docs.github.com/en/rest/repos/contents#download-a-repository-archive-tar> ; la stabilité des archives (2023) : <https://github.blog/open-source/git/update-on-the-future-stability-of-source-code-archives-and-hashes/> ; l'attestation de provenance : <https://docs.github.com/en/actions/security-for-github-actions/using-artifact-attestations>.
- minisign : <https://jedisct1.github.io/minisign/> ; l'updater de Tauri : <https://v2.tauri.app/plugin/updater/>.
- Ce que font les autres places de marché : `docs/PLUGINS.md`, §2.1 (Raycast et sa revue par PR, JetBrains et sa signature, VS Code et ses éditeurs vérifiés, Obsidian).
- Escouade : `docs/PLUGINS.md` (manifeste §3.1, installation §3.2, permissions §3.4, télémétrie §3.6), `docs/TODO-2.1.md` (PLG-1 à PLG-12), `docs/registre-rgpd.md`, `website/Dockerfile`.
