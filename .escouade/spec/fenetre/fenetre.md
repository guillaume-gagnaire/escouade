---
title: "Fenêtre"
subject: "Fenêtre"
sources: ["docs/SPEC.md"]
tickets: ["ESC-71","ESC-109","ESC-101"]
decisions: []
compared: null
order: 5
---

## Ce que c'est

La fenêtre principale d'Escouade. Elle est sans cadre et suit un thème clair ou sombre, teinté par la couleur du projet actif. Son zoom est réglable, comme dans un navigateur. Elle ne quitte jamais l'interface de l'app : un lien s'ouvre dans le navigateur. Fermée, elle se réduit dans la zone de notification et les agents continuent.

## Comportement

### Cadre

- Fenêtre sans cadre : la barre de titre de l'app sert à déplacer la fenêtre, avec les boutons Windows ─ ☐ ✕ à droite. Sous macOS, la barre de titre native garde ses boutons. Ce que porte la barre de titre : voir *Barre des projets*.
- Polices Hanken Grotesk et JetBrains Mono, embarquées (aucun accès réseau).
- La taille minimale de la fenêtre est 1000 × 600 px : l'interface est prévue pour elle.

### Thème

- « Thème » : « Système » (défaut), « Clair » ou « Sombre ». « Système » suit le thème du système, et ses changements à chaud. Une valeur inconnue vaut « Système ».
- L'interface est teintée par la couleur du projet actif.
- Le choix s'applique tout de suite et s'enregistre aussitôt, hors du brouillon des réglages. Barres de défilement, champs et menus natifs suivent le thème.
- La fenêtre native prend son thème avant d'être montrée : une app claire ne s'affiche jamais en sombre d'abord.

### Zoom et tailles du texte

