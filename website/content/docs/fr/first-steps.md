# Premiers pas

Ce que fait Escouade à son premier lancement : sept stations, une question chacune, du harness à ton premier agent qui répond, en deux minutes environ si ton harness est déjà connecté. Tout se passe et se reprend.

## Le parcours

Au premier démarrage d’une nouvelle installation, Escouade ouvre le « Premier lancement » à la place de la barre des projets et de la place de travail. À gauche, la ligne des stations, tracée comme le git graph : pleine quand la station est faite, teal pour celle en cours (« maintenant »), creuse quand elle est passée (« passé · à reprendre »), en pointillés pour celles à venir. Chaque station est un bouton qui y mène. À droite, la question de la station, pourquoi elle est posée et ses choix, le choix recommandé déjà pris.

![La première station du premier lancement : la ligne des stations à gauche, la question « Avec quel harness tes agents travaillent-ils ? » et Claude Code connecté, recommandé](/images/first-launch-harness.jpg)

Le parcours ne règle rien à sa façon : chaque station écrit ce qu’écrivent les réglages, et une station est faite dès que ce qu’elle règle l’est, où que ce soit (un compte ajouté dans Réglages coche la station du compte). Il ne s’ouvre jamais de lui-même sur un poste déjà réglé, même après une mise à jour.

| Station | La question | Ce qu’elle règle |
|---|---|---|
| 1. Harness | « Avec quel harness tes agents travaillent-ils ? » | Le harness en tête de l’ordre des harness, son plugin (depuis l’app, sans réseau) et, s’il est connecté sur la machine, son compte |
| 2. Compte | « Quel compte pour Claude Code ? » | « Le compte de la machine » ou « Un compte à part » (voir [plusieurs comptes](accounts)) ; fondue dans la première quand le harness est déjà connecté |
| 3. Réglages de départ | « Quand tes agents doivent-ils te demander ? » | Le mode des nouveaux agents : « Édits auto » (recommandé), « Demander », « Plan » ou « Auto » ; le modèle et l’effort sont ceux de ton compte |
| 4. Ce que tu veux brancher | « Que veux-tu brancher ? » | Rien de coché d’avance : une source de tickets, la voix, un hébergeur de code |
| 5. Premier projet | « Où travaille ton premier agent ? » | Un dossier de ta machine (recommandé), un dépôt à cloner, ou un projet de démonstration |
| 6. Premier agent | « Ton premier message » | Un agent créé dans ce projet, un message proposé que tu envoies toi-même |
| 7. Statistiques | « Aider à améliorer Escouade ? » | « Non merci » ou « Partager les statistiques » |

## Au clavier

Tout le parcours se fait au clavier : `↑` et `↓` vont d’un choix à l’autre (depuis la question, ils amènent aux choix), `Espace` prend le choix qui a le focus, `Entrée` fait le bouton principal (« Continuer », « Garder Édits auto », « Ouvrir le projet »…), `Échap` fait « Plus tard ». Le focus arrive sur la question à chaque station, et un lecteur d’écran entend la station en cours dans la ligne. Dans la conversation du premier agent, les touches restent les siennes : `Échap` y arrête l’agent.

## Les réglages de départ

