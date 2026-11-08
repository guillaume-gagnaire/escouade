# L’écran partagé

La zone de travail se partage en plusieurs volets, côte à côte ou l’un au-dessus de l’autre, pour voir en même temps un agent et un terminal, deux agents, ou l’éditeur à côté d’une conversation. Cette page explique comment ouvrir un volet, lequel reçoit le clavier et ce qui est retenu.

## Ouvrir un volet

Ce qui va dans un volet : un agent, un terminal, le journal d’un lancement ou l’éditeur. Le Kanban, les pull requests et les Docs prennent toute la zone, comme avant ; en les quittant, tu retrouves tes volets.

| Pour… | Fais ceci |
|---|---|
| Ouvrir à côté en glissant | Glisse la carte d’un agent, la ligne d’un terminal ou d’un lancement depuis la barre latérale, ou le bouton « Éditeur » de la barre de titre, sur la zone de travail. Une boussole de cinq cases s’ouvre sur le volet survolé : haut, gauche, droite et bas ouvrent à côté dans ce sens, le milieu remplace. La case sous le pointeur s’allume et la place que prendra le volet se dessine. |
| Annuler le geste | `Échap`, ou lâcher hors de la zone de travail. |
| Ouvrir à côté sans glisser | `Ctrl+clic` sur une carte ou une ligne ; ou un clic droit, puis « Ouvrir à côté » ou « Ouvrir en dessous ». |
| Partager d’un coup | `Ctrl+\` : ce qui est affiché et l’agent suivant (ou un terminal) côte à côte. Le même raccourci revient à un seul volet, celui qui est actif. |

Il y a quatre volets au plus. Une case barrée sur la boussole veut dire qu’il n’y a pas la place : chaque volet garde au moins 320 pixels de large et 200 de haut. Le milieu remplace toujours. Un contenu n’est jamais montré deux fois : le lâcher sur un autre volet l’y déplace, et au milieu les deux volets échangent.

## Le volet actif

Un seul volet est actif : celui que tu as cliqué en dernier. Il est entouré de la couleur du projet, et sa carte ou sa ligne est surlignée dans la barre latérale ; celles des autres volets sont simplement entourées.

Le volet actif reçoit :

- ta saisie, et la dictée quand aucun champ n’a le focus ;
- les réponses au clavier aux questions et aux autorisations (`Ctrl+Entrée`, `Alt+1` … `Alt+9`) : celles d’un autre volet se répondent à la souris, ou après avoir activé son volet ;
- `Ctrl+Tab`, le panneau des fichiers (`Ctrl+Maj+B`), l’éditeur et la barre d’état.

Un clic dans la barre latérale remplace le contenu du volet actif. Un agent montré dans n’importe quel volet est vu : il ne clignote pas.

## Taille et sens

La poignée entre deux volets se tire à la souris ; quand elle a le focus, les flèches la déplacent (avec `Maj`, plus vite) et `Entrée` revient à moitié-moitié, comme un double clic.

| Raccourci | Action |
|---|---|
| `Ctrl+\` | Partager, ou revenir à un seul volet. Sur un clavier AZERTY, c’est la touche `*` : `Ctrl+*`. |
| `Ctrl+Maj+\` | Tourner la séparation du volet actif : côte à côte ou l’un au-dessus de l’autre. |
| `Ctrl+Alt+←` `→` `↑` `↓` | Passer au volet de ce côté. |
| `Ctrl+Maj+W` | Fermer le volet actif. Le `×` en haut à droite d’un volet, au survol, le ferme aussi. |

Ces raccourcis marchent aussi quand un terminal a le focus.

## Ce qui est retenu

La disposition de chaque projet, son sens, la taille des volets, leur contenu et le volet actif, revient quand tu reviens au projet et au prochain lancement d’Escouade. Un terminal ne revient pas, puisqu’il est arrêté à la fermeture : son volet se ferme. Un agent archivé ou supprimé, un terminal fermé quittent leur volet, et les autres prennent la place ; avec un seul volet restant, il occupe toute la zone.

Le « Côte à côte » d’une conversation et de ses fichiers (`Ctrl+Maj+L`) reste tel quel quand un seul contenu est affiché ; dans un volet, les fichiers s’ouvrent par-dessus la conversation.
