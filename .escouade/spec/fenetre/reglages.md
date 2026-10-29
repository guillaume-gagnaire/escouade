---
title: "Réglages"
subject: "Fenêtre"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

Une seule fenêtre de réglages pour toute l'app, avec ses onglets à gauche. Les changements forment un brouillon, enregistré d'un coup ou jeté. Quelques réglages, qui se voient tout de suite, s'appliquent sans attendre. Chaque réglage est décrit sur la page de sa feature ; cette page dit comment la fenêtre marche et ce que chaque onglet contient.

## Comportement

### Ouvrir

- ⚙ de la barre de statut, « Réglages » en bas de la barre des projets, ou `Ctrl+,`.
- Ouvertures directes :
  - « ⚙ Après validation » du Kanban : l'onglet Kanban de son projet ;
  - ⚙, « Configurer » et « ✦ Proposer des commandes » de la section Lancement : l'onglet Projets, sur ses commandes de lancement ; le dernier lance aussi la lecture du projet (voir *Barre latérale*) ;
  - « Réglages du projet… » au clic droit sur un projet : l'onglet Projets de ce projet.
- Chaque onglet, et chaque projet, s'affiche depuis son début.

### La fenêtre

- Les onglets à gauche, un point sur un onglet modifié, la version d'Escouade en bas. Un onglet à la fois, sous son titre et sa description.
- « Enregistrer » enregistre d'un coup ce qui a changé, dans tous les onglets et tous les projets, chaque partie par-dessus ce que l'app a alors (le Kanban, lui, avance seul).
- « Enregistrer » est refusé, avec la raison, tant qu'un projet n'a pas de nom, qu'une commande de lancement n'a pas de nom ou de ligne de commande, ou qu'un dossier des worktrees ou un préfixe de branche n'est pas valide. Ce qui n'a pas pu être enregistré reste dans la fenêtre.
- « Annuler », Échap ou × jettent tout le brouillon.
- Hors du brouillon, appliqués et enregistrés tout de suite : « Affichage » (voir *Fenêtre*), l'onglet Comptes et sa « Projection des quotas » (voir *Barre de statut*), les comptes des intégrations (voir *Intégrations*), l'interrupteur « Les agents peuvent piloter Escouade » (voir *Serveur MCP*), et les modèles de la voix, téléchargés ou effacés sur leur ligne.

### Les onglets

- **Application**, en tête :
  - « Affichage » : « Thème », « Zoom de l'interface », « Taille du texte des terminaux », « Taille du texte de l'éditeur » (voir *Fenêtre*) ;
  - « Langue de l'interface » (« Système (…) » par défaut, « English », « Français ») et « Langue des textes rédigés par Claude » (« Comme l'interface » par défaut), « Modèle des textes rédigés » (voir *Langues*).
- **Harness** :
  - un groupe par harness : exécutable, version, état, « Se connecter… », et pour OpenCode ses « Modèles » favoris (voir *Harness*) ;
  - pour Claude Code : le chemin de `claude`, avec dessous où est le `claude` de ce chemin (voir *Claude Code dans WSL*) ; modèle, effort et mode par défaut des nouveaux agents ; reprise automatique après la limite d'usage ; « Reprendre sur un autre compte un agent de ticket arrêté par la limite », avec plusieurs comptes (voir *Comptes*) ; « Les agents tiennent une liste de tâches », activé par défaut (voir *Avancée d'un plan*) ; arrêt des process inactifs ;
  - « Pièces jointes » : taille maximale, avertissement au-delà d'un seuil, fichier d'origine ou copie, gros textes, purge des copies (voir *Conversation*) ;
  - « Remote control » : « Remote control pour tous les agents » (voir *Barre latérale*) ;
  - « Serveur MCP » : « Les agents peuvent piloter Escouade », et « Activité MCP » (voir *Serveur MCP*). C'est le seul interrupteur du serveur MCP ; il sert les agents de tous les projets.
