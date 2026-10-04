# Vidéo de présentation v2 et site — design

## But

Depuis la première vidéo (28/09, 1 min 22, sans voix), Escouade a gagné le tableau de tickets, le lancement de test, l'éditeur intégré, la synchro git, la reprise après limite d'usage, macOS… La vidéo est refaite pour montrer **toutes** les fonctionnalités, **sans limite de durée** : chaque fonctionnalité prend le temps qu'il faut pour être montrée et expliquée. Le site reprend la nouvelle vidéo, ses images et les nouvelles fonctionnalités.

Ce qui a été choisi :

- **ton d'un lancement de produit, pas d'une formation** (retour du 04/10 sur la première version, « ça fait vidéo de formation ») : répliques courtes, une idée chacune, l'image explique le geste ; rythme serré, musique entraînante, plans qui s'enchaînent ;
- **voix off** française (tutoiement, comme le site), générée par ElevenLabs : voix « Corentin » (jeune, enthousiaste), modèle `eleven_v4`, choisie parmi sept voix françaises « énergiques » comme la plus vive (3,6 mots/s, intonation la plus variée : ±6 demi-tons) ;
- **prononciation** : une table (`SAY` dans `src/script.ts`) réécrit pour la voix les mots qu'elle dirait mal (« git » → « guite », « chat » → « tchatte », « .env » → « point E N V », « gh », « claude.ai », « macOS ») ; une prise où « Claude » sort à l'anglaise est refaite avec une autre graine (`RETAKES`) ;
- **musique** générée par ElevenLabs (`music_v1`, instrumentale, ~124 BPM), composée en sections calées sur les plans, baissée sous la voix ;
- interface toujours **recréée** dans Remotion, avec les mêmes données fictives (`demo-api`, `refacto-auth`…) ;
- textes à l'écran : un titre par plan, qui jaillit mot à mot avec un trait d'accent ; la voix off est aussi sous-titrée sur le site (piste WebVTT, activable dans le lecteur), pas incrustée dans l'image.

## La voix commande le temps

- `src/script.ts` : le script. Un plan = un identifiant, un titre à l'écran, les fonctionnalités qu'il montre, et ses répliques. Une réplique = un identifiant, son texte (sous-titre), et au besoin sa forme prononcée (`say`, quand la table de prononciation ne suffit pas ; aucune réplique n'en a besoin aujourd'hui) et un temps de pause après (`hold`) pour laisser une animation se finir.
- `scripts/voice.ts` (`npm run voice`, clé dans `ELEVENLABS_API_KEY`, jamais écrite dans un fichier) : une requête « text-to-speech with timestamps » par réplique (avec le texte voisin pour une intonation continue). Écrit `public/voice/<id>.mp3` et `src/voice.json` : texte, voix, modèle, durée et horaires de chaque mot. Une réplique dont le texte n'a pas changé n'est pas régénérée. `--check` retranscrit chaque fichier (speech-to-text) et signale une réplique trop éloignée de son texte.
- `src/timeline.ts` : pur. Chaque plan = une entrée (`lead`, 0,35 s), ses répliques bout à bout séparées d'un souffle (0,25 s), les pauses `hold`, puis une sortie (`tail`, 0,5 s). Donne pour chaque plan son début, sa durée et l'image de départ de chaque réplique (« cues »). La durée totale en découle : aucune limite.
- Entre deux plans (`sceneMotion`, `src/anim.ts`) : le plan sortant file à gauche, flouté, le suivant arrive de la droite, sur un « whoosh » ; pendant le plan, la caméra avance lentement (3 % au plus), pour qu'aucune image ne reste figée.
- Les plans animent leurs éléments **par rapport aux cues** de leurs répliques (`useCues()`), pas à des images fixes : si une réplique change de longueur, l'animation suit.

## Musique

