---
title: "Livraison et mises à jour"
subject: "Introduction"
sources: ["docs/SPEC.md",".github/workflows/release.yml",".github/workflows/discord.yml",".github/workflows/ci.yml","scripts/beta-channel.mjs","scripts/beta-notes.mjs","scripts/image-tags.mjs","scripts/server-release.sh","scripts/team-server-trial.sh","scripts/compat-2.2.sh","docs/BETA-2.3.md","src-tauri/src/updates.rs","src/components/modals/UpdateModal.svelte","src-tauri/windows/installer-hooks.nsh","tests/e2e/fixture.ts","tests/e2e/update-guard.ts",".github/workflows/advisories.yml","scripts/release-sums.mjs","scripts/no-testing.mjs","docs/SECURITY-DEPENDENCIES.md","docs/security/audit-2026-11/rapport.md (SEC-04, SEC-16, SEC-18)"]
tickets: ["ESC-374","ESC-380","ESC-533","ESC-543","ESC-586","ESC-594"]
decisions: [114,244,251,277,300]
compared: null
---

## Ce que c'est

Comment Escouade est livrée et se met à jour : un paquet par système, une mise à jour intégrée depuis GitHub Releases, et la chaîne CI/CD qui construit et publie chaque version. L'interface est embarquée dans le binaire : une mise à jour s'installe puis l'app redémarre, sans remplacement à chaud.

## Comportement

### Paquets

- Windows : installeur NSIS, par utilisateur, sans droits d'administrateur, en anglais ou en français selon la langue du système.
- macOS : `.dmg` universel, signé Developer ID et notarisé.
- Linux : AppImage et `.deb`.

L'interface est en français et en anglais (voir *Langues*).

### Recherche et téléchargement

- L'app cherche une nouvelle version sur GitHub Releases (`guillaume-gagnaire/escouade`) 8 s après son lancement, puis toutes les 5 minutes. « Rechercher une mise à jour » vérifie à la demande.
- Une version trouvée est téléchargée tout de suite en arrière-plan (15 min au plus) et sa signature vérifiée. Ses octets sont gardés jusqu'à l'installation. Une seule version est prête à la fois : une plus récente remplace l'autre, et une version retirée du serveur n'est jamais installée.
- Ce qui est lu dépend de « Recevoir les versions bêta » (voir *Versions bêta*) : décochée, `latest.json` de la dernière release seul ; cochée, `beta.json` de la même release d'abord, puis `latest.json` quand il manque. L'adresse de `beta.json` se déduit de celle de la configuration (dernier segment remplacé).
- La vérification et le téléchargement envoient `User-Agent: Escouade/<version> (<système>; <architecture>)` (Windows, macOS ou Linux), jamais celui d'un navigateur. Escouade ne déguise pas son trafic pour passer une règle de sécurité d'entreprise : elle s'annonce pour qu'on puisse l'autoriser.

### Versions bêta

- Une bêta porte la version qu'elle prépare suivie de `-beta.N` (tag `v2.2.0-beta.1`) ; l'ordre est celui de semver (`beta.10` après `beta.9`, une bêta avant sa version).
- « Recevoir les versions bêta » (Réglages › À propos), **décochée par défaut** : décochée, une version distante n'est proposée que plus récente et sans suffixe de préversion. Une app sur une bêta reste dessus jusqu'à la version stable qui la suit (2.2.0 après 2.2.0-beta.3), jamais une plus ancienne (2.1.5). Cochée, toute version plus récente est proposée, bêta comprise.
- `latest.json` de la dernière release ne contient jamais une bêta : les versions sans l'option (2.1.3 et avant), qui ne lisent que lui, ne la voient jamais.
- `beta.json`, toujours dans la dernière release stable, contient la plus récente des versions, stable ou bêta (voir *CI/CD*).
- L'option s'enregistre aussitôt, comme tout réglage (voir *Réglages*) ; la cocher lance une vérification comme « Rechercher une mise à jour », la décocher une vérification silencieuse qui retire une bêta proposée.
- Une app sur une bêta le dit après sa version, dans Réglages › À propos et en bas de la colonne des onglets (« Escouade 2.2.0-beta.1 (bêta) ») ; une mise à jour bêta aussi, dans la barre de statut, l'onglet et la modale (« Escouade 2.2.0-beta.2 (bêta) est prête »).
- **Une bêta n'est jamais annoncée comme la version stable** : partout où elle se lit, elle dit ce qu'elle est et comment revenir à la version stable d'avant (`scripts/beta-notes.mjs`, testé) :
  - sa release GitHub est une pré-version titrée « Escouade v2.3.0-beta.1 (beta) », dont les notes commencent par un encadré : une version d'essai, pas la version stable, qui n'arrive qu'à qui a coché « Recevoir les versions bêta » ; puis comment revenir ;
  - les deux salons Discord l'annoncent avec « (bêta) » (« (beta) » en anglais) après sa version, puis le même encadré, avant ses notes ;
  - les modales « Escouade X est prête » et « Nouveautés d'Escouade X » d'une bêta montrent un encadré : « C'est une version bêta : une version d'essai, pas la version stable. Elle peut avoir des défauts que la version stable n'aura plus. », « Revenir à la 2.2 : décoche « Recevoir les versions bêta » (Réglages › Général), puis réinstalle la dernière 2.2 depuis la page des versions. Tes projets, tes tickets et tes réglages restent. », et le bouton « Page des versions » (`…/releases/latest`, dans le navigateur).
