---
name: Escouade, site de présentation
description: Le site d'une app qui pilote une escouade d'agents Claude Code dans une seule fenêtre.
colors:
  brand: "#35c1bd"
  bg: "#1b1917"
  panel: "#211f1c"
  elev: "#2a2724"
  line: "rgba(255, 236, 214, 0.08)"
  line2: "rgba(255, 236, 214, 0.15)"
  text: "#ede7df"
  muted: "#a8a095"
  dim: "#8a8276"
  ink: "#1b1512"
  ok: "oklch(0.76 0.12 150)"
  wait: "oklch(0.82 0.13 80)"
  wait-soft: "oklch(0.82 0.13 80 / 0.08)"
  wait-ring: "oklch(0.82 0.13 80 / 0.6)"
  add: "oklch(0.76 0.12 150)"
  del: "oklch(0.72 0.14 25)"
  accent: "oklch(0.72 0.12 48)"
typography:
  display:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "clamp(44px, 5.1vw, 84px)"
    fontWeight: 800
    lineHeight: 0.98
    letterSpacing: "-0.035em"
  headline:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "clamp(30px, 4vw, 44px)"
    fontWeight: 700
    lineHeight: 1.15
    letterSpacing: "-0.02em"
  title:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "28px"
    fontWeight: 700
    lineHeight: 1.2
    letterSpacing: "-0.01em"
  title-sm:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "19px"
    fontWeight: 700
    lineHeight: 1.6
  lead:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "19px"
    fontWeight: 400
    lineHeight: 1.6
  body:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "18px"
    fontWeight: 400
    lineHeight: 1.6
  body-sm:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.6
  label:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 600
  button:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 700
  mono:
    fontFamily: "JetBrains Mono, ui-monospace, monospace"
    fontSize: "13px"
    fontWeight: 400
  caption:
    fontFamily: "Hanken Grotesk, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 400
rounded:
  sm: "8px"
  md: "10px"
  lg: "12px"
  xl: "16px"
spacing:
  xs: "8px"
  sm: "12px"
  md: "16px"
  lg: "20px"
  xl: "24px"
  2xl: "40px"
  section: "96px"
components:
  button-primary:
    backgroundColor: "{colors.brand}"
    textColor: "{colors.ink}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: "0 22px"
    height: "48px"
  button-secondary:
    backgroundColor: "{colors.elev}"
    textColor: "{colors.text}"
    typography: "{typography.button}"
    rounded: "{rounded.md}"
    padding: "0 22px"
    height: "48px"
  button-header:
    textColor: "{colors.muted}"
    typography: "{typography.label}"
    rounded: "{rounded.sm}"
    padding: "7px 14px"
  button-header-hover:
    textColor: "{colors.text}"
  nav-link:
    textColor: "{colors.muted}"
    typography: "{typography.label}"
  nav-link-hover:
    textColor: "{colors.text}"
  card:
    backgroundColor: "{colors.panel}"
    rounded: "{rounded.lg}"
    padding: "24px"
  install-step:
    backgroundColor: "{colors.bg}"
    rounded: "{rounded.lg}"
    padding: "26px"
  step-number:
    backgroundColor: "{colors.brand}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    size: "36px"
  squad-window-mid:
    backgroundColor: "{colors.panel}"
  squad-window-front:
    backgroundColor: "#1d1a17"
  agent-card-question:
    backgroundColor: "{colors.wait-soft}"
    rounded: "0.7em"
    padding: "0.62em 0.85em 0.66em"
  question-option:
    textColor: "{colors.text}"
    rounded: "0.5em"
    padding: "0.28em 0.8em"
  question-option-hover:
    backgroundColor: "{colors.wait}"
    textColor: "{colors.ink}"
  badge-wait:
    backgroundColor: "{colors.wait}"
    textColor: "{colors.ink}"
    rounded: "1em"
---

# Design System: Escouade, site de présentation

## Overview

**Creative North Star: "Le poste de pilotage de nuit"**

