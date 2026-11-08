# Réglages : le réseau (Avancé)

Le groupe « Réseau » de la page « Avancé » configure le proxy HTTP(S) et la vérification des certificats TLS pour Claude Code, les quotas, les intégrations, les mises à jour et la dictée (le téléchargement de ses modèles et, en ligne, son service).

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

L’app le dit sous le titre du groupe : « Les intégrations, les quotas et les mises à jour font confiance aux certificats reconnus et à ceux installés sur ce système (celui d’un proxy d’entreprise, le plus souvent). Claude Code, lui, a sa propre liste. »

| Réglage | Effet | Par défaut |
|---|---|---|
| « Ignorer la vérification des certificats TLS » | Escouade ne vérifie plus les certificats TLS. Pour un proxy qui déchiffre le trafic avec un certificat non reconnu (l’erreur « UnknownIssuer »). | Désactivé |

Activé, le réglage s’applique aux intégrations, aux quotas et aux mises à jour, et aux processus Claude Code (au prochain démarrage de leur processus) : ils reçoivent `NODE_TLS_REJECT_UNAUTHORIZED=0`, que les commandes lancées par les agents (npm, node…) héritent aussi. Il ne s’applique pas aux terminaux intégrés ni aux commandes de lancement.

> [!WARNING]
> À réserver à un réseau de confiance : une connexion interceptée, jetons compris, ne serait plus détectée. Si ton proxy a son propre certificat, installe-le plutôt sur le système que de désactiver la vérification : les intégrations, les quotas et les mises à jour le reconnaîtront (Claude Code, lui, a sa propre liste).
