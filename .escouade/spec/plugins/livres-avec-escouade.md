---
title: "Les plugins livrés avec Escouade"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.3, §4.4","docs/TODO-2.2.md (INT-4, INT-8.1)","src-tauri/src/plugins/builtin.rs","docs/PLUGINS.md §4.13","plugins/github/README.fr.md","plugins/gitlab/README.fr.md","plugins/gitea/README.fr.md","plugins/azure-devops/README.fr.md","docs/PLUGINS.md §8.7, §8.13, §8.14","src-tauri/src/plugins/seed.rs","src-tauri/src/plugins/sort.rs","src-tauri/src/plugins_builtin_tests.rs","src-tauri/src/plugins_sort_tests.rs","docs/PLUGINS.md §8.32","plugins/seed.json","tests/e2e/real-seed.spec.ts","https://store.escouade.dev/api/v1/catalog.json"]
tickets: ["ESC-210","ESC-221","ESC-285","ESC-217","ESC-56","ESC-58","ESC-280","ESC-281","ESC-327","ESC-282","ESC-225","ESC-223","ESC-224","ESC-283","ESC-226","ESC-338","ESC-339","ESC-344","ESC-351","ESC-349","ESC-348","ESC-345","ESC-350","ESC-367","ESC-370","ESC-373","ESC-354"]
decisions: [80,92,110,113,117]
compared: null
---

# Les plugins livrés avec Escouade

Certains plugins sont donnés par l'app : livrés avec elle, signés avec elle, mis à jour avec elle. Ils ont l'origine `builtin` et la vue Plugins les range dans « Livrés avec Escouade ». Ce n'est pas le cas des plugins de première main du store (`plugins/<id>/` du dépôt, origine `store`, rangés dans « Ajoutés »).

## Le registre

- La liste est celle des dossiers `plugins/<id-court>/` du dépôt dont le programme est `escouade-plugins`, plus la démonstration (`tests/fixtures/plugins/minimal/`), chacun avec son `escouade-plugin.json` et son `README.md`, embarqués dans l'app (`src-tauri/src/plugins/builtin.rs`). Le dossier porte un nom court (`plugins/claude/`) ; l'identifiant reste celui du manifeste (`dev.escouade.claude-code`). Ces plugins sont sous GPL-3.0-or-later, avec leur `LICENSE` et leur `CHANGELOG.md` dans le dossier. Un test vérifie que chaque dossier a tout cela (la démonstration, son manifeste et son README seulement) et qu'il est dans le registre.
- Leur version est celle d'Escouade (le `version` du manifeste est remplacé à la lecture), leur empreinte `builtin:<version>`. Ils n'ont ni mise à jour à part, ni liste de révocation : un correctif vient avec une release.
- Le tri par l'origine, jamais par l'identifiant. Seule l'origine `builtin` peut lancer les programmes `{escouade}/…`.
- `dev.escouade.minimal` est une démonstration, dans le registre des builds de développement et des tests seulement : la poignée de main du protocole, deux méthodes d'essai, et le harness « Minimal », qui prouve le contrat des harness en process (voir *Le contrat des harness en process*). Sa fiche dit qu'il ajoute un harness ; ce harness n'est pas proposé à la création d'un agent.
- `dev.escouade.github` (« GitHub », INT-8.1) est dans les releases : l'hébergeur GitHub (github.com, GHE.com, GitHub Enterprise Server : PR, merge et auto-merge, CI et journaux, relance, relectures et fils, déploiements) et la source GitHub Issues, sur le même compte. Son dossier porte aussi sa crate (son code et ses tests unitaires) ; sa page du store pointe vers ce dossier.
- `dev.escouade.gitlab` (« GitLab », ESC-56) est dans les releases : l'hébergeur GitLab (gitlab.com et les instances auto-hébergées : MR, merge et merge quand le pipeline passe, jobs et journaux, relance du pipeline, approbations et discussions, déploiements ; voir *Hébergeurs de code*). Son dossier porte sa crate ; sa page du store pointe vers ce dossier. Il porte aussi GitLab Issues (INT-8.2, ESC-218), la source du Kanban, sur les mêmes comptes : le compte d'une instance sert les deux, et le plugin coupé ou arrêté, les deux le disent.
- `dev.escouade.bitbucket` (« Bitbucket », INT-8.3, ESC-57) est dans les releases : l'hébergeur Bitbucket Cloud (PR et brouillons, merge sans auto-merge, statuts, pipelines et journaux d'étapes, commentaires et fils, déploiements), sur un compte e-mail et jeton d'API Atlassian. Même forme que GitHub : un dossier avec sa crate et ses tests.
- `dev.escouade.gitea` (« Gitea / Forgejo », INT-8.4, ESC-58) est dans les releases : l'hébergeur de Gitea, de Forgejo et de Codeberg (PR, merge, auto-merge sur branche protégée, statuts et Actions, relectures et fils). Il n'a pas de source de tickets ni de déploiements (aucune API chez eux), et ce que la version de l'instance ne permet pas (journaux, relance des Actions, réponses dans les fils) est dit appel par appel (voir *Hébergeurs de code*). Même forme que GitHub : sa crate dans son dossier, le kit en dessous.
- `dev.escouade.azure-devops` (« Azure DevOps », INT-8.5, ESC-280) est dans les releases : l'hébergeur Azure DevOps (dev.azure.com, `*.visualstudio.com`, Azure DevOps Server : PR, merge et auto-complete, stratégies et Azure Pipelines, journaux, relance, relectures et fils) et la source Azure Boards, sur le compte de l'organisation. Le jeton de `az` est lu par le cœur et prêté comme un jeton ordinaire ; Azure Boards envoie chaque appel au compte de l'organisation que nomme le projet (`<org>/<projet>`). Même forme que GitHub : sa crate dans son dossier, le kit en dessous.

