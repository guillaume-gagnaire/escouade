# Suivre ses agents

Quand plusieurs agents travaillent en même temps, il faut savoir lequel attend quoi. Cette page présente la vue d’ensemble, qui liste tous les agents de tous les projets, les raccourcis pour passer d’un agent à l’autre, et la recherche dans les conversations.

Sous macOS, `Ctrl` devient `⌘` pour les raccourcis (sauf `Ctrl+Tab`).

## La vue d’ensemble

La vue d’ensemble prend la place du projet affiché pour lister tous les agents de tous les projets. Ouvre-la avec le bouton « Vue d’ensemble », en haut de la barre des projets, ou avec `Ctrl+Maj+A` (le raccourci marche aussi depuis un terminal). Le même bouton ou le même raccourci la referme ; `Échap` aussi, et tu retrouves la vue qu’elle cachait : le projet ou les statistiques.

### Ce qu’elle montre

L’en-tête donne le compte : « 2 projets · 7 agents · 3 en cours ». Dessous, les agents sont rangés en groupes :

1. **« Attend ta réponse »** : les agents au statut « Question », de tous les projets, celui qui attend depuis le plus longtemps en premier, avec le nom de son projet.
2. **Un groupe par projet**, dans l’ordre de la barre des projets, avec sa couleur ; les agents y sont dans l’ordre de la barre latérale.
3. **« Pull requests »** (« Merge requests » quand toutes viennent de GitLab), quand une PR attend ta main, tous projets confondus : « À corriger », « À merger » et « Relectures demandées ». Un clic ouvre son agent, sinon le Kanban de son ticket, sinon la vue « Pull requests » de son projet (voir [les hébergeurs de code](code-hosts#les-nouvelles-des-hebergeurs)).

Un agent n’apparaît qu’une fois, et les agents archivés ne sont pas listés. Chaque ligne a six colonnes :

| Colonne | Contenu |
|---|---|
| Agent | La pastille de statut, le nom, et l’étiquette `▸ CLÉ` du ticket s’il y en a un |
| Activité | « Question », « Reprise à 15:00 » (limite d’usage), « Prépare le worktree » suivi de l’étape, ou ce que l’agent fait (« Lit src/a.ts », « Réfléchit »…) ; pour un agent au repos, son statut : « Prêt », « Terminé » ou « Erreur » |
| Modèle | Le modèle de l’agent |
| Coût | Le coût de l’agent, précédé de « ≈ » pendant un tour |
| Contexte | La part de la fenêtre du modèle déjà remplie, ou « — » tant qu’elle n’est pas connue |
| Depuis | Le temps écoulé depuis son dernier changement (« Dernier changement » avec l’heure dans l’infobulle) |

Si l’agent publie une ligne d’avancement par le serveur MCP d’Escouade (voir [le serveur MCP](mcp)), elle est écrite sous sa ligne.

Un clic sur une ligne, ou `Entrée` quand elle a le focus, ouvre l’agent dans son projet, sur sa conversation.

### Répondre sur place

Les lignes du groupe « Attend ta réponse » montrent la demande de l’agent, et tu peux répondre à certaines sans quitter la vue.

- Une **demande d’autorisation** qu’on peut lire en entier ici (une commande, un fichier, une recherche, une adresse : les outils `Bash`, `PowerShell`, `SlashCommand`, `Read`, `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Grep`, `Glob`, `WebFetch` et `WebSearch`) affiche l’outil et ce qu’il veut faire, avec les boutons « Autoriser » et « Refuser ». Il n’y a pas de « Toujours autoriser » : pour cela, ou pour refuser en expliquant, ouvre la conversation.
- Pour tout le reste, la ligne montre la demande et un bouton « Répondre » qui ouvre l’agent : une **question** (son texte est affiché), un **plan** (« Claude propose un plan »), un agent qui attend dans la vue Claude Code (« Attend ta réponse dans l’interface de Claude Code »), une demande que Claude Code refuserait par défaut, ou un outil dont le résumé ne dit pas tout ce qu’il demande (« À lire dans la conversation avant de répondre. »). Une demande dont l’argument dépasse quatre lignes est aussi renvoyée à la conversation (« Trop long pour être lu ici : lis-la et réponds dans la conversation. »).

Seule la première demande de chaque agent est montrée ; quand tu y réponds, la suivante prend sa place. Quand une ligne change, ou qu’elle glisse parce qu’une ligne au-dessus a disparu, ses boutons restent inactifs une demi-seconde : un clic destiné à la demande d’avant ne répond pas à la suivante.

### Au clavier

| Touches | Effet |
|---|---|
| `↑` / `↓` | Ligne précédente / suivante, d’un groupe à l’autre |
| `Home` / `End` | Première / dernière ligne |
| `Entrée` ou `Espace` | Ouvre l’agent de la ligne |
| `Échap` | Referme la vue d’ensemble |

## Passer d’un agent à l’autre

| Touches | Effet |
|---|---|
| `Ctrl+J` | Met le focus sur la carte de l’agent suivant qui attend une réponse ou qu’il faut regarder (une question, une fin ou une erreur que tu n’as pas encore vues), dans tous les projets, en commençant par celui qui attend depuis le plus longtemps. La conversation affichée ne change pas : `1`, `2`… répondent depuis la carte, `Entrée` ouvre l’agent. |
| `Ctrl+Tab` / `Ctrl+Maj+Tab` | Agent suivant / précédent du projet affiché, dans l’ordre de la barre latérale, en bouclant et sans les agents archivés. Sous macOS, c’est bien `Ctrl+Tab`, pas `⌘Tab`. |
| `Ctrl+1` … `Ctrl+9` | Va au projet n (voir [l’interface](interface)). |

`Ctrl+J` et `Ctrl+Tab` marchent aussi depuis un terminal. Le bouton « N en attente » de la barre d’état, en bas, fait la même chose que `Ctrl+J`.

Pour garder un œil sur les autres projets sans les ouvrir, regarde leurs tuiles, dans la barre des projets : une pastille donne le nombre d’agents qui attendent une réponse, un point vert signale qu’un agent travaille, l’infobulle compte les fichiers modifiés (« Δ 7 »), et une tuile qui clignote en ambre, vert ou rouge signale un agent à voir (voir [les notifications](notifications)). La barre d’état compte les agents « actifs », « en attente » et « terminés ».

## Rechercher dans les conversations

`Ctrl+K` ouvre « Rechercher dans les conversations », une palette qui cherche dans tout ce que toi et tes agents avez écrit. (Dans un terminal sous Windows et Linux, `Ctrl+K` reste au shell.)

- **Ce qui est cherché** : tes messages et ceux de Claude, les commandes et les fichiers des appels d’outils avec leur sortie, les questions et leurs réponses, les demandes d’autorisation, les plans, les avis, les erreurs de tour, et le nom des fichiers joints. La réflexion, repliée dans la conversation, n’est pas cherchée.
- **Comment** : la recherche ne tient compte ni de la casse ni des accents (« cafe » trouve « café »), traite `\` et `/` comme équivalents dans un chemin, et lit une apostrophe typographique comme une droite. Elle commence après deux caractères, un instant après ta dernière frappe, et lit les journaux des conversations sur le disque, y compris ceux des agents en vue Claude Code.
- **Où** : « Ce projet » ou « Tous les projets » (le premier tant qu’un projet est affiché), et la case « Agents archivés », cochée par défaut.

Les résultats sont groupés par agent, le plus récemment actif en premier, avec le nom du projet et la mention « archivé » s’il y a lieu. Chaque résultat montre un extrait avec ce qui correspond en surbrillance, et son ancienneté (« il y a 3 h »). Les messages les plus récents de chaque agent viennent d’abord.

`↑` et `↓` choisissent un résultat, `Entrée` ou un clic l’ouvre, `Échap` ferme la palette. L’agent s’ouvre sur sa conversation (l’éditeur, s’il était ouvert, laisse la place), défilée jusqu’au message, qui est mis en évidence pendant deux secondes ; si le message est ancien, les éléments qui le précèdent sont affichés d’abord. Un agent archivé s’ouvre en lecture. Si l’agent a été supprimé entre-temps, un message dit « Cet agent n’existe plus. »

La palette s’arrête à 200 résultats (« Les 200 premiers résultats : précise ta recherche. ») ou à 5 secondes de lecture (« … recherche arrêtée au bout de 5 s : précise ta recherche. ») : ajoute des mots pour la resserrer.
