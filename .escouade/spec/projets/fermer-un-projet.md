---
title: "Fermer un projet"
subject: "Projets"
sources: ["docs/SPEC.md"]
tickets: ["ESC-89"]
decisions: []
compared: null
---

## Ce que c'est

Fermer un projet le retire de l'app sans rien perdre : son historique (réglages, agents, conversations, tickets) est gardé. Ajouter de nouveau un projet dans son dossier le retrouve, avec le même identifiant, donc les mêmes stats. L'effacer est un choix à part, toujours confirmé.

## Comportement

### Fermer

- « Fermer » retire le projet de l'app et arrête ses process, ses terminaux, ses préparations de worktree, ses lancements de test et ses services isola. Il ne touche ni aux fichiers du dossier, ni aux worktrees, ni aux sessions de Claude Code, ni aux stats.
- La fenêtre de confirmation dit que l'historique est gardé et propose la case « Effacer aussi son historique (conversations, tickets, réglages) », décochée par défaut. La zone sensible de Réglages › Projets montre la même confirmation.
- Refusé tant que la validation d'un ticket du projet tourne : « La validation du ticket {key} est en cours : attends sa fin pour fermer le projet. » ; et une validation ne démarre pas sur un projet en cours de fermeture.
- Un ticket « En cours » à ce moment est gardé bloqué avec « Projet fermé pendant le ticket » (« Project closed during the ticket », dans la langue de l'interface) ; un ticket dont la validation tournait repasse sans étape, bloqué comme après un redémarrage (« Validation interrompue »), sans conflit.
- Deux fermetures du même projet en même temps n'en font qu'une. L'historique gardé est le projet tel qu'il sort de la liste : un ticket créé ou un réglage changé pendant la fermeture y est.

### Ce qui est gardé

- Un fichier `<dossier de données>/closed-projects/<id du projet>.json` : le projet entier (réglages, Kanban, intégrations, commandes de lancement et de préparation…), tous ses agents (archivés compris), ses tickets, l'agent sélectionné et la date de fermeture. Pas les comptes qui lui étaient réservés : ils gardent le projet dans leur « Réservé aux projets » (voir *Comptes*).
- Les journaux de conversation (`conversations/<agent>.jsonl`), ceux de préparation de worktree, les images et les fichiers des tickets (`tickets/<id>/`) et les fichiers joints aux agents (`attachments/<agent>/`) restent où ils sont. Ce qu'un tour coupé laissait ouvert y est fermé : question ou permission annulée, outil interrompu, texte qui ne s'écrit plus.
- Les ports des agents d'un projet fermé restent retenus : aucun autre agent ne les reçoit en attendant sa réouverture.
- Le fichier est écrit deux fois. D'abord avant que rien ne s'arrête : s'il ne peut pas l'être, le projet reste ouvert (« L'historique du projet n'a pas pu être écrit (…) : le projet reste ouvert. »). Puis une fois tout arrêté : un échec, rare, est dit par un toast, et la première écriture reste.
- Un arrêt brutal de l'app pendant la fermeture ne perd rien : au démarrage suivant, un projet resté ouvert qui a un fichier de projet fermé y reprend les agents et les tickets que l'état enregistré n'avait plus, et le fichier s'efface une fois l'état enregistré.
- Les synchros d'intégration en file d'un ticket d'un projet fermé (statut externe, commentaires de boucle…) sont mises de côté, ni tentées ni oubliées ; celles d'un ticket qui n'existe plus nulle part sont oubliées.

### Rouvrir

- Ajouter un projet dans le dossier d'un projet fermé lui rend son historique, avec le même identifiant, donc ses stats : réglages, agents (sans process, comme après un redémarrage de l'app ; un worktree disparu suit la règle habituelle), conversations, tickets (un ticket « En cours » revient bloqué, « Reprendre » le relance) et ses synchros d'intégration en file, relancées. Les réservations de comptes n'ont pas quitté les comptes : rien à remettre. Ses agents Remote Control redémarrent comme au lancement de l'app.
- La fenêtre d'ajout le dit et propose « Rouvrir le projet » ou « Partir d'un projet vierge » (voir *Nouveau projet*). Le nom, la couleur et « Un worktree par agent » sont ceux de la fenêtre (nom vide : celui de l'historique). Le chemin du projet reste celui de l'historique, d'où ses agents et ses worktrees ont été faits (le même dossier, peut-être écrit autrement). Son « Compte préféré » retombe sur Automatique si ce compte n'existe plus. Un premier agent demandé est créé en plus ; la fenêtre n'en demande pas quand un historique est trouvé.
- Plusieurs projets fermés au même dossier : le plus récemment fermé, sauf celui que « Rouvrir » a choisi dans la liste. Un second ajout pendant qu'une réouverture est en cours rend ce projet, pas un projet vierge de plus.
- Le fichier n'est supprimé qu'une fois l'état enregistré : un arrêt brutal ne perd rien. S'il ne peut pas être supprimé (antivirus, fichier ouvert), les agents et les tickets que l'utilisateur supprime ensuite ne ressuscitent pas au démarrage suivant, qui retente de supprimer le fichier sans jamais le refusionner.
- Une réouverture n'est pas comptée comme une création de projet dans les statistiques anonymes.
- « Partir d'un projet vierge » : un projet neuf, avec un nouvel identifiant ; l'historique reste un projet fermé, listé dans Réglages › Projets › « Projets fermés ». Rien n'est effacé sans demande.

### Le même dossier

- Deux chemins désignent le même dossier quand ils s'écrivent pareil une fois ainsi ramenés : espaces de bord retirés ; préfixes `\\?\` et `\\?\UNC\` retirés (le second devient `\\`) ; sous Windows, `\` lu comme `/` ; `/` répétés réduits à un (sauf le `//` d'un chemin UNC) ; `/` final retiré ; un partage WSL (`//wsl$/<distribution>/…` ou `//wsl.localhost/<distribution>/…`, hôte sans égard à la casse) écrit `//wsl.localhost/<distribution en minuscules>/…` (Linux distingue la casse du reste) ; sinon tout en minuscules sous Windows, la casse gardée ailleurs. Des chemins qui mènent au même dossier (noms courts 8.3, liens) le désignent aussi.
- La recherche compare d'abord les chemins sans toucher au disque ; seulement si rien ne correspond, le chemin résolu du dossier saisi à ceux des projets fermés sur un disque local, jamais sur un partage réseau ou WSL (qui pourrait bloquer ou démarrer une distribution).

