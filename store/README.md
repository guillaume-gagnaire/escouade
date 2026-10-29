# Le store : store.escouade.dev

La place de marché des plugins d'Escouade (`docs/STORE.md`, qui fait foi). Ce dossier en est le service : **Nuxt 4 rendu par le serveur** (les pages en Vue dans `app/`, l'API Nitro dans `server/api/`), **PostgreSQL** par **Drizzle**, la file des tâches longues par **pg-boss** (dans PostgreSQL, pas de Redis), les archives sur un volume. Coolify le redéploie à chaque push sur `main` qui le touche ; la CI (job `store` de `.github/workflows/ci.yml`) le vérifie avant.

Ce qu'il fait aujourd'hui (STORE-1, ESC-228) : l'accueil en français (`/`) et en anglais (`/en`), dans les jetons du site (`website/app/assets/main.css`, lu par un chemin relatif) avec un thème clair qui suit le système ; `GET /api/health` ; les migrations jouées au démarrage ; la file et son battement ; la restauration des sauvegardes de Coolify, essayée. Les comptes (STORE-2), la soumission (STORE-3), les pages des plugins (STORE-4) et l'API de l'app (STORE-6) viennent ensuite.

## Lancer en local

Node 22.19+ ou 24, et un PostgreSQL (16 ou 17) pour la base ; sans base, le serveur démarre quand même, sert les pages, et `/api/health` répond 503 `"database": "missing"`.

```sh
cd store
npm ci
cp .env.example .env      # puis DATABASE_URL vers ta base ; .env n'est jamais commité
npm run dev               # http://localhost:3000
```

Un secret manquant (`GITHUB_APP_*`, `SMTP_URL`, `MAIL_FROM`, `NUXT_SESSION_SECRET`, `STORE_ADMINS`) éteint sa fonction et le dit au démarrage, une ligne par fonction, sans la valeur : `[store] mail off (missing SMTP_URL, MAIL_FROM): no e-mail is sent`.

## Les tests

| Commande | Ce qu'elle vérifie |
| --- | --- |
| `npm run format:check` | Prettier |
| `npm run typecheck` | `nuxi typecheck` |
| `npm test` | Vitest : les variables, `/api/health`, les deux langues, aucun secret dans `store/` ; avec `TEST_DATABASE_URL`, les migrations depuis une base vide et la file. **Cette base est vidée** : jamais une base qui compte. Elle n'est jamais lue de `DATABASE_URL`, que Coolify passe aussi au build |
| `npm run build && npm run test:e2e` | Playwright sur le serveur construit (`STORE_PORT`, 3000 par défaut ; la base de `DATABASE_URL` s'il y en a une) : l'accueil comparé à l'aperçu `design/store/` dans les deux thèmes, les liens légaux, la 404, le téléphone, `/api/health` |

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
| Health Check | `GET /api/health`, port 3000, toutes les 30 s, 60 s de démarrage (l'image a aussi son `HEALTHCHECK`, les mêmes) |
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
| `GITHUB_APP_CLIENT_SECRET`, `GITHUB_APP_PRIVATE_KEY`, `GITHUB_WEBHOOK_SECRET` | idem ; la clé PEM telle quelle, sur plusieurs lignes ; STORE-2 | oui |
| `STORE_ADMINS` | l'identifiant numérique GitHub de l'utilisateur ; STORE-8 | non |
| `SMTP_URL`, `MAIL_FROM` | Scaleway Transactional Email, `store@escouade.dev` ; STORE-2 | oui (`SMTP_URL`) |

Posées le 2026-10-29 : toutes sauf `GITHUB_APP_CLIENT_SECRET`, `SMTP_URL` et `MAIL_FROM`. Le serveur démarre sans elles : le journal dit ce qui est éteint, `/api/health` répond 200 dès que la base et la file vont.

**La clé de signature des plugins n'est jamais ni dans Coolify ni dans le dépôt** (`docs/STORE.md`, §4.5) : seules les clés publiques viendront dans `store/keys/*.pub` (STORE-8).

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
- `/api/health` ne rend ni erreur de la base (elle peut nommer une adresse interne) ni variable : elles vont au journal du conteneur.
- La clé de signature des plugins reste sur le poste de l'utilisateur ; le store ne vérifiera qu'avec les clés publiques (STORE-8). Les empreintes des versions (`version.json`) viennent avec la soumission (STORE-3).
