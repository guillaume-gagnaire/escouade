# Le serveur client du VPN

Une recette pour faire entrer ton serveur Escouade dans le VPN de l’entreprise par un client en espace utilisateur, sans droits sur la machine, derrière la porte des agents ; et ses réserves, qui la rendent rarement possible.

> [!WARNING]
> Cette voie (V1) n’est pas construite dans Escouade : c’est une recette, avec des outils que tu installes et tiens toi-même. Escouade n’embarque aucun client VPN. Préfère [le connecteur](private-network/connector) ou [le serveur dans le réseau de l’entreprise](private-network/company-server).

## Avant tout, les réserves

- **Un compte VPN pour une machine.** Un VPN d’entreprise donne un compte à une personne, avec une authentification forte (MFA, SAML) et souvent un contrôle du poste. Un compte pour un serveur hors de l’entreprise est l’exception : demande-le au service informatique, ne prends jamais le tien.
- **L’authentification forte casse le H24.** Avec MFA, la session expire (souvent après 8 à 12 heures) et demande un geste : le serveur perd le réseau privé jusque-là, la nuit comprise.
- **Tout ce que le compte VPN joint, un agent le joint.** Rien, côté Escouade, ne borne le réseau de l’entreprise dans cette voie : les listes des projets du [réseau privé](private-network/connector#choisir-la-liste-d-un-projet) ne valent que pour les connecteurs. La seule borne est celle que l’entreprise met sur ce compte, dans son concentrateur VPN ; demande-lui de le limiter aux hôtes dont tes projets ont besoin.

## Le principe

Un client VPN classique veut une interface réseau et des droits d’administration ; l’image du serveur n’en a aucun. Des clients en espace utilisateur font passer le VPN par un proxy local, sans rien demander au système :

```
agents ──▶ la porte (127.0.0.1) ──▶ proxy de l’espace : Privoxy (127.0.0.1:8118)
                                        │ *.acme.internal              │ le reste
                                        ▼                              ▼
                       SOCKS du client VPN (127.0.0.1:1080)        Internet, directement
                                        ▼
                                  VPN d’Acme
```

1. Le client VPN expose un **SOCKS** sur `127.0.0.1` : `wireproxy` pour WireGuard, `openconnect` avec `ocproxy` pour AnyConnect ou GlobalProtect. FortiClient et Zscaler Private Access veulent les droits du système : pas de recette pour eux.
2. **Privoxy** devant lui, un proxy HTTP local, envoie les noms de l’entreprise au SOCKS et le reste directement. La porte d’Escouade ne parle qu’à un proxy en `http://`.
3. Privoxy devient le **proxy de l’espace** du serveur, et la porte le donne à tous les agents, `ssh` compris.

## Le client VPN

### WireGuard, avec wireproxy

Le fichier WireGuard que donne le service informatique, et la configuration de `wireproxy` (sur ton serveur, lisible de son seul compte) :

```ini
WGConfig = /home/escouade/vpn/acme.conf

[Socks5]
BindAddress = 127.0.0.1:1080
```

`wireproxy -c /home/escouade/vpn/wireproxy.conf` ; les noms se résolvent par le DNS que le fichier WireGuard donne (`DNS = …`).

### AnyConnect ou GlobalProtect, avec ocproxy

```sh
openconnect --script-tun --script "ocproxy -D 1080" vpn.acme.example
openconnect --protocol=gp --script-tun --script "ocproxy -D 1080" vpn.acme.example
```

`ocproxy` résout les noms par le DNS du VPN. La connexion demande ton geste à chaque authentification forte.

## Privoxy, le partage

Dans sa configuration (`/etc/privoxy/config` sur un VPS) :

```
listen-address 127.0.0.1:8118
forward-socks5t .acme.internal 127.0.0.1:1080 .
```

Et, pour que `git` en SSH passe (Privoxy borne sinon les ports de `CONNECT`), dans `user.action` :

```
{ +limit-connect{,} }
.acme.internal
```

Le reste d’Internet sort directement, sans passer par le VPN.

## Dans Escouade

1. **La porte** : lance le serveur avec `--gate` (`ESCOUADE_SERVER_GATE=1`). Elle devient le seul proxy des agents, et leur `ssh` passe par elle (`escouade-server via`) ; sans elle, le git en SSH ne passerait pas par le proxy.
2. **Le proxy de l’espace** : connecté à l’espace du serveur, Réglages › Avancé › Réseau, « Proxy HTTP(S) » à `http://127.0.0.1:8118`, « Exclusions » à `localhost,127.0.0.1`. La porte le prend derrière elle, pour tout ce qui ne va pas à un connecteur (voir [le réseau](settings-network)).
3. **Le certificat racine** de l’entreprise dans « Autorités de certification supplémentaires », pour le registre et l’API de recette.

`git clone git@gitlab.acme.internal:acme/api.git` passe alors par la porte, Privoxy, le SOCKS et le VPN ; `npm install` aussi pour le registre.

## En Docker

Le serveur tourne sans aucune capacité, et le doit. Mets le client VPN et Privoxy dans un conteneur à côté qui partage son réseau (`network_mode: "service:escouade"` dans le Compose) : leur `127.0.0.1` est celui du serveur, et l’image du serveur ne change pas. Le conteneur du client n’a pas besoin de droits non plus : c’est tout l’intérêt d’un client en espace utilisateur.

## Le défaire

Vide le « Proxy HTTP(S) » de l’espace, arrête Privoxy et le client VPN. Et dis au service informatique que le compte VPN du serveur peut être fermé.
