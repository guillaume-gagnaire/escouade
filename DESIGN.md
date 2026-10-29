---
name: Escouade
description: L'environnement de développement pensé pour les agents, dans la langue visuelle de son site (2.0).
colors:
  project: "oklch(0.72 0.12 48)"
  brand-teal: "#35c1bd"
  frame: "color-mix(in oklch, var(--p) 8%, #08090a)"
  panel: "color-mix(in oklch, var(--p) 4.5%, #0e0f11)"
  raise: "color-mix(in oklch, var(--p) 6%, #15161a)"
  raise-2: "color-mix(in oklch, var(--p) 6%, #1b1c20)"
  well: "color-mix(in oklch, var(--p) 2%, #0a0b0d)"
  line: "rgba(255, 255, 255, 0.075)"
  line-2: "rgba(255, 255, 255, 0.12)"
  line-3: "rgba(255, 255, 255, 0.2)"
  text: "#f7f8f8"
  soft: "#d0d3d8"
  muted: "#989da6"
  dim: "#8f949e"
  run: "oklch(0.79 0.15 155)"
  wait: "oklch(0.84 0.14 82)"
  del: "oklch(0.74 0.14 22)"
  add: "oklch(0.8 0.13 155)"
  git-modified: "oklch(0.82 0.11 75)"
  primary: "#f2f3f4"
  on-primary: "#08090a"
  frame-light: "color-mix(in oklch, var(--p) 7%, #f0f1f3)"
  panel-light: "color-mix(in oklch, var(--p) 2.5%, #fdfdfd)"
  raise-light: "color-mix(in oklch, var(--p) 4%, #f4f5f7)"
  well-light: "color-mix(in oklch, var(--p) 2.5%, #f6f7f9)"
  text-light: "#15171c"
  soft-light: "#363a42"
  muted-light: "#555a63"
  run-light: "oklch(0.48 0.13 155)"
  wait-light: "oklch(0.5 0.11 70)"
  del-light: "oklch(0.5 0.18 25)"
  primary-light: "#1b1d22"
  on-primary-light: "#f7f8f8"
  chart-input: "#4c88d3"
  chart-cache: "#008b7a"
  chart-output: "#9555b0"
typography:
  title:
    fontFamily: "Inter Variable, Inter, system-ui, Segoe UI, sans-serif"
    fontSize: "15px"
    fontWeight: 600
    letterSpacing: "-0.01em"
  body:
    fontFamily: "Inter Variable, Inter, system-ui, Segoe UI, sans-serif"
    fontSize: "13px"
    fontWeight: 400
    lineHeight: 1.45
    fontFeature: "'cv11', 'ss03'"
  body-dense:
    fontFamily: "Inter Variable, Inter, system-ui, Segoe UI, sans-serif"
    fontSize: "12.5px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "Inter Variable, Inter, system-ui, Segoe UI, sans-serif"
    fontSize: "12px"
    fontWeight: 550
  meta:
    fontFamily: "Inter Variable, Inter, system-ui, Segoe UI, sans-serif"
    fontSize: "11.5px"
    fontWeight: 500
    fontFeature: "'tnum'"
  mono:
    fontFamily: "JetBrains Mono, ui-monospace, Consolas, monospace"
    fontSize: "0.92em"
    fontWeight: 400
rounded:
  xs: "5px"
  sm: "7px"
  md: "8px"
  lg: "10px"
  panel: "12px"
  window: "14px"
spacing:
  rail: "52px"
  sidebar: "276px"
  gutter: "8px"
  row: "13px"
  column: "780px"
components:
  button:
    backgroundColor: "rgba(255, 255, 255, 0.04)"
    textColor: "{colors.text}"
    rounded: "{rounded.md}"
    height: "30px"
    padding: "0 11px"
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.md}"
    height: "30px"
    padding: "0 11px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.muted}"
    rounded: "{rounded.md}"
    height: "30px"
  field:
    backgroundColor: "{colors.well}"
    textColor: "{colors.text}"
    rounded: "{rounded.sm}"
    height: "30px"
    padding: "0 10px"
  segmented:
    backgroundColor: "rgba(255, 255, 255, 0.035)"
    rounded: "9px"
    padding: "3px"
  work-panel:
    backgroundColor: "{colors.panel}"
    rounded: "{rounded.panel}"
  window:
    backgroundColor: "{colors.raise}"
    rounded: "{rounded.window}"
  engine-mark:
    textColor: "{colors.soft}"
    rounded: "4px"
    height: "16px"
    padding: "0 5px"
