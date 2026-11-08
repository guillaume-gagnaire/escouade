---
title: "Réglages"
subject: "Fenêtre"
sources: ["docs/SPEC.md","design/settings-2-2/PROPOSITION.md","design/settings-2-2/CONSTRUCTION.md","design/settings-2-2/index.html","design/settings-2-2/captures/a-plugins.png"]
tickets: ["ESC-378","ESC-379","ESC-380","ESC-381","ESC-383","ESC-384"]
decisions: [118,119,120,121]
compared: null
---

## Ce que c'est

Une seule fenêtre de réglages pour toute l'app : une barre des catégories à gauche, une page à la fois à droite. Les pages d'Escouade règlent l'app ; celles d'un projet ne règlent que lui. Chaque réglage s'applique et s'enregistre au geste, comme dans les réglages de macOS et de VS Code : ni « Annuler », ni « Enregistrer ». Chaque réglage est décrit sur la page de sa feature ; cette page dit comment la fenêtre marche et ce que chaque page contient.

## Comportement

### Ouvrir

- ⚙ de la barre de statut, « Réglages » en bas de la barre des projets, ou `Ctrl+,` : sur « Comptes et harness ».
- Ouvertures directes :
  - « ⚙ Après validation » du Kanban : la page Kanban de son projet ;
  - ⚙, « Configurer » et « ✦ Proposer des commandes » de la section Lancement : la page Lancement du projet ; le dernier lance aussi la lecture du projet (voir *Barre latérale*) ;
  - « Réglages du projet… » au clic droit sur un projet : la page Projet de ce projet ;
  - « Lier une source » et « ⚙ Gérer les sources » de la fenêtre d'import : la page Tickets importés de son projet (voir *Intégrations*) ;
  - « Réglages › Services connectés » de l'onglet « PR et CI » et de la vue Pull requests (jeton refusé, hébergeur sans compte) : la page Services connectés (voir *Pull requests*) ;
  - « Redémarrer maintenant » d'une mise à jour prête, dans Général : la fenêtre de la mise à jour, dont « Plus tard » revient sur Général.
