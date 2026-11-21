---
title: "Atteindre un réseau privé"
subject: "a-distance"
sources: ["docs/PRIVATE-NETWORK.md (ESC-551, l'étude RP-0 ; § 5 les choix R1 à R12, § 6 les tickets RP-1 à RP-9)","Réponse de l'utilisateur du 2026-11-21 (la demande a-22 de la boîte « À toi », ESC-587) : « A · Les recommandations : V0 guidée, V2 construite »","docs/REMOTE.md : « Le mode serveur », MS-1 (ESC-514, l'appairage), MS-3 (ESC-515), « Ce qui change de machine, MS-4 » (ESC-538), « Le téléphone », « Les agents sur le serveur, en auto-hébergé »","docs/SERVER-UX.md","docs/TEAM-SERVER.md § 13","docs/security/audit-2026-11/rapport.md, § 6.1 et SEC-02","src-tauri/src/fence.rs","crates/escouade-model/src/model.rs (proxy_env, network_env)","crates/escouade-plugins/src/procs.rs (variables refusées aux plugins)","Demande de l'utilisateur du 2026-11-20 (ESC-551) : des agents sur un serveur Escouade qui joignent un git et des services derrière le VPN d'une entreprise"]
tickets: ["ESC-551","ESC-587","ESC-597","ESC-598","ESC-599","ESC-600","ESC-601","ESC-602","ESC-603","ESC-604","ESC-605"]
decisions: ["reseau-prive-r1-le-serveur-dans-l-entrep","reseau-prive-r2-le-connecteur-parle-par","reseau-prive-r3-une-entree-des-connecteu","reseau-prive-r4-un-plafond-au-connecteur","reseau-prive-r5-la-liste-d-un-projet-est","reseau-prive-r6-l-agent-emprunte-la-port"]
compared: null
---

> **Décidé, pas encore construit.** L'utilisateur a choisi le 2026-11-21 (ESC-587) les recommandations de l'étude RP-0 (`docs/PRIVATE-NETWORK.md`, ESC-551) : « V0 guidée, V2 construite ». Les choix R1 à R12 ci-dessous sont donc ceux de l'étude ; R1 à R6 ont chacun une décision proposée. La construction passe par les tickets RP-1 à RP-9 (ESC-597 à ESC-605), qui mettront cette page à jour à mesure.

## Ce que c'est

Des agents qui tournent sur un serveur Escouade (voir *Le mode serveur*) joignent des services qui ne sont visibles que dans le réseau d'une entreprise, derrière son VPN : un git en SSH (`gitlab.acme.internal:22`), un registre de paquets en HTTPS (`nexus.acme.internal:443`), une API appelée par leurs tests (`api-recette.acme.internal:8443`). Ils ne joignent que les hôtes d'une liste, et l'entreprise tient cette liste.

## Les voies (R1)

- **V0, le serveur dans le réseau de l'entreprise** : guidée, rien à construire. `escouade-server` sur une machine de l'entreprise joint ce que son réseau permet ; le client s'y connecte par SSH à travers le VPN de l'utilisateur. Le guide le dit, avec le proxy de sortie de l'entreprise et son certificat racine réglés dans l'espace du serveur. La meilleure quand l'entreprise l'accepte.
- **V2, `escouade-connector`** : **construite**, pour un serveur hors de l'entreprise. Un programme que l'entreprise fait tourner chez elle ouvre une connexion **sortante** vers le serveur et ne laisse passer que les connexions des agents vers sa liste. Aucun port entrant chez l'entreprise.
- **V1, le serveur client du VPN** : non construite ; le guide donne une recette par un SOCKS en espace utilisateur, derrière la porte.
- **V3, le poste en relais** : non construite (elle perd le H24 et contourne la règle du VPN).

## Les choix décidés