## Ce que l'utilisateur peut

- Le couper pour l'app ou pour un projet (comme tout plugin) ; coupé pour l'app, rien de lui ne démarre. Ses choix sont gardés.
- Pas le désinstaller : le bouton n'y est pas.
- Un plugin du store, d'un dossier ou d'un dépôt qui porte le même identifiant est refusé (l'aperçu dit pourquoi). Une copie du même identifiant déjà installée avant que l'app le donne cède la place au démarrage, ses choix gardés.

## Un hébergeur ou une source servi par un plugin livré

- Rien ne change à l'usage : les comptes, les formulaires, les PR sur la carte, l'import et la synchro des issues sont les mêmes. Les comptes déjà connectés restent tels quels.
- Le jeton d'un compte reste au coffre ; le cœur le prête au plugin qui le demande (`secrets/get` de `account:<hôte>`), celui de la CLI (`gh auth token`, `glab auth status --show-token`, `az account get-access-token`) compris, que le cœur demande lui-même. Le plugin n'a que les siens et ne lance aucune commande.
- Le plugin reçoit à son lancement la langue de l'interface et les réglages réseau (proxy, hôtes exclus, certificats non vérifiés) ; un changement le relance.
- **Coupé ou arrêté**, il le dit : la carte de l'agent porte « Hébergeur arrêté : PR plus suivies » (la raison en infobulle) et garde les PR lues en dernier ; Réglages › Intégrations › Hébergeurs de code dit sous chaque compte que le plugin est coupé, ou qu'il s'est arrêté et pourquoi. La première lecture qui marche efface le message.

## Le programme

- `escouade-plugins <nom>` : un seul exécutable à plusieurs entrées (`src-tauri/src/bin/escouade-plugins.rs`), livré à côté de l'app comme `escouade-hook`, un process par plugin. Il parle le protocole de la crate `escouade-plugin-protocol` (`crates/`), partagée avec le cœur ; les plugins d'hébergeurs et de sources s'appuient sur le kit `escouade-plugin-kit` (la boucle asynchrone, le service de `forge/*`, `tickets/*` et `account/me`, les erreurs typées, la limite d'appels dans chaque réponse). L'entrée `minimal-harness` n'est pas un plugin : c'est le harness du plugin de démonstration, que le cœur lance.
- Avant de lancer un programme `{escouade}/…`, le cœur vérifie que c'est bien celui du dossier de l'app et, en release, sa signature là où le système la vérifie (macOS : `codesign` ; Windows : Authenticode valide dès que celle de l'app l'est ; Linux : rien de plus que le paquet).
- Il est signé avec l'app : Authenticode et notarisation dans `release.yml`, fusion universelle sous macOS, présence dans les paquets Linux. La CI vérifie sa présence sur les trois systèmes et la signature de code sous macOS.

