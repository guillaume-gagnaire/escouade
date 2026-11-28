---
number: 381
id: "escouade-en-voiture-2-4-un-whatsapp-des"
title: "Escouade en voiture (2.4) : un « WhatsApp » des agents qui tournent, qu'on appelle à la voix ; tout en vocal ou rien"
date: "2026-11-28"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-721"
pages: ["a-distance/en-voiture"]
tickets: ["ESC-696","ESC-698","ESC-721"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/en-voiture","added":19,"removed":13}]
accepted_at: "2026-11-28T19:48:20+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-696 puis ESC-698 ont étudié Escouade sous CarPlay et Android Auto (docs/CAR.md) et laissé douze choix (C1 à C12) à l'utilisateur, dans la boîte « À toi ». Sous Android Auto, l'étude recommandait un repli : chaque agent comme une conversation de messagerie, lue et dictée par l'assistant de Google. L'utilisateur a répondu le 2026-11-28 (a-56) : « tout recommandé, à part sur l'UI : je veux un whatsapp : liste d'agents qui tournent sur escouade, et je peux les appeler en vocal pour leur dire quoi faire (agents lancés, et overseer quand il sera sorti évidemment) », puis, sur le repli par la messagerie : « nope, on fait tout en vocal ou on fait rien ».

## Décision

1. L'interface en voiture est un « WhatsApp » des agents : la liste des agents qui tournent (l'Overseer en tête quand il existe, puis les agents lancés, ceux qui t'attendent avant ceux qui travaillent ; plus les agents finis ni les projets privés) ; toucher un agent l'appelle, une conversation à la voix avec la voix d'Escouade ; on raccroche ; un appel à la fois.
2. Tout en vocal ou rien : la voiture ne se fait sur un système que si l'appel à la voix s'y fait, sans aucun repli par une messagerie lue et dictée par l'assistant du système. C6 (la messagerie à modèles d'Android Auto) tombe ; ni notifications MessagingStyle de voiture, ni SiriKit de communication.
3. CarPlay : la catégorie de la conversation à la voix, seule (C4). Android Auto : seulement par la catégorie de l'appel (Car App Library CALLING, Core-Telecom CallsManager, l'audio tenu par l'app), en bêta au test fermé ; CAR-5 établit d'abord qu'un appel vers un agent marche avec la voix d'Escouade ; sinon il s'arrête et il n'y a pas d'Android Auto.
4. Les autres choix prennent la version recommandée par ESC-698 : C1 (les deux dans la 2.4, la demande à Apple d'abord), C3, C5 (servers.carMode), C7 (Android Automotive hors 2.4), C8 (aucun feu vert depuis la voiture, lancer compris), C9 (la voix par le cœur, clés sur le cœur, repli sur le téléphone), C10 (exception à M9 pour la seule voix de la voiture), C11 (privés hors de la voiture), C12 (le passage <spoken> sous la garde ear_text()).
5. La sécurité de docs/CAR.md § 7 ne change pas.

## Conséquences

docs/CAR.md réécrit (§ 0, 2, 3.2, 5, 8, 9.3, 10) ; la page « Escouade en voiture » de la spec ; les tickets CAR-1 à CAR-7 en bas d'« À faire », après la 2.3. CAR-5 ne dépend plus de MO-6 (un appel part de l'utilisateur) ; car_feed et car_seen ne sont plus à faire ; CAR-6 devient le rappel d'un agent sous Android Auto, en option, construit seulement sur un « oui » de l'utilisateur. Le risque : Google peut refuser un agent comme correspondant d'un appel (PC-1) ; la production sous Android attend alors, et il n'y a pas de mode voiture sous Android. Restent à l'utilisateur : la demande d'autorisation CarPlay, le facteur de forme Android Auto dans la Play Console, l'essai en voiture.
