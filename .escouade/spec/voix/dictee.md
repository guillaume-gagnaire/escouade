---
title: "Dictée"
subject: "Voix"
sources: ["docs/SPEC.md"]
tickets: ["ESC-16","ESC-96","ESC-110","ESC-132","ESC-134","ESC-135","ESC-136"]
decisions: []
compared: null
---

## Ce que c'est

Dicter un message au lieu de le taper. Par défaut, tout se fait sur la machine : Escouade écoute le micro et Whisper transcrit ; rien de ce qui est dit ne quitte la machine, seul le modèle est téléchargé. En option, la transcription passe par un service en ligne, avec la clé de l'utilisateur, pour une machine trop lente pour Whisper ou pour une meilleure qualité. Le texte est écrit dans le champ, jamais envoyé seul. Le choix de la transcription et la clé d'un service valent aussi pour la conversation vocale (voir *Conversation vocale*).

## Comportement

### Où dicter

- Un bouton micro dans le compositeur (à côté du trombone), dans la fenêtre d'un ticket (à côté de « Joindre », pour la description) et, en vue Claude Code, en haut à droite du terminal.
- Le raccourci de la dictée (voir *Raccourcis*) dicte dans le champ qui a le focus, sinon dans le dernier affiché (le compositeur). Il est pris avant tout champ : un terminal ne le reçoit jamais.
- Le micro est écouté par l'app, jamais par la page de la fenêtre. Sous Linux, PulseAudio et PipeWire passent par ALSA.

### Parler

- Deux façons. Un clic bref (ou un appui bref du raccourci) commence, un second finit. Un appui maintenu 400 ms ou plus fait « appuyer pour parler » : relâcher finit. Relâché pendant que le micro s'ouvre encore, la dictée finit dès qu'il écoute ; la fenêtre quittée pendant l'appui compte comme relâché.
- `Échap` annule : rien n'est écrit.
- Pendant l'écoute, un panneau au-dessus du bouton (en dessous dans un ticket et dans le terminal) montre « À l'écoute », le niveau du son (une barre, et un halo autour du bouton, mis à jour toutes les 50 ms) et la durée. Puis « Transcription sur ta machine… », ou « Transcription par <service>… » en ligne.
- En transcription en direct, le texte partiel s'écrit dans ce panneau (ses dernières lignes) pendant qu'on parle.

### Le texte

- Inséré au curseur du champ (la sélection remplacée), séparé des mots voisins par une espace (pas devant une ponctuation), le curseur juste après. **Jamais envoyé tout seul.**
- En vue Claude Code, il est tapé dans le terminal comme un collage (entre crochets de collage quand Claude Code les demande) : il n'est pas validé.
- Rien entendu : « Rien entendu : parle un peu plus près du micro, puis réessaie. » Les marqueurs de Whisper sans parole (`[BLANK_AUDIO]`, `(musique)`…) sont retirés.

### Le son

Capté au format par défaut du micro, ramené en mono puis à 16 kHz. C'est ce son que Whisper transcrit, ou qui part au service en ligne (en WAV 16 bits).

### Fin de la dictée

La dictée finit à la fin de l'écoute ; après 10 minutes (ce qui a été dit est transcrit) ; micro débranché (idem, et la raison est dite) ; fenêtre fermée (vers la zone de notification), cachée ou rechargée ; app quittée (un téléchargement en cours s'arrête aussi). Le micro est relâché avant la transcription.

### Transcription locale (Whisper)

- Whisper (whisper.cpp) transcrit sur la machine, sans jamais bloquer l'interface. Chaque dictée est transcrite seule, sans le contexte de la précédente. Le modèle est chargé pendant que l'utilisateur parle, puis gardé chargé (un seul à la fois). Une annulation interrompt Whisper en cours.
- **Le modèle** est téléchargé à la demande : au premier usage (le panneau propose « Télécharger le modèle de transcription ? », avec son nom et sa taille) ou depuis Réglages › Voix. La progression est affichée ; le téléchargement est annulable. Il vient de Hugging Face (`ggerganov/whisper.cpp`), par le proxy et le réglage TLS des réglages réseau (voir *Réglages*), sans limite de durée totale (60 s sans un octet : échec). Il n'est gardé qu'entier et de la bonne somme SHA-256 ; sinon « Le fichier téléchargé n'est pas le bon modèle (somme de contrôle différente) : il n'est pas gardé… », et rien ne reste. Il est rangé dans `~/.escouade/voice/models/ggml-<modèle>.bin`.
- Modèles : tiny (78 Mo), **base** (148 Mo, par défaut), small (488 Mo), large-v3-turbo-q5_0 (574 Mo), medium (1,5 Go), large-v3-turbo (1,6 Go).