## Les services de voix en ligne (INT-13)

- OpenAI, Groq, Mistral, ElevenLabs et Deepgram sont cinq plugins livrés, `dev.escouade.openai` (etc.), au registre de tous les builds. Chacun est un module, depuis ESC-350. Réglages › Voix se comporte comme avant : la clé reste au trousseau (`dictation-<service>`), la dictée, le direct et la voix de l'agent passent par le plugin, sans différence visible.
- Les couper dans la vue Plugins retire le service de Réglages › Voix (la dictée repasse en local). Un plugin qui ne démarre pas le dit à son premier appel, et son journal dit pourquoi : le cœur n'a plus de service qui répondrait à sa place.
- Un plugin d'un autre auteur peut servir un nouveau service de voix ou prendre la place de l'un des cinq (même identifiant de service).
- Délais : le plugin ajoute moins de 1 ms au texte final du direct (médiane et 99e centile), son poussé au rythme du micro, sauf Deepgram, à 1 ms environ ; détail et protocole `voice/*` dans `docs/PLUGINS.md` §4.13 et §8.28.
- Le compte Mistral : le plugin « Mistral » (`dev.escouade.mistral`) porte la voix et Mistral Vibe (INT-10.6 bis), une seule instance et un seul compte, partagé ; voir *Mistral Vibe en plugin*. Couper ce plugin retire les deux.

- Gemini CLI est un plugin livré, `dev.escouade.gemini-cli` (INT-10.7, voir *Gemini CLI en plugin*), ses comptes inchangés.

## Les harness livrés

Cursor CLI, Mistral Vibe (dans le plugin Mistral), Gemini CLI, OpenCode, Copilot CLI et Codex (`dev.escouade.codex`, INT-10.4) sont servis par un plugin livré ; voir leurs pages *… en plugin*. `dev.escouade.opencode` est dans les releases : son dossier porte sa crate, sa page du store pointe vers ce dossier.

Antigravity CLI aussi (`dev.escouade.antigravity-cli`, INT-10.8), ses comptes inchangés ; voir *Antigravity CLI en plugin*.
Claude Code (`dev.escouade.claude-code`, INT-10.5) est servi lui aussi par un plugin livré ; voir *Claude Code en plugin*.

## L'amorce : la sortie de l'origine `builtin` (ESC-338)

L'installeur peut porter les versions publiées de la première main : pour chacune, les octets et la signature du store, plus la liste de révocation du jour. La liste est `plugins/seed.json`, embarquée : `id`, `version`, empreinte de `version.json`. Elle est vide tant que la première main n'est pas publiée (Z2) : d'ici là, tout ce qui précède reste vrai. `builtin.rs` dit quels identifiants elle peut tenir.

Un identifiant que `seed.json` nomme n'est plus donné par l'app :