Le site est une pièce sombre et chaude : un brun nuit, des panneaux à peine plus clairs, des filets crème translucides. Une seule couleur, le teal du logo, désigne ce qui appartient à Escouade et ce sur quoi on agit. Les autres couleurs sont des voyants : le vert, l'ambre, l'orange et le rouge de l'app n'apparaissent que là où l'app elle-même est montrée, et y gardent le sens qu'ils ont dans l'app.

La preuve, c'est la vraie chose. Le hero déplie le logo, trois fenêtres arrondies empilées, à l'échelle d'une vraie fenêtre : les deux de derrière sont des projets, celle de devant montre cinq agents qui travaillent, dont l'un pose une question à laquelle on peut répondre et l'autre boucle sur un ticket. Plus bas, les captures et la vidéo de l'app tiennent le même rôle. Le décor reste plat ; seuls ces objets-là portent une ombre.

Densité moyenne, une colonne de 1160px, des sections de 96px. Hanken Grotesk porte la voix, en 800 et resserrée pour le titre ; JetBrains Mono ne sert qu'aux données. Le mouvement a une seule grammaire : décélération exponentielle à l'entrée, 0,15s au survol, 0,3s aux changements de statut, et rien ne bouge sous `prefers-reduced-motion`. Refusés par la direction : la capture inclinée sous un titre centré, le mockup générique entouré de badges.

**Key Characteristics:**
- Fond brun chaud ; toutes les neutres tirent vers la même teinte chaude, jamais vers le gris froid.
- Un seul teal, celui du logo, pour l'identité, l'action et le focus.
- Les couleurs de statut de l'app réservées aux représentations de l'app.
- L'app montrée de face, au travail, avec des données dites fictives.
- La profondeur par les tons ; les ombres réservées aux fenêtres, captures et vidéo.
- La mono pour les données, la grotesque pour tout le reste.

## Colors

Une palette nocturne et chaude, un teal qui signale, et les voyants de l'app.

### Primary
- **Teal Escouade** (brand) : la couleur du logo. Bouton principal (aplat, texte encre), seconde ligne du titre du hero, contours des fenêtres de l'objet-logo, anneau de focus, liens d'action des étapes d'installation, puces carrées des listes, numéros d'étape, « + / − » de la FAQ, bordure des boutons secondaires au survol, sélection de texte (à 38 %). Ses seules nappes sont les deux halos radiaux du hero (11 % et 7 %).

### Secondary
Les couleurs de statut de l'app (`src/app.css`), reprises pour ses agents.
- **Vert de marche** (ok) : agent en cours ; agent terminé (point à 45 %) ; critère atteint ; remplissage de la barre de progression d'un ticket.
- **Ambre d'attente** (wait) : agent qui attend une réponse, pastille qui compte les agents en attente, boutons de réponse au survol ; aussi l'état « À tester » d'un ticket. Le contrat de direction annonçait un jaune distinct pour « À tester » ; le build réutilise l'ambre.
- **Voile ambre** (wait-soft) et **anneau ambre** (wait-ring) : fond et contour de la carte d'un agent qui pose une question, contour de ses boutons de réponse, anneau qui pulse autour de son point.
- **Vert et rouge de diff** (add, del) : lignes ajoutées et retirées d'une modification de fichier. add a la valeur de ok.

### Tertiary
- **Orange chaud de l'app** (accent) : l'accent propre de l'app, gardé dans la démo seulement, pour les pictogrammes d'outil (Read, Grep, Edit, Write, Bash) et la pastille du projet courant. Ce n'est pas une couleur du site.

