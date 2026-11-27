# Escouade en voiture : la liste des agents, l'Overseer, puis une conversation à la voix (étude 2.4)

Étude d'ESC-696, révisée par **ESC-698** pour la 2.4. **Rien du produit ne change dans ces tickets** : ce document, une page de spec en brouillon (« Escouade à distance › En voiture », `a-distance/en-voiture`) et une demande dans la boîte « À toi » où l'utilisateur tranche les choix (§ 10). Cette demande **remplace a-52**, celle d'ESC-696. Aucune décision n'est proposée avant ses réponses.

**La demande** (2026-11-27), d'abord : « Est-ce qu'il y a un monde où l'app mobile Android est compatible Android Auto en mode overseer ? », puis « Étude pour la 2.4, avec Apple aussi ». **La direction donnée après ESC-696** (2026-11-27) : « comme pour WhatsApp, je voudrais afficher la liste de mes agents ou ouvrir l'Overseer, puis ensuite ça ouvre une vue de conversation audio avec l'agent choisi. La conversation vocale pourrait se faire via les paramètres d'Escouade : ElevenLabs, Whisper, etc. » Elle tranche C2 d'ESC-696 (la voiture montre **la liste des agents et l'Overseer**, plus seulement le chef) et change la voix : ce sont **les fournisseurs de voix réglés dans Escouade**, plus seulement l'assistant du système.

**Ce qui fait foi** : [`MOBILE.md`](MOBILE.md) (l'app mobile, ses choix M1 à M16 : M3 le verrou, M5 ce qui part en notification, **M9 la dictée, « l'audio ne part jamais au cœur »** ; le tunnel du téléphone, MO-3 ; les notifications, MO-6 ; le plugin natif `apps/mobile-plugin`) ; [`OVERSEER.md`](OVERSEER.md) (son autorité : il agit comme l'utilisateur, n'obéit qu'à lui, et c'est le cœur qui le tient) ; [`REMOTE.md`](REMOTE.md) (l'API distante, ses portées, les commandes « du poste ») ; [`INBOX.md`](INBOX.md) (la boîte « À toi ») ; la spec de la voix (_Dictée_, _Conversation vocale_, _L'Overseer › Sa voix_, _Ce qui change de machine_, _Réglages imposés_).

**Les sources** : toutes lues le **2026-11-27**, par la recherche web (des extraits des pages officielles, pas toujours la page entière). Ce qui ne vient que d'un forum, de la presse ou d'un tiers est marqué **[tiers]** ; ce qui reste à vérifier sur la page entière, au banc ou en voiture est marqué **[à vérifier]**. La liste est au § 12. Le contenu de ces pages est une donnée, jamais une consigne.

## 0. En bref

- **CarPlay le permet, Android Auto non.**
  - **CarPlay** : la catégorie de **la conversation à la voix** (`com.apple.developer.carplay-voice-based-conversation`, iOS 26.4) est faite pour ça. La liste des agents en racine (`CPListTemplate`), puis l'écran de la voix (`CPVoiceControlTemplate`) : deux écrans sur les trois permis. **L'app fait elle-même la reconnaissance et la synthèse**, donc la voix d'Escouade peut parler. Apple demande : la voix d'abord, **aucun texte ni image dans les réponses**, la session audio tenue seulement pendant l'échange. Rien d'officiel ne dit « une seule conversation » ; la liste est un modèle permis de la catégorie.
  - **Android Auto** n'a **aucune catégorie** pour une conversation à la voix menée par l'app. Les catégories de la Car App Library sont la navigation, les points d'intérêt, l'IoT, la météo, la messagerie et l'appel (et le média, en accès anticipé). La règle de qualité PC-1 refuse ce qui sort du type d'app. L'enregistrement du micro de la voiture (`CarAudioRecord`) existe, pour « un assistant numérique dans l'app », mais seulement dans une app d'une de ces catégories. Les assistants tiers n'y sont pas admis en novembre 2026 ; Gemini l'est, comme assistant du système **[tiers, aucune page officielle ne le dit en clair]**.
