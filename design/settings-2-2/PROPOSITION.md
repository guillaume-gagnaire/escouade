# Réglages 2.2 — proposition

ESC-375, 2026-11-07. Demandé par l'utilisateur : « refais la modale de settings, c'est trop fouillis. Place-toi en expert UX pour bien organiser tout ça. » Ce document est l'étude : le diagnostic, la nouvelle organisation, trois directions à choisir sur aperçu, une recommandation et le plan de construction. Aucun code de l'app n'est changé.

- L'inventaire de chaque réglage : [INVENTAIRE.md](INVENTAIRE.md).
- L'existant, capturé dans la vraie app de `main` (build de debug, outillage des e2e, données de démo) : [existant/](existant/), avec ses mesures dans [existant/mesures.json](existant/mesures.json). Pour les reprendre : `npx tauri build --debug --no-bundle`, puis `npx playwright test -c design/settings-2-2/capture/playwright.config.ts`.
- Les trois directions : [index.html](index.html) (ouvrir le fichier ; `#a-general`, `#b-accueil`, `?theme=light`…), capturées en PNG dans [captures/](captures/) par `node design/settings-2-2/render.mjs`.

**En une phrase** : la fenêtre range ses réglages par l'endroit du code qui les lit, pas par la question que se pose celui qui les cherche. La recommandation est la direction A, une barre de catégories avec les projets dedans et une recherche, qui tient en 9 pages d'app et 7 pages par projet au lieu de 13 onglets et 32 écrans de défilement.

---

## 1. Diagnostic

Mesuré dans la vraie app (fenêtre de 980 × 700, à la taille par défaut de 1440 × 900) : **13 onglets, 52 groupes, 237 contrôles, 32 écrans de défilement**. Un écran est la hauteur visible du corps, 565 px.

| Onglet | Écrans | Groupes | Contrôles | Capture |
|---|---:|---:|---:|---|
| Application | 2 | 4 | 19 | [app.png](existant/app.png), [entier](existant/app-entier.png) |
| Harness (8 harness de démo) | 4,2 | 10 | 19 | [claude.png](existant/claude.png), [entier](existant/claude-entier.png) |
| Comptes (un compte) | 2 | 3 | 35 | [accounts.png](existant/accounts.png), [entier](existant/accounts-entier.png) |
| Voix | 2,7 | 2 | 26 | [voice.png](existant/voice.png), [entier](existant/voice-entier.png) |
| Notifications | 1,6 | 4 | 15 | [notifications.png](existant/notifications.png) |
| **Projets** | **9** | 13 | 58 | [projects.png](existant/projects.png), [entier](existant/projects-entier.png) |
| Kanban | 2,6 | 4 | 30 | [board.png](existant/board.png), [entier](existant/board-entier.png) |
| Intégrations | 2,9 | 5 | 16 | [integrations.png](existant/integrations.png), [entier](existant/integrations-entier.png) |
| Plugins | 1 | 0 | 1 | [plugins.png](existant/plugins.png) |
| Serveur MCP | 1 | 2 | 2 | [mcp.png](existant/mcp.png) |
| Terminaux | 1 | 1 | 3 | [terminals.png](existant/terminals.png) |
| Réseau | 1 | 2 | 4 | [network.png](existant/network.png) |
| À propos | 1,2 | 1 | 9 | [about.png](existant/about.png) |

Chaque capture a sa jumelle claire (`-light.png`).

### Les neuf causes du fouillis

**1. Un même sujet en trois ou quatre endroits.** Ce qui se passe à la limite d'un quota est réglé dans **Comptes** (« Changer de compte à », « Reprendre sur un autre compte… », passation, ordre des harness), dans **Harness** (« Reprise automatique après la limite d'usage ») et dans **Kanban** (« Pause au-delà du quota », marqué « pour tous les projets »). Les deux seuils, « Changer de compte à » (80, 90, 95 %, à la limite) et « Pause au-delà du quota » (80, 90, 95, 100 %), se ressemblent au point que la description du premier doit citer le second. Même chose pour le remote control (Harness pour tous, Projets pour un projet), les comptes (Comptes pour les harness, Intégrations pour les trackers et les hébergeurs, Projets pour les comptes réservés) et la clé Mistral (Voix et Comptes).

**2. Des réglages de l'app dans les onglets d'un projet.** Kanban, Intégrations et Projets ont la rangée de pastilles « Projet » qui dit « ce qui suit est à ce projet ». Pourtant « Pause au-delà du quota », « Synchronisation », « Import automatique », les comptes des trackers et des hébergeurs, « Recherche par le sens », « Nouveaux projets privés » et « Projets fermés » valent pour toute l'app ([board-entier.png](existant/board-entier.png), [integrations-entier.png](existant/integrations-entier.png), [projects-entier.png](existant/projects-entier.png)). On change un réglage en croyant ne toucher qu'un projet.

**3. Un onglet fourre-tout de 9 écrans.** Projets empile identité, remote control, confidentialité, workflow, git, worktrees, lancement, `.escouade.yml`, mémoire partagée, Kanban partagé, zone sensible, puis trois groupes qui ne sont même pas au projet ([projects-entier.png](existant/projects-entier.png)). Les renvois de l'app (« ⚙ » de Lancement, « Réglages du projet… », le panneau Workflow) atterrissent au milieu d'un défilement de 5 000 px.

