# Dépannage

Les messages que tu peux lire avec Mistral, tels que l’app les écrit, ce qu’ils veulent dire et quoi faire. Ceux de la dictée s’affichent à la place du texte ; ceux de la voix, une fois, dans la conversation vocale.

## La dictée et la voix

**« Aucune clé Mistral n’est enregistrée. Ajoute-la dans Réglages › Voix, ou repasse en local. »**
Le trousseau n’a pas de clé Mistral, et aucun compte Vibe actif ne prête la sienne. Colle-la dans Réglages › Voix (onglet Installer), ou mets « Transcrite par » sur « Local (Whisper) ». Sous Linux, la clé ne peut pas être enregistrée : reste en local.

**« Mistral refuse la clé. Vérifie-la dans Réglages › Voix. »**
Mistral a répondu 401 ou 403 : la clé est fausse, révoquée, ou n’a pas le droit demandé. Crée-en une autre chez Mistral, puis « Remplacer ».

**« Mistral refuse pour l’instant : trop de requêtes ou quota épuisé (…). Attends un peu, ou vérifie le forfait de ton compte Mistral. »**
Mistral a répondu 429. Entre parenthèses, ce qu’il a dit. Attends, ou regarde ton forfait et ton crédit chez Mistral.

**« Mistral ne répond pas : … Vérifie le réseau et le proxy (Réglages › Avancé › Réseau), ou repasse en local. »**
La demande n’a pas abouti. Ce qui suit les deux-points dit pourquoi :