### La liste des projets fermés

- Réglages › Projets › « Projets fermés », seulement s'il y en a, sous la note « Leur historique est gardé : ajouter un projet dans leur dossier le retrouve. », du plus récemment fermé au plus ancien. Par projet : pastille, nom, dossier, taille (son fichier, les conversations et les journaux de préparation de ses agents, les fichiers joints à ses agents, les images et les fichiers de ses tickets), « dernière activité le … » (la date la plus récente d'un agent ou d'un ticket, à défaut la fermeture), « Rouvrir » (la fenêtre d'ajout sur son dossier et sur cet historique-là) et « Effacer l'historique… ».
- Un fichier dont l'identifiant est celui d'un projet ouvert (arrêt brutal entre l'écriture du fichier et l'enregistrement de l'état) est ignoré partout : listes et recherche par dossier.
- Un fichier de `closed-projects/` illisible (JSON invalide, ou nom qui n'est pas `<id de son contenu>.json`) est listé quand même : « Historique illisible », avec son nom `<id>.json` et sa taille, sans « Rouvrir », avec « Effacer ». La confirmation : « Ce fichier, que l'application ne peut pas lire, est effacé pour de bon. Les autres historiques, les fichiers et les worktrees sur le disque ne sont pas touchés. » ; seul ce fichier est supprimé.

### Effacer

- « Effacer aussi son historique », à la fermeture : les journaux de conversation et de préparation, les images des tickets et les fichiers joints aux agents sont supprimés ; rien n'est gardé dans `closed-projects`.
- « Effacer l'historique… » d'un projet fermé, après confirmation (« Ses conversations, ses tickets et ses réglages sont effacés pour de bon. Les fichiers et les worktrees sur le disque ne sont pas touchés. ») : supprime son fichier, les journaux de ses agents et les images de ses tickets. Une erreur lisible, dans la langue de l'interface, si aucun projet fermé n'a cet identifiant.
- Dans les deux cas, le projet sort de « Réservé aux projets » de chaque compte, et les synchros d'intégration en file de ses tickets sont oubliées.

## Limites

- Un projet fermé avant la 1.8 a perdu son historique à la fermeture : il n'y a rien à retrouver ni à migrer.