**4. Des onglets mal nommés.** « Harness » règle les pièces jointes, le délai avant l'envoi et l'arrêt des process inactifs, qui ne sont pas propres à un harness ; sa description (« Exécutables et versions de chaque harness ») ne le dit pas. « Application » ne contient pas les mises à jour, qui sont dans « À propos » : le ticket ESC-374 lui-même place « Recevoir les versions bêta » « dans Réglages › Application, près de « Vérifier les mises à jour » », un bouton qui n'y est pas. « Plugins » n'a aucun réglage ([plugins.png](existant/plugins.png)), mais deux textes de l'app disent « Réglages › Plugins le rallume » (`prs.ts:16`, `integrations.ts:94`). « Intégrations » mêle sources du Kanban et hébergeurs de code.

**5. Le rare au même niveau que le fréquent.** Le thème et la langue partagent la hiérarchie avec « Purger les copies après (jours) », « Texte de plus de 256 Ko », « Arrêter les processus des agents inactifs », « Ignorer la vérification des certificats TLS ». D'après l'inventaire, un tiers des réglages ne se touchent jamais ou seulement en panne, et ils prennent autant de place que ceux qu'on règle à l'installation.

**6. Deux façons d'enregistrer sans le dire.** « Annuler » et « Enregistrer » sont en bas de chaque onglet, mais le thème, le zoom, tout l'onglet Comptes, les comptes des intégrations, l'interrupteur du serveur MCP, les modèles de voix, « Nettoyer », l'export de `.escouade.yml` et l'oubli d'un souvenir s'appliquent au geste. Rien ne distingue les uns des autres : « Annuler » laisse croire qu'on peut revenir sur un compte supprimé.

**7. Des libellés techniques et des descriptions trop longues.** « Harness », « Déclarations dans les harness », « NO_PROXY », « .escouade.yml » en titre de groupe, « Zone sensible ». La description du remote control fait 485 caractères et figure deux fois, à deux mots près (Harness et Projets, [claude.png](existant/claude.png)) ; celle de « Changer de compte à », 637 caractères.

**8. Des textes qui ne disent plus vrai.** Réseau dit que le proxy sert à « Claude Code, les intégrations et les mises à jour ». Le code le donne aussi au store, aux plugins WASM, aux modèles de voix et aux fichiers envoyés (`plugins/store.rs`, `plugins/wasm.rs`, `voice/models.rs`, `sentfiles.rs`). La page *Réglages* de la spec décrit encore le serveur MCP dans Harness et n'a pas l'onglet Plugins.

**9. La 2.2 n'a pas de place prévue.** ESC-373 installe un plugin au geste qui le demande et montre ceux de l'amorce « Dans l'app, sans téléchargement » : ni la vue Plugins ni les réglages ne disent aujourd'hui quel plugin sert quel réglage. ESC-374 ajoute un réglage de mises à jour qu'on ne sait pas où mettre. Les comptes et les hébergeurs servis par un plugin, les réglages et secrets d'un plugin, son activation projet par projet sont dans la page du plugin, sans lien depuis le réglage qu'ils servent.

### Critique (skill impeccable)

> ⚠️ DEGRADED: single-context (la consigne du ticket interdit les sous-agents : l'évaluation de conception et la lecture du détecteur ont été faites l'une après l'autre, dans ce fil, la première finie avant de lire la seconde).

| # | Heuristique | Note | Ce qui la tient |
|---|---|---:|---|
| 1 | Visibilité de l'état | 3 | un point sur l'onglet modifié, l'état de chaque harness et de chaque compte ; rien ne dit ce qui est déjà appliqué |
| 2 | Correspondance avec le monde réel | 2 | onglets nommés d'après le code (Harness, Intégrations), `NO_PROXY`, « Déclarations » |
| 3 | Contrôle et liberté | 2 | « Annuler » ne défait pas ce qui s'applique au geste (cause 6) |
| 4 | Cohérence et standards | 2 | réglages d'app dans les onglets de projet (cause 2), deux modes d'enregistrement |
| 5 | Prévention des erreurs | 3 | « Enregistrer » refusé avec sa raison ; confirmations pour fermer, effacer, supprimer |
| 6 | Reconnaître plutôt que se souvenir | 1 | 32 écrans sans recherche ; il faut savoir que les mises à jour sont dans À propos et la pause du quota dans Kanban |
| 7 | Souplesse et efficacité | 2 | de bons renvois depuis l'app (13 points d'entrée), mais aucune recherche ni raccourci vers une page |
| 8 | Esthétique et sobriété | 2 | la langue visuelle de la 2.0 tient ; la densité de texte et l'onglet de 9 écrans la noient |
| 9 | Récupération des erreurs | 3 | états d'erreur nommés avec leur remède (jeton expiré, harness introuvable, trousseau illisible) |
| 10 | Aide et documentation | 3 | une description sous presque chaque réglage, parfois trop longue (cause 7) |
| **Total** | | **23/40** | **Correct, à reprendre** : le style est bon, l'architecture de l'information ne l'est pas |

