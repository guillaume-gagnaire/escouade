# Réglages : Voix

La page « Voix » règle la dictée (le micro, qui transcrit ce que tu dis, la langue parlée et le raccourci) et la conversation vocale (qui génère la voix, la voix de chaque langue, sa vitesse, son volume et le silence qui envoie ce que tu as dit).

## Ouvrir la page

Ouvre les réglages avec `Ctrl+,` (`⌘,` sous macOS), puis choisis « Voix » (« Dictée des messages : micro, modèle de transcription, langue et raccourci »). Ces réglages sont ceux de l’app, communs à tous les projets.

Les choix de la page s’appliquent au geste, comme les autres réglages, et trois actions aussi : télécharger ou effacer un modèle de Whisper, télécharger ou effacer une voix, et enregistrer ou effacer la clé d’un service en ligne.

L’usage de la dictée et de la conversation est décrit dans [la conversation](chat#dicter-un-message).

## Micro

| Réglage | Effet | Par défaut |
|---|---|---|
| « Micro utilisé » | Le micro qu’écoutent la dictée et la conversation : « Celui du système » (avec son nom entre parenthèses quand le système le donne), ou un des micros branchés. Un micro choisi puis débranché reste dans la liste sous son identifiant ; tant qu’il manque, c’est celui du système qui sert. | « Celui du système » |

Sans aucun micro, la ligne dit « Aucun micro trouvé : branche-en un, puis rouvre les réglages. » Les messages d’un micro refusé par le système sont dans le [dépannage](troubleshooting#la-dictee-ou-la-conversation-n-entend-rien).

## Transcription

Ce groupe choisit qui transcrit une dictée, et dans quelle langue. Sous son titre, l’app rappelle ce qui quitte ta machine : avec Whisper, « Whisper transcrit sur ta machine : rien de ce que tu dis n’est envoyé. Seul le modèle est téléchargé, par le proxy des réglages réseau, et sa somme de contrôle vérifiée. » ; en ligne, ce qui part et vers quel service (voir [La dictée en ligne](#la-dictee-en-ligne)).

| Réglage | Effet | Par défaut |
|---|---|---|
| « Transcrite par » | « Local (Whisper) » : la dictée est transcrite sur ta machine. « En ligne » : le son de chaque dictée est envoyé au service choisi, avec ta clé, pour une machine trop lente pour Whisper ou une meilleure qualité. La conversation vocale suit ce choix : en ligne, chaque phrase que tu dis part au même service. | « Local (Whisper) » |
| « Modèle » (local) | Le modèle de Whisper. Plus gros, il comprend mieux et transcrit plus lentement. Celui choisi se télécharge à la première dictée, ou depuis sa ligne. Il sert aussi à la conversation vocale. | base |
| « Service » (en ligne) | Le service qui transcrit : OpenAI, Groq, Mistral, ElevenLabs ou Deepgram, payé à l’usage sur ton compte chez lui. | OpenAI |
| « Modèle » (en ligne) | Le modèle du service qui transcrit, gardé pour chaque service. | Celui du tableau ci-dessous |
| « Transcription en temps réel » (en ligne, sauf Groq) | Le texte s’écrit pendant que tu parles, et la phrase est prête un instant après ton dernier mot. Gardée pour chaque service ; décochée, chaque dictée et chaque phrase partent entières, une fois finies. Voir [La transcription en temps réel](#la-transcription-en-temps-reel). | Cochée |
| « Clé de <service> » (en ligne) | Ta clé d’API chez ce service, la même pour la dictée et pour la voix de la conversation. | Aucune |
| « Langue parlée » | La langue que Whisper (ou le service) attend : « Celle de l’interface », « Détection automatique », ou une langue précise (Français, English, Deutsch, Español, Italiano, Português, Nederlands). Indiquer la langue rend la transcription plus juste ; la détection automatique convient à qui parle plusieurs langues. | « Celle de l’interface » |

### Les modèles de Whisper

Chaque modèle a sa ligne, avec sa taille et deux notes sur 5, comme « Vitesse 4/5 · qualité 2/5 » :

| Modèle | Taille | Vitesse | Qualité |
|---|---|---|---|
| `tiny` | 74 Mo | 5/5 | 1/5 |
| `base` | 141 Mo | 4/5 | 2/5 |
| `small` | 465 Mo | 3/5 | 3/5 |
| `large-v3-turbo-q5_0` | 547 Mo | 2/5 | 4/5 |
| `medium` | 1,4 Go | 1/5 | 4/5 |
| `large-v3-turbo` | 1,5 Go | 2/5 | 5/5 |

Un modèle pas encore téléchargé a un bouton « Télécharger » ; pendant le téléchargement, la ligne montre le pourcentage et « Annuler ». Un modèle téléchargé est marqué « Téléchargé », avec « Effacer » : son fichier quitte le disque, et il se télécharge à nouveau à la prochaine dictée qui le demande. Les modèles viennent de Hugging Face (`huggingface.co`), par le proxy des [réglages réseau](settings-network), et leur somme de contrôle est vérifiée avant qu’ils soient gardés.

### La dictée en ligne

« En ligne » montre trois lignes à la place des modèles de Whisper : « Service », « Modèle » et « Clé de <service> ». Les services et leur modèle par défaut :

| Service | Adresse | Modèle par défaut |
|---|---|---|
| OpenAI | `api.openai.com` | `gpt-4o-mini-transcribe` |
| Groq | `api.groq.com` | `whisper-large-v3-turbo` |
| Mistral | `api.mistral.ai` | `voxtral-mini-latest` |
| ElevenLabs | `api.elevenlabs.io` | `scribe_v2` |
| Deepgram | `api.deepgram.com` | `nova-3` |

La clé se colle dans le champ (« Colle ta clé d’API »), puis « Enregistrer la clé » ou `Entrée`. Elle part aussitôt dans le trousseau du système, une entrée par service, jamais dans les fichiers d’Escouade ni dans son journal, et n’est jamais réaffichée : la ligne dit « Enregistrée dans le trousseau », avec « Remplacer » et « Effacer la clé ». Sans clé, la ligne dit « Aucune clé enregistrée : le service en a besoin. » Sous Linux, où Escouade n’a pas de trousseau, la clé ne peut pas être enregistrée. **Une clé par service** : enregistrée une fois, ici ou dans le groupe « Conversation », elle sert à la dictée et à la [voix en ligne](#la-voix-en-ligne).

Le menu « Modèle » liste les modèles du service qui transcrivent, avec ce qui les distingue (« rapide », « plus précis »), celui par défaut en premier. Avec une clé enregistrée, la liste est celle que donne l’API du service, lue avec ta clé et gardée sur ta machine ; « Actualiser la liste » la relit, et la ligne dit quand elle a été lue. Sans clé, ou pour un service dont l’API ne liste pas ses modèles de transcription (ElevenLabs), c’est la liste de sa documentation, livrée avec l’app. Tant que tu n’en choisis pas un autre, c’est le modèle par défaut : la dictée reste celle d’avant. Le modèle choisi est gardé par service : changer de service puis revenir le retrouve. Un modèle que le service ne propose plus est remplacé par celui par défaut, et la ligne le dit (« Groq ne propose plus « … » : son modèle par défaut, « whisper-large-v3-turbo », le remplace. »).

Ces cinq services sont des plugins livrés avec Escouade (`dev.escouade.openai`, `dev.escouade.groq`, `dev.escouade.mistral`, `dev.escouade.elevenlabs`, `dev.escouade.deepgram`) : la clé reste au trousseau de l’app, qui la donne au seul plugin de son service à chaque appel. **Mistral partage son compte avec Mistral Vibe** : la clé donnée ici sert aussi aux comptes Vibe qui n’ont pas de clé à eux, et sans clé ici, la voix prend celle du premier compte Vibe actif, ce que la ligne dit (« Celle du compte Vibe « Équipe » · clé …wxyz », avec « Donner une autre clé »). Coupé dans la vue Plugins, un service quitte cette page et la dictée repasse en local ; un plugin qui ne démarre pas laisse Escouade répondre à sa place, et son journal dit pourquoi (voir [les plugins livrés avec Escouade](plugins/installer#les-plugins-livres-avec-escouade)).

### La transcription en temps réel

Sous « Modèle », la case « Transcription en temps réel » est cochée par défaut pour les services qui la proposent : OpenAI, Mistral, ElevenLabs et Deepgram (pas Groq). Le son part alors au service pendant que tu parles, par un flux chiffré, et le texte revient au fil de ta parole : il s’écrit dans la pastille de la dictée, puis le texte définitif va dans le champ, un instant après la fin de la dictée ; en conversation vocale, il s’écrit sous l’orbe, et la phrase est prête quelques centaines de millisecondes après ton dernier mot. La ligne dit le modèle pris et que, chez ce service, la minute en temps réel coûte plus cher :

| Service | Modèle en temps réel |
|---|---|
| OpenAI | `gpt-live-transcribe` |
| Mistral | `voxtral-mini-transcribe-realtime-2602` |
| ElevenLabs | `scribe_v2_realtime` |
| Deepgram | Le modèle choisi |

La case est gardée pour chaque service. Décochée, chaque dictée et chaque phrase de la conversation partent entières, une fois finies, comme avant. Si le flux ne passe pas (un proxy d’entreprise qui refuse les WebSockets, par exemple), ou tombe, la phrase part entière, par requête : rien n’est perdu, et la conversation ne réessaie plus le flux jusqu’à la fin.

Sous le titre du groupe, l’app dit ce qui part. Avec « Transcription en temps réel », le son de ce que tu dis, au fil de ta parole, vers le service choisi, avec ta clé, par le proxy des réglages réseau ; en conversation vocale, seulement pendant que tu parles : ni le silence, ni la voix de l’agent. Sinon, le son de chaque dictée (un fichier WAV), une fois que tu la finis, vers le service choisi, avec ta clé, par le proxy des réglages réseau. Rien d’autre : ni le projet, ni le code, ni la conversation avec l’agent. Le texte revient dans le champ dicté ; le service traite le son selon ses propres conditions. Le détail est sur la page [Confidentialité](/privacy/).

## Utilisation

| Réglage | Effet | Par défaut |
|---|---|---|
| « Raccourci » | Le raccourci de la dictée : un appui bref la commence et un second la finit ; maintenu, il dicte tant qu’il est enfoncé. Il dicte dans le champ qui a le focus : le message, la description d’un ticket, le terminal de la vue Claude Code. | `Ctrl+Maj+Espace` (`⇧⌘Espace` sous macOS) |

« Changer » attend le nouveau raccourci (« Appuie sur le raccourci… ») : il doit avoir `Ctrl`, `Alt` ou `Cmd`, ou être une touche `F1` à `F24` ; `Échap` garde l’actuel. Un raccourci changé a un bouton « Par défaut », qui remet `Ctrl+Maj+Espace`.

## Conversation

Ce groupe règle la voix qui lit les réponses de l’agent pendant une [conversation vocale](chat#parler-avec-l-agent-conversation-vocale) : par défaut Piper, sur ta machine, ou un service en ligne (voir [La voix en ligne](#la-voix-en-ligne)). Sous son titre, l’app dit, avec Piper : « Les réponses sont lues sur ta machine par Piper : rien ne quitte ta machine. » Les voix proposées sont celles dont la licence le permet : modèle et jeu de données libres, aucune voix de personnage, de personne connue ni de marque, sauf une exception, signalée.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Voix générée par » | « Local (Piper) » : les réponses sont lues sur ta machine. « En ligne » : par le service choisi, avec la clé de sa dictée. | « Local (Piper) » |
| « Voix en Français » | La voix qui lit les réponses quand la conversation se tient en français, parmi 7. Avec une voix « En ligne », ces lignes disparaissent : une seule, « Si ElevenLabs ne répond pas » par exemple, dit quelle voix locale téléchargée prend le relais (ou que la conversation se met en pause). | Siwis, qualité moyenne (60 Mo) |
| « Voix en English » | La même chose en anglais, parmi 29. | Ljspeech, qualité moyenne (61 Mo) |
| « Locuteur » | Pour une voix qui a plusieurs locuteurs ou styles, celui qui lit : son nom quand la voix en donne un, sinon son numéro. | Le premier |
| « Vitesse » | De 0,5× à 2×, par pas de 0,1 : 1 est le rythme de la voix, plus haut elle lit plus vite. En ligne, ramenée dans les bornes du service ; la ligne dit quand il n’en prend pas. | 1,0× |
| « Volume » | De 0 à 100 %, appliqué sur ta machine, quel que soit le service. | 100 % |
| « Délai avant l’envoi » | Le silence après lequel ce que tu as dit part à l’agent, de 0,5 à 3 s, par pas de 0,1 s. | 1,2 s |

La conversation se tient dans la « Langue parlée » quand c’est le français ou l’anglais, sinon dans la langue de l’interface.

### Les voix

Le menu de chaque langue nomme chaque voix avec sa qualité et sa taille, comme « Siwis - French (FR) Medium · qualité moyenne · 60 Mo ». Sous le menu :

- « Télécharger (60 Mo) » pour une voix pas encore téléchargée, puis le pourcentage et « Annuler » ;
- « ▶ Écouter » (« Lecture… » pendant qu’elle lit une phrase d’essai) et « Effacer » pour une voix téléchargée ;
- sa licence et celle de son jeu de données (« Licence du modèle : mit · Jeu de données : CC-BY 4.0 », ou « Licence du modèle : aucune déclarée » pour Fuze), et le crédit qu’elle demande (« Crédit : … »).

**Fuze**, en français, de haute qualité (74 Mo), est la seule voix proposée par exception, ce que sa fiche dit : son modèle ne déclare aucune licence, et son jeu de données, sous CC BY 4.0, a été généré par une voix de synthèse (ElevenLabs v4). Elle a 14 styles, choisis par « Locuteur » : normal (par défaut), joyeuse, excited, colere, triste, surprise, interrogative, storytelling, moqueur, peur, chuchote, determine, fier et degoute.

Les crédits de Piper, eSpeak NG, ONNX Runtime, whisper.cpp et de chaque voix sont dans [Réglages › À propos](settings-about), « Crédits et licences ».

### La voix en ligne

« En ligne », dans « Voix générée par », fait lire les passages par un service, avec **la clé de sa dictée** (une seule entrée du trousseau par service : enregistrée pour la dictée, elle sert déjà ici). Le groupe montre alors « Service », « Modèle de la voix », « Clé de <service> », puis « Voix en Français » et « Voix en English ». Les services, leur modèle et leur voix par défaut :

| Service | Modèle par défaut | Voix par défaut (fr / en) | Vitesse |
|---|---|---|---|
| OpenAI | `gpt-4o-mini-tts` | `marin` / `marin` | 0,25× à 4× |
| Groq | `canopylabs/orpheus-v1-english` | anglais seulement : `hannah` | Son propre rythme |
| Mistral | `voxtral-mini-tts-2603` | `fr_marie_neutral` / `en_paul_neutral` | Son propre rythme |
| ElevenLabs | `eleven_flash_v2_5` | George (`JBFqnCBsd6RMkjVDRZzb`) | 0,7× à 1,2× |
| Deepgram | `aura-2` | `aura-2-agathe-fr` / `aura-2-thalia-en` | 0,7× à 1,5×, en anglais seulement |

Le menu « Modèle de la voix » marche comme celui de la dictée : la liste de l’API du service, lue avec ta clé (par exemple les modèles d’ElevenLabs : Flash v2.5, v4 Turbo, v4, v3…), sinon celle de sa documentation, le modèle par défaut tant que tu n’en choisis pas un autre, gardé par service.

Le champ « Voix en … » dépend du service :

- quand son API liste ses voix (ElevenLabs, Mistral, Deepgram), **il se complète** : tape quelques lettres, la liste filtre par nom, identifiant, genre, accent ou description, sur les voix qui lisent la langue avec ce modèle ; les voix créées ou clonées sur ton compte viennent en premier, marquées « ton compte ». Choisis avec la souris, ou avec les flèches et `Entrée` ; `Échap` ferme la liste ;
- sinon (OpenAI, Groq), c’est un **champ libre** : écris le nom ou l’identifiant de la voix tel que le service l’attend (la ligne donne un exemple), « Voir ses voix » ouvre la page des voix du service.

Vide, c’est la voix par défaut du service. La voix choisie est gardée par service et par langue (et par modèle chez Deepgram, dont les voix sont des modèles). « ▶ Écouter » lit une phrase avec le service, le modèle et la voix choisis : une voix que le service refuse est dite tout de suite, avec sa réponse. Un modèle qui ne lit pas une langue (Orpheus de Groq en français) : la ligne le dit, et Piper lit la conversation dans cette langue.

Pendant la conversation, rien ne change pour toi : les passages sont lus phrase par phrase, la première demandée tout de suite et les suivantes pendant qu’elle joue, le son joué à mesure qu’il arrive, avec les mêmes sous-titres, le même volume, le même barge-in, la pause et l’arrêt. Sous le titre du groupe, l’app dit ce qui part : le texte de chaque passage lu, phrase par phrase, vers le service, avec ta clé, par le proxy des [réglages réseau](settings-network), pas le son du micro, qui suit « Transcrite par ». Le détail est sur la page [Confidentialité](/privacy/).

Si le service ne répond pas (clé absente ou refusée, quota épuisé, service en panne, réseau ou proxy), la conversation le dit une fois, avec le service et quoi faire, et **Piper lit la suite** si une voix locale est téléchargée ; sinon elle **se met en pause**, et « Reprendre » redemande au service. Une voix que le service ne connaît plus : c’est dit, et sa voix par défaut lit. Le journal d’Escouade ne contient jamais ta clé ni le texte lu.

### Ce qui est lu à voix haute

Pendant une conversation, chaque message dit à l’agent lui demande de finir sa réponse par un court passage écrit pour être lu : deux ou trois phrases, sans code, sans chemin de fichier ni détail technique. Escouade le montre à part sous la réponse (« Lu à voix haute ») et le lit avec la voix de la langue ; la réponse complète reste dans la conversation. Cela vaut pour tous les harness (Claude Code, Codex, OpenCode, Copilot CLI). Un agent qui ne donne pas ce passage fait dire « C’est fait. La réponse est dans la conversation. » Les questions et les demandes de permission de l’agent sont lues elles aussi.

### Quand la voix n’est pas disponible

Piper fait tourner les voix avec ONNX Runtime, livré avec l’app. S’il manque, le groupe dit « La synthèse vocale n’est pas disponible : le moteur de la voix (ONNX Runtime) est introuvable dans l’app. », les boutons des voix sont grisés, et la conversation ne démarre pas : réinstalle Escouade.

## Ce qui se télécharge, et où

Rien ne se télécharge tant que tu ne dictes pas ou ne parles pas avec un agent. À la première dictée, puis à la première conversation, Escouade propose chaque fichier qui manque, avec sa taille, et ne le télécharge qu’après « Télécharger ». Tout passe par le proxy des [réglages réseau](settings-network), depuis Hugging Face, et chaque fichier est vérifié par sa somme de contrôle.

| Quoi | Quand | Où, sous le [dossier de données](data) |
|---|---|---|
| Le modèle de Whisper choisi | À la première dictée ou conversation en local | `voice/models/ggml-<modèle>.bin` |
| La voix de la langue | À la première conversation dans cette langue | `voice/voices/<voix>.onnx` et `<voix>.onnx.json` |
| Les données d’eSpeak NG (prononciation du français et de l’anglais) | Écrites par l’app à la première lecture, sans téléchargement, et réécrites après une mise à jour | `voice/espeak-ng-data/` |
| ONNX Runtime | Jamais : il est livré avec l’app | Le dossier de l’app |

La dictée et la voix en ligne ne téléchargent rien. « Effacer », sur la ligne d’un modèle ou d’une voix, libère sa place sur le disque.
