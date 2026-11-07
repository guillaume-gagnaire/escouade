# Slack

Les tickets à tester et les agents qui t’attendent, dans le salon Slack de chaque projet. Un ticket qui arrive dans « À tester » y poste `🧪 APP-7 to test: …`, un agent qui attend ta réponse y poste `✋ Mila waits for you`.

**Pour qui** : une équipe qui vit dans Slack, avec un salon par projet, et qui veut y voir ce qu’il y a à tester ou à débloquer.

**Ce qu’il demande** : une app Slack à toi, avec un bot qui a le droit `chat:write`, et son jeton (`xoxb-…`). Le bot doit être invité dans chaque salon où il poste.

**Ce qu’il peut faire** :

- recevoir deux événements d’Escouade : un ticket qui change de colonne, un agent qui change de statut ;
- joindre `slack.com` en https, et nulle part ailleurs ;
- garder un secret : le jeton du bot, dans le trousseau du système ;
- faire tourner son programme dans Escouade, isolé (en WASM) : pas de Node à installer.

Chaque projet poste dans `#escouade-<nom du projet>`.

La suite est dans les onglets de la documentation :

1. Installer : créer le bot Slack et coller son jeton dans Escouade.
2. Utiliser : ce qu’il poste, dans quel salon, et ses limites.
3. Permissions : ce qui part vers Slack et ce qui n’y part jamais.
4. Dépannage : les messages de son journal.
5. Développer : son dossier, ses tests, son build.
