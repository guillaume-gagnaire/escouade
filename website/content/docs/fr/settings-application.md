# Réglages : la fenêtre et Général

Cette page décrit la fenêtre des réglages (sa barre, sa recherche, ses points ambre, son enregistrement au geste) et sa page « Général », qui règle l’apparence (le thème, le zoom de l’interface, la taille du texte des terminaux et de l’éditeur) et choisit la langue de l’interface et celle des textes que Claude rédige.

## La fenêtre des réglages

Les réglages d’Escouade tiennent dans une seule fenêtre : une barre à gauche, une page à la fois à droite. Ouvre-la avec `Ctrl+,` (`⌘,` sous macOS) ou avec le bouton « Réglages », en bas de la [barre des projets](interface). Elle s’ouvre sur « Comptes et harness », sauf quand un bouton l’ouvre directement sur la page qui le concerne : le bouton ⚙ de l’en-tête du [Kanban](board) ouvre la page « Kanban » de son projet, le menu d’un projet sa page « Projet », « Configurer » de la section Lancement sa page « Lancement ». Le raccourci ne fait rien tant qu’une autre fenêtre est ouverte.

### La barre

De haut en bas : le champ « Rechercher un réglage », les pages d’Escouade, chaque projet, « Tous les projets », puis « À propos » avec la version d’Escouade. Le titre et la description de la page affichée sont en haut de la partie droite. Quand le focus est sur la barre, les flèches haut et bas passent d’une entrée à l’autre et l’ouvrent, `Home` et `End` vont à la première et à la dernière.

| Page | Ce qu’elle règle |
|---|---|
| « Général » | Apparence, langues, mises à jour, ton nom et les statistiques d’usage |
| [« Agents »](settings-claude-code) | Les nouveaux agents, les textes rédigés, le remote control, le serveur MCP |
| [« Comptes et harness »](settings-accounts) | Chaque harness et ses comptes, et ce qui se passe quand un quota est épuisé |
| [« Services connectés »](settings-integrations) | Les hébergeurs de code et les comptes des sources de tickets |
| [« Voix »](settings-voice) | La dictée et la conversation vocale |
| [« Notifications »](settings-notifications) | Alertes visuelles et sonores |
| « Plugins » | Les plugins installés, ceux que propose Escouade, la sécurité |
| « Avancé » | [Réseau](settings-network), [terminaux](settings-terminals), pièces jointes, recherche par le sens, données locales |
| [« À propos »](settings-about) | Version, crédits et liens |

Sous « Projets », chaque projet a sa couleur devant son nom, et un cadenas s’il est privé. Le projet choisi se déplie sur ses sept pages : [« Projet »](settings-projects), « Git et worktrees », « Lancement », [« Kanban »](settings-board), [« Tickets importés »](settings-integrations), « Équipe » et « Confidentialité ». Passer d’un projet à l’autre garde la même page. Sur une page d’un projet, la fenêtre prend la couleur de ce projet ; sur une page d’Escouade, le teal d’Escouade. Sans projet ouvert, une page de projet dit « Aucun projet ouvert. ».

En bas d’une page, « Avancé » replie les réglages qu’on touche rarement sur une ligne qui les nomme ; un clic les déplie sur place.

### Chercher un réglage

Le champ « Rechercher un réglage », en tête de la barre, trouve un réglage par son nom ou sa description, et une page par son nom ; `Ctrl+F` (`⌘F` sous macOS) y mène depuis toute la fenêtre. Il cherche en français et en anglais quelle que soit la langue de l’interface, sans tenir compte des majuscules ni des accents : « beta » trouve « Recevoir les versions bêta », « proxy » trouve « Proxy HTTP(S) ».