La troisième station choisit quand tes agents te demandent ton accord. « Édits auto » est recommandé : les modifications de fichiers passent sans demander (elles restent dans le worktree de l’agent, visibles dans son diff) et chaque commande te demande. Bypass est montré, éteint, et jamais proposé comme mode de départ : il s’allume dans Réglages › Agents (voir [les modes de permission](managing-agents#modes-de-permission)).

![La station « Quand tes agents doivent-ils te demander ? » en thème clair : les quatre modes, Édits auto recommandé, et Bypass éteint avec sa raison](/images/first-launch-defaults.jpg)

## Ce que tu veux brancher

Trois cartes, aucune cochée : « Une source de tickets » (Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, Azure Boards), « La voix » (Whisper sur ta machine, son poids dit avant le téléchargement, ou un service en ligne avec ta clé) et « Un hébergeur de code » (pour les PR et la CI). Cocher une carte ouvre ses choix ; seul ce que tu choisis installe son plugin, et sa connexion se fait par le même formulaire que dans les réglages (voir [les intégrations](integrations) et [les hébergeurs de code](code-hosts)). Ce que tu passes t’attend dans Aide › À découvrir et dans les Réglages.

## Le premier projet et le premier agent

Un projet est un dossier, avec ses agents, son Kanban et ses docs ; chaque agent y travaille dans son [worktree](worktrees), sur sa branche. La station propose un dossier de ta machine, un dépôt à cloner par son adresse (avec tes accès git habituels), ou « Un projet de démonstration » : un petit dépôt jetable écrit par l’app dans `~/.escouade/demo/bac-a-sable`, une liste de tâches en TypeScript dont deux tests échouent exprès, sans réseau. Une bande en haut du projet de démonstration le signale, et « Supprimer » l’efface avec son dossier.

La station suivante crée ton premier agent dans ce projet et montre sa vraie conversation à droite. Trois messages sont proposés (« Explique-moi ce dépôt en cinq lignes. », « Lance les tests et dis-moi ce qui casse. » quand le projet a une commande de test qui peut tourner ici, « Trouve un petit bug et propose un correctif. ») : le message pris est mis dans le champ, modifiable, et rien ne part avant que tu cliques sur « Envoyer ». Il part sur ton compte et compte dans son quota. Sa première demande de permission est expliquée à côté de la vraie carte (`Ctrl+Entrée` autorise, `Ctrl+Maj+Entrée` autorise toujours ; sous macOS, `Ctrl` devient `⌘`). Une fois le tour fini, « Voir le diff » montre ce qu’il a changé dans son worktree : ton dépôt n’a pas bougé.

![La station du premier agent : « Ton agent a répondu », à droite sa conversation sur le projet de démonstration, son diff d’un fichier](/images/first-launch-agent.jpg)

Un tour qui s’arrête court le dit et laisse reprendre : un compte à sa limite d’usage (« Ajouter un compte… »), hors ligne (« Remettre le message »), un harness déconnecté (« Revenir à la station du compte »).

## Les statistiques

La dernière station pose la question des statistiques d’usage : « Non merci » et « Partager les statistiques » ont le même poids. Rien n’est envoyé avant ta réponse, et ce qui s’est passé pendant le parcours (les stations faites ou passées, sa durée par tranche) n’est envoyé qu’avec ton accord, sans nom, chemin ni contenu. Passée, la question revient une fois au démarrage suivant. Le détail est sur la page [Confidentialité](/privacy/).

## Après le parcours

« Escouade est prêt » : « Commencer » t’amène sur ton premier projet, « Voir ce qu’il y a à découvrir » ouvre la carte Aide › À découvrir. Le reste d’Escouade s’y découvre en le faisant pour de vrai, une chose à la fois : chaque découverte dit ce qu’elle demande avant (un téléchargement, un jeton, du quota), « Essayer » t’amène sur le vrai contrôle, et elle se coche quand l’app voit le geste fait. Un point sur l’icône « Aide » du rail dit qu’il en reste ; « Tout masquer » le retire.

![La carte « À découvrir » en thème clair : le tronc de ce qui est installé et les cinq voies Planifier, Coder, Relire, Livrer et Piloter](/images/first-launch-discover.jpg)

## Reprendre ou refaire le parcours

- **Fermé au milieu** (l’app quittée, une coupure), il rouvre au démarrage suivant, à la première station ni faite ni passée.
- **« Tout passer »** passe toutes les stations qui restent et ferme le parcours ; il ne revient plus de lui-même.
- **Refaire** : Aide › Premier lancement, ou Réglages › Général › « Refaire le premier lancement », le rouvre sur n’importe quel poste, depuis la première station, avec la valeur en place sur la ligne. Rien n’est remis à zéro : ni compte ni projet n’est retiré. « Fermer » le referme.

## Sans harness, ou sur un poste géré

- **Aucun harness trouvé** : la première station donne la commande d’installation de chacun, « Copier » la met dans le presse-papier, et Escouade le trouve dès que tu reviens dans la fenêtre (ou par « Chercher à nouveau »). Voir [les harness](harness).
- **Un poste géré** : ce qu’une stratégie impose est montré verrouillé, avec qui l’impose (« Imposé par la stratégie de ce poste »). Un harness non permis est grisé, un seul permis est dit (« Claude Code est le seul harness permis sur ce poste. »), un mode interdit ne se prend pas. Voir [les stratégies gérées](managed-policies).
- **Un poste réglé avant** : le parcours ne s’ouvre pas ; seul le point sur « Aide » invite à découvrir ce qui est nouveau.
