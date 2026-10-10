# Escouade 1.7 — à faire

Deux chantiers : un serveur MCP pour piloter Escouade depuis Claude, et plusieurs comptes Claude avec bascule automatique. Les deux se touchent : chaque compte a son propre `~/.claude.json`, donc le serveur MCP doit être déclaré dans chacun.

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
- [ ] Barre d'état : le compte actif et son usage ; au clic, tous les comptes avec leurs fenêtres (5 h, semaine) et leurs remises à zéro.
- [ ] « Automatique » : un nouvel agent ou ticket part sur le premier compte actif sous le seuil de « Pause au-delà du quota », dans l'ordre de la liste. Le pilote auto ne se met en pause que quand tous les comptes ont passé le seuil.
- [ ] Toast à chaque bascule : « Le compte Pro a atteint 95 % : les nouveaux agents partent sur Principal. »
- [ ] Agent arrêté par la limite en plein tour : aujourd'hui il attend la remise à zéro. Nouveau, si l'essai de reprise entre comptes marche : « Reprendre sur <compte> » sur sa carte de fin de tour, et automatique pour un agent de ticket si le réglage le permet (la session est copiée dans le dossier de l'autre compte, l'agent change de compte).
- [ ] Serveur MCP (chantier 1) déclaré dans chaque compte ; un compte ajouté plus tard le reçoit aussi.
- [ ] Coûts et statistiques : colonne « Compte » dans « Par agent » / « Par ticket ».

### Tests et docs
- [ ] Le faux `claude` respecte `CLAUDE_CONFIG_DIR` (ses sessions et sa limite simulée par dossier) ; tests du choix de compte, de la bascule, de la pause quand tous sont au seuil, du compte gardé par un agent repris.
- [ ] e2e : deux comptes simulés, le premier atteint la limite, le ticket suivant part sur le second.
- [ ] Docs : SPEC (« Comptes Claude »), README, CHANGELOG ; dire clairement ce qui reste partagé entre comptes et ce qui ne l'est pas.

## Ordre proposé

1. Les essais du point 0 : ils décident de la forme de la bascule et du support macOS.
2. Comptes multiples sans bascule (liste, ajout, compte par agent, quota par compte).
3. Serveur MCP en lecture seule, avec l'installation automatique dans chaque compte.
4. Bascule automatique (nouveaux agents, puis reprise sur un autre compte si l'essai l'a permis).
5. Outils MCP qui agissent, puis les outils réservés aux agents d'Escouade.