- une erreur du réseau ou du proxy : vérifie [Réglages › Avancé › Réseau](https://escouade.dev/docs/settings-network) et que `api.mistral.ai` est joignable ;
- « délai dépassé » : Mistral n’a pas répondu à temps ;
- « le plugin de ce service n’est plus là » : le plugin Mistral est coupé ou désinstallé. Rallume-le dans la vue Plugins, ou installe-le (onglet Installer) ;
- « Le plugin dev.escouade.mistral ne répond pas : … » : le plugin ne tourne pas ou s’est arrêté. Son onglet Journal, dans la vue Plugins, dit pourquoi ;
- « Réponse illisible du plugin dev.escouade.mistral : … » : signale-le, avec son journal.

**« Mistral n’a pas transcrit la dictée (<code>) : … »**
Mistral a répondu autre chose qu’un texte : son code HTTP et ce qu’il a dit (un modèle inconnu, un son refusé, une panne). « réponse sans texte » : il a répondu sans transcription. Règle ce qu’il dit sur ton compte, change de modèle, ou repasse en local.

**« Mistral n’a pas lu le passage (<code>) : … »**
Mistral a refusé de lire une phrase : son code HTTP et ce qu’il a dit. « réponse sans son » : il a répondu sans audio. Vérifie le modèle et la voix dans le groupe « Conversation ».

### Dans la conversation vocale

Le message de Mistral est suivi de ce que fait la conversation :

- « … La conversation continue avec la voix locale (Piper). » : une voix Piper est téléchargée, elle lit la suite.
- « … La conversation est en pause : corrige-le, ou télécharge une voix locale dans Réglages › Voix, puis reprends. » : aucune voix Piper n’est là. Corrige la cause, puis « Reprendre ».
- « … La voix « <voix> » n’est pas celle de Mistral : sa voix par défaut, « <voix par défaut> », la remplace. » : Mistral ne connaît pas la voix choisie (400, 404 ou 422). Choisis-en une autre, ou vide le champ.
- « Mistral ne propose plus le modèle « <modèle> » : « <défaut> », son modèle par défaut, le remplace. »

## Dans Réglages › Voix

- **« Aucune clé enregistrée : le service en a besoin. »** : colle ta clé (onglet Installer).
- **« La clé n’a pas été enregistrée : La clé est vide. »** : le champ était vide.
- **« La clé n’a pas été enregistrée : Le trousseau du système refuse la clé (…) : elle n’est pas gardée, et jamais dans un fichier. »** : le trousseau a refusé, ou il n’y en a pas (Linux : « no system keychain here »).
- **« La clé n’a pas été enregistrée : unknown service mistral »** : le plugin Mistral est coupé ou désinstallé. Rallume-le, puis recommence.
- **« La liste n’a pas pu être lue : … »** : la lecture des modèles chez Mistral a échoué, avec le message de Mistral (ci-dessus). La liste de sa documentation reste.
- **« Liste de la documentation de Mistral : enregistre ta clé pour lire la sienne. »** : pas de clé encore.
- **« Mistral ne propose plus « <modèle> » : son modèle par défaut, « <défaut> », le remplace. »** : choisis un autre modèle, ou garde celui par défaut.
- **« La voix n’a pas pu être lue : … »** : « ▶ Écouter » a échoué ; la suite est le message de Mistral.
- **Le champ de la voix est libre, sans liste** : Mistral n’a pas donné ses voix à ta clé. Le journal du plugin dit « voice: the voices of Mistral not listed: … ».

## Le temps réel ne s’affiche pas

Le texte n’apparaît qu’à la fin, alors que « Transcription en temps réel » est cochée : le flux ne s’est pas ouvert, ou il est tombé, et la dictée est partie entière. Rien n’est affiché ; le journal de l’app (`~/.escouade/app.log`) le dit : « voice: the live transcription by Mistral failed (…): the dictation goes whole, by request ». Un proxy qui refuse les WebSockets en est la cause la plus courante.

## Mistral Vibe

**« Le plugin Mistral (dev.escouade.mistral) est coupé ou n’est pas chargé : rallume-le dans la vue Plugins pour lancer un agent de Mistral Vibe. »**
Rallume-le : l’agent garde sa session.

**« Le plugin Mistral (dev.escouade.mistral) n’est plus installé : la vue Plugins propose de l’installer, pour lancer un agent de Mistral Vibe. »**
Installe-le depuis la vue Plugins.

**« Mistral Vibe introuvable. Installe-le ou indique son chemin dans les réglages. »**
Installe-le : `uv tool install mistral-vibe`.

**« Mistral Vibe n’a pas ouvert la session de l’agent : … S’il n’a pas de clé d’API, lance vibe --setup dans un terminal. »**
Pour le compte de la machine, `vibe --setup` ; pour un autre compte, sa clé dans Réglages › Comptes et harness.

**« Le compte Mistral Vibe « <compte> » n’a pas de clé d’API : donne-la dans Réglages › Comptes et harness, « Changer la clé… ». »**
Ni le compte ni Réglages › Voix n’ont de clé. Donne-en une à l’un des deux.

**« Ce n’est pas une clé d’API de Mistral : copie-la entière depuis console.mistral.ai, API Keys. »**
La clé collée n’en a pas la forme.

**« Mistral Vibe parle ACP <version>, Escouade ACP <version> : mets Mistral Vibe à jour (uv tool upgrade mistral-vibe). »**
Mets-le à jour.

**« Limite de débit de Mistral atteinte : … »**
Le tour s’arrête sur une carte ambre ; l’agent reprend seul une minute plus tard.

**« La session Mistral Vibe n’a pas pu être reprise : nouvelle session (contexte vidé) »**
Vibe n’a pas retrouvé la session : l’agent repart d’une session neuve.

**« Mistral Vibe n’a pas de dossier de configuration où refuser les chemins des projets privés : l’agent ne démarre pas. »** et **« Les chemins des projets privés n’ont pas pu être refusés à Mistral Vibe (…) : l’agent ne démarre pas. »**
Escouade n’a pas pu écrire la clôture des projets privés dans le `VIBE_HOME` de l’agent : l’agent ne démarre pas plutôt que de passer outre. La raison est entre parenthèses.

**« « <fichier> » (<type>) ne peut pas être joint : Mistral Vibe lit les images et les fichiers texte, pas les PDF ni les autres fichiers. »**
Joins une image ou un fichier texte.

L’agent lit aussi, en anglais, « Refused by Escouade: <chemin> is in a private project, out of this agent's reach. » quand il demande un chemin d’un projet privé : c’est la clôture qui tient.

## Le journal

- **Celui du plugin** : vue Plugins, page « Mistral », onglet Journal. Ce que Mistral a répondu y est noté, la clé retirée : « voice: Mistral refuses the key: … », « voice: Mistral answered 429: … », « speech: Mistral answered <code>: … ».
- **Celui de l’app** : `~/.escouade/app.log` (voir le [dépannage du site](https://escouade.dev/docs/troubleshooting)). Il ne contient jamais ta clé, ni le texte lu.
