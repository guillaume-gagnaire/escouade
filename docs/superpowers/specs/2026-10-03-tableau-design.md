# Tableau (kanban d'agents autonomes) — design

## But

Planifier des tickets par projet ; des agents se créent seuls pour les prendre, bouclent jusqu'à atteindre les critères d'acceptation, puis passent le ticket « À tester ». L'utilisateur teste (avec un lancement de test du worktree sur ses propres ports), valide ou renvoie. Ce qui se passe à la validation (merge, PR, push…) se règle par projet, comme le nombre maximal d'agents en parallèle.

Référence visuelle : écrans « Tableau » et « Réglages du tableau » de `design/Claude Code Manager.dc.html` (projet claude.ai/design 9047fbff…, relu le 2026-10-03).

Décisions validées le 2026-10-03 :

- un agent neuf par ticket, dans son propre worktree ;
- **auto-évaluation** : l'agent termine chaque tour par un bilan structuré des critères ; l'app relance tant qu'il en manque ;
- « Renvoyer » rend le ticket au même agent avec un commentaire ;
- un ticket a un titre, une description et des critères ;
- nombre maximal d'agents en parallèle réglable ;
- **lancement de test** : ports réservés par worktree, recette de lancement écrite par l'agent (avec l'adresse qui montre directement la fonctionnalité), bouton « ▶ Tester » qui ouvre une modale, lance les serveurs, attend qu'ils répondent en HTTP puis ouvre le navigateur sur cette adresse ; pour les tickets et pour tout agent à worktree.

## Ticket

