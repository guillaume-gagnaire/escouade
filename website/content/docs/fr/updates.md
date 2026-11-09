# Mises à jour

Escouade se met à jour seule depuis les releases GitHub : elle télécharge en arrière-plan, puis installe au redémarrage que tu choisis ou, si tu le permets, d’elle-même quand elle est au repos. Cette page dit comment, selon le système, que faire si ça échoue, et où lire les nouveautés.

## Comment ça marche

- Escouade cherche une nouvelle version peu après son démarrage, puis cinq minutes après la fin de chaque vérification. Elle lit le fichier `latest.json` de la dernière release publiée sur [GitHub](https://github.com/guillaume-gagnaire/escouade/releases). Une release n’est publiée qu’une fois les fichiers des trois systèmes déposés et vérifiés : Escouade ne tombe jamais sur une release à moitié construite.
- Quand une version plus récente existe, elle la télécharge aussitôt, sans rien te demander, et vérifie sa signature. Pendant le téléchargement, la barre d’état affiche « Mise à jour 1.8.0… » (avec le numéro de la version).
- Une fois téléchargée, la mise à jour attend : la barre d’état affiche « Mise à jour 1.8.0 prête · Redémarrer ». Elle s’installe quand tu redémarres l’app, quand l’app redémarre d’elle-même, ou quand tu la quittes.
- L’installation se fait sans fenêtre d’installeur. Le proxy et la vérification des certificats réglés dans Réglages › Avancé › Réseau valent aussi pour les vérifications et les téléchargements. Quand quelque chose échoue, voir [« Quand ça échoue »](#quand-ca-echoue).

## Versions bêta

Une bêta est une version en avance, publiée pour que des volontaires l’essaient avant tout le monde : elle porte le numéro de la version qu’elle prépare suivi de `-beta` et d’un numéro, par exemple « 2.2.0-beta.1 ». Elle peut avoir des défauts que la version stable n’aura plus.

L’option « Recevoir les versions bêta » (Réglages › Général, décochée par défaut) décide si tu les reçois :

- **Décochée**, Escouade ignore toute bêta et ne lit que `latest.json`, qui n’en contient jamais. Si tu es déjà sur une bêta, tu y restes jusqu’à la version stable qui la suit (2.2.0 après 2.2.0-beta.3) ; une version plus ancienne, comme une 2.1.5, ne la remplace pas.
- **Cochée**, Escouade lit d’abord `beta.json`, dans la dernière release stable, qui donne la version la plus récente, bêta ou stable ; s’il manque, elle lit `latest.json`. La cocher lance aussitôt une recherche, et le réglage s’enregistre tout de suite, sans « Enregistrer ».

Une bêta se reconnaît partout où Escouade dit une version : « Escouade 2.2.0-beta.1 (bêta) » dans Réglages › Général et en bas de la barre des réglages, « Mise à jour 2.2.0-beta.1 (bêta)… » dans la barre d’état. Ses notes sont dans la fenêtre de la mise à jour comme pour une autre version ; elle n’est pas annoncée sur Discord.

## Installer une mise à jour

### Redémarrer toi-même

Un clic sur « Mise à jour 1.8.0 prête · Redémarrer » ouvre la fenêtre « Escouade 1.8.0 est prête », qui montre les notes de la version. Deux boutons : « Plus tard » et « Redémarrer maintenant ».

La fenêtre te prévient de ce qu’un redémarrage coupe :

- Des fichiers de l’éditeur ne sont pas enregistrés : « Enregistre d’abord tes fichiers ». Le bouton « Redémarrer maintenant » reste grisé tant qu’il en reste.
- Des agents travaillent ou attendent ta réponse : leur tour en cours est interrompu. Un agent de ticket reprend de lui-même ; les autres attendent ton prochain message.

Escouade s’arrête proprement avant d’installer : les processus des agents et les terminaux s’arrêtent, la session de chaque agent et l’état de l’app sont enregistrés. Après le redémarrage, la fenêtre revient comme elle était (devant, derrière une autre, réduite ou fermée dans la zone de notification), et un message dans la fenêtre dit « Escouade 1.8.0 est installée. », avec un bouton « Voir les nouveautés ».

### Redémarrer automatiquement

Le réglage « Installer les mises à jour automatiquement » (Réglages › Général, activé par défaut) laisse Escouade redémarrer d’elle-même : « Escouade redémarre d’elle-même quand aucun agent ne travaille et que tout est enregistré. »

L’app est au repos quand rien de ceci n’est vrai :

- Un agent travaille ou attend ta réponse (une question ou une autorisation).
- Un agent a en arrière-plan un sous-agent, un workflow ou une tâche de fond (une commande, une surveillance…) qui tourne : un redémarrage les arrêterait avec son processus.
- Un worktree est en préparation.
- Un ticket est en cours de validation (merge, pull request, push), ou un autre travail est en cours hors d’un tour d’agent, par exemple un ticket qui démarre ou une fusion demandée à la main.
- Une commande de lancement ou une étape d’un lancement de test tourne, ou un lancement de test est en préparation.
- Un terminal intégré a reçu ou affiché quelque chose depuis moins de cinq minutes. Un terminal resté silencieux plus longtemps ne compte pas.
- L’éditeur contient des fichiers non enregistrés.
- Une fenêtre (réglages, commit…) est ouverte.
- Tu as touché le clavier ou la souris dans la fenêtre depuis moins de cinq minutes. Ce point ne compte que si la fenêtre est à l’écran : réduite ou fermée dans la zone de notification, elle ne te retient plus.

Quand l’app est au repos, Escouade te prévient trente secondes avant de redémarrer : la barre d’état affiche « Redémarrage dans 30 s » avec un bouton « Plus tard », et une notification du système dit « Escouade redémarre pour se mettre à jour », quel que soit le réglage « Notifications système » (sous Windows, avec le même bouton « Plus tard » ; sous Linux, seule la barre d’état te prévient). Si l’app cesse d’être au repos pendant ce compte à rebours (un agent se remet au travail, tu touches le clavier), le redémarrage est annulé et attend le prochain repos. « Plus tard » le repousse de trente minutes.

### Quitter l’app

Fermer la fenêtre ne quitte pas Escouade : elle reste dans la zone de notification. Si tu la quittes (« Quitter » dans le menu de son icône, ou `⌘Q` sous macOS) alors qu’une mise à jour est prête, la mise à jour s’installe et l’app reste fermée.

### Si l’installation échoue

L’app reste sur l’ancienne version et le dit : « La mise à jour vers 1.8.0 n’a pas pu s’installer. » avec un bouton « Réessayer », qui rouvre la fenêtre de la mise à jour. Elle ne redémarre plus d’elle-même pour cette version ; elle retente à ta demande, ou quand tu quittes l’app.

## Chercher à la main

Réglages › Général montre la version installée (« Escouade 1.7.5 », par exemple) et un bouton « Rechercher une mise à jour ». La réponse est « Aucune mise à jour disponible. » (seulement quand la vérification a vraiment répondu), ou la mise à jour apparaît dans l’onglet, « Escouade 1.8.0 disponible » puis « Escouade 1.8.0 prête » avec « Redémarrer maintenant », comme dans la barre d’état. Quand tu cherches toi-même, une panne se dit toujours, avec sa raison : « Vérification des mises à jour impossible » ou « Téléchargement de la mise à jour impossible », suivis de la cause. Une recherche automatique est plus patiente : voir [« Quand ça échoue »](#quand-ca-echoue). Derrière un proxy ou un certificat d’entreprise, voir [« Derrière un proxy d’entreprise »](#derriere-un-proxy-d-entreprise) et le [dépannage](troubleshooting).

## Quand ça échoue

Escouade ne te dérange pas pour un accroc passager.

- **Elle réessaie seule.** Un échec passager (réseau coupé, réponse 403, 404, 408, 425, 429 ou 5xx du serveur, réponse illisible) est réessayé une fois, trois secondes plus tard. Un téléchargement qui échoue relit d’abord `latest.json` et, si la même version y figure toujours, réessaie une fois avec ce que le fichier indique : le lien d’un fichier a pu expirer.
- **Elle se tait d’abord.** Pour une recherche ou un téléchargement automatique, l’erreur n’apparaît qu’après au moins trois échecs de suite, étalés sur au moins une heure, et une seule fois par série d’échecs. Une série dont le dernier échec date de plus de trente minutes repart de zéro : un échec avant une mise en veille et deux au réveil ne font pas une alerte. La recherche et le téléchargement ont chacun leur série, et un succès clôt la sienne : un proxy qui refuse l’installateur toutes les cinq minutes finit donc par se dire, même quand la recherche passe.
- **Elle le dit tout de suite quand tu cherches toi-même.** « Rechercher une mise à jour » montre toujours l’échec, avec sa raison.

L’erreur dit ce que la requête a rencontré :

- le code HTTP, par exemple « 403 Forbidden » ;
- le domaine qui a répondu, une fois les redirections suivies (`github.com`, `release-assets.githubusercontent.com`…) ;
- le proxy par lequel la requête est passée, s’il y en a un, par exemple `127.0.0.1:9000` ;
- ce que la réponse dit d’elle-même : l’en-tête `Proxy-Agent`, ou un en-tête `Server` ou une page de blocage qui nomme un proxy connu (Zscaler, Netskope, Squid, Blue Coat, Forcepoint, Fortinet, Palo Alto, Umbrella, Sophos, Skyhigh…), dont elle cite le début. Un simple en-tête `Via` ne suffit pas à nommer un proxy : certains l’ajoutent à toutes leurs réponses, même au refus d’un site.

Pour un 403, elle ajoute « bloqué par le réseau ou un proxy ? ». Par exemple : « 403 Forbidden de release-assets.githubusercontent.com, par le proxy 127.0.0.1:9000 (Zscaler/6.2) : bloqué par le réseau ou un proxy ? ». Pour un 407, elle dit que le proxy demande une authentification, et qu’il faut mettre l’identifiant et le mot de passe dans l’adresse du proxy, dans Réglages › Avancé › Réseau.

Escouade s’annonce avec un User-Agent honnête, `Escouade/<version> (<système>; <architecture>)` pour la recherche comme pour le téléchargement. Elle ne se fait jamais passer pour un navigateur.

## Derrière un proxy d’entreprise

Un proxy d’entreprise (Zscaler, Netskope, Squid…) laisse souvent passer la recherche, un petit fichier, mais refuse l’installateur à tout client qui n’est pas un navigateur. Escouade ne contourne pas cette règle : elle le dit, et te laisse une issue.

### Ce que dit le message

Quand un proxy d’entreprise qui se fait connaître refuse l’installateur, le message le nomme : « le proxy de ton entreprise (Zscaler/6.2) a refusé l’installateur (403 Forbidden de release-assets.githubusercontent.com, par le proxy 127.0.0.1:9000). Télécharge-le dans le navigateur : il faudra peut-être confirmer l’avertissement du proxy. »

Quand le proxy refuse la requête (403, 407, tunnel refusé), Escouade ne la refait pas sans lui : elle ne contourne aucune règle de ton entreprise, et le message explique le refus. Pour un 407, il dit que le proxy demande une authentification : mets l’identifiant et le mot de passe dans l’adresse du proxy, dans Réglages › Avancé › Réseau (par exemple `http://utilisateur:motdepasse@proxy:3128`).

Seul un proxy injoignable (réglé mais arrêté, ou poste hors du réseau de l’entreprise) entraîne un second essai, sans proxy. [Le journal de l’app](troubleshooting#ou-est-le-journal-de-l-app) dit alors si l’accès direct a marché, et si les deux essais échouent, le message d’échec les dit.

### Télécharger dans le navigateur

Un téléchargement raté propose le bouton « Télécharger dans le navigateur », dans un message qui reste affiché trente secondes. Il ouvre la page de la release sur GitHub, où tu prends le fichier de ton système : l’installeur `.exe` sous Windows, le `.dmg` sous macOS, l’`AppImage` ou le `.deb` sous Linux. Le navigateur est autorisé là où Escouade ne l’est pas, mais le proxy peut te demander de confirmer un avertissement avant de laisser passer le fichier. Installe-le comme à la première installation (voir [l’installation](installation)) ; tes réglages, tes projets et tes conversations restent où ils sont.

### Quoi demander au service informatique

Demande d’autoriser :

- les domaines `github.com` (la recherche et la page de la release) et `release-assets.githubusercontent.com` (où GitHub sert les fichiers des releases) ;
- le téléchargement des fichiers de la release pour le client dont le User-Agent commence par `Escouade/` : le `.exe` (Windows), le `.app.tar.gz` (macOS, pour la mise à jour ; le `.dmg` sert à l’installation), l’`AppImage` et le `.deb` (Linux).

### Le réglage de proxy

Si ton entreprise te donne l’adresse de son proxy, renseigne-la dans Réglages › Avancé › Réseau : la recherche et le téléchargement passent alors par lui. Si le proxy déchiffre le trafic avec un certificat de l’entreprise, installe ce certificat sur le système. Voir [les réglages du réseau](settings-network).

## Selon le système

### Windows

L’installeur tourne sans fenêtre et Escouade redémarre sur la nouvelle version.

### macOS

L’app est signée avec un certificat Apple Developer ID et notariée par Apple (depuis la 1.5.4) : macOS l’ouvre sans demander d’autorisation. La mise à jour remplace l’app en place.

Le redémarrage automatique, et l’installation à la fermeture de l’app, demandent que ton utilisateur puisse modifier Escouade.app et le dossier qui la contient. Sinon, la mise à jour attend ton clic sur « Redémarrer maintenant », et macOS peut demander un mot de passe d’administrateur.

### Linux

La mise à jour intégrée remplace l’`AppImage` que tu as lancée. Le `.deb` (Debian, Ubuntu) s’installe d’abord avec `sudo apt install ./Escouade_*_amd64.deb` ; ensuite, la mise à jour intégrée télécharge le nouveau `.deb` de la release et l’installe avec `dpkg` : le système te demande alors ton mot de passe d’administrateur (par `pkexec`, à défaut par `sudo`).

## De Claude Code Manager à Escouade

Escouade s’appelait « Claude Code Manager ». La mise à jour qui l’a renommée remplace l’ancienne installation et garde tes réglages, tes projets et tes conversations. Sous Windows, l’installeur retire d’abord l’ancienne installation en gardant tes données, puis crée les raccourcis d’Escouade. Le dossier de données change de nom : voir les [données locales](data).

## Lire ce qui a changé

- **Dans l’app.** La fenêtre d’une mise à jour prête montre ses notes. Après l’installation, « Voir les nouveautés » ouvre « Nouveautés d’Escouade 1.8.0 ».
- **Sur GitHub.** Le [journal des versions](https://github.com/guillaume-gagnaire/escouade/blob/main/CHANGELOG.md) liste, version par version, ce qui change. Les notes de chaque release en reprennent la section de la version.