- **Comptes** : la liste des comptes, « Ajouter un compte… », « Se connecter… » (voir *Comptes*) ; « Quotas » › « Projection des quotas » (voir *Barre de statut*).
- **Notifications** : « Me prévenir pour », notifications du système, son (activé, « ▶ Tester ») (voir *Notifications*).
- **Voix** : « Micro utilisé » (celui du système par défaut), « Modèle », « Langue parlée », « Raccourci » (voir *Dictée*) ; « Conversation » : la voix de chaque langue, « Vitesse », « Volume », « Délai avant l'envoi » (voir *Conversation vocale*).
- **Projets**, avec un sélecteur de projet (celui affiché par défaut ; un cadenas après le nom d'un projet privé) :
  - « Identité » : nom, dossier (« Ouvrir »), couleur, lettres, logo et libellé de la tuile (voir *Barre des projets*), « Compte préféré » avec plusieurs comptes ;
  - « Remote control » : « Remote control pour les agents de ce projet » (voir *Barre latérale*) ;
  - « Confidentialité » : « Projet privé » (éteint par défaut) et « Ses fichiers » (voir *Serveur MCP*) ; pour un projet privé, ses « Comptes » réservés ou partagés et « Comptes partagés à la limite » (voir *Comptes*) ;
  - « Git » : « Commit » (« Rédigé par l'agent » par défaut, ou « Direct, avec un message proposé »), worktree par agent, dossier des worktrees, préfixes des branches des agents et des tickets, fichiers copiés dans les worktrees, commandes de préparation et de démontage des worktrees (« ✦ Remplir automatiquement », « Monter », « Descendre ») (voir *Git & fichiers*) ;
  - « Worktrees restés » › « Nettoyer » (ci-dessous) ;
  - « Lancement » : les commandes de lancement et leur « ✦ Remplir automatiquement » (voir *Barre latérale*) ;
  - « Assistant de configuration » : « Ouvrir l'assistant… » (voir *Nouveau projet*) ;
  - « Fermer le projet… » : sa confirmation, avec la case « Effacer aussi son historique (conversations, tickets, réglages) » décochée par défaut, revient aux réglages, acceptée ou non, le brouillon intact (voir *Fermer un projet*) ;
  - sous le projet : le groupe « Nouveaux projets » (« Nouveaux projets privés », éteint par défaut : chaque projet ajouté est privé dès le départ), puis « Projets fermés », seulement s'il y en a, avec par projet « Rouvrir » et « Effacer l'historique… », et « Historique illisible » avec « Effacer » pour un fichier que l'app ne sait pas lire (voir *Fermer un projet*).
- **Kanban**, même sélecteur :
  - branche cible, choisie dans le sélecteur de branche en mode « Branche cible » : les branches locales du projet, recherche comprise, sans celles des agents et des tickets (leurs préfixes du projet, `escouade/` et `ticket/`), la branche du dossier comprise, la cible actuelle cochée (voir *Kanban*). D'où partent les nouveaux worktrees se règle dans l'onglet Projets (voir *Git & fichiers*) ;
  - action à la validation (stratégie, ou PR en brouillon), commande de tests et « Relancer les tests avant », « Supprimer le worktree d'un ticket terminé ou supprimé », message de commit généré avec son aperçu, conflit (voir *Validation*) ;
  - pilote auto, agents en parallèle et leurs modèle, effort et mode (voir *Pilote auto*) ;
  - « Pause au-delà du quota », « pour tous les projets » : un réglage de l'app, qui marque aussi cet onglet comme modifié (voir *Comptes*).
- **Intégrations**, même sélecteur : comptes Jira, Trello, GitHub ; sources liées au projet, correspondance des statuts, synchronisation et import automatique (voir *Intégrations*).
- **Terminaux** : PowerShell 7, Git Bash, distribution WSL (bash sous macOS), shells détectés (voir *Barre latérale*).
- **Réseau** : proxy et certificats (ci-dessous).
- **À propos** : version, « Rechercher une mise à jour », « Installer les mises à jour automatiquement » (activé par défaut : « Escouade redémarre d'elle-même quand aucun agent ne travaille et que tout est enregistré. »), données locales (voir *Livraison et mises à jour*).

### Nettoyer les worktrees restés

- « Worktrees restés » › « Nettoyer », dans l'onglet Projets : les worktrees des agents archivés du projet encore sur le disque sont supprimés, avec la même prudence que ceux des tickets finis (voir *Validation*), puis git oublie ceux dont le dossier a disparu (`git worktree prune`).
- La rangée dit ensuite combien ont été supprimés (« Aucun worktree à supprimer. ») et ceux qui sont gardés, avec pourquoi.

### Réseau

- **Proxy** : une adresse HTTP(S), avec identifiants s'il en faut, et ses exclusions (`NO_PROXY`).
  - Il est donné aux process `claude` (`HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`), aux lectures des quotas, aux intégrations et à la recherche de mises à jour.
  - La recherche de mises à jour, si le proxy lui-même est injoignable, refait sa requête une fois sans lui ; si le proxy la refuse, elle ne le contourne pas (voir *Livraison et mises à jour*).
  - Une option l'exporte aussi dans les terminaux intégrés.
- **Certificats** : les requêtes de l'app (intégrations, quotas, mises à jour) font confiance aux autorités habituelles et à celles du magasin du système, dont le certificat d'un proxy d'entreprise qui y est installé. Claude Code garde sa propre liste.
- « Ignorer la vérification des certificats TLS » (désactivé par défaut) : pour un proxy qui déchiffre le trafic avec un certificat que le système ne reconnaît pas (« UnknownIssuer »). Plus aucun certificat n'est alors vérifié, ni pour les intégrations, les quotas et les mises à jour, ni pour les process `claude` (`NODE_TLS_REJECT_UNAUTHORIZED=0`, au prochain démarrage de leur process, hérité par les commandes que lancent les agents). Les terminaux et les commandes des worktrees ne sont pas concernés. Les mises à jour suivent les réglages réseau du moment.

## Limites

- Le proxy n'est pas donné aux terminaux intégrés sans l'option qui l'exporte.
- Ignorer la vérification des certificats ne vaut pour un process `claude` qu'à son prochain démarrage.
