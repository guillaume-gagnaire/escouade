# Le store : store.escouade.dev

La place de marché des plugins d'Escouade (`docs/STORE.md`, qui fait foi). Ce dossier en est le service : **Nuxt 4 rendu par le serveur** (les pages en Vue dans `app/`, l'API Nitro dans `server/api/`), **PostgreSQL** par **Drizzle**, la file des tâches longues par **pg-boss** (dans PostgreSQL, pas de Redis), les archives sur un volume. Coolify le redéploie à chaque push sur `main` qui le touche ; la CI (job `store` de `.github/workflows/ci.yml`) le vérifie avant.

Ce qu'il fait aujourd'hui (STORE-1, ESC-228) : l'accueil en français (`/`) et en anglais (`/en`), dans les jetons du site (`website/app/assets/main.css`, lu par un chemin relatif) avec un thème clair qui suit le système ; `GET /api/health` ; les migrations jouées au démarrage ; la file et son battement ; les sauvegardes. Les comptes (STORE-2), la soumission (STORE-3), les pages des plugins (STORE-4) et l'API de l'app (STORE-6) viennent ensuite.

## Lancer en local

Node 22.19+ ou 24, et un PostgreSQL (16 ou 17) pour la base ; sans base, le serveur démarre quand même, sert les pages, et `/api/health` répond 503 `"database": "missing"`.

```sh
cd store
npm ci
cp .env.example .env      # puis DATABASE_URL vers ta base ; .env n'est jamais commité
npm run dev               # http://localhost:3000
```

Un secret manquant (`GITHUB_APP_*`, `SMTP_URL`, `NUXT_SESSION_SECRET`, `STORE_ADMINS`) éteint sa fonction et le dit au démarrage, une ligne par fonction, sans la valeur : `[store] github off (missing GITHUB_APP_ID, …): signing in with GitHub, submissions and webhooks are off`.

## Les tests

| Commande | Ce qu'elle vérifie |
| --- | --- |
| `npm run format:check` | Prettier |
| `npm run typecheck` | `nuxi typecheck` |
| `npm test` | Vitest : les variables, `/api/health`, les deux langues, aucun secret dans `store/` ; avec `DATABASE_URL`, les migrations depuis une base vide et la file (**la base est vidée** : jamais une base qui compte) |
| `npm run build && npm run test:e2e` | Playwright sur le serveur construit (`STORE_PORT`, 3000 par défaut) : l'accueil comparé à l'aperçu `design/store/` dans les deux thèmes, les liens légaux, la 404, le téléphone, `/api/health` |

Le `Dockerfile` rejoue `npm ci && npm run build && npm test` : un test raté fait échouer le déploiement, la version en ligne reste la dernière qui passait.

## Les migrations

Le schéma est `server/db/schema.ts` ; `npm run db:generate -- --name <quoi>` écrit la migration suivante dans `server/db/migrations/`, à commiter avec lui. Elles se jouent au démarrage du conteneur, avant que `/api/health` dise `ok` (pendant ce temps : 503 `"starting"`, et Coolify garde l'ancien conteneur). **Une migration ne fait qu'ajouter** (tables, colonnes, index) : on ne retire ou ne renomme qu'au déploiement suivant, quand plus aucun code ne lit l'ancien nom (`docs/STORE.md`, §1.3).

## L'application Coolify (pas à pas)

Créée par le chef d'escouade ou l'utilisateur après la fusion (`docs/STORE.md`, §1.3). `store.escouade.dev` pointe déjà sur le serveur (62.210.173.28). **Aucun secret ne va dans le dépôt.**

**1. La base `store-db`** : projet « Escouade », environnement `production`, « + New » › Database › PostgreSQL 17. Nom `store-db`, non exposée (pas de port public). Noter son adresse interne (`postgres://…@<uuid>:5432/postgres`) : c'est `DATABASE_URL`.

**2. L'application `store`** : « + New » › Public/Private Repository (l'App GitHub « guillaume-gagnaire » de Coolify, celle du site), ce dépôt, branche `main`.

