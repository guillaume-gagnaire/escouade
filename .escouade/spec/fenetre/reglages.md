---
title: "Réglages"
subject: "Fenêtre"
sources: ["docs/SPEC.md","design/settings-2-2/PROPOSITION.md","design/settings-2-2/CONSTRUCTION.md","design/settings-2-2/index.html","design/settings-2-2/captures/a-plugins.png","design/settings-2-2/captures/a-projet.png","design/settings-2-2/captures/a-comptes.png"]
tickets: ["ESC-378","ESC-379","ESC-380","ESC-381","ESC-382","ESC-383","ESC-384","ESC-385","ESC-386"]
decisions: [118,119,120,121,123,124,126]
compared: null
---

## Ce que c'est

Une seule fenêtre de réglages pour toute l'app : une barre des catégories à gauche, avec la recherche en tête, une page à la fois à droite. Les pages d'Escouade règlent l'app ; celles d'un projet ne règlent que lui. Chaque réglage s'applique et s'enregistre au geste, comme dans les réglages de macOS et de VS Code : ni « Annuler », ni « Enregistrer ». Une page qui a un problème le dit par un point ambre dans la barre. Chaque réglage est décrit sur la page de sa feature ; cette page dit comment la fenêtre marche et ce que chaque page contient.

## Comportement

### Ouvrir

- « Réglages » en bas de la barre des projets, ou `Ctrl+,` : sur « Comptes et harness ».
- Ouvertures directes : chaque renvoi de l'app vers un projet mène à sa page, « Projets › demo-api › Lancement », et non au milieu d'un défilement :
  - « Réglages du projet… » au clic droit sur un projet, et « Harness par défaut du projet… » du menu « Autres façons de créer un agent » : la page Projet de ce projet ;
  - ⚙ (« Commandes de lancement… »), « Configurer » et « ✦ Proposer des commandes » de la section Lancement : la page Lancement du projet ; le dernier lance aussi la lecture du projet (voir *Barre latérale*) ;
  - « ⚙ Après validation » du Kanban : la page Kanban de son projet ;
  - « Les lire » du bandeau du Kanban (commandes de `.escouade.yml` en attente) : la page Équipe de son projet, sur « Réglages partagés » ;
  - le panneau Workflow (« Réglages du workflow », « Décrire un workflow… », « Ouvrir les réglages du workflow ») : la page Kanban de son projet, sur « Workflow » ;
  - la fenêtre d'un compte ouverte par « Ajouter un compte réservé… » : en se fermant, la page Confidentialité du projet, son brouillon tel qu'il était ;
  - « Tous les projets fermés… » du menu `+` de la barre des projets : la page Tous les projets ;
  - « Lier une source » et « ⚙ Gérer les sources » de la fenêtre d'import : la page Tickets importés de son projet (voir *Intégrations*) ;
  - « Réglages › Services connectés » de l'onglet « PR et CI » et de la vue Pull requests (jeton refusé, hébergeur sans compte) : la page Services connectés (voir *Pull requests*) ;
  - « Redémarrer maintenant » d'une mise à jour prête, dans Général : la fenêtre de la mise à jour, dont « Plus tard » revient sur Général.
- Chaque page, et chaque projet, s'affiche depuis son début ; un renvoi qui vise un groupe (Workflow, Réglages partagés) l'amène en vue, sur la page ouverte seulement.
- Les textes qui nomment une page la nomment par son chemin, une page d'un projet après « Projets », un groupe après sa page : « Réglages › Général › Couleurs » (thèmes d'un plugin), « Réglages › Général » (ton nom, statistiques d'usage), « Réglages › Agents » (remote control pour tous les agents), « Réglages › Agents › Textes rédigés » (un harness qui ne sait pas répondre sans outil), « Réglages › Avancé › Réseau » (un proxy qui demande une authentification), « Settings › Advanced › Attachments » (la taille des pièces jointes, dans les descriptions des outils MCP), « Réglages › Services connectés » (un jeton manquant, dans le kit des plugins), « Réglages › Projets › Équipe » (commandes de `.escouade.yml` en attente, nom dans un projet), « Réglages › Projets › Projet » (plugins proposés par un projet), « Réglages › Projets › Kanban » (« Pas de workflow pour ce projet »), « Réglages › Projets › Confidentialité » (compte réservé), « Réglages › Tous les projets » (historique d'un projet fermé). Un test refuse un chemin qui ne nomme pas une page, en français et en anglais : dans les catalogues de l'interface, dans les sources et les commentaires de l'app, dans les messages et les commentaires du cœur et dans ceux du kit des plugins.

