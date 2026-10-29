# Le store : store.escouade.dev

La place de marché des plugins d'Escouade (`docs/STORE.md`, qui fait foi). Ce dossier en est le service : **Nuxt 4 rendu par le serveur** (les pages en Vue dans `app/`, l'API Nitro dans `server/api/`), **PostgreSQL** par **Drizzle**, la file des tâches longues par **pg-boss** (dans PostgreSQL, pas de Redis), les archives sur un volume. Coolify le redéploie à chaque push sur `main` qui le touche ; la CI (job `store` de `.github/workflows/ci.yml`) le vérifie avant.

Ce qu'il fait aujourd'hui (STORE-1, ESC-228) : l'accueil en français (`/`) et en anglais (`/en`), dans les jetons du site (`website/app/assets/main.css`, lu par un chemin relatif) avec un thème clair qui suit le système ; `GET /api/health` ; les migrations jouées au démarrage ; la file et son battement ; la restauration des sauvegardes de Coolify, essayée. Les comptes des auteurs (STORE-2, ESC-229) : la connexion par la GitHub App « Escouade Store », la session, l'e-mail vérifié facultatif, l'export et la suppression du compte, le journal des actions, les webhooks signés de l'App ; `/account` et `/terms` dans les deux langues ; les en-têtes de sécurité sur tout le site. La soumission (STORE-3, ESC-230) : `/submit` et `/en/submit`, le dépôt et l'étiquette, le commit résolu une fois, l'archive et les binaires de la release gardés tels quels sous leur SHA-256, les dix contrôles sans rien exécuter, `version.json` et son empreinte, la publication non vérifiée, les nouvelles étiquettes par webhook et les étiquettes déplacées (`docs/STORE.md`, §3). Les pages publiques (STORE-4, ESC-231) : l'accueil, la recherche sur le serveur (accents et casse ignorés), la page d'un plugin (son README rendu à la vérification et assaini, ses images copiées, ses permissions en mots, ses versions, son origine, « Ouvrir dans Escouade »), « Signaler » sans compte, la page d'un auteur et celles des plugins d'Escouade (`docs/STORE.md`, §6). L'API de l'app (STORE-6, ESC-233) : le catalogue, `version.json`, les archives, le compteur d'installations et la liste de révocation sous `/api/v1`. L'espace de l'auteur (STORE-5) vient ensuite.

## Lancer en local

Node 22.19+ ou 24, et un PostgreSQL (16 ou 17) pour la base ; sans base, le serveur démarre quand même, sert les pages, et `/api/health` répond 503 `"database": "missing"`.

```sh
cd store
npm ci
cp .env.example .env      # puis DATABASE_URL vers ta base ; .env n'est jamais commité
npm run dev               # http://localhost:3000
```

Un secret manquant (`GITHUB_APP_*`, `SMTP_URL`, `MAIL_FROM`, `NUXT_SESSION_SECRET`, `STORE_ADMINS`) éteint sa fonction et le dit au démarrage, une ligne par fonction, sans la valeur : `[store] mail off (missing SMTP_URL, MAIL_FROM): no e-mail is sent`. Une clé de l'App qui ne se lit pas, ou un `NUXT_SESSION_SECRET` de moins de 32 caractères, le disent aussi.

Pour essayer la connexion en local, sans la vraie App : le faux GitHub des e2e (`tests/fake/github.mjs`) se lance seul, et le store le suit par `GITHUB_URL` et `GITHUB_API_URL` (ces deux variables ne servent qu'aux essais ; en production, elles n'existent pas et le store parle à GitHub). La « clé privée » se fabrique pour l'essai, jamais dans un fichier du dépôt :

```sh
FAKE_GITHUB_PORT=3010 FAKE_SMTP_PORT=3025 STORE_URL=http://localhost:3000 GITHUB_APP_CLIENT_ID=Iv23liFakeStore GITHUB_APP_CLIENT_SECRET=e2e-client-secret GITHUB_WEBHOOK_SECRET=e2e-webhook-secret node tests/fake/github.mjs
# puis, dans .env : les mêmes GITHUB_APP_*, GITHUB_APP_ID=1, GITHUB_URL=GITHUB_API_URL=http://127.0.0.1:3010,
# SMTP_URL=smtp://127.0.0.1:3025, MAIL_FROM=..., NUXT_SESSION_SECRET=$(openssl rand -hex 32),
# GITHUB_APP_PRIVATE_KEY=$(openssl genrsa 2048) ; les mails reçus se lisent sur http://127.0.0.1:3010/_fake/state
```

