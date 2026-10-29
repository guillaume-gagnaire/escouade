---
title: "Livraison et mises à jour"
subject: "Introduction"
sources: ["docs/SPEC.md",".github/workflows/release.yml",".github/workflows/discord.yml"]
tickets: []
decisions: []
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
- La vérification et le téléchargement envoient `User-Agent: Escouade/<version> (<système>; <architecture>)` (Windows, macOS ou Linux), jamais celui d'un navigateur. Escouade ne déguise pas son trafic pour passer une règle de sécurité d'entreprise : elle s'annonce pour qu'on puisse l'autoriser.

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
- Un build raté laisse le brouillon, et « latest » reste la précédente ; relancer les jobs ratés est sans danger.
- **Notes de version**, tirées des deux changelogs, qui ont chacun une section par version :
  - celles de la release GitHub : la section de la version dans `CHANGELOG.en.md`, en anglais ;
  - celles de `latest.json`, que montre la modale « Escouade X est prête » : sa section de `CHANGELOG.md`, en français.
- **Discord** : une fois la release publiée, le job `announce` poste ses notes sur les deux salons du Discord d'Escouade, chacun à part (un salon qui refuse n'empêche pas l'autre) : le salon français (section de `CHANGELOG.md`, webhook du secret `DISCORD_WEBHOOK_URL`) et le salon anglais (section de `CHANGELOG.en.md`, `DISCORD_WEBHOOK_URL_EN`). Rien pour une pré-version ni pour un build raté. `discord.yml` se lance aussi à la main pour une version déjà publiée (`gh workflow run discord.yml -f tag=vX.Y.Z`, `-f lang=fr` ou `en` pour un seul salon) ; un salon refusé se reposte seul en relançant le job raté.
- Les commits sont atomiques, sur `main`, sans push.

## Réglages

- « Installer les mises à jour automatiquement » : activé par défaut.
- « Rechercher une mise à jour » : vérifie à la demande.
- Le proxy des mises à jour est celui de Réglages › Réseau (voir *Réglages*).

## Limites

- Pas de remplacement à chaud : chaque mise à jour passe par un redémarrage.
- Jamais de redémarrage automatique dans une exécution isolée (`ESCOUADE_DATA_DIR`).
- Sous macOS, quand remplacer l'app demanderait le mot de passe d'un administrateur, l'app ne redémarre jamais d'elle-même.
- Une fin de session Windows n'installe rien.
- Les notes montrées par la modale « Escouade X est prête » sont en français, quelle que soit la langue de l'interface.
