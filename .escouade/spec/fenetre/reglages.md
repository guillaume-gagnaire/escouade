---
title: "Réglages"
subject: "Fenêtre"
sources: ["docs/SPEC.md","design/settings-2-2/PROPOSITION.md","design/settings-2-2/CONSTRUCTION.md"]
tickets: ["ESC-378","ESC-379","ESC-380"]
decisions: [118,"les-reglages-s-appliquent-et-s-enregistr"]
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
  - « Réglages du projet… » au clic droit sur un projet : la page Projet de ce projet.
- Chaque page, et chaque projet, s'affiche depuis son début (ou depuis sa partie, tant qu'elle montre un ancien onglet : voir *Les pages*).

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
- **« Avancé »**, en bas d'une page : un groupe replié sur une ligne qui nomme ce qu'il cache ; un clic le déplie sur place. Pour l'instant, seule la page Projet en a un : « Zone sensible », qui cache « Fermer le projet ».
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
- Certaines rangées passent par leur propre appel au backend, comme avant : « Affichage » (voir *Fenêtre*), les comptes des harness et leur « Projection des quotas » (voir *Barre de statut*), les comptes des intégrations (voir *Intégrations*), l'interrupteur « Les agents peuvent piloter Escouade » (voir *Serveur MCP*), et les modèles de la voix, téléchargés ou effacés sur leur ligne.
- Rien ne se perd au redémarrage : ce qui est enregistré est dans `settings.json` et les fichiers des projets, dont aucune clé ne change.

### Les pages

Chaque réglage a sa page dans le registre de la fenêtre. Jusqu'à ce que chaque lot de la refonte les redécoupe, les pages montrent encore le contenu des onglets de la 2.0 :

- **Général** : l'ancien onglet Application.
- **Agents** : le serveur MCP (« Les agents peuvent piloter Escouade », « Activité MCP »).
- **Comptes et harness** : les comptes, puis l'ancien onglet Harness.
- **Services connectés** et **Tickets importés** (sur sa partie « Sources ») : l'ancien onglet Intégrations, pour le projet choisi.
- **Voix**, **Notifications** : comme avant.
- **Plugins** : le lien vers la vue Plugins.
- **Avancé** : Réseau, Terminaux, puis « Recherche par le sens ».
- **À propos** : comme avant.
- **Projet**, **Git et worktrees**, **Lancement**, **Équipe**, **Confidentialité** : l'ancien onglet Projets du projet, chacune ouverte sur sa partie (Git, Lancement, Kanban partagé, Confidentialité) ; « Fermer le projet… » est sous « Zone sensible », repliée. Fermer ou non revient aux réglages (voir *Fermer un projet*).
- **Kanban** : l'ancien onglet Kanban du projet.
- **Tous les projets** : « Nouveaux projets » (« Nouveaux projets privés », éteint par défaut), puis « Projets fermés », seulement s'il y en a, avec par projet « Rouvrir » et « Effacer l'historique… », et « Historique illisible » avec « Effacer » (voir *Fermer un projet*). Effacer un historique revient sur cette page.

Le détail des réglages de chaque ancien onglet :

- **Application** : « Affichage » (« Thème », « Zoom de l'interface », « Taille du texte des terminaux », « Taille du texte de l'éditeur », voir *Fenêtre*) ; « Langue de l'interface » (« Système (…) » par défaut, « English », « Français ») et « Langue des textes rédigés par Claude » (« Comme l'interface » par défaut), « Modèle des textes rédigés » (voir *Langues*).
- **Harness** : un groupe par harness (exécutable, version, état, « Se connecter… », et pour OpenCode ses « Modèles » favoris, voir *Harness*) ; pour Claude Code, le chemin de `claude` et où il est (voir *Claude Code dans WSL*), reprise automatique après la limite d'usage, « Reprendre sur un autre compte un agent de ticket arrêté par la limite » avec plusieurs comptes (voir *Comptes*), « Les agents tiennent une liste de tâches », activé par défaut (voir *Avancée d'un plan*), arrêt des process inactifs ; « Pièces jointes » (voir *Conversation*) ; « Remote control pour tous les agents » (voir *Barre latérale*).
- **Comptes** : la liste des comptes, « Ajouter un compte… », « Se connecter… » (voir *Comptes*) ; « Quotas » › « Projection des quotas » (voir *Barre de statut*).
- **Notifications** : « Me prévenir pour », les nouvelles des hébergeurs, notifications du système, son (voir *Notifications*).
- **Voix** : « Micro utilisé », « Modèle », « Langue parlée », « Raccourci » (voir *Dictée*) ; « Conversation » (voir *Conversation vocale*).
- **Projets** : « Identité » (nom, dossier, couleur, lettres, logo et libellé de la tuile, harness et « Compte préféré », « Assistant de configuration ») ; « Remote control pour les agents de ce projet » ; « Confidentialité » (« Projet privé », « Ses fichiers », ses comptes, voir *Serveur MCP* et *Comptes*) ; « Workflow » ; « Git » (commit, worktrees, préfixes, fichiers copiés, commandes de préparation et de démontage, voir *Git & fichiers*) ; « Worktrees restés » › « Nettoyer » ; « Lancement » ; le Kanban partagé, `.escouade.yml` et la mémoire partagée.
- **Kanban** : branche cible, action à la validation, tests, nettoyage, message de commit, conflit (voir *Validation*) ; pilote auto et ses agents (voir *Pilote auto*) ; « Pause au-delà du quota », « pour tous les projets » : un réglage de l'app, rangé dans la page Comptes et harness (voir *Comptes*).
- **Intégrations** : comptes des services et hébergeurs ; sources liées au projet, correspondance des statuts, synchronisation et import automatique (voir *Intégrations*).
- **Terminaux** : PowerShell 7, Git Bash, distribution WSL (bash sous macOS), shells détectés (voir *Barre latérale*).
- **Réseau** : proxy et certificats (ci-dessous).
- **À propos** : version, « Rechercher une mise à jour », « Installer les mises à jour automatiquement », « Recevoir les versions bêta », statistiques d'usage, données locales (voir *Livraison et mises à jour*).

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
