# La boîte « À toi »

La boîte « À toi » rassemble ce que tes agents attendent de toi sans s’arrêter de travailler : un choix entre des rendus, un feu vert, une réponse, un essai que toi seul peux faire. Cette page dit où la trouver, comment répondre au clavier, comparer des rendus, lire un PDF joint et revenir sur une réponse.

![La boîte « À toi » : la file à gauche, un choix entre trois rendus, son PDF lu à côté du texte et la réponse en bas](/images/inbox.jpg)

## Où la trouver

La boîte existe avec le serveur MCP d’Escouade : allume « Les agents peuvent piloter Escouade » dans Réglages › Agents › Piloter Escouade (voir [le serveur MCP](mcp)). Ses agents déposent leurs demandes par l’outil `ask_user`, puis continuent leur travail.

- **Sa tuile** est dans la barre des projets, sous « Vue d’ensemble », avec le compte de tout ce qui t’attend, tous projets confondus. La pastille est ambre quand un agent s’est arrêté sur une question ou une autorisation.
- **`Ctrl+Maj+J`** l’ouvre et la referme ; `Échap` aussi la referme. `Ctrl+J` va toujours au prochain agent arrêté.
- **Serveur éteint**, rien de la boîte ne se montre : ni tuile, ni compteur, ni raccourci. Éteindre le serveur avec des demandes ouvertes te le demande d’abord : elles sont gardées, sans que tu les voies ni que leurs agents puissent les lire, jusqu’à ce que tu le rallumes, et elles se périment à leur date. Rallumé, la tuile revient avec elles, sans redémarrer.

Le compte de la tuile et celui de la barre d’état disent la même chose : « N en attente » compte les agents arrêtés, et la tuile compte ces mêmes agents plus les demandes ouvertes. Une demande remise à plus tard ne compte qu’à son retour.

## La file

À gauche, la file : « Tous » ou « Ce projet », la jauge (une case par demande, verte une fois traitée, ambre pour un agent arrêté) et « 3 traitées aujourd’hui · il en reste 9 ». Dessous, les groupes :

1. **« Tes agents se sont arrêtés »** : les agents qui attendent ta réponse dans leur conversation (« Ouvrir la conversation » y mène).
2. **« Aujourd’hui »** : les demandes urgentes, revenues de « Plus tard », attendues par un ticket ou déposées aujourd’hui.
3. **« Plus tôt »**, puis **« Plus tard »** (replié), avec l’heure du retour de chacune.
4. **« Répondu récemment »** : les réponses des 7 derniers jours (50 au plus), avec qui a répondu, quand, et si l’agent les a lues.

Chaque ligne dit qui demande (le projet et l’agent), quoi, depuis quand, ses fichiers, et montre les rendus d’un choix sous son titre. Un point marque une demande que tu n’as pas encore ouverte.

## Répondre

À droite, une demande à la fois, en grand : qui la pose, son texte, ses fichiers, la recommandation de l’agent et ce que ta réponse débloque. La réponse est en bas.

| Sorte | Ce que tu fais |
|---|---|
| Un choix | Choisis une option (ses rendus sont sur leurs planches), avec un mot si tu veux, puis « Choisir B ». La recommandation est mise en avant, jamais choisie à ta place : la réponse part de l’option recommandée, que tu changes d’un clic ou d’une touche. |
| Oui ou non | « Oui » ou « Non », avec les mots de l’agent (« Oui, publie »). Un feu vert est à toi seul : ton « oui » revient à l’agent, qui agit ensuite avec ses propres garde-fous. |
| Une réponse | Écris (ou dicte) ta phrase ; le champ part du texte que l’agent propose. `Ctrl+Entrée` l’envoie. |
| Une tâche | Coche ses étapes, joins une capture si quelque chose cloche, puis « C’est fait » ou « Je ne le ferai pas ». |

Le micro dicte dans le champ, comme celui de la conversation (voir [la dictée](settings-voice)). Le trombone joint un fichier à ta réponse.