### Neutral
text, muted et dim passent tous trois 4,5:1 sur bg (vérifié par `tests/tokens.test.ts`).
- **Brun nuit** (bg) : fond de page, fond des étapes posées sur la bande d'installation, base de l'en-tête translucide (à 82 %).
- **Panneau** (panel) : cartes de fonctionnalités, bande « Installer », fenêtre du milieu de l'objet-logo.
- **Relief** (elev) : fond du bouton secondaire.
- **Filet crème** (line, 8 %) : séparateurs d'en-tête, de pied et de bande, bordure des cartes, lignes internes de la fenêtre de démo, fond des étiquettes « Δ ».
- **Filet crème appuyé** (line2, 15 %) : bordure des boutons, des étapes, des captures et de la vidéo, séparateurs de FAQ, rail de progression, bouton pause.
- **Crème** (text) : texte principal et titres.
- **Sable** (muted) : paragraphes d'accompagnement, textes de cartes, liens de navigation au repos, activité d'un agent.
- **Taupe** (dim) : métadonnées (ligne de version, légende, mentions du pied, modèle et durée d'un agent, branche, en-tête de liste de la démo), agent prêt.
- **Encre** (ink) : texte posé sur le teal ou l'ambre (bouton principal, numéros d'étape, pastilles, option survolée, coche d'un critère).

### Named Rules
**La règle du teal unique.** Le teal est Escouade : identité, action, focus. Aucune autre teinte ne tient ces rôles. En aplat, il ne couvre que le bouton principal et de petits marqueurs (numéros, puces) ; ses seules nappes sont les deux halos du hero.

**La règle des voyants.** ok, wait, accent, add et del ne vivent que dans une représentation de l'app, et y gardent le sens qu'ils ont dans l'app : vert en cours ou terminé, ambre en attente ou à tester. Jamais sur un bouton, un lien ou un titre du site.

**La règle du filet crème.** Bordures et séparateurs sont du crème translucide, à 8 % ou à 15 %, jamais un gris opaque : ils prennent la chaleur de la surface qu'ils traversent.

## Typography

**Display Font:** Hanken Grotesk (avec system-ui, sans-serif)
**Body Font:** Hanken Grotesk (avec system-ui, sans-serif)
**Label/Mono Font:** JetBrains Mono (avec ui-monospace, monospace)

**Character:** Une grotesque chaleureuse et dense, qui monte à 800 et se resserre pour le titre, et une mono technique qui ne dit que chiffres, chemins et clés. Le site sert ses polices lui-même : Hanken Grotesk de 400 à 800, JetBrains Mono en 400 et 600.

### Hierarchy
- **Display** (800, clamp(44px, 5.1vw, 84px), 0,98, -0,035em) : le titre du hero seul, en deux phrases sur des lignes distinctes, la seconde en teal et tenue sur une ligne au-dessus de 960px ; équilibré (`text-wrap: balance`). Le contrat visait 92px ; le build plafonne à 84px.
- **Headline** (700, clamp(30px, 4vw, 44px), 1,15, -0,02em) : titres de section.
- **Title** (700, 28px, 1,2, -0,01em) : titre d'une rangée de fonctionnalité.
- **Title small** (700, 19px) : titres de carte, questions de FAQ ; 20px pour les titres d'étape.
- **Lead** (400, 19px, 1,6) : paragraphe sous un titre de section, en sable, 680px au plus. Dans le hero, clamp(17px, 1.35vw, 19px) sur 34em au plus (`text-wrap: pretty`).
- **Body** (400, 18px, 1,6) : texte courant.
- **Body small** (400, 16px) : textes de carte et d'étape, listes à puces.
- **Label** (600, 15px) : navigation, lien GitHub de l'en-tête, liens d'étape.
- **Button** (700, 16px) : libellés des boutons.
- **Mono** (400, 13px) : ligne de version du hero. Dans la fenêtre de démo, la mono descend à 0,86em du texte de la fenêtre, jamais sous 10px.
- **Caption** (400, 13px) : légende de la démo ; 14px pour la mention du pied.

### Named Rules
**La règle de la donnée en mono.** JetBrains Mono est réservée aux données et aux compteurs : version, modèle, durée, tokens, coût, chemins, branche, clé et boucle de ticket, numéros d'étape, « Δ », « + / − » de la FAQ. Avec `tabular-nums` dès qu'un nombre change. Titres, phrases et boutons restent en Hanken Grotesk.

**La règle du titre serré.** Plus un titre est grand, plus il se resserre : -0,035em à 84px, -0,02em à 44px, -0,01em à 28px. Le texte courant garde l'interlettrage normal.

Le surtitre mono en capitales posé au-dessus des titres de section existe dans le code mais n'est pas une règle du système : il ne se reprend pas sur une nouvelle surface.

## Layout

La page est une colonne centrée de min(1160px, 100% − 32px), soit 16px de marge au minimum ; la FAQ se resserre à 820px. Les sections ont 96px en haut et en bas ; 12px séparent un titre de section de son paragraphe, 40px ce paragraphe du contenu. L'en-tête colle en haut sur 64px, et les ancres gardent 80px de marge.

Le hero prend la hauteur du viewport moins l'en-tête (100svh − 64px). Sa scène est plus large que la colonne (1400px au plus, marges de clamp(32px, 6vw, 96px)) : deux colonnes égales, le texte à gauche sur 680px au plus, l'objet-logo carré à droite, aussi grand que la hauteur le permet et 760px au plus. Ses espacements verticaux suivent la hauteur (clamp en svh). Sous 960px, il passe en une colonne : le texte et les boutons d'abord, l'objet dessous (600px au plus), sans le lien « Voir la vidéo ».

Les fonctionnalités alternent texte et capture en 5fr / 7fr (gouttière 52px), puis trois cartes (gouttière 16px) ; les étapes d'installation tiennent sur trois colonnes (gouttière 20px). Sous 900px, tout passe en une colonne. Sous 720px, la navigation disparaît et le lien GitHub reste.

L'objet-logo s'adapte à son conteneur. Sous 570px de large, là où son texte cesse de rétrécir, la fenêtre de devant garde trois agents (celui qui travaille, celui qui demande, le ticket) et prend leur hauteur au lieu du carré du logo (elle part quand même de ce carré au déploiement) ; le nombre d'agents de l'en-tête, ses compteurs, la branche et les coûts par carte s'effacent, la pastille de l'onglet et les cartes disant déjà qui travaille et qui attend.

Le rythme d'espacement observé : 8, 12, 16, 20, 24, 40 et 96px.

## Elevation & Depth

Le site est plat. La profondeur vient de trois tons (brun nuit, panneau, relief) et des filets crème ; cartes, boutons, bandes et en-tête n'ont pas d'ombre. L'en-tête collant est le brun nuit à 82 % sur un flou d'arrière-plan de 10px. Les seules ombres portées, larges et diffuses, appartiennent aux objets qui représentent l'app. Derrière l'objet-logo, deux halos radiaux teal (11 % et 7 %) font la lumière ambiante du hero.

### Shadow Vocabulary
- **Capture** (`box-shadow: 0 24px 70px rgba(0, 0, 0, 0.5)`) : captures d'écran des rangées de fonctionnalités.
- **Vidéo** (`box-shadow: 0 40px 120px rgba(0, 0, 0, 0.55)`) : le lecteur vidéo, le plus grand objet de la page.
- **Fenêtre** (`box-shadow: inset 0 0 0 1.5px <contour>, 0 2.4cqw 7cqw -2cqw rgb(0 0 0 / 0.6)`) : chaque fenêtre de l'objet-logo. Le contour est un trait intérieur teal à 35, 60 ou 100 % ; l'ombre suit la taille du conteneur.

### Named Rules
**La règle de l'objet réel.** Seul ce qui montre l'app (fenêtre, capture, vidéo) porte une ombre. Le décor du site reste plat et se superpose par tons.

## Shapes

Des coins doucement arrondis, de plus en plus ronds avec la taille de l'objet : 8px pour les petits contrôles (GitHub dans l'en-tête, lien « Voir la vidéo »), 10px pour les boutons et les numéros d'étape, 12px pour les cartes, les étapes et les captures, 16px pour la vidéo. Les bordures font 1px, en filet crème.

