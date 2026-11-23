# Réglages : le réseau (Avancé)

Le groupe « Réseau » de la page « Avancé » configure le proxy HTTP(S) et les certificats TLS (un fichier d’autorités supplémentaires, la vérification) pour Claude Code, les quotas, les intégrations, les mises à jour et la dictée (le téléchargement de ses modèles et, en ligne, son service).

## Ouvrir la page

Ouvre les réglages avec `Ctrl+,` (`⌘,` sous macOS), puis choisis « Avancé » : « Réseau » est son premier groupe ; « proxy » dans la recherche de la barre y mène aussi. Ces réglages sont ceux de l’app, communs à tous les projets, et s’appliquent au geste.

Ils sont utiles derrière un réseau d’entreprise : un proxy pour sortir, et parfois un certificat que le proxy impose. Sans proxy, tu n’as rien à régler ici.

## Proxy

L’app le dit sous le titre du groupe : « Le proxy est transmis aux processus Claude Code, à la lecture des quotas, aux intégrations et aux mises à jour. Il s’applique aux agents au prochain (re)démarrage de leur processus. »

| Réglage | Effet | Par défaut |
|---|---|---|
| « Proxy HTTP(S) » | L’adresse du proxy, par exemple `http://utilisateur:motdepasse@proxy:3128`. Vide (« aucun »), les connexions sont directes. Le mot de passe va au trousseau du système : voir [« Le mot de passe du proxy »](#le-mot-de-passe-du-proxy). | Vide |
| « Ne jamais sortir sans le proxy » | Si le proxy ne répond pas, une mise à jour ou un téléchargement échoue avec sa raison au lieu de partir en connexion directe. Grisé sans proxy. | Activé dès qu’un proxy est configuré |
| « Exclusions » | Les hôtes qui ne passent pas par le proxy, séparés par des virgules (la liste `NO_PROXY`). | `localhost,127.0.0.1` |
| « Appliquer aussi le proxy aux terminaux intégrés » | Les terminaux reçoivent aussi les variables du proxy. | Désactivé |

### Ce qui passe par le proxy

- **Les processus Claude Code** : les agents, et les autres appels à Claude Code que fait Escouade (nom d’un agent, messages de commit, connexion à un compte). Ils reçoivent `HTTPS_PROXY`, `HTTP_PROXY`, leurs variantes en minuscules, et `NO_PROXY` (avec `no_proxy`) si « Exclusions » n’est pas vide. Un agent ne les voit qu’au prochain (re)démarrage de son processus : un agent déjà en cours garde ses réglages jusque-là.
- **La lecture des quotas et les intégrations** (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, Azure Boards) : les requêtes de l’app passent par le proxy et respectent « Exclusions ».
- **Les mises à jour** : la recherche et le téléchargement passent par le proxy. Une adresse de proxy invalide fait échouer la recherche, avec le message « adresse du proxy invalide ». Quand le proxy lui-même ne répond pas (arrêté, ou poste hors du réseau de l’entreprise), la mise à jour échoue en le disant, tant que « Ne jamais sortir sans le proxy » est activé ; éteint, Escouade essaie une fois sans proxy et le message d’échec dit les deux essais. Quand le proxy refuse la requête, elle n’est jamais refaite sans lui. Voir [« Derrière un proxy d’entreprise »](updates#derriere-un-proxy-d-entreprise).
- **Les terminaux intégrés** : seulement si « Appliquer aussi le proxy aux terminaux intégrés » est activé. Les mêmes variables sont alors ajoutées aux terminaux, aux commandes de lancement et aux commandes de worktree. Voir [les terminaux](terminals).

Sans adresse de proxy, Escouade n’ajoute aucune variable de proxy : les processus gardent l’environnement de ta session.

### Le mot de passe du proxy

Écris l’adresse avec ton identifiant et ton mot de passe, comme avant. Escouade garde le mot de passe dans le trousseau du système (le Gestionnaire d’identification de Windows, le Trousseau de macOS, le service de secrets du bureau sous Linux), jamais dans `settings.json` : au démarrage suivant, le champ montre l’adresse sans lui (`http://utilisateur@proxy:3128`), et le dit dessous. Les agents et les connexions de l’app reçoivent toujours l’adresse entière. Pour changer le mot de passe, écris l’adresse avec le nouveau ; un autre identifiant, ou une adresse sans identifiant, le retire du trousseau.

Un mot de passe qu’une version précédente avait écrit dans `settings.json` y est retiré au premier démarrage, et va au trousseau. Sans trousseau (Linux sans service de secrets, WSL), il reste dans `settings.json`, à part de l’adresse, lisible par toi seul.

### Ne jamais sortir sans le proxy

Activé dès qu’un proxy est configuré, tant que tu ne l’éteins pas. Si le proxy ne répond pas, la recherche ou le téléchargement d’une mise à jour échoue avec sa raison, par exemple « connexion impossible à github.com par le proxy 127.0.0.1:9000 (…) ; rien n’est tenté sans le proxy : « Ne jamais sortir sans le proxy » est allumé (Réglages › Avancé › Réseau) ». Rien ne part en connexion directe, ce qu’attend une entreprise dont toute sortie passe par le proxy. De même, si le mot de passe du proxy n’est plus dans le trousseau au démarrage, les agents ne démarrent pas et le disent, jusqu’à ce que tu réécrives l’adresse avec son mot de passe. Éteins-le pour qu’Escouade retente une fois sans proxy quand le tien est injoignable (un portable hors du réseau de l’entreprise, par exemple) ; ce qu’elle télécharge reste vérifié par sa signature.

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

## Mode hors ligne

« Mode hors ligne », en bas du groupe « Réseau », coupe d’un geste les quatre flux qu’Escouade lance de lui-même : c’est ce qu’il faut connaître avant de régler un pare-feu.

| Flux coupé | Ce qu’il fait d’habitude |
|---|---|
| Les mises à jour | La recherche d’une nouvelle version de l’app, toutes les 5 minutes, et celle des plugins installés, au démarrage. |
| Le store | La lecture du catalogue de store.escouade.dev, au démarrage puis toutes les 6 heures. |
| La liste de révocation | Lue avec le catalogue, avec sa copie et son attestation sur escouade.dev. |
| Les statistiques | Les statistiques d’usage anonymes (si tu les as acceptées), chaque minute, et le compte des installations du store. Pendant le mode, rien n’est gardé pour partir plus tard. |

Tant qu’il est allumé, la barre d’état dit « Hors ligne » ; un clic ouvre le réglage. « Rechercher des mises à jour » et « Chercher maintenant » le disent au lieu de chercher : « Mode hors ligne : offres du jj/mm hh:mm ». Le catalogue et la liste de révocation gardés sur le poste continuent de s’appliquer.

Ce que tu demandes d’un geste n’est pas coupé : installer un plugin, afficher une image d’un message. Le travail des agents non plus, ni `git fetch`, ni la lecture des quotas.

### Imposé par le poste

Un poste géré peut l’imposer par la stratégie `network.offline` (vrai). Le réglage est alors allumé et grisé, avec « Imposé par la stratégie de ce poste. », quoi que dise le réglage. Escouade la lit :

- sous Windows, dans `HKLM\SOFTWARE\Policies\Escouade` puis `HKCU\SOFTWARE\Policies\Escouade` : une valeur `REG_DWORD` nommée `network.offline`, à 1 ;
- sous macOS, dans les préférences gérées du domaine `dev.gagnaire.escouade` qu’un profil de configuration installe (`/Library/Managed Preferences/`) : la clé `network.offline`, un booléen ;
- sous Linux, dans `/etc/escouade/policies/*.json` : `{"network.offline": true}`, le dossier et les fichiers à root, écrits par root seul.

Une stratégie posée pendant que l’app tourne s’applique dans la minute.

## Images distantes

Une image d’un message qui vient du web ne s’affiche qu’au clic, et jamais depuis ta machine ni ton réseau local : une adresse comme `127.0.0.1`, `192.168.…` ou `10.…`, `localhost`, ou un nom qui y mène, même derrière un proxy. Le message le dit : « Une image de cette machine ou du réseau local ne s’affiche pas ».
