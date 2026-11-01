---
title: "Les plugins livrés avec Escouade"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.3","docs/TODO-2.2.md (INT-4)","src-tauri/src/plugins/builtin.rs","docs/PLUGINS.md §4.13"]
tickets: ["ESC-210","ESC-221","ESC-285"]
decisions: []
compared: null
---

# Les plugins livrés avec Escouade

Certains plugins sont donnés par l'app : livrés avec elle, signés avec elle, mis à jour avec elle. Ils ont l'origine `builtin` et la vue Plugins les range dans « Livrés avec Escouade ». Ce n'est pas le cas des plugins de première main du store (`plugins/<id>/` du dépôt, origine `store`, rangés dans « Ajoutés »).

## Le registre

- La liste est celle des dossiers `plugins/builtin/<id>/` du dépôt, chacun avec son `escouade-plugin.json` et son `README.md`, embarqués dans l'app (`src-tauri/src/plugins/builtin.rs`). Un test vérifie que chaque dossier a son manifeste et son README et qu'il est dans le registre.
- Leur version est celle d'Escouade (le `version` du manifeste est remplacé à la lecture), leur empreinte `builtin:<version>`. Ils n'ont ni mise à jour à part, ni liste de révocation : un correctif vient avec une release.
- Le tri par l'origine, jamais par l'identifiant. Seule l'origine `builtin` peut lancer les programmes `{escouade}/…`.
- Le premier est `dev.escouade.minimal`, une démonstration, dans le registre des builds de développement et des tests seulement (celui d'une release est vide jusqu'aux vrais) : la poignée de main du protocole, deux méthodes d'essai, et le harness « Minimal », qui prouve le contrat des harness en process (voir *Le contrat des harness en process*). Sa fiche dit qu'il ajoute un harness ; ce harness n'est pas proposé à la création d'un agent.

## Ce que l'utilisateur peut

- Le couper pour l'app ou pour un projet (comme tout plugin) ; coupé pour l'app, rien de lui ne démarre. Ses choix sont gardés.
- Pas le désinstaller : le bouton n'y est pas.
- Un plugin du store, d'un dossier ou d'un dépôt qui porte le même identifiant est refusé (l'aperçu dit pourquoi). Une copie du même identifiant déjà installée avant que l'app le donne cède la place au démarrage, ses choix gardés.

## Le programme

- `escouade-plugins <nom>` : un seul exécutable à plusieurs entrées (`src-tauri/src/bin/escouade-plugins.rs`), livré à côté de l'app comme `escouade-hook`, un process par plugin. Il ne parle que le protocole de la crate `escouade-plugin-protocol` (`crates/`), partagée avec le cœur. L'entrée `minimal-harness` n'est pas un plugin : c'est le harness du plugin de démonstration, que le cœur lance.
- Avant de lancer un programme `{escouade}/…`, le cœur vérifie que c'est bien celui du dossier de l'app et, en release, sa signature là où le système la vérifie (macOS : `codesign` ; Windows : Authenticode valide dès que celle de l'app l'est ; Linux : rien de plus que le paquet).
- Il est signé avec l'app : Authenticode et notarisation dans `release.yml`, fusion universelle sous macOS, présence dans les paquets Linux. La CI vérifie sa présence sur les trois systèmes et la signature de code sous macOS.

## Les services de voix en ligne (INT-13)

- OpenAI, Groq, Mistral, ElevenLabs et Deepgram sont cinq plugins livrés, `dev.escouade.openai` (etc.), au registre de tous les builds. Chacun est l'entrée `escouade-plugins voice-<service>`. Réglages › Voix se comporte comme avant : la clé reste au trousseau (`dictation-<service>`), la dictée, le direct et la voix de l'agent passent par le plugin, sans différence visible.
- Les couper dans la vue Plugins retire le service de Réglages › Voix (la dictée repasse en local). Un plugin qui ne démarre pas laisse le service du cœur répondre, et son journal dit pourquoi.
- Un plugin d'un autre auteur peut servir un nouveau service de voix ou prendre la place de l'un des cinq (même identifiant de service).
- Délais : le plugin ajoute moins de 1 ms au texte final du direct (médiane et 99e centile), son poussé au rythme du micro ; détail et protocole `voice/*` dans `docs/PLUGINS.md` §4.13.
- Le compte Mistral : un plugin « Mistral » porte la voix et, plus tard, Mistral Vibe, avec une seule clé.
