---
title: "Le contrat des harness en process"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.5 et §4.6","docs/PLUGINS.md §8.29 (ESC-352)","docs/TODO-2.2.md (INT-9)","crates/escouade-plugin-protocol/src/engine.rs","src-tauri/src/plugins/harness.rs","src-tauri/src/plugins/shared.rs","src-tauri/src/plugins/view.rs"]
tickets: ["ESC-221","ESC-223","ESC-225","ESC-224","ESC-283","ESC-226","ESC-352","ESC-362"]
decisions: [34,68,101]
compared: null
---

# Le contrat des harness en process

Le harness d'un plugin (`contributes.harnesses`) est servi par le process du plugin, comme tout plugin, de première main ou tiers. Il n'y en a encore aucun dans une release : le premier est le harness « Minimal » du plugin de démonstration, dans les builds de développement et les tests. Les harness d'aujourd'hui y passent un par un (INT-10).

## Ce qui reste au cœur

- **Le lancement.** Le plugin décrit le lancement (arguments, variables, fichiers, premières lignes) et ne lance rien. Le cœur lance le programme qu'il a trouvé lui-même, dans son Job object, sous WSL au besoin. Le harness survit à un plantage de son plugin.
- **Ce que le cœur vérifie avant de lancer**, quel que soit le plugin, sinon l'agent ne démarre pas et la raison est dite :
  - le jeton du serveur d'Escouade n'est jamais dans les arguments (« … met le jeton d'Escouade dans les arguments de son harness… ») ;
  - pour un projet privé, la clôture est appliquée et chaque chemin refusé figure dans le lancement (« … n'applique pas la clôture des projets privés… ») ;
  - une discussion de l'onglet Docs est tenue en lecture seule (« … ne tient pas son harness en lecture seule : la discussion ne démarre pas. ») ;
  - les fichiers ne sont écrits que dans le dossier de l'agent.
- **L'état de l'agent.** Le plugin traduit ce que dit le harness en éléments (texte, raisonnement, outil, demande, question, fin de tour, note), et le cœur les applique à la conversation. Ce que le plugin écrit au harness passe par le cœur.
- **Le terminal.** Les octets du PTY et les frappes ne passent jamais par le plugin : il décrit le lancement de la vue et lit ses hooks, et il met la réponse du cœur dans la forme de son harness.
- **Ce qui est déclaré** se lit sans lancer le plugin : le nom, le programme, les versions, les capacités (lecture seule, comptes par clé), les modèles, le dossier où le harness garde la mémoire d'un projet (proposée une fois à la mémoire partagée).

## Un plugin qui plante

- En plein tour : le plugin redémarre (trois fois en cinq minutes au plus) et le cœur lui redonne l'agent et ce qu'il n'avait pas traduit. Rien n'est montré deux fois. La conversation dit « Le plugin de <harness> a redémarré. ». Un outil en cours au moment du plantage est fermé à la fin du tour.
- Après le dernier plantage permis, l'agent s'arrête et la conversation dit « Le plugin de <harness> s'est arrêté après N plantages : l'agent s'arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. ».

## Les seuils

Mesuré sur le harness « Minimal », contre les chiffres d'INT-0 : au plus 5 ms de plus par ligne au rythme réel (0,66 ms au 99e centile), au moins 2 200 lignes par seconde (41 119), au plus 30 Mo par process de plugin (14 Mo). Chaque harness qui passe en process refait ces mesures.

## Un harness partagé par des agents

Un harness peut servir plusieurs agents d'un seul process (`opencode serve`, voir *OpenCode en plugin*). Le contrat le dit pour tous (`Launch::shared`) :

- **Le lancement reste au cœur.** Le plugin décrit le process et le marque partagé ; le cœur le lance une fois pour les agents du même plugin, du même programme et des mêmes variables, avec un agent par dossier si le plugin le demande, et l'arrête avec son dernier agent. Il survit donc au plantage de son plugin, comme tout harness.
- **Ce que le plugin lit seul**, le cœur ne le voit pas passer : les éléments arrivent sans être demandés (`agent/items`), dans leur ordre. Ce qu'un tel harness ne dit pas sur une ligne, la clôture et la lecture seule comprises, le plugin le tient ; seul un plugin livré avec l'app est cru sur la clôture.
- **Un plantage ou un arrêt se voit sans appel en cours** : chaque agent regarde l'état de son plugin. Les messages sont les mêmes que plus haut ; coupé pendant qu'un agent tourne, l'agent s'arrête en le disant, sa session gardée.

Une fin de tour et une question alertent comme pour les autres harness, quel que soit le plugin : c'est le cœur qui applique les éléments.

## Ce qu'un harness peut déclarer de plus

Depuis Copilot CLI (INT-10.3), pour tout harness, dans son manifeste :

- **`framing: "contentLength"`** : le cœur lit et écrit ses messages avec des en-têtes `Content-Length`, pas par lignes.
- **`closeLine`** : une ligne écrite juste avant de fermer son entrée (ce qui lui fait sauver sa session).
- **`opens`** : son plugin ouvre la session par le protocole du harness. L'agent est prêt quand le plugin le dit, avec les modèles du compte pour le menu ; le plugin peut aussi arrêter le harness en disant pourquoi, et cette raison finit l'agent.

Un plugin peut aussi faire demander au harness son quota, commander un titre ou une compaction par l'entrée du harness, écrire une carte entière (permission ou question, avec la portée de « Toujours », un formulaire) et dire le quota. Ce sont de nouveaux éléments : un plugin écrit avant reste tel quel.

Le fichier privé du serveur d'Escouade, quand le moteur en fait un, est donné au lancement : le jeton reste hors des arguments, et le cœur le vérifie toujours.
## Ce que Codex a ajouté au contrat (INT-10.4)

Codex se sert de ce qu'OpenCode et Copilot CLI y ont mis (voir plus haut), et y ajoute, pour tous :

- une compaction demandée au harness peut se poursuivre en un tour de celui-ci, qui la finit ;
- un plugin peut dire la dernière entrée d'où une copie repart quand aucun tour ne la porte, ce que fait l'agent quand l'élément suivant ne le dit pas, et la durée d'un tour selon le harness ;
- l'ouverture d'une session par le plugin a une limite de trois minutes : au-delà, l'agent le dit au lieu de rester bloqué ;
- **la clôture d'un projet privé** se compare au lancement quelle que soit la forme des barres d'un chemin Windows : Codex lit `C:/code/secret`, pas `C:\code\secret`.

## Ce qu'Antigravity CLI a ajouté au contrat (INT-10.8)

Un agent d'`agy` a plusieurs programmes à la suite : il ne prend son modèle, son effort et son mode qu'au lancement, et n'a pas de message qui arrête un tour. Un harness peut le déclarer (`relaunches`), et pour tous :

- **un plugin peut demander une relance** : le cœur arrête le programme et en démarre un autre depuis une nouvelle description, **vérifiée comme la première** (jeton, clôture, lecture seule, fichiers) ; ce qui s'écrit ensuite va au nouveau. Il peut aussi arrêter le programme et garder l'agent, jusqu'à la relance suivante ;
- **un programme qui finit de lui-même est dit au plugin**, derrière ce qu'il a écrit : le plugin en relance un autre, dit pourquoi l'agent s'arrête, ou laisse l'agent finir ;
- **un fichier s'écrit pendant que l'agent tourne**, dans son dossier seul et avec le même contrôle qu'au lancement (des règles lues à chaud) ;
- **toute réponse peut porter des éléments**, appliqués dans leur ordre : un message qui relance est une relance, puis une ligne écrite ;
- le manifeste peut écarter des lignes de la sortie d'erreur du harness : ses journaux nomment le compte ;
- un plugin qui redémarre retrouve le dossier de l'agent, ses dossiers lus, sa clôture, sa lecture seule et ses consignes, pour tout harness ;
- **la clôture d'un projet privé** se lit aussi dans un fichier JSON, où les barres d'un chemin Windows sont doublées, quelle que soit la barre finale.

## Ce que Claude Code a ajouté au contrat (INT-10.5)

Pour tous :

- **une réponse du harness attendue** : un appel peut dire ce qu'il écrit au harness et sous quel nom la réponse reviendra ; le cœur attend cette réponse sans retenir les appels suivants. C'est ce qui garde les erreurs d'un changement de modèle, d'effort ou de mode, d'une interruption et d'un titre ;
- **les tâches de fond et le lien au site de l'éditeur** ont leurs appels (arrêter une tâche, lire sa sortie, lier la session) ; un plugin qui ne les a pas reste refusé ;
- **la clôture d'un projet privé par un fichier** : le cœur écrit le fichier de réglages, et un lancement qui le nomme tient la clôture sans en épeler les chemins. Cela ne vaut que pour le fichier que le cœur a écrit lui-même ;
- une carte peut porter ses règles de « Toujours », son titre et son défaut à non, et rendre ces règles dans la réponse ;
- la fin d'un tour peut être comptée par le harness, modèle par modèle ; l'interruption reste décidée par le cœur ;
- un agent attend la fin de son lancement avant son premier appel : un message envoyé tout de suite n'est plus perdu ;
- **un plugin relancé sans plantage** (la langue ou le réseau changés relancent les plugins livrés) retrouve l'agent avant son prochain appel : une réponse à une carte n'est plus perdue ; coupé pendant un tour, l'agent s'arrête en le disant, sa session gardée ;
- chaque carte et chaque fin de tour d'un lot s'applique à part : aucune notification ni fin de tour n'en remplace une autre ;
- une ligne qui n'est pas du protocole, ou qui n'est pas de l'UTF-8, est gardée pour ce que dit la fin du harness ; les 30 dernières comptent.

## La vue terminal passe par le module (ESC-352)

Le cœur ne lie plus aucune crate de plugin : `src-tauri/Cargo.toml` ne dépend d'aucun dossier de `plugins/`. Une correction publiée au store corrige donc aussi la vue, sans attendre une version de l'app.

- **Le partage.** Le cœur garde le PTY, le fichier que l'interface du harness écrit (transcript, rollout, journal), ses hooks et l'état de l'agent. Le plugin dit ce que les lignes veulent dire, par `view/translate` : le cœur lui passe une ligne à la fois avec ce qu'il sait de l'agent (dossier, mode, modèle, session, tour en cours, interruption), le plugin rend des éléments, ceux d'`agent/frames` et ceux d'une vue (le message tapé, l'attente de l'utilisateur, la fin de tour, une ligne d'outil sans résultat, le lien à claude.ai).
- **Ce que seul le cœur sait** part comme une note, une ligne `{"escouade": …}` : la fin d'un tour, avec le coût que le cœur calcule pour Claude Code ; un hook qui dit ce que le fichier tait (Gemini CLI n'a que ses hooks).
- **Une instance à part.** Les vues d'un plugin tournent dans une instance du même module, à côté de celle de ses agents, sur un fil à elle et sans `escouade.call` : une vue ne demande rien à la machine. Le cœur attend sa réponse, par le même tour que le reste, dans le temps d'un tour du plugin.
- **Une instance en faute** est remplacée à l'appel suivant : les vues repartent de zéro, le cœur leur disant ce que la conversation contient déjà, et rien n'est montré deux fois. Un plugin arrêté ou changé perd son instance de vues.
- **Plugin coupé ou absent.** Le terminal marche encore, mais la conversation ne le suit plus, et elle le dit une fois : « Le plugin de <harness> n'a pas lu ce que sa vue a écrit (…). ». Avant, la vue marchait plugin coupé, sa traduction étant liée au cœur.
- **Ce que le cœur garde de copié.** Le lancement de l'interface n'est pas encore passé par `terminal/launch` : le cœur garde sa propre copie des mots qu'il lui faut (arguments, clôture, modèles d'un compte, prix d'OpenAI), dans `engine/<harness>/`.

- **Un module qui ne répond plus** n'est pas rappelé à chaque ligne, le cœur lisant sous le verrou de l'agent : le problème dit, les lignes passent sans lui 30 s, puis un seul essai, et ainsi de suite. Ces lignes ne sont pas relues. Un tour en cours se ferme quand même, sans son plugin.

## La clôture et la lecture seule après un redémarrage (ESC-362)

Un plugin redémarre sans que son harness s'arrête. Ce qu'il tient lui-même de la clôture et de la lecture seule, il le reprend de `agent/attach`, à la création de l'agent comme quand il lui est redonné, et ce que dit le cœur l'emporte sur ce qu'il avait gardé : une clôture retirée se retire.

- **Copilot CLI, Cursor, Gemini, Mistral Vibe et les harness ACP** tiennent la clôture dans le plugin, qui refuse d'après elle.
- **Codex** la redit à chaque tour (`turn/start`, son profil de clôture ou sa lecture seule).
- **OpenCode** la tient par les règles de sa session, et **Antigravity CLI** par ses règles : leurs plugins la reprenaient déjà.
- **Claude Code** n'a rien à reprendre : son process garde ses arguments et le fichier de réglages du cœur.