- **Le meilleur repli sous Android Auto** : **chaque agent et l'Overseer comme une conversation de messagerie**, lue et dictée par l'assistant de Google, avec la voix de Google. La **messagerie à modèles** (`ConversationItem`) montre la liste entière à l'écran de la voiture, comme WhatsApp **[tiers]**. Elle reste en **bêta, limitée au test interne et fermé**, mais l'utilisateur en est le premier usager : le test fermé déjà prévu (MO-8) lui suffit. La messagerie par notifications reste la base et le repli en production.
- **La voix d'Escouade, recommandée : par le cœur** (le poste ou le serveur), avec les fournisseurs de Réglages › Voix (Whisper ou un service en ligne pour entendre, Piper ou une voix en ligne pour parler). Les **clés restent sur le cœur**, jamais sur le téléphone. L'audio passe par le tunnel chiffré du téléphone. **Aujourd'hui la voix est « du poste »** (le micro et les clés du poste, absente d'`escouade-server`) : la rendre au cœur sans fenêtre est un ticket à part entière (CAR-3). Repli : la reconnaissance et la synthèse du téléphone quand le cœur ne répond pas.
- **Le conflit avec M9** (« l'audio ne part jamais au cœur ») : recommandé, **une exception écrite pour la seule voix de la voiture**, allumée avec le mode voiture, l'audio seulement pendant que l'utilisateur parle, jamais gardé. L'audio part chez lui (son cœur), puis chez le fournisseur en ligne réglé, comme sur le poste. La dictée de l'app reste M9.
- **Ce qui est dit d'un tour d'agent** : le passage pour l'oreille que l'agent écrit déjà en conversation vocale (`<spoken>`), prévenu en plus que l'utilisateur conduit, puis passé par une garde du cœur. Aucun appel de modèle en plus.
- **La sécurité prime, tenue par le cœur** : un message venu de la voiture est marqué (`origin: car`) ; **aucun feu vert ne part de la voiture**, ni du chef ni d'un agent : pas d'autorisation dite « oui » à la voix, pas de validation, de lancement ni de consigne durable. Une demande d'autorisation est dite (« je te la garde pour le téléphone ») et attend dans « À toi », sous verrou.
- **Découpage** : sept tickets, **CAR-1 à CAR-7, environ 27,5 h d'agent** (28,5 h avec UnifiedPush), de l'ordre de **18 h de bout en bout**. **CarPlay n'attend pas MO-6** ; Android Auto oui.

## 1. Ce qui existe : la voix d'Escouade aujourd'hui

Ce que dit la spec (_Dictée_, _Conversation vocale_, _L'Overseer › Sa voix_) et ce que fait le code au 2026-11-27 :

- **Entendre** : Whisper (whisper.cpp) sur la machine, ou un service en ligne : OpenAI, Groq, Mistral, ElevenLabs (Scribe v2, et Scribe v2 Realtime en direct), Deepgram. La transcription en direct passe par un flux (WebSocket) ouvert seulement pendant qu'on parle, et retombe sur une requête si le flux échoue.
- **Parler** : Piper sur la machine (36 voix vérifiées, 7 en français) ou la synthèse d'un des cinq services (ElevenLabs : `eleven_flash_v2_5` par défaut, une voix au choix de la bibliothèque et du compte). La lecture va phrase par phrase, la première demandée tout de suite.
- **Les voix choisies** : **une voix par service et par langue** (« Voix en Français », « Voix en English » ; par modèle chez Deepgram), et « ▶ Écouter ». ElevenLabs, Mistral et Deepgram listent leurs voix, celles du compte marquées « ton compte ». **Il n'existe pas de « voix épinglées »** : ni favoris, ni voix par agent, ni par projet. La voiture prendrait la voix choisie pour la langue de la conversation.
- **La conversation vocale d'un agent** : le micro ouvert de bout en bout, la fin d'une phrase trouvée par le silence, l'interruption (barge-in) par un calcul d'écho fait pour les haut-parleurs du poste. Chaque message porte la consigne `<escouade-voice>` : finir par un passage pour l'oreille entre `<spoken>` et `</spoken>`, de deux ou trois phrases, sans code ni chemin (`src-tauri/src/voice/spoken.rs`). **Une demande d'autorisation y est lue, et « oui » à la voix l'accorde** ; une question est posée une à une.
- **La voix de l'Overseer** : la même, en « appuyer pour parler » dans son écran.
- **Les clés** : une par service, au trousseau du système du poste (`dictation-<service>`), jamais dans un fichier ni renvoyée à la fenêtre. Les cinq services sont des plugins en WASM (`dev.escouade.<service>`).
- **La voix est « du poste »** : `voice_*`, `conversation_*`, `speech_*`, `overseer_voice_start` sont des commandes du poste (`desk-only` dans `REMOTE.md`), faites par la fenêtre. Sur un espace serveur, la voix tourne dans le client du poste, ses clés restent au trousseau du poste, et seul le texte part au serveur (_Ce qui change de machine_). `escouade-server` n'a pas de voix (son état est absent sans fenêtre).
- **Les stratégies** : `voice.dictation` coupe la dictée, et la voix de l'Overseer avec elle ; `voice.onlineAllowed` ramène tout au local (_Réglages imposés_).
- **Le téléphone** dicte par le système (M9) ; la voix d'Escouade n'y est pas (`MOBILE.md`, « Ce qui reste au poste »).

**Pour la voiture**, il faut donc une voix d'Escouade **sans fenêtre**, menée par le cœur pour un appareil distant : c'est le travail neuf de cette étude (§ 4, CAR-3).

## 2. Android Auto

### 2.1 Comment WhatsApp fait

- **Le chemin officiel** pour une app de messagerie, celui de WhatsApp depuis des années : **la messagerie par notifications** (_Extend messaging notifications to Android Auto_, developer.android.com). La notification est de catégorie `CATEGORY_MESSAGE`, au style `MessagingStyle`, avec deux actions : répondre (`RemoteInput`) et marquer comme lu. Android Auto annonce, **lit avec sa propre synthèse**, dicte la réponse avec sa propre reconnaissance (Google Assistant ou Gemini), la relit et l'envoie. L'app ne dessine rien. **Seules les conversations qui ont du neuf** apparaissent.
- **La liste des conversations à l'écran** : WhatsApp montre maintenant ses discussions, ses appels et ses favoris sur l'écran de la voiture, sous Android Auto et sous CarPlay (une interface datée de juillet 2026 par la presse) **[tiers]**. Sous Android Auto, c'est ce que permet la **messagerie à modèles** de la Car App Library (§ 2.2) ; aucune page officielle ne dit quelle API prend WhatsApp **[à vérifier]**.

### 2.2 La messagerie à modèles (`ConversationItem`)

_Build templated messaging experiences for Android Auto_ (lu le 2026-11-27) :

- Une app de la catégorie `androidx.car.app.category.MESSAGING` (filtre d'intention de son `CarAppService`), `minCarApiLevel` 7. Elle doit **aussi** faire la messagerie par notifications.
- La vue d'ensemble : des `ConversationItem` dans un `ListTemplate` ou un `SectionedItemTemplate` ; chacun porte d'office lire, marquer comme lu et répondre, et des actions à soi depuis la bibliothèque 1.7.
- **Toujours en bêta** : « tout le monde peut publier des apps de communication à modèles sur les pistes de test interne et de test fermé » ; « le test ouvert et la production seront permis plus tard ». Aucun changement relevé depuis ESC-696.
- **La lecture et la réponse restent celles de l'assistant de Google** : toucher une conversation la fait lire, « répondre » la fait dicter. L'app ne mène pas la voix.
- **Un retour de terrain** **[tiers]** : un fork d'OpenBubbles rapporte qu'Android Auto, dans une vraie voiture, ne s'est jamais lié à son service à modèles, alors que tout marchait sur le Desktop Head Unit **[à vérifier en voiture]**.

**Pour nous**, c'est **la liste des agents** demandée : l'Overseer en tête, puis les agents (§ 5.1), chacun une conversation. Le test fermé suffit à l'utilisateur ; en production, les notifications seules restent.

### 2.3 Le micro de la voiture et sa propre voix

- **`CarAudioRecord`** (_Record from the car microphone_ ; référence de `CarAudioRecord` ; _Communicate with the app by voice_, lus le 2026-11-27) : « pour bâtir des fonctions comme un assistant numérique dans l'app », la Car App Library laisse enregistrer le conducteur par le micro de la voiture. Il faut la permission `RECORD_AUDIO`, demandée à l'exécution, et **le focus audio pris avant d'enregistrer** (ce qui coupe le média en cours), puis arrêter si on le perd. Un indicateur s'affiche pendant l'enregistrement ; l'audio va à l'app, rien n'est gardé par la bibliothèque. Une source tierce le dit de la Car API 5 **[à vérifier]**.
- **Quelles catégories peuvent s'en servir** : les pages ne restreignent pas l'API à une catégorie ; **mais une app de la Car App Library doit être d'une catégorie admise** (§ 2.4), et aucune n'est « conversation à la voix ». Une app de messagerie qui ouvrirait son propre micro sortirait de ce que la messagerie fait (lire et répondre par l'assistant) **[à vérifier au DHU : l'hôte l'accepte-t-il d'une app de messagerie ?]**.
- **Jouer sa propre voix de synthèse** : rien d'officiel ne le dit pour une app de messagerie ou d'une autre catégorie que le média. L'audio d'une app passe par le téléphone, que la projection envoie à la voiture ; le focus audio est la règle commune. **Hors du média, aucune garantie** **[à vérifier au DHU]**.

### 2.4 La liste fermée des catégories, et les assistants

- **Les catégories** (_Set up your project_, _Android for Cars overview_, _Car app quality_) : `NAVIGATION`, `POI`, `IOT`, `WEATHER`, `MESSAGING`, `CALLING` dans le manifeste d'une app de la Car App Library ; le média, par sa propre API, et par modèles seulement en accès anticipé ; les jeux et la vidéo, voiture à l'arrêt. **Aucune catégorie d'assistant, de robot ou de conversation à la voix.**
- **La règle de qualité** : PC-1 refuse les fonctions hors des types d'apps prévus pour la voiture ; MF-5 veut une messagerie « de pair à pair », pas un service de notifications. La seule mise à jour récente relevée (mai 2026) touche VC-1 : les apps de média et de navigation doivent prendre les commandes vocales de Gemini et de Google Assistant. **Rien en novembre 2026** sur les agents d'IA.
- **Ce qu'on sait des autres** **[tiers]** : en 2026, Google tient une liste des catégories et les chatbots n'y sont pas ; Gemini y est, comme assistant du système ; ChatGPT est venu sur CarPlay, pas sur Android Auto ; Google n'a rien annoncé.
- **La revue** (_Distribute to cars_) : aucune sur le test interne ; **non bloquante sur le test fermé** (la soumission passe, on dit seulement ce qui n'est pas conforme) ; bloquante en test ouvert et en production.

### 2.5 Le meilleur repli

1. **Recommandé** : **la messagerie, une conversation par agent et une pour l'Overseer**. À modèles pour le test fermé (la liste à l'écran) ; par notifications partout (la base, le repli en production). Google lit et dicte, avec sa voix. Ce que l'utilisateur entend d'un agent est le même passage pour l'oreille que sous CarPlay (§ 6.1), et ses réponses partent comme des messages dictés en voiture (§ 7).
2. **Écarté pour le produit** : une app de messagerie qui mènerait la voix d'Escouade par `CarAudioRecord` et sa propre synthèse. Hors de ce que la catégorie fait (PC-1), elle ne passerait pas la production. Un essai au DHU peut dire si l'hôte la laisse faire, **pour savoir, pas pour livrer**.
3. **Pour mémoire** : un « bulletin » joué comme un média (les nouvelles de l'Overseer lues avec la voix d'Escouade), à côté de la messagerie. Ce n'est pas une conversation, et le média attend sa propre revue **[à vérifier]**.
4. **Android Automotive OS** (le système intégré à la voiture) : hors de la 2.4, comme dans ESC-696 (C7).

## 3. CarPlay

### 3.1 La conversation à la voix

D'après le _CarPlay Developer Guide_ (édition d'octobre 2026, developer.apple.com), la page de l'autorisation `com.apple.developer.carplay-voice-based-conversation` et _Requesting CarPlay entitlements_ (lus le 2026-11-27) :

- **Ce qu'elle est** : nouvelle depuis iOS 26.4 ; « une modalité principale qui est la voix dès le lancement », puis répondre aux questions et aux demandes et agir. Celle de ChatGPT (avril 2026), Claude et Gemini **[tiers]**.
- **Ce qu'Apple demande** :
  - l'écran de la voix (`CPVoiceControlTemplate`, réservé à la navigation et à cette catégorie) affiché **pendant que la voix sert** ;
  - **ni texte ni image dans les réponses** (« should not show text or imagery in query responses ») **[tiers pour la citation exacte, à vérifier dans le guide]** ;
  - une session audio tenue **seulement pendant que la voix sert**, pour ne pas couper la musique ou la radio au repos ;
  - **trois écrans de modèle au plus, la racine comprise** ;
  - les modèles permis : feuille d'actions, alerte, grille, **liste**, barre d'onglets, information, point d'intérêt et **contrôle vocal** ;
  - les boutons d'action et de la barre : iOS 26.4 et Xcode 26.4, deux boutons d'action au plus d'après des tiers **[à vérifier]** ;
  - aucune commande de la voiture ni de l'iPhone.
- **Pas de mot d'éveil** : l'utilisateur touche l'icône de l'app sur l'écran de la voiture ; « Dis Siri » ne la lance pas **[tiers]**. Un ingénieur d'Apple dit sur les forums que Siri et App Intents donnent l'autre chemin, et la catégorie celui des modèles **[tiers, forums d'Apple]**.
- **L'autorisation** : sur demande à Apple (CarPlay › Contact Us, le _CarPlay Entitlement Addendum_), ajoutée au compte de développeur ; **une seule catégorie par app**. Aucun délai officiel : de quelques semaines à des mois d'après les forums **[tiers]**.

### 3.2 La liste des agents, puis la voix

- **La racine** : un `CPListTemplate` (une ligne pour l'Overseer, puis une par agent, § 5.1). **Une ligne n'a qu'un nom, un projet et un état** (« attend ta réponse », « travaille », « fini il y a 4 min ») : aucun texte de message.
- **Toucher une ligne** pousse l'écran de la voix de cet agent : **deux écrans sur trois**. Le troisième reste libre (une alerte : « pas de réseau »).
- **Plusieurs agents, donc plusieurs conversations** : la liste est un modèle permis, et rien de ce qu'on a lu ne limite la catégorie à une seule conversation. Mais aucune page ne le dit en clair, et un fil des forums demande sans réponse comment montrer une conversation en cours **[à vérifier : poser la question dans la demande d'autorisation]**. Une seule session de voix à la fois : revenir à la liste ferme la précédente.
- **L'écran de la voix** : ses états (« Je t'écoute », « Il réfléchit », « Il te parle »), le nom de l'agent en titre, deux boutons (« Parler » ou « Arrêter », « Retour à la liste »).

