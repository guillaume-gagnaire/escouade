# Permissions

Unharnessed ne demande jamais rien, et son noyau n’est pas un bac à sable. Cette page le dit sans détour.

## Ce que fait un agent Unharnessed

- **Il ne demande jamais.** Aucune carte de permission n’arrive d’un agent Unharnessed : pas avant une commande, pas avant une écriture, pas avant un accès au réseau. Escouade ne voit rien passer avant que ce soit fait.
- **Tout tourne avec tes droits.** Le code Python du modèle et les commandes qu’il lance (`bash(...)`) tournent avec ton compte, sur tout le disque, avec le réseau. Rien ne le tient au dossier de l’agent.
- **Le noyau n’est pas un bac à sable.** C’est un processus Python ordinaire. Le README d’Unharnessed le dit lui-même : utilise-le dans un conteneur, une machine virtuelle ou une copie jetable.

C’est pourquoi son seul mode dans Escouade est **Bypass** : « Unharnessed ne demande jamais : son code Python et ses commandes tournent avec tes droits, sur tout le disque et le réseau, sans bac à sable. »

## Ce qu’il faut faire

- Ne lance un agent Unharnessed que dans un **worktree jetable**, un **conteneur** ou une **machine virtuelle**, sur un dépôt que tu peux perdre.
- Ne le lance pas sur une machine qui garde des secrets ou des identifiants que le modèle pourrait lire : il peut lire tout ce que tu peux lire.
- Préfère un compte d’Unharnessed à part (un dossier à lui, voir Installer) et un fournisseur dont la clé peut être perdue.

## Ce qu’Escouade tient

- **Le réglage « Autoriser le mode Bypass »** (Réglages › Agents), éteint par défaut. Éteint, un agent Unharnessed ne démarre pas et le dit : « Unharnessed ne tourne qu’en Bypass (il ne demande jamais avant d’agir) : active « Autoriser le mode Bypass » dans Réglages › Agents, ou choisis un autre harness. »
- **Les discussions de l’onglet Docs** : refusées, Unharnessed n’a pas de mode qui ne change rien.
- **Les projets privés** : dès qu’un de tes projets est privé, aucun agent Unharnessed ne démarre, puisque la clôture ne tiendrait pas.
- **Le jeton d’Escouade** : il va à la session par ACP, dans un en-tête, jamais dans les arguments.

## Pourquoi Escouade 2.2.2 au moins

Jusqu’à la 2.2.1, sans Bypass permis, un nouvel agent recevait le mode « Demander », qu’Unharnessed n’a pas, et rien ne le refusait : l’agent démarrait, et tournait comme Unharnessed tourne toujours, sans rien demander (constaté par ESC-484). La 2.2.2 juge le mode où le harness tourne vraiment, son seul mode, Bypass (ESC-489) : ses agents sont montrés en Bypass, et ne démarrent qu’avec « Autoriser le mode Bypass ». Le plugin ne s’installe pas sur une version plus ancienne.

## Ce que le plugin lui-même lance

Ce plugin n’a pas de programme : rien de lui ne s’exécute, son manifeste est lu. Il ne déclare aucune permission : ni réseau, ni secret, ni fichier. Ses agents lancent `unharnessed --mode acp`, le programme de ta machine, avec tes droits ; c’est ce que la fenêtre d’installation montre. Il ne propose aucune commande d’installation.
