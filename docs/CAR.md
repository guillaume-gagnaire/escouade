# Escouade en voiture : un « WhatsApp » des agents qui tournent, qu'on appelle à la voix (étude 2.4)

Étude d'ESC-696, révisée par **ESC-698**, puis par **ESC-721** d'après les réponses de l'utilisateur, pour la 2.4. **Rien du produit ne change dans ces tickets** : ce document, la page de spec « Escouade à distance › En voiture » (`a-distance/en-voiture`), une décision proposée (« Escouade en voiture (2.4) : un « WhatsApp » des agents qui tournent, qu'on appelle à la voix ; tout en vocal ou rien ») et les tickets CAR-1 à CAR-7 (§ 9.3).

**Les réponses de l'utilisateur** (2026-11-28, à la demande a-56 de la boîte « À toi ») :

- « tout recommandé, à part sur l'UI : je veux un whatsapp : liste d'agents qui tournent sur escouade, et je peux les appeler en vocal pour leur dire quoi faire, etc (agents lancés, et overseer quand il sera sorti évidemment) » ;
- puis, sur le repli d'Android Auto par la messagerie lue par l'assistant de Google : « nope, on fait tout en vocal ou on fait rien ».

Le § 10 donne chaque choix tranché. Ce que la première version disait de la messagerie d'Android Auto (ESC-698, § 2.2 à 2.5) est remplacé par l'appel (§ 2).

**La demande** (2026-11-27), d'abord : « Est-ce qu'il y a un monde où l'app mobile Android est compatible Android Auto en mode overseer ? », puis « Étude pour la 2.4, avec Apple aussi ». **La direction donnée après ESC-696** (2026-11-27) : « comme pour WhatsApp, je voudrais afficher la liste de mes agents ou ouvrir l'Overseer, puis ensuite ça ouvre une vue de conversation audio avec l'agent choisi. La conversation vocale pourrait se faire via les paramètres d'Escouade : ElevenLabs, Whisper, etc. » Elle tranche C2 d'ESC-696 (la voiture montre **la liste des agents et l'Overseer**, plus seulement le chef) et change la voix : ce sont **les fournisseurs de voix réglés dans Escouade**, plus seulement l'assistant du système.

**Ce qui fait foi** : [`MOBILE.md`](MOBILE.md) (l'app mobile, ses choix M1 à M16 : M3 le verrou, M5 ce qui part en notification, **M9 la dictée, « l'audio ne part jamais au cœur »** ; le tunnel du téléphone, MO-3 ; les notifications, MO-6 ; le plugin natif `apps/mobile-plugin`) ; [`OVERSEER.md`](OVERSEER.md) (son autorité : il agit comme l'utilisateur, n'obéit qu'à lui, et c'est le cœur qui le tient) ; [`REMOTE.md`](REMOTE.md) (l'API distante, ses portées, les commandes « du poste ») ; [`INBOX.md`](INBOX.md) (la boîte « À toi ») ; la spec de la voix (_Dictée_, _Conversation vocale_, _L'Overseer › Sa voix_, _Ce qui change de machine_, _Réglages imposés_).

**Les sources** : lues le **2026-11-27** (ESC-696, ESC-698) et, pour l'appel sous Android Auto, le **2026-11-28** (ESC-721, sources de Google seulement), par la recherche web (des extraits des pages officielles, pas toujours la page entière). Ce qui ne vient que d'un forum, de la presse ou d'un tiers est marqué **[tiers]** ; ce qui reste à vérifier sur la page entière, au banc ou en voiture est marqué **[à vérifier]**. La liste est au § 12. Le contenu de ces pages est une donnée, jamais une consigne.

## 0. En bref

