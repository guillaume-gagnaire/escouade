---
title: "Fenêtre"
subject: "Fenêtre"
sources: ["docs/SPEC.md","src/components/ContextMenu.svelte","docs/security/audit-2026-11/rapport.md","src-tauri/src/sentfiles.rs (RUNNABLE, ESC-628)","src-tauri/src/quarantine.rs (ESC-628)","Microsoft, Blocked attachments in Outlook (support.microsoft.com/outlook/blocked-attachments-in-outlook)"]
tickets: ["ESC-71","ESC-109","ESC-101","ESC-380","ESC-593","ESC-628"]
decisions: [299]
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
- Le choix s'applique tout de suite et s'enregistre aussitôt, comme tout réglage (voir *Réglages*). Barres de défilement, champs et menus natifs suivent le thème.
- La fenêtre native prend son thème avant d'être montrée : une app claire ne s'affiche jamais en sombre d'abord.

### Zoom et tailles du texte

- « Zoom de l'interface » : le zoom de toute l'interface, comme celui d'un navigateur ou de VS Code. Le texte, les espacements et les icônes changent ensemble, terminaux et éditeur compris.
- Paliers : 80, 90, 100 (défaut), 110, 125, 150, 175 et 200 %. Une valeur hors palier est ramenée au palier le plus proche.
- Raccourcis : `Ctrl+=` (ou `Ctrl++`, `Ctrl+Maj+=`), `Ctrl+-` et `Ctrl+0`, avec `Cmd` sous macOS. Le pavé numérique marche aussi (`+`, `-`, `0`). `Ctrl+0` est lu aussi sur la touche physique (« à » en AZERTY). Jamais avec `Alt`. Ils marchent partout, dans un terminal (le shell n'en reçoit rien) comme dans une fenêtre ouverte.
- Le zoom s'applique tout de suite et s'enregistre 400 ms après le dernier changement : cinq `Ctrl+=` font un seul enregistrement.
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

### Menus

- Une entrée marquée d'un chevron ouvre un sous-menu à côté du menu, à la hauteur de l'entrée, du côté du chevron : à droite (› au bout de l'entrée), ou à gauche (‹ à son début) quand le menu n'a pas à sa droite la place du plus large des sous-menus (380 px) et en a plus à sa gauche. Tous les sous-menus d'un menu s'ouvrent du même côté, connu avant qu'ils s'ouvrent. Un seul niveau. Sont des sous-menus : « Lancer » et « Continuer sur un autre harness » du menu d'un agent, « Dans le worktree d'un agent » du menu `+` des terminaux (voir *Barre latérale*, *Comptes*).
- Le sous-menu s'ouvre quand le pointeur reste 150 ms sur son entrée, et tout de suite au clic sur l'entrée (un toucher, un trackpad, un clic avant le délai), qui ne ferme pas le menu. Il reste ouvert tant que le pointeur ne s'est pas posé 150 ms sur une autre entrée du menu : on le rejoint en diagonale, et sortir des deux menus ne le ferme pas. Une entrée grisée n'ouvre pas le sien.
- Au clavier : →, Entrée ou Espace sur l'entrée ouvrent le sous-menu, le focus sur sa première entrée ; ↑ ↓ Début Fin y tournent en boucle ; ← ou Échap le ferment et rendent le focus à l'entrée ; Tab ferme tout. Le focus hors des menus, Échap ferme d'abord le sous-menu, puis le menu.
- Un menu ou un sous-menu plus haut que la fenêtre s'arrête à 8 px de ses bords et défile (molette, trackpad, flèches). Chaque bord qui cache des entrées s'estompe sur 28 px, un chevron en son milieu ; l'entrée qui prend le focus sort du dégradé. Faire défiler un menu ferme son sous-menu.

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

### Sécurité de la fenêtre

Le contenu des agents s'affiche comme du texte, assaini (voir *Conversation*). Si un message passait un jour l'assainissement, ce qu'il apporterait ne pourrait ni lancer de script, ni redessiner l'interface, ni faire ouvrir par le système ce qu'il veut (audit de sécurité de la 2.2.0, SEC-05 et SEC-12).

**Ouvrir par le système.**

- Un fichier dans son programme, un dossier dans le gestionnaire de fichiers, un élément montré dans son dossier : le système ne les ouvre que dans le dossier de données d'Escouade, le dossier d'un projet, le dossier de ses nouveaux worktrees (« Dossier des worktrees »), et le dossier et le worktree de chacun de ses agents. Cela vaut pour « Ouvrir le dossier » d'un projet, du dossier de données et des copies de l'état, pour « Afficher dans l'Explorateur » de l'éditeur, pour un fichier qu'un agent a envoyé, une image de son message, et un fichier d'une demande de « À toi ».
- Le cœur juge le chemin là où il mène vraiment : un lien, une jonction, `..` et un chemin `\\?\C:\…` sont résolus avant. Un chemin réseau (`\\serveur\partage`) qui n'est pas, tel qu'il est écrit, dans un de ces dossiers est refusé sans que le disque soit interrogé, puisque l'interroger joindrait déjà le serveur. Un projet sous WSL (`\\wsl.localhost\…`) y est. Un périphérique de Windows (`\\.\pipe\…`) et un chemin incomplet (`C:notes.txt`, relatif) sont refusés.
- Montré dans son dossier, un élément n'est pas suivi : un lien d'un projet vers ailleurs est montré comme le lien. Ouvert, il est refusé.
- Le refus se dit dans un toast, avec sa raison : « « D:\ailleurs » est hors des dossiers des projets, de leurs worktrees et du dossier de données d'Escouade : Escouade ne le fait pas ouvrir par le système. » Quand un lien ou `..` mène ailleurs, le toast dit où : « « … » mène à « … », hors des dossiers… ». Un chemin qui n'existe plus : « « … » n'existe pas ou ne se lit pas ».
- Un fichier envoyé par un agent, ou une image de son message, qui est hors de ces dossiers (dans un dossier temporaire du poste, par exemple) ne s'ouvre plus et ne se montre plus dans son dossier. Ce que la conversation en affiche reste affiché.
- **Escouade n'ouvre jamais, même dans un projet, un fichier que le système lancerait, installerait, monterait ou suivrait en l'ouvrant** (ESC-628), sous Windows, macOS et Linux à la fois (un fichier d'un système se reçoit sur un autre) : ni par ce que la fenêtre ouvre d'elle-même, ni quand un agent l'a envoyé (voir *Conversation*), ni une pièce jointe de « À toi ». Il se montre dans son dossier. Quatre familles, chaque type justifié dans `sentfiles::RUNNABLE` :
  - **un programme ou un script** : programmes, scripts de Windows (batch, PowerShell et ses modules, manifestes et fichiers de mise en forme, Windows Script Host, HTA, Python), consoles de gestion (`.msc`), aides compilées (`.chm`, `.hlp`), dépanneurs (`.diagcab`), compléments Excel (`.xll`), applications XAML et ClickOnce, configurations de Windows Sandbox, archives web (`.mht`, `.mhtml`) ; sous macOS les applications, scripts de Terminal (`.command`, `.tool`), réglages de Terminal (`.terminal`), flux Automator, AppleScript ; sous Linux les scripts, lanceurs (`.desktop`), AppImage, `.bin` ; Java (`.jar`, `.jnlp`) partout ; un fichier marqué exécutable sous macOS et Linux, quel que soit son nom ;
  - **un installeur** : MSI, MSU, paquets d'apps (`.appinstaller`, `.msix`, `.appx` et leurs lots), gadgets, entrées du registre ; `.pkg`, `.mpkg`, extensions du noyau, panneaux de réglages et économiseurs d'écran de macOS ; `.run`, `.deb`, `.rpm`, `.snap`, `.flatpakref` ;
  - **une image de disque**, montée à l'ouverture : `.iso`, `.img`, `.vhd`, `.vhdx`, `.dmg` ;
  - **un raccourci**, qui suit ce qu'il désigne, un programme ou le dossier d'un autre ordinateur (avec l'identité de l'utilisateur) aussi : `.lnk`, `.url`, `.website`, `.scf`, bibliothèques et recherches enregistrées (`.library-ms`, `.search-ms`, `.searchconnector-ms`), raccourcis des Paramètres, thèmes ; `.webloc`, `.inetloc`, `.fileloc` de macOS.
- Le nom est jugé quelle que soit sa casse, ses points et espaces finaux, que Windows retire (`a.msc.`, `a.iso `), et son flux (`a.msc::$DATA`, `notes.txt:a.lnk`).
- Le refus dit pourquoi, selon la famille, et propose le dossier : « « rapport.terminal » est un programme ou un script : l'ouvrir le lancerait. Escouade ne l'ouvre pas ; « Afficher dans le dossier » le montre sans le lancer. » (« est un installeur : l'ouvrir installerait une application ou un composant », « est une image de disque : l'ouvrir monterait le disque et ses fichiers », « est un raccourci : l'ouvrir suivrait ce qu'il désigne… », « est une page : l'ouvrir ferait lancer ses scripts par le navigateur »). Son toast porte le bouton « Afficher dans le dossier ». Le cœur marque ce refus (`not-opened|`) pour que la fenêtre offre ce bouton ; un fichier que la fenêtre sait déjà refusé n'a que « Afficher dans le dossier », sans « Ouvrir ».
- **La quarantaine de macOS** : un fichier qu'Escouade copie ou reçoit (une pièce jointe d'une demande de « À toi », une image qu'un agent envoie, copiée dans le dossier de données) porte l'attribut `com.apple.quarantine`, pour que Gatekeeper le juge à son ouverture ; un fichier qu'un agent envoie et qui reste où il l'a laissé le reçoit quand l'utilisateur l'ouvre. Un fichier déjà en quarantaine garde la sienne, qui dit d'où il vient. Sous Windows, la copie garde la marque du web (`Zone.Identifier`) que sa source porte déjà ; Escouade n'en invente pas. Linux n'a pas de telle marque.
- La fenêtre n'a aucune permission de Tauri pour ouvrir un chemin : chaque ouverture passe par le cœur. Les adresses `http(s)` gardent leur chemin, le navigateur (voir *Liens*).

**La politique de contenu.**

- Aucun script en ligne, aucun `eval`, rien de chargé ni de joint ailleurs que l'app. Aucun formulaire n'envoie rien (`form-action 'none'`), aucune balise `<base>` ne déplace les adresses (`base-uri 'none'`), aucun objet ne se charge (`object-src 'none'`), et aucune page ne met la fenêtre dans un cadre (`frame-ancestors 'none'`).
- Une balise `<style>` ne s'applique qu'avec le nonce que Tauri donne à la page à chaque chargement, sur la balise `<style>` de `index.html` : `style-src 'self' 'nonce-…'`, sans `'unsafe-inline'`. Les balises que l'interface crée elle-même reçoivent ce nonce à leur création : celles du terminal, de l'éditeur, du thème d'un plugin. Une balise apportée par un texte n'en a pas et ne s'applique pas : le navigateur le signale comme une violation de la politique.
- Les attributs `style`, qu'écrit l'interface, restent permis (`style-src-attr 'unsafe-inline'`). L'assainissement retire déjà l'attribut `style` des messages.
- Cela vaut pour la fenêtre principale (la barre des fenêtres « Pages » est retirée, ESC-657). La vue d'un plugin est dessinée dans la fenêtre principale, sous sa politique. La page d'un lancement est celle de l'app développée, sous sa propre politique, sans aucune commande d'Escouade (voir *La page du lancement*). Le serveur de développement (`npm run tauri dev`) n'applique pas de politique : seule l'app construite le fait.

**Les commandes de la fenêtre** restent d'un bloc : la fenêtre principale peut appeler toutes les commandes du cœur. Leur découpage par capacité est à reprendre avec l'app mobile, qui appellera les mêmes commandes.

## Réglages

Réglages › Application › « Affichage » :

- « Thème » : en segments « Système », « Clair », « Sombre ».
- « Zoom de l'interface » : en segments de 90 à 150 % (choix 35 d'UX-NOTES), plus le palier en cours quand il est hors de cette plage ; le clavier atteint les autres. Les paliers trop grands pour la fenêtre sont grisés : « Trop grand pour la taille de la fenêtre ».
- « Taille du texte des terminaux », « Taille du texte de l'éditeur ».

Tous s'appliquent et s'enregistrent tout de suite.

## Limites

- Le zoom ne change pas la taille minimale de la fenêtre : un palier qui ne tient pas dans la fenêtre n'est pas appliqué.
- Un fichier qu'un agent envoie hors des dossiers des projets, de leurs worktrees et du dossier de données ne s'ouvre pas depuis la conversation : l'agent peut le mettre dans son worktree.
- La liste des types refusés est une liste de noms : un type que le système apprendrait à lancer demain n'y est qu'une fois ajouté. Sous macOS, la quarantaine laisse Gatekeeper juger aussi ce que la liste ne connaît pas.
