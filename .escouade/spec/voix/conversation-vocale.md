---
title: "Conversation vocale"
subject: "Voix"
sources: ["docs/SPEC.md"]
tickets: ["ESC-97","ESC-110","ESC-114","ESC-115","ESC-131","ESC-132","ESC-134","ESC-136","ESC-137"]
decisions: []
compared: null
---

## Ce que c'est

Parler à l'agent et l'entendre répondre, comme le mode vocal de ChatGPT. Par défaut, tout se fait sur la machine : le micro et la transcription de la dictée (voir *Dictée*), et une voix de synthèse Piper livrée avec l'app. La transcription et la voix peuvent chacune passer par un service en ligne, avec la clé de l'utilisateur. Les demandes de permission et les questions de l'agent se font aussi à la voix, une à une.

## Comportement

### Commencer et arrêter

- Le bouton « Conversation » du compositeur (à gauche du micro). Un clic la commence, un autre (ou « Arrêter ») l'arrête. Elle s'arrête aussi quand le compositeur passe à un autre agent, quand la fenêtre est fermée ou rechargée, et quand l'app quitte.
- Une dictée en cours est annulée ; pendant la conversation, la dictée attend.
- Avant que le micro s'ouvre, un petit panneau au-dessus du bouton propose ce qui manque (le modèle de Whisper, la voix), avec sa taille et sa progression. En transcription en ligne, aucun modèle de Whisper n'est demandé ni chargé.
- La conversation se tient dans la « Langue parlée » de la dictée quand c'est le français ou l'anglais, sinon dans la langue de l'interface.

### Le tour

- Le micro reste ouvert de bout en bout. Pendant que l'agent travaille, il n'est pas écouté.
- La fin de ce qui est dit est trouvée par le silence. Le bruit de la pièce est appris sur les 1,5 s sans parole. Une phrase commence après 160 ms de voix sur 240 ms, et finit après « Délai avant l'envoi » de calme ; 60 s au plus. Elle garde les 540 ms d'avant sa détection, pour ne pas perdre un début dit bas.
- La phrase est transcrite, puis **part seule** à l'agent, marquée « Dit à voix haute » (voir *Conversation*). Rien compris : l'écoute reprend sans rien envoyer.

### Transcription

Elle suit « Transcription » de Réglages › Voix, comme la dictée (voir *Dictée*). Un choix changé pendant la conversation vaut pour la phrase suivante.

