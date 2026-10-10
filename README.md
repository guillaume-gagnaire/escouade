# Escouade

Le poste de pilotage de tes agents [Claude Code](https://claude.com/claude-code) : une application Windows et macOS pour piloter plusieurs instances de Claude Code en local, avec un onglet par projet, autant d'agents que nécessaire, des terminaux intégrés, des notifications quand Claude attend une réponse, les quotas de ton abonnement et des statistiques de consommation.

Site : [guillaume-gagnaire.github.io/escouade](https://guillaume-gagnaire.github.io/escouade/)

Escouade s'appelait auparavant « Claude Code Manager » : la mise à jour remplace l'ancienne installation et garde tes réglages et tes conversations. Projet indépendant, non affilié à Anthropic ; Claude et Claude Code sont des marques d'Anthropic.

## Fonctionnalités

- **Projets en onglets** : compteur de modifications git non commitées, pastille quand un agent attend une réponse, couleur par projet (qui teinte toute l'interface), réordonnables par glisser-déposer.
- **Agents** : une conversation Claude Code par agent, nommée automatiquement d'après la première demande. Modèle (Fable, Opus, Sonnet, Haiku, toujours le plus récent que connaît Claude Code, affiché avec sa version : « Sonnet 5.5 »), effort (jusqu'à `max`) et mode de permission (Auto, Demander, Plan, Édits auto, Bypass) modifiables en cours de conversation. Archivage, suppression, renommage.
- **Chat natif** : markdown avec coloration syntaxique, réflexion repliable, appels d'outils compacts et dépliables (diffs, sorties de commandes, sous-agents). Questions de Claude et demandes d'autorisation sous forme de cartes cliquables (ou réponse libre). Images (coller/glisser), autocomplétion `@fichier` et `/commande`, messages envoyés pendant que Claude travaille (il en tient compte dès sa prochaine étape, comme dans Claude Code), interruption par `Échap`.
- **Notifications** : l'onglet du projet et la carte de l'agent clignotent (ambre : question, vert : terminé, rouge : erreur) jusqu'à ce que tu affiches l'agent ; carillon, notification système (Windows : cliquable), clignotement de la barre des tâches (macOS : rebond de l'icône du Dock) et badge dans la zone de notification / la barre des menus quand un agent pose une question ou termine.
- **Git** : panneau des fichiers non commités (par agent ou pour tout le projet), visionneuse de diff (unifié / côte à côte, un choix qui s'applique aussi aux diffs de la conversation), commit rédigé par l'agent lui-même, **git graph** de tout le dépôt avec la branche de l'agent mise en avant (onglet « Historique », clic sur un commit pour son diff). Disposition **moitié / moitié** au choix : la conversation à gauche, les fichiers modifiés et leur diff à droite, rafraîchis pendant que l'agent travaille. Option **un worktree par agent** avec merge (ou squash) dans la branche du projet et nettoyage.
- **Éditeur intégré** : parcourir et modifier les fichiers du projet ou du worktree d'un agent sans quitter l'app (arborescence, onglets, coloration syntaxique, recherche, lignes modifiées marquées dans la marge, `Ctrl+S` pour enregistrer). Il s'ouvre depuis l'en-tête d'un agent, les fichiers non commités ou les fichiers édités dans la conversation (une modification s'ouvre à sa première ligne modifiée), et prévient quand un fichier ouvert change sur le disque pendant que l'agent travaille.
- **Terminaux** : vrais terminaux (ConPTY ou pty + xterm.js) PowerShell 7, Git Bash et WSL sous Windows, zsh, bash et fish sous macOS, avec l'autocomplétion native du shell.
- **Lancement du projet** : par projet, une liste de commandes (nom, ligne de commande, shell, sous-dossier) à lancer une par une ou toutes ensemble. Chacune tourne dans son propre terminal, en lecture seule, et garde son log d'un lancement à l'autre. Statut en direct (en cours, arrêté, terminé, planté avec le code de sortie et une notification) ; lancer, relancer, stopper. « ✦ Remplir automatiquement » laisse Claude lire le projet (sans rien modifier) et proposer ces commandes, à relire avant d'enregistrer.
- **Barre de statut** : agents actifs, en attente et terminés, quota de session 5 h (avec délai avant réinitialisation), quota hebdomadaire, coût du jour. Tokens et coût montent en direct pendant que Claude travaille (estimation « ≈ » d'après les tarifs publics), puis prennent le chiffre exact de Claude Code à la fin du tour.
- **Statistiques** : tokens (entrée, cache, sortie) par jour, semaine ou mois, coût global, coût moyen par prompt, répartition par projet et par modèle.
- **Remote control** : clic droit sur un agent → « Activer le remote control ». Sa session devient accessible depuis claude.ai et l'app Claude sur mobile ; ce que tu y envoies s'affiche aussi dans l'app, et l'agent reste joignable tant que l'app tourne (même session après un redémarrage).
- **Fermer la fenêtre ne coupe pas les agents** : l'app reste dans la zone de notification. Au redémarrage, chaque agent reprend sa session Claude (`--resume`). Les processus inactifs sont arrêtés après un délai réglable et reprennent automatiquement à la prochaine action.
- **Proxy réseau** configurable (processus Claude, quotas, mises à jour, et optionnellement terminaux).
- **Mises à jour automatiques** via les releases GitHub.

## Prérequis

- Windows 10 ou 11 (WebView2, présent par défaut sur Windows 11), ou macOS 11 (Big Sur) ou plus récent, sur Mac Apple Silicon ou Intel.
- [Claude Code](https://code.claude.com/docs/fr/overview) installé et connecté (`claude` dans le `PATH`, ou chemin indiqué dans les réglages).
- Git : Git for Windows, ou sous macOS celui des outils en ligne de commande Xcode (`xcode-select --install`) ou de Homebrew.
- Optionnel sous Windows : PowerShell 7 (à défaut, les terminaux utilisent Windows PowerShell), WSL.

Sous macOS, une app lancée depuis le Finder ou le Dock ne reçoit pas le `PATH` du terminal : Escouade reprend celui de ton shell de connexion (`$SHELL -ilc`) au démarrage, et cherche aussi `claude` dans `~/.local/bin`, `/opt/homebrew/bin` et `/usr/local/bin`.

## Installation

**Windows** : télécharge l'installeur `.exe` de la [dernière release](https://github.com/guillaume-gagnaire/escouade/releases/latest).

**macOS** : télécharge le `.dmg` (universel, Apple Silicon et Intel) de la [dernière release](https://github.com/guillaume-gagnaire/escouade/releases/latest), ouvre-le et glisse Escouade dans Applications. Depuis la 1.5.4, l'app est signée avec un certificat Apple Developer ID et notarisée par Apple : macOS l'ouvre sans demander d'autorisation.

Les versions suivantes s'installent depuis l'application (barre de statut → « Mise à jour disponible »).

## Données locales

Tout est stocké dans `~/.escouade/` (anciennement `~/.claude-code-manager/`, déplacé au premier lancement de la 0.1.4) :

| Fichier | Contenu |
|---|---|
| `settings.json` | réglages |
| `state.json` | projets, agents, état de l'interface |
| `conversations/<agent>.jsonl` | journal de chaque conversation |
| `stats.db` | statistiques (SQLite) |
| `app.log` | journal de l'application |

La variable d'environnement `ESCOUADE_DATA_DIR` (l'ancienne `CCM_DATA_DIR` marche encore) permet d'utiliser un autre dossier (démonstrations, tests).

## Raccourcis

Sous macOS, `Ctrl` devient `⌘` (sauf `Ctrl+Tab`, `⌘Tab` changeant d'application) ; dans les terminaux, `⌘C` / `⌘V` copient et collent, et les touches `Ctrl` vont au shell.

| Raccourci | Action |
|---|---|
| `Ctrl+1` … `Ctrl+9` | aller au projet n |
| `Ctrl+N` | nouvel agent |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | agent suivant / précédent |
| `Ctrl+J` | prochain agent en attente de réponse ou à voir (question, fin ou erreur pas encore vue) |
| `Ctrl+Entrée` / `Ctrl+Shift+Entrée` (dans la conversation) | autoriser / toujours autoriser la demande d'autorisation en attente (le texte tapé puis `Entrée` la refuse avec ce message) |
| `Alt+1` … `Alt+9` (dans la conversation) | choisir l'option n de la question en attente (coche ou décoche pour un choix multiple) ; `Ctrl+Entrée` valide quand la réponse est complète |
| `Ctrl+T` | nouveau terminal |
| `Ctrl+S` (dans l'éditeur) | enregistrer le fichier |
| `Ctrl+Shift+B` | panneau des fichiers non commités (disposition classique ; toujours affiché dans l'autre) |
| `Ctrl+Shift+L` | disposition classique / conversation et fichiers côte à côte |
| `Ctrl+,` | réglages |
| `Échap` (dans le champ de saisie) | interrompre Claude |
| `↑` (champ vide) | rappeler le dernier message envoyé, pour le renvoyer tel quel ou modifié (l'original reste dans la conversation) |

## Développement

Prérequis : Node.js 20.18+ et rustup (MSVC sous Windows ; sous macOS, les outils en ligne de commande Xcode). La version de Rust est épinglée dans `src-tauri/rust-toolchain.toml` (la même qu'en CI) et rustup l'installe automatiquement.

```powershell
npm ci
npm run tauri dev
```

Pour travailler sans toucher à tes vraies données (une version de développement lancée sans dossier à part déplacerait aussi `~/.claude-code-manager/`) :

```powershell
$env:ESCOUADE_DATA_DIR = "$env:TEMP\escouade-sandbox"; npm run tauri dev
```

Sous macOS :

```sh
ESCOUADE_DATA_DIR="$TMPDIR/escouade-sandbox" npm run tauri dev
```

### Tests

| Commande | Contenu |
|---|---|
| `npm run check` | typage (svelte-check) |
| `npm test` | Vitest + Testing Library : logique, stores et composants |
| `cd src-tauri; cargo test` | tests unitaires Rust et tests d'intégration du cœur contre un faux CLI `claude` (`tests/fixtures/fake-claude.mjs`), de vrais dépôts git temporaires et le runtime de test de Tauri |
| `npx tauri build --debug --no-bundle; npm run test:e2e` | tests de bout en bout : Playwright pilote l'application réelle via le protocole DevTools de WebView2 (Windows uniquement) |

La CI (`.github/workflows/ci.yml`) exécute l'ensemble sur chaque push et pull request sous Windows, et sous macOS tout sauf les tests de bout en bout, plus la construction du `.dmg` (téléchargeable dans les artefacts du run pour tester une branche).

### Publier une version

1. Une seule fois, ajoute les secrets du dépôt GitHub :
   - `TAURI_SIGNING_PRIVATE_KEY` : le contenu de la clé privée de signature des mises à jour (générée avec `npx tauri signer generate`, la clé publique correspondante est dans `src-tauri/tauri.conf.json`) ;
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` : son mot de passe (vide s'il n'y en a pas) ;
   - optionnel, pour signer et notariser l'app macOS (sans eux, elle est signée ad hoc et macOS demande une confirmation au premier lancement) : `APPLE_CERTIFICATE` (certificat Developer ID Application exporté en `.p12`, encodé en base64), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` (ex. `Developer ID Application: Nom (TEAMID)`), `APPLE_ID`, `APPLE_PASSWORD` (mot de passe d'application) et `APPLE_TEAM_ID`.
2. Mets à jour la version partout, committe, tague et pousse :

   ```powershell
   npm run version:set -- 0.2.0
   git commit -am "chore: release 0.2.0"
   git tag v0.2.0
   git push origin main v0.2.0
   ```

   Ou crée la release depuis la page GitHub (« Draft a new release », nouveau tag `v0.2.0` sur `main`, « Publish ») : l'app prend la version du tag, même si celle de `main` n'a pas été montée.

Le workflow `release.yml` vérifie que le tag est une version, crée la release, puis, sous Windows et sous macOS en parallèle, donne à l'app la version du tag, lance les tests, construit l'installeur (`.exe` NSIS ; `.dmg` et `.app.tar.gz` universels), le signe et l'ajoute à la release avec le `latest.json` utilisé par la mise à jour automatique, commun aux deux systèmes. La mise à jour lit le `latest.json` de la dernière release : si une construction échoue, la release repasse en brouillon, pour que la mise à jour retrouve la précédente. Pour réessayer, supprime ce brouillon et son tag sur GitHub, corrige, puis publie à nouveau.

Tague toujours la tête de `main`, et attends que la release soit créée (première minute du workflow) avant de pousser d'autres commits : une fois `main` plus loin que le tag, le `GITHUB_TOKEN` des Actions n'a plus le droit de créer la release (« Resource not accessible by integration »).

### Vidéo de présentation

Le dossier `video/` contient la vidéo de présentation (Remotion). Sa voix off mène tout : chaque scène dure le temps de ses phrases (`src/script.ts`), et l'animation se cale sur les mots prononcés. La voix et la musique viennent d'ElevenLabs (clé dans `ELEVENLABS_API_KEY`, jamais dans un fichier) ; elles sont versionnées, et seules les phrases modifiées sont redemandées :

```powershell
cd video
npm install
$env:ELEVENLABS_API_KEY = "…"
npm run voice               # public/voice/*.mp3 et src/voice.json
npm run voice -- --check    # retranscrit chaque phrase pour vérifier qu'elle est bien prononcée
npm run music               # public/music.mp3, recomposée quand la durée des scènes change
npm run studio              # aperçu
npm run stills -- board     # images de contrôle d'une scène dans out/stills
npm run render              # bruitages + out/presentation.mp4
npm run web-video           # out/escouade-web.mp4 : son à -16 LUFS, plus léger
npm test                    # chaque fonctionnalité montrée, cues, mixage, sous-titres
```

### Site

Le dossier `website/` contient le site de présentation (Nuxt, généré en statique), publié sur GitHub Pages à chaque push qui le touche. Il demande Node 22.19 ou plus récent :

```powershell
cd website
npm install
npm run dev        # aperçu
npm run generate   # site statique dans .output/public
npm test           # vérifie le site généré
```

Ses images, sa vidéo et ses sous-titres viennent de `video/` : `npm run render`, `npm run web-video`, puis `npm run site-images`.

## Architecture

- `src-tauri/` : backend Rust (Tauri 2).
  - `claude.rs` : pilotage d'un processus `claude` en mode `stream-json`, avec son protocole de contrôle (voir [docs/PROTOCOL.md](docs/PROTOCOL.md)).
  - `agent.rs` : normalisation des messages en conversation, statuts, questions et permissions, usage par tour.
  - `core.rs` : orchestration.
  - Modules annexes : `git.rs`, `pty.rs`, `stats.rs`, `usage.rs`, `notify.rs`, `job.rs` (arbres de processus : Job Objects sous Windows, groupes de processus sous macOS), `shellenv.rs` (`PATH` du shell de connexion sous macOS).
  - `tauri.macos.conf.json` : ce qui change sous macOS (barre de titre native avec les boutons de fenêtre, bundle `.app` / `.dmg`).
- `src/` : interface Svelte 5.
- `design/` : maquette de référence.
- `docs/SPEC.md` : spécification.

## Licence

MIT, voir [LICENSE](LICENSE).