### 3.3 La reconnaissance et la synthèse sont faites par l'app

La catégorie ne fournit **ni reconnaissance ni synthèse** : l'app ouvre sa session audio (`AVAudioSession`), capte le micro et joue sa voix. Elle peut le faire elle-même sur l'iPhone (`SFSpeechRecognizer` sur l'appareil, `AVSpeechSynthesizer`), ou **envoyer le son à un service et jouer ce qu'il rend** : c'est ce qui laisse parler la voix d'Escouade (§ 4). Des forums **[tiers]** disent que certaines voitures ne jouent pas un `AVSpeechUtterance` et que le média reprend mal après une capture : **[à vérifier en voiture]**.

### 3.4 La recommandation de C4, revue

- **ESC-696 recommandait la communication** (SiriKit : Siri lit et dicte, annonce les messages app fermée), la voix en recours. **La direction de l'utilisateur la retourne** : la communication ne laisse ni la voix d'Escouade (Siri lit et dicte), ni une liste à soi au-delà des lignes de messages (`CPMessageListItem`).
- **Recommandé (C4)** : **la conversation à la voix**, seule (une catégorie par app). Elle est faite pour les agents d'IA, donc la plus probable à obtenir. Elle porte la liste des agents puis la voix d'Escouade, et n'attend pas MO-6 : sans notification, elle marche app ouverte sur l'écran de la voiture.
- **Ce qu'on perd** : l'annonce spontanée app fermée. Hors de l'app, le téléphone garde les notifications ordinaires de M5, sans texte. Dans l'app ouverte, une nouvelle peut être dite dans la session de voix en cours **[à vérifier : ouvrir la session audio pour une nouvelle, sans que l'utilisateur ait parlé]**.

## 4. La voix d'Escouade en voiture

### 4.1 Où se font la reconnaissance et la synthèse

| Où                                 | Ce que c'est                                                                                                                                                    | Pour                                                                                                         | Contre                                                                                                                                                                |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **V1 · par le cœur** (recommandé)  | Le téléphone capte et joue ; le son passe par le tunnel ; le cœur entend (Whisper ou le service réglé) et parle (Piper ou la voix réglée), avec Réglages › Voix | La voix d'Escouade, celle du poste ; les clés sur le cœur ; un seul réglage ; la même voix pour chaque agent | Le son quitte le téléphone (M9, § 4.4) ; il faut le réseau ; il faut une voix d'Escouade sans fenêtre (CAR-3) ; sur un serveur, la clé de la voix doit y être (§ 4.2) |
| **V2 · sur le téléphone**          | La reconnaissance et la synthèse de l'iPhone, sur l'appareil                                                                                                    | Aucun son ne part ; tient sans réseau pour entendre ; aucune clé                                             | Pas la voix d'Escouade ; une seule voix système ; mettre les clés des fournisseurs sur le téléphone est **exclu**                                                     |
| **V3 · les deux** (le repli de V1) | Entendre sur le téléphone (le texte monte, M9 tenu), parler par le cœur (le son de la voix d'Escouade descend)                                                  | M9 tenu ; la voix d'Escouade pour parler ; moins de données montantes                                        | Pas Whisper ni Scribe pour entendre ; la reconnaissance de l'iPhone, de qualité variable en voiture                                                                   |

**Recommandé (C9)** : **V1**, qui suit les réglages d'Escouade comme la conversation vocale du poste ; **V3 en repli automatique**, quand le cœur n'entend plus (le tunnel tombe, le fournisseur refuse) ; la synthèse du téléphone (`AVSpeechSynthesizer`) pour les seules phrases de service (« pas de réseau, je réessaie »). Sous Android Auto, aucune des trois : c'est l'assistant de Google (§ 2.5).

### 4.2 Les clés des fournisseurs, sur le cœur

- **Un poste** : la clé du service est déjà au trousseau du poste ; le cœur du poste s'en sert pour le téléphone.
- **Un serveur** : aujourd'hui, la clé d'une voix reste au trousseau du poste client, et le serveur n'a pas de voix. Pour la voiture, **la clé du service doit être posée sur le serveur** (son magasin de secrets en fichier, comme les autres clés d'un serveur), par un réglage neuf de l'espace : « La voix en voiture, sur Travail », qui dit « Gardée dans un fichier sur Travail ». Ou bien Whisper et Piper sur le serveur, sans clé (un VPS sans carte graphique : Whisper base de l'ordre de la seconde par phrase **[à mesurer]**).
- **Jamais sur le téléphone**, jamais dans un réveil ni dans le journal. Le téléphone ne voit que du son et du texte.

### 4.3 La latence, le réseau, l'interruption, le bouton, le coût

- **Le tour** (V1, de l'ordre de **[à mesurer sur 4G]**) : la fin de la phrase (un bouton : tout de suite ; le silence : le « Délai avant l'envoi », 1,2 s par défaut) ; le son en Opus par le tunnel (environ 3 ko/s, contre 32 ko/s en PCM 16 kHz ; un aller-retour de 4G, de l'ordre de 50 à 150 ms) ; le texte définitif (en direct chez ElevenLabs, Deepgram, Mistral ou OpenAI : quelques centaines de ms après la fin ; Whisper base sur le poste : de l'ordre de la seconde) ; **le tour de l'agent, qui domine** (des secondes à des minutes) ; le premier son de la synthèse (Flash v2.5 d'ElevenLabs et Piper : en dessous de la seconde **[à mesurer]**). La voix ajoute de l'ordre d'une à deux secondes à ce que met l'agent.
- **Un tour long** : la voix dit tout de suite « C'est parti, je te le dis quand c'est fini », libère la session audio (CarPlay le veut), et dit la réponse à la fin du tour si l'écran de la voix est encore ouvert ; sinon elle attend dans la liste (« fini il y a 2 min »).
- **Les zones sans couverture** : la phrase captée est gardée **en mémoire**, jamais sur le disque, et renvoyée au retour du tunnel (deux minutes au plus, puis « pas envoyé, redis-le ») ; ce que le cœur avait à dire attend dans la conversation ; une alerte « Pas de réseau » est le troisième écran permis.
- **Parler pendant que la voix parle** : le barge-in du poste (l'écho appris sur ses haut-parleurs) ne vaut pas pour une voiture. Recommandé : la **suppression de l'écho du système** (le mode `voiceChat` de la session audio d'iOS) et l'interruption **par le bouton « Parler »** de l'écran ; l'interruption par la voix seulement si l'essai en voiture la montre fiable **[à vérifier en voiture]**.
- **Le bouton de la voix au volant** appartient à Siri (et à l'assistant de Google sous Android Auto) : il ne vient pas à l'app **[à vérifier]**. D'où, sous CarPlay, « Parler » à l'écran et l'écoute qui reprend seule après chaque réponse.
- **Le coût et le quota** : la minute de transcription et les caractères de synthèse au tarif du fournisseur réglé (la minute en direct coûte de l'ordre de 1,8 fois plus chez ElevenLabs et Deepgram, d'après Réglages › Voix) ; Whisper et Piper ne coûtent rien. Chaque phrase est un tour d'agent, sur le quota de son compte : la voiture invite à des tours courts et nombreux. Le passage pour l'oreille ne demande aucun appel en plus (§ 6.1).

### 4.4 Le conflit avec M9

M9 (tranché le 2026-11-20, `MOBILE.md`) : « la dictée du système, sur l'appareil ; **l'audio ne part jamais au cœur** ». V1 le contredit. Les options :

1. **Recommandé (C10)** : **une exception écrite à M9, pour la seule voix de la voiture**. Allumée avec le mode voiture de l'espace (§ 7.4), éteinte par défaut ; le son ne part que pendant que l'utilisateur parle (l'écran de la voix ouvert, son état « Je t'écoute »), par le tunnel chiffré ; le cœur ne le garde jamais (ni disque, ni journal), le transcrit ou le passe au fournisseur réglé, comme sur le poste. **La dictée de l'app reste M9.**
2. **M9 tenu** : V3 seulement (entendre sur le téléphone, parler par le cœur). Pas de Whisper ni de Scribe en voiture.
3. **M9 révisé partout** : la dictée de l'app passe aussi par la voix d'Escouade. Hors de cette étude.

**Ce que l'utilisateur doit savoir**, dit à l'activation du mode voiture : sa voix (et celle des passagers) part de son téléphone **vers son cœur** (son poste, ou son serveur), puis, s'il a réglé un service en ligne, **vers ce fournisseur, avec sa clé**, comme quand il parle à son poste ; elle n'est gardée nulle part par Escouade ; sous Android Auto, c'est Google qui entend et lit, pas Escouade.

## 5. La liste et la conversation

### 5.1 Ce que montre la liste

- **C2 est tranché par l'utilisateur** (2026-11-27) : la liste des agents et l'Overseer.
- **Recommandé (dans C2)** : **l'Overseer en tête** (s'il est allumé), puis les agents **qui t'attendent** (une autorisation, une question, un plan), puis ceux **qui travaillent**, puis ceux **finis depuis moins de 24 h** ; 12 lignes au plus (la limite d'une liste varie selon la voiture **[à vérifier]**). Une ligne : le nom de l'agent, son projet, son état.
- **Les projets privés** : **hors de la voiture** par défaut (C11), comme ils sont hors de la vue du chef (ESC-140) : leurs noms se disent devant les passagers.
- **Les stratégies d'un espace** : `servers.aggregateView` à `false` ne montre de cet espace que des nombres (« 3 agents sur Travail ») ; `servers.contentlessNotifications` y éteint le mode voiture ; `voice.dictation` coupée y éteint la voix en voiture, `voice.onlineAllowed` y ramène tout au local ; la stratégie neuve `servers.carMode` permet à un espace de travail d'interdire la voiture (C5).

### 5.2 La conversation avec un agent

- **Ce qu'on lui dit** part comme un message de l'utilisateur, marqué « dicté en voiture » (`origin: car`, § 7.2).
- **Ce qu'il répond** est dit (§ 6.1) ; la réponse entière reste dans sa conversation, sur le poste et le téléphone.
- **Ses questions** (celles qui ne font pas agir) sont posées une à une, comme en conversation vocale ; la réponse dite y répond. **Ses autorisations ne se donnent pas à la voix** (§ 7.1).
- **L'Overseer** est une conversation comme les autres, la sienne : il résume, lance (C8), dit ce qui attend.

## 6. Ce qui est dit d'un tour d'agent

### 6.1 Qui fait le résumé

La sortie d'un agent a du code, des chemins, des commandes. Les options :

1. **Recommandé (C12) : l'agent lui-même, prévenu**. La conversation vocale l'a déjà : la consigne `<escouade-voice>` lui fait finir sa réponse par un passage `<spoken>` de deux ou trois phrases, sans code, chemin ni liste. En voiture, la consigne dit en plus « l'utilisateur conduit : le plus important d'abord, une seule question, fermée si possible ; ce qui demande à voir, dis "je te le garde pour le téléphone" ». **Coût : quelques dizaines de tokens par message**, aucun appel en plus.
2. **Un modèle léger** (Haiku) résume chaque tour : un appel de plus, du quota, une à trois secondes.
3. **Le cœur seul** : `ear_text()` sur la réponse entière ; souvent illisible.

### 6.2 La garde du cœur

Une consigne ne suffit jamais. `ear_text()` (ESC-696 § 3.4) s'applique à ce qui est dit : il retire les blocs de code, les tableaux, le Markdown, les chemins et les adresses, remplace un bloc retiré par « du détail, sur ton téléphone », et coupe au-delà de 600 caractères. **Sans passage `<spoken>`** : « C'est fait. La réponse est sur ton téléphone. » ; tour en erreur : « L'agent s'est arrêté sur une erreur. » Sous Android Auto, c'est ce texte qui va dans la conversation de messagerie, que Google lit.

## 7. La sécurité, qui prime

### 7.1 Aucun feu vert depuis la voiture

- **La règle** : rien de ce que M3 met sous le verrou du téléphone ne part de la voiture, ni par le chef ni pour un agent : **autoriser une commande** (y compris le « oui » dit à une autorisation, qui existe en conversation vocale), « Toujours autoriser », **approuver un plan**, « oui » à un feu vert (`gate`), **valider un ticket** (un merge), publier, signer, supprimer, **lancer un ticket ou un agent** (recommandé, C8), ajouter une consigne durable.
- **Ce qui reste permis** : refuser, répondre à une question qui ne fait pas agir, envoyer un message, « plus tard », demander des nouvelles, retirer une consigne.
- **Une demande d'autorisation d'un agent**, en voiture : la voix la **dit** sans la lire en entier (« tests-e2e demande à lancer une commande sur Atlas ; je te la garde pour le téléphone ») ; elle **reste** dans « Attend ta réponse » et dans « À toi », pour le téléphone, à l'arrêt, sous verrou. La commande n'est jamais lue.
- **Un agent qui agit sans demander** (le mode Bypass, ou un mode qui accepte seul) : un message de la voiture **ne lui part pas** ; la voix dit « il agit sans te demander : je te le garde pour le téléphone », et le message attend dans « À toi » (recommandé, dans C8). Sinon un passager ou une dictée fautive lui ferait pousser du code.

### 7.2 Tenue par le cœur, avec `origin: car`

- `origin` (`typed`, `dictated`, `car`) est posé **par le natif** (l'app de CarPlay, le receiver d'Android Auto), jamais par le texte, sur `overseer_send` et sur l'envoi d'un message à un agent. Le cœur le garde avec le message et l'affiche (« dicté en voiture »).
- **Pour le chef** : un tour né d'un message `car` se voit **refuser les outils de feu vert** du serveur MCP (comme ses droits sont déjà tenus par le cœur, `OVERSEER.md`), avec « depuis la voiture : déposé dans À toi » ; il crée alors la demande dans la boîte. `add_directive` refuse un message `car`.
- **Pour un agent** : la voix en voiture du cœur **n'a pas** la réponse vocale aux autorisations ; le cœur refuse `answer_permission` (allow), `approve_ticket`, `start_ticket` et les feux verts quand l'appel vient d'une session de voiture ; il refuse l'envoi d'un message `car` à un agent qui agit sans demander.
- **Testé par nextest** : c'est la partie qui protège.

### 7.3 Les passagers, la dictée, les clés

- **Les passagers** entendent tout et peuvent parler : d'où le mode éteint par défaut, ce qui est dit limité au passage pour l'oreille, et aucun feu vert.
- **Les erreurs de la reconnaissance** : la phrase comprise est redite quand elle fait agir (« je lance ESC-712 ? » sans effet sans un « oui » ; et lancer reste refusé sous C8) ; tout ce qui pèse va dans la boîte.
- **Les clés** : la clé de l'appairage et le jeton au Keystore et au Trousseau (MO-4), jamais dans la WebView ; les clés des fournisseurs sur le cœur (§ 4.2). Le son et le texte reçus en voiture ne sont gardés qu'en mémoire.
- **La révocation** (MO-4) coupe tout : sans jeton, le flux de la voix et les commandes de la voiture sont refusés.

### 7.4 Le mode voiture et M5

- **Un mode voiture**, **éteint par défaut**, à allumer **par espace** dans les réglages de l'app, qui dit en clair ce qu'il implique (§ 4.4) : les noms et les passages lus à voix haute, entendus des passagers ; le son vers le cœur et le fournisseur (CarPlay) ; Google qui lit et entend (Android Auto).
- **Le conflit avec M5, sous Android Auto seulement** (le texte doit être dans la notification ou le modèle pour que Google le lise) : la réponse d'ESC-696 tient (C3). Le push reste sans texte ; **seulement pendant la projection**, le natif va chercher les passages par le tunnel (`car_feed`) et les pose dans la conversation ; la notification est masquée sur l'écran verrouillé (`VISIBILITY_PRIVATE`) et retirée en fin de projection ; jamais sous `servers.contentlessNotifications`.
- **Sous CarPlay**, rien ne passe par les notifications : le texte et le son viennent par le tunnel, l'app ouverte, et l'écran de la voiture n'en montre rien (Apple l'interdit, et on ne le voulait pas).

