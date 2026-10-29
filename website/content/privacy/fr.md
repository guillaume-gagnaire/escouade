# Confidentialité

Escouade travaille en local, sur ta machine. La seule chose qui part d’elle de son propre chef, et seulement si tu l’as acceptée, ce sont des statistiques d’usage, sans rien qui t’identifie. Cette page dit ce qui est envoyé, ce qui ne l’est jamais, où cela va et comment l’arrêter.

## En bref

- Les statistiques sont **facultatives**. Escouade te pose la question au premier démarrage (y compris après une mise à jour d’une version qui n’en avait pas) et n’envoie rien avant que tu aies répondu « Partager les statistiques ».
- Elles ne contiennent **rien qui t’identifie** : un identifiant tiré au hasard, jamais dérivé de ta machine ni de toi, et aucun nom, chemin, code, conversation ni donnée Git. Comme cet identifiant dure, le RGPD les traite comme des données personnelles pseudonymes : tu as donc des droits, plus bas.
- Elles vont à un serveur **à nous**, hébergé en France. Aucun tiers ne les reçoit.
- Tu peux les arrêter à tout moment, et l’identifiant est alors effacé.

## Ce qui est envoyé

Chaque événement porte son nom, son heure (UTC, à la milliseconde), un identifiant de session, l’identifiant de l’installation et les informations du système décrites plus bas.

### Les événements

| Événement | Quand | Informations ajoutées |
|---|---|---|
| `app_started` | À chaque démarrage | La langue de l’interface (`fr` ou `en`), le nombre de comptes Claude, le nombre de projets, si « Les agents peuvent piloter Escouade » (le serveur MCP) est activé (oui ou non), si un projet est dans WSL (oui ou non) |
| `app_active` | Une fois par jour, tant que l’app reste ouverte | — |
| `project_created` | Un projet est créé | — |
| `agent_created` | Un agent est créé | `worktree` : avec un worktree ou non (oui ou non) |
| `message_sent` | Un message est envoyé à un agent | — |
| `ticket_created` | Un ticket est créé | — |
| `ticket_started` | Un ticket est démarré | `by` : `user` ou `autopilot` |
| `ticket_approved` | Un ticket est validé | — |
| `committed` | Un commit est fait | — |
| `merged` | Une branche est mergée | — |
| `git_sync` | Une synchronisation Git | `op` : `fetch`, `pull` ou `push` |
| `branch_created` | Une branche est créée | — |
| `terminal_opened` | Un terminal est ouvert | — |
| `launch_started` | Une commande de lancement démarre | — |
| `test_run_started` | Un test démarre | — |
| `claude_code_view` | Un agent passe à la vue Claude Code | — |
| `file_saved` | Un fichier est enregistré dans l’éditeur | — |
| `code_searched` | Une recherche dans le code | — |
| `mcp_enabled` | « Les agents peuvent piloter Escouade » est activé | — |
| `integration_connected` | Un service est connecté | `service` : `jira`, `trello`, `github`, `linear` ou `clickup` |
| `tickets_imported` | Des tickets sont importés | `service` : `jira`, `trello`, `github`, `linear` ou `clickup` |
| `account_created` | Un compte Claude est ajouté | — |
| `remote_control_enabled` | Le remote control est activé, à la main ou par un réglage | `from` : `agent` (clic droit sur un agent), `project` (réglage d’un projet) ou `global` (réglage de tous les agents) |
| `dictation_used` | Une dictée a donné du texte | `target` : `composer` (le champ de message), `terminal` (la vue Claude Code) ou `ticket` (la description d’un ticket) ; `model` : le modèle Whisper (`tiny`, `base`, `small`…), ou le service de la dictée en ligne (`openai`, `groq`, `mistral`, `elevenlabs`, `deepgram`) ; `online_model` : en ligne, le modèle du service (`gpt-4o-mini-transcribe`…) |
| `voice_conversation` | Une conversation vocale commence | `lang` : la langue de la conversation (`fr`, `en`) ; `voice` : la voix qui lit (`fr_FR-siwis-medium`…, ou celle du service) ; `engine` : ce qui la génère (`piper`, ou le service en ligne) ; `model` : son modèle (`piper`, ou celui du service) ; jamais ce qui est dit ni lu |

