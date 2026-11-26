# Mode serveur

Faire tourner Escouade H24 sur un serveur sans bureau, un VPS ou un conteneur, et le piloter depuis ton poste : l’image Docker, Compose, Coolify, le service systemd, l’appairage, l’accès, la sauvegarde, les mises à jour et ce que garde le serveur.

> [!WARNING]
> Le mode serveur est un aperçu de la 2.3. Le programme `escouade-server`, son image et son script d’installation sont publiés avec la release de la 2.3 ; l’écran « Se connecter à un serveur » du client arrive avec elle.

## Ce que fait le serveur

`escouade-server` est le cœur d’Escouade sans fenêtre : les projets, les agents et leurs harness, les worktrees, le Kanban et le pilote auto, les terminaux, le serveur MCP. Il tourne sur le serveur, et Escouade sur ton poste n’en est qu’une vue : fermer le client, éteindre le poste ou couper le réseau ne touche à aucun agent.

Il sert l’API distante d’Escouade sur `127.0.0.1`, port `46480`, et rien d’autre : il ne s’expose jamais lui-même. On l’atteint par un tunnel SSH (voir [Se connecter depuis ton poste](#se-connecter-depuis-ton-poste)). Il existe pour Linux x64 et arm64.

## Avec Docker

L’image `ghcr.io/guillaume-gagnaire/escouade/escouade-server` (linux/amd64 et linux/arm64) est prête à l’emploi. En trois commandes, sur le serveur :

```sh
docker volume create escouade-data
docker run -d --name escouade --restart unless-stopped --stop-timeout 60 \
  -e ESCOUADE_SERVER_LISTEN=0.0.0.0 -p 127.0.0.1:46480:46480 \
  -v escouade-data:/data --read-only --tmpfs /tmp --cap-drop ALL --security-opt no-new-privileges \
  ghcr.io/guillaume-gagnaire/escouade/escouade-server:2.3
docker exec escouade escouade-server pair
```

La dernière affiche le code d’appairage (voir [L’appairage](#l-appairage)).

- **Ce qu’elle contient.** Debian 13 mince, `escouade-server` et l’amorce de ses plugins, `git`, `openssh-client`, `curl`, `tini` (qui recueille les processus orphelins des harness), Node.js et npm (pour les harness qui s’installent par `npm install -g`). Elle pèse 427 Mo décompressée, environ 150 Mo à télécharger. Aucun harness ni aucun compte : tu les installes et les connectes depuis Escouade.
- **Un utilisateur sans privilège.** Elle tourne en `escouade` (uid 10001), sans root ni sudo, avec un système de fichiers en lecture seule (`--read-only`) et sans aucune capacité (`--cap-drop ALL`).
- **Tout dans `/data`.** `HOME` est `/data` : les données d’Escouade (`/data/.escouade`), tes projets et leurs worktrees (clone-les dans `/data/projects`), les harness et leurs connexions (`/data/.local`, `/data/.npm-global`, `/data/.claude`…). Rien n’est écrit ailleurs, hormis `/tmp`, en mémoire.
- **Rien d’exposé par défaut.** Sans `ESCOUADE_SERVER_LISTEN`, l’API n’écoute que sur `127.0.0.1` du conteneur : même un port publié n’y mène pas. `ESCOUADE_SERVER_LISTEN=0.0.0.0` l’ouvre au réseau du conteneur, pour le port publié sur `127.0.0.1` de l’hôte seulement. L’API y est alors **en clair** (HTTP et WebSocket sans TLS) : tout autre conteneur de ce réseau (sur Coolify, les autres ressources du même réseau) l’atteint et peut tenter des codes d’appairage. Le TLS est celui du proxy : mets-en un devant (le Traefik de Coolify, Caddy) ou passe par un tunnel SSH, jamais un port publié sur une adresse publique.
- **Sa santé.** Son `HEALTHCHECK` lance `escouade-server health`, qui vérifie que le serveur tourne et écrit son état. `docker ps` dit « healthy ».
- **Son arrêt.** `docker stop` envoie SIGTERM : les tours en cours sont coupés proprement, l’état est écrit, et chaque agent reprend sa session au démarrage suivant. Donne-lui le temps (`--stop-timeout 60`).
- **Au premier démarrage**, sur un volume vide, le journal (`docker logs escouade`) dit quoi faire : lancer `docker exec <conteneur> escouade-server pair`. Il ne dit jamais le code lui-même.

Un volume monté depuis un dossier de l’hôte (`-v /srv/escouade:/data`) doit appartenir à l’uid 10001 : `sudo chown 10001:10001 /srv/escouade`.

## Avec Docker Compose

Le dépôt donne un exemple, `deploy/docker-compose.yml` : le port publié sur `127.0.0.1` de l’hôte seulement, le système de fichiers en lecture seule, aucune capacité, les limites conseillées.

```sh
curl -fsSLO https://raw.githubusercontent.com/guillaume-gagnaire/escouade/main/deploy/docker-compose.yml
docker compose up -d
docker compose exec escouade escouade-server pair
```

## Sur Coolify

`deploy/coolify/docker-compose.yml` est le Compose tel que Coolify le prend : une ressource « Docker Compose », ce fichier collé tel quel. Le code d’appairage se demande depuis le terminal de la ressource : `escouade-server pair`.

Tel quel, rien ne va sur Internet : l’API est publiée sur `127.0.0.1` du serveur de Coolify, et tu l’atteins par un tunnel SSH vers lui. Le même fichier propose, en commentaire, un domaine derrière le proxy de Coolify (Traefik, en TLS) : deux lignes à décommenter, `SERVICE_FQDN_ESCOUADE_46480` et `ESCOUADE_SERVER_ALLOW_HOSTS`, et le `ports` à retirer. Lis d’abord [ses risques](#un-proxy-tls-devant).

## Sans Docker, avec systemd

`install.sh` pose le programme et un service systemd, sur Linux x64 ou arm64 avec systemd (Debian 12, Ubuntu 22.04 et plus récents).

```sh
curl -fsSLO https://github.com/guillaume-gagnaire/escouade/releases/latest/download/install.sh
less install.sh
sh install.sh
```

- **Ce qu’il vérifie.** Il télécharge l’archive de la release pour ta machine, vérifie sa somme SHA-256 et sa signature, faite par la même clé que les mises à jour de l’app (avec `minisign` s’il est là, sinon `openssl`). Une archive dont la signature ne tient pas est refusée, et rien n’est changé.
- **Un service de ton utilisateur** par défaut (`systemctl --user`) : le programme dans `~/.local/share/escouade-server`, un lien dans `~/.local/bin`, le service dans `~/.config/systemd/user`. Il active `loginctl enable-linger`, pour que le service tourne aussi sans session ouverte.
- **Un service du système** avec `sudo sh install.sh --system` : il crée l’utilisateur `escouade` (dossier `/var/lib/escouade`) s’il n’existe pas, et le service le lance. `--user <nom>` choisit un autre utilisateur.
- **Relancé**, il ne change que ce qui diffère (une autre version, un autre service) ; à la même version, le serveur n’est pas redémarré. `--version X.Y.Z` vise une version précise.
- **Le désinstaller** : `sh install.sh --uninstall` retire le service et le programme, et garde les données. `--uninstall --purge` retire aussi `~/.escouade` ; avec `--system`, l’utilisateur `escouade` et tout son dossier (les harness et leurs connexions compris).

À la fin, il dit comment appairer ton poste. Avec `--system`, les commandes se lancent en tant que l’utilisateur du service : `sudo -u escouade -H escouade-server pair`.

## L’appairage

1. Sur le serveur : `escouade-server pair` (en Docker : `docker exec escouade escouade-server pair`). Il affiche un code, comme `8NUC-W6HS`, valable une fois et 10 minutes ; 5 essais faux y mettent fin.
2. Dans Escouade, sur ton poste : « Se connecter à un serveur », l’hôte SSH, puis ce code et un nom pour l’appareil.
3. Le poste reçoit son propre jeton, qu’il garde. `escouade-server devices` liste les appareils appairés, `escouade-server devices revoke <id>` en révoque un : ses connexions se ferment en quelques secondes.

Ni le code ni le jeton ne sont jamais écrits dans le journal.

## Se connecter depuis ton poste

### Le tunnel SSH

C’est l’accès conseillé. Un serveur s’administre déjà par SSH, avec tes clés :

```sh
ssh -N -L 46480:127.0.0.1:46480 <ton serveur>
```

Escouade sur ton poste vise alors `127.0.0.1:46480`, chez toi, que le tunnel porte jusqu’au serveur. Rien de nouveau n’est ouvert sur le serveur : en Docker, le port n’est publié que sur `127.0.0.1` de l’hôte (`-p 127.0.0.1:46480:46480`), jamais sur son adresse publique.

Ses risques : qui a un compte SSH sur le serveur, ou un processus de l’hôte, atteint le port local et peut tenter de s’appairer (sans code valide, c’est refusé). La sécurité de l’accès est celle de ton SSH : des clés, pas de mot de passe.

### Un proxy TLS devant

Le serveur peut aussi être joint par un domaine, derrière un proxy qui tient le TLS (Traefik de Coolify, Caddy, nginx). Le serveur écoute sur le réseau du conteneur (`ESCOUADE_SERVER_LISTEN=0.0.0.0`) et accepte le nom du domaine dans l’en-tête `Host` (`ESCOUADE_SERVER_ALLOW_HOSTS=escouade.example.org`, ou `--allow-host`) ; tout autre nom reste refusé. Une requête qui donne deux fois `Host` ou `Origin`, ou dont l’adresse complète nomme un autre hôte que son `Host`, est refusée : le proxy et le serveur lisent toujours le même nom.

Ses risques : l’API est alors sur Internet. Qui atteint le domaine peut tenter de s’appairer (un code de 40 bits, valable 10 minutes, une fois, 120 essais lus par minute au plus) ou de deviner un jeton (256 bits) ; chaque essai est dans le journal. Un code faux n’annule jamais le tien, et les essais venus d’Internet ne bloquent jamais celui que tu tapes sur le serveur ou par SSH. Déclare le proxy (`--trust-proxy <son adresse>`, ou `ESCOUADE_SERVER_TRUST_PROXIES`) : chaque client derrière lui compte alors ses essais ratés à part, d’après l’en-tête `X-Forwarded-For` que le proxy écrit ; sinon, tous comptent comme une seule source. Une faille du serveur serait exposée à tous : réserve ce mode à un domaine filtré par adresse IP sur le proxy, ou à un réseau privé (Tailscale, WireGuard). Jamais sans TLS : l’API ne chiffre rien elle-même. Le client d’Escouade se connecte aujourd’hui par le tunnel SSH ; l’adresse d’un domaine (`wss://`) vient avec son écran.

## Les harness sur le serveur

Les agents utilisent les harness installés sur le serveur. Escouade les installe par la commande de leur plugin, comme sur ton poste : pour Claude Code, `curl -fsSL https://claude.ai/install.sh | bash`, qui le pose dans `~/.local/bin` (en Docker : `/data/.local/bin`, dans le volume) ; Codex, Copilot CLI, Gemini CLI et OpenCode s’installent par `npm install -g` (en Docker, dans `/data/.npm-global`). Ils survivent à une nouvelle image.

La connexion d’un compte s’ouvre sur ton poste : l’adresse de connexion du harness s’ouvre dans ton navigateur, et le compte reste sur le serveur.

## Ce que garde le serveur

Tout ce qu’Escouade garde sur un poste, il le garde sur le serveur : les projets, les conversations, le Kanban, les statistiques (voir [Données locales](data)). Deux différences :

- **Les identifiants des comptes vivent sur le serveur.** Les harness s’y connectent (Claude Code, Codex…) et y gardent leurs jetons. La sécurité du serveur devient celle de tes comptes : son disque, son accès SSH, ses sauvegardes.
- **Les secrets sont dans un fichier.** Un serveur n’a pas de trousseau : les jetons des appareils, les clés des intégrations, de Cursor, des voix en ligne et des plugins sont dans `~/.escouade/secrets.json`, lisible de son seul propriétaire (droits 0600, remis à 0600 au démarrage s’ils ne le sont plus).

## Sauvegarder

Tout est dans le volume (`/data`), ou dans le dossier de l’utilisateur du service (`~/.escouade` et les dossiers des harness). Sauvegarde-le, serveur arrêté, pour un état cohérent :

```sh
docker stop escouade
docker run --rm -v escouade-data:/data -v "$PWD":/backup debian:trixie-slim tar -czf /backup/escouade-data.tar.gz -C /data .
docker start escouade
```

Une sauvegarde contient les connexions des comptes et les secrets : garde-la chiffrée, comme le serveur.

## Mettre à jour

- **En Docker**, la mise à jour est une nouvelle image : `docker compose pull && docker compose up -d` (ou `docker pull`, puis le conteneur recréé sur le même volume). Les agents reprennent leurs sessions. `escouade-server update` refuse de tourner dans l’image.
- **Avec systemd** : `escouade-server update`. Il lit la dernière version (`--to X.Y.Z` pour une version précise, `--beta` pour la plus haute du canal bêta et du canal stable, `--check` pour seulement la dire), télécharge l’archive, vérifie sa signature avec la clé des mises à jour de l’app et que cette signature nomme bien cette archive, puis pose chaque fichier à la place de l’ancien, gardé en `.old`. Il attend ensuite que le serveur soit au repos (aucun tour en cours, aucune question en attente, aucune tâche de fond, aucun terminal utilisé dans les 5 dernières minutes) ; dans un terminal, il propose de redémarrer tout de suite en coupant les tours, et `--now` le fait sans attendre. systemd relance le serveur sur la nouvelle version. Il ne revient jamais à une version plus ancienne que la sienne, sauf nommée : `--to X.Y.Z --downgrade`.

Quand la version de ton poste et celle du serveur diffèrent, le serveur refuse la connexion et dit lequel mettre à jour ; pour le serveur, il donne la commande qui le met à la version du poste (`escouade-server update --to <version>`, ou la nouvelle image en Docker), que le client te propose.

## Les limites conseillées

Mesuré sur l’image, sur un poste x64 : le serveur au repos, ses plugins chargés, prend environ 235 Mo et 106 tâches (processus et threads) ; un agent du faux `claude` des tests en plein tour en ajoute environ 18 Mo et 7. Un vrai harness pèse plus : compte plusieurs centaines de Mo par agent qui travaille, et ce que lancent ses commandes (builds, tests). L’exemple Compose fixe `mem_limit: 4g` et `pids_limit: 2048` : de quoi faire travailler quelques agents à la fois ; monte-les avec le nombre d’agents en parallèle. Sur un serveur, le Kanban prend jusqu’à 64 tickets à la fois (16 sur un poste) ; au-delà de 6, Réglages › Kanban dit ce que tient la machine du serveur.

## Les commandes

| Commande | Ce qu’elle fait |
|---|---|
| `escouade-server` ou `escouade-server run` | Le serveur, jusqu’à SIGTERM |
| `escouade-server pair` | Un code d’appairage pour un nouvel appareil |
| `escouade-server devices` | Les appareils appairés |
| `escouade-server devices revoke <id>` | Révoque un appareil |
| `escouade-server health` | Code 0 si le serveur tourne et répond, 1 sinon (le `HEALTHCHECK` de l’image) |
| `escouade-server update` | Met le serveur à la dernière version signée (`--to`, `--beta`, `--now`, `--check`) |

Chaque option a sa variable d’environnement ; l’option passe devant la variable.

| Option | Variable | Par défaut |
|---|---|---|
| `--data-dir <dossier>` | `ESCOUADE_SERVER_DATA` | `~/.escouade` |
| `--port <n>` | `ESCOUADE_SERVER_PORT` | `46480` |
| `--listen <adresse>` | `ESCOUADE_SERVER_LISTEN` | `127.0.0.1` |
| `--allow-host <nom>` | `ESCOUADE_SERVER_ALLOW_HOSTS` (noms séparés par des virgules) | aucun |
| `--trust-proxy <adresse>` | `ESCOUADE_SERVER_TRUST_PROXIES` (adresses séparées par des virgules) | aucun |
| `--debug` | aucune | éteint |

`--listen` sur une autre adresse que `127.0.0.1` avertit dans le journal : ce port ne doit jamais être ouvert sur Internet.