## Les tests

| Commande | Ce qu'elle vérifie |
| --- | --- |
| `npm run format:check` | Prettier |
| `npm run typecheck` | `nuxi typecheck` |
| `npm test` | Vitest : les variables, `/api/health`, les deux langues, aucun secret dans `store/`, les jetons, le chiffrement du jeton GitHub, la signature des webhooks (l'exemple de GitHub), la clé de l'App et son JWT, les en-têtes de sécurité, le lecteur d'archives (liens, chemins sortants, bombe), les captures réencodées sans leurs métadonnées, les licences, semver et `version.json`, les dix contrôles sur 30 plugins piégés et des valides, les attestations, le README assaini (un README piégé : `<script>`, `<style>`, `<img onerror>`, `javascript:`) et ses images copiées, le nonce posé sur les seuls morceaux de Nuxt, la recherche et les permissions en mots, minisign (sur les vecteurs de la crate de l'app), la liste de révocation, les limites de débit, la copie de la liste sur escouade.dev (le script lancé contre un faux store), SPF, DKIM et DMARC, les avis par e-mail de l'auteur ; avec `TEST_DATABASE_URL`, les migrations depuis une base vide, la file, les comptes (connexion, session, e-mail, export, suppression, webhooks rejoués, purge) la soumission (archive gardée à l'octet près, rejets, publication, nouvelle étiquette, étiquette déplacée, panne relancée) et l'API de l'app (catalogue, compatibilité, compteur, liste de révocation prise signée seulement et `serial` qui ne recule pas, versions désactivées retirées), le README rendu à la vérification et les pages publiques qui le lisent, les signalements, et l'espace de l'auteur (états, version en attente publiée ou ignorée, version retirée, hors de la liste, transfert et gel de 7 jours, dépôt transféré repris, suppression annulée puis purgée, installations par semaine et par version). **Cette base est vidée** : jamais une base qui compte. Elle n'est jamais lue de `DATABASE_URL`, que Coolify passe aussi au build |
| `npm run build && npm run test:e2e` | Playwright sur le serveur construit (`STORE_PORT`, 3000 par défaut ; la base de `DATABASE_URL` s'il y en a une), avec le faux GitHub et le faux SMTP de `tests/fake/github.mjs` (`FAKE_GITHUB_PORT` 3010, `FAKE_SMTP_PORT` 3025) : l'accueil et le compte comparés à l'aperçu `design/store/` dans les deux thèmes, la connexion (état forgé, annulée, installation avec et sans état), le cookie, l'e-mail et son lien, l'export, la déconnexion et la suppression (jetons révoqués), l'`Origin` des formulaires, les webhooks signés, la soumission (dépôt privé refusé, étiquette, dix contrôles, publication, plugin piégé refusé avec ses raisons, binaires attestés, nouvelle étiquette et étiquette déplacée par webhook, version d'un autre refusée) comparée à l'aperçu dans les deux thèmes, l'API de l'app (catalogue et son `ETag`, `version.json`, archive et binaires vérifiés par leur SHA-256, `HEAD`, compteur, limites de débit), les pages publiques (`tests/fake/catalog.mjs` publie trois plugins dans la base, dont un au README piégé) comparées à l'aperçu dans les deux thèmes et à 1440, 1028 et 390 px, la recherche, le README sans rien qui s'exécute ni nonce dans le corps, « Signaler » et son e-mail, l'espace de l'auteur (version en attente publiée du bandeau, refus expliqué sous sa ligne, version retirée, hors de la liste, suppression et son annulation, compte qui garde ses plugins, transfert accepté et gelé, avis par e-mail, plugin d'un autre refusé) comparé à l'aperçu dans les deux thèmes, les en-têtes et la CSP sans violation, les conditions, la 404, le téléphone, `/api/health`. Le faux GitHub sert aussi les dépôts où l'App est installée (`tests/fake/repos.mjs` : `*-private` privé, `*-trap` piégé, `*-bin` avec ses binaires) et pousse une étiquette par `POST /_fake/tag` (des fichiers en plus par `extra`, des champs du manifeste par `manifest`). Sans `DATABASE_URL`, les tests des comptes et de la soumission sautent |

Le `Dockerfile` rejoue `npm ci && npm run build && npm test`, sans `DATABASE_URL` ni `TEST_DATABASE_URL` : un test raté fait échouer le déploiement, la version en ligne reste la dernière qui passait.

## Les migrations

Le schéma est `server/db/schema.ts` ; `npm run db:generate -- --name <quoi>` écrit la migration suivante dans `server/db/migrations/`, à commiter avec lui. Elles se jouent au démarrage du conteneur, avant que `/api/health` dise `ok` (pendant ce temps : 503 `"starting"`, et Coolify garde l'ancien conteneur). **Une migration ne fait qu'ajouter** (tables, colonnes, index) : on ne retire ou ne renomme qu'au déploiement suivant, quand plus aucun code ne lit l'ancien nom (`docs/STORE.md`, §1.3).

## L'application Coolify

Créées par le chef d'escouade le 2026-10-29 (Coolify 4.5.3) d'après `docs/STORE.md` §1.3 ; ce qui suit permet de les recréer. `store.escouade.dev` pointe sur le serveur (62.210.173.28). **Aucun secret ne va dans le dépôt.**

**1. La base `store-db`** : projet « Escouade », environnement `production`, une ressource PostgreSQL 17, non exposée. Son adresse interne est `DATABASE_URL`.

**2. L'application `store`** : ce dépôt par l'App GitHub de Coolify (celle du site), branche `main`.

| Réglage | Valeur |
| --- | --- |
| Build Pack | Dockerfile |
| Base Directory | `/` (la racine du dépôt : le build lit `website/app/assets/main.css`) |
| Dockerfile Location | `/store/Dockerfile` (ce que le build voit : `store/Dockerfile.dockerignore`) |
| Watch Paths | `store/**`, `schemas/**`, `website/app/assets/main.css` |
| Auto Deploy | oui |
| Domains | `https://store.escouade.dev` (certificat par Traefik) |
| Ports Exposes | `3000` |
| Health Check | `GET /api/health`, hôte `127.0.0.1`, port 3000, toutes les 30 s, 60 s de démarrage (l'image a aussi son `HEALTHCHECK`, les mêmes). L'hôte compte : dans l'image Alpine, `localhost` se résout d'abord en `::1`, où Nitro n'écoute pas, et le premier déploiement est resté sur « Connection refused » |
| Persistent Storage | un volume monté sur `/data/archives` |
| Notifications | le salon Discord d'Escouade : déploiement raté, conteneur arrêté, sauvegarde ratée |

**3. Ses variables** (les secrets ne se relisent pas après saisie). Coolify les passe aussi au build, ce que son API ne sait pas couper : le `Dockerfile` n'en déclare aucune (pas d'`ARG`) et le build n'en lit aucune ; ses tests tournent sans `DATABASE_URL` ni `TEST_DATABASE_URL` ; l'étape finale de l'image ne lance aucune commande, rien ne reste dans son historique.

| Variable | Valeur | Secret |
| --- | --- | --- |
| `DATABASE_URL` | l'adresse interne de `store-db` | oui |
| `NUXT_PUBLIC_SITE_URL` | `https://store.escouade.dev` | non |
| `ARCHIVE_DIR` | `/data/archives` (déjà la valeur de l'image) | non |
| `NUXT_SESSION_SECRET` | 32 octets aléatoires (`openssl rand -hex 32`) ; STORE-2 | oui |
| `GITHUB_APP_ID`, `GITHUB_APP_CLIENT_ID` | l'App « Escouade Store » ; STORE-2 | non |
| `GITHUB_APP_CLIENT_SECRET`, `GITHUB_APP_PRIVATE_KEY`, `GITHUB_WEBHOOK_SECRET` | idem ; la clé PEM telle quelle, sur plusieurs lignes (ou sur une, aux `
` échappés) ; STORE-2 | oui |
| `STORE_ADMINS` | l'identifiant numérique GitHub de l'utilisateur ; STORE-8 | non |
| `SMTP_URL`, `MAIL_FROM` | Scaleway Transactional Email, `store@escouade.dev` ; STORE-2 | oui (`SMTP_URL`) |

Posées le 2026-10-29 : toutes sauf `SMTP_URL` (l'utilisateur configure Scaleway Transactional Email ; `MAIL_FROM` est posé). Sans elle, l'e-mail reste proposé sur `/account`, le lien ne part pas et la page le dit. Le serveur démarre sans elles : le journal dit ce qui est éteint, `/api/health` répond 200 dès que la base et la file vont.

**La clé de signature des plugins n'est jamais ni dans Coolify ni dans le dépôt** (`docs/STORE.md`, §4.5) : seules les clés publiques viendront dans `store/keys/*.pub` (STORE-8), copiées dans l'image (`/app/keys`, `STORE_KEYS_DIR` pour un autre dossier).

**L'e-mail de `store@escouade.dev`** : chez Gandi, trois enregistrements pour que les avis ne finissent pas en indésirables (`docs/STORE.md`, §8.1). `npx tsx store/scripts/check-mail-dns.ts --dkim <identifiant du projet Scaleway>` dit ce qui manque (le 2026-10-29 : les trois) :

| Nom | Type | Valeur |
| --- | --- | --- |
| `escouade.dev` | TXT (remplace l'actuel, un seul SPF) | `v=spf1 include:_mailcust.gandi.net include:_spf.tem.scaleway.com ~all` |
| `<identifiant du projet>._domainkey.escouade.dev` | TXT | la clé DKIM que Scaleway affiche sur la page du domaine (Transactional Email) |
| `_dmarc.escouade.dev` | TXT | `v=DMARC1; p=none; rua=mailto:guillaume@gagnaire.dev` pour commencer, `p=quarantine` une fois les rapports propres |

## L'API de l'app

Ce que l'app lit (`docs/STORE.md`, §1.2, §4.6, §5.4 ; STORE-9 la branche), sans cookie ni identifiant, sous `https://store.escouade.dev/api/v1/` :

| Route | Ce qu'elle sert | Cache |
| --- | --- | --- |
| `GET catalog.json` | le catalogue entier (`server/utils/catalog.ts`) : chaque plugin publié, ses versions publiées (les plus hautes d'abord), leur empreinte (`sha256` de `version.json`), leur compatibilité (`escouade.api`, `min`, `max`), ce qu'elles demandent (`permissions`, `run`, `contributes`), leurs systèmes et leurs fichiers ; les installations jusqu'à la veille (`installs_until`), le numéro de la liste de révocation. Une version désactivée par la liste n'y est plus ; une version dont le badge est retiré porte `unverified` | `ETag` (304), gzip ; refait à chaque publication, une minute au plus après un autre changement |
| `GET plugins/<id>/<version>/version.json` | le fichier canonique de la version, à l'octet près | un an, `immutable` |
| `GET` ou `HEAD plugins/<id>/<version>/archive.tar.gz` | l'archive gardée à la soumission (son SHA-256 est celui de `version.json`) | un an, `immutable` ; 410 si la liste désactive la version |
| `GET` ou `HEAD plugins/<id>/<version>/binaries/<système>` | le programme de la release (`windows-x64`, `macos-arm64`…) | idem |
| `GET revocations.json`, `revocations.json.minisig` | la dernière liste de révocation et sa signature, telles que signées (404 avant la première) | `ETag`, `no-cache` |
| `POST installs` | `{ id, version, event: "install" | "update" }`, en JSON seulement : un au compteur du jour de la version (`installs_daily`), rien d'autre | — |
| `GET plugins/<id>/<version>/version.json.minisig` | la signature de la revue (STORE-8), telle que minisign l'a écrite sur le poste de l'utilisateur ; 404 pour une version non vérifiée. Le catalogue en donne le chemin (`files.signature`) et le résumé public (`review.summary`) pour une version `verified` | `ETag`, `no-cache` |

## La file de revue

Pour les seuls comptes de `STORE_ADMINS` (`docs/STORE.md` §4.1, STORE-8) : la page `/review` (connecté), et pour le script de la revue (`npm run plugin-review -- report|sign|revoke|lift|revoke-key`), ces routes avec `Authorization: Bearer <jeton d'administration>`, sans cookie :

| Route | Ce qu'elle fait |
| --- | --- |
| `GET /api/v1/review/queue` | la file : à relire, signalements, noms proches, faites |
| `POST /api/v1/review/report` | `{ id, version, verdict, report }` : le rapport (256 Ko au plus), lu sur `/review` comme du texte |
| `POST /api/v1/review/signature` | `{ id, version, minisig, summary: { fr, en } }` : vérifiée avec les clés de `store/keys/` contre `version.json` avant d'être prise ; 422 sinon, 503 sans clé |
| `POST /api/v1/review/revocations` | `{ json, minisig, notify }` : la liste suivante, signée, `serial` au-dessus du dernier ; 422 sinon |

Le jeton se crée dans le compte (« Jetons d'administration »), est montré une fois et gardé haché ; il n'agit que tant que son compte est dans `STORE_ADMINS`, ne publie ni ne supprime rien. Les décisions de la page (« Demander des changements », « Refuser », « Laisser passer » un nom proche, « Retenu » ou « Classer sans suite » un signalement) passent par la session et l'`Origin` du store (`/api/review/*`).

**Les limites de débit** (`server/utils/limits.ts`), par adresse (la dernière de `X-Forwarded-For`, celle que le proxy de Coolify a vue), comptées en mémoire sous une empreinte HMAC à clé tirée au démarrage, jamais écrite : 300 requêtes de l'API par minute, 300 téléchargements par heure, 60 `POST /installs` par heure (et la même version comptée une fois par jour et par adresse), 60 soumissions, relances ou publications par heure (en plus des 20 soumissions par heure et par compte). Au-delà : 429 et `Retry-After`. Larges exprès : un proxy d'entreprise met beaucoup d'utilisateurs derrière une adresse.

**La liste de révocation** est signée sur le poste de l'utilisateur (jamais sur le serveur) ; le store ne la prend que signée par une clé de `store/keys/`, avec le commentaire de confiance `escouade-revocations-v1 <serial> …` et un `serial` plus grand que le dernier (`acceptRevocations`, appelée par `POST /api/v1/review/revocations`).

**Sa copie sur escouade.dev** : le workflow `.github/workflows/revocations.yml`, toutes les 15 minutes, lance `store/scripts/copy-revocations.ts`, qui lit la liste du store, la vérifie de la même façon (signature, commentaire, `serial` au-dessus de la copie), et l'écrit avec sa signature dans `website/public/` ; le commit sur `main` fait redéployer le site par Coolify, qui la sert à `https://escouade.dev/revocations.json` (et `.minisig`, `no-cache`). Une heure de retard au plus. Une liste qui ne tient pas fait échouer le workflow (GitHub le dit par e-mail) et la copie ne bouge pas ; le store tombé la laisse telle quelle.

## Les sauvegardes

Faites par Coolify, vers le stockage objet de Scaleway (`fr-par`), 30 jours gardés :
- **la base** : le dump de `store-db` (`pg_dump`, format custom) chaque nuit à 3 h ;
- **les archives** : le volume `/data/archives` chaque nuit à 3 h 30.

Rien à lancer dans le conteneur : l'image n'a pas d'outil de sauvegarde.

## Essayer la restauration

À la mise en route, puis une fois par trimestre, sur un poste avec `psql`, `pg_restore` et `tar` (sous Windows : dans WSL), et la date notée dans `docs/STORE.md` (§1.4) :

```sh
# 1. Les deux dernières sauvegardes, téléchargées depuis Coolify (ou le bucket) : le dump et l'archive du volume.
# 2. Une base vide, à part :
createdb -h 127.0.0.1 -U postgres store_restore
# 3. La restauration :
sh store/scripts/restore-try.sh ./store-db.dmp postgres://postgres@127.0.0.1/store_restore ./archives.tar.gz /tmp/store-archives
# 4. Le store sur ce qui est revenu : /api/health doit dire database ok et queue ok.
cd store && npm run build && DATABASE_URL=postgres://postgres@127.0.0.1/store_restore ARCHIVE_DIR=/tmp/store-archives node .output/server/index.mjs
```

Le script refuse une base qui n'est pas vide, restaure le dump (custom ou SQL), liste les tables, leurs lignes et les migrations jouées ; puis il refuse une archive du volume dont un chemin sortirait du dossier (absolu, `..`), l'extrait dans un dossier neuf et donne l'empreinte SHA-256 de chaque archive, à comparer à celles de la base (STORE-3).

**Essai du 2026-10-29** (ESC-228, en local : PostgreSQL 16.15 dans WSL) : une base du store en marche (migrations, file pg-boss, battement) sauvegardée par `pg_dump -Fc` comme Coolify, et deux archives dans un `.tar.gz` du volume ; restaurées par `restore-try.sh` dans une base vide et un dossier neuf : 15 tables revenues avec leurs lignes, 1 migration, les deux archives identiques à l'octet (`sha256sum -c`) ; une seconde restauration refusée sur la base déjà pleine, une archive à chemin absolu refusée sans rien extraire ; le store démarré sur la base restaurée : `database: ok`, `queue: ok`. Reste à le refaire sur les vraies sauvegardes de Coolify.

## La sonde extérieure

Coolify redémarre un conteneur qui ne répond plus, mais un serveur qui tombe ne se signale pas lui-même : une sonde **hors du serveur** lit `https://store.escouade.dev/api/health` (`docs/STORE.md`, §1.4). À poser par l'utilisateur, au choix :

- **Uptime Kuma** sur une autre machine : « Add New Monitor » › HTTP(s) - Keyword, URL `https://store.escouade.dev/api/health`, mot-clé `"status":"ok"`, toutes les 60 s, 3 essais avant l'alerte ; notification « Discord » avec le webhook du salon d'Escouade ;
- ou le palier gratuit d'un service de sondes (UptimeRobot, Better Stack…) : la même URL et le même mot-clé, toutes les 5 min, alerte vers le même webhook Discord.

La réponse ne dit rien de secret : `{"status":"ok","checks":{"database":"ok","queue":"ok","disk":"ok (12 %)"}}` ; 503 et `"down"` quand la base ne répond pas, que la file ne bat plus depuis 3 min ou que le disque des archives atteint 80 %.

## La sécurité, ici

- Aucun secret dans le dépôt : `.env` est ignoré, `.env.example` n'a que des valeurs locales ; un test (`tests/unit/no-secrets.test.ts`) refuse une clé privée, une clé minisign secrète, un jeton GitHub ou AWS, ou une adresse de base avec un mot de passe dans ce que git suit sous `store/`.
- Les tests qui vident une base ne lisent que `TEST_DATABASE_URL` : jamais la base du store, même si le build la voit.
- Les comptes : la session est un jeton aléatoire dans le cookie `__Host-store_session` (`HttpOnly`, `Secure`, `SameSite=Lax`, 30 jours), dont la base ne garde que l'empreinte ; de même pour les liens de vérification d'e-mail (24 h, 5 par heure). Le jeton GitHub de la session est chiffré en AES-256-GCM par une clé tirée de `NUXT_SESSION_SECRET`, gardé 8 h au plus, révoqué chez GitHub à la déconnexion et à la suppression ; le jeton de rafraîchissement n'est pas gardé. Changer `NUXT_SESSION_SECRET` rend illisibles les jetons GitHub en cours (les sessions restent).
- La connexion compare l'état emporté dans un cookie de 10 minutes à celui que GitHub renvoie ; une installation commencée sur GitHub revient sans lui et relance la connexion. Les formulaires ne passent que si leur `Origin` est l'adresse du store (`NUXT_PUBLIC_SITE_URL`). Les webhooks ne passent qu'avec leur signature, comparée en temps constant, et une seule fois par livraison.
- Les en-têtes de sécurité (`server/utils/headers.ts`) sont sur toutes les réponses ; les pages ont une CSP stricte dont le nonce, tiré à chaque réponse (`server/plugins/csp.ts`), marque les seuls scripts et styles en ligne de Nuxt. Aucun fichier inliné en `data:` (`assetsInlineLimit: 0`), aucune image d'un tiers : les avatars sont des initiales.
- Chaque nuit à 4 h 40, la file purge les sessions, jetons, liens, livraisons et entrées du journal qui ont passé leur durée (`docs/STORE.md`, §2.3).
- `/api/health` ne rend ni erreur de la base (elle peut nommer une adresse interne) ni variable : elles vont au journal du conteneur.
- La clé de signature des plugins reste sur le poste de l'utilisateur ; le store ne vérifiera qu'avec les clés publiques (STORE-8). Les empreintes des versions (`version.json`) sont écrites à la soumission.
- La soumission (`server/utils/submissions.ts`) n'exécute jamais rien du plugin : l'archive est lue en mémoire (`server/utils/tar.ts`), sans fichier extrait ; ses téléchargements sont coupés à 50 Mo en route, sa décompression à 200 Mo ; un lien, un fichier spécial, un chemin absolu ou en `..` sont refusés. Les captures sont décodées par sharp, bornées en pixels, et réencodées sans leurs métadonnées. Ce qui est gardé (archives, binaires, captures) l'est sous son SHA-256 et ne change plus ; une version refusée ne garde rien. Le dépôt doit être public selon GitHub au moment de la soumission, et géré par le compte (une installation de l'App qu'il administre).