- « Zoom de l'interface » : le zoom de toute l'interface, comme celui d'un navigateur ou de VS Code. Le texte, les espacements et les icônes changent ensemble, terminaux et éditeur compris.
- Paliers : 80, 90, 100 (défaut), 110, 125, 150, 175 et 200 %. Une valeur hors palier est ramenée au palier le plus proche.
- Raccourcis : `Ctrl+=` (ou `Ctrl++`, `Ctrl+Maj+=`), `Ctrl+-` et `Ctrl+0`, avec `Cmd` sous macOS. Le pavé numérique marche aussi (`+`, `-`, `0`). `Ctrl+0` est lu aussi sur la touche physique (« à » en AZERTY). Jamais avec `Alt`. Ils marchent partout, dans un terminal (le shell n'en reçoit rien) comme dans une fenêtre ouverte.
- Le zoom s'applique tout de suite et s'enregistre 400 ms après le dernier changement : cinq `Ctrl+=` font un seul enregistrement. Il est hors du brouillon des réglages : « Enregistrer » ne le reprend pas, « Annuler » ne le défait pas.
- Zoom affiché : le palier choisi, ou le plus grand que la fenêtre tient. Un palier tient quand la taille de la fenêtre à 100 %, divisée par le zoom, fait au moins 1000 × 600 ; le zoom ne descend jamais sous 100 % pour autant. Il est recalculé à chaque redimensionnement, et `Ctrl+=` s'arrête au dernier palier qui tient.
- Le palier choisi reste gardé et revient quand la fenêtre grandit : « La fenêtre est trop petite pour 200 % : l'interface est à 125 % tant qu'elle ne grandit pas. »
- La fenêtre est zoomée avant d'être montrée : elle n'est jamais dessinée à 100 % d'abord.
- « Taille du texte des terminaux » et « Taille du texte de l'éditeur » : de 9 à 24 px, par demi-pixel, 12,5 par défaut. Le zoom s'y ajoute. Elles s'appliquent et s'enregistrent comme le zoom.
- Chaque terminal (shells, logs, vue Claude Code) prend sa taille de texte. Ceux qui sont affichés s'ajustent à leur zone, avec leur pseudo-terminal redimensionné ; les autres s'ajustent quand ils s'affichent. Les lignes de l'éditeur font 1,6 fois la taille du texte.

### Infobulles

- Une seule sorte d'infobulle, celle de l'app, jamais celle du système. Elle a le style de la barre de statut : 11,5 px, sur plusieurs lignes si besoin.
- Elle s'affiche après 400 ms de survol. Elle s'affiche tout de suite quand Tab ou une flèche amène le focus sur l'élément, mais pas au focus d'un clic, ni au focus qu'une fenêtre qui se ferme rend à son bouton. Pendant 300 ms après qu'une infobulle a disparu, la suivante s'affiche tout de suite.
- Une seule à la fois : l'élément le plus intérieur sous le pointeur ou le focus l'emporte, s'il a quelque chose à dire. Revenu sur l'élément qui l'entoure, le pointeur retrouve l'infobulle de celui-ci.
- Elle est centrée au-dessus de l'élément, en dessous faute de place, et toujours dans la fenêtre.
- Elle reste tant que le pointeur ou le focus est sur l'élément. Elle part au clic, et ne revient pas avant que le pointeur ressorte. Elle part aussi au défilement de ce qui contient l'élément (pas quand une conversation suit la réponse en cours), quand la fenêtre perd le focus, et avec Échap.
- Échap suit WCAG 1.4.13 : le focus ne bouge pas. Quand le focus est dans l'élément, Échap ne fait rien d'autre que ce que l'élément en fait lui-même : il ne ferme pas une fenêtre et n'interrompt pas un tour.
- Un texte coupé par des points de suspension n'a d'infobulle que s'il est coupé.
- Lecteur d'écran : un contrôle qui n'affiche qu'un symbole ou une icône (⚙, ×, ⌕) est nommé par son infobulle. Un autre contrôle est décrit par elle tant qu'elle est affichée. Un élément hors de l'ordre de tabulation reçoit le texte de son infobulle comme description.
- Le graphique des stats garde son infobulle riche (titre, séries), au même style.

### Fermer et quitter

- Fermer la fenêtre la réduit dans la zone de notification (tray) : les agents continuent.
- Menu de l'icône de la zone de notification : « Afficher », « Recharger l'interface », « Quitter ».
- « Quitter » arrête l'app proprement. Au relancement, chaque agent reprend sa session.
- « Recharger l'interface » recharge la fenêtre sur l'interface de l'app, quand elle reste bloquée ou blanche. Si la fenêtre est sur l'app et que des fichiers de l'éditeur ne sont pas enregistrés, un dialogue du système demande d'abord : « Recharger » ou « Annuler ».

### Liens

- La fenêtre ne charge que les pages de l'app. Une page web (`http`, `https`) ou une adresse `mailto` s'ouvre hors de l'app, dans le navigateur ou le client de messagerie du système. Tout le reste (fichier, `about:blank`…) est refusé et ne s'ouvre nulle part. Chaque refus est écrit dans le journal, avec le schéma et l'hôte seulement.
- Dans l'interface, un lien `http`, `https` ou `mailto` s'ouvre dans le navigateur par un clic gauche, un clic du milieu, `Ctrl` ou `Maj` + clic, ou `Entrée` quand il a le focus. Un double clic ne l'ouvre qu'une fois. La fenêtre ne suit jamais le lien.
- Un autre lien (chemin relatif, `#…`, lien vide) ne fait rien.
- Dans les terminaux et la vue Claude Code, un lien OSC 8 (un texte différent de l'adresse, comme ceux qu'écrit Claude Code) s'ouvre dans le navigateur au clic, sans demander de confirmation. Seuls `http` et `https` y sont cliquables. Une URL écrite en clair s'ouvre aussi dans le navigateur.

## Réglages

Réglages › Application › « Affichage » :

- « Thème » : en segments « Système », « Clair », « Sombre ».
- « Zoom de l'interface » : en segments de 90 à 150 % (choix 35 d'UX-NOTES), plus le palier en cours quand il est hors de cette plage ; le clavier atteint les autres. Les paliers trop grands pour la fenêtre sont grisés : « Trop grand pour la taille de la fenêtre ».
- « Taille du texte des terminaux », « Taille du texte de l'éditeur ».

Tous s'appliquent et s'enregistrent tout de suite, hors du brouillon.

## Limites

- Le zoom ne change pas la taille minimale de la fenêtre : un palier qui ne tient pas dans la fenêtre n'est pas appliqué.