La forme signature est le logo : trois carrés arrondis empilés en diagonale, celui de devant marqué d'une étincelle à huit rayons. L'objet-logo en garde la géométrie exacte (fenêtres de 66 décalées de 9 sur un carré de 84, rayon de 3,4 % de la largeur, trait intérieur de 1,5px) et repart de la forme du logo au début de son animation (rayon de 16,6 %, trait épais). De petits carrés aux coins à peine arrondis en font l'écho : puces de liste (8px, rayon 2px, teal), pastilles de projet (0,62em, rayon 0,18em).

Dans la fenêtre de démo, les rayons sont en em pour suivre son texte : carte d'agent 0,7em, bouton de réponse 0,5em, étiquette « Δ » 0,35em, pastille d'attente et rail de progression en pilule, bouton pause rond.

## Components

### Buttons
Compacts, francs, en gras.
- **Shape:** coins doucement arrondis (10px), 48px de haut, 22px de padding horizontal, bordure de 1px.
- **Primary:** aplat teal, texte encre, bordure teal. Un seul par vue : « Télécharger pour Windows et macOS ».
- **Secondary:** fond relief, texte crème, bordure en filet appuyé : « Voir sur GitHub ».
- **Hover / Focus:** le secondaire prend une bordure teal, le principal monte de 1px, en 0,15s. Le focus, partout sur le site, est un contour teal de 2px décalé de 3px.
- **Fantôme d'en-tête:** « GitHub » en sable 15px / 600, bordure en filet appuyé, rayon 8px, padding 7px 14px ; crème au survol.

