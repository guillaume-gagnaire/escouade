---
title: "Les plugins livrés avec Escouade"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.3, §4.4","docs/TODO-2.2.md (INT-4, INT-8.1)","src-tauri/src/plugins/builtin.rs","plugins/builtin/dev.escouade.github/README.md"]
tickets: ["ESC-210","ESC-221","ESC-217"]
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

## Ce que l'utilisateur peut

- Le couper pour l'app ou pour un projet (comme tout plugin) ; coupé pour l'app, rien de lui ne démarre. Ses choix sont gardés.
- Pas le désinstaller : le bouton n'y est pas.
- Un plugin du store, d'un dossier ou d'un dépôt qui porte le même identifiant est refusé (l'aperçu dit pourquoi). Une copie du même identifiant déjà installée avant que l'app le donne cède la place au démarrage, ses choix gardés.

## Un hébergeur ou une source servi par un plugin livré

- Rien ne change à l'usage : les comptes, les formulaires, les PR sur la carte, l'import et la synchro des issues sont les mêmes. Les comptes déjà connectés restent tels quels.
- Le jeton d'un compte reste au coffre ; le cœur le prête au plugin qui le demande (`secrets/get` de `account:<hôte>`), celui de la CLI (`gh auth token`) compris, que le cœur demande lui-même. Le plugin n'a que les siens et ne lance aucune commande.
- Le plugin reçoit à son lancement la langue de l'interface et les réglages réseau (proxy, hôtes exclus, certificats non vérifiés) ; un changement le relance.
- **Coupé ou arrêté**, il le dit : la carte de l'agent porte « Hébergeur arrêté : PR plus suivies » (la raison en infobulle) et garde les PR lues en dernier ; Réglages › Intégrations › Hébergeurs de code dit sous chaque compte que le plugin est coupé, ou qu'il s'est arrêté et pourquoi. La première lecture qui marche efface le message.

## Le programme

- `escouade-plugins <nom>` : un seul exécutable à plusieurs entrées (`src-tauri/src/bin/escouade-plugins.rs`), livré à côté de l'app comme `escouade-hook`, un process par plugin. Il parle le protocole de la crate `escouade-plugin-protocol` (`crates/`), partagée avec le cœur ; les plugins d'hébergeurs et de sources s'appuient sur le kit `escouade-plugin-kit` (la boucle asynchrone, le service de `forge/*`, `tickets/*` et `account/me`, les erreurs typées, la limite d'appels dans chaque réponse). L'entrée `minimal-harness` n'est pas un plugin : c'est le harness du plugin de démonstration, que le cœur lance.
- Avant de lancer un programme `{escouade}/…`, le cœur vérifie que c'est bien celui du dossier de l'app et, en release, sa signature là où le système la vérifie (macOS : `codesign` ; Windows : Authenticode valide dès que celle de l'app l'est ; Linux : rien de plus que le paquet).
- Il est signé avec l'app : Authenticode et notarisation dans `release.yml`, fusion universelle sous macOS, présence dans les paquets Linux. La CI vérifie sa présence sur les trois systèmes et la signature de code sous macOS.
