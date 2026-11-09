# Mistral

Mistral dans Escouade, sur un seul compte Mistral : son service de voix et Mistral Vibe comme harness. Avec ta clé Mistral, Escouade lui confie la dictée, la transcription en temps réel et la voix qui lit les réponses de tes agents. Le même plugin fait aussi tourner tes agents Mistral Vibe : Escouade lance le programme `vibe-acp`, et le plugin traduit ce qu’il dit dans la conversation.

C’est un plugin de la main d’Escouade (`dev.escouade.mistral`) : il a sa version, se coupe et se désinstalle comme les autres. Pour qui : toi, si tu veux dicter ou parler à tes agents avec Mistral plutôt qu’avec Whisper et Piper, qui tournent sur ta machine sans compte, ou faire travailler des agents Mistral Vibe.

## Ce qu’il demande

- Escouade 2.2.0 ou plus récent.
- Un compte Mistral et une clé d’API (https://console.mistral.ai/api-keys). Chaque appel est facturé à ce compte.
- Pour Mistral Vibe : le programme lui-même, installé par `uv tool install mistral-vibe`.

## Ce qu’il peut faire

- joindre `api.mistral.ai`, et aucun autre hôte ;
- recevoir ta clé Mistral à chaque appel, sans la garder : elle reste dans le trousseau du système, et Escouade ne la donne qu’à ce plugin (et à Mistral Vibe, pour un compte Vibe qui n’a pas de clé à lui) ;
- envoyer à Mistral le son de ce que tu dictes, et, si tu choisis sa voix, le texte que la conversation lit à voix haute ;
- décrire à Escouade comment lancer Mistral Vibe, et traduire ce que Vibe écrit : Escouade le lance, jamais le plugin ;
- accepter seul, pour un agent Vibe, une écriture dans le dossier de l’agent, et refuser d’office ce qui touche un projet privé ;
- rien d’autre : pas de fichier, pas de commande, pas de secret à lui. Il voit passer ce qu’écrit Vibe, pour le traduire, mais son seul réseau reste `api.mistral.ai`, et il n’y envoie que la voix.

## Les onglets

- **Installer** : avoir le plugin, créer la clé, la donner à Escouade, et brancher Mistral Vibe.
- **Utiliser** : la dictée, le temps réel, la voix de l’agent, les modèles et les voix, les agents Vibe.
- **Permissions** : ce qui part chez Mistral, quand, et ce qui n’y part jamais.
- **Dépannage** : les messages qu’il affiche, et quoi faire.
- **Développer** : son dossier, ses tests, le lancer seul.
