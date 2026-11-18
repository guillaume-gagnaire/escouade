# Prime Agent

Ce plugin fait de Prime Agent, l’agent de code et de recherche open source de Prime Intellect, un harness d’Escouade. Il ne contient pas de code : son manifeste décrit Prime Agent par ACP (`prime-agent --mode acp --no-session`), et le plugin ACP d’Escouade (`dev.escouade.acp`) le lance, lui parle et montre sa conversation : réponses en direct, et chaque cellule de son unique outil, un noyau Python, avec son code et sa sortie.

Il s’adresse à qui a déjà Prime Agent, connecté à un fournisseur de modèles.

**Prime Agent ne demande jamais rien.** Le code Python que son modèle écrit et les commandes qu’il lance tournent avec tes droits, sur tout le disque et le réseau, sans bac à sable. Ses agents n’ont qu’un mode, Bypass : il faut avoir permis le mode Bypass dans Escouade, et les faire travailler dans un worktree jetable, un conteneur ou une machine virtuelle.

## Ce qu’il demande

- Prime Agent installé sur la machine (`prime-agent --version` répond 0.10.3 ou plus), connecté à un fournisseur (`/login` dans son interface, ou un `models.json` à lui).
- Le plugin ACP (`dev.escouade.acp`) installé et allumé : c’est lui qui fait tourner les harness décrits.
- « Autoriser le mode Bypass » allumé dans Réglages › Agents.
- Un compte Prime Agent ajouté dans Réglages › Comptes et harness.

## Ce qu’il peut faire

- Rien de lui ne s’exécute : c’est un plugin sans programme, que la fenêtre d’installation dit « Sans programme ».
- Ses agents lancent le programme `prime-agent --mode acp --no-session` de ta machine, avec tes droits. Ce programme démarre au besoin le démon de Prime Agent, qui lui survit (voir l’onglet Dépannage).
- Il ne demande ni réseau, ni secret, ni fichier, ni compte à lui. Escouade ne lit aucune clé de Prime Agent.

Prime Agent écrit les fichiers lui-même, sans les faire passer par Escouade : dès qu’un de tes projets est privé, aucun agent Prime Agent ne démarre. Il n’a pas de mode qui ne change rien : il ne tient pas de discussion de l’onglet Docs.

## Les onglets

- **Installer** : Prime Agent, le plugin, le mode Bypass, un compte.
- **Utiliser** : son outil Python, son modèle, ce qu’il ne fait pas encore dans Escouade.
- **Permissions** : ce qu’il fait sans rien demander, et comment s’en protéger.
- **Dépannage** : les messages, et son démon.
- **Développer** : le manifeste, ses tests, ce que l’étude a relevé.
