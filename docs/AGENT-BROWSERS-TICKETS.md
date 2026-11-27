# Navigateurs et pages des agents : la liste finale des tickets (2.2)

ESC-666 (AP-0), sur `main`, le 2026-11-26. Cette liste réunit les tickets des deux études, [`AGENT-BROWSERS.md`](AGENT-BROWSERS.md) (ESC-663, les `AB-*`) et [`AGENT-PAGES.md`](AGENT-PAGES.md) (ESC-661, les `AP-*`). Ils sont dans l'ordre, sans doublon, avec leurs dépendances et leurs heures. Le chef d'escouade écrit les cartes d'après elle. Elle fait foi sur les tableaux des deux études, qui détaillent chaque ticket.

Le contrat commun du pilotage (un seul outil `drive_browser`, sa cible `browser`, la règle de la main, le canal unique) est la section « Le pilotage, commun aux deux études », écrite mot pour mot dans les deux études.

## Les règles

- **Tout part de `main` et sort en 2.2.x** (l'utilisateur, 2026-11-26) : les outils MCP des navigateurs, le pilotage, les pages des agents et le vrai onglet hors du projet. Chaque ticket est ensuite reversé dans `release/2.3`. Ne jamais fusionner `release/2.3` dans un de ces tickets.
- **Ce que `main` n'a pas attend la 2.3** : les parties d'un projet, Réglages › Fonctionnalités, l'API distante et le mode serveur, le téléphone, le serveur d'équipe, l'Overseer et le `.escouade.yml` d'équipe (`enforced`, `defaults`). Un ticket de la 2.2 ne les touche pas. Le branchement se fait à part, dans AB-R.
- **Les heures** sont des heures d'agent Claude Code (Opus, effort moyen), relecture et e2e compris. Le Kanban tient six tickets en parallèle. Mes vérifications ajoutent 30 à 60 min par ticket.
- **AP-0 est ce ticket (ESC-666)** : il est fait. Plus rien n'attend un choix de l'utilisateur.
- **Fusions** :
  - AP-6 (le pilotage d'une page) est fondu dans AB-7 et AB-8 : un seul `drive_browser`, une seule bande ;
  - AP-8 (docs) et AB-13 (docs) ne font plus qu'un, DOC ;
  - les parties « à distance » d'AP-2, AP-7 et AP-8 vont dans AB-R.

## Lot 1 : lire les navigateurs, en tête

Il ne dépend de rien d'autre : rien n'ouvre, ne clique, ne tape ni ne fait défiler. Il part tout de suite.

| # | Ticket | Ce qu'il fait | Dépend de | Heures |
| --- | --- | --- | --- | --- |
| 1 | **AB-1** | Le pont de lecture natif (`browserread.rs`) : le script fixe dans le monde isolé par système, les retours par rappel natif, la liste fermée des méthodes du protocole DevTools, les captures par système, les tampons de console et de réseau tenus dès la création de chaque webview, le masquage du HTML ; la sonde mise en `examples/agentbrowser_probe`. | — | 3 h 30 |
| 2 | **AB-2** | `list_browsers` et `read_browser` (HTML, texte), qui peut, projet privé, l'interrupteur « Agents et navigateurs » de Réglages › Agents, la consigne du serveur MCP ; hors du lancement, la bande d'accord « veut lire », l'accord par origine et par navigateur, son retrait. | AB-1 | 3 h |
| 3 | **AB-3** | `capture_browser` : image MCP, fichier de repli, élément, document, largeurs, 20 par minute, l'accord hors du lancement. | AB-1, AB-2 | 2 h |
| 4 | **AB-4** | `browser_console` : curseur ou instant, niveaux, réseau, `limit`, `dropped`, adresses sans chaîne de requête, `from`, l'accord hors du lancement. | AB-1, AB-2 | 2 h |
| 5 | **AB-5** | L'e2e du lot 1 sous Windows (`agent-browsers-read.spec.ts`), et la sonde en CI sous Linux (Xvfb) et macOS : le premier vrai essai de WKWebView. | AB-2, AB-3, AB-4 | 2 h |

**Lot 1 : 12 h 30 d'agent**, une journée du Kanban. Le plus long chemin passe par AB-1, AB-2, AB-3 (ou AB-4) et AB-5 : 10 h 30.

## Lot 2 : le socle des pages des agents, en même temps que le lot 1

AP-1 ne dépend pas du lot 1 : il part le même jour, sur une autre place du Kanban.

| # | Ticket | Ce qu'il fait | Dépend de | Heures |
| --- | --- | --- | --- | --- |
| 6 | **AP-1** | La cloison : le schéma `escouade-page://` servi à sa seule webview, `agentpage-*` refusé par `refuses`, la CSP en en-tête et en `<meta>`, `incognito`, permissions et téléchargements refusés, la navigation stricte et la bande « La page veut ouvrir… », les préchargements et `RTCPeerConnection` retirés ; `agentpage_probe` en CI (Linux, macOS) et la page piège en e2e Windows. | — | 3 h 30 |
| 7 | **AP-2** | Le modèle et les outils : `pages.rs` (registre `p-<n>`, versions, bornes, copie, état durable, purge), `show_page`, `get_page`, `close_page`, projet privé, l'interrupteur « Pages des agents » de Réglages › Agents, la consigne du serveur MCP, journaux et statistiques ; les lignes `page:` dans `list_browsers`. | AP-1, AB-2 (pour `list_browsers` seulement) | 3 h |
| 8 | **AP-5** | Les modèles `review`, `diagram` (Mermaid embarqué, `strict`) et `compare`, leurs schémas validés par le cœur, leurs ancres pour le pilotage. | AP-1 | 3 h |
| 9 | **AP-3** | La vue, dans la direction A : la ligne `page:<p-n>`, l'ouverture à côté de la conversation, la barre (versions, largeurs), **la bande de pilotage commune** (avec sa mention de confiance), la carte dans la conversation et ses états ; fr et en, clair et sombre, 1440 et 1024. | AP-2 | 3 h |
| 10 | **AP-4** | Le retour par `ask_user` : `ask` de `show_page`, le canal `_/choose` validé par le cœur, la bande du choix et son geste, la réponse rendue à la page, « Ouvrir la page » et la vignette dans « À toi » ; un feu vert ne se propose jamais depuis la page. | AP-2, AP-3 | 2 h 30 |
| 11 | **AP-7** | La page à soi : le réglage du poste, `pages.custom` du `.escouade.yml` (le poste l'emporte), `window.escouade`, le refus sans page réglée. | AP-2, AP-5 | 1 h 30 |

**Lot 2 : 16 h 30 d'agent.** Le plus long chemin passe par AP-1, AP-2, AP-3 et AP-4 : 12 h.

## Lot 3 : piloter (le cas phare, pour les deux modes)

| # | Ticket | Ce qu'il fait | Dépend de | Heures |
| --- | --- | --- | --- | --- |
| 12 | **AB-6** | Ouvrir : `open_browser` (`url`, `launch`, `artifact`), `place`, `waiting`, les limites, les origines permises et demandées (C3), `close_browser`. | AB-2 | 3 h |
| 13 | **AB-7** (avec AP-6) | `drive_browser` pour toutes les lignes : les actions et leurs deux paliers par mode, les cibles (`browser`, `target`, clés de modèle), les refus (mot de passe sans accord, fichier, téléchargement, secrets), les bornes, `look`, `wait_user`, la règle de la main (gestes relevés par sondage), les entrées réelles CDP sous Windows et les scripts du monde isolé sous WebKit ; `version` et `tab` sur une page d'agent. | AB-1, AB-6, AP-2 | 4 h 30 |
| 14 | **AB-8** (avec AP-6) | Ce que voit l'utilisateur : la bande de pilotage d'AP-3 branchée sur les onglets, le curseur fantôme et le surlignage (overlay sous Windows, éléments sous WebKit), la ligne pilotée, « Reprendre la main » et « La rendre », « Continuer », les rangées numérotées et les repères du rail d'une page d'agent ; fr et en, clair et sombre. | AB-7, AP-3 | 3 h |
| 15 | **AB-9** | L'écran : n'ouvrir à côté que si l'utilisateur regarde la conversation, l'attente et la reprise, un volet par agent, rien de rouvert après une fermeture, « Laisser piloter par… ». | AB-6, AB-8 | 2 h |
| 16 | **AB-10** | L'artefact Claude : `artifact: "last"`, recharger à chaque version ; l'essai réel de se connecter à claude.ai dans un onglet d'Escouade. | AB-6, AB-7 | 1 h 30 |
| 17 | **AB-11** | Les cadres : `scroll_to` et `highlight` dans le cadre d'une autre origine sous Windows ; le cadre entier sous WebKit. | AB-10 | 1 h |
| 18 | **AP-9** | L'e2e du cas phare, mode 1 (Windows) : `show_page(review, ask)` à côté de la conversation, `drive_browser` défile et met en évidence (capture réelle), l'utilisateur reprend la main, choisit A dans la page, la bande, « Envoyer », la réponse à l'agent ; la page piège ne joint rien. | AP-4, AB-8 | 2 h |
| 19 | **AB-12** | L'e2e du cas phare, mode 2 (Windows) : un onglet à côté, défiler, mettre en évidence, un onglet de la page, la largeur mobile, la main reprise et rendue, une autre origine demandée, les paliers hors du lancement, un mot de passe refusé sans accord. | AB-7, AB-8, AB-9 | 2 h 30 |
| 20 | **DOC** (AP-8 et AB-13) | Les pages du site en fr et en (« Les pages des agents », « Les agents et les navigateurs »), les deux pages de spec passées de « à construire » à construites, et `kanban/page-du-lancement` (« Isolation » : ce qui change). | AP-9, AB-12 | 1 h 30 |

**Lot 3 : 21 h d'agent.**

## En tout, pour la 2.2

- **50 h d'agent**, en 20 tickets.
- Le plus long chemin passe par AP-1, AP-2, AP-3 (ou AB-1, AB-2, AB-6, prêts en même temps), puis AB-7, AB-8, AB-9, AB-12 et DOC : environ 23 h.
- Avec six places et mes vérifications, il faut compter **trois jours du Kanban**. Le lot 1 sort le premier jour et peut partir seul dans une 2.2.x.
- Ordre de départ conseillé : AB-1 et AP-1 ensemble, puis AP-5 dès AP-1 rendu.

## Ce qui attend la 2.3

| Ticket | Ce qu'il fait | Dépend de | Heures |
| --- | --- | --- | --- |
| Reversements | Chaque lot reversé de `main` dans `release/2.3`, environ 15 min par ticket. | chaque lot | 5 h |
| **AB-R** (avec la part d'AP-2, AP-7 et AP-8) | Le branchement sur ce que seule la 2.3 a. Les outils des navigateurs et des pages dans la partie « Lancement et navigateurs » (`parts::of_tool`, `parts::refusal`). Les deux interrupteurs de Réglages › Agents deviennent ceux de Réglages › Fonctionnalités. `desk-only` sur l'API distante pour `devpage_*`, `page_*` et les outils. Le champ `page` des demandes, servi au téléphone (« Une page l'accompagne… ») et la carte d'une fenêtre connectée à un serveur. L'Overseer, qui montre des pages et lit les navigateurs de son projet. `pages.custom` imposé ou recommandé par l'équipe. Un ticket d'équipe laisse le navigateur sur le poste. Tests et e2e de la 2.3. | lots 1 à 3 reversés | 2 h 30 |

## Après la 2.3

| Ticket | Ce qu'il fait | Heures |
| --- | --- | --- |
| AP-10 | L'aperçu en image d'une page, joint à sa demande pour le téléphone et la boîte. | 2 h |
| AP-11 | La page servie à une fenêtre connectée à un serveur (`page_bundle`, cloison locale, canal relayé). | 3 h |
| AP-12 | Plusieurs pages à soi nommées ; d'autres moteurs de schéma (D2, Graphviz en WASM). | 2 h |
| AB-14 | Des entrées réelles sous macOS et Linux, l'arbre d'accessibilité natif. | 3 h |
| AB-15 | Les agents du mode serveur : le navigateur d'une fenêtre connectée, lu et piloté à travers l'API distante. | 3 h |
| AB-16 | Téléchargements et envoi de fichiers, avec accord. | 1 h |
| — | Les plugins d'interface (2.4), qui repartent de la cloison et du pont. | — |

## Ce qui n'avance pas au rythme des agents

- le premier vrai essai de WKWebView, par la CI Mac d'AB-5 et d'AP-1 ;
- les essais réels de la cloison sur un Mac et sous Linux, au-delà de la CI : la CSP d'un schéma maison, WebRTC, les préchargements, les permissions ;
- se connecter à claude.ai dans un onglet d'Escouade (AB-10), un essai réel ;
- un essai des deux cas phares par l'utilisateur avec un vrai agent de chaque harness : les images MCP de `capture_browser` sont lues par Claude Code, et sont à vérifier pour Codex, OpenCode et Copilot.
