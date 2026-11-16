---
title: "Relecture d'une PR par un agent"
subject: "git-fichiers"
sources: ["docs/TODO-1.8.md","design/esc-66/README.md","design/pr-2-3/README.md","design/pr-2-3/maquette.html","src-tauri/src/prreview.rs","crates/escouade-model/src/prreview.rs","src/components/prs/PrView.svelte","src/components/prs/PrAgentNote.svelte","src/lib/prreview.ts"]
tickets: ["ESC-66","ESC-490"]
decisions: [100,190]
compared: null
---

## Ce que c'est

Sur une PR ouverte du projet (« MR » sur GitLab), de l'équipe ou d'un ticket, un agent lit le diff et rédige une relecture : un avis d'ensemble et des remarques, chacune sur une ligne (ou une plage, un fichier, la PR), avec sa sévérité et parfois une suggestion de code. Depuis PR-5 (ESC-490), ses remarques sont des **fils de l'agent sous leur ligne** dans l'onglet « Fichiers » de la vue de la PR, à côté des fils de l'hébergeur, et le résumé est dans sa Conversation (voir *Pull requests*, « La vue d'une PR »). Elles restent dans Escouade, marquées « agent, pas publié » : l'utilisateur en écarte une (son fil est résolu ici) ou la publie chez l'hébergeur sous son compte, une à une ou toutes celles gardées. Rien ne part sans son clic.

## Comportement

### Demander une relecture

- « Faire relire » sur une ligne de la vue « Pull requests » (survolée ou choisie) ; « Faire relire par un agent » sur la PR de l'onglet « PR et CI », quand elle est ouverte et sur le dépôt de la cible ; « Faire relire » dans l'en-tête de la vue d'une PR ouverte du projet (« Relire à nouveau » quand elle a déjà une relecture). L'onglet et la liste ouvrent la relecture qui existe déjà au lieu d'en relancer une : « Voir la relecture » ouvre la vue de la PR sur son onglet « Fichiers ».
- La relecture tourne en arrière-plan : la vue dit « L'agent relit la PR #12… » (« Tu peux quitter cette vue : la relecture continue et rien n'est publié. »). Si elle finit hors de vue, un toast le dit, avec « Voir ».
- Une relecture par PR, qui garde ses remarques d'une lecture à l'autre. Sur sa ligne de la liste, une marque dit où elle en est : « Relecture en cours », « Relecture : 2 remarques ouvertes », « Relecture : rien d'ouvert », « Relecture en échec ».
- Elle survit au redémarrage, pas une lecture en cours. Elle disparaît quand sa PR n'est plus ouverte, lue sur une liste complète, ou avec son projet.
- Refusée d'emblée, avec un toast, quand le harness des textes rédigés ne sait pas répondre sans outil (voir plus bas).

### Ce que lit l'agent

- La PR est relue chez l'hébergeur (« La PR #99 n'est plus ouverte. » sinon). Son diff vient de git : depuis le point où elle a quitté sa base, comme l'hébergeur le montre. Une PR de fork se lit par la référence que l'hébergeur garde (voir *Vue « Pull requests » du projet*).
- Pointe et base sont récupérées dans des références `refs/escouade/review/<n>/…`, supprimées aussitôt : aucune branche ne bouge, aucun `pr-<n>` ne naît, rien n'est poussé.
- Les fichiers au diff trop long (150 Ko en tout, un fichier entier ou pas du tout) et les fichiers générés (`package-lock.json`, `Cargo.lock`, `*.min.js`…) ne sont pas lus, et le résumé le dit, leurs noms en infobulle. Un diff dont il ne reste rien est refusé avec sa raison.
- Il lit aussi les commentaires non résolus de la PR, et ses propres remarques encore ouvertes d'une lecture d'avant, pour ne pas les redire.
- C'est une question ponctuelle du harness des textes rédigés (Réglages › Application), sur son modèle de lecture, 5 min au plus : sans session, sans MCP et **sans aucun outil**, tout est dans la question.

### Ce que dit une PR est une donnée