### Au clavier

| Touche | Ce qu’elle fait |
|---|---|
| `J`, `K` | La demande suivante, la précédente (« 3 sur 9 ») |
| `1` à `6` | Une option du choix |
| `Entrée` | Le geste principal : « Choisir B », « Oui », « C’est fait » |
| `L` | « Plus tard » |
| `↑`, `↓` | D’une ligne de la file à l’autre |

Une réponse envoyée ouvre la suivante.

### Comparer en grand

« Comparer en grand » (ou l’icône d’une planche) montre les options côte à côte sur toute la fenêtre, la recommandation en sceau sur sa planche. Quand l’agent a joint un rendu sombre et un rendu clair, « Sombre · Clair · Les deux » choisit lesquels montrer. `1`, `2`, `3` choisissent, `←` et `→` passent d’une option à l’autre, `Espace` montre la planche seule dans la visionneuse, `Échap` ferme. Le champ « Un mot pour l’agent » et « Choisir B » restent en bas : tu réponds sans revenir.

![La comparaison en grand : trois directions, en sombre et en clair, le sceau de la recommandation sur la deuxième](/images/inbox-compare.jpg)

## Les fichiers joints

Un fichier d’agent est une donnée : Escouade lit sa copie, jamais un chemin donné par l’agent, et n’exécute rien.

- **Une image** (PNG, JPEG, GIF, WebP, AVIF) : sa vignette, puis la visionneuse (galerie de la demande, zoom, « Copier l’image », « Enregistrer sous… »).
- **Un PDF** : « Lire » l’ouvre à côté du texte, page par page (« 3 sur 12 », une page tapée y mène), avec ses vignettes, le zoom, la recherche et « Plein écran ». Il est dessiné par pdf.js, livré avec l’app : aucun script du PDF ne s’exécute, rien ne se charge du web, ses liens ne sont pas actifs. Un PDF protégé par un mot de passe ne s’affiche pas.
- **Un Markdown** est dessiné sous la demande, comme dans la conversation : rien de distant sans clic.
- **Les autres** (un document Word, un tableur, une archive…) : leur nom, leur taille, « Ouvrir » (dans l’app du système) et « Enregistrer sous… ». Un programme, un script ou une page ne s’ouvre jamais : « Afficher dans le dossier » à la place.

Les fichiers d’une demande close sont gardés 30 jours, puis effacés (Réglages › Agents › « Garder les fichiers des demandes closes » : 7, 30 ou 90 jours). Réglages › Avancé › Données locales dit ce qu’ils pèsent et vide ceux des demandes closes.

## Plus tard, revenir, laisser trancher

- **« Plus tard »** (`L`) : « Dans 1 heure », « Ce soir, 18:00 », « Demain, 9:00 », « Lundi, 9:00 » ou « Choisir… ». La demande quitte la file et revient à l’heure dite, marquée « revenue de Plus tard ».
- **Revenir sur ta réponse** : elle part 10 secondes après ton geste (« part dans 10 s · Annuler »). Ensuite, tant que l’agent ne l’a pas lue, « Revenir sur ma réponse » rouvre la demande ; lue, « Changer d’avis… » t’emmène dans sa conversation avec un message prêt à compléter.
- **Laisser le chef d’escouade trancher** : quand le projet a un chef et que « Les agents peuvent répondre aux demandes des autres agents » le permet, « Laisser chef-atlas trancher » lui confie cette demande, et « La reprendre » te la rend. Un feu vert et une tâche restent à toi.
- **Aller à l’agent** ou **au ticket** dont parle la demande, depuis la barre du haut.

## La trace

Une demande close garde sa trace : ta réponse, qui l’a donnée (toi, ou le chef d’escouade délégué), quand et comment (au clic, au clavier, à la voix), si l’agent l’a lue, et son histoire (déposée, reportée, répondue, lue…). Une réponse qui tranche une décision du produit le dit : l’agent la propose dans Docs › Décisions, où tu l’acceptes.
