# Atteindre un réseau privé

Des agents qui tournent sur un serveur Escouade et doivent joindre un git, un registre de paquets ou une API qu’on ne voit que dans le réseau d’une entreprise : les quatre voies, comment choisir, et ce que voit chacun.

> [!WARNING]
> Le réseau privé est un aperçu de la 2.3, comme le [mode serveur](server). Le connecteur et ses fichiers sont publiés avec la release de la 2.3.

## Le besoin

Ton serveur Escouade tourne sur un VPS (Coolify, Hetzner…), hors du réseau de l’entreprise pour laquelle tu travailles, Acme. Ses services ne sont visibles que dans son réseau, `*.acme.internal`, derrière son VPN. Trois cas reviennent :

| Cas | Le service | Ce que fait l’agent |
|---|---|---|
| Le git | `gitlab.acme.internal`, en SSH (port 22) | `git clone`, `git fetch`, `git push` depuis son worktree ; Escouade fait aussi ses propres `git fetch` |
| Le registre de paquets | `nexus.acme.internal`, en HTTPS (443) | `npm install`, `mvn package`, `./gradlew build`, avec les paquets internes |
| L’API des tests | `api-recette.acme.internal:8443`, en HTTPS | Les tests d’intégration qu’il lance appellent l’API de recette |

Dans chaque voie, les identifiants restent les tiens et vivent sur le serveur : une clé SSH ajoutée au GitLab comme clé de déploiement, le jeton du registre dans `.npmrc` ou `settings.xml`, le jeton de recette dans les variables du lancement de test, et le certificat racine d’Acme dans Réglages › Avancé › Réseau de l’espace du serveur (« Autorités de certification supplémentaires », voir [le réseau](settings-network#certificats)).

Ce qui n’en fait pas partie : que le réseau d’Acme joigne le serveur (un webhook d’une CI interne), de l’UDP ou de l’ICMP (un DNS interne interrogé directement, un `ping`), un partage de fichiers Windows, le bureau à distance d’une machine interne.

## Les quatre voies

| | Ce qu’elle demande à l’entreprise | Ce qu’un agent détourné peut joindre | Dans Escouade |
|---|---|---|---|
| **V0, le serveur dans le réseau de l’entreprise** | Une machine dans son réseau, ses règles de pare-feu, la sortie vers les modèles ; accepter des agents chez elle | Ce que le pare-feu d’Acme permet à cette machine | Rien à construire : [le guide](private-network/company-server) |
| **V1, le serveur client du VPN** | Un compte VPN pour une machine, hors de sa règle le plus souvent | Tout ce que le compte VPN joint | Non construite : [une recette](private-network/vpn-client) |
| **V2, le connecteur** | Installer un programme chez elle, écrire sa liste, permettre une sortie sur le 443. Aucun port entrant | La liste du connecteur, au plus | Construite : [le connecteur](private-network/connector) |
| **V3, le poste en relais** | Rien, ou accepter un usage de son VPN qu’elle n’a pas prévu | La liste du relais, tenue par toi | Non construite |

- **V0** : `escouade-server` tourne sur une machine d’Acme et joint ce que son réseau permet ; tu t’y connectes par SSH à travers ton VPN, comme à tout serveur. La borne est le pare-feu d’Acme, avec ses outils et ses journaux.
- **V1** : le VPS entre lui-même dans le VPN d’Acme, par un client VPN en espace utilisateur qui expose un proxy local.
- **V2** : un petit programme, `escouade-connector`, tourne sur une machine d’Acme. Il ouvre une connexion **sortante** vers ton serveur, et ne laisse passer que les connexions des agents vers une liste d’hôtes que le service informatique écrit lui-même.
- **V3** : ton portable, branché au VPN, relaierait pour le serveur.

## Comment choisir

1. **L’entreprise accepte d’héberger la machine** : V0. C’est la plus simple, rien de neuf chez elle qu’une machine, et la borne est la sienne. C’est aussi la décision la plus lourde à demander à un RSSI : des agents qui exécutent des commandes dans son réseau.
2. **Sinon, le serveur reste chez toi** : le connecteur (V2). Le service informatique installe un programme, écrit la liste de ce qu’il laisse passer et garde la main dessus ; il n’ouvre aucun port entrant. Donne-lui [la fiche pour le service informatique](private-network/it-sheet).
3. **L’entreprise te donne un compte VPN pour le serveur**, et rien d’autre : la recette de V1, avec ses réserves.

## Ce que V1 et V3 ne font pas

- **V1 n’est pas construite.** Escouade n’embarque aucun client VPN : la recette passe par un client en espace utilisateur que tu installes et tiens toi-même. Un compte VPN est d’ordinaire celui d’une personne, avec une authentification forte (MFA) qui expire en quelques heures : le serveur perd alors le réseau privé jusqu’à ton prochain geste, et le H24 avec lui. Et rien ne borne ce qu’un agent joint dans le VPN, sinon ce que l’entreprise borne sur son concentrateur.
- **V3 n’est pas construite, et ne le sera pas dans ce chantier.** Elle ne marcherait que portable allumé, réveillé et sur le VPN, ce qui perd le H24, et ferait servir le VPN d’Acme à une machine qu’Acme ne gère pas, ce que la règle d’un VPN d’entreprise interdit le plus souvent. Une redirection SSH inverse dynamique (`ssh -R`, un SOCKS sur le serveur qui sort par ton poste) n’aurait aucune liste : tout ce que joint ton portable deviendrait joignable par un agent. À ne pas faire.

## Où vont mes données ?

- **Escouade, l’éditeur, ne voit rien du réseau privé**, dans aucune voie : aucun relais à lui, aucune télémétrie du connecteur. Le tunnel va du connecteur à ton serveur, sans passer par nous.
- **Ton serveur voit les noms, les ports, les octets et les durées** des connexions de ses agents, jamais le contenu de ce qui est chiffré de bout en bout entre l’agent et l’hôte interne (SSH, TLS). Une requête en HTTP clair passe par lui sans être lue ni gardée.
- **Le connecteur, chez l’entreprise**, journalise chaque connexion de la même façon, sans le contenu.
- **Ce que les agents lisent dans le réseau privé** (le code du GitLab, les réponses de l’API) reste sur ton serveur, dans leurs worktrees et leurs conversations, comme le reste de leur travail (voir [ce que garde le serveur](server#ce-que-garde-le-serveur)).

## La sécurité, en une phrase

Ce qu’un agent détourné par un contenu piégé peut joindre, c’est ce que le dernier point de passage dans le réseau de l’entreprise laisse passer : son pare-feu dans V0, le plafond du connecteur dans V2, le compte VPN dans V1. La liste d’un projet sur le serveur n’est qu’une défense de plus, une consigne donnée aux agents, pas une cloison.