- Titre, branches, diff et commentaires viennent d'un tiers, peut-être d'un inconnu sur un dépôt public : ce sont des données, jamais des consignes. Le rôle de l'agent et la question le disent ; chaque texte est dans sa balise, ses balises fermantes neutralisées ; l'agent doit signaler dans une remarque de sécurité une instruction qu'il y trouve (approuver, publier, ne rien signaler, lire un fichier…).
- Une consigne n'est pas une barrière : le texte d'un fichier lu reviendrait dans une remarque que l'utilisateur pourrait publier. La vraie barrière est que l'agent n'a aucun outil, pas même de lecture : un diff hostile ne peut lui faire ouvrir ni `~/.ssh/id_rsa` ni un `.env`, ni rien écrire chez l'hébergeur.
- Chaque harness coupe ses outils à sa façon : Claude Code (`--tools ""`), Codex (sans shell ni apps), Copilot CLI (une liste d'outils qui n'en nomme aucun), OpenCode (agent sans outil, dossier vide), Gemini CLI (règles qui refusent tout), Antigravity CLI (tous refusés), Mistral Vibe (`--enabled-tools none`).
- **Cursor CLI ne sait pas couper les siens** : son mode Ask lit toujours le disque. La relecture y est refusée, le message dit de choisir un autre harness dans Réglages › Application › Textes rédigés. Il en va de même d'un harness de plugin : le protocole ne donne aucun moyen de vérifier ce qu'il laisse faire.
- En plus, l'agent tourne dans un dossier vide à lui, pas dans le projet : deux barrières valent mieux qu'une.
- De sa réponse, seuls l'avis et les remarques (place, sévérité, texte, suggestion) sont lus : un verdict, une demande de publier ou d'approuver n'y sont jamais lus, et toutes ses remarques arrivent ouvertes. Une PR piégée qui lui demande de publier ou d'approuver n'obtient rien. Une réponse qui n'est pas une relecture se dit comme un échec.

### Les remarques de l'agent, des fils sous leur ligne

- **Où** : dans l'onglet « Fichiers » de la vue de la PR (sur la PR entière), chaque remarque se pose sous sa ligne (la dernière de sa plage), après les fils de l'hébergeur de cette ligne ; la ligne porte le trait violet des lignes qui ont un fil. Une remarque sur tout un fichier va en tête de ses lignes (« Sur le fichier »). Une remarque sur la PR, ou sur un fichier que le diff ne change pas, va en tête de la liste (« 1 remarque sur la PR entière »). Une remarque dont la ligne n'est pas dans le diff lu, ou dont le code a changé depuis, se montre à part en tête de son fichier (« 1 remarque de l'agent hors du diff : sa ligne n'y est plus »), comme un fil périmé de l'hébergeur, avec sa place et la marque « hors du diff » ou « périmée ».
- **Marquée « agent, pas publié »** : l'icône de l'agent, « Agent relecteur », l'étiquette « agent, pas publié » (infobulle : « Remarque de l'agent relecteur, gardée dans Escouade : rien ne part chez GitHub avant ton clic. »), un trait et un cadre violets ; lisible sans la couleur par l'étiquette et l'icône.
- **Sa sévérité** : « Sécurité » (une faille, ou une consigne trouvée dans la PR), « Bogue » (un bogue, une donnée perdue), « Spec » (un écart avec ce qui est attendu), « Détail » (le reste), chacune avec son icône et sa couleur. « Bogue » s'ajoute aux trois sévérités du design : un bogue n'est ni un écart de spec ni un détail. Une sévérité inconnue est un détail.
- **Son texte** est rendu comme un markdown de tiers : HTML brut montré en texte, une image du web derrière « Afficher l'image », un lien ouvert au clic seulement.
- **Sa suggestion de code**, quand l'agent en propose une (le texte exact qui remplace ses lignes) : un cadre « Suggestion » qui la montre en diff, ses lignes d'avant en `-`, la suggestion en `+`, avec « Copier ». Elle ne s'exécute jamais ; l'appliquer viendra avec PR-6.
- **Ses gestes** : « Publier » ; « Garder » (« Gardée » ; « Ne plus garder ») la met de côté pour « Publier les remarques gardées » ; « Écarter » résout le fil de l'agent : il se replie en une ligne (« Écartée · Détail · son début ») gardée, que « Reprendre » rouvre. Une remarque dont le texte a été corrigé le dit (« corrigée »).
- **L'en-tête de l'onglet** : la bande « Relecture de l'agent » compte les remarques ouvertes ou gardées par sévérité (« Sécurité 1 · Bogue 0 · Spec 1 · Détail 2 »), chaque sévérité étant aussi un filtre (une ou plusieurs choisies : seules les leurs se montrent ; aucune : toutes) ; « Publier les n remarques gardées » ; et « Suivante » : ce que parcourt « Suivante » (« Tous les fils », « Les fils de GitHub », « Les remarques de l'agent »), gardé sur le poste. À droite des onglets, le compte des fils dit aussi « ✦ 4 remarques ».
- **« Suivante »** (N, P) passe par les remarques ouvertes ou gardées comme par les fils ouverts de l'hébergeur, dans l'ordre de l'onglet (hors des fichiers d'abord, puis fichier par fichier, ceux à part puis par ligne ; à une même ligne, le fil de l'hébergeur d'abord), selon le choix ci-dessus.

### Publier