- **Au premier lancement**, et après une mise à jour de l'app, Escouade installe ce que l'amorce tient, qui n'est pas installé **et que la configuration de l'utilisateur utilise** (le tri, ci-dessous), sans réseau, par le chemin d'une installation du store : mêmes contrôles (signature, empreintes, `dev.escouade.*` signé, révocation, `escouade.min` et `max`). L'origine est `store`, marquée `seed`, et sa page le dit. Le module se précompile à sa première activation.
- **Une version refusée** n'installe rien, et la vue Plugins dit pourquoi sous « Proposés par Escouade ». Les autres passent.
- **Une version retirée par la liste** (`disable`) s'installe coupée, sa raison dite. Un sceau retiré (`unverify`) n'installe rien. La liste du store l'emporte dès qu'elle est plus récente que celle de l'amorce.
- **Rien ne se met à jour seul.** Une version plus récente, du store ou de l'amorce d'une nouvelle app, est proposée, et s'installe au clic avec la fenêtre habituelle. La plus récente des deux est proposée, jamais une plus ancienne que l'installée.
- **Ces plugins se coupent et se désinstallent.** Le lancement ne remet pas ce que l'utilisateur a désinstallé : l'amorce le note, « Proposés par Escouade » le propose, depuis l'amorce sans réseau, ou depuis le store s'il a plus récent, et le geste qui le demande le remet. Ses choix gardés reviennent avec lui.
- **Sans son plugin**, un agent dit que le plugin n'est plus installé et que la vue Plugins propose de l'installer ; une source ou un hébergeur le dit dans son message habituel.
- **Sans réseau ni store**, tout marche à la version installée ou à celle de l'amorce.

Si l'amorce refuse un plugin de l'origine `builtin` que le tri désigne, il continue de tourner comme avant, jusqu'à Z3.

La release télécharge l'amorce (`store/scripts/seed.ts`), vérifie chaque signature, et échoue sur une version absente, non signée, révoquée ou qui ne tourne pas sur cette version d'Escouade : **le chef publie d'abord, la release ensuite**. Un build de développement accepte les modules non signés de `scripts/seed-dev.sh` ; une signature fausse reste refusée partout.

## Le tri de l'amorce (ESC-373)

Décidé par l'utilisateur le 2026-11-07 (« je veux un tri ») : l'amorce n'installe que les plugins de ce que l'utilisateur a configuré. Les autres restent dans l'amorce, proposés, installables sans réseau au moment où ils servent. La règle se lit dans les réglages, les projets, les agents et les comptes, jamais sur le réseau :

- **Un harness** (Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI, Antigravity CLI, OpenCode, Mistral Vibe, Goose ; `dev.escouade.acp` pour un harness décrit par ACP) : un compte de ce harness dans Réglages › Comptes, un agent sur lui (archivé compris), le harness par défaut d'un projet, ou celui des textes rédigés. `dev.escouade.acp` aussi quand un plugin installé décrit un harness par ACP.
- **Une source de tickets ou un hébergeur** (Trello, ClickUp, Linear, Jira, GitHub, GitLab, Bitbucket, Gitea, Azure DevOps) : un projet qui l'a pour source, un projet dont un remote est chez lui (par le nom de l'hôte, ou par le compte connecté pour cette instance), ou un compte de ce service déjà enregistré, sauf s'il est servi par le plugin d'un autre auteur.
- **Un service de voix** (Deepgram, ElevenLabs, OpenAI, Groq, Mistral) : choisi « En ligne » pour la dictée ou pour la voix, ou dont la clé est gardée (sa présence, jamais sa valeur).
- **isola** : un projet, ou l'un de ses dépôts, avec un `.isola.toml`.
- **Discord, Slack, Contraste élevé, Règles de l'équipe** : ce qui est installé le reste ; sinon, proposés.
- **Ce qui ne se lit pas sûrement s'installe** : des réglages, un état ou des comptes qui ne se lisent pas installent toute l'amorce ; un projet hors d'atteinte (disque non monté) ou dont la configuration git ne se lit pas veut tous les hébergeurs et isola ; un identifiant de l'amorce sans règle s'installe. Le tri ne fait jamais perdre une fonction de la 2.1.

Ce que cela change :