- `scripts/music.ts` (`npm run music`) : un plan de composition ElevenLabs (sections de 3 à 120 s) construit depuis la timeline : le beat dès la première mesure, aucune partie calme, un sommet sur le tableau, une fin franche sur le logo. Écrit `public/music.mp3` et `src/music.json` (durée et empreinte de la timeline pour laquelle elle a été faite).
- `src/mix.ts` : pur. Volume de la musique image par image : fondu d'entrée, baisse sous chaque réplique (rampes douces), fondu de sortie sur la fin.
- Bruitages par code (`sfx/`, l'ancien synthé) : clic de souris sur les clics du curseur, carillon de l'app (deux notes) sur les notifications, « whoosh » entre les plans ; `public/sfx/*.wav`, non commités, produits par `npm run sfx`.

## Plans

| Plan | Ce qu'on voit |
| --- | --- |
| intro | le logo se construit, « Escouade » |
| chaos | des terminaux `claude` s'empilent, l'app les avale |
| projects | les onglets (couleur, point vert, Δ), la modale « Nouveau projet » (dossier, git, couleurs, premier agent, worktree), l'onglet créé |
| agents | les cartes d'agents (statut, modèle, durée, tokens, coût, fichiers), « agent-6 » renommé par Haiku, archivage |
| chat | markdown et code, outils compacts, un Edit déplié avec son diff, activité en direct, tokens et coût qui montent (≈), question en un clic, message « transmis pendant le tour » |
| composer | menus Modèle (avec version), Effort, Mode ; pièce jointe, `@fichier`, `/commande` ; carte « Tâche terminée » (Revoir les fichiers, Commit…) |
| notify | pastille d'onglet, carte qui clignote, carillon, toast Windows, barre des tâches, `Ctrl+J` ; fermer → zone de notification, reprise des sessions |
| git | moitié / moitié, non commités (cet agent / tout le projet), diff côte à côte puis unifié, historique (git graph), worktree et « Merger dans main », synchro ↓ ↑ (Pull, Push, Fetch) |
| editor | « Éditeur » : arborescence, onglets, code coloré, lignes modifiées vs main, `Ctrl+S`, bandeau « modifié sur le disque » |
| board | « Agents / Tableau », 4 colonnes, création d'un ticket (critères, boucles max), pilote auto et places libres |
| loop | worktree `ticket/<clé>` et ports, cartes « En cours » (boucle, critères, barre, avancement, activité), bilan des critères dans la conversation, étiquette « ▸ DEM-4 · boucle 2/5 » |
| test | passage « À tester », notification « prêt à tester », « ▶ Tester » : préparation, serveurs, attente HTTP, navigateur ouvert sur la fonctionnalité, logs dans « Lancement » |
| validate | « Valider et merger » (tests, commit Conventional Commits, merge), réglages du tableau (merge / PR / push / laisser, conflits, agents en parallèle), « Renvoyer », carte « Terminé » |
| launch | commandes de lancement, « Tout lancer », log, plantage avec code et notification |
| terminals | PowerShell, Git Bash, WSL : autocomplétion, historique, plein écran |
| stats | barre de statut (session 5 h et hebdo avec reset, coût du jour ≈, processus Claude), reprise après limite d'usage, page Stats (entrée / cache / sortie, par projet, par modèle) |
| remote | remote control : la session sur le téléphone, le message arrive dans l'app |
| more | proxy, raccourcis, mises à jour automatiques, Windows et macOS |
| outro | logo, gratuit et open source, lien GitHub |

`src/features.ts` liste les fonctionnalités d'Escouade (d'après `docs/SPEC.md` et le changelog), chacune avec un fragment que la voix off doit prononcer. Un test vérifie que chaque fonctionnalité est montrée par au moins un plan et dite dans ses répliques.

## Site

- Vidéo, affiche et images refaites depuis les nouveaux plans (`npm run site-images`), sous-titres `escouade.vtt` générés depuis les horaires de la voix (piste `<track>` du lecteur).
- Nouvelles fonctionnalités : tableau de tickets, lancement de test, éditeur intégré ; textes mis à jour (synchro git, reprise après limite d'usage, macOS).
- Les tests du site vérifient aussi la piste de sous-titres.

## Vérification

- Tests Vitest : script (identifiants uniques, chaque réplique dans `voice.json` avec le même texte), couverture des fonctionnalités, timeline (plans jointifs, chaque plan au moins aussi long que ses répliques), volume de la musique, sous-titres, musique faite pour la timeline actuelle.
- `npm run voice -- --check` : chaque réplique retranscrite proche de son texte.
- Revue visuelle : images fixes de chaque plan à plusieurs moments, puis images extraites du MP4 final ; durée du MP4 = durée de la timeline.

## Hors champ

Version anglaise, sous-titres incrustés, rendu en CI.