### La fenêtre

- 1120 × 780, ou la place disponible avec 16 px autour (à 1028 × 779, la taille de la CI).
- **La barre**, de haut en bas :
  - « Rechercher un réglage » (`Ctrl+F`) : la recherche (ci-dessous) ;
  - « Escouade » : Général, Agents, Comptes et harness, Services connectés, Voix, Notifications, Plugins, Avancé ;
  - « Projets » : chaque projet, sa couleur devant son nom, un cadenas pour un projet privé. Le projet choisi, quand une de ses pages est à l'écran, est déplié sur ses pages : Projet, Git et worktrees, Lancement, Kanban, Tickets importés, Équipe, Confidentialité. Choisir un autre projet garde la même page, sinon ouvre « Projet » ;
  - « Tous les projets » (ce que reçoit un nouveau projet, les projets fermés) ;
  - en bas : « À propos » et la version d'Escouade.
- **Le clavier** : ↑ et ↓ vont d'une entrée de la barre à la suivante et l'ouvrent, Origine et Fin à la première et à la dernière ; passer sur un projet l'ouvre et le déplie. Une seule entrée de la barre est dans l'ordre de Tab : celle où est le clavier, qui a le focus à l'ouverture (la recherche le prend par `Ctrl+F` ou Tab arrière).
- **La teinte suit la portée** : sur une page d'Escouade (Tous les projets compris), la fenêtre prend le teal d'Escouade ; sur une page d'un projet, la couleur de ce projet, dès qu'elle change. À la fermeture, l'app reprend sa propre teinte.
- La page sous son titre et sa description ; pour une page d'un projet, sa tuile devant, et son nom pour titre sur la page Projet.
- **« Avancé »**, en bas d'une page : un groupe replié sur une ligne qui nomme ce qu'il cache ; un clic le déplie sur place, la recherche aussi. Général, Agents, Comptes et harness et Projet en ont un (celui de Projet cache « Fermer le projet » : l'ancienne « Zone sensible », une seule action qu'aucun titre alarmant n'annonce ; celui de Comptes et harness se déplie seul quand Claude Code est introuvable, son chemin s'y réglant).
- Pas de pied : ni « Annuler » ni « Enregistrer ». Échap, × ou un clic à côté de la fenêtre la ferment, sans question.

### La recherche

- Le champ en tête de la barre ; `Ctrl+F` (⌘F sous macOS) y mène de partout dans la fenêtre.
- Il cherche dans le libellé et la description de chaque réglage du registre de la fenêtre, d'Escouade et de chaque projet, et dans le nom et la description de chaque page. En français et en anglais, quelle que soit la langue de l'interface, sans tenir compte de la casse ni des accents : « BETA » trouve « Recevoir les versions bêta », « proxy » « Proxy HTTP(S) », « NO_PROXY » « Exclusions ».
- Tant que le champ a du texte, ses résultats remplacent les entrées de la barre : le libellé, et sous lui où il est (« Avancé », « Général › Avancé » pour une rangée repliée dans Avancé, « demo-api › Git et worktrees »). Un libellé qui commence par ce qui est tapé passe d'abord, puis un libellé qui le contient, puis un libellé qui en a tous les mots, puis une description qui les a ; à égalité, l'ordre de la barre. Un réglage d'un projet vient une fois par projet, le projet choisi d'abord. Sans résultat : « Aucun réglage ne répond à « zzqx » ».
- ↑ et ↓, dans le champ, vont d'un résultat à l'autre ; Entrée ou un clic l'ouvre : sa page et son projet, « Avancé » déplié si la rangée y est, la rangée amenée au milieu de la page et surlignée en ambre un instant (sans fondu avec « réduire les animations »). Le focus reste dans le champ et ses résultats restent là, pour en ouvrir un autre.
- Échap vide le champ et rend les entrées de la barre ; un second Échap ferme la fenêtre.

### Les points ambre