- **R1, les voies** : V0 guidée, V2 construite, V1 en recette du guide, V3 non construite (décision proposée `reseau-prive-r1-le-serveur-dans-l-entrep`).
- **R2, le protocole** : une WebSocket sur TLS, sortante, avec à l'intérieur une poignée de main Noise IK dont les deux clés sont épinglées à l'appairage, puis des flux « ouvre `hôte:port` » à contrôle de débit ; une crate `escouade-connector-proto` partagée ; le serveur accepte la version du connecteur et les deux mineures d'avant (`reseau-prive-r2-le-connecteur-parle-par`).
- **R3, l'entrée du serveur** : un port à part (`--connector-listen`, `ESCOUADE_SERVER_CONNECTOR_LISTEN`), éteint par défaut, qui ne parle que le protocole des connecteurs, jamais l'API distante ; la stratégie `servers.privateNetwork` à `false` l'empêche (`reseau-prive-r3-une-entree-des-connecteu`).
- **R4, où se tient la liste** : un **plafond** au connecteur, écrit par l'entreprise, que le serveur ne peut pas élargir, et une **liste par projet** au serveur, dans ce plafond, vide par défaut (`reseau-prive-r4-un-plafond-au-connecteur`).
- **R5, qui change la liste d'un projet** : l'utilisateur, dans Réglages du projet › Réseau privé ; le `.escouade.yml` du dépôt la propose, accordée par son empreinte ; une stratégie ou le `.escouade.yml` d'équipe peuvent la fixer ; jamais un agent, aucun outil MCP (`reseau-prive-r5-la-liste-d-un-projet-est`).
- **R6, comment l'agent l'emprunte** : la porte devient le seul proxy des agents (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY`), et `ssh` passe par `ProxyCommand="escouade-server via %h %p"` ; ensuite, dans l'image seulement, des noms en boucle locale pour ce qui ignore les proxys (`reseau-prive-r6-l-agent-emprunte-la-port`).
- **R7, le DNS** : résolu par le connecteur, des noms seulement ; l'adresse résolue vérifiée puis jointe telle quelle.
- **R8, le journal** : aux deux bouts, en JSON, chaque connexion permise ou refusée, jamais le contenu ; au serveur, les 1 000 dernières dans Réglages de l'espace › Réseau privé › Journal.
- **R9, les systèmes du connecteur** : Linux (x64, arm64), Windows (x64) et une image ; macOS se construit, sans service ni signature.
- **R10, la mise à jour du connecteur** : manuelle par défaut, automatique sur demande, signée.
- **R11, la sortie du serveur vers Internet** : hors de ce chantier, dite dans la fiche et dans l'interface.
- **R12, plusieurs connecteurs** : un serveur, plusieurs connecteurs, un par réseau, chacun son plafond ; la redondance plus tard.

## Le connecteur

- **Le tunnel** : une WebSocket sur TLS, sortante, vers l'entrée des connecteurs du serveur, à travers le proxy de l'entreprise s'il y en a un ; à l'intérieur, Noise, que le proxy ne peut ni lire ni imiter ; des flux « ouvre `hôte:port` ».
- **L'appairage** : Réglages de l'espace › Réseau privé › « Ajouter un connecteur » (ou `escouade-server connector pair`) donne un code (8 lettres, 10 minutes, une fois, 5 essais) et l'adresse de l'entrée ; le service informatique lance `escouade-connector pair <adresse> <code>`. Ni le code ni une clé privée au journal. Retirer un connecteur, d'un côté ou de l'autre, ferme son canal.
- **Deux listes** : le plafond, dans `connector.toml`, et la liste d'un projet, au serveur (R4, R5).
- **Les contrôles** : nom et port dans le plafond après mise en forme ; adresse littérale refusée sauf écrite dans le plafond ; jamais la boucle, le lien local, les métadonnées d'un cloud ni la machine du connecteur ; 256 flux, 32 ouvertures par seconde, 10 minutes de silence.
- **L'agent l'emprunte sans le savoir** (R6). Le reste d'Internet sort comme avant.
- **Les refus** sont lisibles par l'agent : « gitlab.acme.internal:22 n'est pas dans la liste du projet Démo », « … n'est pas permis par le connecteur d'Acme », « le réseau privé d'Acme est injoignable depuis 14:32 ».
- **La coupure** : le connecteur se reconnecte seul ; pendant la coupure, la porte refuse aussitôt ; les flux ouverts sont fermés, sans reprise. Un connecteur coupé depuis 10 minutes dont un projet a besoin fait une demande dans la boîte « À toi ».
- **Ce que voit Escouade, l'éditeur** : rien ; aucun relais à lui. Le serveur voit les noms, ports, octets et durées, pas le contenu chiffré de bout en bout.

## Limites

- TCP seulement : ni UDP, ni ICMP ; rien du réseau de l'entreprise vers le serveur.
- La porte est une consigne, pas une cloison : les agents d'un serveur partagent un compte, et un agent détourné peut se présenter pour un autre projet ; ce qui tient, c'est le plafond du connecteur. L'interface le dit.
- Le connecteur borne l'entrée dans le réseau de l'entreprise, pas la sortie du serveur vers Internet (R11).

## Les tickets

- **RP-1** (ESC-597) : la porte dans `escouade-server`, les variables des agents, `escouade-server via`.
- **RP-2** (ESC-598) : la crate `escouade-connector-proto`.
- **RP-3** (ESC-599) : le programme `escouade-connector`.
- **RP-4** (ESC-600) : l'entrée des connecteurs, l'appairage, la stratégie `servers.privateNetwork`.
- **RP-5** (ESC-601) : Réglages de l'espace et du projet › Réseau privé, le `.escouade.yml`, la boîte « À toi ».
- **RP-6** (ESC-602) : la distribution et la mise à jour du connecteur.
- **RP-7** (ESC-603) : les noms en boucle locale dans l'image.
- **RP-9** (ESC-604) : les guides du site, la fiche pour le service informatique.
- **RP-8** (ESC-605) : les trois cas de bout en bout, en dernier.

Ce qui attend l'utilisateur ensuite : un essai réel dans un vrai réseau d'entreprise (un connecteur derrière un vrai proxy avec inspection TLS, vers un vrai GitLab), et la fiche lue par un vrai service informatique.