| Champ | Contenu |
|---|---|
| clé | `<PRÉFIXE>-<n>` : préfixe = 3 premières lettres du nom du projet en majuscules (`TIC` à défaut), fixé au premier ticket ; numéro croissant par projet, jamais réutilisé |
| titre, description | texte ; description facultative (contexte, contraintes, fichiers) |
| critères | liste ordonnée ; chacun avec son dernier état (`ok`) et la note de l'agent |
| avancement | liste succincte des fonctionnalités en place, donnée par l'agent à chaque bilan (3 à 8 éléments courts ; le dernier bilan reçu remplace le précédent) |
| boucles max | 3, 5 ou 8 (5 par défaut) |
| colonne | À faire · En cours · À tester · Terminé |
| rang | ordre dans « À faire » (priorité) |
| agent, boucle courante | l'agent du ticket, `n` de « Boucle n/max » |
| partiel | passé « À tester » sans tous ses critères (limite de boucles) |
| bloqué | raison (« Interrompu », « Erreur : … », « Bilan des critères manquant », « Conflit avec main », message d'une validation ratée), ou rien |
| en cours | étape de validation affichée (« Tests… », « Commit… », « Merge… », « Push… »), ou rien |
| issue | texte de fin (« ⤵ Mergé dans main · squash », « ⇡ PR #12 → main » + lien…) |
| dates | création, départ, passage « À tester », fin |

Tickets et réglages sont enregistrés avec l'état de l'app (`~/.escouade/state.json`), pas dans le dépôt. Fermer un projet supprime ses tickets.

## Réglages du tableau (par projet)

Modale « Réglages du tableau » (bouton « ⚙ Après validation : … » de l'en-tête du tableau), appliqués au clic, « Terminé » ferme.

- **Quand je valide un ticket « À tester »** (4 cartes) : Merger dans une branche · Ouvrir une pull request · Pousser la branche du ticket · Laisser en l'état. Défaut : merger.
- **Branche cible** (merge, PR) : liste des branches locales ; défaut : branche courante du projet au premier ticket, même sans commit (dépôt neuf) ; sur une HEAD détachée, le premier ticket est refusé (« Le projet n'est sur aucune branche : choisis la branche cible dans les réglages du tableau. »).
- **Stratégie** (merge) : Merge commit · Squash · Rebase. Défaut : squash.
- **Ouvrir en brouillon** (PR).
- **Relancer les tests avant** + champ **Commande de tests** (ex. `npm test`) : bloque l'action si les tests échouent et renvoie le ticket à l'agent. Interrupteur inactif tant que la commande est vide.
- **Supprimer le worktree après merge** (défaut : oui).
- **Message de commit généré** : Conventional Commits avec la clé (défaut : oui) ; aperçu du message sous l'interrupteur.
- **En cas de conflit** : Me demander · L'agent résout · Annuler. Défaut : me demander.
- **Agents** : **en parallèle** 1 à 6 (défaut 2) ; **modèle**, **effort** et **mode** des agents de ticket (défaut : ceux des réglages de l'app).
- **Pilote auto** : interrupteur de l'en-tête du tableau (défaut : activé).

Libellé de l'en-tête : `merge squash → main`, `PR → main`, `push ticket/*`, `laisser en l'état`. Libellé du bouton de validation : « Valider et merger », « Valider + PR », « Valider et pousser », « Valider ».

## Cycle d'un ticket

### Départ

Le planificateur tourne à chaque changement (ticket créé, modifié, déplacé, fin de tour, réglages, agent archivé ou supprimé, démarrage de l'app). Tant que le nombre de tickets « En cours » non bloqués est sous le maximum, il démarre le premier ticket « À faire » (par rang) si le pilote auto est activé ; sinon seulement ceux lancés à la main (« Lancer »). Il ne démarre rien tant qu'un agent de l'app attend la reprise après une limite d'usage (« Quota atteint — reprise à HH:MM » dans l'en-tête), ni tant que la branche cible n'a aucun commit (« <cible> n'a encore aucun commit — aucun ticket ne démarre ») ou n'existe pas (« Branche cible <cible> introuvable — aucun ticket ne démarre ») : l'en-tête le dit à la place des places libres, les cartes « À faire » disent « En attente de la branche cible », et le rafraîchissement git suivant du projet (premier commit, branche recréée) relance la file.

Démarrer un ticket :

1. crée un agent nommé `<clé>-<slug du titre>` (ex. `atl-42-limiter-tentatives`, déjà nommé : pas de renommage par Haiku), avec le modèle, l'effort et le mode du tableau ;
2. lui crée **toujours** un worktree, même si le projet n'est pas en mode « un worktree par agent » : branche `ticket/<clé en minuscules>` (suffixe `-2`… si elle existe) partant de la **branche cible** ;
3. lui réserve un bloc de ports (voir *Lancement de test*) et copie les fichiers à copier dans les worktrees ;
4. lance son process avec `--append-system-prompt` = protocole du ticket (survit à la compaction et à la reprise) ;
5. lui envoie le premier message : clé, titre, description, critères numérotés, « Boucle 1/max ».

### Protocole (prompt système ajouté)

L'agent travaille sur le ticket ; il vérifie lui-même chaque critère (tests, exécution) avant de le déclarer atteint ; s'il lui faut une décision, il utilise l'outil de question ; à la fin de **chaque** réponse, il écrit un bloc :

````
```escouade
{"criteres": [{"n": 1, "ok": true, "note": "vérifié par tests/auth.test.ts"}, {"n": 2, "ok": false, "note": "reste la rotation"}],
 "avancement": ["Tokens d'accès signés et vérifiés", "Middleware requireSession réécrit", "Adaptateur des sessions legacy"],
 "lancement": { … facultatif, voir Lancement de test … }}
```
````

`avancement` : l'état d'avancement, liste succincte des fonctionnalités mises en place jusque-là (3 à 8 éléments, une ligne chacun) ; un bilan sans `avancement` garde le précédent. Le prompt rappelle la liste des critères numérotés, les ports réservés au worktree, et qu'il ne faut pas changer les ports dans des fichiers versionnés (arguments ou variables d'environnement).

### Fin de tour

Le cœur signale chaque fin de tour (fin de `result`) au tableau avec : erreur ou non, interrompu ou non, texte final de l'assistant. Pour un agent de ticket « En cours » :

- **interrompu** (Stop) → bloqué « Interrompu » ;
- **erreur** : limite d'usage → la reprise automatique existante relance l'agent, le ticket attend (« Reprise à HH:MM ») ; autre erreur → bloqué « Erreur : … » ;
- sinon on lit le **dernier bloc `escouade`** du texte final (JSON ; `n` à partir de 1 ; un critère absent compte comme non atteint ; clé `criteria` acceptée aussi) :
  - **absent ou illisible** : un rappel « Termine par le bilan des critères (bloc escouade). » (ne compte pas comme une boucle) ; si le tour suivant n'en a toujours pas → bloqué « Bilan des critères manquant » ;
  - **tous atteints** → « À tester » ;
  - **boucle = max** → « À tester » avec « Objectif partiel » ;
  - **sinon** → boucle + 1 et message « Boucle n/max. Critères non atteints : 2 (note), 4 (note). Continue jusqu'à les atteindre, puis termine par le bilan. »

Un ticket bloqué garde son agent mais ne compte plus dans les places occupées (un autre ticket peut partir) ; « Reprendre » sur la carte envoie « Reprends le ticket <clé> là où tu en étais, puis termine par le bilan. » Envoyer soi-même un message à l'agent a le même effet : sa fin de tour est lue comme les autres.

Démarrage de l'app : un ticket « En cours » dont le tour a été coupé par l'arrêt de l'app reçoit « L'app a redémarré pendant ton travail sur <clé> : reprends là où tu en étais, puis termine par le bilan. » Une validation coupée laisse le ticket « À tester », bloqué « Validation interrompue ».

### À tester → validation

« Valider… » (sur un ticket « À tester », même partiel) déroule, en affichant l'étape sur la carte :

1. **arrêt** des lancements de test de l'agent ;
2. **tests** (si activés) : la commande tourne dans le worktree (shell par défaut, 20 min max, `ESCOUADE_PORT_BASE` défini). Échec → le ticket repasse « En cours » sur le même agent, boucle 1, message « Les tests (`<commande>`) échouent : » + les 80 dernières lignes ;
3. **commit** des modifications restantes du worktree (sauf « Laisser en l'état ») : message généré par Haiku (appel ponctuel comme le renommage : titre, description, avancement, `git diff --stat`) au format Conventional Commits suivi de ` [<clé>]`, vérifié par expression régulière, `feat: <titre> [<clé>]` à défaut ; sans « Message de commit généré » : `<clé> <titre>` ;
4. **action** :
   - **merger** — la branche cible est-elle extraite dans le dossier du projet ? Oui : le merge s'y fait, refusé si ce dossier a des modifications suivies non commitées (« Le dossier du projet a des modifications non commitées sur main »). Extraite dans le worktree d'un agent : refus (« main est extraite dans le worktree de X »). Sinon : worktree temporaire `.claude/worktrees/.merge-<clé>` sur la cible, retiré ensuite. Stratégies : *merge commit* `merge --no-ff` (message par défaut de git) ; *squash* `merge --squash` + commit avec le message généré ; *rebase* `rebase <cible>` dans le worktree du ticket puis `merge --ff-only` côté cible ;
   - **PR** — push de `ticket/<clé>` (`-u`), puis `gh pr create --base <cible> --head <branche> --title <message> --body <description + avancement + critères> [--draft]` si `gh` est dans le PATH (issue « ⇡ PR #N → main » + lien) ; sinon, dépôt GitHub : ouverture de `https://github.com/<owner>/<repo>/compare/<cible>...<branche>?expand=1&title=…&body=…` (issue « ⇡ ticket/atl-42 poussée · PR à finaliser ») ; autre hébergeur : push seul et message ;
   - **pousser** — push de `ticket/<clé>` (`-u`) ;
   - **laisser** — rien ;
5. **fin** : merge, PR, push → l'agent est archivé (process arrêté) ; après un merge, si « Supprimer le worktree » : worktree et branche supprimés (`branch -D`, nécessaire après un squash ; s'ils ne peuvent l'être, l'issue finit par « · worktree gardé ») ; PR et push gardent la branche et le worktree. « Laisser » garde l'agent actif. Le ticket passe « Terminé » avec son issue.

Une étape ratée (push refusé, merge impossible…) laisse le ticket « À tester », bloqué avec le message ; « Réessayer » relance la validation.

**Conflit** (merge, squash ou rebase) : merge annulé (`merge --abort` / `reset --merge` / `rebase --abort`), puis selon le réglage :

- *me demander* : bloqué « Conflit avec main », boutons « L'agent résout » et « Annuler » (efface le blocage, le ticket reste « À tester ») ;
- *l'agent résout* : merge / squash → l'app lance `git merge <cible>` dans le worktree et envoie à l'agent « Le merge de main dans ta branche a des conflits sur : <fichiers>. Résous-les, commite le merge, revérifie les critères, puis termine par le bilan. » ; rebase → elle lui demande de rebaser sa branche sur la cible et de résoudre les conflits. Le ticket repasse « En cours » (boucle 1) et reviendra « À tester » à revalider ;
- *annuler* : bloqué avec le message du conflit.

### Renvoyer

« Renvoyer » ouvre un champ « Ce qui ne va pas ». Le ticket repasse « En cours » avec le même agent et le même worktree, boucle 1 ; l'agent reçoit « Retour de test sur <clé> : <commentaire>. Corrige, revérifie tous les critères, puis termine par le bilan. » Les lancements de test continuent (rechargement à chaud).

### Gestion manuelle

- Ticket « À faire » : clic → formulaire de modification ; clic droit : Modifier, Passer en tête, Supprimer.
- Ticket « En cours » / « À tester » / « Terminé » : clic → conversation de son agent ; clic droit : Ouvrir l'agent, Supprimer (« En cours » et « À tester » : confirmation, l'agent est archivé avec son worktree).
- Archiver ou supprimer à la main l'agent d'un ticket « En cours » ou « À tester » → le ticket revient « À faire » (boucle et bilan remis à zéro). Archiver depuis la barre latérale demande d'abord « Archiver <agent> ? » (« Son ticket <clé> repartira « À faire ». »).

## Lancement de test

Pour tout agent à worktree, ticket ou non.

- **Ports** : à la création du worktree d'un ticket, ou au premier « Préparer le lancement » d'un autre agent, l'app réserve un bloc de 10 ports consécutifs à partir de 4100, par pas de 10, en sautant les blocs réservés par d'autres agents (tous projets) et ceux dont un port ne se lie pas sur `127.0.0.1`. Le bloc est enregistré avec l'agent et libéré quand il est archivé ou supprimé. Il est donné à l'agent (prompt du ticket ou message de préparation) et exporté dans les lancements de test (`ESCOUADE_PORT_BASE`, `ESCOUADE_PORT_END`).
- **Recette** : écrite par l'agent dans le bloc `escouade` (clé `lancement`), qu'il soit agent de ticket (dans son bilan, au plus tard quand les critères sont atteints) ou non (en réponse à « Préparer le lancement ») ; la dernière reçue remplace la précédente :

  ```json
  {"preparation": [{"commande": "npm install", "dossier": "web"}],
   "processus": [{"nom": "api", "commande": "npm run dev", "dossier": "api", "env": {"PORT": "4110"}, "url": "http://localhost:4110/health"},
                 {"nom": "web", "commande": "npm run dev -- --port 4111", "dossier": "web", "env": {"VITE_API_URL": "http://localhost:4110"}, "url": "http://localhost:4111"}],
   "ouvrir": "http://localhost:4111/connexion?essais=5"}
  ```

  `dossier` est relatif au worktree (confiné) ; les commandes tournent dans le shell par défaut du système. `url` d'un processus : adresse HTTP qui répond quand il est prêt (sans `url`, il n'est pas attendu). `ouvrir` : l'adresse qui montre **directement la fonctionnalité développée** (la page, l'écran, l'état précis à tester) ; à défaut, la première `url` des processus. Le prompt du ticket et la demande de préparation demandent à l'agent de la donner.
- **« Préparer le lancement »** (clic droit sur un agent à worktree, et dans l'en-tête de l'agent tant qu'il n'a pas de recette) : envoie à l'agent la demande de recette avec ses ports.
- **« ▶ Tester »** (carte d'un ticket qui a une recette, en-tête d'un agent qui a une recette) ouvre la **modale « Tester <clé ou agent> »** et lance tout :
  1. la préparation si elle n'a pas encore réussi pour cette recette (étapes en séquence, chacune doit finir à 0 ; une étape en échec arrête tout : « Préparation en échec (code N) » + « Voir le log ») ;
  2. tous les processus, chacun dans son terminal ConPTY en lecture seule comme les commandes de lancement (log gardé, statut en cours / arrêté / terminé / planté, ⟳, ■) ;
  3. pour chaque processus qui a une `url`, l'app l'interroge en HTTP (depuis le backend, sans proxy, toutes les 500 ms, 2 s par essai) jusqu'à recevoir une réponse, quel que soit son code ; 3 min sans réponse → « Pas de réponse de <url> après 3 min » ;
  4. dès que tous répondent, le **navigateur par défaut s'ouvre sur l'adresse `ouvrir`**.

  La modale montre une ligne par étape et par processus (« démarrage… », « en attente de localhost:4110… », « prêt · 1,8 s », « planté (code 1) » + « Voir le log »), puis « Ouvert dans le navigateur : <adresse> » avec « Rouvrir », « Voir les logs », « Tout arrêter » et « Fermer » (fermer la modale laisse tourner les serveurs). « ▶ Tester » alors que tout tourne déjà rouvre la modale sur l'état courant et le navigateur.
  Les processus apparaissent aussi dans la section « Lancement » de la barre latérale sous un groupe au nom de l'agent ; clic → log. La carte du ticket affiche, tant qu'ils tournent, « ■ Arrêter » et le lien « Ouvrir » vers l'adresse `ouvrir`.
- **Arrêt** : à la validation (étape 1), à l'archivage ou la suppression de l'agent, à la fermeture du projet. Les lancements de test ne survivent pas à un redémarrage.
- **Fichiers copiés dans les worktrees** : réglage du projet, dans la modale des commandes de lancement, liste de motifs relatifs au projet (défaut `.env*` ; `**/.env*` pour les sous-dossiers). À la création de tout worktree, les fichiers ignorés par git qui correspondent sont copiés du dossier du projet ; un fichier seulement non suivi ne l'est jamais (il apparaîtrait comme une modification de l'agent, et tout commit le prendrait).
- Ressources partagées (base de données locale, conteneurs) : non isolées ; l'agent peut seulement les désigner par variables d'environnement.

## Interface

- **Barre latérale** : en haut, sélecteur « Agents | Tableau » (pastille jaune = nombre de tickets « À tester »). « Tableau » affiche le tableau dans la zone principale ; choisir un agent, un terminal ou une commande revient aux agents. Les cartes d'agents de ticket portent « ▸ ATL-42 · boucle 2/5 ».
- **En-tête du tableau** : « Tableau », `<projet> · N tickets · k en boucle` ; à droite « n places libres » (ou « Toutes les places sont prises », ou l'état du quota, ou pourquoi aucun ticket ne démarre : Claude Code introuvable, branche cible sans commit ou introuvable), bouton « ⚙ Après validation : … », interrupteur « Pilote auto » (« Pilote auto · off »).
- **4 colonnes** (À faire · En cours · À tester · Terminé), pastille de couleur, compteur ; « + » sur « À faire » ouvre le formulaire en tête de colonne : titre, description (facultative), critères (un par ligne), boucles max 3 / 5 / 8, Annuler / Ajouter (actif avec un titre ; sans critère : « Implémentation conforme au ticket » et « Tests verts »).
- **Cartes** (comme le design) :
  - commun : clé, titre ;
  - « À faire » : « N critères · max M boucles », état d'attente (« Pris dès qu'une place se libère », « En attente d'une place (2/2) », « Pilote auto désactivé ») et « Lancer » si une place est libre et le pilote désactivé ;
  - « En cours » : « Boucle n/max », critères ✓ / ○, barre de progression (critères atteints / total), les derniers éléments de l'avancement (3 au plus, « +n » s'il y en a d'autres), activité en direct de l'agent avec les points animés (« Lit src/db.ts », « Modifie src/middleware/auth.ts », « Lance npm test », « Réfléchit »…) ou « Question en attente de ta réponse » (bordure jaune), pied avec l'agent ; bloqué : bandeau avec la raison et « Reprendre » ;
  - « À tester » : avancement complet (« Ce qui a été fait »), critères, « Objectif partiel », « n/N critères », liens et bouton « ▶ Tester » / « ■ Arrêter » si recette, boutons de validation et « Renvoyer » ; étape de validation en cours ; bandeau de blocage ou de conflit avec ses boutons ;
  - « Terminé » : issue (lien s'il y en a un), « n boucles · coût » (toutes les boucles de son agent, renvois, tests en échec et conflits compris), carte atténuée ; les plus récents en haut.
- **Activité de l'agent** : nouveau champ de la vue d'un agent, tenu par le cœur à partir des outils lancés (Read → « Lit », Grep / Glob → « Cherche », Edit / MultiEdit → « Modifie », Write → « Écrit », Bash → « Lance » + commande tronquée, Task → « Délègue », réflexion → « Réfléchit », texte → « Rédige »), vidé en fin de tour.
- **Conversation** : un bloc `escouade` s'affiche comme une carte « Bilan des critères » (✓ / ○, texte du critère, note ; puis l'avancement en liste) suivie, s'il y a une recette, de « Lancement de test » (processus, URL, bouton « ▶ Tester »).
- **Notifications** : passage « À tester » → « ATL-42 prêt à tester » (carillon ; toast système en arrière-plan, clic → tableau du projet) ; ticket bloqué → « ATL-42 bloqué : <raison> ». Les notifications de fin de tour d'un agent de ticket « En cours » sont supprimées ; ses questions notifient comme d'habitude.

## Backend (Rust)

- `model.rs` : `Ticket`, `Criterion`, `BoardSettings` (dans `Project.board`), `TestRecipe` ; `AgentMeta` gagne `ticket_id`, `append_prompt`, `port_base`, `recipe` ; `AgentView` gagne `activity` ; `Project` gagne `worktree_copy` ; `PersistedState` gagne `tickets` ; `UiEvent` gagne `ticket`, `ticketRemoved` ; `InitialState` gagne `tickets`. Tous les nouveaux champs en `#[serde(default)]`.
- `board.rs` (logique pure, testée unitairement) : lecture du bloc `escouade`, décision de fin de tour, choix des tickets à démarrer, textes des messages, clé et slug, message de commit de secours, libellés d'issue, URL de comparaison GitHub, allocation des ports.
- `tickets.rs` (orchestration sur `Core`) : planificateur, départ, fin de tour (appelé depuis `Core::apply`), reprise au démarrage, validation, conflits, renvoi.
- `Core::create_agent` gagne des options (nom, modèle, effort, mode, worktree imposé avec branche et base, prompt ajouté, ticket) ; `claude_args` ajoute `--append-system-prompt` ; `AgentRt::on_result` remonte le texte final du tour.
- `git.rs` : `branches`, `worktree_add` avec branche et base données, emplacement d'extraction d'une branche (`worktree list --porcelain`), `commit_all`, `rebase` / `merge --ff-only`, `merge` dans un dossier donné, push d'une branche depuis un worktree, URL du dépôt distant.
- Commandes : `ticket_create`, `ticket_update`, `ticket_delete`, `ticket_prioritize`, `ticket_start`, `ticket_resume`, `ticket_approve`, `ticket_reject`, `ticket_resolve_conflict`, `ticket_dismiss`, `board_set`, `git_branches`, `agent_prepare_launch`, `test_run_start` (comme `run_start`, pour une étape de la recette d'un agent), `http_ready(url) -> bool` (un essai HTTP sans proxy, 2 s max, vrai pour toute réponse).

## Frontend

- `types.ts` / `ipc.ts` / `state.svelte.ts` : tickets par projet, événements, `app.board[projectId]` (vue tableau, non persistée) ; dans `App.svelte` le tableau passe avant l'éditeur et les autres vues ; `onScreen()` en tient compte.
- Composants `board/Board.svelte`, `BoardColumn.svelte`, `TicketCard.svelte`, `TicketForm.svelte`, `RejectForm.svelte`, `modals/BoardSettingsModal.svelte`, `modals/TestLaunchModal.svelte`, `conv/CriteriaReport.svelte` ; sélecteur dans `Sidebar.svelte` ; groupe de lancements de test dans `RunsSection.svelte` ; champ « Fichiers copiés dans les worktrees » dans `RunConfigModal.svelte`.

## Hors périmètre

Dépendances entre tickets ; glisser-déposer des cartes ; plusieurs agents sur un même ticket ; vérificateur indépendant des critères ; tickets partagés via le dépôt ; isolation des bases de données et conteneurs ; création de PR sur d'autres hébergeurs que GitHub.

## Tests

- Rust, `board.rs` : bloc `escouade` (dernier bloc, JSON invalide, critères manquants, `criteria`, recette, `dossier` qui sort du worktree), décisions de fin de tour (tous atteints, partiel, limite, rappel puis blocage, interrompu, erreur, limite d'usage), choix des tickets (rang, maximum, bloqués non comptés, pilote off + lancement manuel, quota), clé et slug, message de secours et validation du format, URL GitHub (https, ssh), allocation des ports (blocs pris, port occupé).
- Rust, `core_tests.rs` (faux `claude` étendu : scénarios qui écrivent un fichier dans le worktree et terminent par un bloc `escouade` dont les critères atteints sont dictés par le message ; trace de `--append-system-prompt`) : ticket créé → agent et worktree `ticket/…` depuis la cible → boucles jusqu'à « À tester » ; limite de boucles → partiel ; renvoi ; reprise au démarrage ; validation merge / squash / rebase (cible extraite dans le projet, ou non → worktree temporaire), push et PR (dépôt distant nu local, sans `gh` → issue « à finaliser »), tests en échec → retour à l'agent, conflit (me demander, l'agent résout, annuler), nettoyage du worktree ; agent archivé à la main → ticket « À faire » ; copie des `.env*` ; `test_run_start` (cwd, variables de ports) ; `http_ready` (serveur local qui répond 404 → vrai, port fermé → faux, proxy configuré ignoré).
- Vitest : tableau (colonnes, compteurs, places libres, pilote auto), formulaire, carte par colonne et par état (bloqué, question, conflit, validation en cours), Renvoyer, réglages (options, libellés, aperçu du commit), sélecteur de la barre latérale et pastille, étiquette de ticket sur la carte d'agent, carte « Bilan des critères », groupe de lancements de test, modale « Tester » (préparation puis processus, attente HTTP, ouverture du navigateur sur `ouvrir` une fois tout prêt et pas avant, processus planté, délai dépassé, réouverture quand tout tourne déjà).
- e2e (app réelle, faux `claude`) : créer un ticket → il passe « En cours » puis « À tester » → « Valider et merger » → le fichier de l'agent est sur la branche du projet et le ticket « Terminé ».