---

# Design System: Escouade

## Overview

**Creative North Star: "Qui attend passe devant"**

Escouade est un outil qu'on garde ouvert toute la journée, sur un écran de bureau, pendant que plusieurs agents travaillent. Le système en découle : un cadre presque noir (ou presque blanc) teinté par la couleur du projet ouvert, le travail posé sur un panneau en relief, et tout le reste en retrait jusqu'à ce qu'un agent ait besoin de toi. Alors l'ambre le fait passer en tête de liste, avec sa question et ses réponses à portée du clavier.

C'est la langue du site (`website/app/assets/main.css`), au niveau de finition de Linear et de Raycast : Inter, filets blancs à faible opacité, un seul bouton plein par écran, des icônes tracées, le teal du logo pour Escouade elle-même et la couleur du projet pour le reste. La densité est celle d'un outil de travail : 13 px de base, des métadonnées à 11,5–12 px, des rangées de 26 à 32 px. Le monde rejeté est celui de la 1.x : brun chaud, Hanken Grotesk, accent orange, un cadre autour de chaque appel d'outil, du mono pour toutes les métadonnées, des caractères Unicode en guise d'icônes.

Deux thèmes, mêmes jetons : le sombre est le monde d'origine, le clair (« Système » par défaut) relit les mêmes rôles à l'envers. Aucune couleur n'est écrite en dur dans un composant.

**Key Characteristics:**
- Toute la fenêtre prend la couleur du projet ouvert (`--p`), qui glisse en 0,7 s d'un projet à l'autre.
- L'agent qui attend est en tête, en ambre, avec ce qu'il demande ; on lui répond sans quitter ce qu'on lit.
- Le travail est posé sur un panneau en relief (rayon 12 px, marge de 8 px), le cadre autour reste sombre et calme.
- Un seul bouton plein par vue, en blanc (encre en clair) ; le teal reste au logo et aux vues globales.
- Le texte passe le WCAG AA dans les deux thèmes, pour chacune des 15 couleurs de projet.

## Colors

Des gris neutres teintés par le projet, des filets d'encre à faible opacité, trois couleurs d'état et rien d'autre de saturé.

### Primary
- **La couleur du projet** (`oklch(0.72 0.12 <teinte>)`, `--p`) : une des teintes de `PROJECT_COLORS`, choisie par projet. Elle teinte toutes les surfaces (`color-mix`), tombe en lumière depuis le haut à gauche (dégradé radial à 14 % en sombre, 10 % en clair, `--glow`), et donne le focus, la sélection (`--sel`), les interrupteurs allumés et le texte des liens (`--p-ink` : 70 % avec du blanc en sombre, 50 % avec l'encre en clair). Déclarée par `@property` pour glisser.
- **Le teal d'Escouade** (`--brand`) : le logo, et la couleur de projet des vues qui ne sont à aucun projet (Vue d'ensemble, Statistiques). En clair il fonce à `oklch(0.52 0.09 190)`.

### Secondary
- **En cours** (`--run`) : point d'un agent au travail, pastille « En cours », lancement qui tourne.
- **Attente** (`--wait`) : l'agent qui demande quelque chose, une tuile de projet qui attend, un ticket à tester. Fond à 7 % (9 % en clair) et filet à 32 % (40 %) de sa propre couleur.
- **Erreur, suppression** (`--del`), **ajout** (`--add`), **git modifié** (`--git-m`) : les diffs, les états de git, les plantages.

### Neutral
- **Cadre** (`--frame`) : barre de titre, barre des projets, barre latérale, barre d'état.
- **Panneau** (`--panel`) : la surface du travail (conversation, Kanban, Git, éditeur).
- **Reliefs** (`--raise`, `--raise-2`) : ce qui flotte ou se détache (fenêtres, menus, cartes du Kanban, zone de saisie).
- **Puits** (`--well`) : code, terminal, champs.
- **Encre des filets** (`--fg-rgb`, blanc en sombre, `15, 17, 22` en clair) : filets à 7,5, 12 et 20 % (`--line`, `--line-2`, `--line-3`), survol à 4,5 %.
- **Textes** : `--text` pour ce qu'on lit, `--soft` pour le second plan, `--muted` et `--dim` pour les métadonnées ; `--text-ai` pour la réponse d'un agent.