- **Locale** : Whisper, avec le modèle de la dictée, chargé dès que le micro s'ouvre, dans la langue de la conversation.
- **En ligne** : le service, la clé, le modèle et le proxy de la dictée, dans sa « Langue parlée ».
  - En direct (« Transcription en temps réel » cochée) : un flux par phrase, ouvert quand le début de la phrase est entendu (avec les 540 ms d'avant déjà gardées, ou ce que le barge-in a entendu). Il s'ouvre pendant les premières syllabes ; le son attend dans sa file. Tout le son de la phrase part pendant qu'on parle, de son début à sa fin, ses pauses et le silence qui la finit compris. La phrase n'est validée qu'à sa fin, d'un seul morceau ; son texte définitif arrive un instant après et part à l'agent. Les textes partiels s'affichent en sous-titre pendant qu'on parle (les derniers mots), le définitif les remplace.
  - **Rien ne part hors d'une phrase** : ni la pièce avant elle (au-delà des 540 ms gardées), ni pendant que l'agent est lu (seul le barge-in écoute, sur la machine), ni pendant que la conversation attend. Une attente au milieu d'une phrase ferme son flux, et la phrase part entière, par requête.
  - Par requête (case décochée, ou Groq) : chaque phrase finie part entière, en WAV mono 16 kHz.
  - Un flux qui échoue (il ne s'ouvre pas, tombe, ou ne donne pas son texte définitif dans les 5 s) : la phrase part par requête, jamais perdue. Le journal le dit une fois, et la conversation n'essaie plus de flux jusqu'à son arrêt.
  - Pannes de la requête, sans repli sur Whisper : clé absente, la conversation ne démarre pas et le dit, avant d'ouvrir le micro ; clé refusée, quota, service en panne, réseau ou proxy, la phrase est perdue, la conversation le dit une fois (avec le message de la dictée : le service, la raison, quoi faire) et réécoute, sans rien renvoyer d'elle-même.

### Ce qui est lu

- Chaque message envoyé pendant la conversation porte à sa fin une consigne, entre `<escouade-voice>` et `</escouade-voice>`, dans la langue des textes pour Claude (voir *Langues*) : finir la réponse par un court passage pour l'oreille, entre `<spoken>` et `</spoken>`, de deux ou trois phrases, sans code, chemin, liste ni détail technique.
- Ce passage est isolé une fois le texte écrit en entier : le dernier s'il y en a plusieurs, son Markdown retiré, jamais celui d'un sous-agent. La fenêtre le montre à part sous la réponse (« 🔊 Lu à voix haute ») ; la réponse entière reste dans la conversation.
- À la fin du tour, ce passage est lu. Sans passage : « C'est fait. La réponse est dans la conversation. » Tour en erreur : « L'agent s'est arrêté sur une erreur. »
- La lecture va phrase par phrase (la première est entendue plus tôt), chacune ramenée au même niveau, 150 ms entre deux, à la vitesse et au volume des réglages. Puis l'écoute reprend, après 300 ms de traîne de l'écho.
- Réglages changés pendant la conversation : les lectures suivantes prennent la voix, le locuteur, la vitesse et le volume enregistrés, et Whisper leur langue. Une nouvelle voix est chargée aussitôt ; une voix non téléchargée n'est pas prise. Une voix en ligne change au passage suivant.

### Barge-in et écho

Pendant la lecture, ce qu'entend le micro n'est jamais transcrit : il sert seulement à savoir si l'utilisateur parle par-dessus. Le couplage entre ce qui est joué et ce qui revient au micro est appris sur les 400 premières ms jouées. L'utilisateur est entendu quand le micro dépasse de 9,5 dB l'écho attendu (le plus fort des 300 dernières ms jouées) et de 14 dB le bruit de la pièce, sur 240 ms de 300. La lecture s'arrête aussitôt, et la phrase en cours est écoutée (ses 400 dernières ms gardées). Avec un casque, l'écho est nul et le seuil est celui de la pièce.

### La voix locale (Piper)

- Piper est dans l'app : ni programme à part, ni Python. eSpeak NG (GPL-3.0-or-later) change le texte en phonèmes, coupé en phrases et en propositions comme le fait Piper ; ses données du français et de l'anglais sont dans l'app, écrites une fois dans `~/.escouade/voice/espeak-ng-data`. Le modèle de la voix tourne dans ONNX Runtime. Le son sort par la sortie par défaut du système.
- Mac Intel et Linux : l'app livre la bibliothèque ONNX Runtime 1.23.2 de Microsoft (dans l'app universelle de macOS ; dans `/usr/lib/escouade` du .deb et de l'AppImage, qui visent Ubuntu 22.04) et la charge au premier usage. Introuvable : la conversation dit qu'elle n'est pas disponible.
- **Préchargement** : dès que le micro s'ouvre, la voix est chargée en arrière-plan (avec ONNX Runtime et la voix d'eSpeak NG), comme le modèle de Whisper : la première phrase lue ne l'attend pas. Une seule voix est chargée à la fois, jamais deux fois. Quitter la conversation la garde ; effacer la voix la décharge. Un préchargement raté est noté dans `app.log`, et la voix est chargée à la première phrase.
- **Les voix** : une liste vérifiée, livrée avec l'app et mise à jour aux releases (pas le catalogue de PiperHub lu en direct) : 36 voix, 7 en français et 29 en anglais. Par défaut : `fr_FR-siwis-medium` et `en_US-ljspeech-medium`. Fuze (`fuze-french-fr-multi-high`, française, haute qualité) a 14 styles : le locuteur 0 « normal » par défaut, le style se choisit comme le locuteur.
- **La règle d'admission**, réappliquée par l'app au chargement de la liste :
  - licence du modèle permissive : MIT, Apache-2.0, BSD, CC0, CC BY et CC BY-SA avec attribution, domaine public ; la famille GPL attend une confirmation ;
  - licence du jeu de données aussi, quand elle est connue : ni NC, ni ND, ni « recherche seulement » ;
  - aucune voix de personnage, de personne connue ni de marque (noms connus, « univers » de PiperHub autre que `default`, jeu de données tiré d'un jeu ou d'un film) ;
  - une copie d'une voix officielle est écartée pour l'officielle ;
  - une voix de la communauté dont on ne sait pas qui parle n'est proposée qu'après vérification (Colin, Librivox AU) ;
  - la licence lue sur Hugging Face (fiche du modèle, fiche officielle, fiche du jeu de données) prime sur celle de PiperHub.
  - Écartées entre autres : Tom (jeu de données AGPL), Ryan, HFC, Semaine, L2-ARCTIC (NC), Lessac (licence Blizzard), Mei (GPL), Miro (NC-ND), toutes les voix de personnages (GLaDOS, Wheatley, Valorant, Fortune's Café, Death, HAL 9000…).
- **Téléchargement** d'une voix : à la demande, au premier usage (le panneau propose la voix, avec son nom et sa taille) ou depuis Réglages › Voix. Son `.onnx.json` puis son `.onnx` viennent de Hugging Face, par le proxy des réglages, avec la progression ; taille et SHA-256 vérifiées (une somme fausse : rien n'est gardé). Rangée dans `~/.escouade/voice/voices`.

### La voix en ligne

Les cinq services de la dictée ont une synthèse vocale. La clé d'un service est la même que pour la dictée (voir *Dictée*) ; sa ligne le dit, dans les deux groupes.

| Service | Synthèse | Modèle par défaut | Voix par défaut (fr / en) | Vitesse |
| --- | --- | --- | --- | --- |
| OpenAI | `POST /v1/audio/speech` | `gpt-4o-mini-tts` | `marin` (`alloy` pour `tts-1`) | 0,25 à 4 |
| Groq | `POST /openai/v1/audio/speech`, 200 caractères au plus par requête | `canopylabs/orpheus-v1-english` | — / `hannah` (anglais seulement) | aucune |
| Mistral | `POST /v1/audio/speech` | `voxtral-mini-tts-2603` | `fr_marie_neutral` / `en_paul_neutral` | aucune |
| ElevenLabs | `POST /v1/text-to-speech/<voix>/stream` | `eleven_flash_v2_5` | `JBFqnCBsd6RMkjVDRZzb` (George) | 0,7 à 1,2 |
| Deepgram | `POST /v1/speak` (la voix est un modèle) | `aura-2` | `aura-2-agathe-fr` / `aura-2-thalia-en` | 0,7 à 1,5, en anglais seulement |

- Le volume s'applique sur la machine, quel que soit le service. Une vitesse hors des bornes du service y est ramenée ; un service sans vitesse le dit sous « Vitesse ». Un modèle qui ne lit pas la langue de la conversation (Orpheus, Aura et Flash v2 en français) : la ligne de la voix le dit, et Piper lit la conversation dans cette langue.
- **La lecture** : le passage est coupé en phrases comme pour Piper ; une phrase trop longue pour le service est recoupée aux virgules, puis aux espaces. La première phrase est demandée tout de suite, les suivantes pendant qu'elle joue. Le son est joué à mesure qu'il arrive, par la même sortie que Piper, au même volume, chaque sous-titre au début de sa phrase. Barge-in, écho, pause et arrêt comme avec Piper (la requête en cours est abandonnée). Le journal note le temps jusqu'au premier son.
- **Pannes** :
  - clé absente au début : la conversation le dit une fois et Piper lit ; sans voix Piper téléchargée, elle ne démarre pas, et dit pourquoi ;
  - clé refusée (401, 403), quota (429), service en panne (5xx), réseau ou proxy : la conversation le dit une fois (le service, la raison, quoi faire) et Piper lit le reste de la conversation si une voix locale est téléchargée ; sinon elle se met en pause (« Reprendre » relit au service) ;
  - voix inconnue du service (400, 404, 422) : c'est dit, et sa voix par défaut lit.
- Le journal ne contient jamais la clé ni le texte lu.

### Permissions et questions à la voix

- **Permission** : une demande de permission de l'agent pendant la conversation est lue (« Alice demande la permission : Bash rm -rf build. Tu autorises ? »). La réponse dite y répond : « oui », « d'accord », « vas-y »… autorisent ; « non », « refuse »… refusent, avec ce qui a été dit comme message ; « toujours » autorise toujours quand la permission le permet. Pas comprise : la conversation se met en pause et le dit.
- **Questions, une à la fois** : une demande de questions (de Claude Code, Codex, Copilot CLI ou OpenCode, d'un formulaire d'un serveur MCP, d'un agent de ticket aussi ; posée pendant l'écoute, ou déjà là quand la conversation commence) est posée une question à la fois. La voix dit la question (« Question 2 sur 3. Quels tests ? Plusieurs réponses possibles. » ; une question seule, sans son rang), **jamais ses choix** : ils s'affichent sous l'orbe (voir *Plein fenêtre*).
- La réponse dite répond à cette question. Elle est aussitôt cochée dans la carte de la question (une réponse libre est écrite sous sa question : « Réponse dite : « … » »), s'allume 0,7 s dans le plein fenêtre, puis la voix pose la suivante (celles déjà répondues sont sautées). Après la dernière, la demande part à l'agent, toutes réponses ensemble, comme le bouton de la carte.
- **Comprendre la réponse**, avec les mots de chaque langue :
  - une option par son nom, approché : sans casse ni accents ; un mot qui n'est que dans cette option suffit, ou son début (« Postgres » pour « PostgreSQL ») ; de deux noms dits qui se contiennent, le plus long ;
  - ou par son rang (« la deuxième », « le dernier ») ; un nombre (« 2 », « la une ») seulement dans une réponse faite de rangs ;
  - pour un choix multiple, plusieurs : « la première et la troisième », « unitaires et lint », « les deux » pour deux options, « toutes », « aucune » ;
  - sinon, ce qui a été dit devient la réponse libre, partout où la fenêtre en prend une : toute question sauf une liste de choix d'un formulaire MCP (Claude Code compris, dont l'« Autre » est implicite).
