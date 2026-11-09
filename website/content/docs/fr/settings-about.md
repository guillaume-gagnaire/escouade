# Réglages : À propos

La page « À propos » montre la version d’Escouade, les harness détectés, les crédits et les liens. Depuis la 2.2, la recherche des mises à jour, leur installation automatique et les statistiques d’usage sont dans la page « Général », et les données locales dans « Avancé » : cette page les décrit à leur nouvelle place.

## Ouvrir la page

Ouvre les réglages avec `Ctrl+,` (`⌘,` sous macOS), puis choisis « À propos », en bas de la barre. La version y est écrite après le mot « Escouade », quelle que soit la page ouverte. Ces réglages sont ceux de l’app, communs à tous les projets, et s’appliquent au geste.

## Escouade

Dans la page « Général », le groupe « Mises à jour », avec neuf lignes, plus celle d’une mise à jour trouvée, sous « Installer les mises à jour automatiquement », tant qu’elle n’est pas installée (voir [« Rechercher une mise à jour »](#rechercher-une-mise-a-jour)).

| Ligne | Ce qu’elle montre ou fait | Par défaut |
|---|---|---|
| « Escouade » suivi de la version | La version installée, suivie de « (bêta) » pour une bêta. Dessous, les harness qu’Escouade trouve sur ta machine (« Harness détectés : Claude Code, Codex »), ou « Aucun harness trouvé ». À droite, le bouton « Rechercher une mise à jour ». | — |
| « Installer les mises à jour automatiquement » | « Escouade redémarre d’elle-même quand aucun agent ne travaille et que tout est enregistré. » | Activé |
| « Recevoir les versions bêta » | « Les bêtas arrivent avant tout le monde et peuvent avoir des défauts. Décochée, Escouade reste sur sa bêta jusqu’à la version stable qui la suit. » S’enregistre aussitôt ; cochée, lance une recherche (voir [« Versions bêta »](updates#versions-beta)). | Désactivé |
| « Statistiques d’usage » | « Les fonctionnalités utilisées, la version d’Escouade et le système, sous un identifiant tiré au hasard : rien qui t’identifie, ni tes projets ni ton code. » À droite, le bouton « En détail » (il ouvre la page [Confidentialité](/privacy/)) et un interrupteur. | Désactivé tant que tu n’as pas accepté la question du premier démarrage |
| « Données locales » | Le dossier où Escouade garde ses données : `~/.escouade/`. | — |
| « Crédits et licences » | Les composants et les voix qu’Escouade embarque, avec leur licence. Le bouton « Ouvrir » affiche la liste dans ton navigateur. | — |
| « Site web » | « escouade.dev : la documentation, les nouveautés et la feuille de route ». Le bouton « Ouvrir » ouvre le site dans ton navigateur, dans la langue de l’interface. | — |
| « Discord » | « Poser une question, signaler un problème, suivre ce qui arrive ». Le bouton « Rejoindre » ouvre l’invitation au serveur Discord d’Escouade. | — |
| « Soutenir Escouade » | « Escouade est gratuite et open source : un café aide à la faire avancer ». Le bouton « Offrir un café » ouvre dans ton navigateur la [page de paiement](support:), au montant de ton choix (voir [« Soutenir Escouade »](introduction#soutenir-escouade)). | — |

Si un harness installé n’est pas trouvé, indique son chemin dans la page [« Comptes et harness »](settings-accounts).

## Rechercher une mise à jour

Escouade cherche seule : une première fois quelques secondes après son démarrage, puis cinq minutes après la fin de chaque recherche. Le bouton « Rechercher une mise à jour » (il dit « Recherche… » pendant l’appel) lance une recherche tout de suite. Sans version plus récente, un message dit « Aucune mise à jour disponible. » ; si la recherche échoue, un message d’erreur en donne la raison. Seule une recherche lancée par ce bouton le montre tout de suite : une recherche automatique attend plusieurs échecs de suite (voir [« Quand ça échoue »](updates#quand-ca-echoue)).

Une version trouvée, par le bouton comme par la recherche automatique, est téléchargée aussitôt et sa signature est vérifiée, puis elle attend. La page « Général » la montre sous « Installer les mises à jour automatiquement » : « Escouade 1.8.0 disponible » avec « Téléchargement… », puis « Escouade 1.8.0 prête » avec le bouton « Redémarrer maintenant » et le lien « Voir les nouveautés », qui déplie ses notes. Pendant le compte à rebours d’un redémarrage automatique, la ligne le montre aussi (« Redémarrage dans 20 s ») avec un bouton « Plus tard ». Si le téléchargement échoue, la ligne « Mise à jour 1.8.0 » la remplace et en donne la raison (« Téléchargement impossible : … »), avec, quand c’est possible, « Télécharger dans le navigateur » ; une version plus récente trouvée ensuite reprend la place.

Quand elle est prête, la barre d’état affiche aussi un bouton, par exemple « Mise à jour 1.8.0 prête · Redémarrer ». Lui comme « Redémarrer maintenant » de la page ouvrent une fenêtre, par exemple « Escouade 1.8.0 est prête », qui montre les notes de la version. « Redémarrer maintenant » installe la mise à jour. Il est grisé tant qu’un fichier de l’éditeur n’est pas enregistré (la fenêtre te le dit), et la fenêtre prévient quand des agents travaillent, car le redémarrage interrompt leur tour. « Plus tard » ferme la fenêtre sans rien installer (ouverte depuis « Général », elle y ramène).

La recherche et le téléchargement passent par le proxy de [« Avancé › Réseau »](settings-network). Le détail du parcours est dans [les mises à jour](updates).

## Installer les mises à jour automatiquement

Activé par défaut. Une mise à jour téléchargée redémarre alors Escouade toute seule, dès que l’app est au repos. Désactivé, la mise à jour reste prête : tu la lances avec « Redémarrer » dans la barre d’état, ou elle s’installe quand tu fermes l’app (qui reste alors fermée).

Escouade est au repos quand rien de ceci n’est vrai :

- un agent est en plein tour, ou attend ta réponse ou une autorisation ;
- un worktree est en cours de préparation ;
- un ticket est en cours de validation, ou un autre travail est en cours (démarrage d’un ticket, nettoyage…) ;
- une commande de lancement ou une étape de lancement de test tourne, ou un lancement de test se prépare ;
- un terminal intégré a reçu ou produit du texte depuis moins de 5 minutes ;
- un fichier de l’éditeur n’est pas enregistré ;
- une fenêtre est ouverte par-dessus l’app ;
- tu as utilisé le clavier ou la souris dans la fenêtre depuis moins de 5 minutes (une fenêtre cachée ou réduite ne compte pas).

Une fois au repos, Escouade te prévient 30 secondes avant : une notification système (pas sous Linux), même si les notifications système sont désactivées dans la page [« Notifications »](settings-notifications), dit « Escouade redémarre pour se mettre à jour » avec un bouton « Plus tard », et la barre d’état compte à rebours (« Redémarrage dans 20 s »). « Plus tard » repousse le redémarrage de 30 minutes. Il est dans la barre d’état, à côté du compte à rebours, et dans la notification sous Windows seulement (la notification de macOS n’a pas de bouton). Si, pendant ces 30 secondes, tu reprends la main ou qu’un agent se remet au travail, le redémarrage est annulé.

Si une version n’a pas pu s’installer, Escouade ne redémarre plus seule pour elle : tu peux réessayer à la main.

> [!NOTE]
> Sous macOS, le redémarrage automatique n’a lieu que si Escouade peut remplacer son application sans mot de passe d’administrateur. Sinon, redémarre-la toi-même quand la mise à jour est prête.

## Statistiques d’usage

Escouade peut envoyer des statistiques d’usage, sans rien qui t’identifie : les fonctionnalités utilisées, la version d’Escouade et le système, sous un identifiant tiré au hasard. Elles vont à un serveur d’Escouade hébergé en France (Aptabase, open source), et à aucun tiers.

Elles sont désactivées tant que tu n’as pas accepté. Au premier démarrage, y compris sur une installation qui existait déjà avant cette fonctionnalité, une fenêtre « Aider à améliorer Escouade ? » te le demande : « Partager les statistiques » ou « Non merci ». Rien n’est envoyé avant ta réponse. Si tu fermes la fenêtre sans répondre (avec `Échap`), elle revient au démarrage suivant.

Le bouton « En détail » de la ligne, comme le lien « Ce qui est envoyé, en détail » de la fenêtre, ouvre la page [Confidentialité](/privacy/) : les événements envoyés et leurs informations, ce qui n’est jamais envoyé, la durée de conservation (25 mois), la base légale et tes droits.

Quand tu désactives l’interrupteur, l’identifiant de l’installation est oublié, son fichier `telemetry-id` est supprimé et les événements qui attendaient encore sont abandonnés. Si tu le réactives, un nouvel identifiant est tiré : l’installation compte comme une nouvelle. Une instance lancée avec un dossier de données à part (voir [les données locales](data)) n’envoie jamais rien.

## Données locales

Dans la page « Avancé », le groupe « Données locales » indique `~/.escouade/`, dans ton dossier personnel (`C:\Users\<toi>\.escouade` sous Windows). La page ne fait qu’en indiquer l’emplacement : il n’a pas de bouton pour l’ouvrir, ouvre-le avec ton gestionnaire de fichiers. Ce qu’il contient est décrit dans [les données locales](data).