- La version stable d'avant : la ligne précédente pour une bêta d'une version en `.0` (la 2.2 pour la 2.3.0-beta.1), la même ligne pour une bêta d'un correctif (la 2.3 pour la 2.3.4-beta.1), « la version stable » quand elle ne se déduit pas (une bêta de 3.0.0).
- **Revenir à la version stable** : décocher l'option garde sur la bêta jusqu'à la version stable qui la suit ; pour revenir tout de suite, on réinstalle la dernière version stable par-dessus, après avoir décoché (sinon elle repropose la bêta). Ce que la bêta a écrit se relit par la version stable d'avant sans rien mettre de côté ni remettre à zéro : un champ inconnu est ignoré, une valeur nouvelle d'une énumération prend une valeur par défaut. Pour la 2.3, `scripts/compat-2.2.sh` le vérifie avec le code de `main` (voir *CI/CD*). Ce qui n'existe que dans la bêta ne passe pas : un projet d'un serveur d'équipe redevient pour la 2.2 un projet de la machine, ses tickets ceux de son dernier état sur le poste.

### Échecs

- Une vérification est un essai, plus le repli sans proxy (ci-dessous). Un échec passager (réseau, 403, 404, 408, 425, 429, 5xx, réponse illisible) est réessayé une fois après 3 s.
- Un téléchargement raté relit `latest.json`. Si la même version y est, il réessaie une fois avec ce qu'elle donne ; sinon, ou s'il échoue encore, c'est l'échec. Il est retenté à la recherche suivante.
- Un échec automatique se tait d'abord. Il ne se montre qu'une fois les échecs enchaînés depuis au moins 1 h et au nombre d'au moins 3, et une seule fois par série. Une série dont le dernier échec date de plus de 30 min repart de zéro : un échec avant une mise en veille et deux au réveil ne font pas une alerte.
- La vérification et le téléchargement ont chacun leur série : un téléchargement refusé toutes les 5 min par un proxy finit par se dire, même si les vérifications passent. Un succès termine la sienne (une version trouvée ou rien de neuf pour la vérification, un téléchargement réussi pour le téléchargement).
- « Rechercher une mise à jour » montre toujours l'échec, avec sa raison, et « Aucune mise à jour disponible. » seulement quand la vérification a répondu.
- Le message commence par « Vérification des mises à jour impossible : » ou « Téléchargement de la mise à jour impossible : ». Il dit le code HTTP, le domaine qui a répondu (après redirections), le proxy par lequel la requête est passée, et ce que la réponse dit d'elle-même : `Proxy-Agent`, `Server`, ou la page de blocage d'un proxy connu (Zscaler, Netskope, Squid, Blue Coat, Forcepoint, Fortinet, Palo Alto, Umbrella, Sophos, Skyhigh…), puis le début de la page. Pour le savoir, un échec est suivi d'une sonde : la même requête GET, avec les mêmes proxy, TLS, User-Agent et `Accept`.
- Pour un 403, le message ajoute « bloqué par le réseau ou un proxy ? ». Pour un 407, il dit que le proxy demande une authentification, et qu'il faut mettre l'identifiant et le mot de passe dans l'adresse du proxy, dans Réglages › Réseau (et non « Télécharge-le dans le navigateur »).
- Un proxy d'entreprise qui refuse l'installateur est nommé : « le proxy de ton entreprise (Zscaler/6.2) a refusé l'installateur (403 Forbidden de release-assets.githubusercontent.com, par le proxy 127.0.0.1:9000). Télécharge-le dans le navigateur : il faudra peut-être confirmer l'avertissement du proxy. » Il ne l'est que d'après `Proxy-Agent`, ou un `Server` ou une page de blocage qui nomme un proxy connu. Un `Via` ne suffit pas : Squid l'ajoute à toute réponse, même à un refus de GitHub. Le nom est coupé à 160 caractères.
- Aucun lien signé (la requête `?sig=…` des liens de GitHub) n'apparaît dans un message ni dans le journal.
- Le toast d'un téléchargement raté propose « Télécharger dans le navigateur » et reste affiché 30 s. Il ouvre la page de la release (`…/releases/tag/<tag>`, tirée du lien de téléchargement) seulement si ce lien, celui de `latest.json`, est sur github.com en https, ou sur la même origine (schéma, hôte et port) que l'endpoint de mise à jour, jamais avec un identifiant dans l'adresse. Sinon, il ouvre la page des releases d'Escouade (`https://github.com/guillaume-gagnaire/escouade/releases/latest`).