- **Pas compris** : rien, deux options pour un choix unique, un rang hors de la liste, aucune option là où aucune réponse libre n'est prise. La voix le dit (« Je n'ai pas compris. » puis la question), le plein fenêtre aussi, deux fois au plus ; puis la conversation se met en pause sur cette question (« Reprendre » la repose ; les réponses déjà données restent cochées).
- **Commandes**, jamais des réponses (dans une phrase courte qui ne nomme aucune option) : « Répète » repose la question ; « annule », « laisse tomber » arrêtent les questions (« D'accord : la question attend ta réponse dans la fenêtre. ») et mettent la conversation en pause, la carte à répondre dans la fenêtre.
- Un choix fait dans le plein fenêtre (clic, touche de son rang) ou coché à la main dans la carte répond à la question posée, et la voix passe à la suivante. Une demande répondue entièrement dans la fenêtre (ou par un autre client du serveur MCP) : la conversation reprend l'attente de l'agent.

### Plein fenêtre

- Dès que le micro s'ouvre, la conversation recouvre toute la fenêtre d'Escouade (pas le plein écran du système), le reste flouté derrière.
- Au centre, une orbe : son bord respire et son halo grandit avec le niveau du son. Elle suit la voix lue quand l'agent parle, le micro quand elle écoute (mis à jour toutes les 50 ms) ; un arc tourne autour pendant la transcription et le travail de l'agent. Teinte ambre quand l'agent parle, rouge doux à l'écoute, bleue quand il réfléchit.
- Une question posée : l'orbe monte et rapetisse. Sous elle s'affichent « Question 2 sur 3 » (une demande de plusieurs), « Plusieurs réponses possibles », la question et ses choix, chacun avec son rang (1, 2, 3…), son libellé et sa description, sur deux colonnes à partir de 4 ; « Autre : dis ta réponse » quand une réponse libre est prise. Un clic ou la touche du rang choisit ; pour un choix multiple, les clics cochent et « Valider » (ou Entrée, ou la phrase dite) envoie. Ce qui a été compris s'allume un instant (une réponse libre : « Ta réponse : … ») avant la question suivante ; une réponse pas comprise : « Je n'ai pas compris : redis ta réponse, ou choisis ci-dessous. »
- En dessous, l'état : « À l'écoute : parle quand tu veux », « Je t'écoute… », « Transcription sur ta machine… » (« Transcription par <service>… » en ligne), « Alice travaille… », « Alice te répond », « Alice te pose une question », « Conversation en pause ».
- En bas, des sous-titres de cinéma, centrés, deux lignes au plus : ce que dit l'utilisateur (dès qu'il est transcrit, ou au fil de la parole en direct) et ce que dit la voix, phrase par phrase, au moment où chacune commence à être entendue ; « TOI » ou le nom de l'agent au-dessus. L'historique complet reste dans la conversation.
- `Échap` ou « Arrêter » quitte le mode ; « Reprendre » en pause. L'animation ne tourne que tant que le plein fenêtre est là ; mouvement réduit si le système le demande.