### Named Rules
**The Tinted Frame Rule.** Aucune surface n'a de couleur à elle : chacune est un gris neutre mêlé à `--p` à un dosage fixe (8 % le cadre, 4,5 % le panneau, 6 % les reliefs, 2 % les puits en sombre). Changer de projet change toute la fenêtre, rien d'autre.

**The Three States Rule.** Vert, ambre, rouge disent un état et rien d'autre. Une couleur saturée qui n'est ni le projet, ni le teal, ni un de ces états n'a pas sa place hors du code et des graphiques.

**The Charts Carry Names Rule.** Les palettes des graphiques sont validées pour le daltonisme (tokens : Entrée `#4c88d3`, Cache `#008b7a`, Sortie `#9555b0` ; modèles par rang : `#ca723c`, `#4682cc`, `#339c6d`, `#9555b0`, puis un gris). Les couleurs de projet ne passent pas ce contrôle : chaque barre d'un projet porte son nom.

### Les jetons
La liste complète des jetons de `src/app.css` (`:root`, et `:root[data-theme='light']` pour le clair) ; ce sont les seuls noms qu'un composant lit et les seuls qu'un thème de plugin pourra donner, clair et sombre. Les noms de la 1.x (`--bg`, `--elev`, `--term`, `--line2`, `--accent`, `--ok`, `--wait-soft`, `--ui`…) n'existent plus (ESC-172).

- **Projet** : `--p` (déclaré par `@property`), `--p-ink`, `--p-soft`, `--sel`, `--glow`, `--brand`.
- **Surfaces** : `--frame`, `--panel`, `--raise`, `--raise-2`, `--well`, `--scrim`, `--shade`, `--drop-base`, `--vo-base`, `--orb-base`.
- **Filets et survol** : `--fg-rgb`, `--line`, `--line-2`, `--line-3`, `--hover`, `--sep`, `--ring`.
- **Textes** : `--text`, `--soft`, `--muted`, `--dim`, `--text-ai`, `--idle`.
- **États** : `--run`, `--wait`, `--wait-bg`, `--wait-line`, `--done`, `--add`, `--del`, `--git-m`, `--warn-ink`, `--info`, `--teal-ink`, `--teal-ink-2` ; les discussions de l'onglet Docs : `--disc`, `--brain`.
- **Bouton plein et encres posées sur une couleur** : `--primary`, `--primary-hover`, `--on-primary`, `--on-primary-line`, `--on-primary-kbd`, `--on-primary-dim`, `--on-wait`, `--on-hit`, `--on-tick`, `--on-tile`, `--win-close`, `--on-win-close`.
- **Historique de git** : `--lane-main`, `--lane-1` à `--lane-8`.
- **Code et terminal** : `--code-fg`, `--code-ln`, `--code-nb`, `--code-dim`, `--syn-k`, `--syn-f`, `--syn-s`, `--syn-t`, `--syn-n`, `--ansi-k`, `--ansi-w`, `--ansi-w-2`, `--ansi-y`, `--ansi-cy`, `--ansi-cy-2`, `--term-sel-bg`, `--term-sel-fg`.
- **Hors thème** (la forme, pas la couleur ; un thème ne les donne pas) : `--font`, `--mono`, `--ease`, `--r`, `--r-sm`, `--rail`, `--side`, `--mac-corner`.

## Typography

**Body Font:** Inter Variable (`@fontsource-variable/inter`, avec `cv11` et `ss03`), repli `system-ui`, `Segoe UI`.
**Mono Font:** JetBrains Mono, repli `ui-monospace`, Consolas.

**Character:** Une seule famille, nette et serrée, qui tient la densité d'un outil ; le mono n'apparaît que pour ce qui se tape ou se copie.

