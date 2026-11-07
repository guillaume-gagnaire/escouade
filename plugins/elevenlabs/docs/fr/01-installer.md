# Installer

## 1. Avoir le plugin

Escouade l’installe de lui-même : il est dans l’installeur de l’app, avec les autres plugins de sa main, et s’installe sans réseau à son lancement. Tu le vois dans la vue Plugins, sous le nom « ElevenLabs ».

- **Allumé**, il sert ElevenLabs dans Réglages › Voix. Il démarre au premier appel qui en a besoin.
- **Coupé** (son interrupteur, dans la vue Plugins), il ne sert plus rien : une dictée ou une voix qui passent par ElevenLabs échouent, et le message le dit (voir l’onglet Dépannage).
- **Désinstallé**, Escouade ne le remet pas sans toi : la vue Plugins le propose sous « Proposés par Escouade », avec « Installer ». Tu peux aussi l’installer depuis le store, comme un autre plugin.

## 2. Créer une clé chez ElevenLabs

1. Connecte-toi sur elevenlabs.io, puis ouvre les clés d’API de ton compte (https://elevenlabs.io/app/settings/api-keys).
2. Crée une clé et copie-la. Si tu restreins ses droits, laisse-lui la transcription (speech to text), la synthèse (text to speech) et la lecture des modèles et des voix : sans la lecture des voix, le champ de la voix reste libre.
3. La dictée et la voix sont décomptées des crédits ou du forfait de ton compte.

## 3. La donner à Escouade

1. Ouvre les réglages (`Ctrl+,`, `⌘,` sous macOS), onglet « Voix ».
2. Dans le groupe « Transcription », mets « Transcrite par » sur « En ligne », puis « Service » sur « ElevenLabs ».
3. Sur la ligne « Clé d’ElevenLabs », colle ta clé dans le champ (« Colle ta clé d’API »), puis « Enregistrer la clé » ou `Entrée`. La ligne dit alors « Enregistrée dans le trousseau », avec « Remplacer » et « Effacer la clé ».
4. Clique sur « Enregistrer » pour garder le choix du service : la clé, elle, est gardée tout de suite.

Pour qu’ElevenLabs lise aussi les réponses de tes agents, va au groupe « Conversation » du même onglet : « Voix générée par » sur « En ligne », puis « Service de la voix » sur « ElevenLabs ». **Une seule clé par service** : enregistrée pour la dictée, elle sert déjà à la voix.

## Ce qu’Escouade garde, et où

- **La clé** : dans le trousseau du système (le Gestionnaire d’identification sous Windows, le Trousseau sous macOS), service `dev.gagnaire.escouade`, entrée `dictation-elevenlabs`. Jamais dans un fichier d’Escouade, ni dans son journal, et elle n’est jamais réaffichée. Sous Linux, Escouade n’a pas de trousseau : la clé ne s’enregistre pas, et l’app le dit.
- **Tes choix** (service, modèles, voix, temps réel) : dans les réglages de l’app.
- **La liste des modèles et des voix** lue chez ElevenLabs : dans `voice/online-catalog.json`, sous le [dossier de données](https://escouade.dev/docs/data) d’Escouade, jusqu’à ce que tu la relises.
- **Le plugin ne garde rien** : il reçoit la clé à chaque appel, et n’a ni disque ni secret à lui.
