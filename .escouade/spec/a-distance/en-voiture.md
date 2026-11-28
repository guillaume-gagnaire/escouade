---
title: "Escouade en voiture"
subject: "a-distance"
sources: ["docs/CAR.md (ESC-696, ESC-698, puis ESC-721 : l'étude, les choix C1 à C12 tranchés le 2026-11-28, l'appel sous Android Auto, le découpage CAR-1 à CAR-7)","docs/MOBILE.md (M3 le verrou, M5 ce qui part en notification, M9 la dictée, MO-3 le tunnel)","docs/OVERSEER.md (l'autorité du chef)","docs/REMOTE.md (l'API distante, ses portées, les commandes du poste dont la voix)","Réponses de l'utilisateur à a-56 (2026-11-28) : « tout recommandé, à part sur l'UI : je veux un whatsapp : liste d'agents qui tournent sur escouade, et je peux les appeler en vocal pour leur dire quoi faire » ; « nope, on fait tout en vocal ou on fait rien »","developer.android.com : Build calling experiences for Android Auto ; Communication apps ; Core-Telecom et CallsManager ; Telecom framework overview ; ConnectionService, PhoneAccount ; AEP guideline: Android Telecom Framework ; Set up your project ; Car app quality ; Distribute to cars (lus le 2026-11-28)","developer.apple.com : CarPlay Developer Guide (octobre 2026) ; l'autorisation carplay-voice-based-conversation ; Requesting CarPlay entitlements (lus le 2026-11-27)"]
tickets: ["ESC-696","ESC-698","ESC-721","ESC-723","ESC-724","ESC-725","ESC-726","ESC-727","ESC-728","ESC-729"]
decisions: [381]
compared: null
draft: true
---

> **Conçu, pas encore construit** (2.4). Étude d'ESC-696, révisée par ESC-698, puis par ESC-721 d'après les réponses de l'utilisateur du 2026-11-28 (`docs/CAR.md`). Les tickets CAR-1 à CAR-7 la construisent. Rien de ce qui suit n'est encore dans le produit.

## Ce que c'est : un « WhatsApp » des agents

Dans la voiture, l'écran montre **la liste des agents qui tournent** sur Escouade, et l'utilisateur **les appelle** à la voix pour leur dire quoi faire, comme on appelle un contact dans WhatsApp.

- **La liste** : **l'Overseer en tête, quand il existe** ; puis les agents lancés, **ceux qui t'attendent** (une autorisation, une question, un plan) d'abord, puis **ceux qui travaillent**. Les agents finis et arrêtés n'y sont pas, ni les projets privés. 12 lignes au plus ; une ligne n'a qu'un nom, un projet et un état, aucun texte de message.
- **L'appel** : toucher un agent l'appelle. On lui parle ; ce qu'on dit part comme un message de l'utilisateur, marqué « dicté en voiture » ; il répond avec **la voix d'Escouade** ; l'écoute reprend seule. On **raccroche** ; l'agent continue son travail. **Un appel à la fois.** L'appel ne montre aucun texte.
- **Tout en vocal ou rien** : la voiture ne se fait sur un système que si l'appel à la voix s'y fait. Il n'y a **aucun repli par une messagerie** lue et dictée par l'assistant du système (ni Siri, ni l'assistant de Google).

## Sur quels systèmes

- **CarPlay** : la catégorie de la conversation à la voix d'Apple (iOS 26.4), seule. La liste en racine, puis l'écran de l'appel (« Parler », « Raccrocher »). Il faut l'autorisation d'Apple, demandée par l'utilisateur.
- **Android Auto** : **seulement par l'appel**. La catégorie de l'appel d'Android for Cars (une liste de contacts, un appel déclaré au système par Telecom, l'écran d'appel d'Android Auto, l'audio tenu par l'app) est en bêta, ouverte au test interne et au test fermé. Rien ne dit encore qu'un appel peut aller à un agent plutôt qu'à une personne : le premier ticket Android l'établit d'abord. **S'il ne s'établit pas, il n'y a pas d'Android Auto.**
- **Android Automotive OS** : hors de la 2.4.

## La voix

- **Par le cœur** (le poste ou le serveur), avec Réglages › Voix : Whisper ou le service en ligne réglé pour entendre, Piper ou la voix réglée pour parler. Le téléphone capte et joue ; le son passe par le tunnel chiffré du téléphone. **Les clés des fournisseurs restent sur le cœur**, jamais sur le téléphone ; sur un serveur, la clé d'une voix en ligne y est posée par un réglage de l'espace.
- **Repli** : la reconnaissance du téléphone quand le cœur n'entend plus.
- **Une exception à M9** (« l'audio ne part jamais au cœur »), pour la seule voix de la voiture : le son ne part que pendant un appel, pendant que l'utilisateur parle, n'est jamais gardé, puis va au fournisseur réglé comme sur le poste. La dictée de l'app reste M9.
- **Ce qui est dit d'un tour** : le passage pour l'oreille (`<spoken>`) que l'agent écrit déjà en conversation vocale, prévenu que l'utilisateur conduit, sous une garde du cœur qui retire code, tableaux et chemins ; sans passage, « C'est fait. La réponse est sur ton téléphone. »

## La sécurité, qui prime

- **Un mode voiture éteint par défaut**, par espace, qui dit ce qu'il implique (les noms et les réponses dits à voix haute, entendus des passagers ; le son vers le cœur et le fournisseur). Jamais sous `servers.contentlessNotifications` ; `servers.carMode` permet à un espace de l'interdire ; `voice.dictation` et `voice.onlineAllowed` s'appliquent.
- **Aucun feu vert depuis la voiture, tenu par le cœur** : un message venu de la voiture est marqué (`origin: car`, posé par le natif) ; le cœur refuse, au chef comme pour un agent, d'autoriser une commande (le « oui » dit à une autorisation compris), d'approuver un plan, un feu vert, de valider ou de lancer un ticket, d'ajouter une consigne. Une demande d'autorisation est dite (« je te la garde pour le téléphone ») et attend dans « À toi », sous verrou. Un message de la voiture ne part pas à un agent qui agit sans demander.
- Rien sur l'écran verrouillé, rien hors de la projection ; la notification d'un appel sous Android ne porte que le nom de l'agent.

## Le découpage

CAR-1 le cœur en voiture, CAR-2 le Rust du téléphone, CAR-3 la voix sans fenêtre, CAR-4 CarPlay, CAR-5 Android Auto par l'appel (établi d'abord), CAR-6 le rappel d'un agent sous Android Auto (en option, sur un « oui » de l'utilisateur), CAR-7 la publication : environ 28 h d'agent. Restent à l'utilisateur : la demande d'autorisation CarPlay à Apple, le facteur de forme Android Auto dans la Play Console, la clé d'une voix sur un serveur s'il en veut une, et l'essai en voiture.

## La décision

Cette page suit la décision `escouade-en-voiture-2-4-un-whatsapp-des` (proposée par ESC-721, le 2026-11-28) : un « WhatsApp » des agents qui tournent, qu'on appelle à la voix ; tout en vocal ou rien.
