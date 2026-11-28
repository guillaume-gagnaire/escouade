# Escouade dans le réseau de ton entreprise

Faire tourner le serveur Escouade sur une machine de l’entreprise, qui joint ce que son réseau permet, et t’y connecter par SSH à travers ton VPN : la machine, le proxy de sortie, le certificat racine, ce que fait le service informatique.

## Quand la prendre

C’est la voie V0 de [Atteindre un réseau privé](private-network/introduction) : rien de neuf à installer chez l’entreprise qu’une machine, et la borne est son pare-feu, avec ses outils, ses journaux et sa prévention des fuites. Son proxy de sortie voit ce que les agents envoient sur Internet.

Elle demande à l’entreprise d’héberger des agents qui exécutent des commandes chez elle. Si elle refuse, prends [le connecteur](private-network/connector).

## Ce que fait chacun

| Toi | Le service informatique |
|---|---|
| Demander une machine Linux, x64 ou arm64 (une VM) | La fournir, sur un segment de son réseau |
| Y installer `escouade-server` : le [service systemd](server#sans-docker-avec-systemd) ou [l’image](server#avec-docker) | Ouvrir, depuis ce segment, le GitLab, le registre et l’API de recette |
| T’y connecter par SSH à travers ton VPN | Ouvrir la sortie vers Internet pour les API des modèles, par son proxy |
| Régler le proxy et le certificat racine dans l’espace du serveur | Donner l’adresse de son proxy et son certificat racine (celui d’une inspection TLS) |
| Mettre sur le serveur les identifiants des trois cas (clé SSH, jeton du registre, jeton de recette) | Accepter que des agents tournent dans son réseau |

## Installer le serveur

Comme sur un VPS : voir [le mode serveur](server). Deux choses changent :

- **Le téléchargement passe par le proxy de l’entreprise.** `install.sh` et `docker pull` suivent `HTTPS_PROXY` : lance-les avec la variable posée (`HTTPS_PROXY=http://proxy.acme.internal:3128 sh install.sh`), ou règle le proxy de Docker sur la machine. Si le proxy inspecte le TLS, son certificat racine doit être dans le magasin de la machine (sous Debian, un fichier `.crt` dans `/usr/local/share/ca-certificates`, puis `update-ca-certificates`).
- **La mise à jour automatique** du serveur lit et télécharge ses versions par ce même chemin : le proxy doit laisser passer `github.com` et ses téléchargements.

## Se connecter à travers le VPN

Ton poste, branché au VPN, joint la machine par SSH comme tout serveur : voir [Se connecter à un serveur](connect-server). Un hôte dans `~/.ssh/config`, avec un bastion s’il le faut :

```
Host acme-escouade
  HostName escouade.acme.internal
  User escouade
  ProxyJump bastion.acme.internal
```

Escouade lance le `ssh` de ton poste, avec ta config, ton agent et tes hôtes connus : le `ProxyJump` suit. Le VPN coupé, la connexion tombe, et Escouade la relance seul (voir [quand la connexion tombe](connect-server#quand-la-connexion-tombe)) ; les agents, eux, continuent sur le serveur.

Ton téléphone ne joint ce serveur que par le VPN de l’entreprise.

## Le proxy de sortie et le certificat racine

Les harness joignent leurs modèles sur Internet : par le proxy de l’entreprise, s’il est la seule sortie. Connecté à l’espace du serveur, ouvre Réglages › Avancé › Réseau : ces réglages sont ceux du serveur, et vont à ses agents (voir [le réseau](settings-network)).

| Réglage | Ce que tu y mets |
|---|---|
| « Proxy HTTP(S) » | `http://proxy.acme.internal:3128`, avec identifiant et mot de passe si le proxy en demande (voir [le mot de passe du proxy](settings-network#le-mot-de-passe-du-proxy)). |
| « Exclusions » | Ce qui ne passe pas par le proxy : `localhost,127.0.0.1,.acme.internal`, pour que le GitLab, le registre et l’API soient joints directement |
| « Ne jamais sortir sans le proxy » | Allumé : rien ne part en connexion directe si le proxy ne répond pas |
| « Autorités de certification supplémentaires » | Le certificat racine de l’entreprise, en `.pem` : les harness en Node le reçoivent en `NODE_EXTRA_CA_CERTS`, Codex en `CODEX_CA_CERTIFICATE`, et les commandes des agents en héritent |
| « Appliquer aussi le proxy aux terminaux intégrés » | Allumé si tu lances `npm` ou `git` à la main dans les terminaux du serveur |

Les agents reçoivent ces réglages au prochain démarrage de leur processus.

> [!NOTE]
> Le fichier d’autorités d’Escouade va aux harness et à ce qui lit `NODE_EXTRA_CA_CERTS`, pas à tout. `git` en HTTPS, Python et Java lisent le magasin de la machine : mets-y aussi le certificat racine, ou, en Docker, monte-le dans l’image et règle `GIT_SSL_CAINFO`, `REQUESTS_CA_BUNDLE` ou le magasin de Java.

## Les trois cas

- **Le git en SSH** : la clé de déploiement sur le serveur (dans `~/.ssh`, en Docker `/data/.ssh`), la clé d’hôte du GitLab dans `known_hosts`, vérifiée une fois à la main (`ssh -T git@gitlab.acme.internal`). Le GitLab est joint directement : aucun proxy pour SSH.
- **Le registre** : le jeton dans `.npmrc` (`//nexus.acme.internal/repository/npm/:_authToken=…`) ou `settings.xml`. Maven et Gradle ne suivent pas `HTTPS_PROXY` : pour Maven Central par le proxy, une section `<proxies>` dans `settings.xml`, ou `systemProp.https.proxyHost` dans `gradle.properties`, que l’étape de préparation d’un projet peut écrire.
- **L’API des tests** : son jeton dans les variables du lancement de test, jamais dans le dépôt.

## Ce qu’un agent peut joindre

Ce que le pare-feu permet à cette machine, et Internet par le proxy de l’entreprise. Un agent détourné qui voudrait sortir des données passe par le proxy qui les surveille déjà. Demande au service informatique de borner le segment de la machine aux trois hôtes dont tes projets ont besoin, plutôt que tout le réseau.
