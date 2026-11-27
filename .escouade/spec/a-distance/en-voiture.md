---
title: "Escouade en voiture (brouillon, ESC-696, ESC-698)"
subject: "a-distance"
sources: ["docs/CAR.md (ESC-696 puis ESC-698 : l'étude, ses sources officielles lues le 2026-11-27, les choix C1 à C12, le découpage CAR-1 à CAR-7)","docs/MOBILE.md (M3 le verrou, M5 ce qui part en notification, M9 la dictée, MO-3 le tunnel, MO-6 les notifications, le plugin natif)","docs/OVERSEER.md (l'autorité du chef)","docs/REMOTE.md (l'API distante, ses portées, les commandes du poste dont la voix)","Spec : Dictée, Conversation vocale, L'Overseer › Sa voix, Ce qui change de machine, Réglages imposés (ce que fait la voix d'Escouade aujourd'hui)","Demande de l'utilisateur du 2026-11-27, relayée par le chef : « Est-ce qu'il y a un monde où l'app mobile Android est compatible Android Auto en mode overseer ? », puis « Étude pour la 2.4, avec Apple aussi »","Direction de l'utilisateur après ESC-696 (2026-11-27) : la liste des agents ou l'Overseer, puis une conversation audio avec l'agent choisi, par les réglages de la voix d'Escouade (ElevenLabs, Whisper…)","developer.android.com : Extend messaging notifications to Android Auto ; Build templated messaging experiences ; Record from the car microphone et CarAudioRecord ; Communicate with the app by voice ; Set up your project ; Car app quality ; Distribute to cars ; Connection API ; Test using the Desktop Head Unit (lus le 2026-11-27)","developer.apple.com : CarPlay Developer Guide (octobre 2026) ; l'autorisation carplay-voice-based-conversation ; Requesting CarPlay entitlements ; Using the CarPlay simulator (lus le 2026-11-27)"]
tickets: ["ESC-696","ESC-698","ESC-572"]
decisions: []
compared: null
draft: true
---

> **Brouillon, rien n'est tranché** sauf C2 (par l'utilisateur, le 2026-11-27 : la liste des agents et l'Overseer). Étude d'ESC-696, révisée par ESC-698, pour la 2.4 (`docs/CAR.md`). Les choix C1 à C12 attendent l'utilisateur dans la boîte « À toi » ; cette page dit ce que l'étude recommande, pas ce que fait le produit.

## Ce que c'est

Dans la voiture, comme pour une messagerie, l'utilisateur voit **la liste de ses agents et l'Overseer**, en choisit un, puis lui parle et l'entend répondre. Pas d'écran de l'app ni de Kanban : Android Auto et CarPlay n'acceptent que des catégories fixées et des modèles d'écran imposés.

## Comment (recommandé)

- **CarPlay** : la catégorie de **la conversation à la voix** (iOS 26.4), seule. La liste en racine (`CPListTemplate` : l'Overseer, puis les agents qui t'attendent, qui travaillent, finis depuis 24 h ; un nom, un projet, un état, aucun texte), puis l'écran de la voix (`CPVoiceControlTemplate`) : deux écrans sur trois. L'app fait la reconnaissance et la synthèse : **la voix d'Escouade** parle. Elle n'attend pas les notifications (MO-6).
- **Android Auto** n'a aucune catégorie pour une conversation à la voix menée par l'app. Le repli : **chaque agent et l'Overseer comme une conversation de messagerie**, lue et dictée par l'assistant de Google. La messagerie à modèles (`ConversationItem`, en bêta, test interne et fermé seulement) montre la liste pour le test fermé ; les notifications (`MessagingStyle`) restent la base et le repli en production.
- **La voix d'Escouade, par le cœur** : le téléphone capte et joue ; le son passe par le tunnel chiffré ; le cœur entend et parle avec Réglages › Voix (Whisper ou le service en ligne réglé ; Piper ou la voix réglée). **Les clés des fournisseurs restent sur le cœur**, jamais sur le téléphone ; sur un serveur, la clé d'une voix en ligne y est posée par un réglage de l'espace. Repli : la reconnaissance de l'iPhone quand le cœur n'entend plus. Aujourd'hui la voix est « du poste » : la rendre au cœur sans fenêtre est un ticket.
- **M9** (« l'audio ne part jamais au cœur ») a une exception, pour la seule voix de la voiture : le son ne part que pendant que l'utilisateur parle, n'est jamais gardé, puis va au fournisseur réglé comme sur le poste. La dictée de l'app reste M9.
- **Ce qui est dit d'un tour** : le passage pour l'oreille (`<spoken>`) que l'agent écrit déjà en conversation vocale, prévenu que l'utilisateur conduit, sous une garde du cœur qui retire code, tableaux et chemins ; sans passage, « C'est fait. La réponse est sur ton téléphone. »

## La sécurité

- **Un mode voiture éteint par défaut**, par espace, qui dit ce qu'il implique (lu à voix haute ; le son vers le cœur et le fournisseur ; Google sous Android Auto). Jamais sous `servers.contentlessNotifications` ; `servers.carMode` permet à un espace de l'interdire ; `voice.dictation` et `voice.onlineAllowed` s'appliquent. Les projets privés sont hors de la voiture.
- **Aucun feu vert depuis la voiture, tenu par le cœur** : un message venu de la voiture est marqué (`origin: car`, posé par le natif) ; le cœur refuse, au chef comme pour un agent, d'autoriser une commande (le « oui » dit à une autorisation compris), d'approuver un plan, un feu vert, de valider ou de lancer un ticket, d'ajouter une consigne. Une demande d'autorisation est dite (« je te la garde pour le téléphone ») et attend dans « À toi », sous verrou. Un message de la voiture ne part pas à un agent qui agit sans demander.
- Sous Android Auto, le texte n'entre dans la notification que pendant la projection, masqué sur l'écran verrouillé, retiré à la fin. Sous CarPlay, l'écran de la voiture ne montre aucun texte.

## Le découpage (estimé)

CAR-1 le cœur en voiture (5 h), CAR-2 le Rust du téléphone (3 h), CAR-3 la voix sans fenêtre (6 h), CAR-4 CarPlay (6 h), CAR-5 Android Auto (6 h, après MO-6), CAR-6 UnifiedPush en option (1 h), CAR-7 la publication (1,5 h) : environ 27,5 h d'agent, de l'ordre de 18 h de bout en bout. Restent à l'utilisateur : ses choix, la demande d'autorisation CarPlay à Apple (à faire tôt), le facteur de forme Android Auto dans la Play Console, la clé d'une voix sur un serveur s'il en veut une, et l'essai en voiture.