### Crédits

`CREDITS.md` (Piper, eSpeak NG, ONNX Runtime, whisper.cpp, cpal, et chaque voix avec ses licences et son crédit), ouvert par Réglages › À propos › « Crédits et licences ».

### Journal et statistiques d'usage

- Le journal dit, pour chaque phrase, qui l'a transcrite (Whisper ou le service), en direct ou par requête, sa durée et le temps entre le dernier mot et le texte définitif, et la réponse du service. Jamais le texte ni la clé.
- `voice_conversation` quand une conversation commence à écouter, avec `lang`, `voice` (la voix Piper, ou celle du service), `engine` (`piper`, ou le service) et `model` (le modèle du service, `piper` sinon). Jamais ce qui est dit ni lu.

## Réglages

Réglages › Voix › Conversation (voir *Réglages*) :

- « Voix générée par » : « Local (Piper) » (défaut) ou « En ligne », puis « Service » (OpenAI par défaut), « Modèle », la clé, « Voix en Français », « Voix en English ». La note du groupe dit ce qui part et où : le texte de chaque passage lu, phrase par phrase, vers le service, avec la clé, par le proxy des réglages réseau ; pas le son du micro, qui suit « Transcription » (voir *Dictée*), à laquelle elle renvoie.
- « Modèle » : comme celui de la transcription (voir *Dictée*). Un modèle disparu retombe sur le défaut ; la ligne le dit, et la conversation aussi, à son début.
- « Voix en … » d'un service :
  - quand son API liste ses voix (ElevenLabs, 100 par page, celles du compte comprises, marquées « ton compte » ; Mistral, ses voix prêtes et celles du compte ; Deepgram, par modèle) : un champ qui se complète en tapant (nom, identifiant, genre, accent, description), filtré sur la langue et le modèle, choisi aux flèches et à Entrée, Échap pour fermer ;
  - sans liste (OpenAI, Groq), ou avec une clé qui ne peut pas lire les voix : un champ libre pour le nom ou l'identifiant attendu par le service, avec un exemple, la voix par défaut en indication et « Voir ses voix » (la page des voix du service) ;
  - vide : la voix par défaut du service. Gardée par service et par langue (et par modèle pour Deepgram) ;
  - « ▶ Écouter » lit une phrase avec le service, le modèle et la voix choisis ; une voix refusée est dite tout de suite, avec la réponse du service.
- « Voix locale en … » (la voix de chaque langue en local) : la voix Piper du français et de l'anglais, avec sa qualité et sa taille, son locuteur pour une voix qui en a plusieurs, « ▶ Écouter » (une phrase dans la voix, à la vitesse et au volume choisis), « Télécharger » ou « Effacer », la licence du modèle, celle du jeu de données et le crédit demandé (CC BY). En ligne, ce sont elles qui lisent si le service ne répond pas.
- « Vitesse » (0,5 à 2), « Volume », « Délai avant l'envoi » (1,2 s par défaut, de 0,5 à 3 s).

## Limites

- Vue Escouade seulement : pas dans la vue terminal d'un harness.
- Une transcription en ligne qui échoue n'est jamais refaite par Whisper : la phrase est perdue.
- Groq lit en anglais seulement ; Orpheus, Aura et Flash v2 ne lisent pas le français (Piper le lit).
- Une liste de choix d'un formulaire MCP ne prend pas de réponse libre à la voix.
