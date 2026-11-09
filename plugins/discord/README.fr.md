# Discord

Les tickets à tester et les agents qui t’attendent, dans un salon Discord. Un ticket qui arrive dans « À tester » y poste `🧪 APP-7 to test: …`, un agent qui attend ta réponse y poste `✋ Mila waits for you`.

**Pour qui** : toi ou ton équipe, quand tu n’as pas Escouade sous les yeux et que tu veux savoir qu’il y a quelque chose à tester ou à débloquer.

**Ce qu’il demande** : un webhook d’un salon Discord, que tu crées dans les réglages du salon. Pas de compte Discord à relier, pas de bot, pas de programme.

**Ce qu’il peut faire** :

- recevoir deux événements d’Escouade : un ticket qui change de colonne, un agent qui change de statut ;
- envoyer un message vers `discord.com`, et nulle part ailleurs ;
- garder un secret : l’adresse du webhook, dans le trousseau du système.

Il n’a pas de programme : il déclare deux webhooks, et c’est Escouade qui les envoie. Rien de lui ne tourne sur ta machine.

La suite est dans les onglets de la documentation :

1. Installer : créer le webhook et le coller dans Escouade.
2. Utiliser : ce qu’il poste, quand, et ses limites.
3. Permissions : ce qui part vers Discord et ce qui n’y part jamais.
4. Dépannage : ce qui se passe quand rien n’arrive.
5. Développer : son dossier, ses tests.
