# Données locales

Tout ce qu’Escouade garde sur ton disque : le dossier ~/.escouade/ et chacun de ses fichiers, l’endroit où vivent les jetons, ce qu’elle lit des données de Claude Code, et comment sauvegarder ou remettre à zéro.

## Le dossier de données

Escouade range ses données dans `~/.escouade/` (`C:\Users\<toi>\.escouade` sous Windows). Réglages › À propos le rappelle sur la ligne « Données locales ».

| Fichier ou dossier | Contenu |
|---|---|
| `settings.json` | Les réglages. |
| `state.json` | Les projets, les agents (avec l’identifiant de leur session Claude Code), les tickets du Kanban, la pause du pilote auto et l’état de l’interface. |
| `integrations.json` | Les comptes Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards, sans leurs jetons (voir [Les jetons](#les-jetons)). |
| `sync-queue.json` | Les synchronisations avec ces services qui attendent d’être retentées. |
| `tickets/<ticket>/` | Les images collées ou glissées dans la fenêtre d’un ticket, ou jointes par le serveur MCP, une par fichier : `state.json` n’a que leurs noms. Les autres fichiers joints au ticket sont des copies sous `files/<hasard>/<nom>`, dans ce même dossier (`state.json` n’en garde que le nom, l’emplacement et la taille). Le dossier n’existe que si le ticket a des images ou des fichiers ; effacé avec le ticket, ou quand l’historique de son projet fermé est effacé. |
| `attachments/<agent>/` | Les fichiers que tu joins aux messages d’un agent, ou que tu colles ou déposes dans son terminal (vue Claude Code), quand ils sont copiés : chacun dans un sous-dossier au nom tiré au hasard (8 caractères), sous son nom d’origine, nettoyé. `attachments/_drafts/` garde les copies d’un ticket pas encore enregistré, vidées après 24 h (au démarrage, puis chaque heure). Effacé avec l’agent, avec son projet quand tu le supprimes pour de bon, ou par la purge si tu l’as réglée (voir [Réglages › Harness](settings-claude-code#pieces-jointes)). Un fichier joint à son emplacement d’origine n’est jamais ici. |
| `conversations/<agent>.jsonl` | Le journal de chaque conversation, nommé d’après l’identifiant de l’agent, à raison d’une opération (ajout ou correction d’un élément) par ligne. Effacé avec l’agent ; ceux d’un projet fermé restent avec son historique. |
| `closed-projects/<projet>.json` | L’historique d’un projet fermé, nommé d’après l’identifiant du projet : le projet entier (réglages, Kanban, intégrations, commandes), ses agents (archivés compris), ses tickets, l’agent affiché et la date de fermeture. Ses conversations restent dans `conversations/`, les images et les fichiers de ses tickets dans `tickets/`, les copies de fichiers joints à ses agents dans `attachments/`, ses statistiques dans `stats.db`. Le fichier disparaît quand le projet est rouvert ou que son historique est effacé (voir [Projets fermés](settings-projects#projets-fermes)). |
| `claude/<compte>/` | Le dossier de configuration de Claude Code de chaque compte ajouté dans Réglages › Comptes : sa connexion et ses conversations. Le compte de la machine (Principal) n’en a pas : il utilise celui de ton propre Claude Code. |
| `mcp-token.json` | Le jeton du serveur MCP pour Claude hors Escouade, seulement quand le trousseau du système le refuse (toujours sous Linux). |
| `mcp/<agent>.json` | La configuration MCP d’un agent d’Escouade, avec son jeton. Effacée quand son processus s’arrête, et à chaque démarrage et arrêt de l’app. |
| `tui/<agent>.json` | Les réglages de hooks que la vue Claude Code d’un agent passe à Claude Code. Le jeton n’y est pas : il passe par une variable d’environnement. |
| `logs/<agent>-setup.log` | La sortie complète de la dernière préparation du worktree d’un agent. Effacé avec l’agent ; celui d’un projet fermé reste avec son historique. |
| `voice/models/` | Les modèles de Whisper téléchargés pour la dictée et la conversation vocale (`ggml-<modèle>.bin`). Chacun s’efface depuis [Réglages › Voix](settings-voice#les-modeles-de-whisper). |
| `voice/voices/` | Les voix de la conversation vocale téléchargées (`<voix>.onnx` et `<voix>.onnx.json`). Chacune s’efface depuis Réglages › Voix. |
| `voice/espeak-ng-data/` | Les données de prononciation d’eSpeak NG (français et anglais), écrites par l’app à la première lecture à voix haute et réécrites après une mise à jour. |
| `plugins/<id>/` | La copie de chaque plugin installé depuis un dossier ou un dépôt git, sans son historique git (voir [le manifeste](plugins/manifest#installer-un-plugin)). Effacée quand le plugin est désinstallé. |
| `plugins.json` | La liste des plugins installés : leur origine (dossier, dépôt et étiquette, dossier en développement) et leur empreinte. |
| `plugin-logs/<id>.log` | Le journal de chaque plugin qui a un programme : ses démarrages, ses arrêts, ses plantages, sa sortie d’erreur et ses messages, sans ses secrets (voir [le protocole](plugins/api#le-journal)). Au-delà de 1 Mo, il devient `<id>.log.1`. |
| `update.json` | La mise à jour en cours d’installation, notée pour le démarrage suivant puis effacée. |
| `stats.db` | Les statistiques (une base SQLite, avec ses fichiers `stats.db-wal` et `stats.db-shm` tant qu’Escouade tourne). |
| `telemetry-id` | L’identifiant tiré au hasard de cette installation, pour les statistiques d’usage. Créé quand tu les acceptes, supprimé quand tu les refuses ou les désactives. |
| `app.log` | Le journal de l’application. Au démarrage, s’il dépasse environ 5 Mo, il devient `app.old.log`. |
| `settings.broken.json`, `state.broken.json`… | Les copies des fichiers que l’app n’a pas pu lire (voir [Un fichier illisible](#un-fichier-illisible)). |

## L’ancien dossier de Claude Code Manager

Escouade s’appelait « Claude Code Manager » et gardait ses données dans `~/.claude-code-manager/`. Au premier lancement qui le trouve, elle le déplace en `~/.escouade/` et y laisse un fichier `.migrated-from-claude-code-manager`. Les dossiers du moteur web et de l’état de la fenêtre, nommés d’après l’identifiant de l’app, suivent le même chemin. En pratique, la mise à jour garde tes réglages, tes projets et tes conversations : voir les [mises à jour](updates).

Les cas rares sont traités sans rien perdre :

- Un dossier `~/.escouade/` déjà là sans ce fichier (créé par un lancement qui n’avait pas pu déplacer l’ancien dossier) n’est jamais effacé : il est mis de côté sous le nom `.escouade.before-migration-` suivi d’un nombre, et l’ancien dossier prend sa place.
- Si l’ancien dossier ne peut pas être déplacé parce qu’un fichier est encore ouvert, Escouade continue de l’utiliser et réessaie au lancement suivant.

## Choisir un autre dossier

La variable d’environnement `ESCOUADE_DATA_DIR`, quand elle est définie et non vide, remplace `~/.escouade/` par le dossier indiqué. L’ancienne variable `CCM_DATA_DIR` marche encore, mais `ESCOUADE_DATA_DIR` passe avant. Définis-la dans le terminal depuis lequel tu lances Escouade.

C’est fait pour les démonstrations et les essais. Avec un dossier à part :

- Escouade ne déplace rien de `~/.claude-code-manager/`.
- Les jetons vont dans une entrée du trousseau à part (le service `dev.gagnaire.escouade.sandbox`) : ils ne remplacent jamais les tiens.
- Une autre instance peut tourner à côté de celle qui utilise le dossier habituel.
- Le redémarrage automatique pour une mise à jour est désactivé.

## Les jetons

Les jetons ne sont pas dans les réglages :

- **Jira, Trello, GitHub, Linear, ClickUp.** Le jeton de chaque service est dans le trousseau du système : le Credential Manager de Windows, le Trousseau d’accès de macOS. Il y a une entrée par service (`jira`, `trello`, `github`, `linear`, `clickup`) sous le service `dev.gagnaire.escouade`. `integrations.json` garde le reste du compte. Quand le trousseau refuse l’entrée, Réglages › Intégrations le dit (« Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. ») et le jeton reste dans ce fichier, lisible par toi seul sous macOS et Linux.
- **Linux.** Il n’y a pas de trousseau pris en charge : les jetons des intégrations restent dans `integrations.json`.
- **Serveur MCP.** Le jeton de Claude hors Escouade est dans le trousseau (une entrée `mcp`), sinon dans `mcp-token.json`. Celui de chaque agent est dans son `mcp/<agent>.json`, qui disparaît avec son processus.
- **Dictée en ligne.** La clé de chaque service est dans le trousseau, une entrée par service (`dictation-openai`, `dictation-groq`, `dictation-mistral`, `dictation-elevenlabs`, `dictation-deepgram`), jamais dans un fichier : sous Linux, elle ne peut pas être enregistrée. « Effacer la clé », dans Réglages › Voix, retire son entrée. Voir [la dictée](chat).
- **Connexion de Claude Code.** Elle n’est pas à Escouade : Claude Code la garde dans le dossier de chaque compte (le fichier `.credentials.json`), et sous macOS dans le trousseau. Voir [les comptes Claude](accounts).

Pour retirer un jeton, déconnecte le service dans Réglages › Intégrations : son entrée quitte le trousseau. Voir [les intégrations](integrations).

## Ce que lit Escouade de Claude Code

Escouade pilote Claude Code et garde ses propres conversations, mais elle s’appuie aussi sur ses données :

- **Les sessions.** Chaque agent a une session Claude Code. Escouade en note l’identifiant dans `state.json` et relance `claude --resume` sur cette session quand l’app redémarre ou qu’un processus arrêté reprend : l’agent retrouve tout son contexte. Les sessions elles-mêmes sont dans le dossier de configuration de Claude Code du compte de l’agent (`~/.claude` par défaut pour le compte de la machine, `~/.escouade/claude/<compte>/` pour les autres), sous `projects/`. Supprimer un agent retire sa conversation d’Escouade ; la session Claude Code reste sur le disque. Si une session a disparu, l’agent le dit (« Session Claude introuvable : une nouvelle session sera démarrée au prochain message. »).
- **La connexion, pour les quotas.** Escouade lit, sans l’écrire, la connexion que Claude Code garde pour chaque compte, et s’en sert pour demander ses quotas à Anthropic. Voir [les quotas](usage).
- **Le transcript, dans la vue Claude Code.** Pour suivre un agent qui tourne dans l’interface de Claude Code, Escouade lit la session qu’il écrit (`projects/<dossier>/<session>.jsonl`). Voir [la vue Claude Code](claude-code-view).
- **La configuration MCP.** Quand « Les agents peuvent piloter Escouade » est activé (Réglages › Serveur MCP), Escouade déclare son serveur MCP dans la configuration de chaque compte actif de chaque harness installé (Claude Code, Codex, OpenCode, Copilot CLI), le jeton en clair. Voir [le serveur MCP](mcp).
- **Un nouveau compte** reprend du compte de la machine, par lien ou par copie, ses réglages, son `CLAUDE.md`, ses skills, agents, commandes, plugins, hooks et styles de sortie. Jamais sa connexion ni son `.claude.json`.

## Les statistiques d’usage

Si tu les as acceptées (question du premier démarrage, ou interrupteur de Réglages › À propos), Escouade envoie des statistiques d’usage, sans rien qui t’identifie, à un serveur d’Escouade hébergé en France. Sur ton disque, elles ne laissent que `telemetry-id`, un UUID tiré au hasard, qui ne dérive de rien de ta machine ni de toi. Les événements attendent en mémoire et ne sont jamais écrits dans un fichier.

Le détail de ce qui est envoyé, de ce qui ne l’est jamais et de la façon de demander une suppression est sur la page [Confidentialité](/privacy/). Voir aussi les [réglages « À propos »](settings-about).

## Ce qui n’est pas dans le dossier

- **Tes projets et les worktrees des agents.** Ils sont chez toi, dans chaque projet : `.claude/worktrees` par défaut, ou le « Dossier des worktrees » réglé pour le projet. Fermer un projet ne les touche pas.
- **Les sessions de Claude Code**, vues plus haut.
- **La taille et la position de la fenêtre.** Le fichier `.window-state.json` est dans le dossier de configuration du système, sous `dev.gagnaire.escouade` : `%APPDATA%\dev.gagnaire.escouade` sous Windows, `~/Library/Application Support/dev.gagnaire.escouade` sous macOS, `~/.config/dev.gagnaire.escouade` sous Linux.
- **Quelques préférences d’affichage**, gardées par le moteur web de l’app : le texte que tu as commencé dans un champ de message (il survit à un redémarrage), la largeur de l’arborescence de l’éditeur, la hauteur du bandeau « Plan », la présentation des diffs, la période des statistiques.

## Sauvegarder

1. Quitte vraiment Escouade : fermer la fenêtre la laisse dans la zone de notification. Utilise « Quitter » dans le menu de son icône.
2. Copie le dossier `~/.escouade/`.

La copie contient tes réglages, tes projets, tes tickets, tes conversations et tes statistiques. Elle contient aussi, sous `claude/`, la connexion des comptes Claude que tu as ajoutés (sous Windows et Linux) : garde-la privée.

Pour la rétablir sur une autre machine, remets le dossier au même endroit, avec tes projets aux mêmes chemins (`state.json` retient des chemins complets). Trois choses ne suivent pas :

- Les jetons des intégrations (sauf sous Linux) : reconnecte chaque service.
- La connexion du compte de la machine : elle est dans ton Claude Code.
- Les sessions des agents, qui sont dans le dossier de Claude Code : sans elles, un agent repart d’une nouvelle session.

## Remettre à zéro

Quitte Escouade d’abord, puis :

| Pour… | Fais… |
|---|---|
| Retrouver les réglages par défaut | Supprime `settings.json`. |
| Effacer les statistiques | Supprime `stats.db`. |
| Repartir sans projet, agent ni ticket | Supprime `state.json`, et `conversations/` pour ne pas garder les journaux. Supprime aussi `closed-projects/` pour oublier les projets fermés. |
| Effacer l’historique d’un projet fermé | Dans Réglages › Projets, « Projets fermés », « Effacer l’historique… » : il supprime son fichier de `closed-projects/` et les journaux de ses agents. Supprimer le fichier à la main laisserait ses journaux dans `conversations/`. |
| Tout effacer | Supprime le dossier `~/.escouade/`. Les connexions des comptes ajoutés dans `claude/` partent avec lui. |
| Retirer les jetons des intégrations | Déconnecte chaque service dans Réglages › Intégrations. |

Les fichiers de tes projets, les worktrees des agents et les sessions de Claude Code ne sont jamais touchés par une remise à zéro : supprime les worktrees dont tu ne veux plus avec git.

## Un fichier illisible

Quand `settings.json`, `state.json`, `integrations.json` ou `sync-queue.json` n’est pas du JSON valide, Escouade écrit l’erreur dans `app.log`, garde une copie du fichier sous un nom comme `state.broken.json`, et démarre avec les valeurs par défaut pour ce fichier. La copie permet de retrouver à la main ce qui y était, par exemple la liste des projets. Voir le [dépannage](troubleshooting).