### Cards / Containers
- **Cartes de fonctionnalités :** fond panneau, bordure en filet, rayon 12px, padding 24px, sans ombre ; titre 19px, texte sable 16px.
- **Étapes d'installation :** sur la bande panneau, fond brun nuit, bordure en filet appuyé, rayon 12px, padding 26px. Le numéro tient dans un carré teal de 36px (rayon 10px), chiffre mono 600 en encre ; le lien d'action, teal 600 suivi de « → », se souligne au survol.
- **Bande :** la section « Installer » passe sur fond panneau, entre deux filets.

### Navigation
- **En-tête :** collant, 64px, brun nuit à 82 % et flou de 10px, filet en bas. La marque à gauche (logo 28px et « Escouade » 20px / 800), les liens à droite en sable 15px / 600 espacés de 24px, crème au survol. Sous 720px, seuls la marque et GitHub restent.
- **Pied :** filet en haut, logo 22px et nom en 700, liens sable crème au survol, mention d'indépendance en taupe 14px.

### FAQ
- Questions dépliables en 19px / 700, séparées par des filets appuyés, 20px de padding vertical, un « + » ou un « − » mono teal à droite. Réponses en sable.

### Rangée de fonctionnalité
- Texte (titre 28px, paragraphe sable, puces carrées teal) face à une capture de l'app (rayon 12px, bordure en filet appuyé, ombre « Capture »), côtés alternés d'une rangée à l'autre.

### L'objet-logo (signature)
Le logo d'Escouade à l'échelle d'une vraie fenêtre : trois projets ouverts, celui de devant au travail.
- **Géométrie :** un carré (sous 570px de large, la hauteur de la fenêtre de devant) où les trois fenêtres reprennent celles du logo, chacune avec son contour intérieur teal : derrière à 35 %, au milieu à 60 %, devant à 100 %. Fonds : derrière un mélange de panneau (55 %) et de brun nuit, au milieu le panneau, devant #1d1a17.
- **Fenêtres de derrière :** leur onglet (pastille de projet, nom en gras, « Δ n » en mono sur filet, pastille ambre si un agent attend) dans la bande que laisse la fenêtre suivante au-dessus ; leurs agents en points de statut dans la bande de droite (vert en cours, ambre en attente, taupe prêt, vert à 45 % terminé).
- **Fenêtre de devant :** une barre de titre (étincelle teal dans le coin, onglet du projet avec sa pastille ambre ou son point vert, branche en mono taupe, contrôles de fenêtre au trait), l'en-tête de liste en capitales espacées comme dans l'app, les cartes d'agents, une barre de statut en mono (compteurs à points, coût du jour en crème, bouton pause rond de 2,2em, bordure teal au survol). Son texte suit le conteneur : clamp(11.5px, 2.05cqw, 14.5px).
- **Déploiement :** une fois, au chargement. Le logo tient 0,45s à sa taille, puis en 1,3s sur cubic-bezier(0.16, 1, 0.3, 1) la pile grandit (de 0,15 à 1), ses coins s'ouvrent, ses traits s'affinent et l'étincelle file dans son coin ; le contenu apparaît à 1,2s (0,5s), et les cartes montent l'une après l'autre (0,6s, 70ms d'écart). Sous 570px de large, la fenêtre de devant part carrée et rejoint la hauteur de ses agents pendant ces 1,3s, pour que le logo tenu reste le logo. Le contrat disait « environ 1,5 s ».
- **Simulation :** un battement de 1,2s ; l'ambre clignote sur 1,4s. Elle s'arrête hors écran, onglet caché, en pause et tant que le focus est sur une réponse à la question ; une réponse donnée au clavier rend le focus à la carte. Sous `prefers-reduced-motion`, ni déploiement ni simulation : un état parlant, fixe ; sans JavaScript, le même état, complet.
- **Légende :** « Démonstration animée · données fictives », en taupe 13px.