## 8. La technique

### 8.1 Comment le cœur sait que l'utilisateur conduit

- **Android** : `CarConnection.getType()` (`CONNECTION_TYPE_PROJECTION`), comme dans ESC-696.
- **iOS** : la scène CarPlay (`CPTemplateApplicationScene`), connectée et déconnectée ; l'app n'y vit qu'ouverte.
- **Le téléphone le dit au cœur** : `device_car_state` (`driving: true|false`) à chaque changement et toutes les 10 minutes ; l'état s'éteint seul après 15 minutes sans nouvelle ; en mémoire seulement.

### 8.2 Le chemin d'une phrase, sous CarPlay

1. L'utilisateur touche un agent ; l'app pousse l'écran de la voix, ouvre la session audio (`playAndRecord`, mode `voiceChat`), monte le tunnel (le Rust de MO-3) et ouvre **un flux de voix** vers le cœur (`car_voice_open`, la conversation nommée ; servi, portée `phone`).
2. Elle envoie le son en Opus ; le cœur transcrit (Whisper, ou le fournisseur réglé, en direct quand il le permet), envoie le texte à l'agent (`origin: car`, la consigne de la voiture), et rend à l'app l'état (« Il réfléchit »).
3. À la fin du tour, le cœur prend le passage pour l'oreille (§ 6), le passe par `ear_text()`, le fait dire (Piper ou la voix réglée), et rend le son phrase par phrase par le même flux ; l'app le joue, puis réécoute.
4. Revenir à la liste, ou une minute sans parler, ferme le flux et rend la session audio.