- Un point ambre au bout d'une entrée de la barre dit une page qui a un problème. Sa raison est l'infobulle de l'entrée (et sa description pour un lecteur d'écran), une ligne par problème :
  - **Comptes et harness** : un harness installé et trouvé qui n'a aucun compte, « Codex n'a aucun compte : aucun agent ni ticket n'y démarre » ;
  - **Services connectés** : un jeton d'hébergeur expiré ou révoqué à sa dernière vérification, « Jeton expiré ou révoqué · @ada · github.com », ou un jeton illisible dans le trousseau du système, « Jeton illisible dans le trousseau · … » ;
  - **Plugins** : un plugin désactivé par Escouade, au manifeste invalide, ou dont le process s'est arrêté seul (après ses plantages, faute de mémoire, refusé), « Trello : Arrêté après 3 plantages ». Un plugin éteint, sans programme ou arrêté exprès n'en a pas ;
  - **un projet** qu'aucun compte actif de son harness ne sert : sur son entrée et sur sa page Projet, où se choisit son compte, « Aucun compte Codex ne sert ce projet : ses agents et ses tickets ne démarrent pas ».
- Les jetons (tels que leur dernière vérification les a trouvés) et les process des plugins sont relus à l'ouverture de la fenêtre ; la vérification que lance Services connectés met le point à jour ; les comptes et les projets suivent l'app.

### Enregistrer au geste

- **Un choix** (interrupteur, segments, menu, case, raccourci enregistré, commande ajoutée ou retirée, proposition de Claude prise, logo choisi) s'applique et s'enregistre aussitôt.
- **Un texte** (champ, nombre) s'enregistre quand il perd le focus, ou après 0,7 s sans frappe. Fermer la fenêtre enregistre ce qui attendait encore. Passer d'une page d'un projet à une autre garde son brouillon.
- Chaque enregistrement envoie ce qui a changé par-dessus ce que l'app a alors (le Kanban, lui, avance seul), un à la fois : ce qui change pendant qu'un enregistrement tourne part avec le suivant. Rien n'est envoyé quand rien n'a changé.
- **Une saisie invalide** se dit sur sa rangée, et la valeur enregistrée avant reste tant qu'elle n'est pas corrigée ; le reste s'enregistre :
  - un projet sans nom : « Un projet garde un nom : « demo-api » reste enregistré. » ;
  - une commande de lancement sans nom ou sans ligne de commande : les commandes enregistrées restent celles d'avant tant qu'il en manque ;
  - un dossier des worktrees ou un préfixe de branche refusé, avec la raison sous le champ (voir *Git & fichiers*) ;
  - un nombre vidé ou sous 0, par exemple « Un nombre entier, 0 ou plus : 15 reste enregistré. » ; un nombre à virgule est arrondi à l'entier du dessous ; une taille de texte hors de ses bornes est ramenée dedans.
- Ce qui coûte le dit sur sa rangée, comme avant : le proxy et les certificats ne valent pour un agent qu'au prochain démarrage de son process, le remote control lance aussitôt le process `claude` de chaque agent concerné.
- Certaines rangées passent par leur propre appel au backend, comme avant : l'apparence (voir *Fenêtre*), « Recevoir les versions bêta » (enregistré, puis une recherche de mise à jour), les comptes des harness et leur « Projection des quotas » (voir *Barre de statut*), les comptes des intégrations (voir *Intégrations*), l'interrupteur « Les agents peuvent piloter Escouade » (voir *Serveur MCP*), les modèles de la voix, téléchargés ou effacés sur leur ligne, sur la page Plugins l'interrupteur d'un plugin et « Installer » d'un plugin proposé, et sur la page Confidentialité l'interrupteur d'un plugin dans le projet.
- Rien ne se perd au redémarrage : ce qui est enregistré est dans `settings.json` et les fichiers des projets, dont aucune clé ne change.

### Les pages

Chaque réglage a sa page dans le registre de la fenêtre, et chaque page son contenu. Le registre déclare chaque rangée une fois, avec sa page, son libellé et si elle est dans Avancé : la recherche le lit, et un test vérifie qu'aucune clé des réglages n'est sans page.

- **Général** :
  - « Apparence » : « Thème », « Couleurs », « Zoom de l'interface », « Taille du texte des terminaux » (voir *Fenêtre*) ;
  - « Langue » : « Langue de l'interface » (« Système (…) » par défaut, « English », « Français ») et « Langue des textes rédigés par les agents » (« Comme l'interface » par défaut) (voir *Langues*) ;
  - « Mises à jour » : la version (« Escouade 2.2.0 ») et « Rechercher une mise à jour », sous elle « Aucune mise à jour disponible. » après une recherche sans rien ; la mise à jour trouvée, en téléchargement puis prête, avec son redémarrage et ses nouveautés, ou un téléchargement qui échoue ; « Installer les mises à jour automatiquement » ; « Recevoir les versions bêta » (voir *Livraison et mises à jour*) ;
  - « Toi » : « Ton nom » (voir *Kanban partagé*) et « Statistiques d'usage » ;
  - Avancé : « Taille du texte de l'éditeur », « Ton identifiant ».
