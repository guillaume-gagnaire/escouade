# OpenAI

Le service de voix OpenAI dans Escouade. Avec ta clé OpenAI, Escouade lui confie la dictée, la transcription en temps réel et la voix qui lit les réponses de tes agents.

C’est un plugin de la main d’Escouade (`dev.escouade.openai`) : il a sa version, se coupe et se désinstalle comme les autres. Pour qui : toi, si tu veux dicter ou parler à tes agents avec OpenAI plutôt qu’avec Whisper et Piper, qui tournent sur ta machine sans compte.

## Ce qu’il demande

- Escouade 2.2.0 ou plus récent.
- Un compte OpenAI et une clé d’API (https://platform.openai.com/api-keys). Chaque appel est facturé à ce compte.
- Rien à installer de plus.

## Ce qu’il peut faire

- joindre `api.openai.com`, et aucun autre hôte ;
- recevoir ta clé OpenAI à chaque appel, sans la garder : elle reste dans le trousseau du système, et Escouade ne la donne qu’à ce plugin ;
- envoyer à OpenAI le son de ce que tu dictes, et, si tu choisis sa voix, le texte que la conversation lit à voix haute ;
- rien d’autre : pas de fichier, pas de commande, pas de secret à lui, ni ton projet ni ton code.

## Les onglets

- **Installer** : avoir le plugin, créer la clé, la donner à Escouade.
- **Utiliser** : la dictée, le temps réel, la voix de l’agent, les modèles et les voix.
- **Permissions** : ce qui part chez OpenAI, quand, et ce qui n’y part jamais.
- **Dépannage** : les messages qu’il affiche, et quoi faire.
- **Développer** : son dossier, ses tests, le lancer seul.
