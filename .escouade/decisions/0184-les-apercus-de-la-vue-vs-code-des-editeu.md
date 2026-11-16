---
number: 184
id: "les-apercus-de-la-vue-vs-code-des-editeu"
title: "Les aperçus de la vue VS Code : des éditeurs d'Escouade dans la page, pdf.js et mammoth à la demande"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-473"
pages: ["editeur/editeur"]
tickets: ["ESC-473"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"editeur/editeur","added":32,"removed":7}]
accepted_at: "2026-11-16T08:20:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur a demandé le 2026-11-15 de pouvoir prévisualiser dans l'éditeur VS Code les PDF, images et Markdown, « et aussi si possible les docx ». L'étude de l'éditeur (docs/EDITOR-STUDY.md) n'en prévoyait pas. La vue tourne sans hôte d'extensions (pas de `unsafe-eval`, pas d'iframe) : un fichier binaire s'y ouvrait sur l'avertissement de VS Code. VS Code a ses propres aperçus (media-preview pour les images, l'audio et la vidéo ; markdown-language-features pour le Markdown), mais ce sont des extensions avec du code, qui dessinent dans des webviews : il leur faut l'hôte d'extensions et les webviews sur une origine à part, ce qu'est A6, pas encore fait. Pour le PDF et Word, VS Code n'a rien. Un fichier d'un dépôt ou d'un agent n'est pas de confiance : ni script, ni chargement distant sans clic.

## Décision

1. Les aperçus sont des éditeurs d'Escouade dans le plan de travail de VS Code : `registerEditorPane` et `registerEditor` de monaco-vscode-api, dessinés dans la page par Escouade, sans webview ni iframe. Ils sont l'éditeur par défaut de leurs fichiers : un clic dans l'explorateur, un lien d'une conversation ou Ctrl+P les ouvrent, et « Reopen Editor With… » ramène le texte. Les tableaux `.csv` et `.tsv` s'ouvrent en texte, leur tableau sur demande.
2. Familles faites : les images (PNG, JPEG, GIF, WebP, AVIF, BMP, ICO, et le SVG en `<img>`, jamais dans la page), l'audio et la vidéo (le lecteur de la fenêtre), le Markdown (marked, puis DOMPurify ; seul ou à côté de son source, avec les commandes de VS Code « Open Preview » et « Open Preview to the Side » et leurs touches ; suit la frappe et le défilement), le PDF par pdf.js 6.4.299 (Apache-2.0 ; pages, zoom, recherche, sélection), Word `.docx` par mammoth 1.13.0 (BSD-2-Clause ; titres, listes, tableaux, liens, images ; lecture seule).
3. Écartés pour l'instant, avec un aperçu qui le dit : `.odt` et `.odp` (mammoth ne les lit pas ; le cœur n'en tire que le texte), `.pptx` (aucun rendu léger), `.rtf`, `.doc`, `.xls`, `.ppt` (anciens formats binaires), `.xlsx` et `.ods` (SheetJS n'est plus à jour sur npm ; à faire plutôt dans le cœur, avec calamine, dans un ticket à part), TIFF (WebView2 ne le décode pas).
4. Sécurité : le HTML tiré d'un document est assaini par une instance de DOMPurify à part (profil HTML : ni script, ni événement, ni SVG ou MathML en ligne, ni style, ni formulaire, ni cadre ; identifiants préfixés). Aucune image ne se charge seule : une image du web se télécharge par le cœur au clic sur « Afficher l'image », comme dans la conversation ; une image de la source est lue par le cœur. Les liens passent par l'ouverture de liens d'Escouade, ceux de la source s'ouvrent dans la vue. pdf.js tourne sans scripts (son bac à sable n'est pas livré) ni XFA ; ses wasm (JBIG2, JPEG 2000, couleurs) sont livrés avec l'app, ses polices standard et ses CMaps ne le sont pas. Un `.docx` qui se décompresserait en plus de 256 Mo n'est pas ouvert.
5. Politique de contenu : une seule directive ajoutée, `media-src 'self' blob:`, pour l'audio et la vidéo. Rien d'autre ne change : `img-src` avait déjà `blob:` et `data:`, `script-src` `'wasm-unsafe-eval'` (le wasm de pdf.js), `worker-src 'self' blob:` (son worker), `connect-src 'self'` (ses fichiers wasm).
6. Rien n'est laissé à A6 dans ce ticket. Une fois A6 fait, les aperçus intégrés de VS Code pourront tourner : ce ticket ne s'y oppose pas, mais ceux d'Escouade restent, parce qu'ils couvrent le PDF et Word.

## Conséquences

Poids, mesuré sur la build : environ 6 Ko compressés pour la déclaration des aperçus, dans le morceau de la vue (déjà chargé à sa première ouverture) ; pdf.js environ 606 Ko en brotli (2,5 Mo bruts : bibliothèque, worker, visionneuse, CSS, trois wasm), chargé au premier PDF ; mammoth environ 75 Ko en brotli (300 Ko bruts), chargé au premier Word. Mémoire, mesurée par l'e2e (`ESCOUADE_E2E_MEASURE=1`) : une image ne coûte presque rien, le Markdown +6 Mo de tas JS (le coloriage du code), un PDF de 3 pages +38 Mo pour les processus de WebView2 (worker et pages dessinées), un Word +1,4 Mo de tas ; une fois tout fermé, il reste environ 10 Mo de tas (le code chargé). Les aperçus sont dessinés par nous : leur rendu est moins complet que celui de VS Code (ni Mermaid ni formules dans le Markdown) et que celui de Word (ni mise en page fine, ni commentaires). Les messages de VS Code restent en anglais jusqu'à A4, dont les noms des commandes d'aperçu. Les tickets à venir : `.xlsx` et `.ods` dans le cœur, `.odt` par un rendu à nous, peut-être les aperçus de VS Code après A6.
