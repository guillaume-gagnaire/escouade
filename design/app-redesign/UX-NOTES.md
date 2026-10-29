# Lot 0 du redesign 2.0 (ESC-116) : l'UX des fonctionnalités venues après l'aperçu

L'aperçu (`index.html`) a été validé le 2026-10-12 ; depuis, Escouade a gagné un deuxième harness, les comptes par harness, la passation, la voix, les pièces jointes, les sous-agents et tâches de fond, l'assistant « Nouveau projet », le serveur MCP, le remote control, les statistiques anonymes. Ce lot les dessine dans le même monde visuel, chacun à son adresse, avant tout code. Les harness étaient alors Claude Code et Codex, eux seuls (décidé le 2026-10-17 : l’app n’en connaissait pas d’autre) ; depuis le lot 0 bis (plus bas), ce sont les quatre que l’app connaît.

D'où vient l'état actuel décrit ici : les composants (`src/components`, `git diff 26b4d6c..main`), les catalogues `src/lib/i18n/{fr,en}`, et ce que Codex déclare (`src-tauri/src/engine/codex/mod.rs`, `caps()`). L'app réelle n'a pas été relancée pour ce lot.

Ce que l'aperçu suit partout :
- **Mêmes places, mêmes mots** d'un harness à l'autre : les cinq modes aux mêmes places, « Nouveaux agents » dans chaque groupe de Réglages › Harness, les comptes groupés par harness.
- **La différence dite là où elle compte**, et seulement là : le bac à sable de Codex (en-tête, menu des modes, réglages), son coût estimé (« ≈ … estimé »), ce qu'il ne fait pas (remote control, PDF, tâches de fond) dit en une ligne à l'endroit du réglage.
- **La marque du harness** est un mot (« Claude », « Codex ») dans un filet, la même sur la carte, l'en-tête, le ticket, les menus et le Kanban ; jamais une couleur : la couleur reste celle du projet.
- **Qui attend passe devant**, aussi pour les nouvelles attentes : la question de confiance de Codex met son agent en tête, ambre.

Adresses : `index.html#…` ; `?lang=en` avant le `#` pour les libellés anglais (`index.html?lang=en#modes`). Captures : `node design/app-redesign/render.mjs`.

## Fonctionnalité par fonctionnalité

### Harness : nouvel agent, marque, réglages, accueil (`#new-agent`, `#codex`, `#modes`, `#settings-claude`, `#welcome`)