- Chaque page, et chaque projet, s'affiche depuis son début (ou depuis sa partie, tant qu'elle montre un ancien onglet : voir *Les pages*).
- Les textes de l'app qui nomment une page la nomment par son chemin : « Réglages › Général › Couleurs » (thèmes d'un plugin), « Réglages › Général » (ton nom, statistiques d'usage), « Réglages › Agents » (remote control pour tous les agents).

### La fenêtre

- 1120 × 780, ou la place disponible avec 16 px autour (à 1028 × 779, la taille de la CI).
- **La barre**, de haut en bas :
  - « Rechercher un réglage » (`Ctrl+F`), inerte pour l'instant ;
  - « Escouade » : Général, Agents, Comptes et harness, Services connectés, Voix, Notifications, Plugins, Avancé ;
  - « Projets » : chaque projet, sa couleur devant son nom, un cadenas pour un projet privé. Le projet choisi, quand une de ses pages est à l'écran, est déplié sur ses pages : Projet, Git et worktrees, Lancement, Kanban, Tickets importés, Équipe, Confidentialité. Choisir un autre projet garde la même page, sinon ouvre « Projet » ;
  - « Tous les projets » (ce que reçoit un nouveau projet, les projets fermés) ;
  - en bas : « À propos » et la version d'Escouade.
- **Le clavier** : ↑ et ↓ vont d'une entrée de la barre à la suivante et l'ouvrent, Origine et Fin à la première et à la dernière ; passer sur un projet l'ouvre et le déplie. Une seule entrée de la barre est dans l'ordre de Tab : celle où est le clavier.
- **La teinte suit la portée** : sur une page d'Escouade (Tous les projets compris), la fenêtre prend le teal d'Escouade ; sur une page d'un projet, la couleur de ce projet, dès qu'elle change. À la fermeture, l'app reprend sa propre teinte.
- La page sous son titre et sa description ; pour une page d'un projet, sa tuile devant, et son nom pour titre sur la page Projet.
- **« Avancé »**, en bas d'une page : un groupe replié sur une ligne qui nomme ce qu'il cache ; un clic le déplie sur place. Général, Agents et Projet en ont un (celui de Projet s'appelle « Zone sensible » et cache « Fermer le projet »).
- Pas de pied : ni « Annuler » ni « Enregistrer ». Échap, × ou un clic à côté de la fenêtre la ferment, sans question.

### Enregistrer au geste

- **Un choix** (interrupteur, segments, menu, case, raccourci enregistré, commande ajoutée ou retirée, proposition de Claude prise, logo choisi) s'applique et s'enregistre aussitôt.
- **Un texte** (champ, nombre) s'enregistre quand il perd le focus, ou après 0,7 s sans frappe. Fermer la fenêtre enregistre ce qui attendait encore.
- Chaque enregistrement envoie ce qui a changé par-dessus ce que l'app a alors (le Kanban, lui, avance seul), un à la fois : ce qui change pendant qu'un enregistrement tourne part avec le suivant. Rien n'est envoyé quand rien n'a changé.
- **Une saisie invalide** se dit sur sa rangée, et la valeur enregistrée avant reste tant qu'elle n'est pas corrigée ; le reste s'enregistre :
  - un projet sans nom : « Un projet garde un nom : « demo-api » reste enregistré. » ;
  - une commande de lancement sans nom ou sans ligne de commande : les commandes enregistrées restent celles d'avant tant qu'il en manque ;
  - un dossier des worktrees ou un préfixe de branche refusé, avec la raison sous le champ (voir *Git & fichiers*) ;
  - un nombre vidé ou sous 0, par exemple « Un nombre entier, 0 ou plus : 15 reste enregistré. » ; un nombre à virgule est arrondi à l'entier du dessous ; une taille de texte hors de ses bornes est ramenée dedans.
- Ce qui coûte le dit sur sa rangée, comme avant : le proxy et les certificats ne valent pour un agent qu'au prochain démarrage de son process, le remote control lance aussitôt le process `claude` de chaque agent concerné.
- Certaines rangées passent par leur propre appel au backend, comme avant : l'apparence (voir *Fenêtre*), « Recevoir les versions bêta » (enregistré, puis une recherche de mise à jour), les comptes des harness et leur « Projection des quotas » (voir *Barre de statut*), les comptes des intégrations (voir *Intégrations*), l'interrupteur « Les agents peuvent piloter Escouade » (voir *Serveur MCP*), les modèles de la voix, téléchargés ou effacés sur leur ligne, et, sur la page Plugins, l'interrupteur d'un plugin et « Installer » d'un plugin proposé.
- Rien ne se perd au redémarrage : ce qui est enregistré est dans `settings.json` et les fichiers des projets, dont aucune clé ne change.

### Les pages

Chaque réglage a sa page dans le registre de la fenêtre. Les pages d'Escouade et Tickets importés ont leur contenu propre ; Comptes et harness et les autres pages d'un projet montrent encore le contenu des onglets de la 2.0, jusqu'à ce que leur lot de la refonte les redécoupe.

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
- **Comptes et harness** : les comptes (« Ajouter un compte… », « Se connecter… », voir *Comptes* ; « Quotas » › « Projection des quotas », voir *Barre de statut*) ; puis un groupe par harness (exécutable, version, état, installation, coût estimé, et pour OpenCode ses « Modèles » favoris, voir *Harness*) ; pour Claude Code, le chemin de `claude` et où il est (voir *Claude Code dans WSL*) et sa mise à jour par Escouade ; puis « Tous les harness » › reprise automatique après la limite d'usage.
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
- **Projet**, **Git et worktrees**, **Lancement**, **Équipe**, **Confidentialité** : l'ancien onglet Projets du projet, chacune ouverte sur sa partie (Git, Lancement, Kanban partagé, Confidentialité) ; « Fermer le projet… » est sous « Zone sensible », repliée. Fermer ou non revient aux réglages (voir *Fermer un projet*). Leur contenu : « Identité » (nom, dossier, couleur, lettres, logo et libellé de la tuile, harness et « Compte préféré », « Assistant de configuration ») ; « Remote control pour les agents de ce projet » ; « Confidentialité » (« Projet privé », « Ses fichiers », ses comptes, voir *Serveur MCP* et *Comptes*) ; « Workflow » ; « Git » (commit, worktrees, préfixes, fichiers copiés, commandes de préparation et de démontage, voir *Git & fichiers*) ; « Worktrees restés » › « Nettoyer » ; « Lancement » ; le Kanban partagé, `.escouade.yml` et la mémoire partagée.
- **Tickets importés** : les sources liées au projet et la correspondance de leurs statuts ; sans compte connecté, un renvoi à Services connectés (voir *Intégrations*).
- **Kanban** : branche cible, action à la validation, tests, nettoyage, message de commit, conflit (voir *Validation*) ; pilote auto et ses agents (voir *Pilote auto*) ; « Pause au-delà du quota », « pour tous les projets » : un réglage de l'app, rangé dans la page Comptes et harness (voir *Comptes*).
- **Tous les projets** : « Nouveaux projets » (« Nouveaux projets privés », éteint par défaut), puis « Projets fermés », seulement s'il y en a, avec par projet « Rouvrir » et « Effacer l'historique… », et « Historique illisible » avec « Effacer » (voir *Fermer un projet*). Effacer un historique revient sur cette page.

### La page Plugins

Le tableau des plugins, côté réglages. La vue Plugins reste l'endroit où l'on installe depuis le store, un dossier ou un dépôt, où l'on active un plugin projet par projet, retire une permission et lit un journal (voir *Les plugins : vue d'ensemble*).

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

- La recherche de la barre ne cherche rien encore.
- Rien ne revient sur un réglage enregistré : pour le défaire, on le change de nouveau.
- Le proxy n'est pas donné aux terminaux intégrés sans l'option qui l'exporte.
- Ignorer la vérification des certificats ne vaut pour un process `claude` qu'à son prochain démarrage.
- La page Plugins n'active pas un plugin projet par projet : c'est l'onglet Projets de sa page, dans la vue Plugins (et, plus tard, la page Confidentialité du projet).
