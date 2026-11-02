---
title: "Les plugins livrés avec Escouade"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.3, §4.4","docs/TODO-2.2.md (INT-4, INT-8.1)","src-tauri/src/plugins/builtin.rs","docs/PLUGINS.md §4.13","plugins/builtin/dev.escouade.github/README.md","plugins/builtin/dev.escouade.gitlab/README.md","plugins/builtin/dev.escouade.gitea/README.md","plugins/builtin/dev.escouade.azure-devops/README.md"]
tickets: ["ESC-210","ESC-221","ESC-285","ESC-217","ESC-56","ESC-58","ESC-280","ESC-281"]
decisions: []
compared: null
---

# Les plugins livrés avec Escouade

Certains plugins sont donnés par l'app : livrés avec elle, signés avec elle, mis à jour avec elle. Ils ont l'origine `builtin` et la vue Plugins les range dans « Livrés avec Escouade ». Ce n'est pas le cas des plugins de première main du store (`plugins/<id>/` du dépôt, origine `store`, rangés dans « Ajoutés »).

## Le registre

- La liste est celle des dossiers `plugins/builtin/<id>/` du dépôt, chacun avec son `escouade-plugin.json` et son `README.md`, embarqués dans l'app (`src-tauri/src/plugins/builtin.rs`). Un test vérifie que chaque dossier a son manifeste et son README et qu'il est dans le registre.
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

- OpenAI, Groq, Mistral, ElevenLabs et Deepgram sont cinq plugins livrés, `dev.escouade.openai` (etc.), au registre de tous les builds. Chacun est l'entrée `escouade-plugins voice-<service>`. Réglages › Voix se comporte comme avant : la clé reste au trousseau (`dictation-<service>`), la dictée, le direct et la voix de l'agent passent par le plugin, sans différence visible.
- Les couper dans la vue Plugins retire le service de Réglages › Voix (la dictée repasse en local). Un plugin qui ne démarre pas laisse le service du cœur répondre, et son journal dit pourquoi.
- Un plugin d'un autre auteur peut servir un nouveau service de voix ou prendre la place de l'un des cinq (même identifiant de service).
- Délais : le plugin ajoute moins de 1 ms au texte final du direct (médiane et 99e centile), son poussé au rythme du micro ; détail et protocole `voice/*` dans `docs/PLUGINS.md` §4.13.
- Le compte Mistral : un plugin « Mistral » porte la voix et, plus tard, Mistral Vibe, avec une seule clé.

- Mistral Vibe est pour l'instant un plugin livré à part, `dev.escouade.mistral-vibe` (INT-10.6, voir *Mistral Vibe en plugin*), ses comptes par clé inchangés ; il rejoint « Mistral » et sa clé avant la release 2.2, dans un ticket à part.
