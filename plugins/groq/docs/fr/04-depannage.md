# Dépannage

Les messages que tu peux lire avec Groq, tels que l’app les écrit, ce qu’ils veulent dire et quoi faire. Ceux de la dictée s’affichent à la place du texte ; ceux de la voix, une fois, dans la conversation vocale.

## La dictée et la voix

**« Aucune clé Groq n’est enregistrée. Ajoute-la dans Réglages › Voix, ou repasse en local. »**
Le trousseau n’a pas de clé Groq. Colle-la dans Réglages › Voix (onglet Installer), ou mets « Transcrite par » sur « Local (Whisper) ». Sous Linux, la clé ne peut pas être enregistrée : reste en local.

**« Groq refuse la clé. Vérifie-la dans Réglages › Voix. »**
Groq a répondu 401 ou 403 : la clé est fausse, révoquée, ou n’a pas le droit demandé. Crée-en une autre chez Groq, puis « Remplacer ».

**« Groq refuse pour l’instant : trop de requêtes ou quota épuisé (…). Attends un peu, ou vérifie le forfait de ton compte Groq. »**
Groq a répondu 429. Entre parenthèses, ce qu’il a dit. Attends, ou regarde ton forfait et ton crédit chez Groq.

**« Groq ne répond pas : … Vérifie le réseau et le proxy (Réglages › Avancé › Réseau), ou repasse en local. »**
La demande n’a pas abouti. Ce qui suit les deux-points dit pourquoi :

- une erreur du réseau ou du proxy : vérifie [Réglages › Avancé › Réseau](https://escouade.dev/docs/settings-network) et que `api.groq.com` est joignable ;
- « délai dépassé » : Groq n’a pas répondu à temps ;
- « le plugin de ce service n’est plus là » : le plugin Groq est coupé ou désinstallé. Rallume-le dans la vue Plugins, ou installe-le (onglet Installer) ;
- « Le plugin dev.escouade.groq ne répond pas : … » : le plugin ne tourne pas ou s’est arrêté. Son onglet Journal, dans la vue Plugins, dit pourquoi ;
- « Réponse illisible du plugin dev.escouade.groq : … » : signale-le, avec son journal.

**« Groq n’a pas transcrit la dictée (<code>) : … »**
Groq a répondu autre chose qu’un texte : son code HTTP et ce qu’il a dit (un modèle inconnu, un son refusé, une panne). « réponse sans texte » : il a répondu sans transcription. Règle ce qu’il dit sur ton compte, change de modèle, ou repasse en local.

**« Groq n’a pas lu le passage (<code>) : … »**
Groq a refusé de lire une phrase : son code HTTP et ce qu’il a dit. « réponse sans son » : il a répondu sans audio. Vérifie le modèle et la voix dans le groupe « Conversation ».

### Dans la conversation vocale

Le message de Groq est suivi de ce que fait la conversation :

- « … La conversation continue avec la voix locale (Piper). » : une voix Piper est téléchargée, elle lit la suite.
- « … La conversation est en pause : corrige-le, ou télécharge une voix locale dans Réglages › Voix, puis reprends. » : aucune voix Piper n’est là. Corrige la cause, puis « Reprendre ».
- « … La voix « <voix> » n’est pas celle de Groq : sa voix par défaut, « <voix par défaut> », la remplace. » : Groq ne connaît pas la voix choisie (400, 404 ou 422). Choisis-en une autre, ou vide le champ.
- « Groq (<modèle>) ne lit pas le français : la voix locale (Piper) lit la conversation. » : ce modèle ne lit pas la langue de la conversation (Orpheus).
- « Groq ne propose plus le modèle « <modèle> » : « <défaut> », son modèle par défaut, le remplace. »

## Dans Réglages › Voix

- **« Aucune clé enregistrée : le service en a besoin. »** : colle ta clé (onglet Installer).
- **« La clé n’a pas été enregistrée : La clé est vide. »** : le champ était vide.
- **« La clé n’a pas été enregistrée : Le trousseau du système refuse la clé (…) : elle n’est pas gardée, et jamais dans un fichier. »** : le trousseau a refusé, ou il n’y en a pas (Linux : « no system keychain here »).
- **« La clé n’a pas été enregistrée : unknown service groq »** : le plugin Groq est coupé ou désinstallé. Rallume-le, puis recommence.
- **« La liste n’a pas pu être lue : … »** : la lecture des modèles chez Groq a échoué, avec le message de Groq (ci-dessus). La liste de sa documentation reste.
- **« Liste de la documentation de Groq : enregistre ta clé pour lire la sienne. »** : pas de clé encore.
- **« Groq ne propose plus « <modèle> » : son modèle par défaut, « <défaut> », le remplace. »** : choisis un autre modèle, ou garde celui par défaut.
- **« La voix n’a pas pu être lue : … »** : « ▶ Écouter » a échoué ; la suite est le message de Groq.

## Le journal

- **Celui du plugin** : vue Plugins, page « Groq », onglet Journal. Ce que Groq a répondu y est noté, la clé retirée : « voice: Groq refuses the key: … », « voice: Groq answered 429: … », « speech: Groq answered <code>: … ».
- **Celui de l’app** : `~/.escouade/app.log` (voir le [dépannage du site](https://escouade.dev/docs/troubleshooting)). Il ne contient jamais ta clé, ni le texte lu.