| Réglage de Coolify | Valeur |
| --- | --- |
| Build Pack | Dockerfile |
| Base Directory | `/` (la racine du dépôt : le build lit `website/app/assets/main.css`) |
| Dockerfile Location | `/store/Dockerfile` (ce que le build voit : `store/Dockerfile.dockerignore`) |
| Watch Paths | `store/**`, `schemas/**`, `website/app/assets/main.css` |
| Auto Deploy | oui |
| Domains | `https://store.escouade.dev` (certificat par Traefik) |
| Ports Exposes | `3000` |
| Health Check | activé, `GET /api/health`, port 3000, toutes les 30 s, statut attendu 200 (l'image a aussi son `HEALTHCHECK`) |
| Persistent Storage | un volume `store-archives` monté sur `/data/archives` |
| Notifications | le salon Discord d'Escouade : déploiement raté, conteneur arrêté, tâche planifiée ratée |

**3. Ses variables** (onglet Environment Variables ; les secrets cochés « secret » / non lisibles, et **décochés de « Build Variable »** : aucun ne sert au build) :

| Variable | Valeur | Secret |
| --- | --- | --- |
| `DATABASE_URL` | l'adresse interne de `store-db` | oui |
| `NUXT_PUBLIC_SITE_URL` | `https://store.escouade.dev` | non |
| `ARCHIVE_DIR` | `/data/archives` (déjà la valeur de l'image) | non |
| `NUXT_SESSION_SECRET` | 32 octets aléatoires (`openssl rand -hex 32`) — STORE-2 | oui |
| `GITHUB_APP_ID`, `GITHUB_APP_CLIENT_ID` | l'App « Escouade Store » — STORE-2 | non |
| `GITHUB_APP_CLIENT_SECRET`, `GITHUB_APP_PRIVATE_KEY`, `GITHUB_WEBHOOK_SECRET` | idem (la clé PEM sur une ligne, retours échappés en `\n`) — STORE-2 | oui |
| `STORE_ADMINS` | l'identifiant numérique GitHub de l'utilisateur — STORE-8 | non |
| `SMTP_URL`, `MAIL_FROM` | Scaleway Transactional Email, `store@escouade.dev` — STORE-2 | oui (`SMTP_URL`) |
| `RESTIC_REPOSITORY` | `s3:https://s3.fr-par.scw.cloud/escouade-store-backups/archives` | oui |
| `RESTIC_PASSWORD` | le mot de passe du dépôt restic (gardé aussi hors ligne : sans lui, la sauvegarde ne se relit pas) | oui |
| `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY` | la clé d'API Scaleway du bucket (droits sur ce seul bucket) | oui |

Au premier déploiement, sans les variables de STORE-2, le journal dit ce qui est éteint et `/api/health` répond 200 dès que la base et la file vont.

**La clé de signature des plugins n'est jamais ni dans Coolify ni dans le dépôt** (`docs/STORE.md`, §4.5) : seules les clés publiques viendront dans `store/keys/*.pub` (STORE-8).

## Les sauvegardes

**La base** : dans `store-db` › Backups, une sauvegarde planifiée `0 3 * * *` (chaque nuit à 3 h) vers une destination S3 de Coolify (Settings › S3 Storages : stockage objet de Scaleway, endpoint `https://s3.fr-par.scw.cloud`, région `fr-par`, bucket `escouade-store-backups`), 30 sauvegardes gardées. Coolify fait un `pg_dump` au format custom (`.dmp`).

**Les archives** : dans l'application `store` › Scheduled Tasks, une tâche :

| Champ | Valeur |
| --- | --- |
| Name | `backup-archives` |
| Command | `backup-archives` (`scripts/backup-archives.sh`, copié dans l'image) |
| Frequency | `30 3 * * *` (chaque nuit à 3 h 30) |
| Container | celui de l'application |

Elle lit les variables `RESTIC_*` et `AWS_*` de l'application, crée le dépôt restic la première nuit, n'envoie que les archives nouvelles (une archive ne change jamais), garde 30 nuits et vérifie le dépôt (`restic check`). Une erreur fait échouer la tâche, que Coolify signale sur Discord.

## Essayer la restauration

À la mise en route, puis une fois par trimestre, sur un poste avec `psql`, `pg_restore` et `restic` (sous Windows : dans WSL), et la date notée dans `docs/STORE.md` (§1.4) :

```sh
# 1. La dernière sauvegarde de la base : la télécharger depuis store-db › Backups (ou le bucket).
# 2. Une base vide, à part :
createdb -h 127.0.0.1 -U postgres store_restore
# 3. Les variables de restic (celles de Coolify, le temps de l'essai), puis :
export RESTIC_REPOSITORY=… RESTIC_PASSWORD=… AWS_ACCESS_KEY_ID=… AWS_SECRET_ACCESS_KEY=…
sh store/scripts/restore-try.sh ./pg-dump-store.dmp postgres://postgres@127.0.0.1/store_restore /tmp/store-archives
# 4. Le store sur la base restaurée : /api/health doit dire database ok et queue ok.
cd store && npm run build && DATABASE_URL=postgres://postgres@127.0.0.1/store_restore ARCHIVE_DIR=/tmp/store-archives/data/archives node .output/server/index.mjs
```

Le script refuse une base qui n'est pas vide, restaure le dump (custom ou SQL), liste les tables et leurs lignes et les migrations jouées, puis restaure le dernier instantané des archives avec `--verify` (chaque fichier relu contre le dépôt restic).

**Essai du 2026-10-29** (ESC-228, en local, PostgreSQL 16.15 et restic 0.16.4 dans WSL) : une base du store en marche (migrations, file pg-boss, battement) sauvegardée par `pg_dump -Fc` comme Coolify, deux archives sauvegardées deux nuits de suite par `backup-archives.sh` ; restaurées par `restore-try.sh` dans une base vide et un dossier neuf : 15 tables revenues avec leurs lignes, 1 migration, les deux archives identiques à l'octet (`sha256sum -c`), une seconde restauration refusée sur la base déjà pleine ; le store démarré sur la base restaurée : `database: ok`, `queue: ok`. Reste à refaire sur les vraies sauvegardes de Coolify après la mise en route.

## La sonde extérieure

Coolify redémarre un conteneur qui ne répond plus, mais un serveur qui tombe ne se signale pas lui-même : une sonde **hors du serveur** lit `https://store.escouade.dev/api/health` (`docs/STORE.md`, §1.4). À poser par l'utilisateur, au choix :

- **Uptime Kuma** sur une autre machine de l'utilisateur : « Add New Monitor » › HTTP(s) - Keyword, URL `https://store.escouade.dev/api/health`, mot-clé `"status":"ok"`, toutes les 60 s, 3 essais avant l'alerte ; notification « Discord » avec le webhook du salon d'Escouade ;
- ou le palier gratuit d'un service de sondes (UptimeRobot, Better Stack…) : la même URL et le même mot-clé, toutes les 5 min, alerte vers le même webhook Discord.

La réponse ne dit rien de secret : `{"status":"ok","checks":{"database":"ok","queue":"ok","disk":"ok (12 %)"}}` (503 et `"down"` quand la base ne répond pas, que la file ne bat plus depuis 3 min ou que le disque des archives passe 80 %).

## La sécurité, ici

- Aucun secret dans le dépôt : `.env` est ignoré, `.env.example` n'a que des valeurs locales ; un test (`tests/unit/no-secrets.test.ts`) refuse une clé privée, une clé minisign secrète, un jeton GitHub ou AWS, ou une adresse de base avec un mot de passe dans ce que git suit sous `store/`.
- `/api/health` ne rend ni erreur de la base (qui peut nommer une adresse interne) ni variable : elles vont au journal du conteneur.
- La clé de signature des plugins reste sur le poste de l'utilisateur ; le store ne vérifiera qu'avec les clés publiques (STORE-8). Les empreintes des versions (`version.json`) viennent avec la soumission (STORE-3).
