---
version: 1
slug: "app-components-herosection-vue"
primary_target: "app/components/HeroSection.vue"
related_targets: []
---

# Hero du site de présentation

## Scope

`website/app/components/HeroSection.vue` (et ses composants) : la première section de la page unique du site, plein écran et animée. Mode : Persuade. Le reste de la page (vidéo, fonctionnalités, installation, FAQ) ne change pas.

## Audience, job, action

- Des développeurs qui font déjà travailler Claude Code et voudraient en faire tourner plusieurs à la fois.
- Ils doivent croire qu'Escouade leur permet de lancer plusieurs agents en parallèle sans les surveiller : tout est dans une fenêtre, et on est prévenu quand un agent attend.
- Action : « Télécharger pour Windows et macOS » (dernière release) ; secondaire : « Voir sur GitHub ».
- Preuve : l'app elle-même, au travail (données de démonstration fictives, dites comme telles). Aucun chiffre d'usage, témoignage ni benchmark.

## Constraints

- Garder : lien de téléchargement, lien GitHub, « Version x.y.z », plateformes, gratuit et open source (MIT) ; tests de `website/tests/site.test.ts`.
- Pas de nouvelle dépendance ; Nuxt 4 statique sous `/escouade/` ; polices servies par le site.
- `prefers-reduced-motion` : pas de déploiement, simulation à l'arrêt sur un état parlant. Animation qui se met à jour seule : bouton pause (WCAG 2.2.2), arrêt hors écran et onglet caché.
- Sans JavaScript : l'état final, complet, visible.

## Direction contract

THESIS: Le logo devient l'app. Les trois fenêtres empilées du logo, à l'échelle d'une vraie fenêtre, sont trois projets ouverts ; celle de devant montre cinq agents qui travaillent pendant qu'on lit. Refuse le hero de catégorie : capture d'écran inclinée sous un titre centré, ou mockup générique entouré de badges.

OWN-WORLD: Le monde du site, inchangé. Fond chaud #1b1917, panneaux #211f1c / #2a2724, filets crème translucides. Teal #35c1bd du logo pour les contours des fenêtres (opacités 0,35 / 0,6 / 1, comme le logo) et pour l'action principale. Couleurs de statut de l'app : vert (en cours, terminé), ambre (question), jaune (à tester). Hanken Grotesk 800 pour le titre ; JetBrains Mono seulement pour les données (modèle, durée, tokens, coût, chemins, clés de ticket).

STORY: En quelques secondes, le visiteur voit plusieurs Claude Code avancer en parallèle, l'un poser une question (ambre, avec sa pastille), un agent de ticket boucler jusqu'à « À tester », le coût du jour monter. Il comprend « une escouade, une fenêtre, je suis prévenu ». Il peut répondre lui-même à la question, puis il télécharge.

FIRST VIEWPORT: Hauteur : le viewport moins l'en-tête. À gauche (environ 45 %) : le titre « Une escouade de Claude. / Une seule fenêtre. » sur deux lignes, jusqu'à 92 px, la seconde en teal ; puis le paragraphe « Escouade, le poste de pilotage… », les boutons Télécharger (teal plein) et GitHub, et la ligne de version en mono. À droite (environ 55 %) : l'objet-logo carré, aussi grand que la hauteur le permet. La fenêtre de devant, en bas à gauche, porte une barre de titre (l'étincelle du logo et demo-api), cinq cartes d'agents et une barre de statut. Les fenêtres arrière portent les onglets studio-web et mobile-app, avec leurs agents en points. Sur mobile : le texte et les boutons dans le premier écran, l'objet dessous, avec trois agents.

FORM: Extension d'une surface établie (new-work §3, « Extend an existing surface ») : pas de tirage concept-seed, donc pas de clé de tirage. Signature : le logo qui se déploie en fenêtres, une seule fois, en 1,5 s environ, avec une décélération exponentielle ; puis la simulation des agents en boucle, avec une question à laquelle on peut répondre ; pause accessible.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance

## Memorable moment

Le logo de l'en-tête, en grand, qui s'ouvre : ses trois fenêtres deviennent des projets, l'étincelle file dans le coin de la fenêtre de devant, et un agent lève la main en ambre.

## Unresolved

Aucune.