### Transcription en ligne

Seule la transcription change : l'écoute, le texte et la fin de la dictée restent les mêmes. Aucun modèle de Whisper n'est demandé ni téléchargé.

| Service | Adresse (par requête) | Modèle par défaut |
|---|---|---|
| OpenAI | `https://api.openai.com/v1/audio/transcriptions` | `gpt-4o-mini-transcribe` |
| Groq | `https://api.groq.com/openai/v1/audio/transcriptions` | `whisper-large-v3-turbo` |
| Mistral | `https://api.mistral.ai/v1/audio/transcriptions` | `voxtral-mini-latest` |
| ElevenLabs | `https://api.elevenlabs.io/v1/speech-to-text` | Scribe `scribe_v2` |
| Deepgram | `https://api.deepgram.com/v1/listen` | `nova-3` |

- La clé part dans un en-tête de la requête, jamais dans l'adresse. Délais : connexion 15 s, lecture 60 s. `Échap` abandonne la requête.
- **Par requête** : le son de toute la dictée part une fois la dictée finie.
- **En direct** (« Transcription en temps réel ») : le son part au fil de la parole, par un flux chiffré (WebSocket) ouvert avec le micro ; ce qui a été entendu pendant son ouverture part d'abord. Le texte partiel s'affiche dans le panneau ; le texte définitif, prêt un instant après l'arrêt, va dans le champ à la place. Le flux a 5 s au plus pour s'ouvrir, et 5 s au plus pour donner le texte définitif. Services en direct :
  - ElevenLabs : Scribe v2 Realtime (`scribe_v2_realtime`, `wss://api.elevenlabs.io/v1/speech-to-text/realtime`) ;
  - Mistral : Voxtral Realtime (`voxtral-mini-transcribe-realtime-2602`, `wss://api.mistral.ai/v1/audio/transcriptions/realtime`) ;
  - Deepgram : le modèle choisi, en streaming (`wss://api.deepgram.com/v1/listen` ; avec Nova-3, langue trouvée par le service en « auto ») ;
  - OpenAI : une session de transcription (`wss://api.openai.com/v1/realtime?intent=transcription`, `gpt-live-transcribe`, qui rend le texte au fil de la parole).