Un événement dit qu’une fonctionnalité a servi, pas sur quoi : jamais le nom du projet, de l’agent ou de la branche, ni ce qui a été écrit.

### Le système et l’installation

Tous les événements portent aussi :

| Information | Exemple ou précision |
|---|---|
| Identifiant de l’installation | Un UUID v4 tiré au hasard quand tu acceptes, gardé dans `~/.escouade/telemetry-id`, effacé quand tu refuses |
| Identifiant de session | Un nombre aléatoire, renouvelé après 4 heures sans aucun événement |
| Version d’Escouade | Par exemple `1.8.0` |
| Version de développement | Si l’app est une version de développement (oui ou non) |
| Système et sa version | Par exemple Windows 10.0.26200, macOS 15.1 |
| Langue du système | Par exemple `fr-FR` (10 caractères au plus) |
| Moteur web et sa version | WebView2, WebKit ou WebKitGTK |

## Ce qui n’est jamais envoyé

- Les noms : le tien, ceux de tes projets, de tes agents, de tes branches, de tes comptes.
- Les adresses e-mail.
- Les chemins et les noms de fichiers.
- Ton code.
- Le contenu des conversations et des instructions données aux agents.
- Les titres et les textes de tes tickets.
- Les données Git : commits, remotes.
- Les jetons et les secrets.
- Tout ce que tu tapes.
- Ce que tu dictes : le son du micro et le texte qu’en tire Whisper restent sur ta machine. La transcription s’y fait ; seul le modèle est téléchargé, une fois, depuis Hugging Face (`huggingface.co`), par le proxy des réglages. La dictée en ligne, éteinte par défaut, est à part : voir « La dictée en ligne, si tu l’allumes », plus bas.
- Ce que tu dis à un agent en conversation vocale : la transcription suit celle de la dictée : par Whisper sur ta machine, ou, si la dictée est en ligne, par son service (voir « La dictée en ligne, si tu l’allumes », plus bas). Ce qu’il te répond à voix haute est lu par Piper sur ta machine, sauf si tu choisis une voix en ligne : voir « La voix de la conversation en ligne, si tu l’allumes », plus bas. Seules les voix de Piper sont téléchargées, une fois chacune, depuis Hugging Face, par le proxy des réglages.

## L’outil et l’endroit où vivent les données