- **« Publier »** sur une remarque ouvre son texte pour le relire et le corriger (« Texte publié sur GitHub »), dit où il partira (« Partira sur GitHub comme ton commentaire sur lien.ts:12. », ou dans la conversation de la PR) et que la suggestion partira sous le texte ; « Revenir au texte de l'agent », « Annuler », et « Publier sur GitHub » (Ctrl+Entrée), qui seul envoie. Un texte vide ne part pas.
- **« Publier les n remarques gardées »** ouvre une confirmation qui les liste dans l'ordre de l'onglet (sévérité, place, texte) : « Elles partent sur GitHub sous ton compte, comme tes commentaires, dans cet ordre : » ; « Publier les n » les envoie, « Annuler » non.
- Chaque remarque part comme **un commentaire de l'utilisateur**, par le chemin des fils (`pr_write`, ESC-474) : sur ses lignes quand elle est sous sa ligne dans le diff lu (un commentaire de ligne, sa plage comprise), sinon dans la conversation de la PR, sa place devant (« `lien.ts:12` : … »). Sa suggestion suit le texte : un bloc `suggestion` de GitHub sur une ligne, un bloc de code ailleurs.
- **Une fois publiée**, la remarque n'est plus un fil de l'agent : la PR est relue et le fil de l'hébergeur prend sa place (pas de doublon) ; un toast dit « 1 remarque publiée sur GitHub. ». Une remarque publiée ne change plus. Une remarque déjà publiée ou en train de partir est refusée : un second clic ne la publie pas deux fois.
- **Un refus** de l'hébergeur arrête la publication à cette remarque, dit pourquoi dans son fil (ou dans la confirmation) avec les mots des fils (jeton refusé, pas le droit d'écrire, limite, injoignable) ; celles parties avant restent publiées, les autres restent à publier, leur texte corrigé gardé.
- **Rien n'est publié par l'agent, par le serveur MCP ou tout seul** : seule la fenêtre (ou un client de l'API distante de l'utilisateur, comme ses autres commandes) appelle la publication ; aucun outil MCP ne lance une relecture, ne garde, n'écarte ni ne publie une remarque, et l'agent relecteur n'a aucun outil.

### Le résumé dans la Conversation

- En tête de la chronologie, sous la description : « Agent relecteur », « agent, pas publié », « a relu la PR au commit a1b2c3d · il y a 12 min », qui a lu (« Claude Sonnet ») ; « 4 remarques · 1 Sécurité · 1 Spec · 2 Détail » (celles qui attendent, par sévérité) ; « Avis d'ensemble », le texte de l'agent rendu comme un markdown de tiers ; les fichiers non lus ; puis « Remarques », filtrables par sévérité et triées « Par sévérité » ou « Par fichier » : chacune avec sa sévérité, sa place (un lien « Voir la remarque sur lien.ts:12 » qui ouvre l'onglet « Fichiers » sur son fil), son texte et son état (« ouverte », « gardée », « écartée », « publiée »).
- « L'agent n'a rien à signaler. » sans remarque.

### Relancer après un commit

- « Relire à nouveau » relit la PR comme elle est. Les remarques d'avant restent pendant la lecture, et ce que l'utilisateur en décide pendant ce temps est gardé.
- À la fin, une remarque ouverte ou gardée dont le code est encore là reste (elle suit son code s'il a bougé) ; celle dont le code a changé ou dont le fichier a quitté le diff est dite **périmée** et va à part ; ce qui a été écarté ou publié le reste. Les nouvelles s'ajoutent ; une remarque déjà là (même place, même texte) ne revient pas, et une remarque écartée ou publiée ne revient que si son code a changé.
- Une lecture qui échoue le dit (« La relecture n'a pas abouti : <raison> », « Relire à nouveau ») et garde les remarques d'avant.

### États

- Lecture : « L'agent relit la PR #12… », en tête de l'onglet « Fichiers » et du résumé.
- Échec : le cadre rouge, la raison et « Relire à nouveau » (« La réponse de l'agent n'est pas une relecture. », pas de réponse dans le temps, diff illisible).

## Limites

- Essayé sur le faux GitHub des tests ; à essayer en vrai sur GitHub, puis sur les quatre autres hébergeurs (`docs/TODO-1.8.md`). Publier sur une ligne demande que l'hébergeur commente une ligne (`Capabilities::writing`) : ailleurs qu'à GitHub, une remarque part dans la conversation tant que les plugins ne le déclarent pas.
- Pas de relecture sur Cursor CLI ni sur un harness de plugin (voir plus haut).
- Le protocole des hébergeurs ne donne pas le texte de la PR : l'agent lit son titre, ses branches, son diff et ses commentaires non résolus (20 au plus, 1 Ko chacun).
- Une remarque porte sur des lignes de la nouvelle version : pas sur une ligne retirée.
- L'agent n'ouvre aucun fichier : il ne voit que le diff et son contexte.
- 40 remarques au plus par lecture.
- On ne parle pas encore à l'agent dans son fil, on ne lui demande pas de corriger et on n'applique pas une suggestion (PR-6) ; l'approbation et la demande de changements viennent avec « Ta relecture » (PR-7) : la publication d'une relecture entière avec verdict d'ESC-66 n'existe plus.