Tant que le champ a du texte, ses résultats remplacent les entrées de la barre, chacun avec la page où il est : « Avancé », « Général › Avancé », « demo-api › Git et worktrees ». Un réglage de projet vient une fois par projet, le projet choisi d’abord. Les flèches haut et bas passent d’un résultat à l’autre ; `Entrée` ou un clic ouvre sa page, déplie « Avancé » s’il y est, et surligne le réglage un instant. `Échap` vide le champ ; un second `Échap` ferme la fenêtre.

### Les points ambre

Un point ambre au bout d’une entrée de la barre dit qu’une page attend quelque chose de toi ; son info-bulle dit quoi :

- « Comptes et harness » : un harness installé qui n’a aucun compte (« Codex n’a aucun compte : aucun agent ni ticket n’y démarre ») ;
- « Services connectés » : le jeton d’un hébergeur expiré ou révoqué, ou un jeton illisible dans le trousseau du système ;
- « Plugins » : un plugin désactivé par Escouade, au manifeste invalide, ou arrêté après ses plantages ;
- un projet : aucun compte actif de son harness ne le sert. Le point est sur le projet et sur sa page « Projet », où se choisit son compte.

### Tout s’enregistre au geste

Il n’y a ni « Annuler » ni « Enregistrer » : comme dans les réglages de macOS ou de VS Code, un choix (un interrupteur, un menu, une commande ajoutée) s’applique et s’enregistre aussitôt, un texte quand tu quittes son champ ou après une courte pause dans la frappe. Fermer la fenêtre (la croix, `Échap` ou un clic sur le fond) enregistre ce qui attendait encore.

Une saisie qui ne peut pas être enregistrée le dit sur sa ligne, et la valeur d’avant reste tant qu’elle n’est pas corrigée : un projet sans nom, une commande de lancement sans nom ou sans ligne de commande, un dossier des worktrees ou un préfixe de branche refusé, un nombre vidé.

## La page « Général »

La page a quatre groupes, « Apparence », « Langue », « Mises à jour » et « Toi » (voir [À propos](settings-about) pour les deux derniers), puis « Avancé » : la taille du texte de l’éditeur et ton identifiant.

### Apparence

Ces quatre réglages s’appliquent tout de suite, sans redémarrage, et restent d’un lancement à l’autre. La taille du texte de l’éditeur est dans « Avancé ».

| Réglage | Effet | Par défaut |
|---|---|---|
| « Thème » | « Système », « Clair » ou « Sombre ». « Système » suit le thème de Windows, macOS ou Linux, et ses changements, à chaud. La barre de titre et les contrôles natifs (barres de défilement, menus) suivent aussi. | « Système » |
| « Zoom de l’interface » | Agrandit ou réduit toute la fenêtre, comme le zoom d’un navigateur : le texte, les espacements et les icônes ensemble, terminaux et éditeur compris. Paliers : 80 %, 90 %, 100 %, 110 %, 125 %, 150 %, 175 % et 200 %. Les réglages proposent les plus courants, de 90 à 150 %, en segments (et celui en cours quand il n’en fait pas partie) ; le clavier atteint tous les paliers. | 100 % |
| « Taille du texte des terminaux » | La taille de la police des terminaux et de la [vue Claude Code](claude-code-view), en pixels, de 9 à 24 par demi-pixel. Le zoom s’y ajoute. | 12.5 |
| « Taille du texte de l’éditeur » | La taille de la police de l’[éditeur](editor), en pixels, de 9 à 24 ; les lignes grandissent avec elle. Le zoom s’y ajoute. | 12.5 |

Au clavier, partout, même dans un terminal ou avec une fenêtre ouverte : `Ctrl+=` (ou `Ctrl++`) agrandit d’un palier, `Ctrl+-` réduit d’un palier, `Ctrl+0` revient à 100 % (`⌘=`, `⌘-`, `⌘0` sous macOS). Les touches `+` et `-` du pavé numérique marchent aussi.

Un terminal dont la police change se redimensionne aussitôt dans sa zone, et son programme apprend sa nouvelle taille en colonnes et en lignes : rien ne dépasse ni ne se coupe.

