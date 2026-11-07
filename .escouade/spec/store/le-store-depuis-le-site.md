---
title: "Le store depuis le site"
subject: "store"
sources: ["website/app/data/site.ts","website/app/components/SiteHeader.vue","website/app/components/SiteFooter.vue","website/app/components/RoadmapSection.vue","store/app/data/language.ts"]
tickets: ["ESC-372"]
decisions: [111]
compared: null
---

## Ce que c'est

Le site escouade.dev mène au store des plugins (store.escouade.dev) depuis chacune de ses pages, dans la langue de la page.

## Comportement

- **Adresse par langue** : le store sert le français à sa racine et l'anglais sous `/en`. Une page française du site mène à `https://store.escouade.dev/`, une page anglaise à `https://store.escouade.dev/en` (constante `STORE` de `website/app/data/site.ts`).
- **En-tête** : un lien « Plugins » (même mot dans les deux langues), juste après « Documentation » sur l'accueil, après la langue dans la documentation. Il suit la règle de Discord : il disparaît sous 900 px de large ; l'en-tête reste sur une ligne à 1280 et 1024 px.
- **Pied de page** : le même lien « Plugins », après « Documentation ». Sur un téléphone, les liens du pied passent à la ligne au lieu de déborder.
- **Accueil** : la carte « Les plugins et leur place de marché » de la feuille de route dit ce que font les plugins (sources de tickets, hébergeurs, harness, voix, notifications, thèmes et règles d'autorisation), qu'ils s'installent depuis l'app et sont listés sur le store, où chaque version passe onze contrôles, avec un lien « Voir le store » / « Browse the store ».
- **Documentation** : la page « Présentation » des plugins met le lien du store en tête, dans un encadré « Astuce » sous son premier paragraphe.

## Limites

- Ce que la carte de l'accueil énumère doit rester vrai du store : chaque famille citée y a au moins un plugin listé (Trello, Gitea, Claude Code, Deepgram, Discord, Contraste élevé, Règles de l'équipe au 2026-11-07).