- **Agents** :
  - « Nouveaux agents » : « Vue des nouveaux agents », « Les agents tiennent une liste de tâches », activé par défaut (voir *Avancée d'un plan*), « Délai avant l'envoi pendant un tour » ; les deux premiers seulement quand un harness installé les a ;
  - « Textes rédigés » : « Modèle des textes rédigés », et son modèle pour un harness qui le prend (voir *Langues*) ; absent sans harness installé ;
  - « Remote control » : « Remote control pour tous les agents », qui dit quels harness l'ont (voir *Barre latérale*) ; absent sans harness qui l'a ;
  - « Piloter Escouade » : « Les agents peuvent piloter Escouade » et ses déclarations (voir *Serveur MCP*) ;
  - Avancé : arrêt des process inactifs, « Activité MCP ».
- **Comptes et harness** (voir *Comptes*, « Réglages › Comptes et harness ») : un groupe par harness, son état et sa version en pastille (« Prêt · 2.1.301 », « Aucun compte », « Introuvable »), « Ajouter un compte <harness>… » et ses comptes ; un harness introuvable donne sa commande d'installation à la place, un harness non installé « Installer <harness> » depuis l'amorce ; pour OpenCode, ses « Modèles » favoris ; le coût estimé d'un harness qui ne le donne pas. Puis « Quand un quota est épuisé » : « Changer de compte à », « Pause du pilote auto au-delà du quota » (« pour tous les projets »), « Reprendre sur un autre compte un agent de ticket arrêté par la limite », « Reprise automatique après la limite d’usage », « Continuer sur le harness suivant… ». Avancé : « Ordre des harness », « Modèle de passation », le chemin de `claude` et où il est (voir *Claude Code dans WSL*), « Mettre à jour Claude Code automatiquement », « Projection des quotas » (voir *Barre de statut*).
- **Services connectés** : « Hébergeurs de code » (les instances et leurs jetons), puis « Sources de tickets » : leurs comptes (« servi par » un plugin), « Synchronisation » et « Import automatique », pour tous les projets (voir *Intégrations*, *Hébergeurs de code*). Elle ne demande aucun projet.
- **Voix** : « Micro utilisé », « Modèle », « Langue parlée », « Raccourci » (voir *Dictée*) ; « Conversation » (voir *Conversation vocale*).
- **Notifications** : « Me prévenir pour », « Pull requests et déploiements », notifications du système, son (voir *Notifications*).
- **Plugins** : les plugins installés, ceux que propose Escouade, et « N'installer que des versions vérifiées » (ci-dessous).
- **Avancé** :
  - « Réseau » : proxy, exclusions, proxy des terminaux, « Ignorer la vérification des certificats TLS » (ci-dessous) ;
  - « Terminaux » : PowerShell 7, Git Bash, distribution WSL (bash sous macOS), shells détectés (voir *Barre latérale*) ;
  - « Pièces jointes » (voir *Conversation*) ;
  - « Recherche par le sens » › « Modèle local » (voir *Recherche par le sens*) ;
  - « Données locales » : le dossier des données d'Escouade, et « Ouvrir le dossier ».
- **À propos** : le logo, « Escouade » et sa version, les harness détectés ; « Crédits et licences », « Site web », Discord, « Soutenir Escouade ».
- **Projet** (titrée du nom du projet) : la bande des plugins que propose son `.escouade.yml` (voir *Plugins proposés par un projet*) ; « Identité » : nom, dossier (« Ouvrir »), couleur, lettres, logo et libellé de la tuile, « Harness par défaut » (dès deux harness trouvés) et « Compte préféré » (dès deux comptes) ; puis « Assistant de configuration » ; Avancé : « Fermer le projet… ». Fermer ou non revient sur cette page (voir *Fermer un projet*).
- **Git et worktrees** : « Git » (un worktree par agent, au premier message, branche de départ, dossier des worktrees, préfixes des branches, fichiers copiés, commit, branche des docs, voir *Git & fichiers* et *Docs du projet*) ; « Worktrees » (préparation et démontage, « Remplir automatiquement », « Worktrees restés » › « Nettoyer ») ; « Dépôts » pour un projet de plusieurs dépôts (voir *Projet à plusieurs dépôts*).
- **Lancement** : « Ouvrir les pages des lancements », les commandes de lancement, « Ajouter une commande » et « Remplir automatiquement » (voir *Barre latérale*).
- **Kanban** : branche cible, action à la validation, tests, nettoyage, message de commit, conflit (voir *Validation*) ; pilote auto et ses agents (voir *Pilote auto*) ; puis « Workflow », le workflow du projet (voir *Workflows*). La pause au-delà du quota est dans Comptes et harness.
- **Tickets importés** : les sources liées au projet et la correspondance de leurs statuts ; sans compte connecté, un renvoi à Services connectés (voir *Intégrations*).
- **Équipe** : « Kanban partagé » (« Ton nom dans ce projet », les tickets partagés, « Partager chaque nouveau ticket », voir *Kanban partagé*) ; « Réglages partagés », `.escouade.yml` en sous-titre (son état, ses commandes en attente, « Exporter vers .escouade.yml », voir *Réglages partagés*) ; « Mémoire partagée » (voir *Mémoire partagée*).
- **Confidentialité** : « Projet privé », et pour un projet privé « Ses fichiers » harness par harness, ses comptes réservés ou partagés (« Réserver … à ce projet », « Ajouter un compte réservé… ») et « Comptes partagés à la limite » (voir *Projets privés*, *Comptes*) ; « Remote control » › « Remote control pour les agents de ce projet », quand un harness l'a (voir *Barre latérale*) ; « Plugins dans ce projet » (ci-dessous).
- **Tous les projets** : « Nouveaux projets » (« Nouveaux projets privés », éteint par défaut), puis « Projets fermés », seulement s'il y en a, avec par projet « Rouvrir » et « Effacer l'historique… », et « Historique illisible » avec « Effacer » (voir *Fermer un projet*). Effacer un historique revient sur cette page.

### Les plugins d'un projet

Dans Confidentialité, le pendant de l'onglet Projets de la page d'un plugin : « Plugins dans ce projet », sous « Un plugin s’active projet par projet. Un projet privé est coupé d’office : rien ne lui parvient tant que tu ne l’y actives pas. ».

- Une ligne par plugin installé qui s'active projet par projet (un thème, qui vaut pour toute l'app, n'y est pas) : ses deux lettres, son nom, et l'interrupteur « <plugin> dans ce projet », appliqué aussitôt.
- Un plugin coupé pour toute l'app est montré éteint, « désactivé pour toute l’app », sans interrupteur actif ; dans un projet privé, un plugin qui n'y a pas été activé dit « projet privé : coupé d’office, rien ne lui parvient ».
- Aucun : « Aucun plugin installé qui s’active projet par projet. »

### La page Plugins

Le tableau des plugins, côté réglages. La vue Plugins reste l'endroit où l'on installe depuis le store, un dossier ou un dépôt, retire une permission et lit un journal (voir *Les plugins : vue d'ensemble*).

- **Installés**, avec « Ouvrir la vue Plugins » à droite du titre (la vue, sur ses plugins installés). Une ligne par plugin installé, livrés avec l'app compris :
  - ses deux lettres, son nom et sa version ;
  - ce qu'il ajoute et où cela se règle, en deux points au plus, dans cet ordre : harness (« Un harness, dans « Nouvel agent » : Claude Code »), source du Kanban et hébergeur de code (« … dans Réglages › Services connectés : GitHub »), thème (« Un thème, dans Réglages › Général › Couleurs : … »), service de voix (un plugin qui s'active sur `voice:…` : « Un service de voix, dans Réglages › Voix »), puis environnements de lancement, webhooks, commandes et le reste. Ses types de comptes ne sont dits que s'il n'ajoute rien d'autre. Sans rien de tout cela : sa description ;
  - sur combien de projets il est actif (« Actif sur 2 projets sur 3 », un projet privé ne le comptant que s'il y a été activé), ou « Pour toute l'app » pour un thème ; rien sans projet ouvert ;
  - son état s'il ne peut pas tourner (désactivé par Escouade, manifeste invalide) ;
  - à droite : « Vérifié » (bouclier, teal) pour une version vérifiée par Escouade ; « Réglages » s'il a des réglages ou des secrets à saisir, qui ferme la fenêtre et ouvre sa page dans la vue Plugins, sur son onglet Réglages ; son interrupteur « Actif dans Escouade », appliqué aussitôt. C'est lui que nomment « Réglages › Plugins le rallume » (un hébergeur dont le plugin est coupé) et « Le plugin … est coupé (Réglages › Plugins) ».
  - Aucun plugin : « Aucun plugin installé. Un plugin ajoute un harness, une source du Kanban, un hébergeur de code, un thème ou un service de voix. »
- **Proposés par Escouade**, s'il en reste : les plugins de l'amorce qui ne sont pas installés (voir *Les plugins livrés avec Escouade*), sous la note « Dans l'installeur, rien à télécharger : chacun s'installe aussi tout seul au geste qui le demande (ajouter un compte, choisir une source, un hébergeur ou un service de voix). ». Par plugin, pourquoi il n'est pas là (« Dans l'app, sans téléchargement. Il s'installe quand tu en as besoin. », « Tu l'as désinstallé (v1.3.0). Escouade ne le remet que sur un geste qui le demande. », ou la raison d'un refus) et « Installer » : installé d'un clic depuis l'amorce, sans réseau ni fenêtre, dit par « Plugin Trello installé » ; il passe alors dans les installés. Une version plus récente du store se propose ensuite comme une mise à jour, dans la vue.
- **Sécurité** › « N'installer que des versions vérifiées », le réglage de la vue Plugins (`pluginsVerifiedOnly`), avec l'état de la liste de révocation (« Liste de révocation lue à 14:02 · nº 418, signée, relue toutes les 6 heures »).

### Nettoyer les worktrees restés

- « Worktrees restés » › « Nettoyer », dans la page Git et worktrees : les worktrees des agents archivés du projet encore sur le disque sont supprimés, avec la même prudence que ceux des tickets finis (voir *Validation*), puis git oublie ceux dont le dossier a disparu (`git worktree prune`).
- La rangée dit ensuite combien ont été supprimés (« Aucun worktree à supprimer. ») et ceux qui sont gardés, avec pourquoi.

### Réseau

- **Proxy** : une adresse HTTP(S), avec identifiants s'il en faut, et ses exclusions (`NO_PROXY`).
  - Il est donné aux process `claude` (`HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`), aux lectures des quotas, aux intégrations et à la recherche de mises à jour.
  - La recherche de mises à jour, si le proxy lui-même est injoignable, refait sa requête une fois sans lui ; si le proxy la refuse, elle ne le contourne pas (voir *Livraison et mises à jour*).
  - Une option l'exporte aussi dans les terminaux intégrés.
- **Certificats** : les requêtes de l'app (intégrations, quotas, mises à jour) font confiance aux autorités habituelles et à celles du magasin du système, dont le certificat d'un proxy d'entreprise qui y est installé. Claude Code garde sa propre liste.
- « Ignorer la vérification des certificats TLS » (désactivé par défaut) : pour un proxy qui déchiffre le trafic avec un certificat que le système ne reconnaît pas (« UnknownIssuer »). Plus aucun certificat n'est alors vérifié, ni pour les intégrations, les quotas et les mises à jour, ni pour les process `claude` (`NODE_TLS_REJECT_UNAUTHORIZED=0`, au prochain démarrage de leur process, hérité par les commandes que lancent les agents). Les terminaux et les commandes des worktrees ne sont pas concernés. Les mises à jour suivent les réglages réseau du moment.

## Limites

- La recherche trouve un réglage par son libellé ou sa description, et une page par son nom ; pas par sa valeur, ni par le titre d'un groupe (« Réseau » trouve la page Avancé par sa description, pas le groupe). Un réglage qu'une page ne montre pas dans l'état présent (« Ton identifiant » avant qu'il existe, un harness non installé) ouvre sa page sans rien surligner.
- Un point ambre ne dit pas un jeton expiré tant qu'aucune vérification ne l'a trouvé (Services connectés en lance une à chaque ouverture de sa page) ; un plugin qui s'arrête pendant que la fenêtre est ouverte ne l'allume qu'à la prochaine ouverture.
- Rien ne revient sur un réglage enregistré : pour le défaire, on le change de nouveau.
- Le proxy n'est pas donné aux terminaux intégrés sans l'option qui l'exporte.
- Ignorer la vérification des certificats ne vaut pour un process `claude` qu'à son prochain démarrage.
- La page Plugins n'active pas un plugin projet par projet : c'est la page Confidentialité de chaque projet, ou l'onglet Projets de la page du plugin, dans la vue Plugins.
- Les textes des plugins publiés à part (Jira, ClickUp, Azure DevOps, Copilot) disent encore les anciens noms d'onglets jusqu'à leur prochaine version (ESC-387) ; le test des chemins ne lit pas `plugins/`.