### Proxy

- Sans proxy réglé, la mise à jour suit celui du système, sans repli ni nom de proxy dans le message.
- Avec le proxy de Réglages › Réseau, le repli sans proxy ne vaut que quand le proxy lui-même est injoignable : réglé mais arrêté, ou poste hors du réseau de l'entreprise (connexion refusée ou délai dépassé vers le proxy). La requête est refaite une fois sans proxy. Le journal dit que l'accès direct a marché ; un message d'échec dit les deux essais (« connexion impossible à github.com par le proxy 127.0.0.1:9000 (…) ; sans proxy : connexion impossible à github.com (…) »).
- Un refus venu du proxy (403, 407, tunnel refusé) n'entraîne aucun essai sans proxy : Escouade ne contourne jamais une règle de l'entreprise, et le message l'explique. Pour un tunnel refusé : « le proxy 127.0.0.1:9000 a refusé d'ouvrir la connexion à github.com (tunnel CONNECT refusé) : une règle de ton entreprise le bloque peut-être. », plus « Télécharge l'installateur dans le navigateur. » pour un téléchargement.

### Une version prête

- Modale « Escouade X est prête » : les notes de la version (Markdown ; celles de `latest.json`, en français, voir *CI/CD*), « Plus tard » et « Redémarrer maintenant ».
- « Redémarrer maintenant » est désactivé tant que des fichiers ne sont pas enregistrés (« Enregistre d'abord tes fichiers : N fichiers ne sont pas enregistrés. »), et le cœur le refuse aussi.
- Avec des agents en tour ou en attente : « N agents travaillent ou attendent ta réponse : leur tour en cours sera interrompu. Les agents de ticket reprennent d'eux-mêmes ; les autres attendent ton prochain message. » Au singulier : « 1 agent travaille ou attend ta réponse : son tour en cours sera interrompu. S'il travaille sur un ticket, il reprend de lui-même ; sinon, il attend ton prochain message. »

### Redémarrage automatique

L'app redémarre d'elle-même pour s'installer quand elle est au repos :

- aucun agent en tour, en question ou en préparation de worktree ;
- aucun sous-agent, workflow ou tâche de fond (commande, surveillance…) en cours pour un agent non archivé qui a un process : un redémarrage les tuerait avec lui ;
- rien en cours qu'un redémarrage couperait : ticket qui démarre, validation, merge, message en cours d'envoi, création ou copie d'agent, proposition ou commit direct, renommage ou suppression dans l'éditeur ;
- aucun lancement de test en préparation, aucune commande de lancement en cours ;
- aucun terminal avec une entrée ou une sortie depuis 5 min (un terminal au repos ne compte pas) ;
- aucun fichier non enregistré, aucune modale ouverte ;
- ni clavier ni souris depuis 5 min, ou fenêtre non visible.

Elle prévient 30 s avant, par la notification « Escouade redémarre pour se mettre à jour » (bouton « Plus tard » sous Windows) et le compte à rebours de la barre de statut. « Plus tard » repousse de 30 min. Tout ce qui rend l'app occupée pendant ces 30 s annule le redémarrage : la première touche ou le premier geste de souris est vu tout de suite, et le redémarrage est annulé au plus 2 s après. Les brouillons des zones de saisie sont écrits avant.

### Installation

- Toujours après l'arrêt propre de l'app (préparations, agents et terminaux arrêtés, état sauvegardé), borné à 15 s.
- Windows : l'installeur NSIS en mode `quiet` (aucune fenêtre), lancé après cet arrêt. macOS : le paquet `.app` est remplacé en place, puis l'app redémarre.
- Windows : une mise à jour attend d'abord que l'app qui l'a lancée ait fini de sortir, 30 s au plus : tant qu'un process tient encore `escouade.exe`. Windows met du temps à défaire un gros process, qu'il ne peut alors plus arrêter : sans cette attente, le contrôle « app ouverte » de l'installeur échouait et l'installeur silencieux abandonnait sans rien dire (événement 10006 du Restart Manager). Une app restée vraiment ouverte au-delà est fermée par ce contrôle, comme avant. Une installation à la main n'attend pas.
- Windows : un autre programme du dossier qui tourne pendant l'installation (`escouade-hook`, `escouade-plugin`, lancés hors de l'app) est mis de côté sous `<nom>.<n>.old`, son process continuant dessus, et le nouveau écrit à sa place : le dossier n'est jamais à moitié à jour. Ce qui a été mis de côté part à l'installation suivante, une fois que rien ne le fait plus tourner, ou à la désinstallation ; une installation qui échoue le remet à sa place.
- Windows : une mise à jour qui devait relancer l'app et ne s'installe pas la relance quand même, sur sa version : elle dit alors que la mise à jour n'a pas pu s'installer (voir *Après l'installation*), au lieu de disparaître sans rien.
- Une installation ratée avant l'arrêt laisse l'app tourner ; après, l'app redémarre telle qu'elle était.
- Redémarrée d'elle-même, l'app revient comme elle était : cachée dans la zone de notification ou la barre des menus, réduite dans la barre des tâches ou le Dock, derrière une autre app sans prendre le focus, ou devant (noté dans `update.json` du dossier de données).
- Quitter avec une mise à jour prête l'installe sans relancer l'app : « Quitter », et sous macOS aussi Cmd+Q ou le Dock, si le paquet est modifiable sans administrateur.
- Pendant une installation déjà en cours, « Quitter » attend sa fin (2 min au plus) au lieu de fermer l'app sous elle, et sous macOS Cmd+Q aussi (1 min au plus). Une installation qui attend le mot de passe d'un administrateur fait exception : l'app se ferme alors tout de suite, sans rien installer.

### Après l'installation

- Au démarrage suivant : « Escouade X est installée. » avec « Voir les nouveautés », qui ouvre la modale « Nouveautés d'Escouade X » (« Fermer »).
- Une installation ratée, vue au démarrage suivant ou pendant que l'app tourne, n'est plus retentée d'elle-même pour cette version pendant la session : « La mise à jour vers X n'a pas pu s'installer. » avec « Réessayer », qui ouvre la modale ; « Redémarrer maintenant » la retente.
- « Réessayer » et « Voir les nouveautés » n'ouvrent jamais leur modale par-dessus une autre : le toast revient pour plus tard. Une mise à jour qui n'est plus prête le dit : « La mise à jour vers X n'est plus prête : Escouade te la proposera de nouveau une fois téléchargée. »
- Un clic sur le bouton d'un toast laisse le focus où il était.

### CI/CD

Sur GitHub Actions :

- `ci.yml` (push, PR) : lint, typecheck et tests du frontend, `cargo fmt --check`, `clippy`, `cargo test`, build de vérification, sous Windows et sous macOS ; les tests de bout en bout sous Windows seulement.
- `release.yml` (tag `v*`) : la release GitHub est créée en brouillon (une release déjà là pour le tag y repasse ; un tag avec un `-` est une pré-version). Les builds Windows, macOS et Linux (AppImage et `.deb`) déposent chacun leurs fichiers dans ce brouillon : la signature des artefacts de mise à jour (secret `TAURI_SIGNING_PRIVATE_KEY`), l'installeur, le `.dmg`, l'AppImage, le `.deb`, et `latest.json`, que lit la mise à jour.
- Un dernier job vérifie `latest.json` : la version du tag ; les plateformes `windows-x86_64`, `darwin-aarch64`, `darwin-x86_64` et `linux-x86_64` ; la signature et le lien de chaque entrée ; chaque fichier désigné, présent et entier. Puis il publie la release (« latest », sauf une pré-version) et relit ce que lit la mise à jour. La mise à jour ne voit donc jamais une release à moitié construite.
- **Canal bêta** (`scripts/beta-channel.mjs`, testé) : une bêta publiée dépose son `latest.json` (avec ses notes) en `beta.json` dans la release « latest », à la place de celui qui y est, sauf s'il porte une version plus récente. Une version stable, avant d'être marquée « latest », met dans sa release son propre `latest.json` en `beta.json`, sauf si le `beta.json` de la release « latest » actuelle est plus récent (une bêta de la version suivante en cours) : elle reprend alors celui-là. Après la publication, le contrôle relit aussi `…/releases/latest/download/beta.json` (la version du tag ou une plus récente) et vérifie que `latest.json` n'y porte pas de préversion.
- **Les serveurs** (job « Servers », un par architecture, Linux x64 et arm64) : `escouade-server` et `escouade-team-server`, à la version de l'étiquette, construits, essayés chacun dans un conteneur Debian nu avant tout envoi (`scripts/server-trial.sh`, `scripts/team-server-trial.sh` : sa version, `init` et son jeton jamais au journal, `/healthz` sur 127.0.0.1, un client sans la poignée de main refusé en 426, un arrêt propre à SIGTERM), mis en archive avec leur SHA-256, signés par la clé des mises à jour (signature vérifiée avant l'envoi), et l'image de l'architecture poussée par son empreinte seule. Le même script fait les deux (`scripts/server-release.sh`). L'archive du serveur d'équipe porte le programme, sa licence et ses exemples de déploiement (systemd, compose avec Caddy, Coolify) ; son image se construit de cette archive.
- **Les images** : une fois la release publiée, un job par programme met les deux architectures sous un index `ghcr.io/<dépôt>/<programme>`, étiqueté du numéro de la version, et de `latest` pour une version stable seulement, `beta` pour une pré-version (`scripts/image-tags.mjs`, testé) ; puis il atteste sa provenance et la vérifie comme un utilisateur.
- La CI essaie aussi le serveur d'équipe : le conteneur nu, puis l'image faite de son archive, essayée comme un utilisateur la lance (volume neuf, lecture seule, aucune capacité, `init`, `/healthz`, arrêt propre, redémarrage sur le même volume).
- **Le retour à la 2.2** (`scripts/compat-2.2.sh`, ESC-586) : un test de la 2.3 écrit, par son propre code, un dossier de données et un dépôt avec ce que la 2.2 ne connaît pas ; les sources de `main` (par `git archive`) reçoivent un module de test qui les relit avec le code de la 2.2 : rien de mis de côté au démarrage, chaque fichier lu, et ce que la 2.3 a écrit retrouvé. Ce que la 2.2 a et que la 2.3 n'a pas écrit est dit en avertissement : perdu à l'aller-retour tant que `main` n'est pas reversée dans `release/2.3`.
- Un build raté laisse le brouillon, et « latest » reste la précédente ; relancer les jobs ratés est sans danger.
- **Notes de version**, tirées des deux changelogs, qui ont chacun une section par version :
  - celles de la release GitHub : la section de la version dans `CHANGELOG.en.md`, en anglais, après l'encadré d'une bêta ;
  - celles de `latest.json`, que montre la modale « Escouade X est prête » : sa section de `CHANGELOG.md`, en français.
  - une préversion sans section à elle prend celle de `## [Non publié]` (`## [Unreleased]` en anglais) : une bêta sort sans qu'on renomme la section.
