# Journal des versions

Les changements visibles d'Escouade, version par version. Les notes de chaque release GitHub (et de la mise à jour intégrée) reprennent la section de sa version.

## [1.7.0] — 2026-10-11

### Ajouts

#### Escouade en anglais et en français

- Réglages › Application (nouvel onglet, en tête) : « Langue de l'interface » propose « Système (Français) » (par défaut), « English » et « Français ». Par défaut, Escouade parle la langue de ton système : le français s'il est en français, l'anglais sinon (pour une installation neuve : des réglages d'avant la 1.7 gardent l'interface, et les textes de Claude, en français jusqu'à ce que tu choisisses). Le changement s'applique dès « Enregistrer », sans redémarrer : la fenêtre, le menu de l'icône de la barre des tâches (« Afficher », « Quitter ») et son infobulle suivent, et sous macOS le menu de l'app aussi, avec les mots de macOS (ce que macOS y ajoute de lui-même suit la langue du système). L'installeur Windows parle lui aussi anglais ou français, selon le système.
- Tout est traduit : conversation, éditeur, Kanban, statistiques, vue d'ensemble, barre latérale et barre d'état, git et ses fenêtres, lancements, terminaux, réglages, mises à jour, confirmations et messages. Ce que l'app écrit elle-même aussi : erreurs, notices de la conversation, ce que fait l'agent (« Lit src/a.ts » / « Reading src/a.ts »), raison d'un ticket bloqué, ce que dit le Kanban quand aucun ticket ne démarre, issue d'une validation, notifications du système (« Autoriser Bash : npm test ? » / « Allow Bash: npm test? ») et étapes des statistiques. En anglais, la colonne « À tester » s'appelle « To review ».
- Les nombres, montants, pourcentages, durées, dates, heures et tailles s'écrivent à la façon de la langue (« 0,10 $ » / « $0.10 », « 42 % » / « 42% », « 4j 12h » / « 4d 12h »), les pluriels suivent sa règle (« 0 fichier » en français, où 0 et 1 sont au singulier ; « 0 files » en anglais, où seul 1 l'est) et les touches des raccourcis s'écrivent comme sur son clavier (« Ctrl+Maj+Entrée » / « Ctrl+Shift+Enter »).
- Réglages › Application › « Langue des textes rédigés par Claude » (« Comme l'interface » par défaut, « English », « Français ») : la langue de tout ce qu'Escouade dit aux agents et fait écrire à Claude, car beaucoup d'équipes commitent en anglais avec une interface en français. Elle porte sur le protocole d'un ticket et ses messages (boucles, relance du bilan, reprise, retour de test, tests en échec, merge et rebase de la branche cible), « Préparer le lancement », le message de commit d'un ticket et la description de sa pull request, le message proposé pour un commit direct, le nom donné à un agent, les propositions de « ✦ Remplir automatiquement » (noms des commandes compris), la consigne d'une copie d'agent, et les commentaires publiés dans Jira, Trello et GitHub (« Escouade : ESC-12 est prêt à tester. » / « Escouade: ESC-12 is ready to review. »).
  - Le message de commit d'un ticket nomme sa langue : un ticket importé dans une autre langue ne la décide plus. Le message proposé pour un commit direct est écrit dans la langue choisie quand le réglage en nomme une ; avec « Comme l'interface », il imite toujours la langue des derniers commits du dépôt, comme avant.
- Un texte déjà écrit garde la langue dans laquelle il a été écrit : une notice de la conversation, la raison d'un ticket bloqué, l'issue d'une validation, les critères par défaut d'un ticket, le protocole donné à un agent quand il a été créé.

#### Le site et la vidéo en anglais

- Le site existe en deux langues : le français à la racine (`/escouade/`, les liens existants restent bons) et l'anglais sous `/escouade/en/`. Chaque page a sa langue, son titre, sa description, ses balises de partage et ses liens `hreflang` vers l'autre version.
- Un sélecteur « FR / EN » dans l'en-tête mène à l'autre version et garde ton choix. À ta première visite de la racine, un navigateur sans français est envoyé vers la version anglaise, sauf si tu as déjà choisi ou si l'adresse demande le français (`?lang=fr`). Sans JavaScript, rien ne bouge.
- La démonstration animée du haut de page parle la langue de la page. Une adresse inconnue affiche une page « Page introuvable » aux couleurs du site, dans la langue de l'adresse.
- La page anglaise montre ses propres captures d'écran et son affiche, avec l'app en anglais, aux mêmes instants que les françaises. Sa vidéo propose par défaut des sous-titres anglais, aux mêmes repères que les français : la voix reste française, et les sous-titres français restent disponibles.

#### Plusieurs comptes Claude

- Réglages › Comptes Claude (nouvel onglet, après « Claude Code ») : la liste de tes comptes, dans l'ordre où les nouveaux agents les essaient. Ton Claude Code habituel y devient le compte « Principal » (« Main » en anglais), créé au premier lancement de la 1.7 : il se lance exactement comme avant, sans `CLAUDE_CONFIG_DIR`, et ta connexion n'est pas touchée (sous macOS non plus). Les agents d'avant sont dessus.
  - Chaque compte montre son nom (modifiable), son dossier, son exécutable (« Celui des réglages » par défaut, pour essayer une autre version de Claude Code), son état (« Connecté · ton@email », « Pas connecté », « Inactif »), un interrupteur « Actif » et des flèches pour l'ordre. Chaque changement est enregistré tout de suite.
  - Un compte ne se supprime pas tant qu'un agent non archivé s'en sert (« Le compte sert encore à 2 agents. »), Principal jamais ; le dossier d'un compte supprimé reste sur le disque avec sa connexion et ses conversations, et il faut toujours au moins un compte actif.
- « Ajouter un compte… » crée un compte avec son propre dossier (`~/.escouade/claude/<nom>`) et y reprend ce que tu choisis du dossier de Principal (`settings.json`, `CLAUDE.md`, `skills`, `agents`, `commands`, `plugins`, `hooks`, `output-styles`), lié (un changement vaut pour les deux comptes) ou copié. Sous Windows, les dossiers sont liés par une jonction et les fichiers copiés (un lien de fichier y demande des droits d'administrateur) ; sous macOS, les fichiers sont liés eux aussi. La connexion et `.claude.json` ne sont jamais partagés, ni les conversations : chaque compte a les siens.
- « Créer et se connecter » (ou « Se connecter… » / « Se reconnecter… » sur un compte) ouvre Claude Code pour ce compte dans un terminal de la fenêtre. Dès que la connexion est là, Escouade le dit (« Connecté au compte ton@email »), ferme le terminal et lit le quota du compte. Fermée avant, la fenêtre laisse le compte « Pas connecté ».
- Chaque agent retient son compte : il se lance avec le `claude` et le dossier de configuration de ce compte, y reprend sa session après un arrêt ou un redémarrage d'Escouade, et sa copie (« Dupliquer la conversation ») reste sur le même compte. Le nom proposé par Haiku et les messages de commit qu'il rédige passent par le compte de l'agent.
- Le quota (fenêtre de 5 h et semaine) est suivi pour chaque compte, avec sa propre connexion, au plus une fois toutes les 5 minutes chacun. Un compte pas connecté le dit (« Pas connecté »), une connexion expirée aussi (« Connexion expirée : relance Claude Code pour ce compte. »).
- Un nouvel agent part sur le compte que tu demandes, sinon sur le « Compte préféré » de son projet, sinon sur le premier compte actif sous le seuil de « Pause au-delà du quota » (dans l'ordre de l'onglet) ; si tous l'ont passé, sur le premier compte actif. Les tickets du Kanban et « ✦ Remplir automatiquement » suivent les mêmes règles.
  - Avec plusieurs comptes, une puce « Compte : Pro » à côté du modèle, dans le champ de saisie, choisit le compte d'un agent tant qu'il n'a rien reçu ; ensuite elle se lit sans se changer, la conversation étant rangée dans ce compte. Les réglages d'un projet ont un « Compte préféré » (« Automatique » par défaut).
  - Quand le compte en cours passe le seuil, un toast le dit une fois : « Le compte Pro a atteint 95 % : les nouveaux agents partent sur Principal. »
- Le pilote auto ne se met en pause que quand plus aucun compte n'est utilisable pour le projet (en « Automatique », tous les comptes actifs ont passé le seuil ; avec un compte préféré, celui-là seul) et attend la plus proche remise à zéro. La bannière dit quels comptes ont passé le seuil (« Les comptes Principal et Pro ont passé le seuil. »). Un agent qui attend la remise à zéro de son compte, ou une limite d'usage sans reprise, ne retient plus que les tickets de ce compte.
- Avec plusieurs comptes, tout le groupe des quotas de la barre d'état est un bouton qui ouvre, au-dessus de la barre, le panneau « Quotas des comptes Claude » : chaque compte avec ses deux fenêtres (barre, pourcentage, remise à zéro) et ce qu'il a dépensé aujourd'hui, le compte en cours en tête et marqué « en cours », « au-delà du seuil de pause » pour un compte qui a passé « Pause au-delà du quota », un compte inactif, pas connecté ou à la connexion expirée grisé avec sa raison. « Aujourd'hui », dans la barre d'état, reste le total de tous les comptes.
- Statistiques : chaque tour et chaque prompt est enregistré avec le compte de l'agent à ce moment ; ceux d'avant la 1.7 comptent pour Principal. Avec plusieurs comptes, l'écran « Stats » propose « Tous les comptes » ou un compte, qui filtre la période, les chiffres du haut, le graphique, « Par projet », « Par modèle », « Par agent » et « Par ticket ». Une carte « Par compte » met les comptes côte à côte : tokens, coût, tours et agents sur la période.
- Quand un tour s'arrête sur la limite d'usage, la carte de fin de tour propose « Reprendre sur Pro » (un menu pour les autres comptes s'il y en a plusieurs) : l'agent repart sur le premier autre compte actif, connecté et sous « Pause au-delà du quota », avec sa conversation, et reçoit « continue » comme à la reprise automatique. Jamais deux process sur une même session : celui de l'agent est arrêté avant que la session ne soit copiée dans le dossier de l'autre compte.
  - Si le tour de la reprise échoue sur le nouveau compte (connexion absente, erreur de l'API, Claude Code qui s'arrête avant de démarrer), la carte le dit (« La reprise sur Pro a échoué : … ») et propose « Revenir sur Principal » : la session revient avec ce que le tour raté y a ajouté, et l'agent attend de nouveau la remise à zéro de son compte.
  - Réglages › Claude Code › « Reprendre sur un autre compte un agent de ticket arrêté par la limite » (désactivé par défaut, tant que la reprise sur un second compte réel n'est pas vérifiée : voir « Limites connues » ; visible avec plusieurs comptes) : un agent de ticket arrêté par la limite part tout seul sur le compte que donnerait un nouvel agent, si un autre est utilisable ; sinon, ou si le projet préfère le compte de l'agent, il attend la remise à zéro comme avant. Le compte quitté ne reçoit plus de ticket avant sa remise à zéro. Un agent ordinaire n'est jamais déplacé seul.
- La présence de Claude Code se vérifie sur le compte où partirait l'agent : un compte qui a son propre `claude` fonctionne même si celui des réglages manque.

#### Piloter Escouade depuis Claude : le serveur MCP

- Escouade embarque un serveur MCP, qui n'écoute que sur ta machine (127.0.0.1, un port libre entre 47000 et 47999 choisi au premier démarrage puis gardé). Il tourne quand « Claude peut piloter Escouade » est activé ou qu'un projet laisse ses agents utiliser Escouade, et s'arrête avec l'app.
- Réglages › Claude Code › « Escouade dans Claude » : l'interrupteur « Claude peut piloter Escouade » (enregistré tout de suite) démarre le serveur et le déclare dans Claude Code pour chaque compte actif. L'onglet dit « Déclaré dans Claude · compte Principal, compte Pro » ; pour un compte où ça échoue, il dit pourquoi et donne la commande à lancer à la main (jeton masqué à l'écran, « Copier la commande » copie la vraie). Une entrée déjà à jour est laissée telle quelle, une entrée différente (autre port, autre jeton) est remplacée, et un compte ajouté ou réactivé plus tard reçoit la déclaration. Le jeton est écrit en clair dans la config de Claude Code ; désactiver change le jeton aussitôt, puis retire l'entrée de chaque compte, et supprimer un compte retire la sienne.
- Claude (dans un terminal, un autre outil ou un agent d'Escouade) lit ce qu'Escouade sait, sans rien changer :
  - `list_projects` : les projets, leur branche, leur nombre d'agents et de tickets par colonne ;
  - `list_agents` : les agents (tous les projets ou un seul), leur projet, leur état, s'ils attendent une réponse, leur ticket, leur modèle, leur compte Claude et ce qu'ils ont coûté ;
  - `list_tickets` : les tickets d'un projet dans l'ordre du Kanban, avec leurs dépendances, leur agent et pourquoi ils sont bloqués ;
  - `get_ticket` : un ticket entier (description, critères, boucles, avancement, ticket Jira, Trello ou GitHub d'origine) ; un agent d'Escouade lit le sien sans le nommer ;
  - `get_usage` : le quota de chaque compte Claude, le compte où partent les nouveaux agents et la pause du pilote auto ;
  - `get_agent_summary` : la dernière réponse d'un agent (2 000 caractères au plus) et les fichiers qu'il a modifiés, jamais sa conversation.
- Claude agit sur Escouade comme tu le ferais dans la fenêtre :
  - `create_ticket` crée un ticket « À faire » (titre, description, critères, tickets dont il dépend), `update_ticket` le modifie champ par champ, `move_ticket` le place en tête, en bas ou avant un autre, `start_ticket` le lance comme « Lancer » ;
  - `create_agent` crée un agent dans un projet avec son premier message (dans un worktree ou non, avec le modèle de ton choix) sans changer l'agent que tu regardes ; `send_message` écrit à un agent, `stop_agent` interrompt son tour en cours.
  - Ces outils suivent les garde-fous de la fenêtre et du pilote auto : rien ne démarre quand aucun compte n'est utilisable pour le projet (pause du pilote auto, quota ou limite d'usage : le refus dit jusqu'à quelle heure) ; le maximum « En parallèle » du projet n'est pas dépassé ; un ticket qui attend d'autres tickets n'est pas lancé sans eux ; une dépendance qui ferait attendre deux tickets l'un l'autre est refusée ; un agent ne s'écrit pas et ne s'arrête pas lui-même, et on n'écrit pas à un agent archivé.
  - Aucun outil n'accepte une permission, ne lance une commande ou un test, ne merge ni ne supprime (projet, agent, ticket) : un agent poussé par un contenu piégé ne peut pas s'en servir pour monter en droits.
- Un texte qui vient du serveur n'est jamais pris pour toi : un message envoyé à un agent (`send_message`, premier message de `create_agent`) est lu par l'agent et montré dans sa conversation sous « Message de <auteur> : » (le nom de l'agent qui l'envoie, ou « Claude (hors Escouade) »), un ticket créé ou découpé par le serveur se termine par « Créé par <auteur> via Escouade », et un ticket que `update_ticket` modifie par « Modifié par <auteur> via Escouade » (qui remplace la ligne d'origine qu'il portait), sur sa carte et dans le protocole de son agent.
- Chaque requête doit porter un jeton : celui de Claude hors Escouade, gardé dans le trousseau du système (ou, s'il le refuse, dans un fichier que toi seul peux lire), ou celui d'un agent d'Escouade, propre à chacun et oublié quand son process s'arrête. Une requête sans jeton valide, envoyée par une page web ou adressée à un autre nom que 127.0.0.1 ou localhost est refusée.
- Réglages d'un projet › « Serveur MCP » › « Les agents peuvent utiliser Escouade » (désactivé par défaut) : ses agents reçoivent le serveur avec un jeton à eux, lisent les tickets et les agents, et peuvent créer des tickets ou lancer d'autres agents, ce qui dépense du quota. Un projet qui ne le permet pas leur refuse les outils d'Escouade, même quand Claude Code est réglé pour piloter Escouade. Ce réglage retire des outils aux agents et protège ton quota, ce n'est pas un cloisonnement : un agent qui peut les utiliser n'agit que dans son propre projet, mais lit les autres.
  - Deux outils sont réservés aux agents d'Escouade : `report_progress` (une ligne d'état de 120 caractères au plus, affichée sous le nom de l'agent dans la barre latérale et la vue d'ensemble, et sur la carte de son ticket « En cours », jusqu'à son prochain appel) et `split_ticket` (découpe ce qui reste de son ticket en 10 tickets au plus, créés « À faire » après le sien, à la suite l'un de l'autre ou tous en parallèle).
  - Une demande d'autorisation d'un outil d'Escouade le nomme en clair (« Escouade · Créer un ticket », « Escouade · Lancer un agent »…) et montre ce que Claude lui donne, un argument par ligne ; une valeur de plus de 2 000 caractères est coupée, la carte dit combien ne sont pas montrés et renvoie à la conversation. La vue d'ensemble l'envoie lire dans la conversation avant de répondre.
- Réglages › Claude Code › « Activité MCP » : les 200 derniers appels et refus, les plus récents en tête (heure, appelant, outil, résumé, résultat ; les refus et les erreurs en couleur d'alerte), mis à jour en direct, avec « Effacer ». Jamais de jeton.

#### Branches

- Le bouton de branche de la barre d'état ouvre un sélecteur de branche, au-dessus de la barre, dans tout dépôt git : recherche approchée, branches locales (la courante cochée, en tête) et distantes, avance et retard de chacune sur sa branche distante (« ↑2 ↓5 », aussi à côté du nom dans la barre), clavier complet (↑ ↓ Entrée Échap) et lecteurs d'écran. Les actions de synchro sont au pied sous les noms « Récupérer », « Pousser » (« Publier » sans branche suivie) et « Fetch ».
- Changer de branche d'un clic : une branche prise par l'agent d'un worktree est marquée « utilisée par l'agent X » et ne se choisit pas (la raison s'affiche au survol) ; avec des changements non commités, Escouade propose « Mettre de côté (stash) et changer » (le stash s'appelle « escouade: avant de passer sur <branche> », un toast le dit, et il est remis en place si le changement échoue) ; si un agent travaille dans le dossier du projet, il faut attendre la fin de son tour. Le changement est aussi refusé pendant un merge, un rebase, un cherry-pick ou un revert en cours, depuis une HEAD détachée dont les commits n'appartiennent à aucune branche, et quand des fichiers non suivis seraient écrasés (ils sont nommés). Une branche distante se prend par la branche locale qui la suit, créée au besoin.
- « Nouvelle branche… » : nom vérifié à la frappe comme le fait git, point de départ au choix (branche courante, autre branche, branche distante, ou le commit choisi dans le graphe), « et y passer » coché par défaut.
- Supprimer une branche (clic droit dans le sélecteur) : confirmation, puis, si son travail n'est pas dans la base du projet (merge, rebase, squash, ou pull request mergée et récupérée), une seconde qui dit combien de commits ne sont dans aucune autre branche ; la branche distante en option (« Supprimer aussi origin/… »), qui ne part que si c'est bien celle de la branche (même nom, aucune autre branche locale ne la suit) et qu'aucun commit poussé par quelqu'un d'autre depuis ton dernier fetch ne s'y trouve. Jamais la branche du dossier du projet ni celle d'un worktree.
- « Branches mergées… » liste les branches locales déjà dans la base du projet, toutes cochées, et les supprime d'un coup sans jamais forcer.
- Clic droit sur un commit du graphe git (ou touche Menu / Maj+F10 sur la ligne) : « Créer une branche ici… », « Passer sur <branche> » et « Supprimer la branche… » pour chaque branche locale du commit (grisées, avec la raison, si le worktree d'un agent la tient), « Comparer avec la branche courante » (« Comparer avec HEAD » sur une HEAD détachée), qui ouvre la fenêtre de diff sur ce qui change d'une branche à l'autre, titrée « main ↔ feat/x ».
- « Nouvel agent sur une branche… » (menu ▾ à côté de « + Nouvel agent ») et, au clic droit d'une branche du sélecteur, « Lancer un agent sur cette branche » : l'agent travaille dans un worktree sur une branche qui existe déjà, vers la même base qu'un agent ordinaire. Refusé pour la branche du dossier du projet, la branche cible du Kanban et celle d'un autre worktree. C'est ta branche : supprimer l'agent avec son worktree la conserve (« Supprimer aussi le worktree (la branche X est conservée) »), et le nom que Haiku donne à l'agent ne la renomme jamais.
- Un ticket peut reprendre une branche existante : champ « Branche » du formulaire (« Nouvelle branche ticket/<clé> » par défaut, ou « Reprendre une branche existante… »). Son agent y travaille et la validation la merge comme celle d'un ticket, mais Escouade ne la supprime jamais, quel que soit le réglage du Kanban : seul le worktree part. Si la branche est celle du dossier du projet, la branche cible du Kanban ou une branche prise par un agent au moment du départ, le ticket est bloqué avec la raison. Si l'ancien agent du ticket, archivé, tient encore la branche, son dossier est supprimé quand il n'a aucun changement non commité ; sinon rien n'est jeté et le ticket est bloqué avec ce qu'il faut faire.
- « Intégrer <base> » (liste des fichiers d'un agent à worktree, menu de l'agent) : la branche de base entre dans la branche de l'agent, par merge ou par rebase selon la stratégie du Kanban. Refusé pendant le tour de l'agent et avec des changements non commités dans son worktree. Un conflit de merge est laissé dans le worktree et l'agent reçoit la demande de le résoudre, avec les fichiers ; un conflit de rebase est annulé et l'agent reçoit la demande de rebaser.

#### Kanban : un ticket s'écrit dans une grande fenêtre

- Un nouveau ticket et la modification d'un ticket s'écrivent dans une grande fenêtre plutôt que dans la colonne : titre pleine largeur, description de 12 lignes qui grandit avec le texte, critères, boucles, branche et « Après » dessous.
- Ctrl+Entrée (Cmd+Entrée sous macOS) ajoute ou enregistre le ticket depuis n'importe quel champ ; le rappel est affiché au pied de la fenêtre.
- Échap, la croix ou un clic à côté demandent avant d'abandonner ce qui a été saisi, et « Annuler » ramène la fenêtre avec tout son contenu.

#### Avancée d'un agent qui exécute un plan

- Un bandeau « Plan » entre l'en-tête d'un agent et ses messages dit où il en est, sans lire la conversation : le titre du plan (sinon celui du ticket, sinon le nom de l'agent), « 2/7 tâches », le pourcentage, les sous-agents qui travaillent (« 3 sous-agents actifs », une pastille par sous-agent dont l'infobulle donne le nom) et une barre à un segment par tâche. Un clic ou le clavier le plie et le déplie : ouvert par défaut quand la fenêtre fait au moins 800 px de haut, puis au choix de chaque agent jusqu'au redémarrage de l'app.
  - La liste montre, pour chaque tâche, son rang, son état dit en toutes lettres (« Terminé », « En cours », « À faire », « Bloqué »), une barre et son avancement : 100 % quand elle est faite, 0 % quand elle reste à faire, et pour une tâche en cours la part des étapes cochées dans le plan (« 2/5 étapes » à l'infobulle) quand on la connaît, sinon « En cours » : Escouade n'invente jamais de pourcentage. Celui du plan est la moyenne de ces avancements.
  - Sous le titre d'une tâche : le sous-agent qui y travaille, avec son modèle (« ↳ Implémenter la tâche 2 · Haiku 5.5 »), ce qu'il fait en ce moment (« Édite src/a.ts ») ou, sans sous-agent, ce que dit la tâche elle-même (« Running tests »), et « après 03, 05 » pour une tâche qui en attend d'autres. Quand l'agent a posé une question ou demande une autorisation, sa première tâche en cours passe en « Bloqué » et dit « En attente de ta réponse ».
  - La tâche à regarder (la bloquée, sinon la première en cours, sinon la première à faire) est surlignée et la liste la centre toute seule, sauf si tu viens de la faire défiler. Une poignée sous la liste en règle la hauteur (souris ou clavier ; double-clic ou Entrée pour revenir à « auto »), retenue d'un lancement à l'autre ; les messages gardent toujours 160 px.
  - Sans liste de tâches mais avec des sous-agents en cours, le bandeau se réduit à la pastille « Sous-agents », leurs pastilles et leur compteur, et disparaît quand ils ont fini.
- Sur la carte d'un agent, dans la barre latérale : une mini-barre et « Plan 2/7 » ou « Plan 2/7 · 3 sous-agents ». La carte d'un ticket et la vue d'ensemble ne changent pas.
- Dans la conversation, les outils de tâches de Claude Code se lisent d'un coup d'œil : « Tâche : <sujet> », « Tâche #3 : terminée » ; lister ou relire les tâches reste discret.
- D'où vient la liste :
  - Du flux de l'agent : la liste qu'il tient avec les outils de tâches de Claude Code (`TodoWrite`, ou `TaskCreate` et `TaskUpdate`), qu'il ait un ticket ou non, et les sous-agents qu'il lance (en cours, finis, en échec, arrêtés), avec leur modèle, ce qu'ils font en ce moment, et la tâche sur laquelle ils travaillent quand on peut le dire (nommée dans leur consigne, sinon la seule en cours). Tout est gardé avec l'agent et retrouvé au redémarrage de l'app ; ce qui tournait quand son process s'est arrêté, ou quand son tour a été interrompu, n'est plus compté comme actif.
  - D'un plan superpowers (`writing-plans`, `executing-plans`, `subagent-driven-development`) : Escouade lit, en lecture seule et dans le dépôt de l'agent seulement, le plan (`docs/superpowers/plans/…` : titre, tâches « Task N: », étapes cochées) et son registre d'exécution (`.superpowers/sdd/<plan>/progress.md` : tâches faites ; tâches en cours d'après les briefs `task-N-brief.md` et les tours de correction). Les fichiers sont relus quand une commande, un sous-agent ou un tour se termine ; une lecture qui échoue garde le dernier état.
  - La liste que l'agent tient passe avant celle des fichiers, qui ne lui donnent alors que le titre et le nom du plan, quand ses tâches sont celles du plan. Un plan ou un registre qu'aucune activité n'a touché depuis le début de la conversation n'est pas le sien : une nouvelle conversation ne retrouve pas le plan fini d'une précédente. Quand l'agent supprime le dossier de travail du plan à la fin, comme les skills le disent, le plan passe à « terminé ».
  - La liste repart de zéro avec une nouvelle conversation, ou avec le premier message qui suit un plan terminé.
- Réglages › Claude Code › « Les agents tiennent une liste de tâches » (activé par défaut) lance chaque agent avec les outils de liste de tâches de Claude Code, que les modèles actuels n'ont pas sans cela (`CLAUDE_CODE_ENABLE_TODO_TOOLS=1`) ; il vaut pour les agents lancés ensuite, pas pour le nom d'un agent, le message de commit ni la connexion d'un compte. Ces outils prennent un peu de place dans le prompt de chaque agent. Désactivé, Escouade n'ajoute rien et ne force jamais la valeur contraire : la variable de ton environnement, si tu en as une, s'applique.
- En français et en anglais, avec le changement de langue à chaud ; tout au clavier, focus visible, un message poli pour les lecteurs d'écran, mouvement réduit respecté.
- Limites :
  - Le réglage rend la liste possible, pas certaine : dans Claude Code, ces outils sont différés, le modèle doit d'abord les chercher. Sur les trois plans superpowers essayés avec le réglage actif (Claude Code 2.1.296, superpowers 6.4.1, Sonnet 5.5 à effort bas, `subagent-driven-development`), il ne l'a jamais fait : le bandeau venait du plan et de son registre. Sans liste ni fichiers de plan, l'agent n'a pas de bandeau, seulement celui des sous-agents en cours. La variable vient du binaire de Claude Code, pas de sa documentation, et peut changer : sans elle, le bandeau continue avec les sous-agents et les fichiers.
  - Les fichiers : le registre est lu à la lettre (`Task <id>: …` en début de ligne) ; un plan sans registre se montre avec ses tâches « à faire », rien dans les fichiers ne disant où l'on en est (les sous-agents se voient quand même) ; un contrôleur qui n'écrit pas la dernière ligne du registre ni ne supprime le dossier laisse le plan à « 1/2 » ; plusieurs agents sans worktree dans un même dépôt lisent le même dossier de travail, donc l'un peut montrer le plan de l'autre (un agent à worktree a le sien).
  - Les workflows de Claude Code ne sont pas montrés : un plan qui n'est fait que d'un workflow n'a pas de bandeau.
  - Vérifié sur de vrais flux (Claude Code 2.1.296, superpowers 6.4.1) : les outils de tâches, les sous-agents en arrière-plan (tous ceux qu'il a lancés l'étaient) et les fichiers. Pas vus sur un vrai flux : les sous-agents au premier plan et les workflows, dont la prise en charge repose sur le binaire et sur le faux `claude` des tests.

### Modifications

- La barre d'état montre les quotas du compte en cours sous une forme compacte : `5h <barre> reset 3h01 | 7j <barre> reset 4j 12h` (en anglais `5h … | W … reset 4d 12h`). Le pourcentage n'est plus écrit : il s'affiche, avec la date de remise à zéro (« 42 % · remise à zéro le 10/10 à 18:00 »), dans une infobulle au survol de la barre et à son focus. La barre reste un indicateur lisible par les lecteurs d'écran et passe à la couleur d'alerte au-delà de 80 %. Avec plusieurs comptes, le nom du compte en cours précède les quotas (« Pro · 5h … »).

### Corrections

- Récupérer (pull) et pousser (push) lisent les messages de git en anglais, quelle que soit la langue du système : un refus (« would be overwritten », « [rejected] ») est reconnu et expliqué même avec un git en français.
- Dans l'éditeur, un projet ou un agent introuvable n'est plus pris pour un fichier absent : l'onglet montre l'erreur au lieu de « Ce fichier n'existe pas (ou plus) ».
- « L'agent a 2 fichiers non commités » au lieu de « 2 fichier(s) non commité(s) », « 1 tâche » au lieu de « 1 tâches », « … et 1 autre fichier » au lieu de « … et 1 autres fichiers ».
- Sous macOS, les chemins de ton dossier personnel s'affichent raccourcis en `~`, comme sous Windows.
- Les infobulles de recherche des terminaux et des lancements disent « Ctrl+Maj+F » en français.
- Dans l'import de tickets, le nombre de tickets cochés est groupé par milliers (« 1 234 »), à l'écran et pour les lecteurs d'écran.
- Sous macOS, quand tu refuses à Escouade l'accès au trousseau pour lire ta connexion à Claude Code, la demande ne revient plus toutes les minutes : Escouade attend 10 minutes avant de redemander.

### Limites connues

- Sous macOS, le nom de l'entrée du trousseau où Claude Code range la connexion d'un compte ajouté est lu dans son binaire, pas vérifié sur un Mac avec un second compte connecté : un compte ajouté pourrait s'y afficher « Pas connecté » alors qu'il l'est.
- La reprise sur un autre compte n'est pas vérifiée avec un second compte réel : que l'API accepte une conversation commencée sur un autre compte. Si elle la refuse, c'est le cas que la carte sait dire et défaire (« Revenir sur Principal ») ; c'est pourquoi la reprise automatique est désactivée par défaut.
- Les agents peuvent ne pas se servir des outils de liste de tâches, même quand le réglage est actif (ils sont différés dans Claude Code) : le bandeau « Plan » vient alors des fichiers d'un plan superpowers et des sous-agents.

## [1.6.0] — 2026-10-10

### Ajouts

#### Éditeur : aller à une définition, et revenir

- Ctrl+clic (Cmd+clic sous macOS) ou F12 suit ce qui est sous la souris ou sous le curseur, souligné tant que la touche est enfoncée :
  - un import relatif ou aliasé (`tsconfig.json` / `jsconfig.json`, `$lib`) en JS, TS, Svelte et Vue, un `@import` ou un `url()` de feuille de style, un `mod x;` de Rust, un import Python, un lien relatif de Markdown ou de HTML, et dans n'importe quel fichier un chemin de l'arborescence, suivi ou non de `:ligne:colonne` ;
  - un nom (variable, fonction, classe, type…), jusqu'à sa définition, sans serveur de langage : l'éditeur la cherche dans le fichier (JS/TS, Svelte, Vue, Rust, Python, Go), puis dans le fichier d'où le nom est importé, sinon dans les fichiers du même langage.
- Plusieurs définitions trouvées : une liste s'ouvre sous le mot (« chemin:ligne » et la ligne), à parcourir avec ↑ ↓ et Entrée. Aucune : « Aucune définition trouvée pour « X ». » ; un fichier absent : « Fichier introuvable : <chemin> ».
- Alt+← et Alt+→ (⌃- et ⌃⇧- sous macOS, avec la touche Contrôle), ou les boutons « précédent » et « suivant » de la souris, te ramènent là où tu étais avant chaque saut, puis t'y renvoient. Ces touches ne valent que dans l'éditeur : dans un terminal ou le composer, elles restent au shell.

#### Éditeur : rechercher dans les fichiers, ouvrir un fichier par son nom

- La colonne de gauche a deux vues, « Fichiers » et « Rechercher dans les fichiers ». Ctrl+Maj+F (Cmd+Maj+F sous macOS) ouvre la recherche avec le texte sélectionné dans le code, s'il tient sur une ligne. Elle part dès que tu arrêtes de taper, dans les fichiers de la source affichée (le projet ou le worktree de l'agent), et repart quand tu changes de source. Trois options : « Respecter la casse », « Mot entier », « Expression régulière ».
- Les résultats sont groupés par fichier, chaque correspondance surlignée dans sa ligne. En bas : « N résultats dans M fichiers », « Résultats limités aux 2 000 premiers. » ou « Recherche arrêtée après 10 s : résultats partiels. ». Un clic ou Entrée ouvre le fichier à la ligne et à la colonne, et Alt+← te ramène où tu étais ; au clavier, ↓ depuis le champ, ↑ ↓ entre les résultats, → et ← pour déplier ou replier un fichier. La recherche est gardée quand tu fermes l'éditeur.
- Maj+F12 cherche le nom sous le curseur dans tous les fichiers de la source, en mot entier et en respectant la casse.
- Ctrl+P (Cmd+P sous macOS) ouvre « Ouvrir un fichier » : quelques lettres du nom ou du chemin, dans l'ordre, suffisent ; `nom:42` ouvre le fichier à la ligne 42. Champ vide, tes derniers fichiers ouverts passent devant. La liste porte sur la source de l'éditeur s'il est ouvert, sinon sur le worktree de l'agent sélectionné, sinon sur le projet ; Alt+← revient à l'endroit que tu viens de quitter.

#### Éditeur : voir les changements, comparer avec le disque

- « Voir les changements », à droite de « N lignes modifiées vs main », montre dans le texte les lignes supprimées au-dessus de chaque bloc modifié, et tu peux continuer à écrire ; le bouton reste enfoncé tant que les changements sont affichés, et le même bouton les masque. « Annuler ce bloc » remet le bloc de la référence, sans enregistrer (Ctrl+Z le reprend). Pas de bouton pour un nouveau fichier. Même dans un gros fichier, chaque changement d'un lockfile a son bloc, et taper dans un long bloc réécrit reste instantané.
- Quand un fichier que tu modifies change sur le disque, « Comparer » montre bloc par bloc ce que le disque a de différent, sans toucher à ton texte (« Comparaison avec la version du disque. »). « Prendre ce bloc » copie un bloc du disque, puis « Garder ma version » enregistre le résultat ; « Recharger » prend la version du disque.
- Si l'agent écrit encore le fichier pendant la comparaison, le bandeau et un message te le disent (« Le fichier a encore changé sur le disque : la comparaison montre sa nouvelle version. »), aussi quand tu reviens sur son onglet, et « Garder ma version » attend une seconde : tu n'écrases jamais une version que tu n'as pas vue. Si le fichier est revenu entre-temps à la version que tu avais ouverte : « Rien n'a été enregistré : le fichier est revenu à la version que tu avais ouverte. ».

#### Éditeur : arborescence, onglets et colonne

- Clic droit sur un fichier ou un dossier › « Renommer… » (ou F2 sur la ligne) : le nom se modifie sur place, extension non sélectionnée ; `dossier/nom.ts` le déplace dans un sous-dossier créé au besoin (refusé par le disque, il ne laisse pas de dossier vide derrière lui), et changer seulement la casse est permis pour un fichier ou un dossier que git ne suit pas (pour un fichier suivi, passe par `git mv` dans un terminal : « git suit « App.ts » sous ce nom : pour n'en changer que la casse, passe par git mv dans un terminal. »). Les onglets du fichier, ou des fichiers du dossier, le suivent sans perdre tes modifications non enregistrées ni ton historique d'annulation ; Alt+← et Ctrl+P suivent aussi. Renommer un gros dossier ne retient pas le reste de l'app. Renommer un `.env` copié en un nom que git n'ignore pas te prévient : il peut alors être commité.
- « Supprimer » (ou Suppr sur la ligne, ⌘⌫ sous macOS) envoie le fichier ou le dossier à la corbeille du système, après confirmation (« Supprimer « nom » ? », « Le dossier et ses N fichiers partent dans la corbeille. »). Un fichier concerné qui a des modifications non enregistrées te demande d'abord « Enregistrer » ou « Ne pas enregistrer », et un fichier dans lequel tu tapes pendant la suppression garde son onglet et ton texte. Le dossier d'un autre projet du même dépôt, un dossier qui le contient et les worktrees des agents ne se renomment ni ne se suppriment.
- « Nouveau dossier… » au clic droit, et « Nouveau dossier » en tête de l'arborescence : le dossier créé s'affiche, même vide, jusqu'à ce que tu y crées un fichier.
- « Ouvrir un terminal ici » au clic droit ouvre un terminal dans ce dossier (celui du fichier, ou la racine depuis l'espace libre), dans la source affichée, nommé d'après le dossier.
- L'arborescence montre en grisé, avec l'infobulle « Ignoré par git », les fichiers ignorés qui correspondent aux « Fichiers copiés dans les worktrees » du projet (`.env*` par défaut), à la racine du projet (ou de son sous-dossier dans le dépôt) comme dans les worktrees : tu peux les ouvrir, les modifier et les retrouver avec Ctrl+P. Aucun autre fichier ignoré n'apparaît (pas de `node_modules`), et ceux-là restent hors des commits.
- La colonne Fichiers / Recherche se redimensionne en glissant la poignée entre elle et le code (de 160 px à la moitié de l'éditeur), ou au clavier : ← et → par 16 px, Début et Fin jusqu'aux bornes. Un double-clic sur la poignée revient à 240 px. La largeur vaut pour tous les projets.
- Quand deux onglets ouverts ont le même nom (`index.ts`), chacun montre en gris le plus court dossier qui les distingue (« index.ts · editor », « index.ts · board ») ; l'infobulle de l'onglet donne toujours le chemin complet.

#### Conversations

- Tu réponds au clavier : Ctrl+Entrée (⌘Entrée sous macOS) autorise, Ctrl+Maj+Entrée (⇧⌘Entrée) autorise toujours quand Claude propose une règle ; Alt+1 … Alt+9 choisit l'option d'une question (coche ou décoche pour un choix multiple), et Ctrl+Entrée valide quand la réponse est complète. Les boutons affichent leur raccourci. Ces touches marchent dans la conversation et son champ de saisie, pas dans un terminal ni dans l'éditeur, et jamais avec AltGr ; avec plusieurs demandes en attente, elles répondent à la plus ancienne. Écrire un texte puis Entrée refuse toujours avec ce message, et Ctrl+J mène toujours au champ de saisie de l'agent qui attend. Une demande qui vient d'arriver, ou qui prend la place de celle que tu viens de régler, ignore ces touches une demi-seconde, Entrée dans le champ de saisie comprise : un message tapé à ce moment-là ne l'autorise ni ne la refuse sans que tu l'aies lue, et il reste dans le champ. Une demande que Claude Code refuserait par défaut ne s'autorise qu'avec le bouton « Autoriser ».
- « Rechercher dans les conversations » (Ctrl+K, Cmd+K sous macOS) cherche dans ce que toi et tes agents avez écrit, les commandes lancées, les fichiers lus ou modifiés et ce que les outils ont répondu, sans tenir compte de la casse ni des accents (« evenement » trouve « événement »). « Ce projet » ou « Tous les projets », avec ou sans les « Agents archivés » (inclus par défaut). Les résultats sont groupés par agent, avec son projet, la correspondance surlignée et la date ; 200 au plus. Dans un terminal sous Windows, Ctrl+K reste au shell.
- Ouvrir un résultat affiche l'agent et fait défiler sa conversation jusqu'au message, surligné 2 s, même s'il est plus ancien que ceux affichés ; un agent archivé s'ouvre en lecture.
- Le texte que tu as commencé à écrire pour un agent est gardé : tu le retrouves après un redémarrage, une mise à jour ou un rechargement de la fenêtre. Il est vidé à l'envoi et supprimé avec l'agent ; les fichiers joints ne sont pas gardés.
- Quand tu reviens sur un agent, la conversation est remise là où tu l'avais laissée, messages plus anciens affichés compris ; si tu étais en bas, elle reste en bas et suit les nouveaux messages.

#### Agents

- « Vue d'ensemble » : le bouton à gauche des onglets de projets, ou Ctrl+Maj+A (Cmd+Maj+A sous macOS), même depuis un terminal, remplace la fenêtre par la liste de tous tes agents, groupés par projet. En tête, « Attend ta réponse » rassemble ceux qui ont une question ou une autorisation en attente, dans tous les projets, le plus ancien d'abord.
  - Chaque ligne montre le statut, le ticket (« ▸ DEM-1 · boucle 2/5 »), ce que fait l'agent (« Lit src/db.ts »), son modèle, le coût de sa session, son contexte utilisé (%) et depuis quand.
  - Une autorisation en attente montre l'outil et sa commande, sur 4 lignes au plus, avec la description et la raison de la demande, et « Autoriser » et « Refuser » sur place. Ce que la vue ne peut pas montrer en entier (une commande trop longue, un outil MCP, une demande sans argument), comme une demande que Claude Code refuserait par défaut, se lit dans l'agent : « Répondre » l'ouvre. Une question montre son texte, et « Répondre » ouvre l'agent.
  - Une ligne qui vient de bouger sous ton curseur (celle du dessus vient d'être réglée, ou une autre demande a pris la place) attend une demi-seconde avant d'accepter un clic, et les caractères invisibles ou de direction d'une commande sont écrits en clair (« ⟨U+202E⟩ ») : tu n'autorises jamais sans avoir lu.
  - Clic ou Entrée sur une ligne ouvre l'agent dans son projet ; ↑ ↓ passent d'une ligne à l'autre ; Échap ou le bouton reviennent là où tu étais.
- Clic droit sur un agent › « Dupliquer la conversation » crée « <nom> (copie) » (puis « (copie 2) »…) dans le même projet, avec le même modèle, le même effort et le même mode de permission. La copie montre la conversation de l'original, et Claude Code y reprend tout son contexte dans une nouvelle session, telle qu'elle était au moment de la copie : l'original n'est pas touché, et les tours que tu lui fais faire ensuite ne sont pas dans la copie. Seule exception, la copie d'un agent qui n'a fait aucun tour depuis cette mise à jour, ou qui vient de compacter sa conversation, reprend sa session telle qu'elle est au premier lancement de la copie. Pendant un tour de l'agent, l'entrée est désactivée (« Attends la fin de son tour. »).
  - Un agent avec worktree donne une copie avec son propre worktree, sur une nouvelle branche `escouade/<nom>-copie` partie du commit où en est l'original, et préparé comme celui d'un nouvel agent ; Claude y est prévenu qu'il travaille dans ce nouveau dossier. Si l'original a des modifications non commitées, la conversation de la copie le dit : « Les modifications non commitées de <original> ne sont pas dans cette copie. ». Sans worktree, la copie travaille dans le même dossier.
  - Un agent de ticket se duplique en agent ordinaire, sans ticket.
- Clic droit sur un agent › « Ouvrir un terminal » ouvre un terminal dans son worktree (dans le dossier du projet pour un agent sans worktree), nommé d'après son shell et l'agent (« pwsh · refacto-auth »).

#### Notifications

- Réglages › Notifications › « Me prévenir pour » : « Questions et autorisations », « Tâches terminées », « Erreurs » et « Tickets (prêt à tester, bloqué) », tous activés par défaut. Désactivé, un type n'a plus ni notification système ni carillon ; l'onglet, la carte et la barre des tâches (le Dock) signalent toujours l'agent.

#### Kanban

- Réglages › Kanban › Pilote auto : « Pause au-delà du quota » (« 80 % », « 90 % », « 95 % » ou « 100 % », 100 % par défaut, pour tous les projets). Aucun ticket ne démarre tant que la fenêtre de 5 h ou la fenêtre hebdomadaire dépasse ce seuil ; la file repart d'elle-même 30 secondes après la fin de la fenêtre, ou dès que les quotas lus repassent sous le seuil.
- Sous son en-tête, le Kanban dit pourquoi le pilote auto est en pause : « Pilote auto en pause : quota hebdo à 96 % (reprise à 14:00) », « … quota de 5 h à 100 % (reprise à 14:00) » ou « … limite d'usage atteinte (reprise vers 14:30) », avec « Reprendre maintenant ». Les cartes « À faire » disent alors « En attente : pilote auto en pause ». La pause tient après un redémarrage de l'app.
- Un ticket peut attendre d'autres tickets du projet : dans son formulaire, « Après » liste ceux qui ne sont pas terminés (clé et titre), avec « Rechercher une clé » (la clé Jira, Trello ou GitHub d'un ticket importé marche aussi). Le pilote auto passe au suivant de la file tant qu'un ticket attendu n'est pas « Terminé » ; la carte dit « ⏸ après DEM-3 ». Un ticket qui attend déjà celui que tu modifies est refusé (« DEM-3 attend déjà DEM-5 (directement ou non). »), « Lancer » sur un ticket qui attend demande « DEM-5 attend DEM-3, pas encore terminé. Le lancer quand même ? », et supprimer un ticket le retire des tickets qui l'attendaient.
- Supprimer un ticket « À faire » demande d'abord confirmation (« Supprimer DEM-1 ? ») ; pour un ticket importé, le message précise qu'il ne sera plus importé depuis Jira, Trello ou GitHub. Échap ou « Annuler » dans le formulaire d'un ticket que tu as modifié demandent « Abandonner les modifications ? » ; sans changement, le formulaire se ferme tout de suite.
- Les cartes « En cours » et « À tester » montrent le coût cumulé de tous les agents du ticket, archivés compris : exact pour les tours finis, avec un « ≈ » tant qu'un tour tourne, comme la barre de statut. Dans une colonne étroite, le coût et « n/N critères » (ou « n boucles · coût ») passent sous le nom de l'agent, qui garde sa place.
- Une synchro ratée avec Jira, Trello ou GitHub (changement de statut, commentaire) n'est plus perdue : Escouade la garde, même si tu quittes l'app. Si le service est injoignable ou en panne, elle la retente 1, 5, 15 puis 60 minutes plus tard, puis toutes les heures pendant 24 h, et à chaque démarrage. Le ticket externe reçoit ses changements dans l'ordre, et un changement de statut plus récent remplace celui qui n'est pas passé ; un ticket qui revient en arrière (vers « À faire », par exemple) abandonne aussi celui qui n'est pas passé, alors qu'un ticket passé à une colonne suivante qui laisse le statut « — inchangé » le garde. Un refus qui ne changera pas (ticket externe introuvable, transition impossible dans Jira…) est mis de côté sans bloquer les synchros suivantes du ticket.
- Clique sur le ⚠ d'une carte pour lire la raison de l'échec et « Resynchroniser », qui retente tout de suite, y compris ce qui a été mis de côté. Quand tout passe, le ⚠ disparaît ; sinon, un message dit pourquoi.
- « ⤓ Importer » : « Afficher plus » en bas de la liste quand Jira ou GitHub ont d'autres tickets que les 50 premiers, et « 50 affichés sur 312 » quand le service compte plus de tickets que la liste n'en montre.

#### Statistiques

- Une section « Par agent » (les agents de la période, archivés compris, du plus cher au moins cher : nom, projet, tokens, coût) et une section « Par ticket » (clé, titre, nombre de boucles et coût de ses agents sur la période, archivés compris : un ticket repris par un autre agent compte les deux). Elles suivent la plage choisie (Jour, Semaine, Mois) et montrent 20 lignes, puis « Tout voir ».

#### Git

- Réglages › Projets › Git : « Commit » choisit qui écrit les commits de « Commit… » et « Commit tout… ». « Rédigé par l'agent » (par défaut) les demande à l'agent, comme avant. Avec « Direct, avec un message proposé », une fenêtre « Commit » s'ouvre, même sans agent sélectionné :
  - elle liste les fichiers concernés : ceux de l'agent (son worktree, sinon ses fichiers dans le dossier du projet), ou tout le dossier du projet, sans les worktrees ;
  - Haiku y propose un message dans le style des derniers commits du dépôt (« Haiku rédige le message… », et tu peux écrire pendant ce temps) ; tu le relis, le modifies ou le fais « Régénérer », puis tu cliques sur « Commiter » : rien n'est jamais commité sans ce clic ;
  - les fichiers copiés dans les worktrees (`.env*`) ne sont jamais commités, même si l'agent ne les ignore plus dans son worktree, et Haiku ne lit jamais le contenu d'un fichier qui correspond à ces motifs ;
  - si git refuse le commit (un hook, par exemple), son message s'affiche dans la fenêtre, qui ne se ferme pas pendant le commit, et ce que tu avais indexé reste tel quel ;
  - Échap, un clic à côté ou « Annuler » sur un message que tu as écrit (pas la proposition telle quelle) demandent « Abandonner le message ? » ; « Annuler » te ramène au commit, ton message intact.
- Un diff de plus de 1 500 lignes est replié : « Diff volumineux (N lignes) » et « Afficher », puis « Afficher 500 lignes de plus » pour le lire par tranches, sans figer la fenêtre.

#### Lancement de test : la recette lue avant de tourner

- « ▶ Tester » montre d'abord la recette que l'agent a écrite : « Préparation » (commandes et dossier), « Lancement » (nom, commande, dossier, variables et adresse de chaque processus) et « Ouverture » (l'adresse qui s'ouvrira), avec « Ces commandes ont été écrites par <agent>. Elles tournent dans ton shell, hors du mode de permission de Claude Code. ». Rien ne tourne avant « Lancer » ; « Annuler » ferme la fenêtre.
- Avec isola, « ▶ Tester » montre le `.isola.toml` du worktree sous « Configuration isola (.isola.toml) » (un fichier que l'agent peut écrire ou modifier, et dont `isola up` lance les services et les commandes `setup` dans ton shell) et l'adresse à ouvrir : rien ne tourne avant « Lancer ».
- Une recette ou un `.isola.toml` que tu as approuvés se lancent directement la fois suivante ; dès que l'agent en donne un autre (une commande, une variable, un dossier, une ligne du fichier ou l'adresse qui change), il est montré de nouveau. L'approbation est gardée avec l'agent, et l'app refuse de lancer une étape d'une recette, ou `isola up`, que tu n'as pas approuvés. Si la recette change pendant que tu la lis : « La recette vient de changer : relis-la avant de lancer. », et « Lancer » reste fermé une seconde.
- Ce que tu lis est ce que le shell lit : tout caractère qui ne dessine rien (retour chariot, séquence d'échappement, espace de largeur nulle, sélecteur de variation…) est écrit en clair (« ⟨U+FE0F⟩ »), le texte est affiché dans l'ordre d'exécution (lettres de droite à gauche comprises), et les longues suites de lignes vides ou d'espaces sont comptées (« ⟨12 lignes vides⟩ ») au lieu de repousser la fin d'une commande hors de la fenêtre.

#### Lancement et worktrees

- « ✦ Remplir automatiquement » existe aussi pour les commandes de lancement : dans les réglages du projet, groupe « Lancement », Claude lit le projet (manifestes, scripts, docker-compose, README…) sans rien modifier et propose les commandes à lancer pour développer (serveurs de dev, watchers, workers, base locale…), chacune avec son nom, son sous-dossier et sa commande pour ton shell par défaut. La proposition s'affiche d'abord en entier sous le bouton ; « Remplacer les commandes » la met à la place des commandes du brouillon (à relire avant d'enregistrer, « Annuler » les jette), « Ignorer » les laisse telles quelles.
- Quand la section « Lancement » de la barre latérale n'a aucune commande, « ✦ Proposer des commandes » ouvre les réglages du projet sur le groupe « Lancement » et lance la lecture tout de suite.
- Pendant la préparation d'un worktree, « Voir la sortie » sous « Préparation du worktree » déplie la sortie en direct de l'étape en cours (ses 500 dernières lignes, avec son rang et son nom : « 2/3 · npm ci »), qui défile toute seule tant que tu restes en bas.
- La sortie complète de chaque étape de préparation va dans un fichier de log de l'agent ; si une étape échoue, la conversation montre toujours ses dernières lignes et donne le chemin du log (« Sortie complète : … »).
- Réglages › Projets › Worktrees : les commandes de préparation et de démontage se réordonnent, avec « Monter » et « Descendre » sur chacune, ou Alt+↑ / Alt+↓ depuis l'un de ses champs.

#### Mises à jour

- Les mises à jour se téléchargent en arrière-plan : la barre de statut affiche « Mise à jour X… », puis « Mise à jour X prête · Redémarrer ». Un clic ouvre les notes de la version, avec « Redémarrer maintenant » et « Plus tard ». Si des fichiers ne sont pas enregistrés, la fenêtre te demande d'abord de les enregistrer (« Enregistre d'abord tes fichiers : N fichiers ne sont pas enregistrés. ») ; elle te dit aussi ce que le redémarrage fait aux agents qui travaillent ou attendent ta réponse : leur tour en cours est interrompu, les agents de ticket reprennent d'eux-mêmes, les autres attendent ton prochain message.
- Escouade redémarre d'elle-même pour se mettre à jour quand aucun agent ne travaille, qu'aucune question n'attend, que rien ne se prépare ni ne se valide (worktree, ticket qui démarre, validation, merge, commit), qu'aucune commande de lancement ne tourne, qu'aucun terminal n'a servi depuis 5 minutes, qu'aucune boîte de dialogue n'est ouverte, que tout est enregistré et que tu n'as pas touché l'app depuis 5 minutes (ou que sa fenêtre est cachée). Une notification « Escouade redémarre pour se mettre à jour » et un compte à rebours dans la barre de statut te préviennent 30 secondes avant, avec « Plus tard » ; ta première touche ou ton premier mouvement de souris pendant ce compte à rebours l'annule aussitôt. Réglage « Installer les mises à jour automatiquement » dans « À propos », activé par défaut. Sous macOS, si remplacer l'app demande le mot de passe d'un administrateur, elle ne redémarre pas d'elle-même : « Redémarrer maintenant » reste là.
- Redémarrée d'elle-même, l'app revient comme elle était : cachée si sa fenêtre l'était, réduite dans la barre des tâches (le Dock) si elle l'était, sans prendre le focus si elle était derrière une autre app.
- Quitter Escouade avec une mise à jour prête l'installe, sans relancer l'app (sous macOS aussi avec Cmd+Q ou le Dock). Pendant une installation déjà lancée, Cmd+Q attend qu'elle finisse au lieu de fermer l'app en plein remplacement ; si elle attend le mot de passe d'un administrateur, l'app se ferme tout de suite, sans rien installer.
- Après une mise à jour, « Escouade X est installée. » avec « Voir les nouveautés ». Une mise à jour qui n'a pas pu s'installer ne relance plus l'app d'elle-même : « La mise à jour vers X n'a pas pu s'installer. » avec « Réessayer ». Ces deux boutons n'ouvrent jamais leur fenêtre par-dessus une autre (le message revient pour plus tard, et un clic dessus laisse le focus dans la fenêtre ouverte), et une mise à jour qui n'est plus prête le dit : « La mise à jour vers X n'est plus prête : Escouade te la proposera de nouveau une fois téléchargée. ».

### Modifications

- Éditeur : Alt+clic (Option+clic sous macOS) ajoute un curseur, à la place de Ctrl+clic (Cmd+clic), qui sert maintenant à la navigation.
- Éditeur : la colonne de la barre d'état compte en caractères (un emoji en vaut un), comme les liens `fichier:ligne:colonne` et les résultats de recherche.
- Éditeur : créer un fichier qui existe déjà et que git ignore le dit : « existe déjà à cet endroit (ignoré par git) ».
- Ctrl+P et Ctrl+Maj+P n'ouvrent plus jamais la boîte d'impression de la fenêtre, même quand tu tapes dans un champ ou qu'une boîte de dialogue est ouverte ; dans un terminal sous Windows, Ctrl+P reste au shell (commande précédente).
- Les notifications disent de quoi il s'agit : « Autoriser Bash : npm test ? » (la commande, le fichier, l'adresse…), « Approuver le plan : <première ligne du plan> ? », le texte de la question, la première ligne de la réponse de l'agent (« Tâche terminée » s'il n'a rien écrit) ou « Erreur : <raison> », en 120 caractères au plus. Le code entre accents graves est repris tel quel.
- Quand un agent attend plusieurs réponses, ses demandes restent dans l'ordre où elles sont arrivées : la conversation, Ctrl+Entrée et la vue d'ensemble répondent d'abord à la plus ancienne.
- La carte d'une demande que Claude Code refuserait par défaut fait de « Refuser » (ou « Continuer à planifier ») son bouton principal.
- Une longue conversation s'ouvre plus vite : elle n'affiche que ses 80 derniers éléments. « Afficher les N précédents », en haut, ajoute les 80 d'avant, autant de fois que tu veux, sans faire sauter ta lecture : le message que tu lisais reste à sa place. Les lecteurs d'écran entendent « 80 messages précédents affichés », et le focus reste sur le bouton pour la tranche suivante.
- La conversation d'un agent au repos que tu n'as pas affiché depuis 10 minutes quitte la mémoire de la fenêtre ; elle se relit quand tu reviens dessus, à l'endroit où tu l'avais laissée. Celle d'un agent qui travaille ou attend ta réponse reste toujours en mémoire.
- Un message envoyé pendant la préparation du worktree s'affiche tout de suite avec « En attente de la préparation… », au lieu de disparaître jusqu'à la fin de la préparation. Un deuxième message envoyé avant la fin le dit (« Un message attend déjà la fin de la préparation. ») et reste dans le champ, au lieu d'être ignoré sans un mot.
- Les jetons de Jira et de GitHub, la clé et le jeton de Trello vont dans le trousseau du système (Gestionnaire d'identification Windows, Trousseau macOS) ; `~/.escouade/integrations.json` ne garde que le reste des comptes. Au premier démarrage, ceux que le fichier contient y sont déplacés, et n'en sont retirés qu'une fois relus dans le trousseau. Revenir ensuite à une version plus ancienne d'Escouade demande de reconnecter les comptes.
  - « Déconnecter » les efface du trousseau ; s'il n'y arrive pas, un message le dit (« Le jeton Jira n'a pas pu être retiré du trousseau du système : Escouade réessaiera au prochain démarrage, ou retire-le à la main (son nom contient « escouade »). ») et le démarrage suivant le retire, sans jamais toucher aux jetons des autres comptes.
  - Si le trousseau est indisponible, le jeton reste dans le fichier (lisible par toi seul sous macOS), et l'onglet Intégrations le dit sous le compte : « Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. » ; le démarrage suivant retente.
  - Si le trousseau ne rend pas le jeton d'un compte au démarrage, l'onglet le dit (« Trousseau du système illisible : relance Escouade ou reconnecte le compte. ») et aucun appel ne part sans lui : un compte GitHub avec son propre jeton n'utilise pas celui de `gh` à sa place.
- La liste des fichiers non commités n'affiche que les 500 premiers, puis « … et N autres fichiers » ; le compte de l'onglet, « Voir le diff » et les commits les prennent tous.
- Un fichier dont le diff dépasse 4 Mo (lockfile réécrit, dump commité) n'est plus transmis : il reste dans la liste et affiche « Diff trop volumineux pour être affiché. », dans le panneau, dans « Voir le diff » et dans le détail d'un commit.
- Réglages › Projets › Worktrees : « ✦ Remplir automatiquement » ne remplace plus directement les commandes de préparation et de démontage. Il montre d'abord la proposition de Claude en entier, sous le bouton : chaque commande jusqu'au bout, avec son shell et son dossier, ses caractères invisibles écrits en clair, et quand chaque liste tourne seule (la préparation à l'ouverture de chaque nouveau worktree, le démontage avant sa suppression). « Remplacer les commandes » la prend, « Ignorer » laisse les commandes telles quelles. Le groupe « Lancement » fait de même.
- Les deux boutons « ✦ Remplir automatiquement » des réglages d'un projet (Worktrees et Lancement) sont décrits chacun par le texte de leur rangée pour les lecteurs d'écran.
- Les commandes que « ✦ Remplir automatiquement » propose, pour le lancement comme pour la préparation et le démontage des worktrees, sont écartées quand leur sous-dossier sort du projet ou n'existe pas, quand elles tiennent sur plusieurs lignes ou contiennent un caractère invisible, quand elles comptent 24 espaces de suite ou plus, ou quand elles dépassent 300 caractères : ce que tu relis dans les réglages est ce qui sera lancé. Le message compte celles qui ont été écartées (« 2 commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant d'enregistrer. ») ; si toutes l'ont été : « Claude a proposé des commandes illisibles : aucune n'a été gardée. ». Un nom avec des caractères invisibles est nettoyé.
- L'installation d'une mise à jour n'ouvre plus aucune fenêtre d'installeur : plus d'impression de désinstaller puis réinstaller l'app, juste un redémarrage.

### Corrections

- « Voir le diff » en portée « Tout le projet » montre exactement les fichiers de la liste : ceux du dossier du projet et ceux du worktree de chaque agent, chacun étiqueté avec le nom de son agent. Avant, un fichier modifié seulement dans un worktree n'apparaissait pas, et la vue pouvait afficher « Aucune différence. » alors que la liste n'était pas vide. Un worktree que git n'a pas pu lire est dit (« Diff du worktree de « refacto-auth » non lu : … ») au lieu d'être laissé de côté sans rien dire ; un worktree supprimé entre-temps (validation, archivage) est simplement laissé de côté.
- « Voir le diff » d'un agent sans worktree qui a créé des centaines de fichiers garde les modifications de ses fichiers suivis : sous Windows, la ligne de commande trop longue les faisait disparaître du diff.
- « Merger <branche> → <base> » merge dans la branche de base de l'agent, et plus dans la branche extraite dans le dossier du projet ; les commits à merger sont comptés depuis cette base. Quand le projet est sur une autre branche, Escouade te propose de basculer : « Basculer sur « <base> » ? », bouton « Basculer et merger ». Si git refuse de basculer, son message s'affiche tel quel et rien n'est mergé ; si la base n'existe plus, « Merger » le dit au lieu de merger ailleurs.
- Quand l'agent d'un ticket atteint la limite d'usage sans reprise prévue (reprise automatique coupée ou annulée, clé API, heure de reprise inconnue), le ticket est bloqué comme avant, mais le pilote auto se met en pause 30 minutes au lieu de lancer le ticket suivant : il ne parcourt plus toute la colonne « À faire » en bloquant chaque ticket sur la même limite.
- Un ticket « Terminé » affiche le coût de tous ses agents, archivés compris, et plus seulement celui de l'agent qui l'a validé : le coût ne baisse plus entre « À tester » et « Terminé » quand un autre agent a repris le ticket.
- Ni « ≈ » ni « $ » ne passent plus seuls à la ligne dans un coût (« ≈ 1,63 $ »), par exemple au pied d'une carte « Terminé » dans la fenêtre la plus étroite.
- Une sélection de texte commencée dans un champ et relâchée hors d'une fenêtre (renommer, nouveau projet, commit…) ne ferme plus cette fenêtre, et ce que tu y avais tapé n'est plus perdu.
- « Quitter Escouade ? » (fichiers non enregistrés), annulé, te ramène à la fenêtre qu'il avait remplacée (la fenêtre « Commit » avec ton message ; une autre, comme les réglages, se rouvre), sauf si elle a été réglée entre-temps : un commit lancé ou fait (un commit refusé par git revient, avec son message), une confirmation dont le choix est en cours, la fenêtre d'une mise à jour qui n'est plus prête.
- Éditeur : « Garder ma version » sans passer par « Comparer » n'écrase plus une version que l'agent a écrite après l'apparition du bandeau « Ce fichier a changé sur le disque. » : rien n'est enregistré, tes modifications restent, un message le dit (« Le fichier a encore changé sur le disque : rien n'est enregistré, tes modifications sont toujours là. ») et le bouton attend une seconde avant d'écraser la nouvelle version. Si le fichier est revenu à la version que tu avais ouverte : « Rien n'a été enregistré : le fichier est revenu à la version que tu avais ouverte. ».
- Éditeur : « Nouveau fichier… » et l'enregistrement ne créent plus de fichier dans les worktrees des agents depuis le projet (un fichier recréé dans le worktree d'un agent supprimé refaisait ses dossiers là où git le garde) ; un fichier de worktree ouvert depuis le projet par un lien s'enregistre toujours.
- Éditeur : pour un agent dont le worktree a été supprimé (après une pull request ou un push), l'arborescence, la recherche et l'ouverture d'un fichier disent « Le worktree de l'agent n'existe plus » au lieu d'une erreur du système en anglais.
- Entrée dans le champ de saisie, tapée juste quand une demande arrive, ne la refuse plus (ni ne répond à une question) avec ton texte : elle attend une demi-seconde, et ton texte reste dans le champ.
- Un message refusé à l'envoi (Claude Code introuvable…) revient dans le champ de son agent au-dessus de ce que tu as écrit depuis, ses fichiers joints avec les nouveaux, au lieu de l'écraser, même si tu regardes un autre agent entre-temps.
- Un fichier texte non suivi de 1 à 4 Mo n'est plus pris pour un fichier binaire : son diff s'affiche (replié s'il est long). Dans « Voir le diff », les fichiers non suivis au-delà du volume de diff envoyé ne disparaissent plus : ils restent listés avec « Diff trop volumineux pour être affiché. ».
- Un nouveau fichier déjà indexé dont le nom contient une espace n'apparaît plus en double dans « Voir le diff », et un nom de fichier avec un saut de ligne ne peut plus fausser l'affichage du diff.
- Sous un patch tronqué de la conversation, « … 1 ligne de plus » s'écrit au singulier.
- Une ligne illisible dans l'historique d'une conversation ne fait plus disparaître les messages qui la suivent, et une erreur de lecture du disque au milieu de cet historique ne fait plus perdre la suite : la conversation montre ce qui a pu être lu, et son fichier n'est plus réécrit à partir de là.
- La conversation ne quitte plus le bas quand son contenu rétrécit (fin d'un tour, champ de message qui se replie) juste après un coup de molette vers le bas : seul un geste vers le haut l'arrête de suivre. Au clavier, Maj+Tab vers un élément plus haut (même depuis le champ de message) arrête de suivre le bas : l'élément reste à l'écran au lieu d'être ramené en bas par les nouveaux messages.
- Le dossier où tourne une commande (vue d'une commande de lancement ou d'une étape de recette) s'écrit avec `/` sous macOS, au lieu d'un `\` collé au chemin.
- Installer une mise à jour arrête d'abord proprement l'app (agents, terminaux, état sauvegardé) : avant, l'installeur la fermait sans cet arrêt.

## [1.5.4] — 2026-10-10

### Modifications

- macOS : l'app est signée avec un certificat Apple Developer ID et notarisée par Apple. Au premier lancement, macOS ne la bloque plus : plus besoin de l'autoriser dans Réglages Système › Confidentialité et sécurité. Après cette mise à jour, macOS peut redemander une fois les autorisations déjà accordées à Escouade.

## [1.5.3] — 2026-10-07

### Corrections

- macOS : les commandes de lancement lancées avec zsh lisent `~/.zshrc`, comme un terminal, et prennent donc en compte les gestionnaires de versions qui s'y initialisent (rbenv, rvm, asdf, mise, nvm…). Un serveur Rails ne démarre plus avec le Ruby du système, sans bundler. Il en va de même pour les tests et pour la préparation et le démontage des worktrees.

## [1.5.2] — 2026-10-07

### Ajouts

- Réglages › Réseau › Certificats : « Ignorer la vérification des certificats TLS », pour un proxy d'entreprise qui déchiffre le trafic avec son propre certificat (erreur « UnknownIssuer », par exemple en se connectant à Jira). L'option vaut pour les intégrations, les quotas, les mises à jour et les processus Claude Code, avec les commandes que lancent les agents. Désactivée par défaut, elle est à réserver à un réseau de confiance.

### Modifications

- Les intégrations, les quotas et les mises à jour font aussi confiance aux certificats installés sur le système (le plus souvent, celui du proxy de l'entreprise) : pour eux, l'option ci-dessus n'est alors pas nécessaire.
- La recherche et l'installation des mises à jour suivent les réglages réseau du moment (proxy et certificats).

### Corrections

- L'agent d'un ticket supprimé pendant la préparation de son worktree ne prend plus le ticket une fois la préparation arrêtée.

## [1.5.1] — 2026-10-06

### Ajouts

#### Worktrees : préparation et démontage

- Nouveau groupe « Worktrees » dans l'onglet « Projets » des réglages : des commandes lancées dans chaque nouveau worktree, l'une après l'autre (installer les dépendances, générer du code…), pour l'agent d'un ticket comme pour un agent créé à la main. Chacune a son shell et son sous-dossier.
  - Pendant la préparation, l'agent affiche « Préparation… », Claude Code ne démarre pas encore et ses messages attendent qu'elle soit finie : Claude ne travaille jamais sur un worktree à moitié installé.
  - Archiver ou supprimer l'agent, fermer le projet ou quitter Escouade arrête la préparation.
  - Un échec s'affiche dans la conversation avec les dernières lignes de la commande ; l'agent d'un ticket le lit à la suite de son premier message.
- Des commandes de démontage, lancées dans le worktree juste avant qu'Escouade le supprime (agent supprimé avec son worktree, ticket validé), pour défaire ce que la préparation a créé ailleurs (base de données, conteneurs…).
- « ✦ Remplir automatiquement » : Claude lit le projet en arrière-plan (sans rien modifier ni exécuter) et propose les commandes, à relire avant d'enregistrer.
- Les commandes disposent de `ESCOUADE_PROJECT_DIR`, `ESCOUADE_WORKTREE_DIR`, `ESCOUADE_BRANCH`, et des ports réservés d'un ticket.

#### isola

- Quand [isola](https://github.com/cyucelen/isola) est installé et que le projet a un `.isola.toml`, isola lance les services de chaque worktree sur ses propres ports :
  - l'agent d'un ticket ne reçoit plus de bloc de ports, et on ne lui demande plus de recette, seulement l'adresse de la fonctionnalité ;
  - « ▶ Tester » lance `isola up`, attend que chaque service réponde, puis ouvre la fonctionnalité dans le navigateur ;
  - « Tout arrêter », la validation, l'archivage et la fermeture du projet arrêtent les services (`isola down`) ; supprimer le worktree supprime aussi ses données (`isola destroy`).

### Modifications

- Une fois un ticket validé par « Valider + PR » ou « Valider et pousser », son worktree est supprimé lui aussi (sa branche reste). Le réglage s'appelle désormais « Supprimer le worktree une fois validé ».
- Restaurer un agent archivé dont le worktree a été supprimé le recrée depuis sa branche, avec ses fichiers copiés et sa préparation.

### Corrections

- Un ticket validé sans aucune modification de code passe « Terminé » avec « ∅ Aucune modification », au lieu de rester bloqué sur « Rien à merger ».

## [1.5.0] — 2026-10-04

### Ajouts

#### Éditeur : une arborescence à la manière de VS Code, et la création de fichiers

- L'arborescence de l'éditeur prend l'allure de l'explorateur de VS Code. On y retrouve :
  - des chevrons, des guides d'indentation et une icône colorée par type de fichier ;
  - le nom en couleur d'un fichier modifié (M) ou ajouté (A), et un point sur les dossiers qui le contiennent ;
  - un dossier qui ne contient qu'un autre dossier sur la même ligne que lui (`src/lib/editor`).
- Le fichier affiché défile jusqu'à l'écran.
- Au clavier : ↑ ↓ Début Fin pour se déplacer, → pour déplier un dossier ou y entrer, ← pour le replier ou remonter au parent.
- En tête de l'arborescence : « Nouveau fichier », « Actualiser » et « Tout réduire ».
- Nouveau fichier : le nom se tape directement dans l'arborescence, et l'icône suit l'extension.
  - Le fichier va dans le dossier de la dernière ligne cliquée, ou à côté du fichier affiché. Au clic droit, « Nouveau fichier… » le crée dans le dossier, à côté du fichier, ou à la racine depuis l'espace libre.
  - `dossier/nom.ts` crée les dossiers manquants.
  - Entrée crée le fichier vide et l'ouvre, le curseur dedans ; Échap annule.
  - Un nom refusé affiche la raison sous le champ : fichier déjà présent (sans tenir compte de la casse), chemin invalide, ou nom que Windows modifierait ou réserve (`a.`, `?`, `CON`…).
  - Un fichier créé mais ignoré par git s'ouvre quand même, avec un message : l'arborescence ne l'affiche pas.
- Clic droit sur une ligne : « Copier le chemin » et « Copier le chemin relatif ».

### Corrections

- Un dossier nommé comme un membre des objets JavaScript (`constructor`, `toString`…) s'ouvre désormais dans l'arborescence.

## [1.4.0] — 2026-10-04

### Ajouts

#### Intégrations : Jira, Trello et GitHub Issues

- Onglet « Intégrations » des réglages : connecte un compte Jira Cloud (site, e-mail, jeton d'API), Trello (clé d'API et jeton ; « Obtenir un jeton » ouvre la page d'autorisation de Trello) ou GitHub (un jeton, ou à défaut celui de `gh`). La connexion est vérifiée tout de suite, et un refus s'affiche sous le formulaire. Les jetons restent dans `~/.escouade/integrations.json`, à part des réglages, et la fenêtre de l'app ne les voit jamais.
- Par projet, une source par service : un projet Jira, un tableau Trello ou un dépôt GitHub (celui du projet en tête de liste).
- « ⤓ Importer », dans l'en-tête du Kanban :
  - une pastille par source liée, une recherche et les filtres du service (Jira : Assignés à moi, Sprint actif, À faire ; Trello : Mes cartes et une par liste ; GitHub : Assignées à moi et une par étiquette) ;
  - les tickets à cocher, avec leur type, leur priorité, leur assigné, leur statut et le nombre de critères détectés ; ceux déjà importés sont grisés, « Déjà dans le Kanban » ;
  - le nombre de boucles max, puis « Importer » : les tickets arrivent à la fin de « À faire ».
- Un ticket importé reprend le titre et la description d'origine (le texte Jira converti en markdown), avec le lien vers le ticket. Ses critères d'acceptation viennent de la liste sous « Critères d'acceptation », « Acceptance criteria » ou « Definition of done », sinon de ses cases à cocher (Trello : de la checklist de ce nom, sinon de toutes) ; à défaut, ce sont les critères par défaut. Sa carte porte la pastille du service et la clé d'origine (`ATL-1287`, `#42`), qui ouvre le ticket dans le navigateur.
- Synchro des statuts : quand un ticket importé change de colonne, son pendant suit (transition Jira, liste Trello, issue GitHub ouverte, fermée ou étiquetée). Choisir une source pré-remplit la correspondance : « En cours » et « À tester » vers l'état qui ressemble à « en cours », « Terminé » vers celui qui ressemble à « à tester ». Elle se règle par colonne et par source.
- Commentaires sur le ticket d'origine, à l'arrivée dans les colonnes cochées (« À tester » et « Terminé » par défaut) : pris par un agent, prêt à tester (critères et ce qui a été fait), terminé (issue de la validation), revenu « À faire ». En option, un résumé à chaque boucle.
- Une synchro ratée ne bloque jamais le ticket : sa carte montre ⚠ et la raison, jusqu'à la suivante qui réussit.
- Import automatique (désactivé par défaut) : toutes les 5, 15 ou 60 minutes, les tickets ouverts des sources liées qui portent l'étiquette choisie (`claude-ready` par défaut) arrivent dans « À faire ». Un ticket supprimé du Kanban n'est jamais ramené.

### Modifications

- Une seule fenêtre de réglages pour toute l'app, en onglets : Claude Code, Notifications, Projets, Kanban, Intégrations, Terminaux, Réseau, À propos. Elle remplace « Réglages du tableau » et « Commandes de lancement ».
  - « Enregistrer » enregistre d'un coup ce qui a changé, dans tous les onglets et pour tous les projets ; un point marque les onglets modifiés. « Annuler », Échap ou × jettent tout.
  - « ⚙ Après validation » ouvre l'onglet Kanban du projet, le ⚙ de la section Lancement son onglet Projets. Au clic droit sur un onglet de projet, « Réglages du projet… » remplace « Commandes de lancement… ».
- Le tableau s'appelle désormais « Kanban » (sélecteur « Agents | Kanban »).

## [1.3.1] — 2026-10-04

### Corrections

- Quand un ticket passe « Terminé », les terminaux de test de son agent quittent la section « Lancement », avec leurs logs, et sa fenêtre « Tester » se ferme.
- Un lancement relancé pendant l'arrêt du précédent ne démarre plus si sa commande a disparu entre-temps (agent archivé ou supprimé, ticket terminé, projet fermé…).

## [1.3.0] — 2026-10-04

### Ajouts

#### Tableau : des tickets que des agents prennent seuls

- Sélecteur « Agents | Tableau » en haut de la barre latérale, avec une pastille jaune qui compte les tickets « À tester ». Le tableau d'un projet remplace la zone principale : À faire, En cours, À tester, Terminé.
- Tickets avec une clé tirée du projet (DEM-1, DEM-2…), un titre, une description et des critères d'acceptation, un par ligne. Sans critère, deux par défaut : « Implémentation conforme au ticket » et « Tests verts ». On règle aussi le nombre de boucles max : 3, 5 ou 8. Un ticket « À faire » se modifie, passe en tête, se supprime ou se lance à la main (« Lancer »).
- Pilote auto : des agents se créent seuls pour prendre les tickets, dans l'ordre, jusqu'au nombre d'agents en parallèle choisi (1 à 6, 2 par défaut). Rien ne part tant qu'un agent attend la fin de sa limite d'usage.
- Chaque agent de ticket a son worktree `ticket/<clé>`, créé depuis la branche cible, et un bloc de 10 ports réservé à partir de 4100. Il reçoit aussi les fichiers du projet que git ignore (`.env*` par défaut, liste réglable dans les commandes de lancement) : ces copies ne peuvent jamais être commitées.
- L'agent boucle jusqu'au but. À chaque fin de tour, il fait le bilan de ses critères. S'il en manque, il repart avec les critères manquants et leurs notes. Tous atteints, le ticket passe « À tester ». À la limite de boucles, il y passe aussi, avec « Objectif partiel ».
- Un ticket bloqué libère sa place : interruption, erreur, ou bilan manquant après un rappel. « Reprendre » le relance.
- Avancement : l'agent tient une liste courte des fonctionnalités en place. Les cartes « En cours » en montrent les 3 dernières (« +n » s'il y en a d'autres). Les cartes « À tester » montrent la liste complète sous « Ce qui a été fait ».
- Les cartes « En cours » montrent la boucle en cours (« Boucle 2/5 »), les critères ✓ / ○ et une barre des critères atteints. Elles montrent aussi l'activité de l'agent en direct (« Lit src/db.ts », « Lance npm test »…) ou « Question en attente de ta réponse ».
- La carte d'un agent de ticket, dans la barre latérale, porte « ▸ DEM-1 · boucle 2/5 ».
- Validation réglable par projet (« ⚙ Après validation ») :
  - quatre actions : merger dans une branche (merge commit, squash ou rebase), ouvrir une pull request, pousser la branche du ticket, ou laisser en l'état ;
  - en option, relancer les tests avant : en échec, le ticket repart chez son agent avec la fin de la sortie ;
  - un message de commit généré au format Conventional Commits, avec la clé du ticket ;
  - la suppression du worktree après merge ;
  - en cas de conflit : me demander, l'agent résout, ou annuler.
- Pull request : par `gh` quand il est installé, avec la description, l'avancement et les critères. Sinon, la page GitHub de la pull request s'ouvre pré-remplie.
- « Renvoyer » rend un ticket « À tester » à son agent avec ce qui ne va pas.
- Garde-fous de la validation :
  - les fichiers copiés du projet (`.env`…) ne partent jamais dans un commit, une branche ou un dépôt distant ;
  - rien n'est validé ni renvoyé pendant que l'agent travaille ;
  - un seul merge à la fois par dépôt ;
  - refus clair quand la branche n'a rien de nouveau (« Rien à merger / proposer / pousser »).
- Reprise : un ticket coupé par l'arrêt de l'app repart au démarrage suivant, et une validation coupée laisse « Validation interrompue ». Archiver ou supprimer l'agent d'un ticket le renvoie « À faire », après confirmation.
- Dans la conversation, le bilan de l'agent s'affiche comme une carte « Bilan des critères » : ✓ / ○, notes, avancement, puis la recette de lancement s'il y en a une.
- Notifications « DEM-1 prêt à tester » et « DEM-1 bloqué : <raison> » ; un clic ouvre le tableau du projet. Les fins de tour d'un agent de ticket en cours ne notifient plus (ses questions, si).
- L'en-tête du tableau donne les places libres et l'état du quota. Il dit aussi pourquoi rien ne démarre : Claude Code introuvable, ou branche cible absente ou sans commit. Le premier commit d'un projet neuf relance la file.
- « Réglages du tableau » : action à la validation, branche cible, stratégie, brouillon de PR, tests, message de commit, conflits, nombre d'agents en parallèle et leur modèle.

#### Lancement de test, sur des ports à part

- Pour tout agent à worktree, « Préparer le lancement » lui demande sa recette sur ses ports réservés : préparation, processus à lancer, adresse de la fonctionnalité à ouvrir.
- « ▶ Tester » ouvre une fenêtre qui fait tout :
  - elle prépare (une fois par recette) et lance chaque serveur ;
  - elle attend que chacun réponde en HTTP (3 min max) ;
  - elle ouvre le navigateur directement sur la fonctionnalité développée ;
  - « Rouvrir », « Voir les logs » et « Tout arrêter » restent à portée.
- Les étapes de la recette apparaissent dans la section « Lancement », sous le nom de l'agent, avec leurs logs. Elles s'arrêtent à la validation, à l'archivage ou à la suppression de l'agent, et à la fermeture du projet. Une nouvelle recette arrête les lancements de l'ancienne.

#### Mises à jour

- L'app cherche une nouvelle version toutes les cinq minutes, plus seulement au lancement.
- Les notes de version viennent de ce journal.

### Corrections

- Les liens de la conversation, des terminaux et du contrôle à distance s'ouvrent de nouveau dans le navigateur.

## [1.1.0] — 2026-10-03

### Ajouts

- Mode éditeur dans un projet :
  - arborescence, onglets et code coloré ;
  - enregistrement par `Ctrl+S` ;
  - lignes modifiées marquées par rapport à la version de référence ;
  - avertissement quand un fichier change sur le disque ;
  - fins de ligne conservées, et jamais d'écriture hors du dossier du projet ou du worktree.
- L'éditeur s'ouvre depuis l'en-tête d'un agent (« Éditeur »), le clic droit sur un agent, « Parcourir » sur la branche du projet et la barre d'onglets. Il s'ouvre aussi depuis les fichiers non commités (bouton `</>`) et les fichiers modifiés cités dans la conversation.
- Les modèles affichent la version que Claude Code utilise (par exemple Sonnet 5.5).
- Quitter avec des fichiers non enregistrés demande confirmation.

### Modifications

- L'éditeur intégré remplace l'ouverture des fichiers et des projets dans un éditeur externe (réglage et menus retirés).

### Corrections

- Une release créée depuis la page GitHub prend la version de son tag, et une release incomplète ne bloque plus les mises à jour.