### Carte d'agent
- **Rangée du haut :** point de statut, nom en gras (tronqué), « modèle · durée » en mono taupe, état à droite dans la couleur du statut (600).
- **Ticket :** caret teal, clé et « boucle n/max » en mono, critères en cercles qui se remplissent de vert en traçant leur coche (0,45s), rail de progression rempli de vert (0,6s).
- **Activité :** pictogramme d'outil au trait orange chaud, nom de l'outil en crème, cible en mono tronquée, diff en vert et rouge ; ou bien un résultat, coché dans la couleur du statut. Le coût « tok · $ » en mono à droite. Une activité remplace l'autre en fondu montant de 0,18s.
- **Question :** voile ambre, anneau ambre, point qui pulse ; la question en crème et ses boutons de réponse (contour ambre, rayon 0,5em ; au survol aplat ambre et texte encre ; au focus contour ambre de 2px).
- **Changement de statut :** les couleurs passent en 0,3s.

## Do's and Don'ts

### Do:
- **Do** réserver le teal à l'identité, à l'action principale, aux liens d'action et au focus (contour de 2px décalé de 3px).
- **Do** écrire le texte sur le fond en text, muted ou dim : tous trois passent 4,5:1 sur bg, et `tests/tokens.test.ts` le vérifie.
- **Do** composer les données (version, modèle, durée, tokens, coût, chemins, clés) en JetBrains Mono, avec `tabular-nums` quand elles changent.
- **Do** montrer l'app de face et au travail, avec ses propres couleurs de statut, en disant ses données fictives.
- **Do** tirer tout nouvel objet-fenêtre de la géométrie du logo : fenêtres de 66 décalées de 9 sur 84, contours teal à 35, 60 et 100 %.
- **Do** faire entrer le mouvement sur cubic-bezier(0.16, 1, 0.3, 1) ; 0,15s pour un survol, 0,3s pour un changement de statut.
- **Do** donner à chaque animation un état fixe sous `prefers-reduced-motion` et sans JavaScript, et une pause à tout ce qui boucle (WCAG 2.2.2), arrêtée hors écran.

### Don't:
- **Don't** utiliser ok, wait, accent, add ou del hors d'une représentation de l'app : ni sur un bouton, ni sur un lien, ni sur un titre du site.
- **Don't** mettre d'ombre sur une carte, un bouton, une bande ou l'en-tête : l'ombre est réservée aux fenêtres, aux captures et à la vidéo.
- **Don't** tracer une bordure en gris opaque ou en blanc : les filets sont en crème translucide (line, line2).
- **Don't** utiliser de blanc pur ni de gris froid : texte et surfaces du site sont des neutres chauds (le noir ne sert qu'au fond du lecteur vidéo).
- **Don't** incliner une capture sous un titre centré, ni entourer un mockup générique de badges.
- **Don't** poser un second bouton teal à côté du principal : l'action secondaire est toujours le bouton relief.
