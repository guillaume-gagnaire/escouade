# Permissions

Cette page dit ce que le plugin fait faire sur ta machine, et ce qu’il ne peut pas faire. Elle dit d’abord ceci : **Prime Agent ne demande jamais rien, et il n’y a pas de bac à sable.**

## Ce que fait un agent Prime Agent

Le modèle écrit du code Python, et Prime Agent l’exécute dans son noyau, avec tes droits d’utilisateur, sans rien demander ni à toi ni à Escouade. Ce code peut lire, écrire et effacer n’importe quel fichier que tu peux toucher, sur tout le disque et pas seulement dans le dossier de l’agent ; lancer n’importe quelle commande ; aller sur le réseau ; lire tes variables d’environnement et tes clés. Le README de Prime Agent le dit lui-même : ses process de noyau et de worker « ne sont pas un bac à sable de sécurité ».

Escouade ne voit rien passer avant : aucune carte de permission ne s’affiche, aucune règle d’Escouade ne s’applique. Il ne voit le code d’une cellule qu’au moment où elle s’exécute.

## Comment s’en protéger

- Ne fais travailler un agent Prime Agent que sur un dépôt et des instructions de confiance.
- Fais-le travailler dans un **worktree jetable** (le worktree d’un ticket), que tu relis avant de fusionner.
- Pour du code ou des instructions dont tu n’es pas sûr, fais tourner Escouade et Prime Agent dans un **conteneur** ou une **machine virtuelle**, sans tes clés ni tes dossiers personnels.
- Garde « Autoriser le mode Bypass » coupé si tu ne veux pas de ce genre d’agent : un agent Prime Agent en Bypass ne démarre pas sans lui.

## Le mode Bypass dans Escouade

Prime Agent n’a qu’un mode, Bypass, qui dit ce qu’il est. Sans « Autoriser le mode Bypass » (Réglages › Agents, la stratégie `permissions.allowBypass` ou `.escouade.yml`), un agent ne démarre pas et le dit : « Prime Agent ne tourne qu’en Bypass (il ne demande jamais avant d’agir) : active « Autoriser le mode Bypass » dans Réglages › Agents, ou choisis un autre harness. »

**Pourquoi Escouade 2.2.2 au moins** : jusqu’à la 2.2.1, sans Bypass permis, un nouvel agent recevait le mode « Demander », que Prime Agent n’a pas, et rien ne le refusait : l’agent démarrait, et Prime Agent tournait comme toujours, sans rien demander, alors que l’agent était affiché en « Demander » (constaté pour Unharnessed par ESC-484, et de même ici). La 2.2.2 le corrige (ESC-489) ; le plugin ne s’installe pas sur une version plus ancienne.

## Ce qu’il lance

Ce plugin n’a pas de programme : rien de lui ne s’exécute, son manifeste est lu. Il ne déclare aucune permission : ni réseau, ni secret, ni fichier.

Ses agents lancent `prime-agent --mode acp --no-session`, le programme de ta machine, avec tes droits. C’est ce que la fenêtre d’installation montre. Ce programme démarre au besoin le **démon** de Prime Agent (`prime-agent --mode daemon`), un pour toutes les sessions de l’utilisateur, qui lui survit : voir l’onglet Dépannage.

## Ce qui est lu, et les secrets

Le plugin lui-même ne lit rien et ne reçoit rien. Le plugin ACP qui sert Prime Agent reçoit le jeton MCP de chaque agent et le donne à sa session dans un en-tête, sur son entrée, jamais dans ses arguments. Le code de son modèle, lui, peut lire ce qu’il veut, ce jeton compris : il tourne avec tes droits.

## La clôture d’un projet privé

Elle ne peut pas tenir : Prime Agent lit et écrit les fichiers lui-même (`"fs": false` ; essayé, il n’a jamais demandé les fichiers au client). Escouade refuse donc de lancer un agent Prime Agent dès qu’il a une clôture, c’est-à-dire dès qu’un de tes projets est privé, quel que soit le projet de l’agent.

## Ce qu’il ne peut pas faire

- exécuter quoi que ce soit lui-même : il n’a pas de programme ;
- lancer autre chose que ce que son manifeste décrit, que la fenêtre d’installation t’a montré ;
- tenir une discussion en lecture seule, ou tourner à côté d’un projet privé.