- **Spécificité** : la fenêtre est bien celle d'Escouade (teinte du projet, filets, rangées) ; ce qui manque est propre au produit et pas au style : la fenêtre ne sait pas ce qui attend l'utilisateur (un harness sans compte, un jeton expiré), alors que toute l'app est construite sur « qui attend passe devant ».
- **Charge cognitive** : 13 choix dans la barre, 5 groupes de 2 à 4 options dans l'onglet Comptes ; au-delà de 4 options visibles à chaque décision de navigation.
- **Détecteur** (`impeccable detect`) : sur `src/components/settings` et `SettingsModal.svelte`, 22 avertissements, tous de jetons (16 rayons hors de l'échelle, 4 tailles de texte, 2 couleurs), aucun motif interdit. Le désordre n'est pas dans les pixels : il est dans le rangement, que le détecteur ne lit pas.

---

## 2. Nouvelle organisation

Elle vaut pour les trois directions : elles ne diffèrent que par la façon d'y naviguer.

### Principes

1. **Ranger par la question, pas par le code.** « Qu'est-ce qui se passe quand un quota est épuisé ? » a une seule réponse, à un seul endroit.
2. **Deux portées, jamais mêlées.** Ce qui vaut pour Escouade et ce qui vaut pour un projet ne partagent aucune page. Un réglage d'app aujourd'hui dans un onglet de projet monte dans l'app.
3. **La fenêtre prend la couleur de ce qu'elle règle** : le teal d'Escouade pour les pages de l'app (comme la Vue d'ensemble et les Stats), la couleur du projet pour ses pages. La portée se voit avant de se lire.
4. **Le rare descend dans « Avancé »** : replié en bas de sa page, avec le nom des réglages qu'il cache, jamais supprimé.
5. **Chaque page tient en un ou deux écrans** à la taille par défaut.
6. **Ce qui attend l'utilisateur se voit** : un point ambre sur la page qui a un problème (harness sans compte, jeton expiré, plugin arrêté), comme une tuile de projet.

### Les pages d'Escouade, dans l'ordre