- **Discord** : une fois la release publiée, le job `announce` poste ses notes sur les deux salons du Discord d'Escouade, chacun à part (un salon qui refuse n'empêche pas l'autre) : le salon français (section de `CHANGELOG.md`, webhook du secret `DISCORD_WEBHOOK_URL`) et le salon anglais (section de `CHANGELOG.en.md`, `DISCORD_WEBHOOK_URL_EN`). Une pré-version est annoncée comme une bêta (voir *Versions bêta*) ; rien pour un build raté. `discord.yml` se lance aussi à la main pour une version déjà publiée (`gh workflow run discord.yml -f tag=vX.Y.Z`, `-f lang=fr` ou `en` pour un seul salon) ; un salon refusé se reposte seul en relançant le job raté.
- Les commits sont atomiques, sur `main`, sans push.

## Réglages

- « Installer les mises à jour automatiquement » : activé par défaut.
- « Rechercher une mise à jour » : vérifie à la demande.
- « Recevoir les versions bêta » : désactivé par défaut, enregistré aussitôt (voir *Versions bêta*).
- Le proxy des mises à jour est celui de Réglages › Réseau (voir *Réglages*).

## Limites

- Pas de remplacement à chaud : chaque mise à jour passe par un redémarrage.
- Jamais de redémarrage automatique dans une exécution isolée (`ESCOUADE_DATA_DIR`).
- Sous macOS, quand remplacer l'app demanderait le mot de passe d'un administrateur, l'app ne redémarre jamais d'elle-même.
- Une fin de session Windows n'installe rien.
- Les notes montrées par la modale « Escouade X est prête » sont en français, quelle que soit la langue de l'interface.
- Sous Windows, c'est l'installeur de la nouvelle version qui tourne : l'attente de la sortie de l'app, la mise de côté des programmes en cours d'usage et la relance après un échec valent dès la mise à jour vers la version qui les porte, même lancée par une app plus ancienne.

