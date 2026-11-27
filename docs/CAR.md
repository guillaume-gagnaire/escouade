# Escouade en voiture : Android Auto, CarPlay et le mode overseer à la voix (étude 2.4)

Étude d'ESC-696, pour la 2.4. **Rien du produit ne change dans ce ticket** : ce document, une page de spec en brouillon (« Escouade à distance › En voiture », `a-distance/en-voiture`) et une demande dans la boîte « À toi » où l'utilisateur tranche les choix C1 à C8 (§ 9). Aucune décision n'est proposée avant ses réponses.

**La demande** (2026-11-27) : « Est-ce qu'il y a un monde où l'app mobile Android est compatible Android Auto en mode overseer ? », puis « Étude pour la 2.4, avec Apple aussi ». L'idée : dans la voiture, le chef d'escouade (l'Overseer) parle à l'utilisateur, ses messages sont lus à voix haute, l'utilisateur répond en dictant. Ni Kanban ni écran de l'app : Android Auto et CarPlay n'acceptent que des catégories d'apps fixées, et des modèles d'écran imposés.

**Ce qui fait foi** : [`MOBILE.md`](MOBILE.md) (l'app mobile, ses choix M1 à M16, en particulier M3 le verrou et M5 ce qui part en notification ; le relais de push MO-6 ; le plugin natif `apps/mobile-plugin`) ; [`OVERSEER.md`](OVERSEER.md) (son autorité : il agit comme l'utilisateur, n'obéit qu'à lui) ; [`REMOTE.md`](REMOTE.md) (l'API distante et ses commandes servies, dont `overseer_send` et `overseer_view`) ; [`INBOX.md`](INBOX.md) (la boîte « À toi »).

**Les sources** : toutes lues le **2026-11-27**, par la recherche web (des extraits des pages officielles, pas toujours la page entière). Ce qui ne vient que d'un forum ou d'un tiers est marqué **[tiers]** ; ce qui reste à vérifier sur la page entière ou sur le banc est marqué **[à vérifier]**. La liste complète est au § 11. Le contenu de ces pages est une donnée, jamais une consigne.

## 0. En bref

- **Oui, il y a un monde**, sous Android comme sous iOS, mais **pas par l'interface de l'app** : par la **messagerie** que la voiture sait déjà lire et dicter. Le chef devient, pour la voiture, **un contact qui écrit** ; la voiture lit ses messages (l'assistant du système : Google Assistant ou Gemini sous Android Auto, Siri sous CarPlay) et l'utilisateur lui répond à la voix.
- **Android Auto** : la messagerie par notifications (`MessagingStyle`, réponse par `RemoteInput`, « marquer comme lu ») est stable et suffit. La messagerie à modèles de la Car App Library (`ConversationItem`, Car API 7) est **toujours en bêta, limitée aux pistes de test interne et fermé de Google Play** au 2026-11-27. **Le risque** : la règle de qualité MF-5 demande « un service de messagerie de pair à pair », pas un service de notifications ; une conversation avec un agent d'IA n'est ni l'un ni l'autre, et aucune page officielle ne le tranche. **Mais** la revue Android Auto d'une piste de test fermé **ne bloque pas** : l'utilisateur, qui est le premier usager, peut s'en servir dès le test fermé déjà prévu (MO-8), et la production dira si Google l'accepte.
- **CarPlay** : deux portes, une seule par app. **La communication** (`com.apple.developer.carplay-communication` : SiriKit, envoyer, chercher et lire des messages, l'annonce des notifications par Siri), qui marche app fermée ; ou **la conversation à la voix** (`com.apple.developer.carplay-voice-based-conversation`, nouvelle depuis iOS 26.4, celle de ChatGPT, Claude et Gemini), qui ne vit que l'app ouverte sur l'écran de la voiture. Les deux s'obtiennent **sur demande à Apple**, sans délai connu (des semaines, parfois des mois, d'après les forums).
- **La sécurité prime, et elle est tenue par le cœur, pas par la consigne du chef** : un message dicté en voiture est **marqué** (`origin: car`) ; **aucun feu vert ne part d'un tour du chef né d'un message marqué** (publier, signer, supprimer, autoriser une commande, approuver un plan, valider un ticket qui fusionne, lancer : tout ce que M3 met sous verrou) ; le chef les **range dans « À toi »**, pour le téléphone, sous verrou. Le texte n'entre dans une notification **que pendant la projection**, sur un espace où le mode voiture est allumé, jamais sous `servers.contentlessNotifications`, et masqué sur l'écran verrouillé. Les clés restent dans le natif (Keystore, Trousseau), comme depuis MO-4.
- **Découpage** : six tickets, **CAR-1 à CAR-6, environ 17,5 h d'agent** (18,5 h avec CAR-5, en option), de l'ordre de **11 h de bout en bout** à six agents, **après MO-6** (ESC-572, les notifications) et, pour la sortie, MO-8 (la publication). Ce qui attend l'utilisateur : ses réponses à C1–C8, la demande d'autorisation à Apple, le facteur de forme Android Auto dans la Play Console, et l'essai en voiture.

## 1. Android Auto

### 1.1 La messagerie par notifications (la voie retenue)

Ce que demande Android Auto à une app de messagerie, d'après _Extend messaging notifications to Android Auto_ et _Build messaging apps for Android Auto_ (developer.android.com, lus le 2026-11-27) :

- **La notification** est de la catégorie `CATEGORY_MESSAGE`, au style `NotificationCompat.MessagingStyle`, et ne porte **que les messages non lus** d'une conversation (un ou plusieurs `MessagingStyle.Message`, chacun avec son auteur, une `Person`).
- **Deux actions** : **répondre** (sémantique `SEMANTIC_ACTION_REPLY`, avec un `RemoteInput` qui rend à l'app le texte dicté) et **marquer comme lu** (`SEMANTIC_ACTION_MARK_AS_READ`, déclenché quand l'utilisateur touche la notification sur l'écran de la voiture). Les deux se traitent **en arrière-plan**, sans ouvrir d'interface (`setShowsUserInterface(false)`). Le guide montre encore un `IntentService`, déprécié : on prendra un `BroadcastReceiver` qui confie le travail à un job (`goAsync()` ou un travail accéléré de WorkManager) **[à vérifier]**.
- **La déclaration** : un fichier `res/xml/automotive_app_desc.xml` qui dit `<uses name="notification"/>`, désigné dans le manifeste par la méta-donnée `com.google.android.gms.car.application`.
- **Le reste vient de la voiture** : Android Auto annonce le message, le lit à voix haute sur demande (« lire »), propose de répondre, dicte et relit la réponse avant de l'envoyer. L'app n'a rien à dessiner.

**Les règles de qualité** (_Car app quality_, catégorie « Messaging », MF-1 à MF-5, lues le 2026-11-27) : recevoir les messages entrants (MF-1) ; les grouper et les ordonner ; répondre ; une forme courte (MF-4 : la messagerie longue, comme le courriel, n'est pas permise) ; et **MF-5 : « un service de messagerie de pair à pair, pas des services de notification comme la météo, la bourse ou les scores »**. Plus haut, la règle de toutes les apps de voiture : éviter avant tout la distraction du conducteur, et ne pas allumer l'écran du téléphone pendant la projection.

**Ce que ça veut dire pour nous** : le chef est un interlocuteur qui répond, pas une alerte à sens unique ; c'est une conversation, courte, qu'on peut lire et à laquelle on répond, ce qui tient MF-1 à MF-4. **MF-5 est le point dur** : « de pair à pair » suppose deux personnes. Aucune page officielle ne dit si une conversation avec un agent d'IA y entre (§ 1.4).

### 1.2 La messagerie à modèles de la Car App Library

_Build templated messaging experiences for Android Auto_ (lu le 2026-11-27) :

- Une app qui la prend doit **aussi** faire la messagerie par notifications ; elle déclare la catégorie `androidx.car.app.category.MESSAGING` dans le filtre d'intention de son `CarAppService`, et `minCarApiLevel` à **7**, car `ConversationItem` n'existe qu'à partir de la Car API 7.
- Les conversations s'affichent dans un `ListTemplate` ou un `SectionedItemTemplate` ; chaque `ConversationItem` porte d'office lire, marquer comme lu et répondre (et, depuis les bêtas de la 1.7, des actions à soi).
- **Où elle en est** : **toujours en bêta**. « Tout le monde peut publier des apps de communication à modèles sur les pistes de test interne et de test fermé » ; « la publication en test ouvert et en production sera permise plus tard » ; une soumission qui l'embarque sur ces pistes est refusée. **Rien n'a changé depuis le dernier relevé** (Google I/O 2025, puis le billet de mai 2026 qui annonce « de nouveaux composants et des modèles de conversation » à venir).
- **La bibliothèque** : 1.7.0 stable (celle qui a introduit `ConversationItem`), 1.8.0-rc01 (2026-08-26, avec un correctif de sécurité), 1.9.0-alpha03 au 2026-10-07 (page _Car App_ des versions de Jetpack).
- **Un retour de terrain** **[tiers]** : un fork d'OpenBubbles rapporte que, dans une vraie voiture, Android Auto ne s'est jamais lié à son service `ConversationItem` et retombait sur les notifications, alors que tout marchait sur le Desktop Head Unit.

**Pour nous** : elle n'apporte que l'historique de la conversation à l'écran, que les notifications suffisent à remplacer pour l'oreille, et elle ferme la production. **Pas dans la 2.4** (C6) ; à revoir quand Google l'ouvrira à la production.

### 1.3 Android Automotive OS

Android Automotive OS est le système **intégré** à la voiture (pas de téléphone projeté). D'après _Notifications on Android Automotive OS_ et _Communication apps_ (lus le 2026-11-27) : il ajoute de lui-même « Lire » et « Muet » aux notifications de messagerie compatibles, ne connaît pas les canaux de notification, ni les vues personnalisées (`RemoteViews`) ; l'app y est **installée dans la voiture**, depuis le Google Play de la voiture, et y tourne seule. **Ce qui change pour nous** : l'app devrait vivre dans la voiture, sans WebView ni interface Capacitor pendant la conduite, avec son propre appairage (une clé de plus, dans une voiture qui peut être prêtée ou louée), sur un parc de voitures étroit. **Hors de la 2.4** (C7).

### 1.4 Un agent d'IA dans la revue Android Auto de Google Play

- **Rien d'officiel ne le dit** : les pages de qualité et de distribution ne parlent ni de robot ni d'assistant. MF-5 exclut « les services de notification » ; une conversation avec le chef n'en est pas un, mais n'est pas non plus « de pair à pair » au sens strict.
- **Ce qu'on sait des autres** **[tiers]** : en avril 2026, Google « n'avait pas encore ouvert Android Auto aux chatbots tiers » (pas de catégorie comme celle d'Apple, § 2.4) ; Gemini y est l'assistant du système.
- **Comment la revue s'applique** (_Distribute to cars_, lu le 2026-11-27) : le facteur de forme « Android Auto » s'ajoute dans la Play Console (Paramètres avancés › Facteurs de forme) ; **pas de revue** sur le partage interne et le test interne ; une revue **non bloquante** sur le **test fermé** (« la soumission est approuvée, on vous dit seulement que la version n'est pas conforme ») ; une revue **bloquante** en test ouvert et en production.
- **Les conditions qui donnent le plus de chances** : un vrai échange (le chef répond à ce qu'on lui dit), des messages courts, un seul contact (« Escouade », le chef), une `Person` marquée comme robot (`setBot(true)`), aucune alerte à sens unique (« CI rouge » seule n'est pas un message : c'est le chef qui le dit, dans sa conversation), et la fiche du store qui le dit clairement.
- **La conclusion** : l'utilisateur, premier usager, s'en sert dès le **test fermé** (MO-8 le prévoit, avec ses 14 jours) ; la **production** dépend d'un avis de Google qu'on ne peut pas connaître d'avance. On ne bâtit rien qui ne servirait qu'en production.

## 2. CarPlay

### 2.1 Les catégories et l'autorisation

D'après _CarPlay Developer Guide_ (édition d'octobre 2026, PDF sur developer.apple.com), _Requesting CarPlay entitlements_ et la page CarPlay d'Apple (lus le 2026-11-27) :

- **Toute app CarPlay demande une autorisation (entitlement) propre à sa catégorie**, et **une seule catégorie par app** (sauf la recharge et le carburant, cumulables).
- **La communication** : `com.apple.developer.carplay-communication` (depuis iOS 14 ; elle remplace les anciennes `carplay-messaging` et `carplay-calling`, à ajouter seulement pour iOS 13). « Les apps de communication doivent offrir de la messagerie courte, de l'appel en VoIP, ou les deux. » Elles passent par **SiriKit** (et CallKit pour les appels).
- **La conversation à la voix** : `com.apple.developer.carplay-voice-based-conversation`, **nouvelle depuis iOS 26.4** (février 2026) : « une modalité principale qui est la voix dès le lancement », puis répondre aux questions et aux demandes et agir ; une session audio active **seulement pendant que la voix sert** ; le modèle `CPVoiceControlTemplate` (réservé à la navigation et à cette catégorie) ; trois écrans de modèle au plus, la racine comprise. ChatGPT y est entré en avril 2026 **[tiers]**. Le nombre de boutons d'action du modèle (deux ou quatre) diffère selon les sources **[à vérifier]**.
- **La demande** : par la page « CarPlay › Contact Us » d'Apple, en acceptant le _CarPlay Entitlement Addendum_ ; Apple juge sur des critères qu'elle ne publie pas en entier, et, si la demande y répond, **ajoute l'autorisation au compte de développeur** (« managed capabilities ») ; on l'ajoute alors à l'App ID, et on régénère le profil d'approvisionnement.
- **Les délais** : **aucun délai officiel**. Les forums **[tiers]** : de l'ordre de 4 semaines pour certains, des mois sans réponse pour d'autres, une demande d'octobre 2026 encore en attente après 15 jours ; Apple ne notifie pas toujours un refus ; certains redéposent après 2 ou 3 semaines avec une autre justification.

### 2.2 SiriKit : envoyer, chercher, lire

- **Ce qu'Apple demande à une app de messagerie** (guide de programmation CarPlay, édition archivée, et _SiriKit › Messaging_) : trois intentions, **`INSendMessageIntent`** (envoyer), **`INSearchForMessagesIntent`** (chercher et lire les messages reçus) et, d'après les forums, **`INSetMessageAttributeIntent`** (marquer comme lu) **[à vérifier sur le guide d'octobre 2026]**. Toute interaction vocale passe par SiriKit ; une intention que l'app ne prend pas reçoit de Siri « pas disponible en voiture ».
- **L'extension d'intentions** (Intents Extension) les traite **sans lancer l'app** : c'est elle qui ira chercher les messages du chef, et qui enverra la réponse.
- **L'annonce des notifications par Siri** : les **notifications de communication** (_Implementing communication notifications_, et la session WWDC21 _Send communication and Time Sensitive notifications_) : l'extension de service de notification (celle de MO-6) crée une `INSendMessageIntent` entrante, la **donne** (donate) au système et en met à jour la notification ; `NSUserActivityTypes` doit contenir `INSendMessageIntent`. Siri peut alors annoncer la notification dans CarPlay (réglage de l'utilisateur : Siri › Annoncer les notifications) et proposer d'y répondre. Un fil des forums **[tiers]** dit que le don seul n'a pas suffi à faire lire les messages dans CarPlay **[à vérifier au simulateur et en voiture]**.
- **Ce qu'on ne fait pas** : afficher le texte des messages sur l'écran de la voiture (une source tierce dit que c'est interdit aux apps de communication **[à vérifier]** ; on ne le voulait pas de toute façon).

### 2.3 Les modèles de CarPlay pour une app de communication

Une app de communication peut montrer une liste de conversations (`CPListTemplate`, ses lignes `CPMessageListItem`, dont le toucher ouvre Siri sur la conversation) et des onglets ; **jamais le texte** d'un message. Pour nous : une seule ligne (« Escouade, le chef d'escouade », le nombre de non lus), et un toucher qui demande à Siri de lire. C'est un confort ; l'essentiel passe par Siri, app fermée.

### 2.4 Ce qu'Apple accepte, et laquelle prendre

- **La communication** colle à la demande telle qu'elle est faite : le chef écrit, Siri **annonce** et lit, l'utilisateur dicte sa réponse, **app fermée**, comme sous Android Auto, avec le même cœur. Le doute : Apple pourrait juger qu'une conversation avec un agent d'IA n'est pas de la « messagerie » et la renvoyer vers la catégorie de la voix.
- **La conversation à la voix** est faite pour les agents d'IA, donc l'autorisation la plus probable ; mais elle ne vit **que l'app ouverte** sur l'écran de la voiture (pas d'annonce spontanée d'une nouvelle du chef quand on ne l'a pas ouverte), et l'app y fait elle-même la reconnaissance et la synthèse de la voix (`SFSpeechRecognizer` sur l'appareil, `AVSpeechSynthesizer`), en natif.
- **Recommandé (C4)** : demander **la communication** d'abord (un seul modèle pour les deux systèmes, l'annonce spontanée, app fermée) ; si Apple la refuse ou ne répond pas en six semaines, demander la conversation à la voix (CAR-4 en a deux variantes, § 8).

## 3. Le mode overseer en voiture

### 3.1 Ce que la voiture expose

- **Recommandé (C2) : une seule conversation, celle du chef**, qui dit le reste. Il annonce, à sa façon, les agents arrêtés (« tests-e2e attend une autorisation sur Atlas : je te la garde pour le téléphone »), les nouvelles (« ESC-31 est fini, la CI est verte ») et la boîte « À toi » résumée (« trois choses t'attendent : deux choix de design et un feu vert »).
- **Pas les agents un par un** : leurs questions demandent souvent de lire du code ou une commande, et une conversation par agent dans la voiture inviterait à répondre de la voix à une autorisation. **Pas la boîte en détail** : ses demandes ont des rendus, des PDF, des commandes.
- **Les projets privés** restent hors de la vue du chef (ESC-140), donc hors de la voiture.
- **Les stratégies d'un espace** valent : `servers.aggregateView` à `false` ne laisse au chef que des nombres pour cet espace ; `servers.contentlessNotifications` y **éteint le mode voiture** (§ 4.1) ; une nouvelle stratégie `servers.carMode` permet à un espace de travail de l'interdire (C5).

### 3.2 Ce qui est lu

Seulement **les messages du chef écrits pour la voiture** (§ 3.4), jamais ses messages d'avant la projection, jamais le texte d'une commande, d'une demande de la boîte, d'un message d'agent, d'un chemin ou d'un nom de branche : le chef les résume. Une file de messages non lus par conversation ; Android Auto et Siri lisent, l'utilisateur dit « lire » ou « répondre ».

### 3.3 Comment le cœur sait que l'utilisateur conduit

- **Sous Android** : `androidx.car.app.connection.CarConnection` (_Connection API_, référence de `CarConnection`, lues le 2026-11-27) : `getType()` rend un `LiveData` de `CONNECTION_TYPE_NOT_CONNECTED` (0), `CONNECTION_TYPE_NATIVE` (1, Android Automotive OS) ou `CONNECTION_TYPE_PROJECTION` (2, Android Auto). Avec une cible Android 14 ou plus, la bibliothèque doit être 1.3.0-beta01 ou plus récente. Le natif l'observe tant que le process vit, et l'interroge à la réception d'un push (le process peut avoir été tué) **[à vérifier : l'interrogation ponctuelle depuis un `FirebaseMessagingService`]**.
- **Sous iOS** : avec la communication, la sortie audio de la voiture (`AVAudioSession.Port.carAudio` dans la route courante) et, quand l'app est ouverte sur CarPlay, la scène (`CPTemplateApplicationScene`, connectée et déconnectée) ; avec la conversation à la voix, la scène seule (l'app n'y vit qu'ouverte).
- **Le téléphone le dit au cœur** par une commande servie nouvelle, `device_car_state` (`driving: true|false`), à chaque changement et toutes les 10 minutes pendant la projection ; **l'état s'éteint seul au bout de 15 minutes** sans nouvelle (un téléphone tué, une batterie vide ne laissent pas le chef « en voiture »). Le cœur le garde **en mémoire seulement**, par appareil.

### 3.4 Comment le chef écrit pour l'oreille

- **Le cœur prévient le chef** : à `driving: true`, un événement dans sa session (« l'utilisateur conduit, par tel téléphone ; écris pour l'oreille ») ; à `false`, le retour à l'écrit.
- **Sa consigne en voiture** : une à trois phrases ; le plus important d'abord ; ni tableau, ni code, ni chemin, ni lien, ni liste de plus de trois éléments ; les nombres et les clés de tickets dits simplement (« le ticket sept cent douze ») ; une seule question à la fois, fermée si possible ; « je te le garde pour le téléphone » pour tout ce qui demande à voir.
- **Une garde du cœur**, parce qu'une consigne ne suffit jamais : `ear_text()` retire du message envoyé à la voiture les blocs de code, les tableaux et le Markdown, remplace un bloc retiré par « (du détail, sur ton téléphone) », et coupe au-delà de 600 caractères. Le message entier reste dans la conversation du chef, sur le poste et le téléphone.

### 3.5 Les réponses dictées

- La voiture dicte (Google Assistant ou Gemini, Siri), relit et envoie ; l'app reçoit **un texte**, jamais l'audio.
- Le natif l'envoie au chef par `overseer_send`, avec **`origin: "car"`** (§ 4.3). Le chef montre, comme toujours, le message « tel qu'il l'a reçu », marqué « dicté en voiture », puis ce qu'il fait, avec « Annuler ».
- **En voiture, le chef redit ce qu'il a compris avant d'agir** quand l'action n'est pas annulable, et la fait dans le même tour ; pour une action annulable, il la fait et le dit (« j'ai lancé ESC-712, dis "annule" pour revenir »).

### 3.6 L'app fermée

- **Sous Android** : le push à haute priorité de MO-6 réveille le `FirebaseMessagingService` même app tuée (pas arrêtée de force) ; la réponse et « marquer comme lu » arrivent à un `BroadcastReceiver`. La WebView ne démarre jamais.
- **Sous iOS, avec la communication** : l'extension de service de notification (MO-6) et l'extension d'intentions tournent sans l'app.
- **Sous iOS, avec la conversation à la voix** : CarPlay lance l'app (en arrière-plan sur le téléphone, la scène sur la voiture) quand l'utilisateur la touche sur l'écran de la voiture ; fermée, elle ne dit rien : le chef passe alors par les notifications ordinaires (M5), sans texte.

## 4. La sécurité, qui prime

### 4.1 Le conflit avec M5, et les options

M5 dit que **le texte d'un message ne part jamais en notification**, même chiffré de bout en bout, parce que l'écran verrouillé se lit par-dessus l'épaule. Or Android Auto ne lit **que** ce qui est dans la notification, et Siri n'annonce que ce qui l'est. Les options :

1. **Recommandé (C3)** : un **mode voiture**, **éteint par défaut**, à allumer **par espace** dans les réglages de l'app (« Lire les messages du chef en voiture »), avec ce qu'il implique dit en clair (le texte sera lu à voix haute, donc entendu par les passagers, et passera par l'assistant du système, Google ou Apple, qui le lit et dicte la réponse). Puis :
   - **le push ne change pas** : toujours le réveil scellé de M5, **sans le texte** ;
   - **seulement pendant la projection**, à la réception, le natif **va chercher** le message du chef par le tunnel (`overseer_car_feed`, § 5.1), et pose la notification de messagerie avec le texte ;
   - cette notification est **masquée sur l'écran verrouillé** du téléphone (`VISIBILITY_PRIVATE`, avec une version publique sans texte, « Le chef t'a écrit ») **[à vérifier au DHU : ce qu'Android Auto montre d'une notification privée]** ;
   - **à la fin de la projection**, le natif retire les notifications de voiture et n'en laisse qu'une, sans texte ;
   - **jamais sous `servers.contentlessNotifications`** : le mode voiture n'est pas proposé pour cet espace, et le cœur refuse `overseer_car_feed` à un appareil de cet espace.
2. **Le texte scellé dans le push**, quand le cœur sait que le téléphone est en voiture : plus rapide (pas de tunnel à monter), mais le texte « part » en notification, à rebours de M5 même si le relais ne le lit pas, et le cœur peut se tromper d'état. **Écarté.**
3. **Aucun texte** (« Le chef a deux nouvelles ; regarde ton téléphone à l'arrêt ») : fidèle à M5, mais inutile en voiture. **Écarté.**

Sous iOS, la même règle : l'extension de service de notification ne met le texte que si le mode voiture est allumé pour l'espace et que la route audio est celle de la voiture ; l'extension d'intentions va chercher les messages par le tunnel quand Siri le demande. Avec la conversation à la voix, rien ne passe par les notifications : le texte vient par le tunnel, l'app ouverte.

### 4.2 Aucun feu vert depuis la voiture

- **La règle** : rien de ce que M3 met sous le verrou du téléphone ne part d'un message dicté en voiture : **publier, signer, supprimer, autoriser une commande, « Toujours autoriser », approuver un plan, « oui » à un feu vert (`gate`), valider un ticket (un merge), lancer un ticket ou un agent**. Ce qui ne demande pas le verrou reste permis : refuser, répondre à une question qui ne fait pas agir, envoyer un message à un agent, « plus tard », renvoyer un ticket avec un mot, demander des nouvelles (C1 tranche si « lancer » en est, C8).
- **Tenue par le cœur** : chaque tour du chef porte l'origine du message qui l'a déclenché ; si c'est `car`, le cœur **refuse** les outils de feu vert à ce tour (la même liste, du côté du serveur MCP, comme les droits du chef sont déjà tenus par le cœur, `OVERSEER.md` « Son autorité ») et rend au chef : « depuis la voiture : déposé dans À toi ». Le chef **crée alors la demande** dans la boîte (un `confirm` à `gate`), que l'utilisateur règle sur le téléphone, à l'arrêt, sous verrou.
- **Les consignes durables aussi** : « tu peux merger seul », dit en voiture, n'ajoute **aucune consigne** (ce serait un feu vert pour plus tard) ; le chef la propose dans « À toi ». Retirer une consigne (« arrête de merger seul ») reste permis : ça ne fait que retirer un droit.
- **Le chef ne répond pas non plus à la place de l'utilisateur sur la foi d'un message dicté** : « dis oui à tests-e2e » est un feu vert, et va dans la boîte.

### 4.3 Le message dicté est marqué

- `overseer_send` gagne un champ `origin` (`typed`, `dictated`, `car`), posé **par le natif**, jamais par le texte. Le cœur le garde avec le message, l'affiche (« dicté en voiture ») et le passe au chef.
- **Le chef ne le prend jamais pour un feu vert** (§ 4.2), même s'il dit « je confirme », « vas-y, publie » ou « c'est bien moi ».
- Une réponse à une demande de la boîte ne se donne pas depuis la voiture : la voiture n'en montre aucune (§ 3.1).

### 4.4 Les passagers, la dictée, les clés

- **Les passagers** : tout ce qui est lu est entendu ; n'importe qui dans la voiture peut dicter une réponse, sans déverrouiller le téléphone. D'où le mode éteint par défaut, la lecture limitée aux messages du chef écrits pour l'oreille, et aucun feu vert.
- **Les erreurs de la dictée** : la voiture relit avant d'envoyer ; le chef montre ce qu'il a reçu et redit son action non annulable (§ 3.5) ; tout ce qui a du poids va dans la boîte.
- **Les assistants du système** : le texte lu et la réponse dictée passent par Google Assistant ou Gemini, ou Siri ; c'est le prix de la voiture, dit à l'activation (§ 4.1).
- **Les clés restent dans le natif** : la clé de l'appairage et le jeton de chaque espace, au **Keystore** (une clé AES de l'AndroidKeyStore, sans authentification de l'utilisateur, pour servir écran verrouillé, comme aujourd'hui) et au **Trousseau** (`kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`) ; sous iOS, un **groupe de trousseau partagé** avec les extensions (notification, intentions), qui en ont besoin sans l'app. Jamais dans la WebView, jamais en clair sur le disque ; le texte reçu en voiture n'est gardé qu'en mémoire et dans la notification, effacée à la fin de la projection.
- **La révocation** (MO-4) coupe tout : sans jeton, `overseer_car_feed` et `overseer_send` sont refusés.

## 5. La technique

### 5.1 Le chemin d'un message du chef

1. Le chef écrit ; le cœur voit qu'un appareil de l'utilisateur est en voiture (§ 3.3) et que le mode voiture est allumé pour cet espace.
2. Le cœur envoie **le réveil scellé de MO-6** (sorte `overseer`, sans texte, M5) par le relais (FCM, APNs) ou par UnifiedPush (MO-9).
3. Le natif ouvre le réveil par le Rust (`openNotification`, MO-6), vérifie la projection (§ 3.3), **monte le tunnel** (`startTunnel`, le Rust de MO-3) et appelle **`overseer_car_feed`** (nouvelle commande servie : les messages du chef non lus par cet appareil, passés par `ear_text()`, depuis un curseur) ; **sans la WebView**.
4. Il pose la notification de messagerie (§ 1.1) ou donne l'intention entrante (§ 2.2), puis referme le tunnel.

**Le budget** : un `FirebaseMessagingService` a de l'ordre de 10 à 20 secondes, une extension de service de notification 30 secondes et une mémoire bornée (de l'ordre de 24 Mo) **[à vérifier]** ; monter le tunnel (boringtun et smoltcp) et un appel HTTP tiennent dans ce budget sur le banc de MO-3, à mesurer sur un vrai téléphone. Si le délai est dépassé, la notification reste celle de M5, sans texte.

### 5.2 La réponse, en arrière-plan

Le `BroadcastReceiver` (Android) ou l'extension d'intentions (iOS) reçoit le texte, monte le tunnel, appelle `overseer_send` (`origin: "car"`) puis, sous Android, **remet à jour la notification** (la réponse ajoutée à la conversation, sinon Android Auto garde un indicateur d'envoi). « Marquer comme lu » appelle `overseer_car_seen` (le curseur avance). En cas d'échec (pas de réseau), la réponse est gardée **en mémoire** et réessayée trois fois, puis la notification dit « pas envoyé ».

### 5.3 Ce que le plugin natif doit apprendre à faire seul

`apps/mobile-plugin` sait déjà, sans la WebView : ouvrir une notification scellée (MO-3, MO-6), garder les clés (MO-4), monter un tunnel (MO-3). Il doit apprendre :

- **Android** : observer et interroger `CarConnection` ; un client natif de l'API dans le tunnel (deux commandes : `overseer_car_feed`, `overseer_send`, plus `device_car_state` et `overseer_car_seen`), sans passer par le proxy de la WebView ; le constructeur de la notification `MessagingStyle` (la `Person` du chef, robot ; un raccourci de conversation) ; le `BroadcastReceiver` des réponses et du lu ; le retrait des notifications de voiture à la fin de la projection ; `automotive_app_desc.xml` et la méta-donnée du manifeste ; la dépendance `androidx.car.app:app` (pour `CarConnection` seulement, version 1.7.0 stable).
- **iOS (communication)** : l'extension d'intentions (les trois intentions) ; la donation dans l'extension de service de notification ; le groupe de trousseau partagé ; la ligne `CPListTemplate` de la scène CarPlay ; les entrées `NSUserActivityTypes` et l'autorisation dans les entitlements.
- **iOS (conversation à la voix, en recours)** : la scène CarPlay, `CPVoiceControlTemplate`, la reconnaissance sur l'appareil (`requiresOnDeviceRecognition`), la synthèse, la session audio tenue seulement pendant l'échange.
- **Le Rust** (`crates/escouade-mobile`) : une fonction d'appel de l'API dans le tunnel, exposée par UniFFI, pour que Kotlin et Swift n'aient pas à refaire HTTP par-dessus smoltcp.
- **Le cœur** : `device_car_state`, `overseer_car_feed`, `overseer_car_seen` servis (portée `phone`, `REMOTE.md` « Les portées des appareils ») ; `origin` sur `overseer_send` ; le refus des feux verts à un tour d'origine `car` ; l'événement « en voiture » du chef ; `ear_text()` ; la stratégie `servers.carMode`. La liste des commandes reste unique (`remoteapi/list.rs`).

### 5.4 Les dépendances

- **MO-6 (ESC-572), les notifications** : le relais, le réveil scellé, le `FirebaseMessagingService`, l'extension de service de notification. **Rien de la voiture ne marche sans lui.**
- **Le tunnel** (MO-2, MO-3, faits) et **l'appairage** (MO-4, fait) : le natif monte le tunnel sans l'app ; les clés au Keystore et au Trousseau.
- **UnifiedPush** (MO-9, en option) : le même réveil par ntfy ; CAR-5 le branche s'il existe.
- **MO-8, la publication** : le test fermé de Google Play et TestFlight, où la voiture s'essaie.
- **L'Overseer** (ESC-462, ESC-468) et l'API distante : faits ; `overseer_send` et `overseer_view` sont déjà servis.

## 6. Les essais

| Quoi                                                                                                                                                      | Où                                                                                                                                                      | En CI                                                                        |
| --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Le cœur : le refus des feux verts à un tour `car`, les consignes refusées, `ear_text()`, `overseer_car_feed` et ses refus (révocation, `contentlessNotifications`, `servers.carMode`), l'état qui s'éteint seul | nextest                                                                                                                                                 | **Oui** : c'est la partie qui protège, elle est entièrement testable           |
| Le Rust du téléphone : l'appel de l'API dans le tunnel, contre le pair de test de MO-3                                                                     | nextest de `crates/escouade-mobile`                                                                                                                    | **Oui** (job Linux de la crate)                                               |
| Android : la notification construite (style, actions, sémantiques, visibilité), le receiver, le retrait en fin de projection                              | tests unitaires Kotlin (Robolectric) **[à vérifier : Robolectric dans le plugin]**                                                                     | **Oui** (job Linux)                                                           |
| Android Auto de bout en bout : la notification annoncée, lue, la réponse dictée (le DHU accepte un micro ou des commandes), le lu                          | **Desktop Head Unit** (_Test using the Desktop Head Unit_ : Windows, macOS, Linux ; l'émulateur ou un vrai téléphone par USB ou ADB ; ligne de commande) | **Non** au départ : le DHU veut un écran et l'app Android Auto du Play Store sur l'émulateur ; procédure locale écrite, scriptée autant que possible |
| iOS : l'extension d'intentions et la donation                                                                                                             | tests XCTest                                                                                                                                            | **Oui** (job Mac, sans signer)                                                |
| CarPlay de bout en bout                                                                                                                                   | **Simulateur CarPlay** (Simulator › I/O › External Displays › CarPlay, ou _CarPlay Simulator_ des Additional Tools for Xcode) ; **demande l'autorisation sur le profil**, d'après les forums **[tiers]** | **Non** : un Mac, l'autorisation d'Apple, et un simulateur que des versions de Xcode ouvrent en noir **[tiers]** |
| **En voiture**                                                                                                                                            | **L'utilisateur**, avec sa voiture (Android Auto ou CarPlay), l'app du test fermé ou de TestFlight                                                      | Non : un geste de l'utilisateur, déposé dans « À toi » par chaque ticket       |

Apple et Google disent tous deux de ne pas se fier au seul simulateur : l'essai en voiture fait partie du ticket, pas de l'après.

## 7. La publication

- **Google Play** (_Distribute to cars_) : dans la Play Console, Paramètres avancés › Facteurs de forme › Ajouter « Android Auto », puis une version avec Android Auto sur une piste de test. **Au test fermé déjà prévu (MO-8)**, la revue ne bloque pas : l'utilisateur s'en sert ; en production, elle bloque, et décide (§ 1.4). Le retirer : retirer Android Auto de toutes les versions actives, puis « Retirer » le facteur de forme.
- **Apple** : la demande de l'autorisation (§ 2.1), par le compte de développeur de l'utilisateur, avec la description de l'app et de son usage en voiture (le texte, préparé par CAR-6) ; puis l'autorisation ajoutée à l'App ID, le profil régénéré, TestFlight, et **l'App Review**, qui revoit CarPlay avec l'app.
- **Ce qui attend l'utilisateur** :
  - ses réponses aux choix C1 à C8 (§ 9), d'une seule réponse s'il les prend tels que recommandés ;
  - **la demande de l'autorisation CarPlay** à Apple (son compte, l'addendum à accepter) ; à faire **tôt** : c'est le plus long délai, et le seul qu'on ne maîtrise pas ;
  - **ajouter le facteur de forme Android Auto** dans la Play Console, sur le test fermé ;
  - **l'essai en voiture**, sous Android Auto et sous CarPlay, d'après les étapes que lui laissera chaque ticket ;
  - l'option « Sources inconnues » d'Android Auto (paramètres de développeur) pour essayer une version qui ne vient pas du Play Store, s'il le veut avant le test fermé.
- **Les agents ne créent aucun compte, aucun projet, aucune demande chez Apple ou Google**, ne lisent ni ne copient aucune clé ; ils écrivent le texte des demandes et des fiches.

## 8. Le découpage en tickets

Estimations en **heures d'agent** (Claude Code, six agents en parallèle), jamais en jours-homme.

| Ticket                                           | Ce qu'il fait                                                                                                                                                                                                                                                                                                                | Dépend de                                   | Heures |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------- | ------ |
| **CAR-1** · le cœur en voiture                    | `origin` sur `overseer_send` ; `device_car_state` (l'état en mémoire, éteint seul à 15 min) ; `overseer_car_feed` et `overseer_car_seen` servis (portée `phone`) ; `ear_text()` ; l'événement « en voiture » et la consigne du chef ; **le refus des feux verts et des consignes à un tour `car`, la demande déposée dans « À toi »** ; la stratégie `servers.carMode` ; le refus sous `contentlessNotifications` ; nextest ; spec et `REMOTE.md` | les réponses C2, C3, C5, C8                 | 4 h    |
| **CAR-2** · le Rust du téléphone                  | L'appel de l'API dans le tunnel exposé par UniFFI (`crates/escouade-mobile`), sans la WebView ; ses tests contre le pair de test                                                                                                                                                                                             | —                                           | 2 h    |
| **CAR-3** · Android Auto                          | `CarConnection` ; la notification `MessagingStyle` (la `Person` du chef, robot, ses actions, `VISIBILITY_PRIVATE`) ; le receiver des réponses et du lu ; le retrait en fin de projection ; `automotive_app_desc.xml` ; le réglage « Lire les messages du chef en voiture » par espace ; Robolectric ; la procédure du DHU ; l'essai en voiture déposé dans « À toi » | **MO-6 (ESC-572)**, CAR-1, CAR-2, C1, C3    | 5 h    |
| **CAR-4** · CarPlay                               | **Communication** : l'extension d'intentions (les trois intentions), la donation et le texte conditionnel dans l'extension de service de notification, le groupe de trousseau, la ligne `CPListTemplate`, XCTest ; **ou, en recours, conversation à la voix** : la scène, `CPVoiceControlTemplate`, la voix sur l'appareil (+1 h) ; l'essai en voiture déposé dans « À toi » | **MO-6 (ESC-572)**, CAR-1, CAR-2, C4, **l'autorisation d'Apple** pour l'essai | 5 h (6 h) |
| **CAR-5** · UnifiedPush en voiture (en option)    | Le même chemin depuis le réveil par ntfy                                                                                                                                                                                                                                                                                     | MO-9, CAR-3                                 | 1 h    |
| **CAR-6** · la publication                        | Le texte de la demande d'autorisation à Apple (à déposer par l'utilisateur), la fiche des stores, la page de confidentialité (ce que lisent les assistants), les notes pour l'App Review et la revue Android Auto ; les gestes de l'utilisateur déposés dans « À toi »                                                          | CAR-3, CAR-4, MO-8                          | 1,5 h  |

- **L'ordre, à six agents** : **CAR-1 et CAR-2 en parallèle** (4 h), qui n'attendent pas MO-6 ; puis, **une fois MO-6 fait**, **CAR-3 et CAR-4 en parallèle** (5 h, 6 h en recours) ; puis **CAR-6** (1,5 h) ; CAR-5 quand MO-9 existe. **Total : environ 17,5 h d'agent** (18,5 h avec CAR-5, une heure de plus avec la voix sous iOS), **de l'ordre de 11 h de bout en bout** (le plus long chemin : CAR-1, CAR-4, CAR-6), sans compter MO-6 ni les délais d'Apple.
- **Le texte de la demande à Apple peut partir avant tout le reste** : CAR-6 peut l'écrire seul, d'abord (une demi-heure), pour que le délai d'Apple coure pendant CAR-1 à CAR-4 (C4).
- **Ce qui attend l'utilisateur** : § 7 ; et, comme pour l'app mobile, chaque ticket laisse son essai réel dans « À toi », avec ses étapes.

## 9. Les choix

Ils restent à l'utilisateur, déposés en une demande dans « À toi ». Aucune décision n'est écrite avant ses réponses.

1. **C1. Le périmètre et l'ordre** : **Recommandé** : Android Auto et CarPlay dans la 2.4, **Android d'abord** (pas d'autorisation à attendre, le test fermé suffit à l'utilisateur), CarPlay dès que l'autorisation d'Apple arrive. Autres : Android seul ; attendre que Google ouvre la messagerie à modèles à la production. _Débloque_ CAR-1 à CAR-3.
2. **C2. Ce que la voiture expose** : **Recommandé** : la conversation du chef seule, qui résume les agents arrêtés, les nouvelles et la boîte. Autres : le chef et une conversation par agent ; le chef et la boîte lue demande par demande. _Débloque_ CAR-1.
3. **C3. Le conflit avec M5** : **Recommandé** : un mode voiture éteint par défaut, par espace ; le push reste sans texte ; le texte va chercher par le tunnel seulement pendant la projection ; masqué sur l'écran verrouillé ; retiré en fin de projection ; jamais sous `contentlessNotifications`. Autres : le texte scellé dans le push quand le cœur sait le téléphone en voiture ; aucun texte. _Débloque_ CAR-1, CAR-3, CAR-4.
4. **C4. La catégorie de CarPlay** : **Recommandé** : demander **la communication** (SiriKit, l'annonce par Siri, app fermée, le même modèle qu'Android), et la conversation à la voix en recours si Apple refuse ou ne répond pas en six semaines ; la demande déposée au plus tôt, son texte écrit d'abord. Autre : la conversation à la voix d'emblée (plus sûre d'être acceptée, mais app ouverte seulement, sans annonce spontanée). _Débloque_ la demande à Apple et CAR-4.
5. **C5. Une stratégie pour les espaces de travail** : **Recommandé** : `servers.carMode` (`true` par défaut, forcé à `false` sous `contentlessNotifications`), pour qu'un espace de travail puisse interdire la lecture en voiture. Autre : pas de stratégie, le mode voiture par espace suffit. _Débloque_ CAR-1.
6. **C6. La messagerie à modèles de la Car App Library** : **Recommandé** : pas dans la 2.4 (bêta, production fermée, rien de plus pour l'oreille) ; à revoir quand Google l'ouvrira. Autre : la faire pour le test fermé (l'historique à l'écran de la voiture). _Débloque_ CAR-3 tel qu'il est écrit.
7. **C7. Android Automotive OS** : **Recommandé** : hors de la 2.4 (une app dans la voiture, sans WebView, un appairage de plus dans une voiture qu'on prête ; un parc étroit). Autre : un ticket d'étude à part. _Débloque_ : rien, il ferme une porte.
8. **C8. Ce que la voiture ne peut pas faire** : **Recommandé** : **tout ce que M3 met sous verrou**, lancer un ticket ou un agent compris (une voix non authentifiée ne vaut pas le verrou, et lancer dépense du quota) ; le chef le range dans « À toi ». Autre : la liste minimale du ticket (publier, signer, supprimer, autoriser une commande, valider un ticket qui fusionne), en laissant « lance ESC-712 » permis depuis la voiture. _Débloque_ CAR-1.

## 10. Ce qui serait un échec, même si tout marche

- Un feu vert parti d'une phrase dite en voiture, par l'utilisateur, un passager ou une dictée fautive.
- Le texte d'un message du chef sur l'écran verrouillé, ou dans une notification hors de la projection.
- Une clé de l'appairage hors du Keystore ou du Trousseau, ou dans la WebView.
- Un chef qui lit en voiture un tableau, un chemin ou une commande.
- Une version de production refusée par Google ou Apple parce qu'on y a mis ce qu'ils interdisent : la production attend leur avis, le test fermé sert d'abord.

## 11. Les sources (lues le 2026-11-27)

**Android (developer.android.com, sauf mention)**

- _Extend messaging notifications to Android Auto_ : https://developer.android.com/training/cars/communication/notification-messaging (et https://developer.android.com/training/cars/messaging)
- _Build templated messaging experiences for Android Auto_ : https://developer.android.com/training/cars/communication/templated-messaging
- _Communication apps_ (design) : https://developer.android.com/design/ui/cars/guides/app-types/communications
- _Car app quality_ : https://developer.android.com/docs/quality-guidelines/car-app-quality
- _Distribute to cars_ : https://developer.android.com/training/cars/distribute
- _Notifications on Android Automotive OS_ : https://developer.android.com/training/cars/notifications
- _Connection API_ : https://developer.android.com/training/cars/apps/library/connection-api ; référence de `CarConnection` : https://developer.android.com/reference/androidx/car/app/connection/CarConnection
- _Car App_ (versions de Jetpack) : https://developer.android.com/jetpack/androidx/releases/car-app ; `CarAppApiLevels` : https://developer.android.com/reference/androidx/car/app/versioning/CarAppApiLevels
- _Test using the Desktop Head Unit_ : https://developer.android.com/training/cars/testing/dhu
- _What's new in Android for Cars_ (Android Developers Blog, mai 2026) : https://android-developers.googleblog.com/2026/05/android-for-cars-unifying-platforms-premium-experiences.html ; _New in-car app experiences_ (Google I/O 2025) : https://android-developers.googleblog.com/2025/05/android-for-cars-google-io-2025.html

**Apple (developer.apple.com, sauf mention)**

- _CarPlay Developer Guide_ (édition d'octobre 2026) : https://developer.apple.com/download/files/CarPlay-Developer-Guide.pdf
- _Requesting CarPlay entitlements_ : https://developer.apple.com/documentation/carplay/requesting-carplay-entitlements
- _CarPlay_ : https://developer.apple.com/carplay/
- _CarPlay App Programming Guide_ (édition archivée) : https://developer.apple.com/carplay/documentation/CarPlay-App-Programming-Guide.pdf
- _SiriKit › Messaging_ : https://developer.apple.com/documentation/sirikit/messaging ; `INSendMessageIntent` : https://developer.apple.com/documentation/intents/insendmessageintent ; _Improving interactions between Siri and your messaging app_ : https://developer.apple.com/documentation/sirikit/improving-interactions-between-siri-and-your-messaging-app
- _Implementing communication notifications_ : https://developer.apple.com/documentation/usernotifications/implementing-communication-notifications ; WWDC21 _Send communication and Time Sensitive notifications_ : https://developer.apple.com/videos/play/wwdc2021/10091/
- _Using the CarPlay simulator_ : https://developer.apple.com/documentation/carplay/using-the-carplay-simulator
- Les règles de l'App Review : https://developer.apple.com/app-store/review/guidelines/ (CarPlay y renvoie au guide ci-dessus)

**Tiers, pour mémoire (jamais seuls à fonder une conclusion)**

- Forums d'Apple sur les délais des autorisations : https://developer.apple.com/forums/thread/848946, https://developer.apple.com/forums/thread/847440, https://developer.apple.com/forums/thread/736111 ; sur l'annonce dans CarPlay : https://developer.apple.com/forums/thread/783730 ; sur le simulateur d'une app de communication : https://developer.apple.com/forums/thread/664874
- La catégorie de la voix dans iOS 26.4 : https://www.macrumors.com/2026/02/18/ios-26-4-carplay-support/
- Le retour d'OpenBubbles sur `ConversationItem` : https://github.com/mumeicopperhead/openbubbles-app
