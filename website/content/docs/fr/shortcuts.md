# Raccourcis clavier

Tous les raccourcis d’Escouade, classés par usage : navigation, conversation, questions et autorisations, éditeur, terminaux, panneaux, tickets et réglages. Chacun dit où il marche ; la page dit aussi ce qui change sous macOS et ce qui reste au shell dans un terminal.

## Lire cette page

Les raccourcis s’écrivent comme l’app les écrit en français : `Ctrl+Maj+A`, `Ctrl+Entrée`, `Échap`. La colonne « Où » dit où chacun marche :

- « Partout » : n’importe où dans la fenêtre, tant qu’aucune fenêtre (réglages, commit, ticket…) n’est ouverte. Sous Windows et Linux, un terminal qui a le focus garde pourtant certaines de ces touches pour son shell : voir [Terminaux](#terminaux).
- « Partout, terminaux compris » : même quand un terminal a le focus.
- Un endroit précis (le champ de message, l’éditeur, un formulaire…) : le raccourci ne marche que là.

`Échap` ferme la fenêtre ouverte. Deux précisions sur le clavier :

- Les raccourcis globaux ignorent toute combinaison avec `Alt`, parce que `Ctrl+Alt` est `AltGr` sur un clavier français, sauf `Ctrl+Alt` et une flèche, qui ne tape rien, pour passer d’un volet à l’autre de l’[écran partagé](split-screen). Les raccourcis avec `Alt` sont propres à une zone : `Alt+1` … `Alt+9` (questions), `Alt+←` et `Alt+→` (éditeur), `Alt+↑` et `Alt+↓` (réglages des worktrees), `Alt+clic` (éditeur).
- Les chiffres de `Ctrl+1` … `Ctrl+9` et de `Alt+1` … `Alt+9` se lisent sur la touche physique : sur un clavier AZERTY, pas besoin de `Maj`.

## Sous macOS

`Ctrl` devient `⌘`, et `Ctrl+Maj` devient `⇧⌘` : `Ctrl+N` s’écrit `⌘N`, `Ctrl+Maj+A` s’écrit `⇧⌘A`. Les exceptions :

| Windows et Linux | macOS |
|---|---|
| `Ctrl+Tab`, `Ctrl+Maj+Tab` | Inchangés : `⌘Tab` change d’application. |
| `Alt+1` … `Alt+9`, `Alt+↑`, `Alt+↓`, `Alt+clic` | Inchangés : `Alt` est la touche Option (`⌥`). |
| `Alt+←`, `Alt+→` (éditeur) | `⌃-`, `⌃⇧-`, avec la touche Contrôle et non `⌘`. |
| `Ctrl+clic` (éditeur) | `⌘+clic`. |
| `Ctrl+Y` (éditeur, rétablir) | `⇧⌘Z`. |
| `Suppr` (arborescence de l’éditeur) | `⌘⌫` ; la touche `Suppr` marche aussi. |
| `Ctrl+C`, `Ctrl+V` (terminal) | `⌘C`, `⌘V`. Les touches `Ctrl` vont au shell. |

## Navigation

| Raccourci | Action | Où |
|---|---|---|
| `Ctrl+1` … `Ctrl+9` | Aller au projet n, dans l’ordre des onglets. | Partout, terminaux compris |
| `Ctrl+Tab` / `Ctrl+Maj+Tab` | Agent suivant / précédent du projet affiché. | Partout, terminaux compris |
| `Ctrl+Maj+A` | Ouvrir ou fermer la [vue d’ensemble](overview), qui réunit les agents de tous les projets. | Partout, terminaux compris |
| `Ctrl+K` | Rechercher dans les conversations : ce que toi et tes agents avez écrit, les commandes, les fichiers et les sorties d’outils, sans tenir compte de la casse ni des accents. Un résultat ouvre l’agent sur le message. | Partout |
| `Ctrl+P` | Ouvrir un fichier par son nom. `nom:42` l’ouvre à la ligne 42, `nom:42:7` à la ligne 42, colonne 7. Le fichier se cherche dans la source de l’éditeur s’il est ouvert, sinon dans le worktree de l’agent affiché, sinon dans le dossier du projet. | Partout |

## Palettes et listes

| Où | Touches |
|---|---|
| `Ctrl+K` et `Ctrl+P` | `↑` / `↓` choisissent, `Entrée` ouvre, `Échap` ferme. Sans rien taper, `Ctrl+P` propose les derniers fichiers ouverts : `Entrée` rouvre celui d’avant le fichier affiché. |
| Vue d’ensemble | `↑` / `↓`, `Début` et `Fin` changent de ligne. `Entrée` ou `Espace` ouvre l’agent de la ligne. `Échap` revient à la vue d’avant. |
| Sélecteur de branche (barre d’état) | `↑` / `↓` choisissent, `Entrée` valide la ligne choisie, `Échap` ferme. |
| Branche du sélecteur, commit du graphe git | `Menu` ou `Maj+F10` ouvre le menu qu’ouvre le clic droit. |
| Menu contextuel | `↑` / `↓`, `Début` et `Fin` choisissent, `Échap` ferme. |
| Onglets des réglages | `↑` / `↓`, `Début` et `Fin` changent d’onglet. |
| Carte d’un ticket du Kanban | `Entrée` ou `Espace` l’ouvre : le formulaire du ticket s’il est « À faire », sinon son agent. |

## Agents et conversation

| Raccourci | Action | Où |
|---|---|---|
| `Ctrl+N` | Nouvel agent dans le projet affiché. | Partout |
| `Ctrl+J` | Mettre le focus sur la carte du prochain agent qui attend ta réponse ou qui est à voir (une question, une fin ou une erreur pas encore vues), tous projets confondus, sans quitter la conversation affichée ; `Entrée` l’ouvre. | Partout, terminaux compris |
| `1`, `2`… | Répondre depuis la liste des agents : la réponse ou la décision de ce numéro, sur la carte qui a le focus, sinon sur la première qui attend. | Hors d’un champ de saisie, d’un terminal et de l’éditeur |
| `Entrée` | Envoyer le message. Pendant que Claude travaille, il en tient compte dès sa prochaine étape. | Champ de message |
| `Maj+Entrée` | Aller à la ligne. | Champ de message |
| `Échap` | Interrompre Claude. | Champ de message, pendant que l’agent travaille |
| `↑` | Rappeler le dernier message envoyé, pour le renvoyer tel quel ou modifié (l’original reste dans la conversation). | Champ de message vide |
| `Ctrl+Maj+Espace` | [Dicter](chat#dicter-un-message) : un appui bref commence et un second finit, maintenu il dicte tant qu’il est enfoncé ; le texte va au curseur, sans être envoyé. Modifiable dans [Réglages › Voix](settings-voice#utilisation). | Le champ de message, la description d’un ticket, le terminal de la vue Claude Code |
| `Échap` | Annuler la dictée en cours, sans rien écrire. | Pendant une dictée |
| `Échap` | Quitter la [conversation vocale](chat#parler-avec-l-agent-conversation-vocale) : le micro se ferme, comme avec « Arrêter ». | Pendant une conversation vocale |
| `↑` / `↓`, `Entrée` ou `Tab`, `Échap` | Choisir, prendre ou fermer une suggestion `@fichier` ou `/commande`. | Champ de message, liste ouverte |

## Questions et autorisations

Quand Claude pose une question ou demande une autorisation, la carte se traite au clavier :

| Raccourci | Action |
|---|---|
| `Ctrl+Entrée` | « Autoriser » la demande en attente (pour un plan, « Approuver le plan »). Pour une question, valide la réponse quand elle est complète (« Valider »). |
| `Ctrl+Maj+Entrée` | « Toujours autoriser » (pour un plan, « Approuver et accepter les édits »), seulement si la carte propose ce bouton. |
| `Alt+1` … `Alt+9` | Choisir l’option n de la question en attente ; pour un choix multiple, cocher ou décocher. |
| Un texte dans le champ de message, puis `Entrée` | Refuser la demande avec ce texte pour message, ou répondre librement à la question. |

Ces touches marchent dans la conversation, champ de message compris, ou quand rien d’autre n’a le focus. Elles ne marchent ni dans un terminal, ni dans l’éditeur, ni quand une fenêtre est ouverte. Les détails :

- Plusieurs demandes peuvent attendre : le clavier répond à la première.
- Une demande qui vient d’arriver ignore ces touches pendant une demi-seconde, `Entrée` du champ de message comprise : le texte déjà tapé reste là. Un `Ctrl+Entrée` prévu pour envoyer ton message ne répond donc pas à une demande arrivée au même moment, et un deuxième `Ctrl+Entrée` ne répond pas à la demande suivante. Une touche maintenue enfoncée ne répond qu’une fois.
- Une demande que Claude Code refuserait par défaut ne s’autorise qu’avec son bouton.
- Une question à choix unique part dès que tu choisis une option. Avec plusieurs questions ou un choix multiple, `Ctrl+Entrée` passe à la question suivante qui n’a pas de réponse, puis valide quand toutes en ont une.
- Dans la [vue d’ensemble](overview), `Ctrl+Entrée` ne fait rien : une autorisation qui s’y lit en entier s’accorde ou se refuse avec ses boutons.

## Éditeur

L’[éditeur](editor) intégré. Les raccourcis marchent tant qu’il est ouvert.

### Le texte

| Raccourci | Action | Où |
|---|---|---|
| `Ctrl+S` | Enregistrer le fichier. | Éditeur ouvert |
| `Ctrl+Maj+F` | Rechercher dans tous les fichiers de la source. | Éditeur ouvert |
| `Ctrl+F` | Rechercher dans le fichier. `F3` ou `Ctrl+G` va à l’occurrence suivante, avec `Maj` à la précédente. | Texte du fichier |
| `Ctrl+clic` ou `F12` | Aller à ce que désigne le mot : un fichier importé, un lien, un chemin, la définition d’un nom. S’il y a plusieurs définitions, une liste s’ouvre. | Texte du fichier |
| `Maj+F12` | Trouver les références du nom sous le curseur. | Texte du fichier |
| `Alt+←` / `Alt+→` | Revenir là où tu étais avant un saut / y retourner. Les boutons « précédent » et « suivant » de la souris font de même. | Texte du fichier |
| `Alt+clic` | Ajouter un curseur. | Texte du fichier |
| `Ctrl+Z` / `Ctrl+Y` | Annuler / rétablir. | Texte du fichier |
| `Tab` / `Maj+Tab` | Indenter / désindenter. | Texte du fichier |

Le texte est un éditeur CodeMirror : ses autres raccourcis habituels y marchent aussi.

### L’arborescence

Les touches suivantes valent quand une ligne de l’arborescence a le focus.

| Raccourci | Action |
|---|---|
| `↑` / `↓`, `Début`, `Fin` | Changer de ligne. |
| `→` | Déplier le dossier, puis aller à son premier élément. |
| `←` | Replier le dossier, ou aller au dossier parent. |
| `Entrée` ou `Espace` | Ouvrir le fichier, ou déplier le dossier. |
| `F2` | Renommer. |
| `Suppr` | Supprimer vers la corbeille, après confirmation. |

Dans les résultats de « Rechercher dans les fichiers », `Entrée` dans le champ lance la recherche, `↓` passe aux résultats et `↑` depuis le premier revient au champ.

## Terminaux

| Raccourci | Action | Où |
|---|---|---|
| `Ctrl+T` | Nouveau [terminal](terminals) dans le projet affiché. | Partout |
| `Ctrl+Maj+F` | Rechercher dans la sortie. `Entrée` va à l’occurrence suivante, `Maj+Entrée` à la précédente, `Échap` ferme la recherche. | Terminal, journal d’une commande de lancement |
| `Ctrl+C` | Copier la sélection ; sans sélection, la touche va au shell (interruption). | Terminal, sous Windows et Linux |
| `Ctrl+V` | Coller. | Terminal, sous Windows et Linux |
| `Ctrl+Maj+C` | Copier la sélection. | Terminal, sous Windows et Linux |
| `⌘C` / `⌘V` | Copier / coller. | Terminal, sous macOS |
| `Ctrl+V` | Va au shell ; dans la [vue Claude Code](claude-code-view), c’est la touche de Claude Code pour lire l’image du presse-papiers. | Terminal, sous macOS |
| `Maj+PageHaut`, `Maj+PageBas`, `Maj+Début`, `Maj+Fin` | Faire défiler le journal. | Journal d’une commande de lancement (lecture seule) |

Le journal d’une commande de lancement est en lecture seule : hors la copie et le défilement, toutes ses touches vont à l’app, dont `Ctrl+K` et `Ctrl+P`.

### Ce qui reste au shell

Sous Windows et Linux, un terminal qui a le focus garde pour son shell les touches `Ctrl` suivies d’une lettre, dont `Ctrl+K` (couper la fin de la ligne), `Ctrl+P` (commande précédente), `Ctrl+N` et `Ctrl+T` : pour chercher dans les conversations, ouvrir un fichier, créer un agent ou un terminal, clique d’abord hors du terminal. L’app n’y prend que `Ctrl+Tab` et `Ctrl+Maj+Tab`, `Ctrl+1` … `Ctrl+9`, `Ctrl+J` (pour un shell, ce n’est qu’un saut de ligne, que `Entrée` fait déjà), `Ctrl+,`, `Ctrl+Maj+A`, et le zoom (`Ctrl+=`, `Ctrl+-`, `Ctrl+0`). `Ctrl+Maj` suivi d’une lettre n’envoie rien au shell : `Ctrl+Maj+B` et `Ctrl+Maj+L` marchent aussi dans un terminal, où `Ctrl+Maj+F` cherche dans sa sortie.

Sous macOS, les raccourcis de l’app utilisent `⌘`, que le shell ne reçoit jamais : ils marchent tous dans un terminal, sauf `⌘C`, `⌘V` et `⌘A` (tout sélectionner), qui sont ceux du terminal. Les touches `Ctrl` vont au shell.

## Panneaux et disposition

| Raccourci | Action | Où |
|---|---|---|
| `Ctrl+Maj+L` | Passer de la disposition classique à « Conversation et fichiers côte à côte », et inversement. | Partout, terminaux compris |
| `Ctrl+Maj+B` | Afficher ou masquer le panneau des fichiers non commités. | Partout, terminaux compris, en disposition classique (côte à côte, le panneau est toujours affiché) |
| `Ctrl+\` | [Écran partagé](split-screen) : partager ce qui est affiché avec l’agent suivant, ou revenir à un seul volet. Sur un clavier AZERTY : `Ctrl+*`. | Partout, terminaux compris |
| `Ctrl+Maj+\` | Tourner la séparation du volet actif. | Écran partagé, terminaux compris |
| `Ctrl+Alt+←` `→` `↑` `↓` | Passer au volet de ce côté. | Écran partagé, terminaux compris |
| `Ctrl+Maj+W` | Fermer le volet actif. | Écran partagé, terminaux compris |
| `Ctrl+clic` | Sur une carte d’agent, la ligne d’un terminal ou d’un lancement : l’ouvrir à côté. | Barre latérale |

Un séparateur qui a le focus (avec `Tab`) se règle au clavier :

- La colonne des fichiers de l’éditeur : `←` / `→` la rétrécissent ou l’élargissent, `Début` et `Fin` vont à la largeur minimale et maximale.
- La liste des tâches du bandeau « Plan » : `↑` / `↓` changent sa hauteur, `Début` et `Fin` vont au minimum et au maximum, `Entrée` ou `Échap` rétablissent la hauteur habituelle.

Un double-clic sur un séparateur rétablit aussi sa taille.

## Tickets

| Raccourci | Action | Où |
|---|---|---|
| `Ctrl+Entrée` | Ajouter ou enregistrer le ticket, depuis n’importe quel champ. | Fenêtre d’un ticket |
| `Entrée` | Ajouter ou enregistrer le ticket. Dans les champs de plusieurs lignes, `Entrée` va à la ligne. | Titre du ticket |
| `Échap` | Fermer la fenêtre ; si tu as tapé quelque chose, l’app demande d’abord si tu l’abandonnes. | Fenêtre d’un ticket |

## Réglages

| Raccourci | Action | Où |
|---|---|---|
| `Ctrl+,` | Ouvrir les réglages. | Partout, terminaux compris |
| `Ctrl+=` (ou `Ctrl++`) / `Ctrl+-` | Agrandir / réduire toute l’interface d’un palier (voir [« Apparence »](settings-application#apparence)). | Partout, terminaux compris, fenêtre ouverte ou non |
| `Ctrl+0` | Revenir au zoom de 100 %. | Partout, terminaux compris, fenêtre ouverte ou non |
| `Échap` | Fermer les réglages. | Réglages ouverts |
| `Alt+↑` / `Alt+↓` | Monter / descendre une commande de préparation ou de démontage, depuis n’importe quel champ de la commande. | Réglages d’un projet, listes des commandes de worktree |