> [!NOTE]
> L’interface est prévue pour au moins 1000 × 600 pixels, la taille minimale de la fenêtre. Un palier qui ne laisserait pas cette place est grisé, avec l’info-bulle « Trop grand pour la taille de la fenêtre », et `Ctrl+=` s’arrête avant. Si la fenêtre rétrécit sous le palier choisi, l’interface passe au plus grand palier qu’elle peut tenir, et la page le dit ; le palier choisi revient dès que la fenêtre est assez grande.

### Langue

Les deux choix prennent effet aussitôt, sans redémarrer : la fenêtre, les menus de l’app et les textes qu’Escouade écrit passent dans la nouvelle langue.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Langue de l’interface » | La langue de la fenêtre, des menus de l’app et des messages qu’Escouade affiche. | « Système » |
| « Langue des textes rédigés par les agents » | La langue de ce que Claude écrit pour toi : commits, pull requests, commentaires, consignes aux agents. | « Comme l’interface (par défaut) » |

### Langue de l’interface

Trois choix, chacun écrit dans sa propre langue pour que tu retrouves la tienne quelle que soit celle de l’interface :

- « Système (Français) » suit la langue de l’ordinateur. Escouade existe en français et en anglais : elle prend le français quand la première langue préférée du système est le français, l’anglais sinon. La langue du système est nommée dans celle de l’interface, donc un système en anglais avec une interface en français donne « Système (Anglais) ».
- « English ».
- « Français ».

Le changement touche aussi les menus natifs de l’app et l’info-bulle de l’icône de la barre des tâches.

### Langue des textes rédigés par les agents

Ce choix décide dans quelle langue Escouade demande à Claude d’écrire ce qui sort de l’app :

- les messages de commit et les descriptions de pull request proposés, voir [Git](git) ;
- les commentaires publiés dans Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards, voir [les intégrations](integrations) ;
- les consignes qu’Escouade donne aux agents.

Les trois choix sont « Comme l’interface (par défaut) », « English » et « Français ». Avec « Comme l’interface », le message de commit proposé reprend la langue des derniers commits du dépôt, pour s’y fondre. Avec « English » ou « Français », c’est cette langue qui est demandée.

> [!TIP]
> Beaucoup d’équipes commitent en anglais avec une interface en français : choisis « Français » pour l’interface et « English » pour les textes de Claude.

## Textes rédigés

Ce groupe est dans la page « Agents ». « Modèle des textes rédigés » choisit le modèle qui écrit les textes courts qu’Escouade fait rédiger hors d’une conversation d’agent :

- le nom donné à un nouvel agent ;
- les messages de commit et les descriptions de pull request proposés, voir [Git](git) ;
- les commandes du worktree que « ✦ Remplir automatiquement » propose après avoir lu le projet.

Chaque question a 90 secondes pour sa réponse ; passé ce délai, le processus est arrêté avec tout ce qu’il a lancé, et la réponse est lue jusqu’à une taille bornée. Seuls les [harness](settings-claude-code) installés sont proposés. Par défaut, c’est « Claude Haiku » quand Claude Code est installé, sinon le premier harness trouvé. « Codex (GPT-6-Luna) » les écrit avec `codex exec`, sur le compte Codex de l’agent ou du projet concerné : sans session gardée (`--ephemeral`), sans tes réglages ni tes serveurs MCP, dans un bac à sable en lecture seule, et sans outil pour un nom ou un message. « Copilot (GPT-5 mini) » les écrit avec `copilot -p`, sur le compte Copilot de l’agent ou du projet : la question passe par son entrée, jamais par sa ligne de commande, sans outil pour un nom ou un message, sans tes serveurs MCP ni tes consignes, et sa session est retirée une fois la réponse lue ; un compte Copilot Free n’accepte aucun modèle nommé, et la question passe alors par `auto`. Un harness choisi puis désinstallé laisse la place à ce choix par défaut. Sans aucun harness installé, le groupe n’apparaît pas.
