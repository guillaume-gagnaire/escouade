# Réglages : le réseau (Avancé)

Le groupe « Réseau » de la page « Avancé » configure le proxy HTTP(S) et les certificats TLS (un fichier d’autorités supplémentaires, la vérification) pour Claude Code, les quotas, les intégrations, les mises à jour et la dictée (le téléchargement de ses modèles et, en ligne, son service).

## Ouvrir la page

Ouvre les réglages avec `Ctrl+,` (`⌘,` sous macOS), puis choisis « Avancé » : « Réseau » est son premier groupe ; « proxy » dans la recherche de la barre y mène aussi. Ces réglages sont ceux de l’app, communs à tous les projets, et s’appliquent au geste.

Ils sont utiles derrière un réseau d’entreprise : un proxy pour sortir, et parfois un certificat que le proxy impose. Sans proxy, tu n’as rien à régler ici.

## Proxy

L’app le dit sous le titre du groupe : « Le proxy est transmis aux processus Claude Code, à la lecture des quotas, aux intégrations et aux mises à jour. Il s’applique aux agents au prochain (re)démarrage de leur processus. »

| Réglage | Effet | Par défaut |
|---|---|---|
| « Proxy HTTP(S) » | L’adresse du proxy, par exemple `http://utilisateur:motdepasse@proxy:3128`. Vide (« aucun »), les connexions sont directes. | Vide |
| « Exclusions » | Les hôtes qui ne passent pas par le proxy, séparés par des virgules (la liste `NO_PROXY`). | `localhost,127.0.0.1` |
| « Appliquer aussi le proxy aux terminaux intégrés » | Les terminaux reçoivent aussi les variables du proxy. | Désactivé |

### Ce qui passe par le proxy

- **Les processus Claude Code** : les agents, et les autres appels à Claude Code que fait Escouade (nom d’un agent, messages de commit, connexion à un compte). Ils reçoivent `HTTPS_PROXY`, `HTTP_PROXY`, leurs variantes en minuscules, et `NO_PROXY` (avec `no_proxy`) si « Exclusions » n’est pas vide. Un agent ne les voit qu’au prochain (re)démarrage de son processus : un agent déjà en cours garde ses réglages jusque-là.
- **La lecture des quotas et les intégrations** (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, Azure Boards) : les requêtes de l’app passent par le proxy et respectent « Exclusions ».
- **Les mises à jour** : la recherche et le téléchargement passent par le proxy. Une adresse de proxy invalide fait échouer la recherche, avec le message « adresse du proxy invalide ». Quand le proxy lui-même ne répond pas (arrêté, ou poste hors du réseau de l’entreprise), Escouade essaie une fois sans proxy et le message d’échec dit les deux essais ; quand le proxy refuse la requête, elle n’est pas refaite sans lui. Voir [« Derrière un proxy d’entreprise »](updates#derriere-un-proxy-d-entreprise).
- **Les terminaux intégrés** : seulement si « Appliquer aussi le proxy aux terminaux intégrés » est activé. Les mêmes variables sont alors ajoutées aux terminaux, aux commandes de lancement et aux commandes de worktree. Voir [les terminaux](terminals).

Sans adresse de proxy, Escouade n’ajoute aucune variable de proxy : les processus gardent l’environnement de ta session.

## Certificats

L’app le dit sous le réglage du fichier : « Escouade fait confiance aux certificats reconnus, à ceux installés sur ce système et à ceux du fichier d’autorités. Les agents reçoivent ce fichier au prochain démarrage de leur processus. »

| Réglage | Effet | Par défaut |
|---|---|---|
| « Autorités de certification supplémentaires » | Un fichier `.pem` (le certificat racine d’un proxy d’entreprise, celui de Zscaler par exemple) auquel Escouade fait confiance en plus du magasin du système. « Choisir… » le prend ; sous le champ, l’app dit combien de certificats elle y lit, ou pourquoi elle ne le lit pas. | Vide |
| « Ignorer la vérification des certificats TLS » | Pour les connexions qui passent par le proxy ci-dessus seulement ; sans proxy, il n’a aucun effet. Gardé pour les installations qui l’ont : préfère le fichier d’autorités. | Désactivé |

### Le fichier d’autorités

Il sert à toutes les connexions d’Escouade : les quotas, les intégrations, les mises à jour, le store, la dictée et le réseau des plugins (leurs requêtes et leurs WebSockets). Les agents le reçoivent au prochain démarrage de leur processus : `NODE_EXTRA_CA_CERTS` pour les harness en Node (Claude Code, Copilot CLI, Cursor CLI, Gemini CLI, OpenCode) et `CODEX_CA_CERTIFICATE` pour Codex. Les commandes que lancent les agents (npm, node…) héritent de ces variables. Les certificats reconnus et ceux du système restent valables : le fichier s’y ajoute.

### Ignorer la vérification

Activé, le réglage ne vaut que pour les connexions qui passent par le proxy configuré : vers un hôte de « Exclusions », ou sans proxy, le certificat est vérifié. Les processus des agents reçoivent `NODE_TLS_REJECT_UNAUTHORIZED=0` seulement quand un proxy est configuré. Tant que la case est cochée, un bandeau en haut de la fenêtre le dit, et son bouton « Choisir le certificat racine » mène au fichier d’autorités.

> [!WARNING]
> Une connexion interceptée, jetons compris, ne serait plus détectée. Si tu emportes ton poste sur un autre réseau avec le même proxy, ses connexions resteraient non vérifiées.

### Passer de la case au fichier (Zscaler)

Derrière Zscaler ou un autre proxy qui inspecte le TLS :

1. Récupère le certificat racine du proxy. Sous Windows, ouvre `certmgr.msc`, « Autorités de certification racines de confiance › Certificats », trouve celui du proxy (« Zscaler Root CA »), puis clic droit, « Toutes les tâches › Exporter… », au format « Codé à base 64 X.509 (.CER) ». Sous macOS, exporte-le depuis « Trousseaux d’accès » au format `.pem`. Ton service informatique peut aussi te le donner.
2. Dans Réglages › Avancé › Réseau, « Autorités de certification supplémentaires », « Choisir… » et prends ce fichier : l’app doit dire « 1 certificat lu ».
3. Décoche « Ignorer la vérification des certificats TLS » : le bandeau disparaît.
4. Redémarre les agents en cours (ou attends leur prochain démarrage) : ils prennent le fichier à ce moment-là.

Si une connexion échoue encore avec « UnknownIssuer », le proxy présente un autre certificat que celui choisi : prends celui que montre le navigateur pour le même site.