### Hierarchy
- **Title** (600, 15 px, -0.01em) : le nom de l'agent dans son en-tête, le titre d'une fenêtre ; 17 à 26 px pour les seuls titres de page (Vue d'ensemble, Statistiques, un état vide).
- **Body** (400, 13 px, 1,45) : l'interface ; la conversation se lit à 13,5 px dans une colonne de 780 px.
- **Body dense** (400, 12,5 px, 1,5) : les descriptions des réglages, les champs, les listes.
- **Label** (550, 12 px) : les étiquettes, les titres de groupe, les boutons secondaires.
- **Meta** (500, 11,5 px, chiffres tabulaires) : durées, tokens, coûts, compteurs, touches (`kbd`), marque d'un harness (10,5 px).
- **Mono** (0,92 em de son contexte) : code, chemins, branches, commandes.

### Named Rules
**The Mono Is For Code Rule.** JetBrains Mono pour le code, les chemins, les branches et les commandes, jamais pour une métadonnée (un coût, une durée, un état s'écrivent en Inter à chiffres tabulaires).

**The Tabular Figures Rule.** Tout chiffre qui change sous les yeux (tokens, coût, durée, pourcentage) est en chiffres tabulaires, pour ne pas faire danser la ligne.

## Layout

Une grille fixe de bureau : barre des projets de 52 px à gauche (de la barre de titre à la barre d'état), barre de titre de 44 px, barre latérale de 276 px, barre d'état de 30 px, tout sur le cadre ; à droite, le travail sur son panneau, à 8 px du bord. La conversation se lit dans une colonne de 780 px centrée, la zone de saisie flotte à sa largeur. Le panneau Git s'ouvre à côté (340 px, 44 % du travail côte à côte).

La fenêtre fait 1440 × 900 par défaut, 1000 × 600 au minimum, et se vérifie à 1028 × 779 (celle de la CI), 1280 et 1440 px, en français et en anglais. Les replis suivent la largeur du conteneur (`@container`), jamais celle de la fenêtre : l'en-tête d'un agent perd ses chiffres puis ses mots par paliers (1100, 960, 880, 800, 680, 580, 440 px), l'éditeur ses détails (560, 460 px), les réglages passent leurs contrôles sous leur nom sous 520 px de corps, une carte du Kanban lâche les carrés de sa boucle sous 270 px. Rien ne déborde, rien ne passe sur deux lignes sans le vouloir : ce qui se coupe se coupe par une ellipse, son texte entier dans une infobulle.

À 820 px de haut et moins, Lancement et Terminaux se replient sur leur titre pour laisser la place aux agents.

## Elevation & Depth

Un système hybride : la profondeur vient d'abord des tons (cadre, panneau, reliefs, puits), puis d'ombres douces sous ce qui flotte seulement. Les ombres se multiplient par `--shade` (1 en sombre, 0,3 en clair), qui les adoucit dans le thème clair.

### Shadow Vocabulary
- **Panneau du travail** (`0 0 0 1px var(--line), 0 8px 24px -6px rgba(0,0,0,.5 × shade)`) : la seule surface posée sur le cadre.
- **Zone de saisie** (`inset 0 0 0 1px var(--line-2), 0 12px 32px -12px rgba(0,0,0,.6 × shade)`) : elle flotte au-dessus de la conversation.
- **Fenêtres** (`inset 0 0 0 1px var(--line-2), 0 24px 64px -12px rgba(0,0,0,.75 × shade)`) : sur un voile `--scrim`.
- **Cartes du Kanban** (`inset 0 0 0 1px var(--line), 0 1px 2px rgba(0,0,0,.3 × shade)`) : à peine levées.

### Named Rules
**The Hairline Before Shadow Rule.** Un contour est un filet intérieur (`inset 0 0 0 1px`), jamais une bordure de couleur ; une ombre n'apparaît que sous ce qui flotte (panneau, saisie, fenêtres, menus, notifications).

## Shapes

Des coins doux, d'autant plus ronds que la surface est grande : 4–5 px pour une marque ou une pastille, 7 px pour un champ ou un bouton-icône, 8 px pour un bouton, 9 px pour des segments, 10 px pour une carte, 12 px pour le panneau du travail, 14 px pour une fenêtre. Les points d'état sont des disques de 6 à 8 px ; l'agent qui attend a un anneau. Les icônes sont des tracés SVG de 1,5 px sur une grille de 16 (`src/lib/icons.ts`), une seule famille, en `currentColor`.

## Components

### Buttons
- **Shape:** coins de 8 px, 30 px de haut, 11 px de côté.
- **Default:** fond d'encre à 4 %, filet intérieur `--line-2`, texte `--text` à 550 ; au survol, fond à 8 % et filet `--line-3`.
- **Primary:** plein, `--primary` (blanc en sombre, encre en clair), texte `--on-primary`, un filet bas à peine marqué ; un seul par vue.
- **Ghost:** transparent, texte `--muted`, fond `--hover` au survol. **Danger:** texte `--del`, fond rouge à 10 % au survol.
- **Focus:** contour de 2 px en `--p-ink`.

### Chips
- **Segmented:** un rail à 3,5 % d'encre et filet `--line`, des segments de 28 px ; celui choisi est levé (`--raise-2`, filet) — pour les vues, les dispositions, le thème, les seuils.
- **Marque d'un harness** (`EngineMark`) : un mot dans un filet (« Claude », « Codex », « OpenCode », « Copilot »), jamais une couleur ; seulement quand des agents tournent sur deux harness.
- **Pastille d'état** : fond et filet tirés de la couleur de l'état (`oklch(from var(--run) l c h / 0.1)`), point devant.

### Cards / Containers
- **Carte d'agent** (barre latérale) : sans cadre au repos, en relief quand elle est ouverte, en ambre quand l'agent attend, avec sa question et ses réponses numérotées.
- **Carte du Kanban** : coins de 10 px, `--raise`, filet `--line`, 12 px de marge ; la clé du ticket ne se coupe jamais.
- **Groupes des réglages** : pas de boîte ; un titre, des rangées séparées par un filet à 5 %.

### Inputs / Fields
- **Style:** puits (`--well`), filet intérieur `--line-2`, coins de 7 px, 30 px de haut, 12,5 px.
- **Focus:** filet à 60 % de `--p` et halo de 3 px à 14 % de `--p`.
- **Error / Disabled:** filet `--del` (une expression régulière invalide de la recherche) ; opacité 0,5.

### Navigation
- **Barre des projets:** une tuile par projet (sa couleur, deux lettres), l'actif marqué d'un trait de 4 px au bord et d'un anneau, une pastille ambre pour celui qui attend ; Ctrl+1…9.
- **Barre latérale:** Agents | Kanban en segments, « Nouvel agent » (Ctrl N) et sa flèche, puis les agents, ceux qui attendent en tête.
- **Onglets soulignés** (Git, réglages par groupe) : texte `--muted`, l'actif en `--text` souligné de 2 px.

### L'agent qui attend (signature)
La carte de l'agent qui attend passe en tête de la liste, sur fond ambre à 5 % et filet `--wait-line`, son point en anneau. Elle porte ce qu'il demande et ses réponses, cliquables ou au clavier (1, 2…, Ctrl+J pour y aller) ; l'agent repart d'un éclair vert et reprend sa place. C'est le seul endroit où l'ambre occupe une surface.

### Mouvement
Courbe `cubic-bezier(0.16, 1, 0.3, 1)` (`--ease`). Trois moments écrits : le teint qui glisse d'un projet à l'autre (0,7 s), l'anneau de l'agent qui attend, le reflet de « Claude travaille… ». S'y ajoutent des signaux d'état (les points des sous-agents actifs qui respirent, « Attend ta réponse » dans la vue d'ensemble) et les montées des fenêtres et menus (0,2–0,35 s). Tout s'arrête avec `prefers-reduced-motion`.

## Do's and Don'ts

### Do:
- **Do** passer par un jeton de `src/app.css` pour chaque couleur, avec ses deux valeurs (sombre et clair) ; `src/lib/contrast.test.ts` vérifie l'AA pour chaque couleur de projet.
- **Do** dessiner les icônes avec `Icon` (tracés de 1,5 px sur 16), jamais un caractère Unicode.
- **Do** replier par `@container` sur la largeur de la colonne, et vérifier à 1028, 1280 et 1440 px en français et en anglais, panneau latéral ouvert ou non.
- **Do** couper par une ellipse ce qui ne tient pas, son texte entier dans l'infobulle `use:tip` (jamais un `title` natif).
- **Do** garder un seul bouton plein par vue, et le teal pour Escouade elle-même.

### Don't:
- **Don't** encadrer chaque appel d'outil ni chaque message : la conversation se lit sans boîtes, les appels en rangées.
- **Don't** écrire une métadonnée en mono, ni une couleur en dur dans un composant.
- **Don't** utiliser une bordure de couleur sur le côté d'une carte ou d'une alerte : l'état se dit par un fond et un filet de sa couleur.
- **Don't** ajouter un mouvement décoratif : les trois moments et les signaux d'état suffisent, et tous s'arrêtent avec `prefers-reduced-motion`.
- **Don't** revenir au monde de la 1.x (brun chaud, Hanken Grotesk, accent orange, cadres partout).