### 8.3 Le chemin d'un message, sous Android Auto

Celui d'ESC-696 § 5.1 et 5.2, **par conversation** : le réveil scellé de MO-6 ; le natif, pendant la projection, va chercher les passages non lus de chaque conversation (`car_feed`) et pose les notifications `MessagingStyle` (une `Person` par agent, robot) ou met à jour les `ConversationItem` ; la réponse dictée par Google arrive au receiver, qui l'envoie (`origin: car`) ; « marquer comme lu » avance le curseur (`car_seen`).

### 8.4 Ce qu'il faut construire

- **Le cœur** : `origin` sur `overseer_send` et sur l'envoi à un agent ; `device_car_state`, `car_feed`, `car_seen`, `car_voice_open` servis (portée `phone`) ; la consigne de la voiture pour le chef et pour les agents ; `ear_text()` ; les refus du § 7.2 ; `servers.carMode` ; la liste des conversations (§ 5.1). La liste des commandes reste unique (`remoteapi/list.rs`).
- **La voix sans fenêtre** (le plus gros) : une session de voix du cœur pour un appareil distant, qui prend le son d'un flux au lieu du micro et rend le son au lieu des haut-parleurs ; Whisper, les plugins de voix en ligne et Piper appelés par le cœur sans fenêtre, aussi dans `escouade-server` **[à vérifier : ce que les crates `escouade-voice` et `escouade-voice-local` portent déjà hors de Tauri]** ; le décodage et l'encodage Opus ; les stratégies de la voix ; la clé d'un service sur un serveur (§ 4.2) ; un journal sans texte ni son.
- **Le Rust du téléphone** (`crates/escouade-mobile`) : l'appel de l'API dans le tunnel et le flux de voix, exposés par UniFFI.
- **iOS** : la scène CarPlay, `CPListTemplate` et `CPVoiceControlTemplate`, la session audio, la capture et la lecture, le repli V3 (`SFSpeechRecognizer` sur l'appareil) ; l'autorisation dans les entitlements.
- **Android** : `CarConnection`, les notifications `MessagingStyle` par conversation, le `CarAppService` de messagerie à modèles (`ConversationItem`, Car API 7), le receiver, `automotive_app_desc.xml`.

### 8.5 Les dépendances

- **Le tunnel** (MO-2, MO-3) et **l'appairage** (MO-4) : faits.
- **MO-6 (ESC-572), les notifications** : **pour Android Auto seulement**. CarPlay n'en a pas besoin.
- **MO-8, la publication** : le test fermé de Google Play et TestFlight, où la voiture s'essaie.
- **MO-9** (UnifiedPush, en option) : le réveil d'Android sans Google ; CAR-6 le branche s'il existe.
- **L'Overseer** (ESC-462, ESC-468) et l'API distante : faits.

## 9. Les essais, la publication, le découpage

### 9.1 Les essais

| Quoi                                                                                                                                                                                       | Où                                                                                                                                        | En CI                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Le cœur : `origin: car`, chaque feu vert refusé (chef et agents), le message refusé à un agent qui agit seul, `ear_text()`, la liste (privés exclus, stratégies), l'état qui s'éteint seul | nextest                                                                                                                                   | **Oui** : c'est ce qui protège                                                                                   |
| La voix sans fenêtre : un flux de son enregistré (un fichier WAV de test) transcrit par Whisper et par un faux service, la synthèse rendue en Opus, les refus des stratégies               | nextest (un faux service de voix, comme les tests de la voix en plugins)                                                                  | **Oui**                                                                                                          |
| Le Rust du téléphone : l'appel et le flux dans le tunnel, contre le pair de test de MO-3                                                                                                   | nextest de `crates/escouade-mobile`                                                                                                       | **Oui**                                                                                                          |
| Android : les notifications et les `ConversationItem` construits, le receiver                                                                                                              | tests Kotlin (Robolectric) **[à vérifier dans le plugin]**                                                                                | **Oui**                                                                                                          |
| Android Auto de bout en bout : la liste, la lecture, la réponse dictée                                                                                                                     | **Desktop Head Unit** (Windows, macOS, Linux ; l'émulateur ou un téléphone par ADB ; le DHU accepte un micro ou des commandes)            | Non : un écran et l'app Android Auto ; une procédure locale écrite                                               |
| iOS : la liste, l'écran de la voix, la session audio, le repli V3                                                                                                                          | XCTest ; **le simulateur CarPlay** (Simulator › I/O › External Displays › CarPlay, ou _CarPlay Simulator_ des Additional Tools for Xcode) | XCTest **oui** (job Mac, sans signer) ; le simulateur **non** (il veut l'autorisation sur le profil **[tiers]**) |
| **En voiture** : la latence sur 4G, une zone sans couverture, l'écho et l'interruption, le bouton du volant                                                                                | **L'utilisateur**, avec sa voiture, l'app du test fermé ou de TestFlight                                                                  | Non : un geste de l'utilisateur, déposé dans « À toi » par chaque ticket                                         |

Apple et Google disent de ne pas se fier au seul simulateur : l'essai en voiture fait partie du ticket.

### 9.2 La publication

- **Apple** : la demande de l'autorisation de **la conversation à la voix**, par le compte de développeur de l'utilisateur, avec son texte (CAR-7, écrit d'abord : c'est le plus long délai, et le seul qu'on ne maîtrise pas) ; puis l'App ID, le profil, TestFlight, l'App Review.
- **Google Play** : le facteur de forme Android Auto dans la Play Console, sur le test fermé (la revue n'y bloque pas) ; la production attend que Google ouvre la messagerie à modèles, et reste aux notifications d'ici là.
- **Les agents ne créent aucun compte, ne déposent aucune demande** chez Apple ou Google, ne lisent ni ne copient aucune clé : ils écrivent les textes.

### 9.3 Le découpage en tickets

Estimations en **heures d'agent** (Claude Code, six agents en parallèle), jamais en jours-homme.

| Ticket                              | Ce qu'il fait                                                                                                                                                                                                                                                                  | Dépend de                                                   | Heures |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------- | ------ |
| **CAR-1** · le cœur en voiture      | `origin: car` (chef et agents) ; `device_car_state` ; `car_feed`, `car_seen` ; la liste des conversations (§ 5.1) ; la consigne de la voiture ; `ear_text()` ; **les refus du § 7.2** et la demande déposée dans « À toi » ; `servers.carMode` ; nextest ; spec et `REMOTE.md` | C2, C3, C5, C8, C11, C12                                    | 5 h    |
| **CAR-2** · le Rust du téléphone    | L'appel de l'API et le flux de voix dans le tunnel, par UniFFI, sans la WebView ; ses tests                                                                                                                                                                                    | —                                                           | 3 h    |
| **CAR-3** · la voix sans fenêtre    | La session de voix du cœur pour un appareil (`car_voice_open`) : le son d'un flux, Whisper ou le service réglé, Piper ou la voix réglée, Opus ; dans le poste et dans `escouade-server` ; la clé d'un service sur un serveur ; les stratégies ; nextest                        | CAR-1, C9, C10                                              | 6 h    |
| **CAR-4** · CarPlay                 | La scène ; `CPListTemplate` puis `CPVoiceControlTemplate` ; la session audio (`voiceChat`) tenue pendant l'échange ; le flux vers le cœur ; le repli V3 ; « Pas de réseau » ; XCTest ; l'essai au simulateur et en voiture déposé dans « À toi »                               | CAR-2, CAR-3, C4, **l'autorisation d'Apple** (pour l'essai) | 6 h    |
| **CAR-5** · Android Auto            | `CarConnection` ; une conversation par agent et pour l'Overseer : les notifications `MessagingStyle` et la messagerie à modèles (`ConversationItem`) ; le receiver ; le mode voiture par espace ; Robolectric ; la procédure du DHU ; l'essai en voiture                       | **MO-6 (ESC-572)**, CAR-1, CAR-2, C3, C6                    | 6 h    |
| **CAR-6** · UnifiedPush (en option) | Le même chemin depuis le réveil par ntfy                                                                                                                                                                                                                                       | MO-9, CAR-5                                                 | 1 h    |
| **CAR-7** · la publication          | Le texte de la demande à Apple (d'abord), les fiches des stores, la page de confidentialité (le son vers le cœur et le fournisseur ; Google sous Android Auto), les notes pour les revues ; les gestes de l'utilisateur dans « À toi »                                         | — (le texte) ; CAR-4, CAR-5, MO-8 (le reste)                | 1,5 h  |

- **L'ordre, à six agents** : **CAR-7 (le texte à Apple), CAR-1 et CAR-2 tout de suite**, en parallèle ; puis **CAR-3** ; puis **CAR-4** dès CAR-3 et l'autorisation d'Apple ; **CAR-5** dès MO-6 et CAR-1, en parallèle de CAR-3 et CAR-4 ; CAR-6 quand MO-9 existe ; la fin de CAR-7 après eux.
- **Total : environ 27,5 h d'agent** (28,5 h avec CAR-6), **de l'ordre de 18 h de bout en bout** (le plus long chemin : CAR-1, CAR-3, CAR-4, CAR-7), sans compter MO-6 ni les délais d'Apple et de Google.
- **ESC-696 comptait 17,5 h** : la voix sans fenêtre (CAR-3, 6 h), la liste des agents (CAR-1 et CAR-5 plus lourds) et le flux de voix (CAR-2) font la différence ; l'extension d'intentions de SiriKit sort.
- **Ce qui attend l'utilisateur** : ses réponses (§ 10) ; la demande de l'autorisation CarPlay à Apple ; le facteur de forme Android Auto dans la Play Console ; sur un serveur, la clé d'une voix en ligne à y poser s'il en veut une ; l'essai en voiture, sous CarPlay et sous Android Auto.

## 10. Les choix

Ils restent à l'utilisateur, déposés en une demande dans « À toi » (a-53), qui **remplace a-52**. Aucune décision n'est écrite avant ses réponses.

1. **C1. Le périmètre et l'ordre** : **Recommandé** : les deux dans la 2.4 ; la demande à Apple déposée d'abord ; CAR-1 à CAR-3 tout de suite ; **CarPlay dès l'autorisation** (il n'attend pas MO-6, et c'est lui qui porte la voix d'Escouade) ; Android Auto après MO-6. Autres : CarPlay seul ; Android Auto seul (sans la voix d'Escouade). _Débloque_ CAR-1 à CAR-3, CAR-7.
2. **C2. Ce que la voiture expose** : **tranché par l'utilisateur** (2026-11-27) : la liste des agents et l'Overseer. **Reste à confirmer, recommandé** : l'Overseer en tête, puis les agents qui t'attendent, qui travaillent, finis depuis 24 h ; 12 lignes au plus ; un nom, un projet, un état. _Débloque_ CAR-1.
3. **C3. Le conflit avec M5, sous Android Auto** : **Recommandé** : la réponse d'ESC-696 (le mode voiture par espace ; le push sans texte ; les passages cherchés par le tunnel pendant la projection seulement, masqués sur l'écran verrouillé, retirés en fin de projection ; jamais sous `contentlessNotifications`). Autres : le texte scellé dans le push ; aucun texte. _Débloque_ CAR-1, CAR-5.
4. **C4. La catégorie de CarPlay** : **Recommandé (revu)** : **la conversation à la voix**, seule ; la communication abandonnée. Autre : la communication (Siri lit et dicte, annonce app fermée ; ni la voix d'Escouade, ni la liste à soi). _Débloque_ la demande à Apple et CAR-4.
5. **C5. Une stratégie pour les espaces de travail** : **Recommandé** : `servers.carMode` (vrai par défaut, faux sous `contentlessNotifications`), qui coupe la liste et la voix en voiture. Autre : pas de stratégie. _Débloque_ CAR-1.
6. **C6. La messagerie à modèles d'Android Auto** : **Recommandé (revu)** : **oui, pour le test fermé** : c'est elle qui montre la liste des agents ; les notifications restent la base et le repli en production. Autre : les notifications seules (seules les conversations qui ont du neuf). _Débloque_ CAR-5.
7. **C7. Android Automotive OS** : **Recommandé** : hors de la 2.4. Autre : un ticket d'étude à part.
8. **C8. Ce que la voiture ne peut pas faire** : **Recommandé** : tout ce que M3 met sous verrou, **lancer un ticket ou un agent compris**, **le « oui » dit à une autorisation**, et **aucun message à un agent qui agit sans demander** (gardé pour le téléphone). Autre : la liste minimale (publier, signer, supprimer, autoriser une commande, valider un ticket qui fusionne), « lance ESC-712 » permis. _Débloque_ CAR-1.
9. **C9. Où se fait la voix** : **Recommandé** : par le cœur, avec Réglages › Voix (V1), clés sur le cœur ; repli sur la reconnaissance de l'iPhone quand le cœur n'entend plus (V3). Autres : tout sur le téléphone (V2, pas la voix d'Escouade) ; V3 toujours. _Débloque_ CAR-3, CAR-4.
10. **C10. Le conflit avec M9** : **Recommandé** : une exception écrite pour la seule voix de la voiture (mode voiture allumé, le son seulement pendant qu'on parle, par le tunnel, jamais gardé, puis au fournisseur réglé) ; la dictée de l'app reste M9. Autres : M9 tenu (V3 seulement) ; M9 révisé partout. _Débloque_ CAR-3.
11. **C11. Les projets privés** : **Recommandé** : hors de la voiture. Autre : un réglage par projet privé. _Débloque_ CAR-1.
12. **C12. Ce qui est dit d'un tour d'agent** : **Recommandé** : l'agent lui-même, prévenu (le passage `<spoken>` et la consigne de la voiture), sous la garde `ear_text()` ; aucun appel de plus. Autres : un modèle léger qui résume chaque tour (du quota, une à trois secondes) ; le cœur seul. _Débloque_ CAR-1, CAR-3.

## 11. Ce qui serait un échec, même si tout marche

- Un feu vert parti d'une phrase dite en voiture : une autorisation accordée d'un « oui », un merge, un lancement, une consigne.
- Une commande d'un agent lue à voix haute, ou un tableau, un chemin, du code.
- Une clé d'un fournisseur de voix sur le téléphone, ou une clé de l'appairage hors du Keystore ou du Trousseau.
- Le son de la voiture gardé sur le disque ou dans un journal, du téléphone ou du cœur.
- Le texte d'un message sur l'écran verrouillé, ou dans une notification hors de la projection.
- Une version de production refusée par Google ou Apple pour ce qu'ils interdisent : la production attend leur avis.

## 12. Les sources (lues le 2026-11-27)

**Android (developer.android.com, sauf mention)**

- _Extend messaging notifications to Android Auto_ : https://developer.android.com/training/cars/messaging
- _Build templated messaging experiences for Android Auto_ : https://developer.android.com/training/cars/communication/templated-messaging
- _Communication apps_ (design) : https://developer.android.com/design/ui/cars/guides/app-types/communications
- _Record from the car microphone_ : https://developer.android.com/training/cars/apps/library/car-microphone ; référence de `CarAudioRecord` : https://developer.android.com/reference/androidx/car/app/media/CarAudioRecord
- _Communicate with the app by voice_ : https://developer.android.com/design/ui/cars/guides/ux-requirements/communicate-app-by-voice
- _Set up your project_ (les catégories du manifeste) : https://developer.android.com/training/cars/apps/library/set-up-project ; _Android for Cars App Library_ (design) : https://developer.android.com/design/ui/cars/guides/foundations/cal ; _Android for Cars overview_ : https://developer.android.com/training/cars
- _Car app quality_ (PC-1, MF-1 à MF-5, VC-1) : https://developer.android.com/docs/quality-guidelines/car-app-quality
- _Distribute to cars_ : https://developer.android.com/training/cars/distribute
- _Connection API_ : https://developer.android.com/training/cars/apps/library/connection-api
- _Test using the Desktop Head Unit_ : https://developer.android.com/training/cars/testing/dhu
- _What's new_ (Android for Cars) : https://developer.android.com/training/cars/whats-new

**Apple (developer.apple.com, sauf mention)**

- _CarPlay Developer Guide_ (édition d'octobre 2026) : https://developer.apple.com/download/files/CarPlay-Developer-Guide.pdf
- L'autorisation `com.apple.developer.carplay-voice-based-conversation` : https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.carplay-voice-based-conversation
- _Requesting CarPlay entitlements_ : https://developer.apple.com/documentation/carplay/requesting-carplay-entitlements
- _CarPlay_ : https://developer.apple.com/carplay/ ; _Using the CarPlay simulator_ : https://developer.apple.com/documentation/carplay/using-the-carplay-simulator
- Les règles de l'App Review : https://developer.apple.com/app-store/review/guidelines/

**Tiers, pour mémoire (jamais seuls à fonder une conclusion)**

- La catégorie de la voix dans iOS 26.4 : https://www.macrumors.com/2026/02/18/ios-26-4-carplay-support/ ; https://9to5mac.com/2026/02/18/ios-26-4-adds-support-for-voice-based-ai-apps-to-carplay/ ; https://appleinsider.com/articles/26/02/18/ai-agents-are-coming-to-carplay-but-theyre-not-getting-the-keys
- Android Auto et les chatbots : https://www.androidauthority.com/chatgpt-apple-carplay-vs-android-auto-3653922/
- WhatsApp sur l'écran de la voiture : https://thecarplayer.com/blogs/news/whatsapp-official-app-android-auto-apple-carplay
- Forums d'Apple : la catégorie de la voix (https://developer.apple.com/forums/thread/821632, https://developer.apple.com/forums/thread/831674, https://developer.apple.com/forums/thread/849047, https://developer.apple.com/forums/thread/831720) ; la synthèse en voiture (https://developer.apple.com/forums/thread/807197) ; les délais des autorisations (https://developer.apple.com/forums/thread/848946, https://developer.apple.com/forums/thread/847440)
- Le retour d'OpenBubbles sur `ConversationItem` : https://github.com/mumeicopperhead/openbubbles-app
