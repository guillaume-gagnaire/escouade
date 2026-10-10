# Escouade 1.7 — à faire

Quatre chantiers : un serveur MCP pour piloter Escouade depuis Claude, plusieurs comptes Claude avec bascule automatique, une meilleure gestion des branches, et l'interface en plusieurs langues (langue du système par défaut, anglais, français). Les deux premiers se touchent : chaque compte a son propre `~/.claude.json`, donc le serveur MCP doit être déclaré dans chacun.

## 0. À vérifier avant de concevoir (essais jetables)

- [ ] **Trousseau macOS et `CLAUDE_CONFIG_DIR`** : les sources se contredisent. Certaines disent que Claude Code range ses identifiants dans une entrée unique « Claude Code-credentials », la même quel que soit le dossier de config (deux comptes s'écraseraient) ; d'autres qu'un dossier de config a son entrée à lui. Essai sur un Mac : deux dossiers, deux `/login`, relire les deux entrées avec `security find-generic-password`. Ça décide si les comptes multiples marchent sous macOS sans bricolage.
- [ ] **Reprendre une session dans un autre compte** : copier `projects/<dossier>/<session>.jsonl` d'un dossier de config à l'autre, puis `claude --resume=<id>` avec l'autre `CLAUDE_CONFIG_DIR`. Si ça marche, un agent arrêté par la limite peut continuer sur un autre compte sans perdre sa conversation. Sinon, la bascule ne vaut que pour les nouveaux agents.
- [ ] **Crate MCP côté Rust** : vérifier le SDK officiel (`rmcp`) et son transport HTTP « streamable » ; ce qu'il ajoute comme dépendances (serveur HTTP) ; compatibilité avec le tokio de l'app.
- [ ] **`claude mcp add` sous Windows** : la commande et son `--header` passent-ils proprement par `Command` (guillemets, `--` avant une commande stdio) ? Le faire avec le vrai `claude`, et le scénariser dans le faux `claude` des tests.

## 1. Serveur MCP : piloter Escouade depuis Claude

But : un Claude (dans un terminal, dans un autre outil, ou un agent d'Escouade lui-même) lit l'état d'Escouade et agit dessus : projets, agents, tickets, Kanban.

### Transport et connexion
- [ ] Serveur MCP HTTP sur `127.0.0.1`, port fixe gardé dans les réglages (pour que la config de Claude reste valable d'un démarrage à l'autre), choisi libre au premier lancement.
- [ ] Jeton (bearer) généré par Escouade, gardé dans le trousseau du système comme les jetons d'intégration ; toute requête sans lui est refusée.
- [ ] Contre le DNS rebinding : refuser un en-tête `Host` autre que `127.0.0.1:<port>` / `localhost:<port>`, et toute requête qui porte un `Origin` de navigateur.
- [ ] Le serveur ne tourne que si le réglage est activé ; il s'arrête avec l'app. Plus tard : un petit pont stdio (`escouade-mcp.exe`) pour les clients qui ne savent parler qu'en stdio (Claude Desktop).

### Installation automatique dans Claude
- [ ] Réglages › « Claude » : un interrupteur « Claude peut piloter Escouade ». Activé, Escouade lance `claude mcp add --scope user --transport http escouade http://127.0.0.1:<port>/mcp --header "Authorization: Bearer <jeton>"` (portée `user` : tous les projets).
- [ ] Avant d'ajouter : `claude mcp get escouade` ; une entrée qui ne correspond plus (port, jeton) est remplacée (`remove` puis `add`), une entrée identique laissée telle quelle.
- [ ] Désactivé : `claude mcp remove escouade --scope user`, et le jeton est changé (l'ancien ne vaut plus rien).
- [ ] Une fois par compte Claude (chantier 2) : la même commande lancée avec le `CLAUDE_CONFIG_DIR` de chaque compte.
- [ ] Ce qui est fait se dit dans l'onglet (« Déclaré dans Claude · compte Principal, compte Pro ») ; un échec aussi, avec la commande à lancer à la main.
- [ ] Le jeton se retrouve en clair dans `~/.claude.json` (fichier de l'utilisateur) : le dire dans l'onglet. Autre piste à évaluer : `headersHelper`, un script qui lit le jeton dans le trousseau à chaque connexion (plus sûr, plus fragile).

### Agents lancés par Escouade
- [ ] Les agents d'Escouade reçoivent le serveur par `--mcp-config` (rien à écrire dans la config de l'utilisateur), avec un en-tête qui dit quel agent appelle : les outils savent alors ce que « mon ticket » veut dire.
- [ ] Réglage par projet : « Les agents peuvent utiliser Escouade » (défaut : non), car un agent qui crée des tickets ou lance d'autres agents dépense du quota.
- [ ] Les outils MCP passent par les mêmes demandes de permission que les autres (`mcp__escouade__*`) ; la carte de permission dit l'outil et ses arguments en clair.

### Outils, première version
Lecture :
- [ ] `list_projects`, `list_agents` (statut, attend une réponse, ticket, coût), `list_tickets` (par colonne, dépendances), `get_ticket` (description, critères, boucles, état de synchro), `get_usage` (quota, fenêtres, pause du pilote auto).
- [ ] `get_agent_summary` : dernier message et fichiers touchés, jamais la conversation entière (taille, et données qu'un autre client n'a pas à lire en bloc).

Actions :
- [ ] `create_ticket` (titre, description, critères, `after`), `update_ticket`, `move_ticket` ; mêmes règles que la fenêtre (cycles de dépendances refusés, synchro externe).
- [ ] `start_ticket`, `create_agent` (projet, message de départ, worktree ou non), `send_message` à un agent, `stop_agent` : soumis aux mêmes garde-fous que le pilote auto (agents en parallèle, pause au quota).
- [ ] Pour un agent d'Escouade : `report_progress` (une ligne d'état sur sa carte) et `split_ticket` (sous-tickets reliés par des dépendances).

Exclus de la v1, volontairement :
- [ ] Aucun outil qui accepte une permission, lance une commande ou un test, merge ou supprime (projet, agent, ticket) : un agent poussé par un contenu piégé pourrait s'en servir pour monter en droits. Plus tard, éventuellement, avec une confirmation dans la fenêtre (« Claude demande à supprimer DEM-4 »).

### Visibilité et tests
- [ ] Journal « Activité MCP » dans l'app : qui a appelé quel outil, quand, et le résultat (refus compris).
- [ ] Tests : serveur interrogé par un vrai client MCP (liste d'outils, appels, refus sans jeton, `Host` étranger refusé) ; le faux `claude` enregistre les `mcp add` / `remove` reçus ; e2e : activer l'interrupteur, voir l'entrée « déclarée », créer un ticket par MCP et le voir dans le Kanban.
- [ ] Docs : SPEC (section « Serveur MCP »), README, CHANGELOG, PROTOCOL.md (outils et formats).

## 2. Plusieurs comptes Claude, avec bascule automatique

But : plusieurs comptes Claude (perso, pro…) dans la même app ; chaque agent tourne sur un compte ; quand un compte atteint sa limite, Escouade continue sur un autre au lieu de mettre le pilote auto en pause.

### Ce qu'on sait
- Claude Code lit sa config, ses identifiants et l'historique de ses sessions dans un seul dossier : `CLAUDE_CONFIG_DIR` (défaut `~/.claude`). Un compte = un dossier ; deux dossiers restent connectés en même temps.
- Escouade aujourd'hui : un seul exécutable (`claude_path`) ; le quota est lu dans `~/.claude/.credentials.json`, ou dans le trousseau macOS « Claude Code-credentials » (`usage.rs`) ; un tour arrêté par la limite est détecté (`rate_limited`) et reprend à la remise à zéro ; la pause au-delà du quota est globale.
- Deux processus Claude ne doivent pas écrire la même session en même temps : une session reste dans le dossier de son compte.

### Plusieurs exécutables ?
- [ ] Pas besoin pour avoir plusieurs comptes : un seul `claude` suffit, lancé avec un `CLAUDE_CONFIG_DIR` différent.
- [ ] Mais un exécutable par compte, en option, sert pour essayer une autre version de Claude Code, ou un `claude` lancé autrement (WSL, wrapper). Défaut : celui des réglages.

### Modèle
- [ ] Réglages › « Comptes Claude » : liste de comptes `{ nom, dossier de config, exécutable (optionnel), actif, ordre }`. Le dossier `~/.claude` existant devient le compte « Principal » à la mise à jour, sans rien déplacer.
- [ ] « Ajouter un compte… » : Escouade crée `~/.escouade/claude/<nom>`, ouvre un terminal intégré avec ce `CLAUDE_CONFIG_DIR` et lance `claude` pour le `/login`, puis détecte que le compte est connecté (identifiants présents, quota lisible).
- [ ] Ce qui se partage entre comptes (`settings.json`, `CLAUDE.md`, skills, agents, plugins, hooks) : proposé à la création, par lien (jonction sous Windows, sans droits d'admin) ou par copie. Les identifiants ne sont jamais copiés.
- [ ] `AgentMeta.account` (serde default : le compte Principal) : un agent garde son compte, ce qui garde ses `--resume` valables.
- [ ] Choix du compte : à la création d'un agent (menu, défaut « Automatique ») et par projet (« Compte préféré »).

### Quota et bascule
- [ ] `usage.rs` par compte : identifiants lus dans le dossier de chaque compte (sous macOS, selon l'essai du trousseau) ; un appel par compte toutes les 5 min, comme aujourd'hui.
- [ ] **Barre d'état, compacte** : seulement les quotas du compte en cours d'utilisation, sous la forme `5h <barre> reset 3h01 | W <barre> reset 4j 12h` (aujourd'hui : « Session 5 h <barre> 42 % reset 3h01 » et « Hebdo <barre> 12 % reset 4j 12h »). Les libellés suivent la langue de l'interface (chantier 4) : en anglais `5h … reset 3h01 | W … reset 4d 12h`, en français `5h … reset 3h01 | 7j … reset 4j 12h` (« 7j » plutôt que « W », à valider), et de même pour l'infobulle et le panneau.
  - Le pourcentage n'est plus écrit : il s'affiche au survol de la barre, dans une infobulle (« 42 % · remise à zéro le 10/10 à 18:00 »). Pour le clavier et les lecteurs d'écran, la barre reste un `role="meter"` avec sa valeur, et l'infobulle s'affiche aussi au focus.
  - La barre passe à la couleur d'alerte au-delà de 80 %, comme aujourd'hui.
  - Le nom du compte en cours apparaît devant les quotas dès qu'il y a plusieurs comptes (« Pro · 5h <barre> … »), pour savoir de qui sont ces quotas.
- [ ] **Détail des autres comptes** : avec plusieurs comptes, tout le groupe des quotas de la barre d'état est un bouton. Au clic, un panneau s'ouvre au-dessus de la barre d'état : chaque compte avec ses deux fenêtres (barre, pourcentage écrit cette fois, remise à zéro), le compte en cours en tête et marqué, un compte au-delà du seuil de pause signalé, un compte inactif ou déconnecté grisé avec sa raison. Échap, un clic dehors ou un nouveau clic sur le groupe le referment ; le focus va dans le panneau à l'ouverture et revient au bouton à la fermeture. Avec un seul compte, le groupe n'est pas cliquable.
- [ ] « Automatique » : un nouvel agent ou ticket part sur le premier compte actif sous le seuil de « Pause au-delà du quota », dans l'ordre de la liste. Le pilote auto ne se met en pause que quand tous les comptes ont passé le seuil.
- [ ] Toast à chaque bascule : « Le compte Pro a atteint 95 % : les nouveaux agents partent sur Principal. »
- [ ] Agent arrêté par la limite en plein tour : aujourd'hui il attend la remise à zéro. Nouveau, si l'essai de reprise entre comptes marche : « Reprendre sur <compte> » sur sa carte de fin de tour, et automatique pour un agent de ticket si le réglage le permet (la session est copiée dans le dossier de l'autre compte, l'agent change de compte).
- [ ] Serveur MCP (chantier 1) déclaré dans chaque compte ; un compte ajouté plus tard le reçoit aussi.
- [ ] **Statistiques globales et par compte** :
  - Chaque tour enregistre son compte (colonne `account` dans la table des tours, comme `ticket_id` en 1.6 ; les tours d'avant la 1.7 comptent pour « Principal »).
  - Vue « Stats » : un sélecteur « Tous les comptes » (défaut : les totaux globaux, comme aujourd'hui) / un compte, qui filtre toutes les vues (période, « Par agent », « Par ticket ») ; une vue « Par compte » qui met les comptes côte à côte (coût, tokens, tours, agents, sur la période choisie).
  - « Aujourd'hui » dans la barre d'état reste le total de tous les comptes ; le panneau des comptes donne celui de chaque compte.
  - Un agent repris sur un autre compte compte ses tours pour le compte de chaque tour, pas pour celui de départ.

### Tests et docs
- [ ] Le faux `claude` respecte `CLAUDE_CONFIG_DIR` (ses sessions et sa limite simulée par dossier) ; tests du choix de compte, de la bascule, de la pause quand tous sont au seuil, du compte gardé par un agent repris.
- [ ] e2e : deux comptes simulés, le premier atteint la limite, le ticket suivant part sur le second.
- [ ] Docs : SPEC (« Comptes Claude »), README, CHANGELOG ; dire clairement ce qui reste partagé entre comptes et ce qui ne l'est pas.

## 3. Branches : en créer, en changer, les tenir à jour

But : gérer les branches depuis Escouade sans passer par un terminal.

### Ce qui existe aujourd'hui
- Chaque agent avec worktree a sa branche (`ticket/<clé>` ou celle de l'agent), créée depuis la branche de base du projet ; elle se renomme, se pousse, et se merge à la validation (squash, merge ou rebase ; « Ouvrir une pull request » en option).
- La liste des branches ne sert qu'à choisir la branche cible dans les réglages du Kanban ; le graphe git est en lecture seule (il ouvre un commit) ; `fetch` et `pull` existent pour la synchro.

### À faire
- [ ] **Sélecteur de branche** du projet (barre d'état ou onglet Git) : la branche courante du dossier du projet, la liste des branches locales (la courante en tête) et distantes, une recherche.
- [ ] **Changer de branche** dans le dossier du projet : refusé tant qu'il reste des changements non commités, avec le choix « Mettre de côté (stash) et changer » ; une branche déjà prise par un worktree d'agent est signalée (« utilisée par l'agent X ») au lieu d'échouer sur l'erreur de git.
- [ ] **Créer une branche** : nom (vérifié comme git le fait : `git check-ref-format`), point de départ (branche courante, une autre branche, un commit du graphe), « et y passer » coché par défaut.
- [ ] **Lancer un agent sur une branche existante** (une branche distante comprise, suivie en local) au lieu d'une branche neuve ; et un ticket pareil (« Reprendre la branche … »).
- [ ] **Mettre à jour la branche d'un agent avec sa base** : « Intégrer <base> » (merge ou rebase, le choix du projet) ; un conflit est rendu à l'agent avec la liste des fichiers en conflit, comme une demande de correction.
- [ ] **Depuis le graphe git** : clic droit sur un commit ou une branche → « Créer une branche ici », « Passer sur cette branche », « Comparer avec la branche courante » (le diff des deux, dans la fenêtre de diff existante).
- [ ] **Supprimer une branche** : seulement une branche mergée dans la base, ou après confirmation (« n commits ne sont dans aucune autre branche ») ; jamais la branche courante ni celle d'un worktree d'agent ; la branche distante en option.
- [ ] **Nettoyage** : « Branches mergées » liste les branches locales déjà dans la base (les branches de ticket validés surtout) et propose de les supprimer en une fois.
- [ ] **Avance et retard** sur la branche distante (« ↑2 ↓5 ») à côté du nom de branche, avec « Récupérer » (pull) et « Pousser ».
- [ ] Tests : dépôts git réels dans les tests Rust (changer avec et sans changements, branche prise par un worktree, création depuis un commit, suppression refusée, intégration de la base avec et sans conflit) ; composants (sélecteur, confirmations) ; un e2e : créer une branche, y passer, la voir dans le graphe.
- [ ] Docs : SPEC (section Git), README, CHANGELOG.

## 4. Langues : système par défaut, anglais, français

But : toute l'interface en anglais ou en français ; par défaut, la langue du système (français si elle commence par « fr », anglais sinon).

### Ampleur
- Environ 1 500 textes côté fenêtre (Svelte et TypeScript) et quelques centaines côté Rust (erreurs montrées telles quelles, toasts, notifications du système, menus de la barre des tâches et de l'app macOS, commentaires publiés dans Jira, Trello et GitHub). À compter précisément au début du chantier.
- Une dizaine d'endroits formatent en dur à la française (`toLocaleString('fr-FR')`, montants « 0,10 $ », durées « 4j 12h »).

### Réglage
- [ ] Réglages › Application : « Langue » avec « Système (par défaut) », « English », « Français ». Le changement s'applique tout de suite, sans redémarrer, fenêtre et menus compris.
- [ ] La langue du système est lue par Rust (crate `sys-locale` ou équivalent, à vérifier) et envoyée à la fenêtre, pour que les deux côtés soient d'accord.

### Mécanique
- [ ] Fenêtre : un catalogue par langue, aux clés typées (une clé absente ou des paramètres qui diffèrent d'une langue à l'autre ne compilent pas), avec interpolation et pluriels (`Intl.PluralRules` : « 1 fichier », « 2 fichiers » ; « 1 file », « 2 files »). Choisir entre un petit module maison et une bibliothèque compilée (Paraglide…) au début du chantier.
- [ ] Rust : les textes qui arrivent à l'écran passent par un catalogue de même forme côté Rust ; quand c'est simple, le backend envoie un code et ses paramètres, et c'est la fenêtre qui rédige.
- [ ] Formats selon la langue, en un seul endroit (`format.ts` et son pendant Rust) : nombres, montants (« 0,10 $ » / « $0.10 »), dates, durées et comptes à rebours (« 4j 12h » / « 4d 12h »), tailles de fichiers.
- [ ] Quotas : les libellés compacts de la barre d'état, les infobulles et le panneau des comptes (chantier 2) dans la langue choisie.
- [ ] Menus natifs (barre des tâches, menu de l'app sous macOS) reconstruits au changement de langue.
- [ ] Installateur Windows (NSIS) en anglais et en français (configuration du bundle Tauri, à vérifier).

### Textes rédigés par Claude
- [ ] Les textes qu'Escouade fait écrire à Claude (message de commit proposé, description de PR, commentaires publiés dans Jira, Trello et GitHub, commandes de lancement suggérées et leurs noms) : réglage « Langue des textes rédigés par Claude », avec « Comme l'interface (par défaut) », « English » et « Français », car beaucoup d'équipes commitent en anglais même avec une interface en français.
- [ ] Les consignes qu'Escouade ajoute aux agents (message système ajouté, invites du pilote auto) dans cette langue aussi.

### Tests
- [ ] Un test vérifie que les deux catalogues ont les mêmes clés et les mêmes paramètres.
- [ ] Un test (ou une règle de lint) repère un texte écrit en dur dans un composant, hors catalogue.
- [ ] Les tests existants tournent en français (leurs attentes ne changent pas) ; une série de tests en anglais couvre les formats, les pluriels et quelques écrans ; un e2e de bout en bout en anglais.

### Site (GitHub Pages)
Le site est une page Nuxt générée en statique (`website/`, preset `github_pages`), dont les textes sont dans `website/app/data/site.ts`.
- [ ] Deux versions générées : le français à la racine (les liens existants restent bons) et l'anglais sous `/en/` ; `<html lang>`, titre, description et balises de partage (Open Graph) dans la langue de la page ; balises `hreflang` entre les deux.
- [ ] Les textes de `site.ts` en deux catalogues de mêmes clés (un test le vérifie, comme pour l'app) ; liens de téléchargement, version et captures communs.
- [ ] Sélecteur « FR / EN » dans l'en-tête ; le choix est gardé (stockage local). GitHub Pages ne lit pas la langue du navigateur côté serveur : à la première visite de la racine, un petit script envoie vers `/en/` un navigateur qui n'est pas en français, sauf choix gardé ou lien explicite vers la version française.
- [ ] Captures d'écran : celles de l'app en anglais pour la version anglaise, une fois l'app traduite.
- [ ] Vidéo : sa voix est en français. Pour la version anglaise, des sous-titres anglais d'abord ; une voix anglaise est à décider (nouvelle piste ElevenLabs, même montage).
- [ ] Les tests du site (`website/tests`) tournent sur les deux langues ; `nuxt generate` produit les deux pages.

### Docs
- [ ] SPEC : la règle des langues et des formats. README et CHANGELOG restent en français ; un README en anglais (`README.en.md`) est à décider.

## Ordre proposé

1. Langues, la mécanique et l'extraction des textes existants (app, puis site) : sinon chaque texte des autres chantiers serait écrit en dur puis repris. Les textes nouveaux s'écrivent ensuite directement dans les deux langues.
2. Les essais du point 0 : ils décident de la forme de la bascule et du support macOS.
3. Comptes multiples sans bascule (liste, ajout, compte par agent, quota par compte, barre d'état compacte, statistiques par compte).
4. Serveur MCP en lecture seule, avec l'installation automatique dans chaque compte.
5. Bascule automatique (nouveaux agents, puis reprise sur un autre compte si l'essai l'a permis).
6. Outils MCP qui agissent, puis les outils réservés aux agents d'Escouade.

Les branches (chantier 3) ne dépendent pas des autres : elles peuvent avancer en parallèle dès que la mécanique des langues est en place. Les outils MCP pourront ensuite s'en servir (créer une branche, lancer un agent dessus).