Les statistiques sont reçues par [Aptabase](https://aptabase.com), un outil de mesure d’audience open source ([son code](https://github.com/aptabase/aptabase)). Il est **hébergé par l’auteur d’Escouade**, sur `aptabase.gagnaire.dev`, sur un serveur Scaleway dans un centre de données à Paris (France). Aucun tiers ne reçoit ces données, et rien ne passe par le cloud d’Aptabase.

Côté serveur, Aptabase ne garde pas l’adresse IP (celle que le serveur de ce site garde dans ses journaux est expliquée plus bas). Il s’en sert à la réception pour déduire le pays et, quand il le peut, la région, et, combinée à un sel qui change chaque jour, pour calculer un identifiant de visiteur quotidien.

## Comment les événements partent

- Les événements attendent en mémoire, jamais sur le disque.
- Ils sont envoyés toutes les minutes, par lots de 25, et quand tu quittes l’app (3 secondes au plus), en HTTPS vers `https://aptabase.gagnaire.dev/api/v0/events`.
- Hors connexion, jusqu’à 1 000 événements attendent en mémoire, les plus anciens étant abandonnés en premier. Si tu quittes l’app sans connexion, ils sont perdus.

## Combien de temps

Le serveur supprime les événements automatiquement au bout de 25 mois.

## Dire oui, non, ou changer d’avis

- **Au premier démarrage**, une fenêtre « Aider à améliorer Escouade ? » propose « Partager les statistiques » ou « Non merci », avec un lien « Ce qui est envoyé, en détail » vers cette page. Si tu la fermes sans répondre (avec `Échap`), elle revient au démarrage suivant.
- **Dans les réglages**, onglet « À propos », la ligne « Statistiques d’usage » a un interrupteur et un bouton « En détail » qui ouvre cette page. Il est désactivé tant que tu n’as pas accepté dans la fenêtre du premier démarrage.
- **Quand tu désactives**, l’identifiant de l’installation est oublié, son fichier `telemetry-id` est supprimé et les événements qui attendaient encore sont abandonnés. Si tu réactives, un nouvel identifiant est tiré : l’installation compte comme une nouvelle.

Voir aussi [les réglages « À propos »](settings-about).

## Les essais et les démonstrations

Une instance lancée avec un dossier de données à part (la variable `ESCOUADE_DATA_DIR`, utilisée par les tests et les démonstrations) n’envoie jamais rien. Voir [les données locales](data).

## Pourquoi, et sur quelle base

Pour mesurer l’usage d’Escouade : combien de personnes l’utilisent (les utilisateurs actifs sur 30 jours, comptés par identifiant d’installation), quelles versions sont encore utilisées (pour savoir quand une ancienne version peut ne plus être prise en charge), sur quels systèmes, et quelles fonctionnalités servent, afin de décider quoi améliorer.

La base légale est ton **consentement** (article 6.1.a du RGPD, et article 82 de la loi Informatique et Libertés pour l’identifiant gardé sur ta machine). Tu peux le retirer à tout moment, dans Réglages › À propos, sans que cela remette en cause ce qui a été fait avant.

## Des données pseudonymes

L’identifiant est tiré au hasard. Il ne dérive de rien de ta machine ni de toi, et les statistiques ne contiennent ni ton nom, ni ton adresse e-mail, ni tes projets. Mais, comme il reste le même d’un lancement à l’autre, il permet de reconnaître une installation : le RGPD traite donc ces statistiques comme des données personnelles pseudonymes. C’est pourquoi cette page donne aussi le responsable, la base légale, les destinataires et tes droits.

## Le responsable du traitement

GAGNAIRE DEVELOPPEMENT, SAS, 2 rue du Taillis, 33620 Cavignac, France, éditeur d’Escouade (voir les [mentions légales](/legal/)). Contact : [guillaume@gagnaire.dev](mailto:guillaume@gagnaire.dev). Un délégué à la protection des données n’est pas obligatoire pour ce traitement : il n’y en a pas.

## Qui reçoit les données

Seule GAGNAIRE DEVELOPPEMENT. Son sous-traitant est Scaleway SAS, qui héberge le serveur à Paris. Aucune donnée ne sort de l’Union européenne.

## Les journaux du serveur

Le serveur qui reçoit les statistiques et celui qui sert ce site sont derrière un reverse proxy (Traefik, géré par Coolify). Il écrit un journal d’accès de chaque requête qu’il reçoit, y compris les statistiques envoyées par l’app et les visites de ce site : adresse IP, date, adresse demandée, navigateur (User-Agent) et page d’origine (referer).

- **Pourquoi** : la sécurité et le bon fonctionnement du serveur. La base légale est l’intérêt légitime (article 6.1.f du RGPD).
- **Où et combien de temps** : ces journaux restent sur le serveur, ne servent à rien d’autre et ne sont pas analysés. Ils tournent chaque jour et sont gardés un an au plus : au-delà, ils sont supprimés.
- Le serveur web propre au site (nginx) ne garde aucun journal d’accès.

## Tes droits

Tu as le droit d’accéder à tes données, de les faire rectifier ou effacer, de limiter leur traitement, de t’y opposer, de les récupérer (portabilité) et de retirer ton consentement.

Pour les exercer, écris à [guillaume@gagnaire.dev](mailto:guillaume@gagnaire.dev) en indiquant l’identifiant qui est dans ton fichier `telemetry-id` (dans `~/.escouade/`), ou ouvre une [demande sur GitHub](https://github.com/guillaume-gagnaire/escouade/issues). La réponse vient dans un délai d’un mois.

Si tu estimes que tes droits ne sont pas respectés, tu peux porter plainte auprès de la CNIL : [www.cnil.fr/fr/plaintes](https://www.cnil.fr/fr/plaintes), 3 place de Fontenoy, TSA 80715, 75334 Paris Cedex 07.

## Le code

Le code qui envoie les statistiques est ouvert : [`telemetry.rs`](https://github.com/guillaume-gagnaire/escouade/blob/main/src-tauri/src/telemetry.rs). Ce qui est écrit sur cette page doit y être vrai.

## Ce qui sort d’Escouade en dehors des statistiques

Escouade n’a pas de compte ni de serveur à elle pour tes projets, tes conversations ou tes réglages : ils restent dans `~/.escouade/` ([les données locales](data)). Elle parle tout de même à quelques services, et seulement pour ce que tu lui demandes :

- **Claude Code**, qu’elle lance sur ta machine, envoie ses requêtes à Anthropic comme il le ferait dans un terminal. Escouade interroge aussi le service d’usage d’Anthropic pour afficher tes quotas ([les quotas](usage)).
- **Les mises à jour** : Escouade lit le fichier `latest.json` de la dernière release sur GitHub, puis télécharge la nouvelle version ([les mises à jour](updates)).
- **Les intégrations** que tu connectes (Jira, Trello, GitHub, Linear, ClickUp, GitLab, Azure Boards) : Escouade s’adresse à ces services avec ton jeton ([les intégrations](integrations)).
- **La dictée en ligne**, si tu l’allumes : voir juste en dessous.

## La dictée en ligne, si tu l’allumes

Par défaut, la dictée se fait sur ta machine et rien de ce que tu dis n’en sort. Dans Réglages › Voix › « Transcrite par », tu peux choisir « En ligne » et un service, avec **ta propre clé** d’API : c’est éteint tant que tu ne le choisis pas.

- **Ce qui part** : le son de chaque dictée (un fichier WAV, mono, 16 kHz), une fois que tu la finis, avec la langue parlée des réglages et ta clé. Rien d’autre : ni le projet, ni le code, ni la conversation avec l’agent, ni le texte des tickets. En conversation vocale, le son de chaque phrase que tu dis part de la même façon, une fois sa fin trouvée par le silence.
- **En temps réel** : avec « Transcription en temps réel » (cochée par défaut, sauf chez Groq), le son part au fil de ta parole, par un flux chiffré (WebSocket) vers le même service, avec ta clé et par le même proxy, et non plus une fois la dictée ou la phrase finie. En conversation vocale, seulement pendant que tu parles : un flux par phrase, ouvert quand ta voix commence, fermé un instant après ton dernier mot ; jamais le silence entre deux phrases, ni la voix de l’agent, ni ce que le micro entend pendant qu’il travaille. Pour la dictée, ce que le micro entend tant qu’elle dure, comme avant. Les modèles en temps réel : `gpt-live-transcribe` (OpenAI), `voxtral-mini-transcribe-realtime-2602` (Mistral), `scribe_v2_realtime` (ElevenLabs), le modèle choisi (Deepgram).
- **Où** : au seul service choisi, par le proxy des réglages réseau, avec le modèle choisi dans Réglages › Voix (par défaut ceux-ci) :

| Service | Adresse | Modèle de la dictée | Modèle de la voix |
| --- | --- | --- | --- |
| OpenAI | `api.openai.com` | `gpt-4o-mini-transcribe` | `gpt-4o-mini-tts` |
| Groq | `api.groq.com` | `whisper-large-v3-turbo` | `canopylabs/orpheus-v1-english` |
| Mistral | `api.mistral.ai` | `voxtral-mini-latest` | `voxtral-mini-tts-2603` |
| ElevenLabs | `api.elevenlabs.io` | `scribe_v2` | `eleven_flash_v2_5` |
| Deepgram | `api.deepgram.com` | `nova-3` | `aura-2` |

- **Ce qu’il en fait** : le service te renvoie le texte, qu’Escouade met dans le champ dicté. Il traite le son selon ses propres conditions, celles de ton compte chez lui : c’est entre lui et toi, Escouade n’en reçoit rien.
- **Ta clé** : gardée dans le trousseau du système (le Gestionnaire d’identification de Windows, le Trousseau de macOS), jamais dans les fichiers d’Escouade ni dans son journal. Sous Linux, où Escouade n’a pas de trousseau, elle n’est pas enregistrée. Une clé par service : la même sert à la dictée et à la voix de la conversation.
- **La liste des modèles et des voix** : avec ta clé enregistrée, Escouade demande au service ses modèles (et ses voix, celles de ton compte comprises, quand son API les liste) ; seule la clé part. La liste est gardée sur ta machine (`~/.escouade/voice/online-catalog.json`) et relue quand tu l’actualises.

## La voix de la conversation en ligne, si tu l’allumes

Par défaut, les réponses de la conversation vocale sont lues par Piper, sur ta machine. Dans Réglages › Voix › Conversation › « Voix générée par », tu peux choisir « En ligne », un service du tableau ci-dessus et une de ses voix, avec la clé de sa dictée : c’est éteint tant que tu ne le choisis pas.

- **Ce qui part** : le texte de chaque passage lu, phrase par phrase, avec le modèle, la voix, la langue de la conversation, la vitesse quand le service la prend, et ta clé. Pas le son du micro : ce que tu dis est transcrit comme ta dictée (ci-dessus). Ni le projet, ni le code, ni le reste de la conversation.
- **Où** : au seul service choisi, par le proxy des réglages réseau.
- **Ce qu’il en fait** : il renvoie le son, joué sur ta machine. Il traite le texte selon ses propres conditions, celles de ton compte chez lui.
- **Si le service ne répond pas** : la conversation le dit, et Piper lit la suite si une de ses voix est téléchargée ; sinon elle se met en pause. Le journal d’Escouade ne contient jamais ta clé ni le texte lu.

## Le store

[store.escouade.dev](https://store.escouade.dev), la place de marché des plugins d’Escouade, a des comptes pour les **auteurs** de plugins, et pour eux seuls : les utilisateurs de l’app n’y ont pas de compte et ne lui donnent rien.

- **La connexion** se fait avec GitHub, par la GitHub App « Escouade Store », en lecture seule, sur les seuls dépôts que l’auteur choisit. Le store reçoit de GitHub ton identifiant numérique, ton login, ton nom affiché et l’adresse de ton avatar, relus à chaque connexion. GitHub, Inc. est aux États-Unis : ces informations en viennent à ta demande, quand tu te connectes, sous le cadre de protection des données UE–États-Unis dont GitHub se prévaut ; le store ne lui envoie rien qu’il ne sache déjà.
- **Ce qui est gardé, et combien de temps** :

| Donnée | Pourquoi | Combien de temps |
|---|---|---|
| Identifiant GitHub, login, nom affiché, adresse de l’avatar | Le compte, la page de l’auteur | Jusqu’à la suppression du compte, puis 30 jours dans les sauvegardes |
| E-mail, facultatif, saisi par toi et vérifié par un lien | Les avis (rejet, revue faite, signalement) | Idem ; effacé dès que tu le retires |
| Session (l’empreinte d’un jeton aléatoire, sa date) | Rester connecté | 30 jours |
| Jeton d’accès GitHub de la session, chiffré | Lire les installations de l’App pendant la session | 8 heures au plus ; effacé et révoqué chez GitHub à la déconnexion |
| Plugins, versions, contrôles, résumés de revue | Le service | Tant que le plugin existe |
| Journal des actions (connexion, adresse, export, suppression, installations de l’App) | La sécurité, les litiges | 1 an |
| Signalements | La modération | 1 an après leur clôture |

- **Les bases légales** : l’exécution des [conditions des auteurs](https://store.escouade.dev/terms) (article 6.1.b du RGPD) pour le compte ; l’intérêt légitime (article 6.1.f) pour le journal des actions et les signalements.
- **Un seul cookie**, celui de la session (`__Host-store_session`, `HttpOnly`, `Secure`, `SameSite=Lax`), strictement nécessaire : pas de bandeau, aucune mesure d’audience. Les avatars sont dessinés par le store (tes initiales) : un visiteur ne charge rien chez GitHub.
- **Où** : sur le serveur de Scaleway à Paris, comme ce site ; les sauvegardes dans le stockage objet de Scaleway à Paris. Le reverse proxy garde son journal d’accès comme pour le site (voir [les journaux du serveur](#les-journaux-du-serveur)).
- **Tes droits** : la page de ton compte exporte tes données (un fichier JSON) et supprime ton compte ; ton nom et ton avatar se rectifient chez GitHub et sont relus à la connexion suivante. Pour le reste, et pour t’opposer au journal, écris à [guillaume@gagnaire.dev](mailto:guillaume@gagnaire.dev).

## Ce site

Le code de ce site n’inclut ni cookie ni outil de mesure d’audience. Il garde seulement ton choix de langue dans le stockage local de ton navigateur (la clé `escouade-lang`), pour te laisser sur la bonne langue à ta prochaine visite. Cette valeur reste dans ton navigateur et n’est envoyée nulle part. Comme pour les statistiques, le reverse proxy du serveur garde un journal d’accès de tes visites : voir [les journaux du serveur](#les-journaux-du-serveur).