| # | Page | Ce qu'elle règle | D'où ça vient |
|---|---|---|---|
| 1 | **Général** | Apparence (thème, couleurs, zoom, taille des terminaux), Langue (interface, textes rédigés), Mises à jour (version, rechercher, automatiques, **bêta** d'ESC-374), Toi (ton nom, statistiques d'usage). Avancé : taille du texte de l'éditeur, ton identifiant | Application ; mises à jour et statistiques d'**À propos** |
| 2 | **Agents** | Nouveaux agents (vue, liste de tâches), Délai avant l'envoi pendant un tour, Textes rédigés (harness et modèle), Remote control pour tous les agents, Piloter Escouade (serveur MCP : l'interrupteur, ses déclarations). Avancé : arrêt des process inactifs, activité MCP | Harness ; « Textes rédigés » d'Application ; l'onglet **Serveur MCP** |
| 3 | **Comptes et harness** | Un groupe par harness : son état, sa version, ses comptes et « Ajouter un compte… » (un harness non installé s'y installe, ESC-373). Puis **Quand un quota est épuisé**, réuni : changer de compte à, **pause du pilote auto au-delà du quota**, reprendre un agent de ticket sur un autre compte, **reprise automatique après la limite d'usage**, continuer sur le harness suivant. Avancé : ordre des harness, modèle de passation, chemin de l'exécutable, mise à jour de Claude Code, projection des quotas | Harness (état, chemin, mise à jour) et Comptes fusionnés ; la pause du quota de **Kanban** ; la reprise automatique de **Harness** |
| 4 | **Services connectés** | Hébergeurs de code (instances, jetons), Sources de tickets (comptes Jira, Trello, GitHub, Linear, ClickUp, GitLab, Azure Boards, « servi par » un plugin), Synchronisation et Import automatique (réglages d'app aujourd'hui) | Les parties d'app d'**Intégrations** |
| 5 | **Voix** | Dictée (transcrite par, service, clé, micro, raccourci, langue, modèle), Conversation (voix, vitesse, volume, délai, repli) | Voix, inchangée ; la clé Mistral partagée y reste, nommée depuis Comptes |
| 6 | **Notifications** | Me prévenir pour, Pull requests et déploiements, Canaux, Son | Notifications, inchangée |
| 7 | **Plugins** | Installés (ce que chacun ajoute, où il se règle, actif sur N projets, « Réglages » ouvre sa page), Proposés par Escouade (l'amorce, ESC-373), N'installer que des versions vérifiées, « Ouvrir la vue Plugins » | Le lien de l'onglet Plugins, enrichi ; « vérifiées seulement » de la vue |
| 8 | **Avancé** | Réseau (proxy, exclusions, terminaux, certificats), Terminaux (PowerShell 7, Git Bash, WSL), Pièces jointes (taille, avertissement, origine, gros textes, purge), Recherche par le sens (modèle local), Données locales | Réseau, Terminaux, les pièces jointes de Harness, « Tous les projets » de Projets |
| — | **À propos** (en bas) | Version, harness détectés, crédits et licences, site, Discord, soutenir | À propos, sans les mises à jour |

### Les pages d'un projet, dans l'ordre

| # | Page | Ce qu'elle règle | D'où ça vient |
|---|---|---|---|
| 1 | **Projet** | Nom, dossier, couleur, tuile (lettres, logo, libellé), harness par défaut, compte préféré, assistant de configuration, plugins proposés par le projet. Avancé : zone sensible (fermer le projet) | Identité de Projets |
| 2 | **Git et worktrees** | Un worktree par agent, au premier message, branche de départ, dossier, préfixes, fichiers copiés, commit, branche des docs, préparation et démontage, worktrees restés, dépôts | Git, Worktrees, Dépôts de Projets |
| 3 | **Lancement** | Ouvrir les pages des lancements, commandes de lancement, remplir automatiquement | Lancement de Projets |
| 4 | **Kanban** | Validation (action, cible, stratégie, PR), avant et après, pilote auto (attribution, pause CI rouge), agents des tickets (parallèle, modèle, effort, mode), workflow du projet | Kanban sans la pause du quota ; Workflow de Projets |
| 5 | **Tickets importés** | Sources liées au projet, correspondance des statuts | Les parties de projet d'**Intégrations** |
| 6 | **Équipe** | Kanban partagé (ton nom ici, tickets partagés, partager chaque nouveau ticket), `.escouade.yml` (état, commandes en attente, exporter), mémoire partagée | Kanban partagé, `.escouade.yml`, Mémoire partagée de Projets |
| 7 | **Confidentialité** | Projet privé, ses fichiers (garde-fou, barrière par harness), comptes réservés ou partagés, comptes partagés à la limite, remote control de ce projet, **plugins actifs dans ce projet** (le pendant de l'onglet Projets de la page d'un plugin) | Confidentialité et Remote control de Projets ; nouveau : la liste des plugins |

Et **Tous les projets** (sous la liste) : nouveaux projets privés, projets fermés (rouvrir, effacer l'historique, historique illisible).

### Les noms

| Aujourd'hui | Proposé | Pourquoi |
|---|---|---|
| Application | Général | les mises à jour et les statistiques y viennent ; « Application » laissait croire que le reste n'en était pas |
| Harness | (fusionné) Comptes et harness, Agents | le mot reste là où il désigne un harness ; les réglages des agents vont à Agents |
| Intégrations | Services connectés (app), Tickets importés (projet) | dit ce qu'on y trouve, et sépare les deux portées |
| Serveur MCP | Agents › Piloter Escouade | le libellé de l'interrupteur dit déjà ce que fait le serveur |
| Zone sensible | Avancé › Fermer le projet… | une seule action, qu'aucun titre alarmant ne doit annoncer |
| .escouade.yml (titre de groupe) | Équipe › Réglages partagés (.escouade.yml) | le nom du fichier passe en sous-titre |
| Quotas (groupe de Comptes) | Avancé de Comptes et harness › Projection des quotas | un seul réglage, d'affichage |

### Ce qui part ailleurs

- **Plugins** : la vue Plugins reste l'endroit où l'on installe, retire une permission et lit un journal. La page Plugins des réglages devient un vrai tableau des plugins installés (ce que chacun ajoute, où il se règle) et des plugins proposés : aujourd'hui elle n'est qu'un lien.
- **Activation d'un plugin par projet** : visible aussi sur la page Confidentialité du projet (lecture et bascule), en plus de la page du plugin.
- **Mémoire partagée** (proposé, à décider) : sa lecture et son oubli iraient mieux dans la vue Docs du projet, à côté de la spec et des décisions ; la page Équipe garderait un lien. Ce déplacement n'est pas dans le plan tant que l'utilisateur ne l'a pas choisi.
- **Rien d'autre ne quitte la fenêtre.** Les réglages qui étaient déjà hors de la fenêtre (largeurs des barres, vue et remote control d'un agent, modèle d'un agent) y restent.

### La recherche

Elle vaut la peine : 237 contrôles, et les textes de l'app renvoient à 13 endroits par leur nom. Un champ en tête (`Ctrl+F` dans la fenêtre) cherche dans les libellés et les descriptions des réglages, d'Escouade et de chaque projet, sans tenir compte de la casse ni des accents (comme la recherche dans les conversations). Un résultat ouvre la page, déplie Avancé s'il le faut et surligne la rangée. Elle demande que chaque rangée soit déclarée une fois avec son libellé et sa page (le registre du lot 1), ce qui sert aussi le test « rien ne se perd ».

### Sans perdre un réglage

Chaque ligne de l'[inventaire](INVENTAIRE.md) a sa place dans les tableaux ci-dessus ; la table de correspondance complète est en annexe A. La réorganisation ne touche à aucune clé enregistrée (`settings.json`, fichiers des projets, `.escouade.yml`) : seule la page où un réglage s'affiche change.

---

## 3. Trois directions

Toutes trois gardent le design de la 2.0 (`DESIGN.md`) : les jetons de `src/app.css`, Inter, les rangées `.sg`/`.sr` sous filets, les segments, les interrupteurs, un seul bouton plein (« Enregistrer »). Elles montrent les mêmes réglages, avec les textes de l'app ; seuls les nouveaux noms de pages et la ligne d'aide de « Recevoir les versions bêta » (tirée d'ESC-374) sont nouveaux. Les données sont celles de la démo du site (`demo-api`, `studio-web`, `mobile-app` privé, `infra`), fictives.

### A — La barre des catégories (recommandée)

Une barre à gauche, toujours là : la recherche, les huit pages d'Escouade, puis chaque projet ; le projet choisi déplie ses sept pages. À propos et la version en bas.

| Général | Un projet | Comptes | Plugins |
|---|---|---|---|
| ![](captures/a-general.png) | ![](captures/a-projet.png) | ![](captures/a-comptes.png) | ![](captures/a-plugins.png) |
| ![](captures/a-general-light.png) | ![](captures/a-projet-light.png) | ![](captures/a-comptes-light.png) | ![](captures/a-plugins-light.png) |

- **Pour** : le modèle que tout développeur connaît (réglages de macOS, de VS Code, de Linear) ; la portée se lit dans la barre (« Escouade » ou le projet) et dans la teinte ; chaque renvoi de l'app devient une adresse (« Projets › demo-api › Lancement ») ; la navigation au clavier reste celle de la fenêtre actuelle (flèches dans la barre).
- **Contre** : la barre grandit avec les projets (au-delà d'une dizaine, la liste défile ; le projet ouvert reste en tête) ; la fenêtre passe de 980 × 700 à 1120 × 780 (à 1028 × 779, celle de la CI, elle prend toute la place disponible, comme aujourd'hui).

### B — L'accueil des réglages

La fenêtre s'ouvre sur un accueil : ce qui attend en tête, en ambre (un harness sans compte, un jeton expiré), puis chaque page d'Escouade et chaque projet avec ce qu'il vaut (« Système · 100 % · Français »). Un clic ouvre une page ; un fil d'Ariane y ramène.

| Accueil | Général | Un projet | Comptes | Plugins |
|---|---|---|---|---|
| ![](captures/b-accueil.png) | ![](captures/b-general.png) | ![](captures/b-projet.png) | ![](captures/b-comptes.png) | ![](captures/b-plugins.png) |
| ![](captures/b-accueil-light.png) | ![](captures/b-general-light.png) | ![](captures/b-projet-light.png) | ![](captures/b-comptes-light.png) | ![](captures/b-plugins-light.png) |

- **Pour** : la plus fidèle à la signature d'Escouade, « qui attend passe devant » ; les résumés disent l'état sans ouvrir ; la page lue a toute la largeur.
- **Contre** : un clic de plus pour tout, et un aller-retour pour passer d'une page à l'autre ; les résumés de chaque page sont du code à tenir juste (une valeur fausse sur l'accueil est pire que pas de résumé) ; « Annuler » et « Enregistrer » sur un accueil qui ne règle rien.

### C — Deux portées, des sections qui se replient

En haut, « Escouade » ou un projet, choisi par segments ; quatre onglets soulignés de chaque côté (Général, Comptes, Plugins, Avancé ; Projet, Git et lancement, Kanban et tickets, Équipe et confidentialité) ; dans l'onglet, des sections repliées qui disent leur valeur.

| Général | Un projet | Comptes | Plugins |
|---|---|---|---|
| ![](captures/c-general.png) | ![](captures/c-projet.png) | ![](captures/c-comptes.png) | ![](captures/c-plugins.png) |
| ![](captures/c-general-light.png) | ![](captures/c-projet-light.png) | ![](captures/c-comptes-light.png) | ![](captures/c-plugins-light.png) |

- **Pour** : peu de choix visibles à la fois (2 portées, 4 onglets) ; les sections repliées donnent un sommaire lisible ; la plus proche de la fenêtre actuelle à construire.
- **Contre** : les sections repliables cachent des réglages derrière deux clics (onglet, section) et la recherche doit déplier ; quatre onglets forcent des regroupements larges (Voix, Notifications et Agents dans Général) ; changer de projet passe par un menu.

---

## 4. Recommandation : A

1. **C'est l'organisation qui manquait, sans un détour de plus.** Le fouillis vient du rangement (causes 1 à 5) ; A expose ce rangement en entier, à un clic de chaque page, là où B ajoute un clic et C replie.
2. **La portée devient visible** : « Escouade » et « Projets » sont deux parties de la barre, et la fenêtre change de teinte en passant de l'une à l'autre. C'est la réponse directe à la cause 2.
3. **Les renvois de l'app deviennent des adresses stables** (annexe B) : `Projets › demo-api › Lancement` au lieu de « l'onglet Projets, défilé jusqu'à `data-anchor="launch"` ».
4. **Elle passe à l'échelle de la 2.2** : un plugin qui ajoute des réglages d'app ajoute une rangée à une page existante (Comptes et harness, Services connectés, Voix), jamais une page.
5. **Un emprunt à B**, qui fait sa force sans son clic de plus : les points ambre dans la barre (Comptes et harness, Services connectés, un projet sans compte de son harness), avec la raison en infobulle. **Un emprunt à C** : « Avancé » replié en bas de chaque page, qui nomme ce qu'il cache.

Ce que A ne règle pas seul : le double mode d'enregistrement (cause 6). C'est une décision de comportement, laissée à l'utilisateur (§ 7).

---

## 5. Plan de construction (direction A)

Huit lots, chacun un ticket, dans cet ordre ; chacun se fusionne seul, l'app reste utilisable entre deux. Aucun ne change une clé enregistrée.

| Lot | Ce qu'il fait | Ce qu'il touche | Tests |
|---|---|---|---|
| **R1 — Le registre des pages** | `SETTINGS_PAGES` remplace `SETTINGS_TABS` : chaque page (id, portée app ou projet, icône, libellé, description) et, pour chaque rangée, son id, sa page, son libellé et s'il est dans Avancé. `SETTINGS_OF` passe des onglets aux pages. Les anciens ids d'onglet (`claude`, `board`, `integrations`…) sont lus encore et menés à leur page (`legacyTab()`) | `src/lib/settings.svelte.ts`, `src/lib/state.svelte.ts` (`modal.kind === 'settings'` prend `page` et `projectId`), i18n `settings.pages.*` fr et en | unitaires : chaque clé de `Settings` et de `ProjectSettings` est réglée par exactement une page (la garantie « rien ne se perd ») ; chaque ancien id d'onglet mène à une page ; `changed()` par page |
| **R2 — La coque** | La barre (recherche inerte, Escouade, Projets avec le projet déplié, Tous les projets, À propos), 1120 × 780, la teinte par portée, le pied « Annuler / Enregistrer », le point de changement par page, le clavier (flèches, Origine, Fin), Avancé replié | `SettingsModal.svelte`, un `SettingsNav.svelte`, `Group.svelte` (prop `advanced`), `app.css` si un jeton manque | composant : navigation au clavier, portée et teinte, point de changement ; e2e : `settings-look.spec.ts` réécrit contre `design/settings-2-2/index.html#a-*` (les deux thèmes, 1440 × 900 et 1028 × 779) |
| **R3 — Escouade : Général, Agents, Avancé, À propos** | Déplacer les rangées : mises à jour et statistiques vers Général (avec la bêta d'ESC-374 si elle est fusionnée), textes rédigés et MCP vers Agents, réseau, terminaux, pièces jointes et recherche par le sens vers Avancé ; découper `AppTab.svelte` en une page par fichier | `ApplicationTab.svelte`, `AppTab.svelte` (supprimé à la fin), `McpSection.svelte`, `VectorsSection.svelte` | composant : `ApplicationTab.test.ts`, `McpSection.test.ts` déplacés ; e2e : `about-update.spec.ts`, `mcp.spec.ts`, `display.spec.ts`, `theme.spec.ts` sur leurs nouvelles pages |
| **R4 — Comptes et harness** | Un groupe par harness avec son état et ses comptes ; « Quand un quota est épuisé » réuni (la pause du quota quitte Kanban, la reprise automatique quitte Harness) ; Avancé | `AccountsTab.svelte`, la partie harness d'`AppTab.svelte`, `BoardTab.svelte` (retrait de la pause), `ModelFavorites.svelte` | composant : `AccountsTab.test.ts` ; e2e : `accounts.spec.ts`, `harness.spec.ts`, `account-models.spec.ts`, `engines.spec.ts`, un e2e de la pause du quota depuis sa nouvelle page |
| **R5 — Services connectés et Tickets importés** | Couper `IntegrationsTab.svelte` en deux : comptes, hébergeurs, synchronisation et import automatique (app) ; sources liées et correspondance (projet) | `IntegrationsTab.svelte`, `ForgesSection.svelte`, `ImportModal.svelte`, `PrPanel.svelte`, `prs/PrsView.svelte` (renvois) | composant : `IntegrationsTab.test.ts`, `ForgesSection.test.ts` ; e2e : `integrations.spec.ts`, `prs*.spec.ts`, `azure-devops.spec.ts` |
| **R6 — Les pages d'un projet** | Découper `ProjectTab.svelte` en Projet, Git et worktrees, Lancement, Équipe, Confidentialité ; Kanban reçoit Workflow ; les plugins actifs du projet dans Confidentialité | `ProjectTab.svelte`, `BoardTab.svelte`, `WorkflowSection.svelte`, `SharedSection.svelte`, `MemorySection.svelte`, `NewProjects.svelte`, `ClosedProjects.svelte`, `RunsSection.svelte`, `Sidebar.svelte`, `Board.svelte`, `WorkflowPanel.svelte`, `ProjectRail.svelte`, `lib/project-rail.ts`, `AccountModal.svelte` (renvois) | composant : `ProjectTab.test.ts` et `BoardTab.test.ts` découpés ; e2e : `projects.spec.ts`, `escouade-yml.spec.ts`, `workflow-settings.spec.ts`, `memory.spec.ts`, `shared-board.spec.ts`, `multi-repo.spec.ts`, et un e2e par renvoi (annexe B) |
| **R7 — Plugins** | La page Plugins : installés (ce qu'ils ajoutent, où ils se règlent, actifs sur N projets, « Réglages »), proposés par Escouade (après ESC-373), vérifiées seulement | un `PluginsPage.svelte` dans `settings/`, `lib/plugins*` en lecture | composant ; e2e : `plugins-view.spec.ts`, `plugins-seed.spec.ts` (le lien et le geste) |
| **R8 — Recherche et points ambre** | La recherche sur le registre de R1 (casse et accents ignorés, Avancé déplié, rangée surlignée) ; les points ambre de la barre avec leur raison | `SettingsNav.svelte`, `lib/settings-search.ts` | unitaires : la recherche trouve chaque rangée par son libellé en fr et en en ; e2e : chercher « proxy » ouvre Avancé › Réseau, chercher « bêta » ouvre Général |

Après R8 : la page *Réglages* de la spec (`fenetre/reglages`) réécrite, avec une décision ; la documentation du site qui nomme un onglet (`website/content/docs/{fr,en}/`) ; les captures du site (`npm run site-shots`) ; le CHANGELOG. Les textes « Réglages › … » de l'app sont refaits dans le lot qui déplace leur cible.

### Migration

- **Les données** : aucune. Les clés de `settings.json`, des fichiers de projet, du Kanban et de `.escouade.yml` restent. Un test de R1 le garde : il liste les clés réglées par chaque page et les compare au type `Settings`.
- **Les renvois du code** : les 19 appels de `app.modal = { kind: 'settings', … }` (annexe B) passent d'un `tab` à une `page` dans le lot qui déplace leur cible ; jusqu'à R6, `legacyTab()` mène un ancien id à sa page, puis il est retiré avec un test qui vérifie qu'aucun appel ne le prend plus.
- **Les textes qui nomment un onglet** : les 28 « Réglages › … » du catalogue fr et leurs 29 « Settings › … » en anglais (liste dans l'inventaire) sont réécrits avec le nouveau chemin ; un test de catalogue refuse un « Réglages › X » dont X n'est pas une page du registre.
- **Le brouillon** : `settingsForm` garde son brouillon unique ; seules ses clés de regroupement passent des onglets aux pages. Un brouillon en cours n'est jamais gardé d'une version à l'autre : rien à migrer.
- **Les e2e** : les sélecteurs `getByRole('tab', { name: 'Harness' })` deviennent des liens de la barre (`getByRole('link', …)` ou `tab` gardé, à choisir en R2) ; une aide `openSettingsPage(page, 'Comptes et harness')` dans `tests/e2e/fixture.ts` remplace `openTab`.

---

## 6. Passes de relecture (faites par l'agent, sans sous-agent)

Le skill impeccable confie la relecture finale et la documentation à des sous-agents (`impeccable-finish-reviewer`, `impeccable-documenter`). La consigne du ticket les interdit : je les ai faites moi-même, l'une après l'autre.

**Relecteur (finish-reviewer)**, sur les 26 captures et `index.html` :
- Corrigé : le point ambre de l'onglet « Comptes » de C était étiré par la règle des onglets (`.c-tabs span` prenait le point) ; l'accueil de B avait « Réglages » deux fois (barre et titre) et coupait sa dernière ligne ; le bas de B est remonté (fenêtre de 820 px, rangées à 8 px).
- Vérifié par `render.mjs` : aucune rangée plus large que sa boîte, aucune fenêtre hors de l'écran, dans les 26 rendus.
- Vérifié sur l'existant : aucun chemin de la machine dans les captures (données sous `C:\escouade-375`, chemin des faux harness masqué en `C:\escouade-demo\bin`).
- Laissé tel quel, et dit : les aperçus n'ont pas d'états de survol ni de focus (ce sont des captures fixes) ; la version de l'existant est 2.1.3 (celle de `main`), celle des aperçus 2.2.0.
- Détecteur sur `index.html` : 11 avertissements de jetons (rayons de 9 px des segments et de 4 px des touches, déjà ceux de l'app), aucun motif interdit.

**Documenteur** : ce ticket ne change ni le système de design ni le comportement de l'app. `DESIGN.md` reste juste (aucun jeton nouveau) ; la page *Réglages* de la spec n'est pas réécrite ici, puisque rien n'est construit : elle le sera au lot R8, avec une décision. Ce que la spec a déjà de faux est noté (cause 8).

---

## 7. Décisions à prendre par l'utilisateur

1. **La direction** : A (recommandée), B ou C, sur les captures.
2. **L'enregistrement** (cause 6), au choix : garder le brouillon et marquer « appliqué tout de suite » sur les rangées qui le sont ; ou tout appliquer au geste, comme les réglages de macOS et de VS Code, et retirer « Annuler / Enregistrer » (les erreurs de saisie, comme une commande sans nom, se disent alors sur la rangée). La seconde est plus simple à l'usage et change le comportement de toute la fenêtre.
3. **La mémoire partagée** : la laisser dans Équipe, ou la déplacer dans la vue Docs du projet.
4. **Synchronisation et Import automatique** : réglages d'app aujourd'hui ; les garder ainsi (Services connectés) ou en faire des réglages de projet (Tickets importés), ce qui change leur comportement.

---

## Annexe A — Correspondance de chaque groupe

| Aujourd'hui (onglet › groupe) | Proposé (page › groupe) |
|---|---|
| Application › Affichage : thème, couleurs, zoom, terminaux | Général › Apparence |
| Application › Affichage : taille du texte de l'éditeur | Général › Avancé |
| Application › Textes rédigés | Agents › Textes rédigés |
| Application › Langue | Général › Langue |
| Application › Équipe : ton nom | Général › Toi |
| Application › Équipe : ton identifiant | Général › Avancé |
| Harness › *harness* : état, version, installation, coût | Comptes et harness › *harness* |
| Harness › *harness* : chemin de l'exécutable, mise à jour de Claude Code | Comptes et harness › Avancé |
| Harness › *harness* : modèles favoris (OpenCode) | Comptes et harness › OpenCode |
| Harness › *harness* : remote control pour tous | Agents › Remote control |
| Harness › Tous les harness : vue des nouveaux agents, liste de tâches, délai avant l'envoi | Agents › Nouveaux agents |
| Harness › Tous les harness : reprise automatique après la limite | Comptes et harness › Quand un quota est épuisé |
| Harness › Tous les harness : arrêt des process inactifs | Agents › Avancé |
| Harness › Pièces jointes | Avancé › Pièces jointes |
| Comptes › Comptes *harness* | Comptes et harness › *harness* |
| Comptes › Quand un quota est épuisé : seuil, reprise d'un agent de ticket, harness suivant | Comptes et harness › Quand un quota est épuisé |
| Comptes › Quand un quota est épuisé : ordre des harness, modèle de passation | Comptes et harness › Avancé |
| Comptes › Quotas › Projection | Comptes et harness › Avancé |
| Voix › Transcription, Conversation | Voix › Dictée, Conversation |
| Notifications › tout | Notifications › tout |
| Projets › Plugins proposés, Identité, Assistant | Projet › Projet |
| Projets › Remote control | Projet › Confidentialité |
| Projets › Confidentialité | Projet › Confidentialité |
| Projets › Dépôts, Git, Worktrees | Projet › Git et worktrees |
| Projets › Workflow | Projet › Kanban |
| Projets › Lancement | Projet › Lancement |
| Projets › .escouade.yml, Mémoire partagée, Kanban partagé | Projet › Équipe |
| Projets › Zone sensible | Projet › Projet › Avancé |
| Projets › Tous les projets (recherche par le sens) | Avancé › Recherche par le sens |
| Projets › Nouveaux projets, Projets fermés | Tous les projets |
| Kanban › validation, avant et après, pilote auto (sans la pause du quota), agents | Projet › Kanban |
| Kanban › pause au-delà du quota | Comptes et harness › Quand un quota est épuisé |
| Intégrations › Hébergeurs de code, Comptes connectés, Synchronisation, Import automatique | Services connectés |
| Intégrations › Sources liées, Correspondance des statuts | Projet › Tickets importés |
| Plugins (lien) | Plugins |
| Serveur MCP › interrupteur, déclarations | Agents › Piloter Escouade |
| Serveur MCP › Activité MCP | Agents › Avancé |
| Terminaux, Réseau | Avancé |
| À propos › mises à jour, statistiques | Général |
| À propos › version, crédits, liens, données locales | À propos (données locales : Avancé) |
| 2.2 : Recevoir les versions bêta (ESC-374) | Général › Mises à jour |
| 2.2 : plugins de l'amorce (ESC-373) | Plugins › Proposés par Escouade |
| 2.2 : réglages et secrets d'un plugin | page du plugin (vue Plugins), ouverte par « Réglages » de la page Plugins |

## Annexe B — Les renvois et leur nouvelle adresse

| D'où | Aujourd'hui | Proposé |
|---|---|---|
| `Ctrl+,`, bouton Réglages | dernier onglet | dernière page |
| Tuile › Réglages du projet…, ⚙ de la barre latérale | Projets, ce projet | Projets › *projet* › Projet |
| Lancement : ⚙, Configurer, Proposer des commandes | Projets › `launch` | Projets › *projet* › Lancement |
| Kanban : ⚙ Après validation | Kanban, ce projet | Projets › *projet* › Kanban |
| Kanban : commandes de `.escouade.yml` en attente | Projets, ce projet | Projets › *projet* › Équipe |
| Import d'un ticket, sans source | Intégrations › `sources` | Projets › *projet* › Tickets importés |
| PR : jeton refusé, aucun hébergeur | Intégrations | Services connectés |
| Panneau Workflow | Projets, ce projet | Projets › *projet* › Kanban (Workflow) |
| Accueil : aucun harness | Harness | Comptes et harness |
| Fenêtre d'un compte : réserver à un projet | Projets, ce projet | Projets › *projet* › Confidentialité |
| Retours de fenêtres (compte, mise à jour, fermeture) | l'onglet d'origine | la page d'origine |
| Textes « Réglages › Harness » (harness introuvable) | Harness | Comptes et harness |
| Textes « Réglages › Plugins le rallume » | Plugins (lien) | Plugins (interrupteur sur la rangée du plugin) |
| Textes « Réglages › Application › Couleurs » | Application | Général |
| Texte « Réglages › À propos » (statistiques) | À propos | Général |