- Un flux qui ne s'ouvre pas (proxy qui refuse les WebSockets, clé refusée, quota), qui tombe, ou qui ne donne pas son texte définitif à temps : le son part entier par requête, jamais perdu. Le journal le dit une fois.
- **Réseau** : requêtes et flux suivent les réglages réseau (voir *Réglages*) : le proxy (identifiants compris), ses exclusions, « certificats non vérifiés ». Le flux fait confiance aux autorités du système (celle d'un proxy qui inspecte TLS) et à celles de Mozilla.
- **La clé** : une entrée par service dans le trousseau du système (`dictation-<service>`, sous le service de l'app, comme les intégrations). Elle sert à la dictée, à la conversation vocale et à lire les modèles et les voix du service. « Enregistrer la clé » l'enregistre tout de suite (pas avec le reste des réglages), puis la relit pour vérifier ; « Remplacer », « Effacer la clé ». Jamais dans `settings.json` ni dans un autre fichier : un trousseau qui la refuse (ou aucun, sous Linux) la laisse non enregistrée, et le dit. La fenêtre ne la reçoit jamais en retour : elle sait seulement quels services en ont une. Jamais dans le journal, même dans ce qu'un service répond en erreur.
- **Erreurs**, dites avec la marche à suivre ; rien n'est écrit dans le champ, et Whisper ne prend jamais le relais :
  - pas de clé, dit avant toute écoute : « Aucune clé <service> n'est enregistrée. Ajoute-la dans Réglages › Voix, ou repasse en local. » ;
  - clé refusée (401, 403) : « <service> refuse la clé. Vérifie-la dans Réglages › Voix. » ;
  - réseau coupé, proxy, délai : « <service> ne répond pas : <raison>. Vérifie le réseau et le proxy (Réglages › Réseau), ou repasse en local. » ;
  - autre réponse : « <service> n'a pas transcrit la dictée (<statut>) : <ce qu'il a dit> ».

### Le micro et ses erreurs

Chaque erreur est dite avec la marche à suivre.

- macOS demande l'accès au micro, avec sa raison en français ou en anglais. Un refus n'y donne que du silence : une écoute d'une demi-seconde ou plus faite uniquement de zéros est dite « Aucun son n'est arrivé du micro : il est coupé, ou le système le refuse à Escouade. Pour l'autoriser, ouvre Réglages Système › Confidentialité et sécurité › Microphone, coche Escouade, puis relance l'app. »
- Windows : un refus renvoie à Paramètres › Confidentialité et sécurité › Microphone (« Accès au microphone », puis « Autoriser les applications de bureau à accéder à votre microphone »).
- Linux : PulseAudio ou PipeWire, micro coupé, groupe audio.
- Aucun micro : « Aucun micro trouvé. Branche un micro, ou choisis-en un dans Réglages › Voix. » Un micro pris par une autre application est dit aussi. Un micro choisi qui n'est plus là : celui du système écoute à sa place.

### Statistiques d'usage

`dictation_used` quand une dictée a donné du texte, avec `target` (`composer`, `terminal`, `ticket`) et `model` (le modèle de Whisper, ou le service en ligne, avec alors `online_model`). Jamais le texte, le son ni la clé.

## Réglages

Réglages › Voix (voir *Réglages*) :

- « Micro utilisé » : celui du système par défaut.
- « Langue parlée » : celle de l'interface par défaut, la détection automatique, ou fr, en, de, es, it, pt, nl.
- « Raccourci » : voir *Réglages* et *Raccourcis*.
- **Transcription** :
  - « Transcrite par » : « Local (Whisper) » (défaut) ou « En ligne ».
  - En local : « Modèle » (taille, vitesse et qualité de chacun ; « Télécharger » ou « Effacer » sur sa ligne, tout de suite).
  - En ligne, les modèles de Whisper sont cachés : « Service » (OpenAI par défaut), sa clé, « Modèle », « Transcription en temps réel ».
  - « Modèle » d'un service : la liste de son API, lue avec la clé, gardée dans `~/.escouade/voice/online-catalog.json` et relue par « Actualiser la liste » ou à l'enregistrement d'une clé ; sinon la liste de sa documentation, livrée avec l'app (ElevenLabs ne liste pas ses modèles de transcription). Chaque modèle a son nom et ce qui le distingue (« rapide », « plus précis », « 32 langues »…), le défaut en premier. Par défaut, le modèle du jour du service : rien n'est gardé tant que l'utilisateur n'en choisit pas un autre. Le choix est gardé par service (changer de service puis revenir le retrouve). Un modèle que le service ne propose plus retombe sur son défaut, et la ligne le dit.
  - « Transcription en temps réel » : sous « Modèle », seulement pour un service qui la propose (pas Groq), cochée par défaut, retenue par service. Sa ligne dit le modèle pris (celui du service en direct ; pour Deepgram, celui choisi) et que la minute en direct coûte plus cher chez ce service (environ 1,8 fois pour ElevenLabs et Deepgram, plus pour OpenAI et Mistral).
  - La note du groupe dit ce qui part et où : le son de chaque dictée vers <service> (<adresse>), avec ta clé, par le proxy des réglages réseau ; au fil de la parole par un flux chiffré quand « Transcription en temps réel » est cochée (en conversation, seulement pendant qu'on parle), sinon en un fichier WAV une fois la dictée finie ; le son de chaque phrase de la conversation vocale part aussi au service. « Rien d'autre : ni le projet, ni le code, ni la conversation avec l'agent. »

## Limites

- La langue n'est devinée que si « Langue parlée » est sur la détection automatique (par Whisper ou par le service).
- Groq n'a pas de transcription en direct.
- Une transcription en ligne qui échoue n'est jamais refaite par Whisper.
- Whisper demande un processeur x86-64 avec AVX2 (2013 et après) ou Apple Silicon. Il calcule avec Metal sous macOS, sur le processeur ailleurs.