- **Une mise à jour depuis la 2.1** installe au premier lancement ce que la règle désigne, et rien d'autre : tout ce qui marchait marche, sans un clic. Une entrée `builtin` que la règle désigne prend la version de l'amorce, ses choix gardés ; une autre est retirée proprement, ses choix gardés comme « Garder ses réglages » les garde, ses secrets au trousseau, et proposée. Elle n'est pas notée désinstallée.
- **Une première installation** n'installe rien d'office. L'accueil propose les harness trouvés ; « Ajouter un compte… » sur l'un d'eux installe son plugin depuis l'amorce, sans réseau, puis ajoute le compte.
- **Le geste qui demande un plugin absent** l'installe depuis l'amorce, sans réseau, et le dit dans une ligne (« Plugin Codex installé ») : ajouter un compte d'un harness, choisir une source de tickets ou un harness par défaut pour un projet, ajouter un projet (l'hébergeur de ses remotes, isola), connecter un compte d'une source ou d'un hébergeur, choisir un service de voix « En ligne » ou lui donner sa clé, préparer ou ouvrir le test d'un worktree avec un `.isola.toml`, démarrer un ticket sur un tel projet. Le geste vaut accord : pas de fenêtre de plus. Un plugin désinstallé n'est remis que sur un tel geste.
- **La vue Plugins** montre les plugins de l'amorce non installés sous « Proposés par Escouade » (« Dans l'app, sans téléchargement »), installables d'un clic, sans fenêtre, comme le geste ; le store prend la place de l'amorce quand il a plus récent, avec la fenêtre habituelle.
- **Les listes de choix** (harness d'un compte, source de tickets, hébergeur, service de voix) gardent tous les choix de l'amorce, installés ou non : un choix non installé s'installe quand on le prend.

## Les plugins du registre qui sont des modules (ESC-344)

Linear, Trello, Jira et ClickUp restent au registre, mais ne sont plus dans `escouade-plugins` : leur programme est un module (`run.wasm`). La liste du registre est donc celle des dossiers dont le programme est `escouade-plugins` ou un module.

- Leur version est toujours celle d'Escouade tant qu'ils ont l'origine `builtin`, et ils restent rangés dans « Livrés avec Escouade ».
- Leur module n'est pas dans l'installeur : l'amorce le donnera (Z2). D'ici là, seuls un build de développement et les tests les lancent, depuis `<target>/plugins-wasm/<id-court>/plugin.wasm`, écrit par `scripts/plugins-wasm.sh`. Sans ce fichier, le plugin s'arrête et son journal dit pourquoi.

- Un plugin du registre tourne toujours sur l'app qui le donne : tant qu'il est `builtin`, son `escouade.min` lu est la version de l'app, comme sa `version`.
- Pendant qu'un compte se vérifie, avant d'être enregistré, le cœur prête l'hôte qu'il nomme au plugin, comme il lui prête son jeton : `{account}` l'atteint dès le premier appel.

GitHub, GitLab, Bitbucket, Gitea et Azure DevOps rejoignent ces modules (ESC-345), aux mêmes conditions : au registre, plus dans `escouade-plugins`, et leur module vient de l'amorce (Z2). Une release faite avant Z2 n'aurait plus ces hébergeurs.

## isola, un module du registre (ESC-351)

isola rejoint Linear, Trello, Jira et ClickUp : il reste au registre, n'est plus dans `escouade-plugins`, et son programme est un module (`run.wasm`). Tout ce que dit la section précédente vaut pour lui. Comme tout plugin, il n'atteint un projet privé qu'une fois activé.

OpenCode est aussi un module du registre (ESC-349), avec les mêmes règles que ces quatre plugins : il n'est plus dans `escouade-plugins`, et une release faite avant Z2 ne l'aurait pas.

Antigravity CLI suit le même chemin (ESC-348) : au registre, en module, plus dans `escouade-plugins`, et son module n'est que dans un build de développement et les tests jusqu'à Z2. Sa vue terminal reste au cœur.

## La voix et Mistral sont des modules (ESC-350)

OpenAI, Groq, Mistral, ElevenLabs et Deepgram sont sortis d'`escouade-plugins` : ce sont des modules (`run.wasm`) en `2.2.0`, sur la crate `escouade-voice`, à publier sur crates.io avant leur `Cargo.lock`. Il ne reste à `escouade-plugins` que la démonstration. Un plugin de voix ne démarre qu'à son premier appel ; coupé ou désinstallé, il retire son service ; qui ne démarre pas le dit, sans repli dans le cœur. Une release faite avant Z2 n'aurait plus de voix en ligne.

## La documentation d'un plugin de première main (ESC-370)

Le README d'un dossier de `plugins/` peut être un par langue : `README.fr.md` et `README.en.md` à la place de `README.md`, comme le store les prend. Le test du registre accepte l'un ou l'autre (les deux langues, non vides, sinon `README.md`). La documentation du plugin est dans `docs/fr/` et `docs/en/`, les mêmes pages dans les deux langues, nommées pour leur ordre (`01-installer.md` / `01-install.md`…), et le manifeste la déclare : `"store": { "docs": "docs" }`. isola, Discord, Slack, Contraste élevé et Règles de l'équipe ont cette forme, comme Trello, ClickUp, Linear, Jira, GitHub, GitLab, Bitbucket, Gitea / Forgejo et Azure DevOps (ESC-367).

## L'amorce remplie (ESC-354, Z2)

`plugins/seed.json` n'est plus vide : il nomme la dernière version signée au store de **26 plugins de première main** (lue au catalogue du 2026-11-07). Ce que les sections plus haut disent « d'ici Z2 » ne vaut plus pour eux : ils ne sont plus donnés par l'app, mais installés depuis l'amorce, origine `store`, marque `seed`, et ils se désinstallent.

- **Dans l'amorce** : Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI, OpenCode, `dev.escouade.acp`, Trello, Linear, Jira, ClickUp, GitHub, GitLab, Bitbucket, Gitea / Forgejo, Azure DevOps, OpenAI, Groq, Mistral, ElevenLabs, Deepgram, et, hors du registre, **Goose** et ceux **sur demande** (Discord, Slack, Contraste élevé, Règles de l'équipe). `builtin::seeds` accepte donc aussi Goose et ceux sur demande, que le tri connaissait déjà ; la vue les nomme sans leur manifeste.
- **Pas encore dans l'amorce** : Antigravity CLI (aucune version signée au store) et isola (pas publié). Ils restent donnés par l'app, origine `builtin`, comme avant ; chacun entre dans la liste d'une ligne une fois signé.
- **Un build de développement** dont le dossier de l'amorce n'a pas les fichiers d'un plugin que la liste nomme (ni `scripts/seed-dev.sh`, ni `seed.ts fetch`) lance ce plugin du registre, comme avant l'amorce. Une release garde l'entrée, et la vue dit ce qui manque.
- **La release** attend la première liste de révocation du store : sans elle, `seed.ts fetch` s'arrête (« no revocation list could be read »). C'est un geste de l'utilisateur (sa clé). Le reste de l'étape tient contre le vrai store : les 26 signatures, les commentaires, les empreintes et les modules.
- **Essayé hors ligne** sur une build release : un dossier vierge n'installe rien, et « Ajouter un compte… » sur Claude Code installe son plugin depuis l'amorce en 250 à 270 ms ; une configuration de 2.1 (Claude Code, Codex, Trello) installe ces trois-là au lancement, 130 à 300 ms chacun, et rien d'autre ; le module de Claude Code se précompile à sa première activation en 100 à 120 ms ; un plugin désinstallé n'est pas remis au lancement suivant.
- **Le poids** : l'amorce fait 17,4 Mo en clair et ajoute 2,5 Mo à l'installeur, contre les 34,5 Mo qu'aurait pesés `escouade-plugins`.
- Le journal de l'app dit le temps de chaque installation depuis l'amorce et de chaque mise en route d'un module.
