# Escouade — Spécification v1

Anciennement « Claude Code Manager » (jusqu'à la 0.1.3 pour les noms internes). Au premier lancement de la 0.1.4, ce qui existait sous les anciens noms est déplacé : dossier `~/.claude-code-manager/` → `~/.escouade/`, dossiers de l'identifiant `dev.gagnaire.claude-code-manager` → `dev.gagnaire.escouade` (WebView, taille de fenêtre), préférences `ccm.*` → `escouade.*`. La variable `CCM_DATA_DIR` reste acceptée à côté d'`ESCOUADE_DATA_DIR`. Les nouveaux agents travaillent sur des branches `escouade/` ; ceux d'avant gardent leurs branches `ccm/`. La mise à jour réapplique le nouvel identifiant aux raccourcis, dont Windows se sert pour les notifications.

Application desktop pour piloter plusieurs instances de Claude Code en local, organisées par projet.
Référence visuelle : `design/Claude Code Manager.dc.html` (source de vérité pour le look & feel).

## Stack

| Couche | Choix |
|---|---|
| Shell applicatif | Tauri 2 (Windows uniquement en v1) |
| Backend | Rust (tokio) : process `claude`, PTY (portable-pty), git (gix/CLI), watchers (notify), SQLite (rusqlite) |
| Frontend | Svelte 5 + TypeScript + Vite |
| Terminaux | xterm.js (addon WebGL) |
| Markdown | rendu incrémental en streaming + coloration syntaxique (shiki) |
| Stockage | `~/.escouade/` (config JSON + SQLite) |

## Pilotage de Claude Code

- Chat natif uniquement (pas de TUI embarqué).
- Chaque agent = un process `claude -p --input-format stream-json --output-format stream-json --verbose --include-partial-messages` dans le dossier du projet (ou du worktree de l'agent).
- Les demandes de permission et `AskUserQuestion` remontent via le protocole de contrôle (permission prompts « host ») et s'affichent comme cartes « Claude attend ta réponse ».
- Modèle, effort et mode de permission changeables en cours de conversation.
- **1 agent = 1 conversation.** Reprise après redémarrage via `--resume <session_id>`.

## Fenêtre

- Fenêtre sans cadre : la barre d'onglets du design sert de barre de titre (zone de drag), boutons Windows ─ ☐ ✕ à droite (à la place des pastilles macOS).
- Thème sombre du design, teinté par la couleur du projet actif (`color-mix` sur les tokens).
- Polices Hanken Grotesk + JetBrains Mono embarquées (hors-ligne).
- Fermer la fenêtre → réduction dans le tray, les agents continuent. « Quitter » (menu tray) → arrêt propre ; au relancement, chaque agent reprend sa session.

## Barre d'onglets

- Un onglet par projet : pastille couleur, nom, point vert si un agent tourne, compteur `Δ n` (fichiers non commités, worktrees des agents inclus), pastille pulsante = nombre d'agents en attente.
- `+` ouvre la modale « Nouveau projet ». Onglet « Stats » à droite.
- Défauts : onglets réordonnables par glisser ; clic droit → renommer, couleur, commandes de lancement, fermer le projet (retire de l'app, ne touche pas au disque).

## Modale « Nouveau projet »

Dossier (+ Parcourir…), détection git (sinon `git init`), nom, aperçu d'onglet, 14 couleurs, bascule « Créer un premier agent » (+ modèle), bascule « Un worktree git par agent ».

## Barre latérale projet

- **Agents** : compteur, « + Nouvel agent ». Carte : statut (Prêt / En cours / Question / Terminé), nom, modèle · durée active, tokens · coût · nb fichiers.
- Nom créé en `agent-N`, puis renommé automatiquement en slug par Haiku après le premier message. Renommable (double-clic).
- Clic droit : Renommer, Archiver (historique du projet, réouvrable), Ouvrir dans l'éditeur, Supprimer (arrête le process, supprime le worktree après confirmation si non mergé).
- **Remote control** (clic droit, par agent, désactivé par défaut) : la session de l'agent devient accessible depuis claude.ai et l'app Claude mobile, sous le nom « projet · agent » (requête de contrôle `remote_control`). Son process reste lancé (démarré avec l'app, jamais arrêté pour inactivité) et retrouve la même session distante après un redémarrage (`reattach_session_id`, `keep_session_on_exit`), donc le lien reste valable. Les messages envoyés depuis claude.ai s'affichent dans l'app (« depuis claude.ai ») grâce à `--replay-user-messages` ; l'écho des messages envoyés depuis l'app est écarté par son uuid. Icône sur la carte (connecté / en attente) ; clic droit : Ouvrir sur claude.ai, Copier le lien. Archiver ou supprimer l'agent met fin à sa session distante.
- **Lancement** (entre Agents et Terminaux) : les commandes qui lancent le projet, configurées via ⚙ ou le clic droit sur l'onglet (nom, ligne de commande, shell parmi ceux détectés, sous-dossier optionnel ; enregistrées avec le projet). Chacune tourne dans son propre terminal ConPTY (`pwsh -Command …`, `bash --login -c …`, `wsl -- bash -lc …`), sans saisie : un clic sur la commande affiche son log dans la zone principale (xterm.js en lecture seule, gardé d'un lancement à l'autre avec un séparateur « relancé à HH:MM »). Statut en direct : prêt, en cours, arrêté (stoppé depuis l'app), terminé (code 0), planté (code N, avec une notification). Lancer ▶, relancer ⟳, stopper ■ (arrête aussi les programmes lancés par la commande), « Tout lancer » / « Tout arrêter ». Fermer le projet arrête ses commandes ; elles ne survivent pas à un redémarrage.
- **Terminaux** : menu `+` → PowerShell 7 (cherché dans le PATH, Program Files et l'alias du Microsoft Store ; à défaut, Windows PowerShell 5.1), Git Bash, WSL (Ubuntu). Vrai terminal (ConPTY via portable-pty + xterm.js WebGL) : autocomplétion native du shell (Tab / PSReadLine / bash-completion), historique, Ctrl+R, couleurs ANSI, programmes plein écran (vim, less, htop), redimensionnement, copier/coller, liens cliquables, recherche. Les terminaux ne survivent pas à un redémarrage.
- **Pied** : chemin, branche (et « Parcourir » : l'éditeur sur la branche du projet), `~ modifiés / + ajoutés / − supprimés`, sélecteur de couleur.

## Conversation

- En-tête : nom, statut, projet / branche ; modèle, tokens, coût, « Fichiers ▸ » (ouvre le panneau), durée ; sélecteur de disposition. Quand la colonne est étroite, tokens et durée (puis coût et fichiers) s'effacent : la carte de l'agent les affiche aussi.
- **Disposition** (globale, mémorisée dans `state.json`, `Ctrl+Maj+L`) : classique (panneau des fichiers à ouvrir) ou **moitié / moitié** : conversation à gauche, à droite le panneau « Non commités » toujours visible avec, sous la liste, le diff du fichier sélectionné (premier fichier par défaut, rafraîchi pendant que l'agent édite).
- Messages : utilisateur (bulle à droite), assistant (markdown), lignes d'outils compactes **dépliables** (diff pour Edit/Write, unifié ou côte à côte selon le style choisi dans les vues de diff ; sortie pour Bash…), réflexion repliée dépliable.
- Carte question (AskUserQuestion / permission) : options en boutons, réponse libre possible dans le composer ; carte répondue grisée « → réponse ».
- Carte « Tâche terminée » : durée, tokens, coût, fichiers ; « Revoir les fichiers », « Commit… ».
- Indicateur « Claude travaille… » + bouton Stop (interrompre).
- Composer : menus déroulants ouverts vers le haut — Modèle (Fable · Opus · Sonnet · Haiku, alias CLI), Effort (Bas · Moyen · Élevé · Très élevé · Max), Mode (Auto · Demander · Plan · Édits auto · Bypass ; **Auto par défaut**) — puis Envoyer (↵), toujours sur une seule ligne : dans une colonne étroite, les libellés des menus s'effacent et Stop se réduit à ■. Bordure jaune si une question attend.
- Composer v1 : fichiers joints — images (PNG, JPEG, GIF, WebP ; 5 Mo), PDF (20 Mo), fichiers texte UTF-8 (1 Mo) — par le trombone, le collage ou le glisser-déposer (tout autre fichier est refusé avec un message), autocomplétion `@fichier`, slash commands (intégrées + `.claude/commands` + skills), messages envoyés pendant que Claude travaille : transmis tout de suite, le CLI les intègre à l'étape suivante du tour (mention « transmis pendant le tour » jusqu'à la fin du tour).

## Git & fichiers

- Panneau « Non commités » : portée « Cet agent » / « Tout le projet », attribution par worktree (ou par outils Edit/Write de l'agent hors mode worktree). Clic → diff ; bouton `</>` ou clic droit → éditeur.
- Onglet « Historique » du même panneau : git graph des 300 derniers commits de tout le dépôt (branches locales et distantes, tags, branches `ccm/…` des agents étiquetées de leur nom). La branche de l'agent (son worktree, sinon la branche courante) est mise en avant, le reste atténué. Clic sur un commit → son diff (contre le premier parent pour un merge) dans la vue diff. Rafraîchi à chaque commit, merge ou branche.
- « Voir le diff » : vue diff plein écran (unifiée / côte à côte ; choix partagé avec le volet de la disposition moitié / moitié et les diffs de la conversation, et mémorisé).
- « Commit… » : envoie à l'agent une demande de commit de ses changements (il rédige le message).
- Mode worktree : `<projet>/.claude/worktrees/<agent>`, branche dédiée. Bouton « Merger dans <branche> » (merge ou squash) ; suppression de l'agent → nettoyage worktree + branche.
- Compteurs git rafraîchis par watcher de fichiers (debounce), pas par polling.
- Synchro du checkout principal du projet avec son dépôt distant : commits à tirer / à pousser par rapport à la branche suivie. Fetch en arrière-plan peu après le démarrage puis toutes les 5 min, un dépôt à la fois, sans jamais demander d'identifiants (ni fenêtre Git Credential Manager) ; échecs seulement journalisés. Pull en avance rapide uniquement (branches divergées : message clair, rebase ou merge à faire à la main), Push (une branche sans branche suivie, ou dont la branche distante a été supprimée, est publiée sur origin, ou l'unique dépôt distant), Fetch à la demande. Rien à synchroniser en HEAD détachée ou sans dépôt distant.

## Éditeur

- Mode éditeur dans la zone principale d'un projet. Points d'entrée : « `</>` Éditeur » dans l'en-tête d'un agent et « Ouvrir dans l'éditeur » au clic droit sur un agent (son worktree, ou le projet sans worktree) ; « Parcourir » sur la ligne de la branche, dans le pied de la barre latérale, et « Éditeur » dans la barre d'onglets (branche du projet) ; bouton `</>` et clic droit sur un fichier non commité ; dans la conversation, le chemin du fichier des lignes Edit / Write / MultiEdit (ouvert à la première ligne modifiée) et les fichiers listés par la carte « Tâche terminée ». Un fichier en dehors du dossier de la source ne s'ouvre pas (message). Remplace l'ouverture dans un éditeur externe.
- Choisir un agent dans la barre latérale bascule l'éditeur sur sa source ; « + Nouvel agent », un terminal, une commande de lancement, une notification ou `Ctrl+J` ferment l'éditeur pour montrer ce qu'ils ouvrent (ses onglets sont gardés).
- Sélecteur de source : branche du projet ou worktree d'un agent (nombre de modifications) ; arborescence des fichiers connus de git (ignorés exclus), lettres M / A, onglets avec point « non enregistré ».
- CodeMirror 6 aux couleurs du design : coloration, recherche (Ctrl+F), indentation détectée, marques des lignes modifiées par rapport à HEAD (projet) ou à la branche de départ (worktree), compte dans le fil d'Ariane (« N lignes modifiées vs main »).
- Ctrl+S ou « Enregistrer » enregistre (fins de ligne et BOM conservés, écriture atomique). Un fichier changé sur le disque se recharge s'il n'est pas modifié ; sinon un bandeau propose « Recharger » ou « Garder ma version ». Un fichier supprimé du disque : bandeau « Ce fichier a été supprimé. » avec « Fermer » ou « Le recréer en enregistrant ». Fermer un onglet ou quitter avec des fichiers non enregistrés demande confirmation ; supprimer un agent ou fermer un projet prévient des fichiers non enregistrés qui seront perdus.

## Notifications

- Déclencheurs : question de Claude (AskUserQuestion / permission) et fin de tour.
- Dans l'app : pastilles pulsantes (onglet, carte agent, barre de statut) + carillon du design (2 notes synthétisées), coupable via « ♪ On/Off ».
- App en arrière-plan / tray : toast Windows (clic → ouvre l'agent) + clignotement barre des tâches + badge tray.

## Barre de statut

Actifs · en attente · terminés │ Session 5 h (barre, %, reset dans) · Hebdo (barre, %) │ Coût du jour │ ⎇ branche ↓ à tirer ↑ à pousser │ ♪ · ⚙ (réglages).

- Synchro git : pour le projet affiché, si son dépôt a un dépôt distant et une branche. ↓ en couleur d'attente quand il y a des commits à tirer ; infobulle : branche suivie et dernier fetch. Clic → menu Pull · Push (« Publier la branche » sans branche suivie ou si elle a été supprimée du dépôt distant) · Fetch ; l'action en cours s'affiche (« Pull… »), puis un toast résume (« 3 commits tirés »).

- Quotas : endpoint utilisé par `/usage` (token OAuth de Claude Code, lecture seule), sauf si une source officielle équivalente existe dans le stream.
- Coûts : équivalent API (compte Max). Le coût exact de chaque tour vient de Claude Code à la fin du tour ; pendant le tour, tokens et coût (préfixé « ≈ ») montent en direct, estimés à partir des tarifs publics (`pricing.rs`), puis remplacés par le chiffre exact. Les statistiques n'enregistrent que les coûts exacts.

## Stats

- Périmètre : uniquement les agents lancés par l'app (stockage SQLite).
- Plages : Jour (14 j) / Semaine (12 sem.) / Mois (12 mois).
- KPIs : Tokens (+ évolution vs période précédente), Coût global (depuis le premier agent), Coût moyen / prompt, Prompts.
- Histogramme empilé 3 séries : entrée, cache, sortie. Répartition par projet et par modèle.

## Réglages (⚙ barre de statut → modale)

Chemin de `claude`, modèle / effort / mode par défaut, son, notifications Windows, shells, arrêt des process inactifs, raccourcis.

- **Proxy réseau** : URL HTTP(S) (avec identifiants éventuels) + exclusions `NO_PROXY`. Injecté dans les process `claude` (`HTTPS_PROXY` / `HTTP_PROXY` / `NO_PROXY`), les appels de quotas et le vérificateur de mises à jour ; option pour l'exporter aussi dans les terminaux intégrés.
- Les coûts enregistrés viennent directement de Claude Code (`costUSD` par modèle). La grille de `pricing.rs` ne sert qu'à l'estimation en cours de tour ; un modèle absent de la grille n'a simplement pas d'estimation.

## Raccourcis (défauts)

`Ctrl+1..9` projets · `Ctrl+N` nouvel agent · `Ctrl+J` prochain agent en attente · `Ctrl+Maj+L` disposition · `Ctrl+S` enregistrer (éditeur) · `Échap` interrompre · `Ctrl+,` réglages.

## Livraison

- UI en français.
- Installeur Windows (NSIS) + auto-update via GitHub Releases (`guillaume-gagnaire/escouade`). L'app cherche une nouvelle version 8 s après son lancement puis toutes les 5 minutes, sans rien dire en cas d'échec ; « Rechercher une mise à jour » dans les réglages vérifie à la demande.
- CI/CD GitHub Actions :
  - `ci.yml` (push / PR) : lint + typecheck + tests frontend, `cargo fmt --check`, `clippy`, `cargo test`, build de vérification.
  - `release.yml` (tag `v*`) : `tauri-action` → build Windows, signature des artefacts de mise à jour (secret `TAURI_SIGNING_PRIVATE_KEY`), publication de la release GitHub avec l'installeur et `latest.json` consommé par l'updater.
- Commits atomiques sur `main`, sans push.