- Un build de debug (développement, tests de bout en bout, tests Rust) ne lit jamais le vrai serveur des mises à jour, seulement un serveur de test : celui de `ESCOUADE_UPDATE_ENDPOINT`, ou les adresses de sa configuration quand elles sont toutes sur cette machine (`localhost`, `127.x.x.x`, `::1`). Sans serveur de test, la recherche est coupée : rien de neuf, et « Rechercher une mise à jour » dit « les mises à jour sont coupées dans un build de debug ». Il ne télécharge et n'installe un installeur que venu d'un serveur sur cette machine, jamais une vraie version : celle-ci s'installerait dans le dossier de l'app installée sur le poste, qu'elle fermerait d'abord. Un installeur refusé laisse « Quitter » fermer l'app comme d'habitude. Un build de release ne change pas : il lit l'adresse de sa configuration et ignore `ESCOUADE_UPDATE_ENDPOINT`.
- Les tests de bout en bout ne donnent un serveur de mises à jour que dans les tests des mises à jour, et un test échoue si son app a lu une autre adresse ou lancé une installation (lu dans son journal).

## Depuis ESC-594 : l'intégrité de la chaîne de publication (audit SEC-04, SEC-16, SEC-18)

- **Les empreintes des installateurs** : avant de publier la release, son dernier job calcule l'empreinte SHA-256 de chacun de ses fichiers, hors `latest.json`, `beta.json` et signatures (`.sig`), les relit par `sha256sum -c` et les dépose dans `SHA256SUMS.txt`, au format de `sha256sum` (`<empreinte>  <nom>`, par nom ; `scripts/release-sums.mjs`, testé). Un job relancé le remplace. Les notes de la release le disent, avec les commandes de chaque système, et la page Installation du site a une section « Vérifier le téléchargement » (`Get-FileHash`, `shasum -a 256`, `sha256sum -c SHA256SUMS.txt --ignore-missing`), qui rappelle l'attestation de provenance (`gh attestation verify`).
- **Aucun code de test dans un build publié** : `scripts/no-testing.mjs` lit `cargo tree` de l'espace de travail de l'app (ses trois binaires), pour toutes les cibles, sans les dépendances de développement, et échoue en nommant chaque crate qui y a la fonction `testing`. La CI le lance (job Windows de format et Clippy), et `release.yml` avant la construction.
- **Les secrets Apple** ne sont donnés qu'à l'étape qui construit et signe l'app macOS, comme la clé des mises à jour et les secrets Azure ; plus rien ne les pose dans l'environnement du job. Le job ne sait que s'il y a un certificat (`APPLE_SIGNING`, `true` ou `false`) : avec, la construction signée par Developer ID et notarisée ; sans, la même construction que Windows et Linux, l'app macOS signée sur place. Les deux formes de l'étape restent pareilles (un test le vérifie).
- **Les avis des dépendances** (`advisories.yml`) : l'app échoue sur tout avis, le store et le site sur un avis élevé ou critique (`npm audit --omit=dev --audit-level=high`), leurs avis faibles et moyens restant dits dans le run. Les délais de correction sont dans `docs/SECURITY-DEPENDENCIES.md` : 7 jours pour un avis élevé ou critique.
- **Trois variables lues par l'app** : `ESCOUADE_TERM_TRACE` (les temps des terminaux au journal) n'est lue que par un build de développement ; `ESCOUADE_ORT_DYLIB` (une autre bibliothèque ONNX Runtime) que par un build qui a la fonction `ort-dynamic`, que la release n'a pas ; `ESCOUADE_JAVY` reste lue partout, l'image du store nommant ainsi sa Javy (voir *Les programmes JS/TS des plugins*).