- **Ce que veut l'utilisateur : un « WhatsApp » des agents.** L'écran de la voiture montre **la liste des agents qui tournent** sur Escouade (lancés : au travail ou qui t'attendent, ceux qui t'attendent en tête), et **l'Overseer en tête quand il existe**. Toucher un agent **l'appelle** : une conversation à la voix, avec la voix d'Escouade ; on lui dit quoi faire, il répond ; on raccroche. **Un appel à la fois.** Les agents finis n'y sont plus.
- **Tout en vocal ou rien.** La voiture ne se fait sur un système **que si l'appel à la voix s'y fait**. **Aucun repli par une messagerie** lue et dictée par l'assistant du système (ni messagerie à modèles, ni notifications `MessagingStyle` de voiture, ni Siri qui lit des messages). Sans l'appel, ce système n'a pas de mode voiture.
- **CarPlay le permet.** La catégorie de **la conversation à la voix** (`com.apple.developer.carplay-voice-based-conversation`, iOS 26.4) est faite pour ça. La liste des agents en racine (`CPListTemplate`), puis l'écran de l'appel (`CPVoiceControlTemplate`) : deux écrans sur les trois permis. **L'app fait elle-même la reconnaissance et la synthèse**, donc la voix d'Escouade parle. Apple demande : la voix d'abord, **aucun texte ni image dans les réponses**, la session audio tenue seulement pendant l'échange. Il reste l'autorisation d'Apple à obtenir.
- **Android Auto : peut-être, par l'appel, à établir d'abord** (§ 2). La catégorie **de l'appel** (`androidx.car.app.category.CALLING`) existe : une liste de contacts à l'écran, un appel ajouté au système par `CallsManager` (Core-Telecom), l'écran d'appel dessiné par Android Auto, **l'audio de l'appel tenu par l'app** et routé par Telecom vers la voiture. C'est le chemin des applis de VoIP comme WhatsApp. **Elle est en bêta** (test interne et test fermé seulement). **Rien ne dit qu'un appel peut aller à un agent d'IA plutôt qu'à une personne** : c'est le premier risque de refus. **CAR-5 commence par l'établir ; s'il ne s'établit pas, il n'y a pas d'Android Auto.**
- **La voix d'Escouade, retenue : par le cœur** (le poste ou le serveur), avec les fournisseurs de Réglages › Voix (Whisper ou un service en ligne pour entendre, Piper ou une voix en ligne pour parler). Les **clés restent sur le cœur**, jamais sur le téléphone. L'audio passe par le tunnel chiffré du téléphone. **Aujourd'hui la voix est « du poste »** (le micro et les clés du poste, absente d'`escouade-server`) : la rendre au cœur sans fenêtre est un ticket à part entière (CAR-3). Repli : la reconnaissance et la synthèse du téléphone quand le cœur ne répond pas.
- **Le conflit avec M9** (« l'audio ne part jamais au cœur ») : retenu, **une exception écrite pour la seule voix de la voiture**, allumée avec le mode voiture, l'audio seulement pendant que l'utilisateur parle, jamais gardé. L'audio part chez lui (son cœur), puis chez le fournisseur en ligne réglé, comme sur le poste. La dictée de l'app reste M9.
- **Ce qui est dit d'un tour d'agent** : le passage pour l'oreille que l'agent écrit déjà en conversation vocale (`<spoken>`), prévenu en plus que l'utilisateur conduit, puis passé par une garde du cœur. Aucun appel de modèle en plus.
- **La sécurité prime, tenue par le cœur** : un message venu de la voiture est marqué (`origin: car`) ; **aucun feu vert ne part de la voiture**, ni du chef ni d'un agent : pas d'autorisation dite « oui » à la voix, pas de validation, de lancement ni de consigne durable. Une demande d'autorisation est dite (« je te la garde pour le téléphone ») et attend dans « À toi », sous verrou. **L'appel n'y change rien** (§ 7).
- **Découpage** : sept tickets, **CAR-1 à CAR-7, environ 28 h d'agent** (29 h avec CAR-6, en option), de l'ordre de **18 h de bout en bout**. **Ni CarPlay ni Android Auto n'attendent MO-6** : un appel part de l'utilisateur, l'app ouverte sur l'écran de la voiture, sans réveil.

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

## 2. Android Auto : l'appel, ou rien

**La règle de l'utilisateur** (2026-11-28) : « on fait tout en vocal ou on fait rien ». Le repli d'ESC-698 (chaque agent comme une conversation de messagerie, lue et dictée par l'assistant de Google) **est abandonné** : ni messagerie à modèles (`ConversationItem`), ni notifications `MessagingStyle` de voiture, ni `CarAudioRecord` dans une app de messagerie. La seule question est donc : **une app comme Escouade peut-elle tenir, sous Android Auto, un appel à la voix vers un agent, avec sa propre voix, comme WhatsApp tient ses appels ?**

Sources de Google seulement, lues le 2026-11-28 (§ 12) : _Build calling experiences for Android Auto_, _Communication apps_ (design), _Set up your project_, _Car app quality_, _Distribute to cars_, _Core-Telecom_, la référence de `CallsManager`, _Telecom framework overview_, la référence de `ConnectionService` et de `PhoneAccount`, _AEP guideline: Android Telecom Framework_, _Android multi-platform app quality guidelines_ (aide de la Play Console).

### 2.1 Comment WhatsApp fait

- **Les messages** : la messagerie par notifications (`MessagingStyle`, lue et dictée par l'assistant), le chemin officiel depuis des années. **Ce n'est pas le nôtre** : c'est l'assistant de Google qui parle et entend.
- **Les appels** : une app de VoIP déclare ses appels au système Android (**Telecom**). Android Auto les montre alors dans **son propre écran d'appel** (décrocher, raccrocher, couper le micro) et l'audio de l'appel passe par la voiture comme celui d'un appel téléphonique. Les pages de Google décrivent ce chemin pour toute app de communication ; aucune ne nomme WhatsApp **[à vérifier : quelle API WhatsApp emploie ; la presse le dit, Google non]**.

### 2.2 Ce qui est établi

- **La catégorie de l'appel existe** : `androidx.car.app.category.CALLING`, dans le filtre d'intention du `CarAppService` (_Set up your project_, _Build calling experiences_). Une app qui fait aussi la messagerie déclare les deux ; **nous ne déclarons que l'appel**.
- **Ce qu'elle montre** : la Car App Library « permet de créer des apps avec des listes ou des grilles de contacts que l'utilisateur touche pour lancer un appel » ; l'appel est intégré à l'expérience d'appel d'Android Auto par **`CallsManager`** (Core-Telecom, Jetpack) ; **l'écran d'appel est fourni par Android Auto**, avec peu de personnalisation, et remplace les écrans de l'app pendant toute la durée de l'appel. C'est exactement la liste des agents, puis l'appel : **les agents sont les contacts**.
- **L'audio est celui de l'app** : un appel de VoIP est tenu par l'app (sa capture, son réseau, sa lecture) ; **Telecom tient le focus audio et le routage** (vers la voiture, le Bluetooth, le haut-parleur), et l'app ne doit pas les tenir elle-même (_AEP guideline_, _Telecom framework overview_). Rien dans les pages ne réserve l'audio d'un appel à une voix humaine : **la voix d'Escouade (le son rendu par le cœur, § 4) peut y passer**, comme la voix d'un correspondant passe dans un appel de WhatsApp.
- **Ce que l'app doit faire** (_Build calling experiences_, `CallsManager`, _AEP guideline_) :
  - s'intégrer à Telecom par Core-Telecom **en tout temps**, pas seulement sous Android Auto ;
  - déclarer **chaque** appel par `CallsManager.addCall` (permission `MANAGE_OWN_CALLS`), répondre à ses rappels (`onAnswer`, `onDisconnect`, `onSetActive`, `onSetInactive`) en **moins de 5 secondes**, un ajout d'appel à la fois ;
  - poser une notification **`CallStyle`** dans les 5 secondes de l'ajout (c'est elle qui donne à l'app la priorité de premier plan pendant l'appel) ;
  - garder l'état et les commandes de l'appel en accord avec les surfaces distantes (Android Auto, montres, casques) : raccrocher depuis la voiture doit raccrocher l'appel ;
  - **le chemin ancien** (`ConnectionService` géré par l'app, `PhoneAccount` avec `CAPABILITY_SELF_MANAGED`) marche encore, mais cette capacité est **dépréciée au niveau d'API 37** : Google dit de passer aux API de VoIP de Telecom, que Core-Telecom enveloppe. **Retenu : Core-Telecom (`CallsManager`)**, pas `ConnectionService` à la main.
- **La qualité de la voiture** (_Car app quality_) : les critères communs valent pour l'appel (une tâche en cinq écrans au plus, AC-1 ; un bouton qui répond en deux secondes, DR-1 ; aucun texte qui défile seul, ST-1) ; PC-1 refuse les fonctions hors du type d'app.
- **La publication** : l'appel est **en bêta** : « tout le monde peut publier des apps de communication avec appel sur les pistes de **test interne** et de **test fermé** » ; le test ouvert et la production « plus tard », ou en se portant candidat comme **partenaire d'accès anticipé** (_Communication apps_, _Distribute to cars_). L'utilisateur est le premier usager : **le test fermé (MO-8) lui suffit**, comme pour la messagerie à modèles d'ESC-698. La revue n'y bloque pas (_Distribute to cars_ : non bloquante en test fermé).
- **Android Automotive OS** : les apps de communication n'y sont pas admises (_Communication apps_). Hors de la 2.4 (C7).

### 2.3 Ce qui reste à vérifier

1. **Un agent comme correspondant** **[à vérifier, le point qui décide]** : les pages parlent de « contacts » et d'« appels » ; aucune ne dit si le correspondant peut être un agent d'IA, ni ne l'interdit. La règle de la messagerie (MF-5, de pair à pair, pas un service de notifications) n'a pas d'équivalent relevé pour l'appel. À poser à Google (le formulaire d'accès anticipé) et à lire dans la page entière de _Car app quality_, section de l'appel.
2. **Les exigences annoncées** : _Build calling experiences_ dit que « d'autres exigences pour l'intégration à Telecom seront données bientôt » **[à relire au moment de CAR-5]**.
3. **L'hôte au DHU** : qu'un `CarAppService` de la catégorie `CALLING` se lie au Desktop Head Unit, que l'appel ajouté par `CallsManager` y prenne l'écran d'appel d'Android Auto, et que **l'audio joué par l'app pendant l'appel** (la voix d'Escouade, en `MODE_IN_COMMUNICATION`) sorte par la voiture et que le micro de la voiture y entre **[à vérifier au DHU, puis en voiture]**.
4. **Un appel sans réseau téléphonique** : l'appel est un appel de VoIP « sortant », sans numéro ; l'adresse de l'appel (`CallAttributes`) est un identifiant de l'agent **[à vérifier : ce que l'écran d'appel en montre]**.
5. **La vraie voiture** : ESC-698 relevait, pour la messagerie à modèles, un service à modèles jamais lié dans une vraie voiture alors que le DHU marchait **[tiers]** ; le même risque vaut pour l'appel **[à vérifier en voiture, CAR-7]**.

### 2.4 Les risques de refus à la revue

- **PC-1** (une fonction hors du type d'app) : un « appel » à un programme peut être lu comme un assistant déguisé en app de communication. **C'est le risque principal**, et il ne se lève qu'à la revue de production, pas au test fermé.
- **La bêta** : la production attend que Google ouvre l'appel à tous, ou qu'il nous admette en accès anticipé.
- **Les assistants tiers** : Google n'admet aucun assistant tiers dans Android Auto en novembre 2026 ; Gemini y est comme assistant du système. Rien d'officiel ne le dit en ces mots ; c'est le contexte du risque, pas une règle lue.
- **Ce qui ne serait pas un refus** : le test interne et le test fermé, où la revue ne bloque pas. L'utilisateur peut donc s'en servir dès que l'appel marche au DHU et dans sa voiture, même si la production attend.

### 2.5 Sans l'appel, pas d'Android Auto

- **CAR-5 commence par établir l'appel** : un `CarAppService` `CALLING`, un appel ajouté par `CallsManager`, l'audio d'Escouade entendu au DHU, le micro de la voiture capté. **Si l'appel ne s'établit pas** (l'hôte ne le prend pas, l'audio ne passe pas, ou Google dit non à un agent comme correspondant), **CAR-5 s'arrête et le dit** : ni messagerie à modèles, ni notifications `MessagingStyle` de voiture, ni `CarAudioRecord` dans une autre catégorie. Android Auto attend alors.
- **Ce qui le prouverait** : l'appel au DHU de bout en bout (la liste, l'appel, la voix d'Escouade, la réponse dite, raccrocher depuis la voiture), puis l'essai de l'utilisateur dans sa voiture sur le test fermé.
- **Ce qui le permettrait plus tard**, si l'appel est refusé ou bloqué : une réponse de Google admettant un agent comme correspondant ; l'appel sorti de la bêta avec des règles qui l'admettent ; une catégorie d'Android for Cars pour les assistants ou la conversation à la voix (aucune n'existe au 2026-11-28 : navigation, points d'intérêt, IoT, météo, messagerie, appel, et le média). À relire dans _What's new_ d'Android for Cars à chaque version.
- **CarPlay ne l'attend pas** : il a sa catégorie (§ 3), et l'ordre des tickets ne fait pas dépendre CAR-4 de CAR-5.

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

### 3.2 La liste des agents qui tournent, puis l'appel

- **La racine** : un `CPListTemplate`, la liste du § 5.1 : **l'Overseer en tête quand il existe**, puis **les agents qui t'attendent**, puis **ceux qui travaillent**. Les agents finis n'y sont pas. **Une ligne n'a qu'un nom, un projet et un état** (« t'attend », « travaille ») : aucun texte de message, aucune image.
- **Toucher une ligne appelle l'agent** : l'app pousse l'écran de l'appel (`CPVoiceControlTemplate`) et ouvre la session audio. **Deux écrans sur trois.** Le troisième reste libre (une alerte : « pas de réseau »).
- **L'écran de l'appel** : le nom de l'agent en titre ; ses états (« Je t'écoute », « Il réfléchit », « Il te parle ») ; deux boutons, « Parler » (ou « Arrêter » pendant qu'il parle) et **« Raccrocher »**, qui ferme l'appel et revient à la liste.
- **Un appel à la fois** : raccrocher ferme le flux de la voix et rend la session audio ; toucher un autre agent pendant un appel raccroche d'abord. Rien de ce qu'on a lu ne limite la catégorie à une seule conversation, mais aucune page ne le dit en clair **[à vérifier : poser la question dans la demande d'autorisation]**.
- **Le mot « appel »** est celui de l'écran et de nos textes ; pour Apple, c'est la catégorie de la conversation à la voix, sans rien du téléphone (aucun CallKit).
### 3.3 La reconnaissance et la synthèse sont faites par l'app

La catégorie ne fournit **ni reconnaissance ni synthèse** : l'app ouvre sa session audio (`AVAudioSession`), capte le micro et joue sa voix. Elle peut le faire elle-même sur l'iPhone (`SFSpeechRecognizer` sur l'appareil, `AVSpeechSynthesizer`), ou **envoyer le son à un service et jouer ce qu'il rend** : c'est ce qui laisse parler la voix d'Escouade (§ 4). Des forums **[tiers]** disent que certaines voitures ne jouent pas un `AVSpeechUtterance` et que le média reprend mal après une capture : **[à vérifier en voiture]**.

### 3.4 La catégorie, C4

- **ESC-696 recommandait la communication** (SiriKit : Siri lit et dicte, annonce les messages app fermée), la voix en recours. **La direction de l'utilisateur la retourne** : la communication ne laisse ni la voix d'Escouade (Siri lit et dicte), ni une liste à soi au-delà des lignes de messages (`CPMessageListItem`).
- **Retenu (C4)** : **la conversation à la voix**, seule (une catégorie par app). Elle est faite pour les agents d'IA, donc la plus probable à obtenir. Elle porte la liste des agents puis l'appel avec la voix d'Escouade, et n'attend pas MO-6 : sans notification, elle marche app ouverte sur l'écran de la voiture. La communication de SiriKit (Siri qui lit et dicte des messages) serait un repli par la messagerie : **exclu** (tout en vocal ou rien).
- **Ce qu'on perd** : l'annonce spontanée app fermée. Hors de l'app, le téléphone garde les notifications ordinaires de M5, sans texte. Pendant un appel, ce que l'agent appelé finit est dit ; ce que font les autres attend la liste (pas d'appel entrant, § 5.2).

## 4. La voix d'Escouade en voiture

### 4.1 Où se font la reconnaissance et la synthèse

| Où                                 | Ce que c'est                                                                                                                                                    | Pour                                                                                                         | Contre                                                                                                                                                                |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **V1 · par le cœur** (retenu)     | Le téléphone capte et joue ; le son passe par le tunnel ; le cœur entend (Whisper ou le service réglé) et parle (Piper ou la voix réglée), avec Réglages › Voix | La voix d'Escouade, celle du poste ; les clés sur le cœur ; un seul réglage ; la même voix pour chaque agent | Le son quitte le téléphone (M9, § 4.4) ; il faut le réseau ; il faut une voix d'Escouade sans fenêtre (CAR-3) ; sur un serveur, la clé de la voix doit y être (§ 4.2) |
| **V2 · sur le téléphone**          | La reconnaissance et la synthèse de l'iPhone, sur l'appareil                                                                                                    | Aucun son ne part ; tient sans réseau pour entendre ; aucune clé                                             | Pas la voix d'Escouade ; une seule voix système ; mettre les clés des fournisseurs sur le téléphone est **exclu**                                                     |
| **V3 · les deux** (le repli de V1) | Entendre sur le téléphone (le texte monte, M9 tenu), parler par le cœur (le son de la voix d'Escouade descend)                                                  | M9 tenu ; la voix d'Escouade pour parler ; moins de données montantes                                        | Pas Whisper ni Scribe pour entendre ; la reconnaissance de l'iPhone, de qualité variable en voiture                                                                   |

**Retenu (C9)** : **V1**, qui suit les réglages d'Escouade comme la conversation vocale du poste ; **V3 en repli automatique**, quand le cœur n'entend plus (le tunnel tombe, le fournisseur refuse) ; la synthèse du téléphone (`AVSpeechSynthesizer`) pour les seules phrases de service (« pas de réseau, je réessaie »). **Sous Android Auto, la même V1** : le son de l'appel est capté et joué par l'app (Telecom le route vers la voiture, § 2.2), monte au cœur et en redescend ; le repli V3 y prend la reconnaissance d'Android sur l'appareil (`SpeechRecognizer`, si l'essai la montre utilisable pendant un appel **[à vérifier]**), sinon l'appel dit « je ne t'entends plus » et raccroche.

### 4.2 Les clés des fournisseurs, sur le cœur

- **Un poste** : la clé du service est déjà au trousseau du poste ; le cœur du poste s'en sert pour le téléphone.
- **Un serveur** : aujourd'hui, la clé d'une voix reste au trousseau du poste client, et le serveur n'a pas de voix. Pour la voiture, **la clé du service doit être posée sur le serveur** (son magasin de secrets en fichier, comme les autres clés d'un serveur), par un réglage neuf de l'espace : « La voix en voiture, sur Travail », qui dit « Gardée dans un fichier sur Travail ». Ou bien Whisper et Piper sur le serveur, sans clé (un VPS sans carte graphique : Whisper base de l'ordre de la seconde par phrase **[à mesurer]**).
- **Jamais sur le téléphone**, jamais dans un réveil ni dans le journal. Le téléphone ne voit que du son et du texte.

### 4.3 La latence, le réseau, l'interruption, le bouton, le coût

- **Le tour** (V1, de l'ordre de **[à mesurer sur 4G]**) : la fin de la phrase (un bouton : tout de suite ; le silence : le « Délai avant l'envoi », 1,2 s par défaut) ; le son en Opus par le tunnel (environ 3 ko/s, contre 32 ko/s en PCM 16 kHz ; un aller-retour de 4G, de l'ordre de 50 à 150 ms) ; le texte définitif (en direct chez ElevenLabs, Deepgram, Mistral ou OpenAI : quelques centaines de ms après la fin ; Whisper base sur le poste : de l'ordre de la seconde) ; **le tour de l'agent, qui domine** (des secondes à des minutes) ; le premier son de la synthèse (Flash v2.5 d'ElevenLabs et Piper : en dessous de la seconde **[à mesurer]**). La voix ajoute de l'ordre d'une à deux secondes à ce que met l'agent.
- **Un tour long** : la voix dit tout de suite « C'est parti, je te le dis quand c'est fini » ; sous CarPlay elle libère la session audio en attendant (Apple le veut) ; l'appel reste ouvert et dit la réponse à la fin du tour. Si l'utilisateur a raccroché, la réponse attend dans la conversation, et l'agent remonte en tête de la liste s'il l'attend.
- **Les zones sans couverture** : la phrase captée est gardée **en mémoire**, jamais sur le disque, et renvoyée au retour du tunnel (deux minutes au plus, puis « pas envoyé, redis-le ») ; ce que le cœur avait à dire attend dans la conversation ; une alerte « Pas de réseau » est le troisième écran permis.
- **Parler pendant que la voix parle** : le barge-in du poste (l'écho appris sur ses haut-parleurs) ne vaut pas pour une voiture. Retenu : la **suppression de l'écho du système** (le mode `voiceChat` de la session audio d'iOS ; sous Android, celle d'un appel en `MODE_IN_COMMUNICATION`, routé par Telecom) et l'interruption **par le bouton « Parler »** de l'écran sous CarPlay ; l'interruption par la voix seulement si l'essai en voiture la montre fiable **[à vérifier en voiture]**.
- **Le bouton de la voix au volant** appartient à Siri (et à l'assistant de Google sous Android Auto) : il ne vient pas à l'app **[à vérifier]**. D'où, sous CarPlay, « Parler » à l'écran et l'écoute qui reprend seule après chaque réponse ; sous Android Auto, l'écran d'appel n'a que les boutons d'un appel (raccrocher, couper le micro) : l'écoute reprend seule, la fin d'une phrase est trouvée par le silence, et couper le micro suspend l'écoute.
- **Le coût et le quota** : la minute de transcription et les caractères de synthèse au tarif du fournisseur réglé (la minute en direct coûte de l'ordre de 1,8 fois plus chez ElevenLabs et Deepgram, d'après Réglages › Voix) ; Whisper et Piper ne coûtent rien. Chaque phrase est un tour d'agent, sur le quota de son compte : la voiture invite à des tours courts et nombreux. Le passage pour l'oreille ne demande aucun appel en plus (§ 6.1).

### 4.4 Le conflit avec M9

M9 (tranché le 2026-11-20, `MOBILE.md`) : « la dictée du système, sur l'appareil ; **l'audio ne part jamais au cœur** ». V1 le contredit. Les options :

1. **Retenu (C10)** : **une exception écrite à M9, pour la seule voix de la voiture**. Allumée avec le mode voiture de l'espace (§ 7.4), éteinte par défaut ; le son ne part que pendant un appel, pendant que l'utilisateur parle (son état « Je t'écoute »), par le tunnel chiffré ; le cœur ne le garde jamais (ni disque, ni journal), le transcrit ou le passe au fournisseur réglé, comme sur le poste. **La dictée de l'app reste M9.**
2. **M9 tenu** : V3 seulement (entendre sur le téléphone, parler par le cœur). Pas de Whisper ni de Scribe en voiture.
3. **M9 révisé partout** : la dictée de l'app passe aussi par la voix d'Escouade. Hors de cette étude.

**Ce que l'utilisateur doit savoir**, dit à l'activation du mode voiture : sa voix (et celle des passagers) part de son téléphone **vers son cœur** (son poste, ou son serveur), puis, s'il a réglé un service en ligne, **vers ce fournisseur, avec sa clé**, comme quand il parle à son poste ; elle n'est gardée nulle part par Escouade. Sous Android Auto comme sous CarPlay, c'est Escouade qui entend et parle, jamais l'assistant du système.

## 5. La liste et l'appel

### 5.1 Ce que montre la liste

- **L'interface est celle de l'utilisateur** (2026-11-28) : « un whatsapp : liste d'agents qui tournent sur escouade, et je peux les appeler en vocal pour leur dire quoi faire (agents lancés, et overseer quand il sera sorti évidemment) ».
- **L'Overseer en tête**, **quand il existe** (allumé sur un espace) ; sinon la liste commence aux agents.
- **Les agents qui tournent**, c'est-à-dire **lancés** : d'abord **ceux qui t'attendent** (une autorisation, une question, un plan), puis **ceux qui travaillent** ; dans chaque groupe, le plus récent d'abord. **Les agents finis n'y sont plus** (ni « fini il y a 4 min », ni les tickets à tester), ni les agents arrêtés.
- **12 lignes au plus** (la limite d'une liste varie selon la voiture **[à vérifier]**). Une ligne : le nom de l'agent, son projet, son état (« t'attend », « travaille »), rien d'autre.
- **Les projets privés** : **hors de la voiture** (C11), comme ils sont hors de la vue du chef (ESC-140) : leurs noms se disent devant les passagers.
- **Les stratégies d'un espace** : `servers.aggregateView` à `false` ne montre de cet espace que des nombres (« 3 agents sur Travail ») ; `servers.contentlessNotifications` y éteint le mode voiture ; `voice.dictation` coupée y éteint l'appel, `voice.onlineAllowed` y ramène tout au local ; la stratégie neuve `servers.carMode` permet à un espace de travail d'interdire la voiture (C5).
- **La même liste** sous CarPlay (§ 3.2) et sous Android Auto (§ 2.2, une liste de contacts de la catégorie de l'appel) : le cœur la donne (`car_agents`), le natif la dessine.

### 5.2 L'appel d'un agent

- **Toucher un agent l'appelle** : une conversation à la voix avec lui, avec la voix d'Escouade (§ 4), jusqu'à ce qu'on raccroche. **Un appel à la fois.**
- **Ce qu'on lui dit** part comme un message de l'utilisateur, marqué « dicté en voiture » (`origin: car`, § 7.2) : on lui dit quoi faire, on lui répond, on lui demande où il en est.
- **Ce qu'il répond** est dit (§ 6.1), puis l'écoute reprend seule ; la réponse entière reste dans sa conversation, sur le poste et le téléphone.
- **Un tour long** : la voix dit « C'est parti, je te le dis quand c'est fini » ; l'appel reste ouvert et dit la réponse à la fin du tour, ou l'utilisateur raccroche et rappelle plus tard (§ 4.3).
- **Ses questions** (celles qui ne font pas agir) sont posées une à une, comme en conversation vocale ; la réponse dite y répond. **Ses autorisations ne se donnent pas à la voix** (§ 7.1).
- **L'Overseer** s'appelle comme un agent : il résume, dit ce qui attend, transmet ; il ne lance rien depuis la voiture (C8).
- **Raccrocher** (le bouton, ou l'écran d'appel d'Android Auto, ou une minute sans parler ni réponse en cours) ferme le flux de la voix et rend la session audio. L'agent, lui, continue son travail.
- **Pas d'appel entrant dans le socle** : c'est l'utilisateur qui appelle ; ce qui attend se voit en tête de la liste. CarPlay n'a pas d'appel entrant (la catégorie s'ouvre quand l'utilisateur touche l'app). Sous Android Auto, un agent qui t'attend pourrait **te rappeler** (un appel entrant par `CallsManager`, que tu décroches ou non) : c'est **CAR-6, en option**, qui ne se construit qu'après un « oui » de l'utilisateur, car ses réponses ne le demandent pas.

## 6. Ce qui est dit d'un tour d'agent

### 6.1 Qui fait le résumé

La sortie d'un agent a du code, des chemins, des commandes. Les options :

1. **Retenu (C12) : l'agent lui-même, prévenu**. La conversation vocale l'a déjà : la consigne `<escouade-voice>` lui fait finir sa réponse par un passage `<spoken>` de deux ou trois phrases, sans code, chemin ni liste. En voiture, la consigne dit en plus « l'utilisateur conduit : le plus important d'abord, une seule question, fermée si possible ; ce qui demande à voir, dis "je te le garde pour le téléphone" ». **Coût : quelques dizaines de tokens par message**, aucun appel en plus.
2. **Un modèle léger** (Haiku) résume chaque tour : un appel de plus, du quota, une à trois secondes.
3. **Le cœur seul** : `ear_text()` sur la réponse entière ; souvent illisible.

### 6.2 La garde du cœur

Une consigne ne suffit jamais. `ear_text()` (ESC-696 § 3.4) s'applique à ce qui est dit : il retire les blocs de code, les tableaux, le Markdown, les chemins et les adresses, remplace un bloc retiré par « du détail, sur ton téléphone », et coupe au-delà de 600 caractères. **Sans passage `<spoken>`** : « C'est fait. La réponse est sur ton téléphone. » ; tour en erreur : « L'agent s'est arrêté sur une erreur. » Sous CarPlay comme sous Android Auto, c'est ce texte que la voix d'Escouade dit dans l'appel ; il ne va dans aucune messagerie.

## 7. La sécurité, qui prime

**ESC-721 n'y change rien** : les règles ci-dessous valent telles quelles pour l'appel, sous CarPlay et sous Android Auto. Deux mentions datent de la messagerie d'Android Auto, abandonnée, et se lisent ainsi : « le receiver d'Android Auto » (§ 7.2) est l'app d'Android Auto qui tient l'appel, et c'est elle qui pose `origin: car` ; « Google qui lit et entend » et le conflit avec M5 (§ 7.4) n'ont plus d'objet, puisque aucun texte de message n'entre dans une notification et que c'est Escouade qui entend et parle. La règle de C3 reste écrite (rien hors de la projection, rien sur l'écran verrouillé, jamais sous `servers.contentlessNotifications`) et vaut pour la notification `CallStyle` de l'appel, qui ne porte que le nom de l'agent.

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

1. L'utilisateur touche un agent (il l'appelle) ; l'app pousse l'écran de l'appel, ouvre la session audio (`playAndRecord`, mode `voiceChat`), monte le tunnel (le Rust de MO-3) et ouvre **un flux de voix** vers le cœur (`car_voice_open`, la conversation nommée ; servi, portée `phone`).
2. Elle envoie le son en Opus ; le cœur transcrit (Whisper, ou le fournisseur réglé, en direct quand il le permet), envoie le texte à l'agent (`origin: car`, la consigne de la voiture), et rend à l'app l'état (« Il réfléchit »).
3. À la fin du tour, le cœur prend le passage pour l'oreille (§ 6), le passe par `ear_text()`, le fait dire (Piper ou la voix réglée), et rend le son phrase par phrase par le même flux ; l'app le joue, puis réécoute.
4. Raccrocher, ou une minute sans parler ni tour en cours, ferme le flux et rend la session audio.

### 8.3 Le chemin d'un appel, sous Android Auto

**Si CAR-5 établit l'appel** (§ 2.5) ; sinon, rien.

1. Pendant la projection (`CarConnection`), le `CarAppService` de la catégorie `CALLING` montre la liste du cœur (`car_agents`, par le tunnel) en liste de contacts.
2. L'utilisateur touche un agent : l'app ajoute un appel sortant par `CallsManager.addCall` (l'adresse : l'identifiant de l'agent), pose sa notification `CallStyle` dans les 5 secondes, passe l'appel actif ; Android Auto prend l'écran d'appel.
3. L'app ouvre le flux de voix (`car_voice_open`, comme sous CarPlay), capte le micro en `MODE_IN_COMMUNICATION` (Telecom le route depuis la voiture) et joue le son rendu par le cœur, que Telecom route vers la voiture. Le reste est le chemin du § 8.2, étapes 2 et 3.
4. Raccrocher depuis la voiture (`onDisconnect`), depuis l'app ou par le cœur (fin du flux) termine l'appel des deux côtés.

Aucune notification `MessagingStyle`, aucun `ConversationItem`, aucun receiver de réponse dictée.

### 8.4 Ce qu'il faut construire

- **Le cœur** : `origin` sur `overseer_send` et sur l'envoi à un agent ; `device_car_state`, `car_agents` (la liste du § 5.1), `car_voice_open` servis (portée `phone`) ; la consigne de la voiture pour le chef et pour les agents ; `ear_text()` ; les refus du § 7.2 ; `servers.carMode`. `car_feed` et `car_seen` (la messagerie) ne sont plus à faire. La liste des commandes reste unique (`remoteapi/list.rs`).
- **La voix sans fenêtre** (le plus gros) : une session de voix du cœur pour un appareil distant, qui prend le son d'un flux au lieu du micro et rend le son au lieu des haut-parleurs ; Whisper, les plugins de voix en ligne et Piper appelés par le cœur sans fenêtre, aussi dans `escouade-server` **[à vérifier : ce que les crates `escouade-voice` et `escouade-voice-local` portent déjà hors de Tauri]** ; le décodage et l'encodage Opus ; les stratégies de la voix ; la clé d'un service sur un serveur (§ 4.2) ; un journal sans texte ni son.
- **Le Rust du téléphone** (`crates/escouade-mobile`) : l'appel de l'API dans le tunnel et le flux de voix, exposés par UniFFI.
- **iOS** : la scène CarPlay, `CPListTemplate` et `CPVoiceControlTemplate`, la session audio, la capture et la lecture, le repli V3 (`SFSpeechRecognizer` sur l'appareil) ; l'autorisation dans les entitlements.
- **Android** (si l'appel s'établit) : `CarConnection` ; un `CarAppService` de la catégorie `CALLING` (la liste des agents en contacts) ; l'intégration à Telecom par Core-Telecom (`CallsManager`, `MANAGE_OWN_CALLS`, la notification `CallStyle`), tenue en tout temps comme Google le demande ; l'audio de l'appel sur le flux de voix ; `automotive_app_desc.xml`.

### 8.5 Les dépendances

- **Le tunnel** (MO-2, MO-3) et **l'appairage** (MO-4) : faits.
- **MO-6 (ESC-572), les notifications** : **plus nécessaire** à la voiture : un appel part de l'utilisateur, l'app ouverte. Seul CAR-6 (le rappel, en option) s'en sert.
- **MO-8, la publication** : le test fermé de Google Play et TestFlight, où la voiture s'essaie.
- **MO-9 (ESC-577)** (UnifiedPush, en option) : le réveil d'Android sans FCM ; CAR-6 le branche s'il existe, pour le rappel.
- **L'Overseer** (ESC-462, ESC-468) et l'API distante : faits.

## 9. Les essais, la publication, le découpage

### 9.1 Les essais

| Quoi                                                                                                                                                                                       | Où                                                                                                                                        | En CI                                                                                                            |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| Le cœur : `origin: car`, chaque feu vert refusé (chef et agents), le message refusé à un agent qui agit seul, `ear_text()`, la liste (privés exclus, stratégies), l'état qui s'éteint seul | nextest                                                                                                                                   | **Oui** : c'est ce qui protège                                                                                   |
| La voix sans fenêtre : un flux de son enregistré (un fichier WAV de test) transcrit par Whisper et par un faux service, la synthèse rendue en Opus, les refus des stratégies               | nextest (un faux service de voix, comme les tests de la voix en plugins)                                                                  | **Oui**                                                                                                          |
| Le Rust du téléphone : l'appel et le flux dans le tunnel, contre le pair de test de MO-3                                                                                                   | nextest de `crates/escouade-mobile`                                                                                                       | **Oui**                                                                                                          |
| Android : la liste des contacts, l'appel ajouté par `CallsManager` (ses rappels, `CallStyle`), raccrocher des deux côtés                                                                                                              | tests Kotlin (Robolectric) **[à vérifier dans le plugin]**                                                                                | **Oui**                                                                                                          |
| Android Auto de bout en bout : la liste, l'appel, la voix d'Escouade entendue, la réponse dite, raccrocher depuis la voiture                                                                                                                     | **Desktop Head Unit** (Windows, macOS, Linux ; l'émulateur ou un téléphone par ADB ; le DHU accepte un micro ou des commandes)            | Non : un écran et l'app Android Auto ; une procédure locale écrite                                               |
| iOS : la liste, l'écran de l'appel, « Raccrocher », la session audio, le repli V3                                                                                                                          | XCTest ; **le simulateur CarPlay** (Simulator › I/O › External Displays › CarPlay, ou _CarPlay Simulator_ des Additional Tools for Xcode) | XCTest **oui** (job Mac, sans signer) ; le simulateur **non** (il veut l'autorisation sur le profil **[tiers]**) |
| **En voiture** : la latence sur 4G, une zone sans couverture, l'écho et l'interruption, le bouton du volant                                                                                | **L'utilisateur**, avec sa voiture, l'app du test fermé ou de TestFlight                                                                  | Non : un geste de l'utilisateur, déposé dans « À toi » par chaque ticket                                         |

Apple et Google disent de ne pas se fier au seul simulateur : l'essai en voiture fait partie du ticket.

### 9.2 La publication

- **Apple** : la demande de l'autorisation de **la conversation à la voix**, par le compte de développeur de l'utilisateur, avec son texte (CAR-7, écrit d'abord : c'est le plus long délai, et le seul qu'on ne maîtrise pas) ; puis l'App ID, le profil, TestFlight, l'App Review.
- **Google Play** (si l'appel s'établit) : le facteur de forme Android Auto dans la Play Console, sur le test fermé (la revue n'y bloque pas) ; la production attend que Google ouvre l'appel (en bêta) ou nous admette en accès anticipé, et qu'il admette un agent comme correspondant (§ 2.4). **D'ici là, la production n'a pas de mode voiture sous Android** : aucun repli par la messagerie.
- **Les agents ne créent aucun compte, ne déposent aucune demande** chez Apple ou Google, ne lisent ni ne copient aucune clé : ils écrivent les textes.

### 9.3 Le découpage en tickets

Estimations en **heures d'agent** (Claude Code, six agents en parallèle), jamais en jours-homme. Révisé par ESC-721 : la messagerie sort, l'appel entre ; les tickets sont créés dans le Kanban, en bas d'« À faire », après ceux de la 2.3 : **ESC-723 à ESC-729**.

| Ticket                                    | Ce qu'il fait                                                                                                                                                                                                                                                                                                                         | Dépend de                                                   | Heures |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------ |
| **CAR-1** (ESC-723) · le cœur en voiture            | `origin: car` (chef et agents) ; `device_car_state` ; `car_agents` (la liste du § 5.1 : l'Overseer en tête, les agents qui t'attendent puis qui travaillent, sans les finis ni les privés) ; la consigne de la voiture ; `ear_text()` ; **les refus du § 7.2** et la demande déposée dans « À toi » ; `servers.carMode` ; nextest ; spec et `REMOTE.md` | —                                                           | 5 h    |
| **CAR-2** (ESC-724) · le Rust du téléphone          | L'appel de l'API et le flux de voix dans le tunnel, par UniFFI, sans la WebView ; ses tests                                                                                                                                                                                                                                         | —                                                           | 3 h    |
| **CAR-3** (ESC-725) · la voix sans fenêtre          | La session de voix du cœur pour un appel (`car_voice_open`) : le son d'un flux, Whisper ou le service réglé, Piper ou la voix réglée, Opus ; dans le poste et dans `escouade-server` ; la clé d'un service sur un serveur ; les stratégies ; l'exception à M9 ; nextest                                                              | CAR-1                                                       | 6 h    |
| **CAR-4** (ESC-726) · CarPlay                       | La scène ; `CPListTemplate` (la liste) puis `CPVoiceControlTemplate` (l'appel, « Raccrocher ») ; un appel à la fois ; la session audio (`voiceChat`) tenue pendant l'échange ; le flux vers le cœur ; le repli V3 ; « Pas de réseau » ; XCTest ; l'essai au simulateur et en voiture déposé dans « À toi »                          | CAR-2, CAR-3, **l'autorisation d'Apple** (pour l'essai)     | 6 h    |
| **CAR-5** (ESC-727) · Android Auto, par l'appel     | **D'abord établir l'appel** (§ 2.5) : un `CarAppService` `CALLING`, un appel par `CallsManager`, la voix d'Escouade et le micro de la voiture au DHU. **S'il ne s'établit pas, CAR-5 s'arrête et le dit**, sans messagerie. Sinon : la liste en contacts, l'intégration à Telecom en tout temps, `CallStyle`, le mode voiture par espace, Robolectric, la procédure du DHU | CAR-1, CAR-2, CAR-3                                         | 6,5 h  |
| **CAR-6** (ESC-728) · le rappel (en option)         | Après un « oui » de l'utilisateur : un agent qui t'attend te rappelle pendant la projection (un appel entrant par `CallsManager`), éteint par défaut ; réveillé par le push scellé de MO-6, ou par UnifiedPush (MO-9) sans FCM                                                                                                         | CAR-5, MO-6 (ESC-572), MO-9 (ESC-577)                       | 1,5 h  |
| **CAR-7** (ESC-729) · la publication                | Le texte de la demande à Apple (d'abord), les fiches des stores, la page de confidentialité (le son vers le cœur et le fournisseur), les notes pour les revues, la question à Google sur un agent comme correspondant ; **les gestes de l'utilisateur** dans « À toi », jamais faits par un agent                                         | — (la fiche finale des stores, écrite par CAR-4 et CAR-5 ; la mise en ligne, MO-8)      | 1,5 h  |

- **L'ordre, à six agents** : **CAR-7 (le texte à Apple), CAR-1 et CAR-2 tout de suite**, en parallèle ; puis **CAR-3** ; puis **CAR-4** et **CAR-5** en parallèle (CAR-4 essayé dès l'autorisation d'Apple) ; CAR-6 seulement sur un « oui ». CAR-7 n'attend rien : il écrit les textes d'après cette étude ; CAR-4 et CAR-5 mettent à jour la fiche de leur système, et la mise en ligne passe par MO-8 (ESC-576).
- **Total : environ 28 h d'agent** (29,5 h avec CAR-6), **de l'ordre de 18 h de bout en bout** (le plus long chemin : CAR-1, CAR-3, puis CAR-5 ou CAR-4), sans compter les délais d'Apple et de Google. Si l'appel ne s'établit pas, CAR-5 s'arrête au bout de l'essai (de l'ordre de 2 h) et CAR-6 tombe.
- **ESC-698 comptait 27,5 h** : la messagerie (notifications et modèles, `car_feed`, `car_seen`, le receiver) sort ; l'appel par Telecom et l'essai qui le conditionne entrent ; **CAR-5 n'attend plus MO-6**.
- **Les gestes de l'utilisateur** (CAR-7 les dépose dans « À toi », un agent ne les fait jamais) : la demande de l'autorisation CarPlay à Apple ; le facteur de forme Android Auto dans la Play Console (et, s'il le veut, la candidature d'accès anticipé de Google pour l'appel) ; sur un serveur, la clé d'une voix en ligne à y poser s'il en veut une ; l'essai en voiture, sous CarPlay et sous Android Auto.

## 10. Les choix

**Tranchés par l'utilisateur le 2026-11-28** (a-56) : « tout recommandé, à part sur l'UI », puis « on fait tout en vocal ou on fait rien ». Chaque choix reçoit donc la version recommandée, sauf l'interface (la sienne) et C6 (qui tombe).

1. **C1. Le périmètre et l'ordre** : **tranché, le recommandé** : les deux dans la 2.4 ; la demande à Apple d'abord ; CAR-1 à CAR-3 tout de suite ; CarPlay dès l'autorisation. **Ajusté par « tout en vocal ou rien »** : Android Auto **seulement par l'appel**, établi d'abord dans CAR-5 ; sans l'appel, pas d'Android Auto ; et il n'attend plus MO-6.
2. **C2. Ce que la voiture expose** : **tranché, la version de l'utilisateur** (l'interface) : **un « WhatsApp » des agents qui tournent** : l'Overseer en tête quand il existe, puis les agents lancés, ceux qui t'attendent en tête, puis ceux qui travaillent ; **plus les agents finis** ; 12 lignes au plus ; un nom, un projet, un état. **Toucher un agent l'appelle** ; on raccroche ; un appel à la fois (§ 5).
3. **C3. Le conflit avec M5, sous Android Auto** : **tranché, le recommandé** : le mode voiture par espace ; rien sur l'écran verrouillé, rien hors de la projection, jamais sous `contentlessNotifications`. Sans messagerie, aucun texte de message n'entre dans une notification ; la règle vaut pour la notification `CallStyle` de l'appel (le nom de l'agent).
4. **C4. La catégorie de CarPlay** : **tranché, le recommandé** : la conversation à la voix, seule ; la communication (Siri qui lit et dicte) abandonnée.
5. **C5. Une stratégie pour les espaces de travail** : **tranché, le recommandé** : `servers.carMode` (vrai par défaut, faux sous `contentlessNotifications`), qui coupe la liste et l'appel en voiture.
6. **C6. La messagerie à modèles d'Android Auto** : **tombé** (« tout en vocal ou rien ») : ni messagerie à modèles, ni notifications `MessagingStyle` de voiture. L'appel la remplace (§ 2).
7. **C7. Android Automotive OS** : **tranché, le recommandé** : hors de la 2.4 (les apps de communication n'y sont d'ailleurs pas admises).
8. **C8. Ce que la voiture ne peut pas faire** : **tranché, le recommandé** : tout ce que M3 met sous verrou, lancer un ticket ou un agent compris, le « oui » dit à une autorisation, et aucun message à un agent qui agit sans demander (gardé pour le téléphone).
9. **C9. Où se fait la voix** : **tranché, le recommandé** : par le cœur, avec Réglages › Voix (V1), clés sur le cœur ; repli sur la reconnaissance du téléphone quand le cœur n'entend plus (V3). Sous Android Auto aussi, dans l'appel.
10. **C10. Le conflit avec M9** : **tranché, le recommandé** : une exception écrite pour la seule voix de la voiture (mode voiture allumé, le son seulement pendant un appel et pendant qu'on parle, par le tunnel, jamais gardé, puis au fournisseur réglé) ; la dictée de l'app reste M9.
11. **C11. Les projets privés** : **tranché, le recommandé** : hors de la voiture.
12. **C12. Ce qui est dit d'un tour d'agent** : **tranché, le recommandé** : l'agent lui-même, prévenu (le passage `<spoken>` et la consigne de la voiture), sous la garde `ear_text()` ; aucun appel de modèle en plus.

## 11. Ce qui serait un échec, même si tout marche

- Un feu vert parti d'une phrase dite en voiture : une autorisation accordée d'un « oui », un merge, un lancement, une consigne.
- Une commande d'un agent lue à voix haute, ou un tableau, un chemin, du code.
- Une clé d'un fournisseur de voix sur le téléphone, ou une clé de l'appairage hors du Keystore ou du Trousseau.
- Le son de la voiture gardé sur le disque ou dans un journal, du téléphone ou du cœur.
- Le texte d'un message sur l'écran verrouillé, ou dans une notification hors de la projection.
- Une version de production refusée par Google ou Apple pour ce qu'ils interdisent : la production attend leur avis.
- Un mode voiture qui ne serait pas un appel à la voix : une messagerie lue et dictée par l'assistant du système, sous Android Auto ou sous CarPlay (« tout en vocal ou rien »).

## 12. Les sources (lues le 2026-11-27, et le 2026-11-28 pour l'appel)

**Android, l'appel (lues le 2026-11-28 par ESC-721, sources de Google seulement)**

- _Build calling experiences for Android Auto_ : https://developer.android.com/training/cars/communication/calling
- _Communication apps_ (design : messagerie par notifications, messagerie à modèles et appel, ces deux derniers en bêta) : https://developer.android.com/design/ui/cars/guides/app-types/communications
- _Core-Telecom_ : https://developer.android.com/develop/connectivity/telecom/voip-app/telecom ; référence de `CallsManager` : https://developer.android.com/reference/androidx/core/telecom/CallsManager
- _Telecom framework overview_ : https://developer.android.com/develop/connectivity/telecom ; _Manage calls using the Telecom API_ : https://developer.android.com/develop/connectivity/bluetooth/ble-audio/telecom-api-managed-calls
- Référence de `ConnectionService` : https://developer.android.com/reference/android/telecom/ConnectionService ; de `PhoneAccount` (`CAPABILITY_SELF_MANAGED`, dépréciée au niveau 37) : https://developer.android.com/reference/android/telecom/PhoneAccount
- _AEP guideline: Android Telecom Framework_ : https://developer.android.com/distribute/aep/aep-req-telecom-api
- _Android multi-platform app quality guidelines_ (aide de la Play Console) : https://support.google.com/googleplay/android-developer/answer/18058043
- Et, déjà lues : _Set up your project_, _Car app quality_, _Distribute to cars_, _What's new_ (ci-dessous).

**Android (developer.android.com, sauf mention ; lues le 2026-11-27 ; la messagerie n'est plus suivie, gardée pour mémoire)**

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