- **Aujourd'hui.** Le menu « Nouvel agent sur … » n'apparaît qu'à partir de deux harness trouvés, sans raccourci pour l'autre. Les modes de Codex portent les mots de Claude Code ; leur sens propre (bac à sable) n'est que dans le détail du menu et l'infobulle : une fois choisi, plus rien ne le rappelle. Réglages › Harness mélange tout : exécutables, modèle par défaut, processus, vue, liste de tâches, pièces jointes, remote control et serveur MCP.
- **Aperçu.** Un bouton divisé : `Ctrl N` part sur le harness du projet, la flèche ouvre le menu « Harness » (chaque harness avec son modèle, son mode, son raccourci ; Claude Code ou Codex pas installé y reste, grisé, avec où le régler ; « Harness par défaut du projet… »). Le menu des modes dit le sens de chaque mode sur ce harness, et l'en-tête d'un agent Codex porte « Bac à sable » à côté de son chemin (icône seule quand la place manque). Réglages › Harness : un groupe par harness (état et version, exécutable, nouveaux agents), puis « Tous les harness » ; la différence en une ligne (« Claude Code seulement : Codex n'a pas de remote control », « Pas de PDF joint · pas de tâches de fond »).
- **États.** Prêt (une version plus récente que celles testées aussi) ; trop ancienne (rouge, refusée, avec la commande de mise à jour) ; introuvable (commande d'installation) ; aucun harness (`#welcome` : premier démarrage, fenêtre vide, les commandes à copier, « Chercher à nouveau »).
- **Pas de versions testées dans la liste (2026-10-23, ESC-159).** Décidé par l'utilisateur : ni « testée avec Escouade (de la … à la …) » ni « Version non testée » sous la version d'un harness ; seul ce qui l'empêche de démarrer ou dit quoi faire reste (trop ancienne, non prise en charge, introuvable, aucun compte). Le lot 11b (ESC-152) ne les remet pas.

### Passation à un autre harness, ESC-88 (`#handover`, `#agent-menu`)

- **Aujourd'hui.** « Continuer sur un autre harness… » vit dans le menu contextuel de la carte (clic droit seulement) et sur la carte du tour arrêté par la limite. Ce qui se passe quand un quota est épuisé est réglé à deux endroits : l'ordre des harness dans Comptes, la passation automatique et la reprise sur un autre compte dans Harness.
- **Aperçu.** Le même menu de l'agent s'ouvre au clic droit et par « … » dans l'en-tête. La passation se lit dans la conversation : la carte du quota épuisé (répondue : « Continué sur Codex à 15:42 · choisi par toi »), un séparateur « Suite sur Codex · quota Claude Code épuisé jusqu'à 18:00 », le dossier de passation replié, puis les tours marqués « Codex · modèle · mode ». Le retour n'est jamais pris seul : une bande au-dessus du champ « Le quota Claude Code est revenu… Revenir sur Claude Code ». Réglages › Comptes réunit « Quand un quota est épuisé » : autre compte du même harness, puis harness suivant, ordre des harness, modèle de passation.
- **États.** Proposée (carte ambre, boutons) ; faite (carte répondue, séparateur) ; pendant un tour (« Attends la fin du tour ») ; impossible (harness sans compte actif, grisé avec la raison) ; retour possible (bande).

### Comptes et quotas par harness (`#quota`, `#settings-accounts`, `#account-codex`)

- **Aujourd'hui.** La barre d'état montre les jauges d'un seul harness à la fois (celui du compte où partiraient les nouveaux agents) ; le coût du jour mêle exact et estimé sans le dire ailleurs que dans une infobulle.
- **Aperçu.** Un groupe de jauges par harness qui fait travailler des agents (« Claude 5 h · 7 j », « Codex 5 h · 7 j ») ; un clic ouvre le panneau des comptes, groupé par harness, chaque compte avec ses fenêtres (5 h, 7 j, ou 30 j sur l'offre gratuite de Codex), son coût du jour (« ≈ … estimé » pour Codex), « compte en cours », « au-delà du seuil de pause ». Le total du jour prend « ≈ » dès qu'il contient de l'estimé. Ajouter un compte Codex : le harness d'abord (segments), ce qui se partage (config.toml, AGENTS.md…), puis la connexion `codex login` dans un terminal, avec l'attente dite. Le Principal se réserve à des projets comme les autres comptes.
- **États.** Connecté ; offre gratuite (ambre) ; au-delà du seuil ; inactif ; connexion en attente ; projet fermé encore réservé (« mobile-app (fermé) »).

### Vue terminal du harness (`#terminal-view`)

- **Aujourd'hui.** Un bouton unique dont le libellé nomme l'autre vue (« Vue Codex » quand on est dans celle d'Escouade, et inversement) : on lit où l'on irait, pas où l'on est. La question de confiance du dossier de Codex n'est visible que dans le terminal.
- **Aperçu.** Deux segments « Escouade | Codex » dans l'en-tête, l'actif nommé, l'autre en icône. Quand Codex demande la confiance du dossier, une bande ambre au-dessus du terminal le dit (« sans confiance, il s'arrête »), et l'agent passe en tête de la barre latérale avec « Confiance du dossier : réponds dans son terminal ».

### Voix : dictée, conversation, réglages (`#dictation`, `#dictation-offer`, `#voice`, `#voice-question`, `#settings-voice`)

- **Aujourd'hui.** Micro, « Conversation » et trombone sont dans la barre du champ, entre le mode et Envoyer ; avec le menu du compte, la barre déborde vite dans une colonne étroite. Réglages › Voix se présente comme « Dictée des messages » alors qu'il règle aussi la conversation.
- **Aperçu.** La barre du champ garde à gauche ce qui règle l'agent (modèle, effort, mode) et met à droite, en icônes, ce qui part avec le message (joindre, dicter, converser), puis Stop et Envoyer. La dictée s'écoute dans le champ (niveau, durée, `Ctrl Espace` pour finir, `Échap` pour annuler) ; la première fois, l'offre de télécharger le modèle et sa progression au même endroit. La conversation est plein écran (direction A) : l'orbe, la phase (« te répond », « te pose une question »), les sous-titres « Toi » / l'agent, ce qui reste à dire en plus pâle, les choix d'une question dits comme ils se prononcent (« Le premier · Plafonner »), Pause et Arrêter. Un message dit à voix haute le reste dans la conversation (« Dit à voix haute », « Lu à voix haute »). Réglages › Voix : Dictée (micro, raccourci, langue, modèles avec vitesse et qualité, téléchargé / à télécharger / en cours) puis Conversation (voix par langue, licences, l'exception de Fuze dite en ambre, vitesse, délai avant l'envoi).
- **États.** À l'écoute ; transcription ; rien entendu ; modèle à télécharger, en cours, téléchargé ; voix indisponible (moteur absent) ; conversation en pause.

### Pièces jointes (`#attachments`, `#drop`, `#ticket`, `#board`)

- **Aujourd'hui.** Dans le formulaire du ticket, les images et les autres fichiers sont deux listes. Sur la carte du Kanban, rien ne dit qu'un ticket a des fichiers.
- **Aperçu.** Une seule rangée de puces, au message comme au ticket : vignette pour une image, icône pour un fichier, nom, taille, et ce qui lui arrive (« dans le message », « copié », « Fichier d'origine, lu là où il est ») ; un gros fichier en ambre (« joint là où il est »), une copie en cours avec sa barre. Glisser sur le champ l'entoure de la couleur du projet et dit ce qui part dans le message et ce qui part par son chemin. Sur la carte du ticket, un trombone et le nombre. Codex ne lit pas de PDF : dit dans Réglages › Harness, le PDF lui part par son chemin.
- **États.** Lu ; copie en cours ; gros fichier ; refusé (format, taille : puce rouge) ; trop de fichiers pour un ticket.

### Sous-agents et tâches de fond (`#subagent`, `#task`)

- **Aujourd'hui.** Leurs listes se déplient sous la carte de l'agent dans la barre latérale ; à 779 px de haut, avec Lancement et Terminaux, il reste deux ou trois agents visibles.
- **Aperçu.** Sur la carte, deux compteurs à côté du nom (↳ sous-agents, horloge pour les tâches de fond) ; la liste ne se déplie que pour l'agent ouvert. La vue d'un sous-agent : fil d'Ariane jusqu'à son agent, faits (type, modèle, tokens, outils, durée, tâche du plan), consigne repliée, son fil, et le champ « Lui écrire, par … » avec la raison (Claude Code ne permet pas d'écrire directement à un sous-agent). La vue d'une tâche : faits, sortie suivie en direct (« Suivre la fin »), Arrêter. À 820 px de haut et moins, Lancement et Terminaux se replient sur leur titre, un plantage toujours dit (« 1 planté »).
- **États.** En cours, terminé, échoué, arrêté, interrompu ; sortie vide, tronquée, illisible ; sous-agent fini (brouillon gardé, plus envoyable). Codex n'a pas de tâches de fond : ni compteur ni liste.

### Assistant « Nouveau projet » et projets fermés (`#wizard`, `#wizard-summary`, `#settings-projects`)

- **Aujourd'hui.** L'assistant (forme A, ESC-94) est en place ; les projets fermés sont listés dans Réglages › Projets.
- **Aperçu.** Le rail des étapes à gauche (faites cochées, l'actuelle cerclée, « facultatif »), le contenu à droite, « Créer sans l'assistant » toujours là. Le récapitulatif met d'abord ce qui empêcherait la méthode de marcher, chacun avec sa correction en un clic, puis ce qui sera réglé, harness compris. Réglages › Projets : « Harness par défaut », « Assistant de configuration », et « Projets fermés » (historique gardé, Rouvrir, Effacer ; dossier introuvable dit).

### Serveur MCP (`#codex`, `#settings-mcp`)

- **Aujourd'hui.** L'interrupteur unique et le journal d'activité sont au bas de Réglages › Harness.
- **Aperçu.** Un onglet « Serveur MCP » (Système) : l'interrupteur, où il est déclaré (par harness et par compte), les agents qui n'auront les outils qu'au prochain démarrage, puis l'activité (heure, qui, quoi, Fait / Refusé / Erreur). Dans la conversation, ce qu'un autre agent a fait porte « via Escouade » à côté de son auteur (« Autorisé pour cette session par chef-escouade », « Répondu par Claude (hors Escouade) »).

### Remote control (`#settings-claude`, `#agent-menu`)

- **Aujourd'hui.** Global dans Réglages › Harness, par projet dans Réglages › Projets, par agent au clic droit ; son état n'est qu'une icône et une infobulle sur la carte.
- **Aperçu.** Même place, dit « Claude Code seulement » ; l'icône de la carte prend le teal quand une session est connectée ; dans le menu de l'agent, l'état est écrit sous l'action (« Connecté depuis claude.ai »).

### Statistiques anonymes, mise à jour, intégrations (`#telemetry`, `#update`, `#update-failed`, `#settings-about`, `#settings-integrations`)

- **Aperçu.** La question du premier démarrage en une fenêtre courte, le détail replié, les deux réponses de même poids. La mise à jour prête : un élément de la barre d'état (« Mise à jour 1.8.0 prête · Redémarrer ») et une fenêtre qui dit ce que le redémarrage fait aux agents ; l'échec derrière un proxy reste écrit dans À propos (« Télécharger dans le navigateur »), pas seulement dans une notification. Linear est un service comme les autres dans Intégrations.

### Après la 2.0 : PR, CI et déploiements (`#hosts`)

- Leur place, sans détail : un troisième onglet du panneau git, « PR et CI », marqué « après la 2.0 », et la ligne « PR #41 → main » de la carte du ticket qui prendra l'état de la CI.

## Libellés anglais vérifiés (1028×779, captures `-en`)

- « Reserved for » ne tenait pas dans la colonne de 70 px des comptes : passée à 90 px.
- L'en-tête du Kanban (« After approval: squash merge → main », « Autopilot ») passait à la ligne : sous 1240 px, « Every slot is taken » s'efface et les contrôles ne coupent plus.
- Le titre de Lancement replié (« Launch 2/3 · 1 crashed ») ne tient qu'en effaçant « Stop all » : le bouton reste dans la ligne survolée de chaque commande.
- Les jauges de la barre d'état gardent le nom du harness et perdent les pourcentages sous 1240 px (« Claude 5h · W », « Codex 5h · W ») ; le pourcentage reste dans le panneau et pour les lecteurs d'écran.
- L'en-tête d'un agent tient en ne gardant que l'étiquette de la vue active ; « Bac à sable » / « Sandbox » devient une icône sous 1000 px de conversation.
- Tiennent tels quels : « New agent on … » et « Ctrl Shift N », les cinq modes et leur sens, « Continued on Codex », « Handover file », « Back to Claude Code », « Codex part estimated », « Listening », « Said aloud », « Follow the end », « Add an account… », les onglets de réglages (« MCP server »).

## Choix validés par l'utilisateur le 2026-10-17

Tous les choix ci-dessous ont été validés sur les captures : c'est le design que suivent les lots ESC-71 à 85.

1. **Ctrl+Maj+N** ouvre le menu « Harness » du nouvel agent (raccourci nouveau) ; Claude Code ou Codex pas installé y est grisé avec où le régler, alors qu'aujourd'hui une fonction absente est cachée.
2. **Réglages réorganisés** : un onglet « Serveur MCP » (Système) prend l'interrupteur et le journal, qui quittent Réglages › Harness ; « Quand un quota est épuisé » réunit dans Comptes la reprise sur un autre compte, la passation automatique, l'ordre des harness et le modèle de passation (aujourd'hui partagés entre Harness et Comptes). Réglages › Harness devient un groupe par harness.
3. **Bascule de vue en deux segments** (« Escouade | Codex ») au lieu d'un bouton qui nomme l'autre vue.
4. **Barre du champ réordonnée** : joindre, dicter et converser passent à droite, en icônes ; « Conversation » perd son mot (infobulle et `Ctrl+Maj+Espace`, raccourci nouveau).
5. **« Éditeur » retiré de l'en-tête de l'agent** : il reste dans la barre de titre (doublon), la place va à la bascule de vue et au menu « … ».
6. **Menu de l'agent aussi par « … »** dans l'en-tête (aujourd'hui au clic droit seulement).
7. **Barre d'état : un groupe de jauges par harness** au lieu de celui du seul harness des nouveaux agents ; le total du jour prend « ≈ » dès qu'il contient de l'estimé.
8. **Bac à sable affiché** dans l'en-tête d'un agent Codex (mot nouveau dans l'interface : « Bac à sable » / « Sandbox »).
9. **Lancement et Terminaux repliés** sur leur titre dans une fenêtre de 820 px de haut ou moins. Le titre les déplie d’un clic et les replie (précisé le 2026-10-25 : sans lui, rien ne lançait plus une commande depuis la barre sur un écran de portable).
10. **Sous-agents et tâches de fond** : deux compteurs sur la carte, la liste dépliée seulement pour l'agent ouvert.
11. **Pièces jointes du ticket** en une seule rangée (images et fichiers ensemble), et le trombone avec le nombre sur la carte du Kanban.
12. **Accueil sans harness** : « Chercher à nouveau » relance la détection sans redémarrer l'app.
13. **Échec de mise à jour** gardé dans Réglages › À propos tant qu'il dure, en plus de la notification.

# Lot 0 bis (ESC-150, 2026-10-23) : les fonctionnalités de la 1.8.0 à la 1.8.9

Ce lot étend l'aperçu à ce que la 1.8.0 à la 1.8.9 ont ajouté et qu'il ne dessinait pas, dans les deux thèmes (ESC-129), chaque surface à son adresse. Il part de `CHANGELOG.md` (1.8.0 à 1.8.9), puis des composants de `src/` et des catalogues `src/lib/i18n/{fr,en}` pour les vrais libellés et états ; rien n'a changé dans `src/`. Les vues « Pull requests » et « Environnements » du projet (ESC-65, ESC-67) n'y sont pas : elles se dessinent dans leurs tickets.

**La règle des harness change.** Celle du lot 0 (« Claude Code et Codex, eux seuls », 2026-10-17) devient : **les quatre harness que l'app connaît, Claude Code, Codex, OpenCode et Copilot CLI ; Cursor CLI reporté**, absent de l'aperçu. Leurs marques sont des mots dans un filet : « Claude », « Codex », « OpenCode », « Copilot ».

**Ce qui partage une surface avec le lot 0** (barre d'état, panneau des quotas, liste des agents) n'apparaît qu'aux adresses de ce lot : les captures des autres écrans restent celles validées, octet pour octet. Le design à suivre est celui des adresses ci-dessous.

## Adresses

Nouvelles (`index.html#…`, avec `?lang=en` ou `?theme=light` avant le `#`) :
- harness et comptes : `#harness-opencode`, `#accounts-copilot`, `#accounts-quotas`, `#account-machine`, `#no-account`, `#welcome-account` ;
- quotas : `#overage` (et `#quota`, changé) ;
- les nouveaux harness dans la conversation : `#opencode`, `#modes-opencode`, `#copilot`, `#models`, `#opencode-view`, `#copilot-view` ;
- agents : `#switch-account`, `#waiting`, `#run-worktree`, `#run-conflict`, `#worktree-kept` ;
- PR d'un agent : `#pr-open`, `#pr` ;
- voix : `#voice-live`, `#dictation-live`, `#settings-voice-online`, `#voice-online-voice` ;
- réglages : `#code-hosts`, `#private`, `#private-accounts`, `#escouade-yml`, `#escouade-yml-pending`, `#escouade-yml-board`, `#worktree-cleanup` ;
- assistant : `#wizard-private`.

Changées (écrans du lot 0 mis à jour à leur adresse) : `#new-agent`, `#agent-menu`, `#quota`, `#account-codex`, `#voice-question`, `#welcome`, `#settings-claude` (Harness), `#settings-accounts`, `#settings-app`, `#settings-voice`, `#settings-board`, `#settings-projects`, `#settings-integrations`, `#settings-mcp`, `#settings-about`.

Déjà dessiné, rien à changer : le modèle réel d'un sous-agent (1.8.9) est la ligne « Modèle » de `#subagent`.

## Libellés anglais vérifiés (1028×779, captures `-small-en`)

- Le menu « Harness » tient avec « The project’s · Main · Opus 5.5 · Auto » et « Add an account… » sur la ligne de Copilot CLI.
- Sous une carte, « Agent addresses the feedback » tient à côté du numéro de PR seulement si l'état passe dans l'infobulle : c'est le choix 26.
- « Wakes at 15:30 » tient à la place de l'état ; « Driven agents · 2 at work » comme le titre des sous-agents.
- Réglages › Comptes : « The harness’s (Sonnet 5.5) » tient dans le menu du modèle d'un compte.
- Sous 1240 px de large, la barre d'état perd aussi les projections et « extra usage », comme les pourcentages au lot 0 : ils restent dans le panneau.
- À 820 px de haut et moins, les lancements dans un worktree se replient avec Lancement (choix 9 du lot 0).

## Choix validés par l’utilisateur le 2026-10-23 (à la suite des 13 du lot 0)

14. **Quatre harness, mêmes règles** que le lot 0 : mêmes places, mêmes mots, la différence dite là où elle compte. Raison : l'app les fait tourner depuis la 1.8.0 ; le lot 0 n'en connaissait que deux. Adresses : `#new-agent`, `#opencode`, `#copilot`.
15. **Le menu « Harness » numérote ses harness** (`1` à `4` une fois ouvert par `Ctrl Maj N`) au lieu d'un raccourci global par harness, et garde un harness sans compte à sa place, avec « Aucun compte » et « Ajouter un compte… ». Raison : quatre raccourcis ne se retiennent pas ; l'absence de compte se voit avant de créer l'agent, pas à son démarrage. `#new-agent`.
16. **Le même menu porte l'agent sans worktree** (ou dans un worktree, l'inverse du réglage du projet), sous un titre « Worktree ». Raison : un seul menu à côté de « Nouvel agent », comme dans l'app 1.8.9. `#new-agent`.
17. **Ce qu'un harness n'a pas reste à sa place, grisé avec sa raison** (le mode Auto d'OpenCode : « OpenCode n'a pas de mode Auto »), alors que l'app 1.8.0 le masque. Raison : la règle du lot 0 « les cinq modes aux mêmes places » ; un mode qui disparaît fait chercher. `#modes-opencode`.
18. **Les réglages des nouveaux agents sont une ligne de la fiche de chaque compte** (modèle, effort, mode, toujours visibles), et « Compte de la machine » une marque sur la ligne du dossier. Réglages › Harness ne garde que l'exécutable, la version et ce qui vaut pour tous, sans « Se connecter… ». Raison : la 1.8.1 les a passés au compte ; les voir d'un coup d'œil évite d'ouvrir chaque compte. `#settings-accounts`, `#settings-claude`.
19. **Un harness sans compte reste visible partout où il compte** : son groupe dans Comptes avec un bouton principal « Ajouter un compte Copilot CLI… », l'accueil « Aucun compte » avec le compte trouvé sur la machine, et la carte ambre d'un agent créé sans compte, qui propose aussi « Continuer sur Claude Code » (le harness du projet). Raison : rien ne démarre en silence (1.8.1), et la sortie est à un clic. `#accounts-copilot`, `#welcome-account`, `#no-account`.
20. **« Changer de compte à » en segments** (80 %, 90 %, 95 %, « À la limite ») en tête de « Quand un quota est épuisé », et la projection dans un groupe « Quotas ». Raison : les deux seuils (pause, changement) se lisent ensemble. `#accounts-quotas`.
21. **La barre d'état ne montre une projection que quand elle passe 80 %** (orange, rouge au-delà de 100 %) ; le panneau les montre toutes. L'app 1.8.9 les montre toujours. Raison : avec quatre harness, la barre n'a pas la place, et une projection verte ne demande rien. `#quota`.
22. **L'usage supplémentaire facturé est un mot ambre dans le groupe du harness** (« usage supplémentaire »), sa phrase sous le compte dans le panneau. Raison : il coûte de l'argent, il doit se voir sans ouvrir le panneau. `#overage`.
23. **OpenCode montre son coût du jour sur son budget** (« 0,42 $ / 5 $ ») à la place des jauges, et Copilot CLI une jauge « crédits » remise à zéro le 1er du mois. Raison : ce sont leurs vraies limites. `#quota`.
24. **Un changement de compte au seuil se lit comme une passation** : un séparateur « Suite sur le compte Pro · Principal est à 90 % de sa fenêtre de 5 h », les tours suivants marqués de leur compte, et une notification. Raison : même geste que le changement de harness du lot 0. `#switch-account`.
25. **« Attend » et l'heure de relance** remplacent « Terminé », avec un anneau vide (le tour est fini, pas le travail), et l'infobulle dit quoi ; les agents pilotés se replient sous la carte comme les sous-agents. Raison : « Terminé » mentait (1.8.9). `#waiting`.
26. **Les PR d'un agent sous sa carte : un numéro avec le glyphe de son état et le seul bouton de correction utile** (« L'agent corrige », « L'agent corrige les retours »), l'état en mots dans l'infobulle ; la PR de l'agent ouvert à côté de sa branche dans l'en-tête. « Ouvrir une PR » prend la place de « Préparer le lancement » dans l'en-tête tant que la branche a des commits sans PR ; le lancement reste dans « … ». Raison : la colonne fait 250 px, et l'en-tête n'a qu'une place de bouton (choix 5). `#pr`, `#pr-open`.
27. **Les hébergeurs de code en tête de Réglages › Intégrations**, une ligne par instance : vert connecté, ambre droits manquants, rouge jeton expiré avec « Reconnecter… » en bouton principal ; un hébergeur sans compte est nommé avec le projet qui en a besoin. Raison : ils servent les PR de chaque agent, Jira et Trello seulement les tickets importés. `#code-hosts`.
28. **Un projet privé porte un cadenas sur son onglet** ; dans ses réglages, « Ses fichiers » est un tableau harness par harness (« Garde-fou » neutre, « Barrière » verte) et « Ses comptes » dit le compte réservé et, en ambre, le compte partagé avec de quoi y remédier. Ces deux parties n'apparaissent que pour un projet privé. Raison : dire honnêtement ce qui protège (ESC-140), sans charger les projets ordinaires. `#private`, `#private-accounts`.
29. **Trois marques pour l'origine d'un réglage** : teal « Défini par .escouade.yml », neutre « Réglage local de cette machine » (avec « Écrire dans .escouade.yml » et « Reprendre la valeur du fichier »), ambre « en attente de ton accord » ; les commandes à approuver se lisent en entier dans un bloc ambre, et le Kanban le rappelle par une bande « Les lire ». Raison : une commande venue du dépôt ne tourne jamais sans avoir été lue. `#escouade-yml`, `#escouade-yml-pending`, `#escouade-yml-board`.
30. **Le ▶ d'une commande ouvre « Dans le projet / Dans le worktree de … »** dès qu'un agent a un worktree, et le lancement tourne sous le nom de l'agent dans Lancement ; relancer ce qui tourne ailleurs demande, « Arrêter l'autre et lancer » en premier. Raison : le choix du dossier se fait au moment de lancer, sans réglage. `#run-worktree`, `#run-conflict`.
31. **Un worktree gardé se dit en ambre sous la carte archivée** (« worktree gardé : 2 commits ni mergés ni poussés ») et dans la notification de la validation ; « Nettoyer » liste ce qu'il garde et pourquoi. Raison : rien ne se perd sans qu'on le sache (ESC-143). `#worktree-kept`, `#worktree-cleanup`.
32. **En conversation vocale, une question à la fois** : « Question 1 sur 2 », les choix en touches numérotées sous les sous-titres, et « Dis le choix, son numéro, ou ta propre réponse ». Raison : les choix s'affichent sans être lus (1.8.2). `#voice-question`.
33. **La transcription en direct s'écrit en plus pâle**, suivie de « ElevenLabs · en direct » (point rouge), dans la dictée comme dans la conversation. Raison : on voit que le texte n'est pas fini et qu'il part en ligne. `#dictation-live`, `#voice-live`.
34. **Réglages › Voix : « Transcription » puis « Conversation », chacun avec « Local | En ligne » en segments** ; en ligne, service, modèle, clé, temps réel et ce qui part, écrit sous le choix ; la voix locale de secours dite en une ligne. Raison : la même question (sur ma machine ou en ligne ?) pour les deux sens de la voix. `#settings-voice`, `#settings-voice-online`, `#voice-online-voice`.
35. **Le zoom en segments** (90 à 150 %), le palier trop grand pour la fenêtre grisé, et « Modèle des textes rédigés » dans un groupe « Textes rédigés ». Raison : les paliers usuels d'un clic. `#settings-app`.
36. **Un long menu de modèles** : recherche, « Favoris » (étoile pleine à la couleur du projet), un groupe par famille ou éditeur, le coût ×N à droite. Raison : celui de l'app 1.8.1, dans le monde visuel de l'aperçu. `#models`.
37. **La vue OpenCode porte une bande verte « Escouade garde le fil »** au lieu de la bande ambre de confiance des vues Codex et Copilot CLI. Raison : c'est la seule vue terminal où la conversation reste à jour. `#opencode-view`, `#copilot-view`.
38. **Les nouvelles demandes de la conversation reprennent les cartes du lot 0** : « Toujours autoriser » d'OpenCode dit le motif gardé avant le clic ; le plafond de crédits de Copilot CLI est une carte ambre comme la limite d'usage (« Ajouter 50 crédits », « Lever le plafond », « Arrêter là ») ; le formulaire d'un serveur MCP, une carte à champs. Raison : une demande, une carte, quel que soit le harness. `#opencode`, `#copilot`.

# La barre des projets (ESC-158, 2026-10-23) : les projets à gauche, comme Slack

Demandé par l'utilisateur le 2026-10-23 : « fais la navigation des projets sur la gauche comme le fait Slack, au lieu des tabs en haut ». Les onglets de la barre de titre deviennent une barre de tuiles à gauche, de la hauteur de la fenêtre entre la barre de titre et la barre d'état, sur tous les écrans de l'aperçu (la coque est commune), en sombre et en clair, en français et en anglais. Rien n'a changé dans `src/` ; les libellés du menu d'une tuile sont ceux de l'app (`nav.titleBar.menu`, `src/lib/i18n/{fr,en}`), Ctrl+1…9 est déjà un raccourci de l'app (`src/lib/shortcuts.ts`).

**Validés par l’utilisateur le 2026-10-23** (« Ça me va ! ») : les choix 39 à 49 ci-dessous.

## Adresses

- Tous les écrans, à leurs adresses habituelles : la barre remplace les onglets partout (`index.html`, `#board`, `#overview`, `#stats`, `#settings-…`, les écrans du lot 0 et du lot 0 bis).
- `#rail` : dix-huit projets, la barre qui défile, l'actif (`notif-service`) gardé en vue, une tuile survolée (son infobulle) et une autre glissée (sa place en creux, le trait où elle tombe) ; `#rail-menu` : le menu d'une tuile ; `#rail-add` : le menu de « + » avec les projets fermés.
- `?os=mac` avant le `#` : la barre de titre de macOS (ses trois boutons à gauche, au-dessus de la barre des projets, dans les 84 px que l'app leur laisse déjà ; sans boutons à droite). Capture `agents-mac.png`.
- Captures : `rail`, `rail-menu`, `rail-add`, `agents-mac` et leurs jumelles `-light` à 1440×900 ; `rail-small`, `rail-small-light`, `rail-small-en`, `rail-small-light-en`, `rail-menu-small-en`, `rail-add-small-en` à 1028×779.

## La place

- La barre des projets fait **52 px** (tuiles de 36 px, 8 px de chaque côté). La barre des agents garde ses **276 px** ; sous 1240 px de large, sa marge gauche passe de 10 à 6 px. Le panneau perd donc 52 px : 692 px à 1028×779 au lieu de 744.
- Une largeur de 256 px pour la barre des agents a été essayée sous 1240 px : la question de l'agent qui attend passait sur deux lignes et un agent sortait de la vue. La barre des agents prime : elle reste à 276 px.
- La colonne de la conversation était une grille dont la colonne implicite grandissait avec son contenu ; avec 52 px de moins, le dossier de passation la poussait hors du panneau (`#handover` en anglais). Elle est maintenant `minmax(0, 1fr)` : une ligne trop longue se coupe en « … ».
- La recherche garde son libellé « Rechercher Ctrl K » à 1028 px (340 px au plus, un tiers de la fenêtre) : sans les onglets, la barre de titre a la place.
- Vérifié à 1028×779 : barre des agents, panneau, zone de saisie (`agents-small`, `handover-small-en`, `modes-small-en`, `pr-small-en`, `subagent-small-en`, `quota-small-en`, `git-small`) ; les replis en petite hauteur (choix 9) et la barre d'état (choix 21) sont inchangés.
- La barre des projets a sa hauteur pour elle : avec la Vue d'ensemble, « + », Stats et Réglages, elle montre 13 projets à 1440×900 et 10 à 1028×779 avant de défiler, un de plus quand aucun projet caché n'attend (mesuré sur `#rail`).

## Libellés anglais vérifiés (1028×779, captures `rail-*-en`)

- Tiennent : « Search Ctrl K » au centre, « Editor », le nom du projet et son Δ, les infobulles (« 1 agent waiting for you », « Δ 3 · uncommitted changes », « Nothing running, nothing changed »), le menu d'une tuile (« Turn off a worktree per agent », « Close the project… »), celui de « + » (« Closed projects · their history is kept », « Reopen »).

## Choix validés le 2026-10-23 (à la suite des 38 premiers)

39. **Les projets dans une barre de tuiles à gauche, comme les espaces de travail de Slack**, à la place des onglets : la Vue d'ensemble en haut, un filet, une tuile par projet, « + », puis Stats et Réglages en bas. Raison : la demande de l'utilisateur ; des onglets à noms longs ne tiennent pas au-delà de quatre ou cinq projets, une colonne en montre dix à quatorze et défile ensuite. Adresses : `index.html`, `#rail`.
40. **Une tuile, c'est la couleur du projet et deux lettres** : les initiales des deux premiers mots du nom (`demo-api` → « DA », `mobile-app` → « MA »), sinon ses deux premières lettres (`infra` → « In »), en encre sombre (`--on-tile`) sur la couleur. Pas d'icône : l'app n'en a pas pour un projet, et la couleur seule ne suffit pas (13 teintes qui se répètent au-delà, daltonisme) ; deux lettres se lisent à 36 px. Adresse : `index.html`.
41. **Ce que disait l'onglet, sur la tuile** : le nombre d'agents qui attendent en pastille ambre en haut à droite, un point vert en bas à droite quand un agent travaille, le cadenas d'un projet privé (choix 28) en bas à gauche. **Le Δ passe dans l'infobulle**, avec le nom, `Ctrl 1…9`, l'attente et le travail en mots (« 2 agents travaillent ») ; pour le projet actif, il reste visible à côté de son nom dans la barre de titre. Raison : un nombre de plus sur 36 px ne se lit pas, et le Δ ne demande rien. Adresses : `index.html`, `#rail`, `#private`.
42. **L'actif est marqué d'un trait de 4 px au bord de la barre et d'un anneau**, à la couleur du projet (`--p-ink`), et tout le teint suit comme aujourd'hui ; une tuile survolée a un trait court. Dans la Vue d'ensemble et les Stats, c'est leur tuile qui prend l'anneau teal, les projets aucun. Adresses : `index.html`, `#overview`, `#stats`.
43. **La barre de titre sans onglets** : au-dessus de la barre des projets, le logo (Windows, Linux) ou les trois boutons de macOS ; au-dessus de la barre des agents, le **nom du projet actif** (pastille, nom, cadenas, Δ, ⌄ qui ouvre son menu) ; la recherche **au centre**, comme Slack ; à droite, Éditeur et les boutons de Windows. Raison : le nom du projet ne se lit plus sur une tuile, il se dit là sans prendre de hauteur ; Éditeur reste en haut car il ouvre l'éditeur du projet (choix 5). Adresses : `index.html`, `?os=mac`.
44. **Stats et Réglages en bas de la barre des projets** : Stats quitte la barre de titre, Réglages la barre d'état (le son des notifications y reste). Raison : ce qui ne dépend d'aucun projet est au même endroit, comme le bas de la barre de Slack ; la barre d'état gagne une place (choix 21). Adresses : `index.html`, `#stats`.
45. **Le menu d'une tuile** (clic droit, ou le nom du projet dans la barre de titre) reprend celui de l'onglet : Renommer…, Couleur, le worktree par agent, Réglages du projet…, Ouvrir le dossier, Fermer le projet… ; il gagne **Monter et Descendre (`Alt ↑`, `Alt ↓`)**, l'équivalent au clavier du glisser-déposer. Raison : réordonner sans souris. Adresse : `#rail-menu`.
46. **Beaucoup de projets** : la liste défile entre la Vue d'ensemble et « + », ses bords fondus disent qu'il y en a d'autres ; l'actif est ramené dans la vue à chaque changement (clic, `Ctrl 1…9`) ; un projet hors de vue qui attend se signale par une pastille ambre sous la liste (« ⌄ 2 »). Glisser une tuile laisse sa place en creux, un trait à la couleur du projet dit où elle tombe. Raison : rien d'urgent ne se cache en bas de la liste. Adresse : `#rail`.
47. **Les projets fermés gardés n'ont pas de tuile** : « + » ouvre un menu court, « Ajouter un projet… » (l'assistant) puis les derniers projets fermés avec « Rouvrir », et « Tous les projets fermés… » (Réglages › Projets) ; sans projet fermé, « + » ouvre l'assistant directement, comme aujourd'hui. Raison : la barre ne montre que ce qui est ouvert, comme Slack, et rouvrir reste à deux clics. Adresse : `#rail-add`.
48. **Les largeurs** : barre des projets de 52 px, barre des agents gardée à 276 px, le panneau qui perd 52 px, la colonne de la conversation qui rétrécit avec lui (voir « La place »). Raison : la barre des agents et la zone de saisie passent avant la largeur du fil. Adresses : `agents-small`, `handover-small-en`.
49. **Au premier démarrage**, la barre ne garde que « + » et Réglages, et la barre de titre que le logo et les boutons de la fenêtre. Raison : le chemin vers un premier projet et vers les réglages, rien d'autre. Adresse : `#welcome`.

# Les colonnes étroites (ESC-166, 2026-10-24) : l'en-tête d'un agent se replie par la largeur de sa colonne

Remonté par l'utilisateur le 2026-10-24 : l'en-tête d'un agent débordait quand le panneau de droite (fichiers, diff, historique) était ouvert. Dans l'aperçu aussi : à 1028 px, `#git` ne laissait du nom qu'un « p » et coupait le chemin avant la branche. Les replis suivent la largeur de la colonne de la conversation (`@container conv`), pas celle de la fenêtre ; l'app (`Conversation.svelte`) et l'aperçu ont les mêmes paliers.

**Validés par l'utilisateur le 2026-10-24** (« Les screens sont ok ! », sur les captures de `final/choice-*`) : les choix 50 à 52.

## Adresses

- `#git` à 1028×779 (`git-small`, `git-small-light`) : le panneau ouvert, la colonne à 352 px ; `#git` et `#history` à 1440×900 (`git`, `history` et leurs jumelles `-light`) : la colonne à 618 px.
- L'app, mesurée par `tests/e2e/narrow-columns.spec.ts` : 1028, 1280 et 1440 px, panneau fermé, ouvert à côté de la disposition classique (340 px, sa largeur par défaut) et côte à côte (44 % du panneau de travail, sa plus grande largeur), en français et en anglais, puis toutes les largeurs de 1000 à 1960 px, aussi dans la vue Claude Code.

## Choix

50. **Les paliers de l'en-tête, colonne de la conversation** : sous 1100 px, « Durée » s'en va et les vues ne gardent que leur icône ; sous 960 px, « Fichiers » ; sous 880 px, « Tokens » ; sous 800 px, les boutons perdent leurs mots (icône et infobulle) ; sous 680 px, la disposition passe dans « … » ; sous 580 px, Contexte et Coût s'en vont ; sous 440 px, les vues passent dans « … ». Le nom (au moins 80 px avant ses « … »), l'état et le bouton principal restent toujours. Raison : le nom et l'état disent de quel agent on parle ; les chiffres se relisent sur sa carte. Les deux largeurs déjà comparées (1104 et 692 px, panneau fermé) ne changent pas.
51. **Le chemin cède le nom du projet avant la branche** : « d… / escouade/pagination-users ». Raison : le projet est déjà dit dans la barre de titre, la branche non.
52. **« … » reprend ce que l'en-tête ne montre plus** : la disposition (« Conversation et fichiers côte à côte » ou « Disposition classique », `Ctrl Maj L`) et « Voir les fichiers non commités » / « Fermer les fichiers » (`Ctrl Maj B`) quand la figure « Fichiers » est repliée ; les vues y étaient déjà (« Vue Claude Code », « Vue Escouade »), comme « Préparer le lancement ». Raison : rien de ce qu'offrait l'en-tête ne devient inaccessible.

## Vérifié panneau ouvert, sans retouche

- La zone de saisie, le bandeau Plan (ses replis `@container conv` du lot 5), la carte d'une question, les onglets et le haut de « Non commités » et d'« Historique » dans le panneau à 340 px : rien ne déborde ni ne passe sur deux lignes, en français et en anglais.
- Les en-têtes d'un terminal et du log d'un lancement ne sont jamais à côté du panneau (il n'accompagne que la conversation) : leur colonne la plus étroite est celle de la fenêtre à 1028 px, déjà vérifiée par `terminals-look.spec.ts`.

# L'éditeur et les recherches (ESC-81, lot 9, 2026-10-24)

L'aperçu dessine l'éditeur (`#editor`) sans ce que l'app y fait de plus : les actions de l'arbre, la recherche dans les fichiers, « Voir les changements », les fenêtres « Ouvrir un fichier » (Ctrl P) et « Rechercher dans les conversations » (Ctrl K). Les replis suivent la place de l'éditeur (`@container editor` et `editor-pane`), pas la largeur de la fenêtre.

**Validés par l'utilisateur le 2026-10-24** (« Les screens sont ok ! », sur les captures de `final/choice-*`) : les choix 53 à 56.

## Adresses

- `#editor` à 1440×900 et 1028×779, clair et sombre : comparé à l'app par `tests/e2e/editor-look.spec.ts`.
- L'app à 1028, 1280 et 1440 px, panneau latéral fermé, seul (340 px) et côte à côte (44 %), en français et en anglais : rien ne déborde, dans l'éditeur (barre, titre de l'arbre, chemin, recherche flottante, ligne d'état) ni dans les deux fenêtres de recherche (même spec).

## Choix

53. **Les actions de l'arbre viennent au survol** : Nouveau fichier, Nouveau dossier, Actualiser et Tout réduire, en icônes, passent par-dessus le compte (« 128 · 3 modif. ») quand la colonne est survolée ou tient le focus, comme dans VS Code ; Fichiers et Recherche restent en deux icônes à droite du titre, celle de la vue montrée allumée, et le titre dit laquelle (« Fichiers », « Recherche »). Raison : l'aperçu n'en montre aucune, et le compte se lit au repos.
54. **Plus d'icône de type de fichier** : l'arbre n'en a pas, comme l'aperçu (les lettres et signes de la 1.x, « TS », « ⚛ », « ▣ », dessinaient des icônes avec des caractères) ; les listes (résultats de la recherche, « Ouvrir un fichier ») ont une seule icône de fichier, celle de la famille de l'app. Raison : une seule famille d'icônes, aucune couleur en dur.
55. **Les replis de l'éditeur, par sa place** : l'arbre fait 200 px dans un éditeur de 904 px ou moins (240 au-delà, tant qu'on ne l'a pas redimensionné) ; la recherche 340 px dans un panneau de 680 px ou moins, jamais plus large que le texte ; sous 600 px de panneau, « Voir les changements » ne garde que son icône (son nom en infobulle) ; sous 420 px, l'encodage et la fin de ligne quittent la ligne d'état ; sous 340 px, ce qui a changé se dit par son compte et l'indentation s'en va ; sous 560 px d'éditeur, « Enregistrer » perd sa touche et la source « worktree » ; sous 460 px, « Enregistré » s'efface. Raison : les deux largeurs comparées (1104 et 692 px) restent celles de l'aperçu, et le nom du fichier, sa position et l'état d'enregistrement restent toujours.
56. **La recherche dans le fichier dit ce qu'elle trouve** : « 1 sur 3 » quand le curseur est sur une occurrence, « 3 résultats » sinon, « Aucun résultat », « 1000+ » au-delà de mille ; une expression régulière invalide entoure le champ de rouge ; « Tout » se lit « Tout remplacer » pour un lecteur d'écran. Raison : le compte de l'aperçu, avec ses autres états.

## Vérifié, sans retouche

- L'éditeur n'est jamais à côté du panneau latéral : il prend tout le panneau de travail (le panneau n'accompagne que la conversation), donc sa place ne dépend que de la fenêtre et de la barre des agents.
- « Ouvrir un fichier » et « Rechercher dans les conversations » sont des fenêtres au-dessus de tout, de 680 px au plus : à 1028 px aussi, leur champ, leur portée et leurs touches tiennent, panneau ouvert ou non.
- Il n'y a pas de recherche dans les réglages (comptes, voix, harness) : rien à reprendre dans ce lot.

# Les réglages (ESC-152, lot 11b, 2026-10-24)

L'app suit `#settings` et ses onglets, et les adresses du lot 0 bis (`#private`, `#private-accounts`, `#worktree-cleanup`, `#accounts-quotas`) : `tests/e2e/settings-look.spec.ts` les compare dans les deux thèmes et aux deux tailles. Les choix 2 (onglet « Serveur MCP », « Quand un quota est épuisé » dans Comptes), 13 (échec de mise à jour gardé dans À propos), 18 à 20, 27 à 29, 31, 34 et 35 y sont en place. La fenêtre des réglages est au-dessus de tout : le panneau latéral ne change pas sa largeur ; ses replis suivent la largeur de son corps (`@container settings`).

**Validés par l'utilisateur le 2026-10-24** (« Les screens sont ok ! », sur les captures de `final/choice-*`) : les choix 57 et 58.

## Choix

57. **Une ligne de réglage garde la place de son nom** : son contrôle prend au plus 62 % du corps ; plus large (des segments nombreux), il passe à la ligne ; sous 520 px de corps, le contrôle se met sous ce qu'il règle, et les lignes d'un compte passent à la ligne sous 620 px. « Langue parlée » (neuf langues) devient un menu, comme dans l'aperçu. Raison : à 1028 px, des segments de neuf langues laissaient à leur nom une colonne d'un mot.
58. **« Ordre des harness » en puces numérotées, chacune avec ses flèches** (« 1 Claude Code ↑ ↓ ») dans « Quand un quota est épuisé », au lieu des puces sans flèches de l'aperçu. Raison : l'aperçu ne dit pas comment l'ordre change ; les flèches de l'app 1.8 restent, au clavier comme à la souris.

## Vérifié, sans retouche

- À 1028, 1280 et 1440 px, en français et en anglais, chaque onglet (un projet privé compris) : rien ne déborde, rien ne se chevauche, aucun bouton, onglet ou titre ne passe sur deux lignes (`settings-look.spec.ts`, « nothing of the settings overflows »).
- Restent pour plus tard : « Données locales » n'a pas le bouton « Ouvrir le dossier » de l'aperçu (le chemin du dossier de l'app n'est pas connu de la fenêtre) ; les comptes n'ont pas la ligne « Aujourd'hui 0,42 $ » d'OpenCode à la place des jauges (son budget a sa ligne) ; le Harness de l'aperçu montre une ligne « Coût » pour Codex que l'app n'a pas.

# La tuile d'un projet à soi (ESC-167, 2026-10-25) : ses lettres, son logo

Demandé par l'utilisateur le 2026-10-25 : « pouvoir personnaliser les lettres dans les icônes de chaque projet et aussi de pouvoir mettre le logo qu'on veut (qui remplacerait les lettres) ». Fait dans l'app (`ProjectTile.svelte`, `TileFace.svelte`, Réglages › Projets, l'assistant) et dans l'aperçu ; les choix 59 à 64 sont **validés par l’utilisateur le 2026-10-25** (« Tout me va », sur les captures `tile*`).

## Adresses

- `#tile` : la barre des projets avec des lettres choisies (`demo-api`, « API », l'actif), un logo opaque (`studio-web`, deux agents qui attendent par-dessus, son infobulle), un logo transparent sur la couleur du projet (`mobile-app`, un agent au travail), un emoji (`infra`, privé, un agent à voir). Les logos de l'aperçu sont dessinés avec les jetons ; l'app montre l'image de l'utilisateur dans un `<img>`.
- `#tile-settings` : Réglages › Projets, « Lettres de la tuile » et « Logo », un logo choisi.
- Captures : `tile`, `tile-settings` et leurs jumelles `-light` à 1440×900 ; `tile-small`, `tile-small-light`, `tile-settings-small-en`, `tile-settings-small-light-en` à 1028×779.

## Choix

59. **Trois lettres au plus, ou un emoji seul** : une à trois lettres ou chiffres (« API », « v2 »), ou un seul emoji ; le reste de ce qui est tapé (espaces, signes, une quatrième lettre) ne s'écrit pas. Vide, la tuile garde les initiales du nom, montrées en indication dans le champ. Raison : dans la tuile de 36 px (la barre en fait 52), trois lettres tiennent à 11 px, une quatrième touche les bords ; deux lettres restent à 13 px comme les initiales, un emoji passe à 18 px.
60. **Le logo remplace les lettres, entier, dans la forme de la tuile** : coins arrondis de la tuile, l'image contenue (`object-fit: contain`, jamais rognée), sur la couleur du projet là où elle est transparente. Raison : un logo large (un nom écrit) reste lisible, et un logo transparent garde la couleur qui teinte la fenêtre.
61. **L'état de la tuile reste par-dessus le logo** : la pastille ambre des agents qui attendent, le point vert du travail, le cadenas d'un projet privé, l'anneau de l'alerte qui respire, l'anneau et le trait de l'actif ; tous sont détourés du cadre (`--frame`), lisibles sur n'importe quelle image et dans les deux thèmes. Le Δ reste dans l'infobulle (choix 41). Au survol, le logo s'éclaircit un peu, comme la couleur d'une tuile sans logo.
62. **Le réglage, dans Identité, après la couleur** : « Lettres de la tuile » avec la tuile telle que la barre la montrera, puis « Logo » avec « Choisir une image… » et « Retirer » ; les mêmes champs dans l'assistant, sous le nom et la couleur. Comme le reste des réglages, un logo choisi ne s'applique qu'à « Enregistrer ». Raison : la tuile se règle là où se règlent son nom et sa couleur.
63. **PNG, JPEG, WebP, SVG ou ICO, 1 Mo au plus**, reconnu par son contenu et non par son extension ; au-delà, ou pour un fichier illisible ou d'un autre format, un message dit pourquoi (« « big.png » pèse 3 Mo : un logo pèse 1 Mo au plus. ») et la tuile reste comme elle était. Raison : un logo de 512 px pèse bien moins, une icône ICO avec toutes ses tailles aussi.
64. **Local au poste, copié dans les données d'Escouade** : lettres et logo ne vont pas dans `.escouade.yml` (comme le nom et la couleur) ; le logo est copié sous le projet (`projects/<id>/` du dossier de données), il ne dépend plus du fichier choisi ; fermé puis rouvert, le projet les garde ; fermé avec son historique effacé (ou son historique effacé plus tard), la copie s'en va. L'assistant ne copie pas la couleur d'un projet existant (« Copier la configuration d'un projet ») : il n'en copie pas non plus les lettres ni le logo.

## Vérifié

- Un SVG n'exécute rien : affiché par `<img>` (une URL `data:`), jamais mis dans le DOM ; `tests/e2e/project-tile.spec.ts` choisit un SVG avec un `<script>` et un `onload` et vérifie qu'ils n'ont pas tourné.
- La tuile d'un projet n'est dessinée que dans la barre des projets et dans l'assistant (et désormais dans Réglages › Projets) ; ailleurs (barre de titre, Vue d'ensemble, Stats, liste des projets des réglages, projets fermés), un projet se dit par une pastille de sa couleur, qui ne change pas.
- `node design/app-redesign/contrast.mjs` : 0 paire sous le seuil, dans les deux thèmes.

# Le libellé d'un projet et la largeur de la barre (ESC-168, 2026-10-25)

Demandé par l'utilisateur le 2026-10-25 : « Pour chaque projet, permettre d'afficher un label sous l'icône », puis « permettre de redimensionner la largeur de la barre avec les tiles », et « au fur et à mesure qu'on élargit, le label passe sur la droite des tuiles, largeur max 300px ». Sa référence : une barre latérale d'une autre app, chaque icône avec son libellé dessous, coupé en « … », l'actif à la couleur d'accent. Fait dans l'app (`ProjectTile.svelte`, `ProjectRail.svelte`, `rail-width.svelte.ts`, Réglages › Projets, l'assistant) et dans l'aperçu ; les choix 65 à 73 sont **à valider**.

## Adresses

- `#labels` : la barre à 72 px avec des libellés sous les tuiles : `demo-api` actif (son nom, à la couleur du projet), `studio-web` avec un texte à soi (« Studio ») et son logo, `infra` privé, `pagination-service` coupé en « … », `mobile-app` sans libellé.
- `#labels-wide` : la barre tirée à 300 px, beaucoup de projets (elle défile) : chaque tuile a son nom à droite, les agents qui attendent au bout de la rangée, « Vue d'ensemble », « + », « Stats » et « Réglages » nommés aussi.
- `#labels-settings` : Réglages › Projets, « Libellé sous la tuile » allumé.
- Partout, le bord droit de la barre se tire (de 52 à 300 px) et un double clic le ramène à la largeur par défaut.
- Captures : `labels`, `labels-wide`, `labels-settings` et leurs jumelles `-light` à 1440×900 ; `labels-small`, `labels-wide-small-en`, `labels-settings-small-en` et leurs `-light` à 1028×779 (la barre y est retenue à 80 px, choix 71). La barre à 52 px est celle des captures `tile*` et `rail*`.

## Choix

65. **Le libellé, éteint par défaut ; allumé, le nom du projet, ou un texte à soi de 12 caractères au plus** (un emoji compte pour un) : dans Identité, après le logo, un champ (le nom en indication) et un interrupteur ; les mêmes dans l'assistant, sous les lettres et le logo. Local au poste, comme les lettres et le logo : pas dans `.escouade.yml`. Raison : 12 caractères tiennent entiers sous la tuile d'une barre de 104 px et à côté d'elle dès 160 px ; un nom de projet est souvent plus long.
66. **Sous la tuile, sur une ligne, coupé en « … »** : 10,5 px, centré, 4 px sous la tuile ; en couleur atténuée (`--muted`), l'encre du texte au survol, et celui du projet affiché dans son encre (`--p-ink`, la couleur du projet rendue lisible sur le cadre) et plus gras. Le nom entier reste dans l'infobulle. Raison : la référence ; l'encre du projet passe le contraste dans les deux thèmes, la couleur brute non.
67. **La largeur par défaut suit les libellés : 52 px sans libellé, 72 px dès qu'un projet a le sien**, tant que le bord n'a pas été tiré. Raison : 72 px laissent environ dix lettres sous une tuile, ce que montre la référence ; sans libellé, rien ne change pour qui n'en veut pas.
68. **La barre se redimensionne par son bord droit, de 52 à 300 px**, comme les autres colonnes (`Splitter`) : un trait d'accent au survol, `←`/`→` de 16 px, `Début`/`Fin` aux bouts, un double clic pour la largeur par défaut (qui suit de nouveau les libellés). Gardée dans les réglages de l'app (comme le zoom), pour toute l'app et après un redémarrage. Raison : 52 px sont les tuiles seules d'aujourd'hui, 300 px une liste aussi large que la barre latérale des agents (276 px) ; au-delà, elle prendrait la place du travail.
69. **Les tuiles gardent leurs 36 px et leurs signes, quelle que soit la largeur** : la pastille d'attente, le point du travail, le cadenas, l'anneau de l'alerte et l'anneau de l'actif ne bougent pas ; le trait de l'actif reste au bord de la barre. Raison : la tuile se reconnaît d'un coup d'œil à la même taille partout ; élargir sert les mots, pas les carrés.
70. **À partir de 160 px, le libellé passe à droite de la tuile** : une rangée par projet, tuile à 8 px du bord, nom à 10 px d'elle, 13 px, coupé en « … » au-delà de la place qui reste ; chaque projet a alors le sien, libellé allumé ou non (son texte à soi, sinon son nom), et « Vue d'ensemble », « + », « Stats », « Réglages » aussi. Le passage se fait par la largeur de la barre (`@container`) : la largeur suit la souris, seule la disposition bascule au seuil. Les agents qui attendent passent au bout de la rangée, en pastille ambre (une colonne de nombres se lit mieux qu'un coin de tuile) ; le reste de l'état reste sur la tuile. L'infobulle s'ouvre à droite de la barre, sur aucun nom. Le logo de la barre de titre reste au-dessus des tuiles. Raison : à 160 px, 12 lettres à 13 px tiennent à côté d'une tuile ; sous ce seuil, un libellé à droite n'aurait que 4 ou 5 lettres.
71. **Une fenêtre étroite dessine la barre moins large que choisi** : au plus la largeur de la fenêtre moins 948 px (la place que laisse aujourd'hui une fenêtre de 1000 px à côté de 52 px de barre), soit 80 px à 1028 px et 300 px dès 1248 px ; la largeur choisie revient avec la place. Décidé le 2026-10-25 (« Barre retenue »). Raison : à 1028 px, 300 px de barre laisseraient 104 px à la conversation, panneau ouvert ; ainsi agents, conversation et panneau gardent la place qu'ils ont aujourd'hui dans une fenêtre de 1000 px. Pour cette conversation la plus étroite (324 px, panneau ouvert, barre à 80 px dans 1028 px), la zone de saisie gagne un palier : sous 300 px de large, « Envoyer » perd son raccourci, comme Stop perd le sien sous 696 px.
72. **Les raccourcis, le glisser pour réordonner, `Alt+↑`/`Alt+↓` et le menu de la tuile ne changent pas** : le libellé fait partie de la tuile (un clic dessus choisit le projet), il n'est pas lu deux fois par un lecteur d'écran (`aria-hidden`, le nom est déjà celui de la tuile).
73. **Beaucoup de projets** : la liste défile comme avant, ses bords fondus ; un projet avec son libellé prend 17 px de plus en hauteur sous sa tuile, à droite rien de plus.

## Vérifié

- `tests/e2e/rail-labels.spec.ts` : le réglage jusqu'à la barre, le bord tiré, gardé après un redémarrage, le double clic, les flèches, à 1440 × 900 et dans la fenêtre du runner (1024 × 728) ; douze projets dans la fenêtre du runner, la barre au plus large.
- `tests/e2e/narrow-columns.spec.ts` : la barre à 300 px (dessinée à sa largeur permise), rien ne déborde de l'en-tête, de la saisie, du bandeau Plan, de la question, du panneau, à 1028, 1280 et 1440 px et de 1000 à 1960 px, en français et en anglais.

# Un projet à plusieurs dépôts git (ESC-169, 2026-10-25)

Demandé par l'utilisateur le 2026-10-25 : « pouvoir gérer plusieurs dépôts git dans un même projet (un utilisateur a son projet dans un dossier et y'a 3-4 sous-dossiers avec chacun son git) ». L'aperçu montre `~/dev/demo-api` qui n'est pas un dépôt et tient `front`, `api`, `infra` et `docs` ; `pagination-users` a un dossier à lui avec un worktree de chacun, et a changé `api` et `front`. Les choix 80 à 103 sont **validés par l’utilisateur le 2026-10-25** (« Validée telle quelle », sur les captures `repos*`) ; au refus partiel, il a confirmé « Rien n’est mergé » (choix 95 et 97), et tout se fait dans ce ticket.

## Adresses

- `#repos` : la fenêtre au travail (barre de titre et Δ, barre latérale et son pied, carte de l'agent, en-tête, panneau « Non commités » groupé par dépôt, infobulle du Δ, barre d'état).
- `#repos-history` (Historique et son menu de dépôt), `#repos-branch` (les branches des dépôts depuis la barre d'état, un dépôt en HEAD détaché), `#repos-commit` (un message par dépôt, l'un en cours de rédaction), `#repos-merge` (le merge refusé, front en conflit), `#repos-pr` (deux PR), `#repos-board` (le Kanban : validation refusée, PR livrées, notification), `#repos-wizard` (l'assistant : 4 dépôts, un sans commit), `#repos-settings` (Réglages › Projets : les dépôts, un nouveau, un retiré, `.escouade.yml`), `#repos-new-agent` (« Nouvel agent » et un agent en préparation), `#repos-editor` (l'éditeur), `#repos-run` (un lancement dans un dépôt), `#repos-overview` (la vue d'ensemble), `#repos-two` (un projet de 2 dépôts, sans worktree).
- Captures : `repos*.png` à 1440×900 et leurs jumelles `-light` ; `repos-small`, `repos-two-small`, `repos-board-small`, `repos-merge-small`, `repos-branch-small` et leurs `-light` à 1028×779. Un projet à un seul dépôt est celui de `git.png`, inchangé.

## Choix

80. **Ce qui est un projet à plusieurs dépôts** : un dossier qui n'est pas un dépôt et en tient au moins un, ou un dépôt qui en tient d'autres qui ne sont pas ses sous-modules (la racine compte alors comme un dépôt, nommé « racine », en tête). Un dossier qui est un dépôt sans autre dépôt dedans est un projet comme aujourd'hui. Raison : le cas demandé (rien à la racine) et celui d'une racine qui tient les autres.
81. **La détection** : les sous-dossiers directs et ceux d'un niveau plus bas (`front/`, `services/billing/`) ; jamais les dossiers cachés (`.git`, `.claude`…), `node_modules`, `target`, `vendor`, `dist`, `build`, le dossier des worktrees du projet, ni, si la racine est un dépôt, ce qu'elle ignore. On ne cherche pas dans un dépôt trouvé : un sous-module reste à son dépôt, il n'est pas listé. Raison : deux niveaux couvrent `apps/`, `services/`, `packages/` sans parcourir un disque entier.
82. **Quand on cherche** : à l'ajout et à l'ouverture du projet, quand la fenêtre revient au premier plan, et par « Rechercher à nouveau » (Réglages › Projets et menu des branches). Un dépôt apparu : une notification (« Nouveau dépôt git : services/billing »), les prochains agents en ont un worktree, ceux qui ont le leur n'en reçoivent pas. Un dépôt disparu est marqué « retiré » ; les worktrees qu'en ont des agents restent jusqu'au nettoyage ; « Oublier docs » le retire de la liste. Raison : rien ne bouge sous un agent qui travaille.
83. **L'assistant « Nouveau projet »** dit « 4 dépôts git : front, api, infra, docs » et ce qu'il a passé ; un dépôt sans commit est signalé (« infra n'a aucun commit », « Faire un premier commit ») ; sans aucun dépôt, l'étape reste celle d'aujourd'hui (« Initialiser un dépôt git »). Le récapitulatif ajoute « Dépôts : 4 ». (`#repos-wizard`)
84. **« Non commités » groupé par dépôt** : un en-tête par dépôt changé (son nom, sa branche, son Δ), repliable, dans l'ordre du dossier ; les dépôts sans changement tiennent en une ligne (« Sans changement : infra, docs ») ; « Cet agent | Tout le projet » et le diff ne changent pas. Raison : on lit d'un coup tout ce qu'un agent a touché, sans choisir de dépôt d'abord. (`#repos`, `#repos-two`)
85. **« Commit… » : un message par dépôt changé**, dans une seule fenêtre, chacun proposé à part (par Haiku, ou rédigé par l'agent selon « Commit ») ; un dépôt dont le commit échoue le dit, les autres sont commités. (`#repos-commit`)
86. **« Historique » d'un dépôt à la fois**, choisi par un menu en tête (touches 1 à 4), celui où l'agent a le plus de commits par défaut. Raison : un graphe ne mêle pas des dépôts sans ancêtre commun. (`#repos-history`)
87. **Barre d'état** : « main · 4 dépôts » si tous sont sur la même branche, « 3 branches · 4 dépôts » sinon, ↓ ↑ additionnés ; au clic, un menu des dépôts (branche, ↓ ↑, état : HEAD détaché, introuvable) ; choisir un dépôt ouvre son sélecteur de branche, le même qu'aujourd'hui (changer, créer, supprimer, nettoyer). Récupérer, tirer et pousser se font par dépôt depuis ce sélecteur. (`#repos-branch`)
88. **Le Δ** (tuile, barre de titre, pied de la barre latérale) est la somme des dépôts ; l'infobulle donne le détail par dépôt. (`#repos`)
89. **Un agent sans worktree** travaille à la racine du projet et voit tous les dépôts, chacun sur sa branche. (`#repos-new-agent`)
90. **Un agent avec worktree** a un dossier à lui sous les worktrees du projet (`.claude/worktrees/<nom>/`), avec un worktree de **chaque** dépôt (qui a un commit et n'est pas en HEAD détaché) à son chemin relatif, tous sous la même branche (`<préfixe><nom>`) ; les fichiers de la racine qui ne sont dans aucun dépôt y sont copiés, comme les « Fichiers copiés » (`api/.env.local`) ; ses commandes de préparation tournent dans le dossier (leur « sous-dossier » peut être un dépôt). Pas de choix des dépôts par agent ni par ticket : un worktree coûte peu, et un agent ne sait pas d'avance où il devra aller. (`#repos-new-agent`)
91. **La branche de départ** (ESC-160) vaut pour chaque dépôt : « Branche actuelle du dossier » est celle de chaque dépôt ; « Branche cible du Kanban » et une branche nommée, là où elle existe, sinon la branche du dépôt ; « Nouvel agent depuis une autre branche… » de même. La base de chaque worktree est gardée par dépôt (merge, PR, « Intégrer »).
92. **Le worktree au premier message** (ESC-156) : tous les worktrees se font une fois l'agent nommé ; sa carte dit « Préparation · 2 sur 3 dépôts ».
93. **Le nettoyage** juge chaque dépôt à part : un worktree sans travail s'en va avec sa branche, un worktree avec des changements non commités ou des commits ni mergés ni poussés reste (« pagination-users (api) : 2 commits ni mergés ni poussés ») ; le dossier de l'agent part quand il est vide. Supprimer l'agent fait de même.
94. **Un dépôt où l'agent n'a rien changé** : ni merge, ni PR, ni ligne sur la carte ; sa branche part avec son worktree.
95. **Merger** : une fenêtre, tout vérifié d'abord (dossier de base propre, branche de base présente, aucun conflit) ; **rien n'est mergé si un dépôt ne peut pas l'être**, pour que les dépôts restent d'accord entre eux ; « Merger api seul » le fait à la demande, « L'agent résout le conflit » lui envoie le conflit comme aujourd'hui. Chaque dépôt va dans sa propre base. Une erreur après la vérification (rare) laisse mergés ceux qui le sont, et la fenêtre dit lesquels. (`#repos-merge`)
96. **Les PR** : une par dépôt changé, dans une seule fenêtre (titre et description communs, une ligne par dépôt avec son dépôt distant et sa base) ; chaque description cite les autres PR ; la carte et l'en-tête montrent « api #43 · front #12 » ; le bouton dit « Ouvrir 2 PR ». (`#repos-pr`)
97. **La validation d'un ticket** fait l'action du Kanban (merge, PR, push) dans chaque dépôt changé, avec la vérification du choix 95 : si un dépôt ne passe pas, rien n'est fait, le ticket reste « À tester » avec la raison sur sa carte et « Renvoyer » à l'agent ; la carte dit quels dépôts ont changé ; en « Terminé », « PR api #41 · front #12 → main ». La notification nomme le dépôt en cause. (`#repos-board`)
98. **L'en-tête de la conversation** ajoute à la branche les dépôts changés (« · api, front »), le dernier repli avant la branche (choix 50 et 51).
99. **Les chemins se disent depuis la racine du projet**, donc avec leur dépôt : la carte d'un agent (« Edit front/src/users/… »), la vue d'ensemble, la recherche, « Ouvrir un fichier ». (`#repos-overview`)
100. **Lancement et terminaux** : une commande garde son « Sous-dossier » (un dépôt) ; lancée dans le dossier d'un agent, elle tourne dans son worktree de ce dépôt (« Dans le dossier de refacto-auth · son worktree d'api »). Un terminal s'ouvre à la racine du dossier. (`#repos-run`)
101. **L'éditeur** part de la racine : chaque dépôt est un dossier en tête avec sa branche, puis les fichiers de la racine ; « modifiées par rapport à main d'api ». (`#repos-editor`)
102. **`.escouade.yml`** est à la racine du dossier : si elle n'est pas un dépôt, il n'est versionné nulle part (Réglages › Projets le dit) ; celui d'un dépôt n'est pas lu. Raison : un seul fichier pour un seul projet ; le partager, c'est le mettre soi-même dans un dépôt. (`#repos-settings`)
103. **Les outils MCP** qui parlent de branche ou de diff (`get_agent_summary`, `list_agents`, `approve_ticket`, `resolve_conflict`) répondent par dépôt pour un tel projet, sans nouveau paramètre. **Un projet à un seul dépôt ne change en rien** : ni groupe, ni menu, ni « · 1 dépôt ».

## Vérifié sur l'aperçu

- Les états : 1 dépôt (`git.png`), 2 (`repos-two-small`), 4 (`repos`) ; en cours (un message en rédaction, un agent en préparation) ; vides (« Sans changement », « rien à merger », « aucun commit de l'agent ») ; erreurs (conflit, HEAD détaché, dépôt sans commit, dépôt retiré).
- À 1028 px, la ligne de merge du panneau se dit « Merger dans api et front… » ; la carte du Kanban passe ses boutons à la ligne quand elle porte un refus ; rien ne déborde, dans les deux thèmes.
- `node design/app-redesign/contrast.mjs` : 0 paire sous le seuil, dans les deux thèmes.

## Construit dans l'app (ESC-169)

- Fait comme dessiné : la détection et ses moments (80 à 82), l'assistant (83, sans le bouton « Faire un premier commit » d'un dépôt : la ligne dit pourquoi au survol), Réglages › Projets « Dépôts » (82, 102), le panneau groupé (84), le commit par dépôt (85), l'historique par menu (86), la barre d'état et son menu de dépôts (87), le Δ (88), les agents et leurs worktrees (89 à 92), le nettoyage et la suppression par dépôt (93, 94), le merge vérifié d'abord (95 : « Merger api seul » quand un seul dépôt est prêt), les PR (96), la validation (97), l'en-tête (98), les chemins (99), les lancements (100, sans changer les libellés du menu ▶ : la commande garde son sous-dossier), l'arbre de l'éditeur (101), le MCP (103).
- Vérifié : `tests/e2e/multi-repo.spec.ts`, à 1028 × 779, de l'ajout d'un dossier de trois dépôts à la validation d'un ticket mergé dans deux d'entre eux ; `src/components/MultiRepo.test.ts` et `src/lib/repos.test.ts` (Vitest) ; `src-tauri/src/repos_tests.rs` et `multi_repo_tests.rs` (Rust).

# Partager des tickets du Kanban avec l'équipe (ESC-190, 2026-10-26)

Demandé par l'utilisateur le 2026-10-26 : « mettre un fichier yml quand on décide de "partager" un ticket aux autres membres de l'équipe. On maintiendra le statut de chaque ticket partagé et le nom de l'utilisateur […], et sur un autre utilisateur escouade sur le même projet, on lance jamais en auto un ticket partagé mais on peut le lancer manuellement (il apparaîtra dans la colonne A Faire, avec mention de l'utilisateur qui l'a ajouté) ». Décidé avec lui le même jour : **les fichiers vivent dans le dossier du projet, l'utilisateur les commite et les pousse ; Escouade ne commite ni ne pousse jamais**. L'aperçu se place chez Bob, sur `demo-api` : il partage son DEM-7 ; son dernier pull a amené trois tickets d'Alice. Les choix 104 à 130 sont **validés par l’utilisateur le 2026-10-26** (« Validée telle quelle »).

## Adresses

- `#share` : le Kanban de Bob : sa DEM-5 partagée (la marque après la clé), DEM-12 d'Alice dans « À faire » (« Ajouté par Alice · le pilote auto ne le lance pas », « Lancer »), DEM-10 d'Alice en cours chez elle, DEM-9 d'Alice terminée ; le menu de DEM-7 ouvert sur « Partager avec l'équipe ».
- `#share-theirs` : le menu d'un ticket d'Alice (Lancer, Modifier, Masquer chez moi, Supprimer pour l'équipe…).
- `#share-ticket` : le formulaire de DEM-7, « Partager avec l'équipe » allumé, son fichier, son image partagée, son gros fichier gardé.
- `#share-name` : la fenêtre du premier lancement, le nom repris de `git config user.name`.
- `#share-settings` (Réglages › Application › Équipe : le nom, l'identifiant) et `#share-settings-project` (Réglages › Projets › Kanban partagé : le nom dans ce projet, les fichiers, « Partager chaque nouveau ticket »).
- `#share-git` : « Non commités » sur « Tout le projet » : le groupe « Tickets partagés » en tête, chaque fichier par sa clé, son titre et son changement de statut ; la notification après un partage.
- `#share-file` : le fichier d'un ticket dans l'éditeur, le format en entier.
- `#share-edge` : ce qui peut mal tourner : un fichier en conflit, un fichier illisible, Alice sur le même ticket, un ticket qu'Alice ne partage plus, un import déjà partagé.
- Captures : `share*.png` à 1440×900, `share*-small.png` à 1028×779, chacune avec sa jumelle `-light`.

## Choix

### Le fichier

104. **Un fichier par ticket, `.escouade/tickets/<id>.yml`**, dans le dossier du projet (jamais dans le worktree d'un agent). `<id>` est un identifiant stable de 8 caractères (minuscules et chiffres sans 0/o ni 1/l, tiré au hasard au partage), pas la clé : deux tickets partagés en même temps par deux personnes n'ont jamais le même fichier, et une clé qui change ne renomme rien. Raison : un fichier par ticket, c'est au pire un conflit git sur un ticket, jamais sur toute la liste ; le dossier `.escouade/` à côté de `.escouade.yml` dit à qui il est.
105. **Les champs** (`#share-file`) : `version` (1), `id`, `key`, `title`, `status` (`todo`, `in-progress`, `review`, `done`), `author` et `updated_by` (`{ name, id }`), `taken_by` (`{ name, id }` ou `null`), `created`, `updated` (ISO 8601 avec le fuseau), `after` (les **ids** des tickets partagés qu'il attend), `description`, `criteria` (une liste de textes), `images`, `files`, `external` (`{ source, key, url }` d'un ticket importé) et `outcome` (« Mergé dans main · squash » une fois fait). Deux lignes de commentaire en tête disent ce qu'est le fichier et qu'Escouade ne le commite jamais, une troisième donne le schéma à l'éditeur. **Restent sur le poste** : les boucles, le coût, l'avancement de l'agent, les critères cochés, le compte, le modèle, l'agent et sa branche. Raison : ce que l'équipe doit savoir, sans un fichier qui change à chaque boucle.
106. **Le fichier n'est réécrit qu'à un changement qui compte** : le partage, une modification du titre, de la description, des critères, des « Après » ou des pièces jointes, un changement de colonne (un agent le prend, il passe à tester, il est validé, renvoyé, remis à faire). Jamais à une boucle. Écrit d'un coup (un fichier temporaire renommé), en YAML stable (mêmes clés, même ordre, fins de ligne LF), pour un diff de quelques lignes.
107. **Les images partagées dans `.escouade/tickets/<id>/`, les fichiers joints jusqu'à 5 Mo** ; au-delà, le fichier reste sur le poste, le formulaire le dit (« connexions-prod.csv reste chez toi : au-delà de 5 Mo, un fichier joint n'est pas partagé ») et le fichier le note en commentaire. Raison : les captures font le ticket ; un export de prod n'a rien à faire dans un dépôt.
108. **Une seule source des clés, et un schéma JSON** comme `.escouade.yml` (ESC-146) : `schemas/escouade-ticket.schema.json`, publié sur le site (`/schemas/ticket-1.json`), comparé dans les tests aux clés que lit Escouade. Un fichier d'une `version` plus récente se lit pour ce qu'il a de connu, est marqué « écrit par une version plus récente d'Escouade » et n'est jamais réécrit.
109. **Un projet à plusieurs dépôts** (ESC-169) : si la racine est un dépôt, le dossier est à la racine ; sinon dans un dépôt choisi dans Réglages › Projets › Kanban partagé (« Dépôt des tickets partagés »), le premier dans l'ordre du dossier par défaut (`api/.escouade/tickets/`). Raison : un fichier qui n'est dans aucun dépôt ne se partage pas.

### Les clés

110. **Un ticket partagé garde la clé de son auteur partout** (DEM-12 d'Alice est DEM-12 chez Bob) ; **la numérotation de chacun saute les clés des fichiers partagés qu'il a reçus**, donc un nouveau ticket de Bob ne prend jamais une clé déjà partagée. Raison : on se parle du même DEM-12 en réunion, et sa branche `ticket/dem-12` reste la même.
111. **Une collision reste possible** (Alice et Bob ont chacun créé DEM-12 avant de tirer) : le ticket local garde sa clé ; celui de l'autre se dit « DEM-12 · Alice » sur la carte, `DEM-12@alice` dans MCP, et son id marche toujours. Écartés : renuméroter au partage (casse la branche et ce qu'on s'est dit), une clé par personne (`DEM-A12`, illisible), un préfixe à part pour les tickets partagés (deux numérotations à suivre).

### Le nom de l'utilisateur

112. **Demandé au premier lancement** (`#share-name`), par une petite fenêtre après l'accueil, prérempli par `git config user.name` : « Plus tard » la ferme ; le nom est alors demandé au premier partage, par la même fenêtre. Montrée aussi une fois à qui a déjà Escouade, au premier lancement de la version qui l'apporte. Un aperçu dit comment il apparaîtra chez les autres.
113. **Réglable dans Réglages › Application › Équipe** (`#share-settings`) **et par projet** dans Réglages › Projets › Kanban partagé (`#share-settings-project`, vide = le nom global). Gardé sur le poste, **jamais dans `.escouade.yml`**. Changer de nom ne réécrit pas les fichiers : chacun le reprend à son prochain changement.
114. **Un identifiant à côté du nom** (`author.id`), tiré de l'e-mail git (8 caractères d'une empreinte, l'e-mail lui-même n'est écrit nulle part), au hasard sans e-mail git : c'est lui, pas le nom, qui dit si un ticket est **à soi**. Le même sur les postes d'une même personne (même e-mail git). Raison : deux « Alice » ne se prennent pas leurs tickets, et l'autopilote de Bob ne lance jamais un ticket d'Alice même si Bob s'appelle Alice.

### Les écrans

115. **Partager** (`#share`) : « Partager avec l'équipe » dans le menu de la carte (clic droit), entre « Mettre en tête » et « Supprimer », avec sous le libellé le fichier qu'il écrit ; un interrupteur dans le formulaire du ticket (`#share-ticket`), sous les réglages, qui dit le fichier, ce qui part avec et qui le commite. Une fois partagé, le menu dit « Arrêter le partage ». La notification : « DEM-7 partagé : .escouade/tickets/q4w7c2na.yml est à commiter avec ton travail », « Voir dans Git ».
116. **La marque d'un ticket partagé par soi** : l'icône « équipe » à la couleur du projet, juste après la clé, rien d'autre ; son infobulle donne le fichier. Raison : discrète sur ses propres cartes, qui sont la majorité.
117. **Le ticket d'un autre** (`#share`) : son nom sur la carte, en haut à droite, avec une pastille à son initiale (une teinte tirée de son id, la même partout) ; dans « À faire », une ligne « Ajouté par Alice · le pilote auto ne le lance pas » et le bouton « Lancer » ; en cours chez lui, « Pris par Alice · à 14:37, dans son fichier », sans bouton ; à tester chez lui, « À tester chez Alice » (seul celui qui l'a pris le valide) ; terminé, « Fait par Alice · Mergé dans main ». Raison : on voit d'un coup ce qui n'est pas à soi, sans une carte d'une autre couleur.
118. **Son menu** (`#share-theirs`) : Lancer (« son fichier dira pris par Bob »), Modifier (pour toute l'équipe : le fichier change, l'auteur reste), Masquer chez moi (le fichier reste ; « Afficher les tickets masqués » dans le menu de la colonne), Supprimer pour l'équipe… (supprime le fichier, à commiter, après confirmation).
119. **Lancé à la main**, le ticket d'un autre se comporte comme un des siens (worktree, boucles, validation) ; son fichier dit `taken_by: Bob` et suit ses statuts. Il garde son auteur.
120. **Le panneau Git** (`#share-git`) : sur « Tout le projet », un groupe « Tickets partagés » en tête, repliable, chaque fichier par sa clé, son titre et ce qui a changé (« partagé », « → À tester », « → En cours · toi »), le chemin dans l'infobulle, puis une ligne : « Écrits par Escouade dans le dossier du projet ; elle ne les commite jamais. Commite-les avec ton travail. » « Commit… » les propose comme les autres fichiers (le message proposé les nomme : « chore(tickets): DEM-5 à tester »). Sur « Cet agent », ils ne sont pas là (ils ne sont pas dans son worktree), sauf pour un agent sans worktree.
121. **Les fichiers de tickets ne bloquent rien** : ni la vérification « dossier de base propre » d'un merge, ni une validation, ni un changement de branche (ils sont relus après). Un agent ne les écrit pas : ses consignes le disent, et ce qu'il changerait dans son worktree n'est pas lu.

### Le statut et la relecture

122. **Escouade relit `.escouade/tickets/` quand il change** (surveillance du dossier, comme `.escouade.yml`) : après un pull, un checkout, une modification à la main. Le fichier fait foi pour le statut d'un ticket d'un autre ; pour un des siens, la colonne locale l'écrit, et un fichier reçu qui en dit plus (« done » posé par un autre) est suivi.
123. **L'autopilote ne lance que ses propres tickets** (`author.id` = le sien) : un ticket d'un autre n'est jamais pris, même en tête d'« À faire » ; ses « Après » se lisent dans les fichiers (un ticket de Bob après DEM-12 d'Alice attend que son fichier dise `done`). « Lancer » sur la carte reste toujours possible.

### Les cas limites (`#share-edge`)

124. **Deux personnes sur le même ticket** : si le fichier reçu dit `taken_by` quelqu'un d'autre alors qu'un de ses agents le tient, l'agent continue et la carte le dit en ambre (« Alice y travaille aussi d'après son fichier (14:52). Ton agent continue ; ton prochain statut réécrit le fichier. »). Pas d'arrêt d'office : c'est à l'équipe de s'entendre.
125. **Un fichier en conflit git** (marqueurs `<<<<<<<`) n'est pas lu : la carte garde son dernier état lisible et propose « Garder le mien » (réécrit le fichier depuis le poste, ce qui lève le conflit) et « Prendre le sien » (le côté reçu) ; rien n'est commité. **Un fichier illisible** (YAML cassé, champ du mauvais type) devient une carte « Ticket partagé illisible », son nom de fichier et la ligne en cause, « Ouvrir le fichier » ; Escouade ne le réécrit jamais.
126. **Un ticket supprimé ou plus partagé chez l'auteur** : son fichier disparaît au pull ; le ticket quitte le Kanban des autres, **sauf chez qui l'avait pris** (un agent, une branche) : il y reste, comme un des siens, avec une ligne qui le dit. Supprimer un de ses tickets partagés demande « Supprimer aussi pour l'équipe ? Son fichier est supprimé, à commiter. »
127. **Arrêter le partage** supprime le fichier (et son dossier d'images), à commiter ; le ticket reste chez soi tel quel. Chez les autres : le choix 126.
128. **Un ticket importé** (Jira, Linear, GitHub, Trello) garde `external` dans son fichier : un import qui retrouve le même ticket externe déjà partagé n'en fait pas de doublon (« ACME-41 est déjà partagé par Alice (DEM-12) : pas de doublon importé ») ; s'il était déjà importé avant le pull et pas lancé, il est remplacé par le partagé, sinon les deux restent, l'un marqué « aussi partagé par Alice ». La synchro vers l'outil externe (commentaire, transition) n'est faite que par celui qui fait avancer le ticket, pour qu'elle ne soit pas faite deux fois.
129. **Un projet privé** (ESC-140) se partage aussi : privé dit ce que voient les autres projets d'Escouade, pas l'équipe du dépôt. Au premier partage d'un projet privé, la confirmation le rappelle (« Le fichier sera lu par qui a accès au dépôt »). Ses tickets partagés restent invisibles des agents des autres projets.

### MCP

130. **Les outils de tickets disent le partage** : `list_tickets`, `get_ticket` et `get_board` ajoutent `shared` (`{ by, by_me, taken_by, file }` ou rien) ; `create_ticket` et `update_ticket` prennent `shared: true|false` (partager, arrêter). **Un agent ne lance jamais un ticket partagé par un autre** : `start_ticket` le refuse, et `move_ticket` vers « En cours » aussi (« DEM-12 est partagé par Alice : il se lance à la main, depuis le Kanban ») ; `delete_ticket` d'un ticket d'un autre est refusé (on le masque ou le supprime pour l'équipe à la main). Une clé en collision (choix 111) se donne `DEM-12@alice`, ou par son id.

## Vérifié sur l'aperçu

- Les états : un ticket à soi partagé, ceux d'un autre dans « À faire », « En cours » et « Terminé », le menu de chacun, le formulaire, la première fenêtre, les deux réglages, le panneau Git, le fichier, et les cas qui tournent mal (conflit, illisible, deux sur le même ticket, plus partagé, import déjà partagé).
- À 1028 px, les lignes des cartes passent à la ligne au lieu de se couper, les boutons du conflit aussi ; rien ne déborde, dans les deux thèmes.

## Construit dans l'app (ESC-190)

Les choix 104 à 130 ont été validés tels quels le 2026-10-26 (réponse reçue par Escouade), et le refus strict de `start_ticket` confirmé (choix 130).

- Fait comme dessiné : le fichier et ses champs (104 à 106), les images et les fichiers jusqu'à 5 Mo (107), le schéma `schemas/escouade-ticket.schema.json` comparé dans les tests (108), les clés et `DEM-12@alice` (110, 111), le nom au premier lancement, au global et par projet, et l'identifiant tiré de l'e-mail git (112 à 114), la marque, la carte d'un autre et ses menus (115 à 119), les fichiers qui ne bloquent ni merge ni validation (121), la relecture et l'autopilote (122, 123), les cas limites (124 à 129), le MCP (130).
- Écarts : les trois lignes de commentaire du fichier sont en anglais, pour qu'un fichier ne change pas selon la langue de qui l'écrit, et pointent vers la page de doc `https://escouade.dev/en/docs/shared-board` ; le schéma est publié sous `/schemas/escouade-ticket.schema.json`, à côté de celui de `.escouade.yml`. Pour un projet de plusieurs dépôts (109), le dossier va dans le premier dépôt, sans réglage pour en choisir un autre. Le panneau Git (120) nomme chaque fichier par la clé et le titre de son ticket, sans le changement de statut (« → À tester ») ; la notification du partage n'a pas de bouton « Voir dans Git ». `move_ticket` ne fait que réordonner « À faire » : il n'a rien à refuser. `get_board` dit les réglages du Kanban, pas ses tickets : `shared` est dans `list_tickets` et `get_ticket`.
- Le formulaire (115) porte « Partager avec l’équipe » dans son pied, à gauche de « Ctrl Entrée pour ajouter », et non sous les réglages : depuis ESC-189, la description prend toute la hauteur libre et une rangée de plus faisait défiler la fenêtre ; ce que le partage écrit et qui le commite est dans l’infobulle.
- Ajouté en construisant : un ticket « En cours » ou « À tester » chez un coéquipier ne prend aucune place du Kanban et n'est jamais repris au démarrage ; un ticket à soi qu'un coéquipier a pris dit « Pris par … », sans « Valider ».
- Vérifié : `src-tauri/src/sharing_tests.rs` (format, schéma, statut, autopilote, conflit, fichier illisible, deux machines sur un dépôt), `src/lib/sharing.test.ts` et `src/components/board/Sharing.test.ts` (Vitest), `tests/e2e/shared-board.spec.ts` (Alice puis Bob sur le même dépôt, à 1024 × 728).

# Les loops d'un projet (ESC-191, 2026-10-26)

Demandé par l'utilisateur le 2026-10-26, pour la 2.1 : « analyser la loop du projet dans les paramètres du projet, pour en extraire les portes d'entrées, les différents agents, etc, montrer dans l'UI le workflow et montrer en temps réel, pour chaque agent, l'avancée, où on est, et l'historique dans cette loop ». Conception seule, avec impeccable ; l'étude est dans `docs/LOOPS.md`, les tickets de construction dans `docs/TODO-2.1.md`. L'aperçu montre `demo-api`, qui suit GSD (`/gsd:discuss-phase` → `plan-phase` → `execute-phase` → `verify-work`, un retour de Vérifier à Planifier, la phase suivante), avec trois agents dedans ; `studio-web`, qui tourne une boucle d'un prompt à la Ralph (`PROMPT.md`, `fix_plan.md`) ; `infra`, qui n'a pas de workflow. **Les choix 131 à 159 sont à valider par l'utilisateur.**

## Adresses

- `#loops` : l'onglet Workflow de `demo-api` (le rail de GSD, les trois agents, le début de l'historique de pagination-users), la carte et la conversation qui disent l'étape.
- `#loops-history` (le rail replié, tout l'historique de pagination-users et son retour en arrière), `#loops-settings` (Réglages › Projets › Workflow : la lecture, l'agent qui affine, la relecture), `#loops-settings-steps` (les workflows trouvés et les étapes corrigées), `#loops-simple` (la boucle d'un prompt de `studio-web` : ses tours, sa liste cochée, les étapes d'un tour), `#loops-empty` (`infra` : rien trouvé), `#loops-edge` (un workflow changé, une étape devinée, un agent hors du workflow, un agent Codex suivi en partie).
- Captures : `loops*.png` à 1440×900, `loops*-small.png` à 1028×779, et leurs jumelles `-light`.

## Choix

### Le mot et la place

131. **« Workflow », pas « boucle »** : l'onglet, le réglage et les outils MCP disent workflow ; « boucle » garde son sens d'aujourd'hui, le tour d'un ticket (« boucle 2/5 »), et nomme un retour en arrière dans un workflow. Raison : deux sens pour un mot se mêleraient sur la même carte ; workflow se dit pareil en anglais.
132. **Un onglet du panneau de droite, entre Historique et PR et CI**, avec le nombre d'agents qui suivent le workflow. Sous 400 px de panneau (la fenêtre de 1028 px), les onglets se resserrent et ce nombre tombe. Raison : à côté de ce que fait l'agent (ses fichiers, son historique git), sans rien prendre à la conversation. (`#loops`, `#loops-small`)

### L'analyse, dans Réglages › Projets

133. **D'abord des règles, sur la machine, sans coût** : Escouade lit les commandes, skills, sous-agents et prompts de chaque harness (la liste par harness est dans `docs/LOOPS.md`), `PROMPT.md` et sa liste (`fix_plan.md`), les fichiers d'état (`.planning/STATE.md`, `ROADMAP.md`), les consignes (`CLAUDE.md`, `AGENTS.md`, règles de Cursor) ; leur frontmatter, les fichiers qu'ils incluent par `@`, les sous-agents qu'ils appellent (`subagent_type="gsd-planner"`), les commandes qu'ils nomment, les fichiers qu'ils écrivent, les listes numérotées d'étapes, les « jusqu'à 3 fois ». Les sources lues sont montrées, avec leur nombre. (`#loops-settings`)
134. **Puis un agent, à la demande seulement** : « Affiner… » lance un agent court qui reçoit ce que les règles ont trouvé et les fichiers en cause, et rend les étapes, transitions et retours dans un schéma fixe. Le modèle, le compte, le coût estimé et la durée sont écrits avant (« Haiku 5.5 · compte Perso · ≈ 0,04 $ · 30 s ») ; par défaut le modèle et le compte des textes rédigés. Jamais lancé seul. Raison : un coût dit avant et décidé par l'utilisateur ; les règles suffisent aux workflows connus.
135. **Ce qui est un workflow** : au moins deux étapes reliées (une commande qui en nomme une autre, appelle des sous-agents ou écrit un fichier d'état qu'une autre lit), ou une boucle d'un prompt (`PROMPT.md` relancé, `/loop`). Une commande seule (`/commit`) n'en est pas un : elle est listée « pas un workflow ». Les commandes de `~/.claude` (comme GSD) comptent pour un projet qui en a la trace (`.planning/`) ou dont un agent en a lancé une. Un projet peut en avoir plusieurs : le menu en tête de l'onglet les propose. (`#loops-settings-steps`)
136. **Ce qui est extrait** : les portes d'entrée (commandes, skills, prompts, `PROMPT.md`, `/loop`), les étapes, les sous-agents de chaque étape, les transitions, les retours (leur condition, leur maximum), les fichiers d'état et ce qu'ils disent (« phase 3 sur 6 »), la condition de sortie. Les consignes sont lues pour comprendre, pas dessinées.
137. **Où c'est gardé** : ce que la lecture trouve, dans les données d'Escouade (`~/.escouade/`), jamais versionné. Les corrections de l'utilisateur aussi, par défaut ; ou, au choix, dans `.escouade.yml` (clé `workflows`) pour que l'équipe voie les mêmes étapes (ESC-146), qui passe alors devant la lecture. Ce qu'Escouade lit elle-même n'y est jamais écrit. (`#loops-settings-steps`)
138. **Quand on relit** : à l'ouverture du projet et quand la fenêtre revient au premier plan si un fichier lu a changé (comme les dépôts, choix 82), et par « Relire » ; « lu à 10:12 » dit l'âge de la lecture. L'agent qui affine, seulement à la demande.
139. **Corriger** : la liste des étapes dans les réglages : renommer, fusionner avec la suivante, retirer, réordonner, ajouter une étape oubliée (son nom et ce qui la signale : une commande, un sous-agent, un fichier écrit), garder ou retirer une étape devinée, modifier les retours ; « Ce n'est pas un workflow » en écarte un. Les corrections passent devant une nouvelle lecture ; celle qui ne correspond plus à rien est signalée. (`#loops-settings-steps`)
140. **Sans workflow** : l'onglet dit ce qui a été cherché, et que c'est très bien de travailler en conversation libre ; il propose « Décrire un workflow… » (les étapes à la main) et « Analyser avec un agent… » ; la boucle des tickets y paraît dès qu'un ticket tourne. (`#loops-empty`)

### Le workflow dans la colonne de droite

141. **Un rail vertical d'étapes**, lisible à 330 px : chaque étape a son nœud, son nom, sa porte d'entrée en mono, puis ses sous-agents (`gsd-planner ⇄ gsd-plan-checker`, « jusqu'à 3 tours ») et ses fichiers. Les retours en arrière sont des arcs dans la gouttière : plein pour un retour (Vérifier → Planifier), pointillé pour le tour suivant (phase finie → Discuter). Pas de graphe libre en deux dimensions : il ne tient pas à 330 px, et les workflows réels sont des suites avec des retours ; ce qui se fait en parallèle (les vagues de GSD) se dit en mots dans l'étape. (`#loops`)
142. **Les agents sur le rail** : une pastille par agent sur son étape, à droite (sous l'étape quand le panneau a moins de 420 px) ; celle de l'agent ouvert est verte, son étape surlignée, son nœud respire (rien ne bouge avec `prefers-reduced-motion`). Un clic sur une pastille ouvre l'agent.
143. **Replié** : une ligne par étape, ses agents à côté ; retenu par projet. Raison : lire l'historique sans perdre le rail. (`#loops-history`)
144. **Les portes d'entrée en tête**, les plus lancées d'abord, « +38 » ouvre la liste ; une porte cliquée met sa commande dans la saisie de l'agent ouvert, sans l'envoyer.
145. **La carte et la conversation** : sur la carte, « GSD · Vérifier » prend la place de « Plan 3/7 » quand l'agent suit un workflow (le plan reste dans son bandeau) ; dans la conversation, un séparateur à chaque entrée dans une étape (« GSD · Vérifier · /gsd:verify-work 3 · 10:47 »), qui mène à l'onglet. Sur la carte d'un ticket, la boucle du ticket garde sa place ; l'étape est dans l'infobulle. (`#loops`)

### Le temps réel et l'historique, pour chaque agent

146. **Les agents dans le workflow**, sous le rail : nom, étape, sous-agent qui tourne, tour ou vague (« tour 2 sur 3 », « vague 2 sur 3 », « 2ᵉ passage »), barrettes des étapes (faites, en cours), durée et coût dans le workflow. L'agent ouvert est choisi d'office ; un autre se choisit d'un clic pour lire son historique, sans changer de conversation. Celui qui attend garde son anneau ambre.
147. **L'historique d'un agent** : une bande dans le temps (une teinte d'encre par étape, l'échec en rouge, l'en cours en vert), puis une ligne par passage dans une étape : heure, ce qui s'y est fait (sous-agents, commits, fichier d'état), durée et coût. Un retour en arrière a sa ligne, avec la raison que l'étape a donnée (« 2 écarts : … ») et ce qui l'a décidé (`verify-work.md`). La phase en cours d'abord, « Tout voir » pour les précédentes. (`#loops`, `#loops-history`)
148. **Plusieurs agents dans le même workflow** : chacun sa pastille, sa ligne, son historique ; deux agents sur la même étape s'empilent ; rien ne les lie (pas de verrou d'étape). (`#loops`, `#loops-edge`)
149. **D'où viennent les faits**, sans que l'agent ait rien à déclarer : la porte tapée (un message qui commence par une commande, un prompt, un skill), les sous-agents lancés (`Task`/`Agent` et leur `subagent_type`, déjà suivis pour le bandeau Plan), les skills appelés, les fichiers d'état écrits, la boucle d'un ticket et les tours d'un `/loop`. Une étape commence au premier fait qui la signale et finit au premier fait d'une autre ; sa durée et son coût sont ceux des tours entre les deux.
150. **Ce que chaque harness permet, dit dans l'interface** : Claude Code (chat, et vue Claude Code par ses hooks), OpenCode et Copilot CLI donnent tout ; Codex, l'étape par ses prompts et les fichiers d'état, sans sous-agents ; Cursor CLI, l'étape par ses commandes et les fichiers d'état, et en vue Cursor par son transcript, à quelques secondes près. Un suivi partiel porte « Suivi partiel » sur la ligne de l'agent et dit ce qui manque. (`#loops-edge`)
151. **La boucle d'un prompt** (Ralph, `/loop`) se montre par ses tours : un anneau de la liste cochée (« 12/20 cochées »), les tours faits (rouges quand la case n'a pas été cochée), à venir jusqu'à la sortie (« plus de case vide, ou 18 tours »), puis les étapes d'un tour d'après la liste numérotée de `PROMPT.md`, avec leur retour (« rouge : retour à Faire ») ; l'historique se lit par tour. (`#loops-simple`)

### Les cas limites

152. **Un workflow mal reconnu** : une étape devinée est en pointillé, « deviné : … », jusqu'à « Garder » ou « Retirer » ; le bouton de réglages de l'onglet mène aux corrections. (`#loops-edge`, `#loops-settings-steps`)
153. **Un agent qui fait autre chose** : « Hors du workflow » quand son message ne vient d'aucune porte et que ses outils ne signalent aucune étape, avec ce qu'il fait (« a quitté Exécuter pour lancer la suite e2e à ta demande ») ; il y revient à la porte suivante ; son historique garde le passage dehors, en gris. Jamais décidé sur une seule commande. (`#loops-edge`)
154. **Le workflow change pendant qu'un agent le suit** : une bande neutre (rien n'attend) le dit ; chaque agent garde la version de son étape en cours et prend la nouvelle à l'étape suivante ; « Revoir les étapes ». (`#loops-edge`)
155. **Un projet à plusieurs dépôts** (ESC-169) : les workflows de la racine et de chaque dépôt sont lus ; celui d'un dépôt porte son nom (« api · GSD ») ; un agent suit celui du dépôt où il a lancé la porte, et ses fichiers d'état sont ceux de ce dépôt.
156. **Un projet privé** (ESC-140) : l'agent qui affine est un agent du projet (sa clôture, ses comptes réservés) ; son workflow et son historique ne vont ni aux agents des autres projets par le serveur MCP, ni aux plugins qui n'y sont pas activés. Les commandes de `~/.claude` sont lues pour reconnaître, rien n'y est écrit.

### MCP, plugins et la boucle des tickets

157. **MCP** : un outil `get_workflow` (les étapes, portes, sous-agents et retours d'un projet, et les agents sur chaque étape) ; `list_agents` et `get_agent_summary` gagnent `workflow` (nom, étape, tour, depuis), `get_agent_summary` l'historique des étapes sur demande. Lecture seule : un agent se met dans une étape par sa porte, que le chef d'escouade envoie par `send_message`. Raison : savoir où en est chaque agent sans lire sa conversation.
158. **Plugins** (`docs/PLUGINS.md`, après PLG-5) : le workflow pose ses faits sur le bus (`workflow.step_entered`, `workflow.step_left` avec durée, coût et issue, `workflow.looped_back` avec sa raison, `workflow.left`, `workflow.changed`) ; il accepte un point d'extension, le **lecteur de workflow**, pour un cadre qu'Escouade ne connaît pas : un plugin reçoit les fichiers du projet et rend le même graphe.
159. **La boucle des tickets d'Escouade** : la même vue, comme un workflow d'Escouade, « Tickets » (À faire → En cours, boucle n/5, travail puis vérification des critères → À tester → Terminé ; « Renvoyer » est son retour), présent quand le Kanban a des tickets ; le pilote auto est sa porte d'entrée. Un agent de ticket qui suit GSD est dans les deux : dans GSD avec `DEM-5` et « boucle 1/5 du ticket », dans Tickets sur En cours. Pas d'onglet à part. (`#loops`, `#loops-settings-steps`)
189. **Sous 420 px de panneau, « PR et CI » quitte la bande des onglets** (ESC-194, décidé le 2026-10-26) : le panneau de droite (340 px à côté de la disposition classique, à toute taille de fenêtre) ne loge pas « Non commités · Historique · Workflow · PR et CI » et le × du choix 185 ; l'onglet des hébergeurs, vide jusqu'à la 2.1, s'efface et revient dès que le panneau s'élargit. Le × garde sa place, les autres onglets se resserrent (choix 132). L'onglet Workflow est toujours là, dans l'aperçu aussi (`#tab-loop`), son nombre seulement quand des agents suivent un workflow. (`#git`, `#loops`, `tests/e2e/workflow.spec.ts` à 340 px)

## Vérifié sur l'aperçu

- Les états : un workflow à étapes (GSD) et une boucle d'un prompt ; un agent en cours, trois agents, deux sur la même étape ; l'historique avec un échec et un retour ; replié ; vide ; une étape devinée, un agent dehors, un suivi partiel, un workflow changé ; les réglages et les corrections.
- À 1028 px : les agents d'une étape passent sous elle, les onglets tiennent sur une ligne, l'historique passe à la ligne sans rien couper que par une ellipse ; dans les deux thèmes.
- `node design/app-redesign/contrast.mjs` : 0 paire sous le seuil, dans les deux thèmes.

# La place de marché des plugins, store.escouade.dev (ESC-202, 2026-10-26)

Demandé par l'utilisateur le 2026-10-26 : une place de marché des plugins sur store.escouade.dev, avec la 2.1 : soumission depuis un dépôt public, comptes, revue en local et badge « Vérifié par Escouade ». L'analyse est dans `docs/STORE.md`. Deux aperçus : le store lui-même, dans le monde du site (`design/store/index.html`, captures `design/store/store-*.png`, sombre et clair à 1440, 1028 et 390 px), et le store dans l'app (`#store*` ici). Décidé par l'utilisateur le même jour : le code du store vit dans ce dépôt (`store/`), déployé par Coolify à chaque push sur `main`. Les choix 160 à 181 sont **à valider**.

## Adresses

- `#store` : Réglages › Plugins › « Parcourir le store » : la recherche, les catégories, « Vérifiés seulement », la liste (installer, installé, mise à jour non vérifiée).
- `#store-plugin` : la page d'un plugin (Slack), dans l'onglet même : sa doc, ses versions (en revue, vérifiées, retirée), ses permissions, sa revue, d'où il vient.
- `#store-install` : la fenêtre d'installation de PLG-4 pour une version vérifiée (Gemini CLI) ; `#store-unverified` : la même pour une version non vérifiée (Goose).
- `#store-installed` : les plugins installés : vérifié avec une mise à jour non vérifiée, un dossier en développement, un plugin retiré par Escouade ; « N'installer que des versions vérifiées », la liste de révocation.
- Le store : `design/store/index.html#home`, `#search`, `#plugin`, `#account`, `#submit`, `#review` (et `?theme=light`).

## Choix

### Le sceau

160. **Un sceau, pas une coche** : l'étincelle du logo dans une petite tuile teal, « Vérifié par Escouade », puis la version qu'il couvre en mono (`v1.2.0`). Le teal est celui d'Escouade : le sceau dit que c'est Escouade qui répond de la version, et rien d'autre ne porte ce teal en aplat dans une liste. « Non vérifié » au trait neutre, « En revue » en pointillé : jamais de rouge pour un plugin simplement pas relu.
161. **Le sceau nomme toujours sa version** sur une ligne de liste et en tête d'une page ; dans la liste des versions, il est seul sur chaque ligne vérifiée. Raison : le badge vaut par version (`docs/STORE.md`, §4.4) ; la mise à jour non vérifiée se lit à côté, en ambre (« v1.3.0 · non vérifiée »).
162. **Le même sceau sur le store et dans l'app**, à la taille près, dans les deux thèmes.

### Le store (le site)

163. **Le monde du site** (`website/app/assets/main.css`) : fond presque noir, Inter, filets blancs translucides, teal pour l'action principale et le focus ; un thème clair en plus (le site n'en a pas), qui suit le système.
164. **L'accueil** : un titre et la recherche en tête, les catégories, « Vérifiés par Escouade », puis « Nouveaux » et « Les plus installés » côte à côte, et une bande « Publier un plugin » en trois pas. Pas de note, pas d'étoiles : les installations et le sceau suffisent (`docs/STORE.md`, §5.3).
165. **Des rangées entre filets, pas des cartes** : tuile, nom, sceau, description sur deux lignes, auteur, catégorie, ce que le plugin demande en une ligne, installations. Deux colonnes à l'accueil, une dans la recherche.
166. **Ce que demande un plugin, partout en une ligne**, du plus fort au plus faible : « Lance un programme » appuyé, « Décide des autorisations » en rouge, les hôtes réseau en mono, « Un secret », ou « Sans programme ».
167. **La recherche** : filtres à gauche (catégories, « Vérifiés seulement », « Sans programme », « Sans réseau », « Sans secret »), le tri à droite ; sur mobile, les filtres passent en puces défilantes au-dessus.
168. **La page d'un plugin** : en tête le nom, le sceau, l'auteur, les installations, la licence, « Ouvrir dans Escouade » (le bouton principal, un lien `escouade://plugin/<id>`) et « Copier l'adresse du dépôt » ; puis la doc à gauche, et à droite **ce qu'il pourra faire**, le **résumé de la revue**, **d'où il vient** (dépôt, étiquette → commit, empreinte, signature : une chaîne dont chaque maillon se vérifie) et « Signaler ». Sur mobile, les permissions passent avant la doc.
169. **Mes plugins** : une bande pour la version trouvée sur GitHub et contrôlée (« Publier la v1.3.0 », avec la permission qui grandit dite), puis le tableau (état, dernière version, installations, 30 jours, actions) ; un rejet s'ouvre sous sa ligne, avec ce qui a échoué, où, et comment corriger. À droite : les installations dans le temps, l'e-mail des avis, mes données (exporter, supprimer). La note sous le tableau dit que les installations sont sous-estimées (seules celles qui ont accepté les statistiques comptent).
170. **La soumission en quatre pas** (Dépôt, Version, Contrôles, Publier) : le dépôt et la version choisis restent en tête, l'étiquette avec son commit et l'empreinte de l'archive ; les dix contrôles en direct, chacun avec ce qu'il a lu ; un échec dit le problème et la correction (« pousse l'étiquette v0.3.1, et le store relancera les contrôles seul ») ; « Continuer » reste éteint, avec la raison.
171. **La file de revue** (l'utilisateur seul) : « À relire », « Signalements », « Noms proches », « Faites » ; les versions avec pourquoi (première version, mise à jour et son diff, nom proche), les installations et l'attente ; à droite, la version choisie : la commande à copier (`npm run plugin-review -- <id>@<version>`, ou `/review-plugin` dans Claude Code), ce qui a changé (les permissions nouvelles en ambre), le rapport quand il arrive, « Demander des changements » et « Refuser ». **Pas de bouton « Vérifier » sur le store** : la signature se fait sur le poste, le store ne fait que la recevoir.
172. **Mobile (390 px)** : l'en-tête garde la marque, la recherche et le menu ; tout passe en une colonne ; le tableau des plugins de l'auteur devient des lignes (nom, puis état dessous).

### Dans l'app

173. **Dans Réglages › Plugins**, groupe Système, avant « Serveur MCP » ; deux vues par un sélecteur en tête : « Installés » et « Parcourir le store », et à droite « Depuis un dossier… » et « Depuis un dépôt git… ». Le store s'ouvre dans l'onglet même, pas dans une fenêtre de plus.
174. **La recherche ne part pas** : « Catalogue de store.escouade.dev lu à 10:42 · la recherche reste sur ta machine » sous les catégories ; l'âge du catalogue en cache se lit là quand le store ne répond pas.
175. **Une ligne par plugin** : tuile, nom, sceau ou état, auteur et installations, description sur une ligne, ce qu'il demande ; à droite « Installer », « Installé » ou la mise à jour (« v1.3.0 · non vérifiée », « Mettre à jour »).
176. **La page d'un plugin dans l'onglet**, « ‹ Parcourir le store » pour revenir ; l'action en tête dit ce qui est installé et ce que vaut la mise à jour (« Installée : v1.2.0, vérifiée. La v1.3.0 n'est pas encore vérifiée, et demande un événement de plus. ») ; les versions avec leur sceau ; à droite les permissions, la revue, d'où il vient (la signature « vérifiée ici », par l'app) et « Signaler sur le store ».
177. **La fenêtre d'installation de PLG-4**, venue du store : la tuile, le nom et le sceau ; un bloc des faits (vient de, version → commit, empreinte « archive reçue identique », signature) ; ce qu'il ajoute ; ce qu'il pourra faire, la plus forte d'abord ; « Installer ».
178. **Une version non vérifiée** s'installe, après une bande qui dit les choses (« Personne n'a relu cette version. Elle vient du store, son archive est bien celle que l'auteur a publiée, mais rien ne dit ce que fait son programme. ») ; le bouton dit « Installer quand même », la signature « Aucune ».
179. **Les installés** : le sceau de la version installée et, à côté, la mise à jour non vérifiée ; l'origine (store, dossier en développement) ; l'interrupteur de l'app.
180. **Un plugin retiré par Escouade** : la tuile grisée, « Désactivé par Escouade » en rouge, une bande qui dit quelle version, quand et pourquoi, et que ses réglages et secrets sont encore là ; « Désinstaller ». Escouade arrête, ne supprime pas.
181. **Deux réglages en bas** : « N'installer que des versions vérifiées » (éteint d'office), et la liste de révocation (« lue à 10:42 · nº 18 », toutes les 6 heures, signée, appliquée même si le store ne répond plus).

## Vérifié sur l'aperçu

- Le store : les six pages à 1440, 1028 et 390 px, dans les deux thèmes (36 captures, `node design/store/render.mjs`), sans débordement horizontal sur mobile.
- L'app : les cinq écrans à 1440 et 1028 px, dans les deux thèmes (20 captures `store*.png`, `node design/app-redesign/render.mjs`) ; à 1028 px la fenêtre des réglages garde sa largeur, la page d'un plugin tient en deux colonnes.
- `node design/app-redesign/contrast.mjs` : 0 paire sous le seuil, dans les deux thèmes (les jetons de l'app ; le sceau et les étiquettes en tirent leurs couleurs).
- Le français seul : l'anglais suivra les libellés du site et de l'app à la construction (STORE-4, STORE-9).

# La largeur de la barre des agents, le panneau de droite qui se ferme (ESC-204, 2026-10-26)

Demandé par l'utilisateur le 2026-10-26 : « pouvoir redimensionner la sidebar de gauche, et pouvoir fermer la sidebar de droite quand elle est ouverte ». Fait dans l'app (`side-width.svelte.ts`, `Sidebar.svelte`, `SidePanel.svelte`, `RunsSection.svelte`, `settings.sideWidth`) et dans l'aperçu ; les choix 182 à 188 sont **à valider**.

## Adresses

- `#side-wide` : la barre des agents tirée à 400 px, son bord allumé, le panneau côte à côte, son × survolé avec son infobulle. À 1028 px, la barre est retenue à 304 px (choix 184).
- `#side-narrow` : la barre à sa largeur la plus étroite, 220 px.
- Partout, le bord droit de la barre des agents se tire (de 220 à 480 px), un double clic la ramène à 276 px ; le × ferme le panneau, la disposition « côte à côte » le rouvre.
- Captures : `side-wide`, `side-narrow` et leurs jumelles `-light` à 1440×900 ; `side-wide-small`, `side-narrow-small` et leurs `-light` à 1028×779.
- Au passage, le trait d'accent du bord de la barre des projets ne se voyait pas dans l'aperçu (`--accent` n'y existe pas) : il prend l'encre du projet, comme dans l'app.
- Au passage aussi, deux règles de l'aperçu des workflows (ESC-191) débordaient sur le panneau Git : la pastille `.who` des agents d'une étape habillait l'auteur des commits de l'Historique, et les onglets resserrés à moins de 420 px valaient sans l'onglet Workflow. Elles sont bornées au workflow (`.wf .who`, `#side:has(#tab-loop) .utabs`) ; `git-look.spec.ts`, qui échouait déjà sur main, repasse. Les captures du panneau refaites ici (`git`, `history`, `loops*`) montrent aussi le × ; les autres le montreront à leur prochain rendu.

## Choix

182. **La barre des agents se redimensionne par son bord droit, de 220 à 480 px**, comme la barre des projets (`Splitter`) : un trait d'accent au survol, `←`/`→` de 16 px, `Début`/`Fin` aux bouts, un double clic pour la largeur par défaut (276 px). Gardée dans les réglages de l'app (`sideWidth`, comme `railWidth`), la même pour tous les projets et pour Agents comme pour Kanban, après un redémarrage aussi. Raison : 220 px gardent « Nouvel agent », le nom d'une carte, son harness et son temps sur leur ligne ; au-delà de 480 px, la conversation perdrait plus que les cartes ne gagnent.
183. **Étroite (sous 250 px), la barre retire ce qui ne tient plus et garde les mots** : « Ctrl N » quitte « Nouvel agent » (il reste dans son infobulle), les petites barres de « Plan 3/7 » et de « boucle 1/5 » quittent les cartes, leurs mots restent. Le reste se coupe en « … » comme aujourd'hui. « Tout arrêter » ne passe plus sur deux lignes. Raison : un bouton coupé en « Nouvel ag… » se lit moins bien qu'un bouton sans son raccourci.
184. **Une fenêtre étroite dessine la barre des agents moins large que choisi** : au plus la largeur de la fenêtre, moins la barre des projets, moins 672 px (ce qu'une fenêtre de 1000 px laisse aujourd'hui au travail, à côté de 52 + 276 px), soit 300 px dans la fenêtre du runner (1024 px) et 304 px à 1028 px ; la largeur choisie revient avec la place. La barre des projets passe d'abord : élargie, elle prend sur la barre des agents, jamais sur le travail. Raison : comme la barre des projets (choix 71), la conversation et le panneau gardent la place qu'ils ont dans la plus petite fenêtre.
185. **Le panneau de droite a un × au bout de ses onglets, côte à côte aussi** : « Fermer le panneau », avec son raccourci (`Ctrl+Maj+B` à côté de la disposition classique, `Ctrl+Maj+L` côte à côte) et la façon de le rouvrir dans son infobulle. Raison : il n'avait de × qu'à côté de la disposition classique ; côte à côte, seul le bouton de disposition le fermait, loin de lui.
186. **Côte à côte, le × revient à la disposition classique** : fermer le panneau, c'est quitter « côte à côte », et c'est retenu comme la disposition (pour toute l'app, après un redémarrage). Le rouvrir, c'est la disposition « côte à côte » de l'en-tête de la conversation ou `Ctrl+Maj+L`, comme avant. Raison : côte à côte veut dire le panneau ouvert ; un troisième état (côte à côte sans panneau) serait la disposition classique.
187. **À côté de la disposition classique, le × ferme le panneau comme avant**, et « Fichiers » de l'en-tête ou `Ctrl+Maj+B` le rouvrent ; il n'est pas retenu après un redémarrage, comme aujourd'hui.
188. **Le panneau de droite garde sa largeur** (340 px à côté de la disposition classique, 44 % côte à côte) : il ne se redimensionne pas aujourd'hui, ce ticket n'y touche pas.

## Vérifié

- `tests/e2e/side-width.spec.ts` : le bord tiré jusqu'à sa borne, gardé après un redémarrage, les flèches, `Début` à 220 px sans rien de cassé ni de coupé dans la barre, le double clic, la barre des projets élargie qui prend sur celle des agents, le panneau fermé puis rouvert dans les deux dispositions, la disposition retenue après un redémarrage ; à 1024 × 728 et 1440 × 900.
- `tests/e2e/narrow-columns.spec.ts`, `shell.spec.ts`, `rail-labels.spec.ts`, `conversation-look.spec.ts`, `git-look.spec.ts`, `board-look.spec.ts`, `sidebar.spec.ts` : verts après la fusion de main (les échecs d'un premier passage, la création du projet trop lente sous la charge, repassent seuls).

# Où en est chaque agent dans son workflow : les étapes faites, l'étape en cours, « Ensuite » (ESC-244, 2026-10-27)

Demandé par l'utilisateur le 2026-10-27 (« tu me confirmes que l'onglet workflows affiche bien les prochaines étapes ? ») : l'onglet Workflow montrait les étapes et le point de chaque agent, rien des étapes faites ni de la suivante. **Les choix 190 à 195 sont à valider par le chef d'escouade.**

## Adresses

- `#loops` : pagination-users, sur Vérifier après un retour à Planifier : Discuter, Planifier et Exécuter faites, la sortie ensuite, ou retour à Planifier ; « Ensuite : … » sous chaque agent.
- `#loops-next` : refacto-auth choisi dans la liste : Discuter et Planifier faites, Exécuter en cours, Vérifier « Ensuite » ; l'infobulle de l'étape sur sa carte.
- `#loops-edge` (une étape devinée après Vérifier est « Ensuite »), `#loops-history` (replié), `#loops-simple` (le tour suivant).
- Captures : `loops.png`, `loops-next.png`, `loops-edge.png`, `loops-history.png`, `loops-simple.png` à 1440×900, leurs `-small` à 1028×779, et leurs jumelles `-light`.

## Choix

190. **Le rail dit où en est l'agent choisi dans la liste** (l'agent ouvert par défaut, choix 146), pas seulement l'agent ouvert : son étape est surlignée, ses étapes faites et sa prochaine marquées. Choisir un autre agent dans la liste change le rail avec son historique. La pastille verte reste celle de l'agent ouvert. Raison : le rail et l'historique parlent du même agent.
191. **Les étapes faites de ce passage sont atténuées et cochées** : le nom en gris, le détail et la porte un peu effacés, une coche dans le nœud ; « (fait) » pour un lecteur d'écran. Ce sont les étapes que l'historique a franchies depuis son entrée par la première étape (la phase en cours) ; un retour en arrière rouvre l'étape où il revient et celles d'après, celles d'avant restent faites. Celles d'un tour précédent ne comptent pas. Sans historique (« Tickets », un agent tout juste lu), les étapes avant la sienne. Les barrettes de sa ligne suivent la même règle.
192. **La prochaine étape est soulignée, avec « Ensuite »** : un trait vert sous son nom, son nœud cerclé de vert, une petite étiquette « Ensuite ». C'est la transition ordinaire depuis l'étape en cours, sinon l'étape d'après sur le rail (une étape devinée compte) ; en fin de cycle, la ligne de sortie porte « Ensuite ». Entre deux étapes, la suivante de la dernière, ou l'étape où un retour l'a renvoyé.
193. **Un choix dit ses deux issues** : l'étape d'un retour possible porte « ou retour », son nom souligné en pointillé, la condition dans l'infobulle. La phrase dit les deux : « Ensuite : Livrer, ou retour à Planifier si écarts » ; la condition entre dans la phrase quand elle tient en quelques mots (24 signes), sinon elle reste dans l'infobulle (« If gaps are found, run /gsd:plan-phase --gaps »). Un retour sur soi-même (« jusqu'à 3 tours ») n'est pas une issue : la ligne le dit déjà.
194. **La ligne de chaque agent dit son « Ensuite : … »**, sous son étape ; rien pour un agent hors du workflow. **Sur sa carte**, la place manque : « Ensuite : … » est dans l'infobulle de « GSD · Vérifier » ; sur la carte d'un ticket, l'infobulle de sa boucle dit l'étape puis la suite (« Ensuite : À tester » sans workflow). « Tickets » suit la même règle : En cours → À tester ; À tester → Terminé, ou retour à En cours (Renvoyer).
195. **La boucle d'un prompt** garde ses tours : le premier à venir est cerclé de vert, et « Ensuite : tour 8 » s'ajoute sous la sortie ; rien quand le dernier tour est atteint.

## Vérifié sur l'aperçu

- Les états : la sortie et un retour (`#loops`), une étape ordinaire et l'infobulle de la carte (`#loops-next`), une étape devinée ensuite (`#loops-edge`), replié (`#loops-history`), le tour suivant (`#loops-simple`), dans les deux thèmes, à 1440 et 1028 px.
- `node design/app-redesign/contrast.mjs` : 0 paire sous le seuil, dans les deux thèmes.

# Le workflow en diagramme (ESC-245, 2026-10-27)

Demandé par l'utilisateur le 2026-10-27 : « sur l'affichage, peut-être la possibilité d'afficher ça sous la forme d'un mermaid qui mettrait mieux en évidence les différentes boucles, retours, et différents choix ? ». Le rail de l'onglet Workflow (choix 141) se lit mal dès qu'un workflow a des choix et plusieurs retours. Conception seule, avec impeccable : une vue Diagramme à côté du rail, sa version agrandie, « Copier en Mermaid », et Mermaid comparé à un dessin d'Escouade. Le dessin de l'aperçu est dans `design/app-redesign/diagram.js` (mise en page en couches, la même que l'app reprendra), les rendus de Mermaid dans `mermaid/` (`node design/app-redesign/mermaid.mjs <mermaid.min.js>`, Mermaid 12.2 installé hors du dépôt). **Les choix 196 à 212 sont à valider par l'utilisateur.**

## Adresses

- `#loops-diagram` : GSD dans l'onglet, en diagramme ; trois agents, l'agent ouvert sur Vérifier après un retour à Planifier, une porte qui entre à Exécuter (`/gsd:quick`).
- `#loops-diagram-choice` : « Livraison », un workflow à deux choix (Tester → Relire si verts, Corriger si rouges ; Relire → Merger si approuvée, retour à Construire sinon), une boucle de correction de 3 tours, une porte `/hotfix` qui entre à Corriger.
- `#loops-diagram-simple` : la boucle d'un prompt (`PROMPT.md`, studio-web) ; `#loops-diagram-tickets` : « Tickets », la boucle n/5 et « Renvoyer ».
- `#loops-diagram-big` : la version agrandie (GSD en travers à 1440, en colonne élargie à 1028) ; `#loops-diagram-copied` : « Copier en Mermaid » et le code qu'il donne.
- `#loops-diagram-compare` : le même GSD et la même Livraison dessinés par Escouade et par Mermaid dans 340 px, les mesures, la recommandation, le code Mermaid.
- Captures : `loops-diagram*.png` à 1440×900 (le panneau côte à côte, ≈ 490 px), `loops-diagram*-small.png` à 1028×779 (≈ 340 px), et leurs jumelles `-light`.

## Choix

### La vue et la bascule

196. **Une vue « Diagramme » à côté de la colonne, dans le même onglet** : en tête, une bascule de deux icônes (colonne d'étapes, diagramme), avec leur nom et ce qu'elles montrent dans l'infobulle. La vue choisie est retenue pour toute l'app, après un redémarrage (réglage de l'app, pas par projet : qui lit mieux en diagramme le lit partout). La colonne reste la vue par défaut ; la liste des agents et l'historique (choix 146, 147) restent sous l'une comme sous l'autre. (`#loops-diagram`)
197. **Dessiné par Escouade, pas par Mermaid** ; Mermaid seulement comme texte, par « Copier en Mermaid ». Mesuré sur les mêmes workflows (`#loops-diagram-compare`) : Mermaid dessine GSD sur 772 px, réduit à 40 % dans la colonne (le texte à 5 px) ; il ajoute 5,3 Mo à l'app (107 morceaux) et en charge 2,3 Mo (670 ko gzip, 30 morceaux) à la première ouverture, 240 ms ; ses couleurs sont fixées à l'initialisation (à redessiner à chaque changement de thème ou de teinte du projet) ; il n'a ni point par agent ni chemin parcouru, et son `click` est refusé en strict. Le dessin d'Escouade tient en ≈ 8 ko, suit les jetons en `var()` (sombre, clair, la teinte du projet) sans redessiner, se lit à 12,5 px dans 340 px, et chaque étape est un bouton. Mermaid a tenu le nom piégé (`securityLevel: 'strict'`, `htmlLabels: false`) : rien d'exécuté, aucun `<img>`, le schéma entier.
198. **Une mise en page en couches** : le rang d'une étape est le plus long chemin en avant jusqu'à elle ; les étapes d'un même rang (les branches d'un choix) côte à côte, dans leur ordre. De haut en bas dans l'onglet ; les retours ne comptent pas pour le rang.

### Ce qu'il met en évidence

199. **Les retours** passent par une gouttière, à droite (à gauche quand l'étape qui revient a une voisine à sa droite), et entrent par le côté de l'étape où ils reviennent. Ils sont dans le rouge des échecs, plus pâle, puisqu'ils reviennent sur un échec (écarts, tests rouges, changements demandés) ; leur condition est écrite sur le trait, dans une pastille avec l'icône de retour, « 3 tours max » quand il y en a. Dans l'onglet, la pastille prend le haut de l'intervalle entre deux rangs, les conditions d'un choix le bas : jamais l'une sur l'autre. (`#loops-diagram-choice`)
200. **Une boucle sur place** (une étape qui se relance, `max` tours : Planifier ⇄ plan-checker, la boucle n/5 des tickets) est une pastille « ×3 » dans l'étape, sa condition dans l'infobulle ; pas d'anse au bord, qui ne tient pas dans 340 px. (`#loops-diagram`, `#loops-diagram-tickets`)
201. **Un choix** : un losange sous l'étape d'où partent deux transitions, ses branches côte à côte, chacune avec sa condition (« verts », « rouges »). (`#loops-diagram-choice`)
202. **Les portes d'entrée** sont en pointillé et en mono, en tête, et mènent à la première étape ; une porte qui entre plus loin (`/gsd:quick`, `/hotfix`) est juste au-dessus de son étape, et le trait qui y arrive s'écarte pour elle. Un clic met sa commande dans la saisie de l'agent ouvert, sans l'envoyer (choix 144).
203. **La sortie** est une pastille au bout (« Phase finie », « PR mergée », « Sortie »), avec sa condition sur le trait (« phase validée », « plus de case vide, ou 18 tours ») ; le tour suivant revient à la première étape en pointillé neutre, par la gouttière de gauche (choix 141 : plein pour un retour, pointillé pour le tour suivant). « Tickets » finit sur Terminé, sans pastille.
204. **Une légende d'une ligne** sous le diagramme (deux à 340 px) : retour et sa condition, choix, tour suivant, prochaine.

### Les agents et leur progression

205. **Pour l'agent ouvert** (ou celui qu'on choisit dans la liste dessous, comme dans la colonne) : les étapes faites portent une coche et un fond plus calme, celle en cours un bord vert qui respire (immobile avec `prefers-reduced-motion`), la prochaine un bord en pointillé et « prochaine » quand la place le permet ; à un choix, les deux branches sont prochaines. Les traits déjà parcourus sont plus marqués, et un retour qu'il a pris est plein et plus épais. Le même vocabulaire que la carte et la ligne de l'agent (faites, en cours, prochaine).
206. **Les autres agents** : un point par agent en haut à droite de son étape, vert pour l'agent ouvert, ambre pour celui qui attend, « +2 » au-delà de trois ; son nom au survol. Un clic sur une étape ouvre son détail dans une bulle, non dessinée dans l’aperçu (ce que dit la colonne : porte, sous-agents, fichiers, retours, et ses agents, qui s'ouvrent d'un clic) ; chaque étape se prend au clavier et dit son état (« Vérifier, en cours »).

### La version agrandie

207. **« Agrandir » couvre tout le panneau de travail** (la conversation et le panneau), pas une fenêtre à part : la barre dit le workflow et son résumé, puis le zoom, « Ajuster », « Copier en Mermaid » et × (Échap). Le zoom va de 50 à 200 % (−, +, `Ctrl` + molette, `Ctrl −` / `Ctrl +`), le fond défile et se tire à la main. Il s'ouvre à 100 %, ou à la taille qui montre tout. Raison : une fenêtre de plus serait à placer et à retrouver ; le panneau de travail suffit, et Échap rend la conversation. (`#loops-diagram-big`)
208. **Le sens suit la place** : en travers si le diagramme y tient à 85 % ou plus (GSD à 1440 px), sinon de haut en bas comme dans l'onglet, élargi jusqu'à 560 px (GSD à 1028 px). En travers, chaque étape a le nom de son agent dessous (l'agent ouvert d'abord, « +1 » pour les autres) et une carte « Agents » dit où en est chacun. (`#loops-diagram-big`, `-small`)

### Copier en Mermaid

209. **« Copier en Mermaid »** : une icône en tête de l'onglet, dans les deux vues, et un bouton nommé dans la version agrandie. Il copie le workflow sans ses agents, qui bougent (une doc ou une PR garde le workflow) : `flowchart TD`, une ligne de commentaire avec son nom, les portes en stade pointillé, chaque étape avec sa commande, les transitions et leur condition, les retours en pointillé, la sortie en double cercle et le tour suivant. Le toast dit « Code Mermaid copié : 19 lignes, sans les agents » et « Voir le code » montre ce qui a été copié. (`#loops-diagram-copied`)
210. **Le code copié est sans danger** : les noms, commandes et conditions viennent des fichiers du dépôt (`.escouade/workflows/`, un clone) ; chacun est entre guillemets, ses caractères qui fermeraient les guillemets, ouvriraient du HTML, une chaîne Markdown ou une entité sont écrits en entités Mermaid (`#quot;`, `#lt;`, `#gt;`, `#amp;`, `#35;`, `#96;`), un retour à la ligne devient une espace. Les identifiants sont les nôtres (`s1`, `in1`, `out`), jamais un nom du dépôt ; aucun `click`, lien ni style ne vient du dépôt. Essayé : `<img src=x onerror=…>"]-->x click s1 call alert(1) %%` reste un libellé.
211. **Mermaid n'entre pas dans l'app** : rien à charger à l'ouverture de la vue. Si l'on voulait un jour un aperçu du code copié rendu par Mermaid, il se chargerait à la demande, en `securityLevel: 'strict'` et `htmlLabels: false`.

### Les cas limites

212. **Un long workflow** défile dans l'onglet ; un nom trop long se coupe par une ellipse et se lit entier au survol et dans la version agrandie ; une étape devinée (choix 152) a un bord en pointillé fin et « deviné » à la place de sa commande ; un agent hors du workflow (choix 153) n'a pas de point sur le diagramme, il reste dans la liste. Plus de trois branches à un choix se resserrent sur leur rang, la version agrandie les montre en entier. La boucle d'un prompt garde ses tours sous le diagramme (choix 151). (`#loops-diagram-simple`)

## Vérifié sur l'aperçu

- Les quatre cas (GSD, un workflow à choix, la boucle d'un prompt, « Tickets »), l'agrandi en travers et en colonne, le code copié, la comparaison ; à 1440 et 1028 px, dans les deux thèmes.
- `node design/app-redesign/contrast.mjs` : 0 paire sous le seuil, dans les deux thèmes.
- `node design/app-redesign/mermaid.mjs` : les rendus de Mermaid, leur temps, et le nom piégé (rien d'exécuté, aucun `<img>`, 8 nœuds).

## Construit dans l'app (ESC-245)

Validé tel quel le 2026-10-27 (réponse reçue par Escouade) : le dessin d'Escouade, Mermaid seulement pour « Copier en Mermaid ».

- Fait comme dessiné : la bascule Colonne / Diagramme retenue pour toute l'app (196), le dessin en couches dans `src/lib/workflow-diagram.ts` (197, 198), les retours, la boucle sur place, les choix, les portes, la sortie et le tour suivant (199 à 203), la légende (204), l'agent lu, ses étapes faites, en cours et prochaines, le retour pris, les points des agents (205, 206), l'agrandi sur tout le panneau de travail, son zoom, « Ajuster », Échap, le sens qui suit la place et la carte « Agents » (207, 208), « Copier en Mermaid » dans les deux vues, échappé (209, 210). Mermaid n'est pas une dépendance (211) ; le composant du diagramme (`WorkflowDiagram.svelte`) n'est chargé qu'à la première ouverture de la vue.
- Avec ESC-244 (fusionné pendant la construction) : mes choix, numérotés 190 à 206 sur l'aperçu, deviennent 196 à 212, ESC-244 ayant pris 190 à 195. Le diagramme prend ses états au lieu de les recalculer : les étapes faites du passage (`doneSteps`) et la suivante (`nextOf`), à laquelle il ajoute les autres branches d'un choix ; la sortie est « Ensuite » en fin de cycle. « prochaine » de l'aperçu devient « Ensuite », le mot de la colonne.
- Écarts : un clic sur une étape n'ouvre pas encore de bulle de détail (206) : son nom entier et sa commande sont dans son infobulle, et ses agents s'ouvrent depuis la liste dessous ou la carte « Agents » de l'agrandi ; les étapes ne se prennent pas une à une au clavier (le diagramme se parcourt par la liste des agents et l'agrandi se zoome au clavier). Le toast de la copie n'a pas de « Voir le code » (209). « Tickets » boucle sur « En cours » jusqu'au plus grand nombre de boucles de ses tickets.
- Vérifié : `src/lib/workflow-diagram.test.ts` (GSD, un workflow à choix, la boucle d'un prompt, « Tickets » ; le code Mermaid, le nom piégé ; la progression ; la mise en page en colonne et en travers), `src/components/WorkflowPanel.test.ts` (la bascule retenue, le diagramme dans l'onglet, la copie, le nom piégé qui reste du texte, l'agrandi), `tests/e2e/workflow-diagram.spec.ts` à 1024 × 728.

# Les plugins, une vue à part entière (direction B, 2026-10-28)

Proposée le 2026-10-28 à côté du design validé le 2026-10-26 (choix 173 à 179, Réglages › Plugins), et **choisie par l'utilisateur** (« ok niquel »). Maquettes : `design/plugins/` (`b1-store.png`, `b2-plugin.png`, `b3-installes.png`, `maquettes-b.html`). Elle remplace la place des choix 173 et 176 ; le reste (le sceau 160 à 162, ce que demande un plugin 166, la fenêtre d'installation 177, la version non vérifiée 178, la recherche locale 174) tient tel quel.

## Choix

213. **Une vue à elle, pas un onglet des Réglages** : une icône dans la barre des projets, sous les statistiques, ouvre les plugins en plein écran. Raison : depuis « Tout l'externe en plugins » (ESC-203), harness, sources du Kanban et hébergeurs sont des plugins ; l'écran pèse autant que le Kanban et ne tient plus dans la fenêtre des Réglages. Réglages › Plugins n'est plus qu'un lien vers la vue.
214. **Sa barre latérale** : Store, Installés, Mises à jour (avec leur nombre, en ambre), En développement ; puis les catégories avec leur nombre (Harness, Sources du Kanban, Hébergeurs de code, Thèmes, Notifications, Autorisations, Outils, Langues).
215. **« Pour <projet> » en tête du store** : trois suggestions tirées de ce qu'Escouade voit sur le poste et dans le projet ouvert, chacune avec sa raison en une ligne (« vibe 3.0.3 est installé sur ce poste », « des clés DEM-… dans les commits de ce dépôt », « 3 agents ont attendu ta réponse plus d'une heure cette semaine »). Rien n'est envoyé ; mêmes règles de rythme que les propositions de `docs/FEATURES.md` (§2). Puis les catégories, et « Vérifiés par Escouade » et « Nouveaux » côte à côte (choix 165).
216. **La page d'un plugin en grand, à onglets** : en tête la tuile, le nom, le sceau, l'auteur, la catégorie, les installations, la licence, l'id, et l'action avec ce qu'elle vaut (choix 176) ; puis Présentation, Permissions, Versions, Projets, Réglages, Journal. À droite, toujours : ce qu'il peut faire, le résumé de la revue, d'où il vient, « Désactiver », « Désinstaller », « Signaler ».
217. **Les projets un par un** : l'activation se règle projet par projet sur la page d'un plugin (avec ce qu'il y fait : « #escouade-demo-api »), un projet privé coupé d'office et dit (« rien ne lui parvient »).
218. **Les installés : première main et tiers ensemble** : deux groupes, « Ajoutés » puis « Livrés avec Escouade » (signés avec l'app, mis à jour avec elle) ; une ligne par plugin avec sa version et son sceau, d'où il vient, **les projets où il est actif en pastilles** (un cadenas pour un projet privé), et son interrupteur. Un plugin retiré par Escouade en bande rouge en tête (choix 179) ; « N'installer que des versions vérifiées » et « Tout mettre à jour » dans l'en-tête.

### Construits avec PLG-4 (ESC-176, 2026-10-29), à valider

219. **Le logo, ou les lettres** (décision de l'utilisateur, ESC-292) : la tuile d'un plugin montre son logo partout où il a sa place (la liste, la page, la fenêtre d'installation, la mise à jour) ; sans logo, ses deux lettres sur une teinte tirée de son id, comme la tuile du store. Les plugins livrés avec Escouade sont aux lettres, jusqu'au choix d'ESC-292. Maquettes B1 à B4 de `design/plugins/maquettes-b.html`.
220. **La fenêtre d'une mise à jour** (B4) : la même que celle d'une installation, de la version installée à la nouvelle (« v1.2.0 → v1.3.0 → 9d41c07 ») ; une bande ambre dit qu'elle demande plus et que l'accord est redemandé, chaque permission en plus marquée « nouveau ». « Tout mettre à jour » ouvre la fenêtre de chacune, l'une après l'autre ; annuler l'une arrête là.
221. **Le Store avant STORE-9** : la page dit que le catalogue n'est pas encore lu dans l'app, et garde « Depuis un dossier… » et « Depuis un dépôt git… » ; un lien ouvre store.escouade.dev.
222. **Le journal sur la page** : l'état du process en tête (prêt, en marche, relancé, arrêté après trois plantages…) avec « Démarrer » ou « Relancer », un filtre par genre, « Copier », relu toutes les 2 secondes.
223. **Les permissions retirables** : un interrupteur par permission dans l'onglet Permissions, sauf « Lance un programme », qui se coupe avec le plugin ; une permission retirée reste barrée dans « Ce qu'il peut faire ».
