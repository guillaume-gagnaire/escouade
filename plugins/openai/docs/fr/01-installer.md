# Installer

## 1. Avoir le plugin

Escouade l’installe de lui-même : il est dans l’installeur de l’app, avec les autres plugins de sa main, et s’installe sans réseau à son lancement. Tu le vois dans la vue Plugins, sous le nom « OpenAI ».

- **Allumé**, il sert OpenAI dans Réglages › Voix. Il démarre au premier appel qui en a besoin.
- **Coupé** (son interrupteur, dans la vue Plugins), il ne sert plus rien : une dictée ou une voix qui passent par OpenAI échouent, et le message le dit (voir l’onglet Dépannage).
- **Désinstallé**, Escouade ne le remet pas sans toi : la vue Plugins le propose sous « Proposés par Escouade », avec « Installer ». Tu peux aussi l’installer depuis le store, comme un autre plugin.

## 2. Créer une clé chez OpenAI

1. Connecte-toi sur platform.openai.com, puis ouvre « API keys » (https://platform.openai.com/api-keys).
2. Crée une clé (« Create new secret key ») et copie-la : OpenAI ne la montre qu’une fois.
3. Le compte doit avoir un moyen de paiement ou du crédit : la dictée et la voix sont facturées à l’usage.

## 3. La donner à Escouade

1. Ouvre les réglages (`Ctrl+,`, `⌘,` sous macOS), onglet « Voix ».
2. Dans le groupe « Transcription », mets « Transcrite par » sur « En ligne », puis « Service » sur « OpenAI ».
3. Sur la ligne « Clé d’OpenAI », colle ta clé dans le champ (« Colle ta clé d’API »), puis « Enregistrer la clé » ou `Entrée`. La ligne dit alors « Enregistrée dans le trousseau », avec « Remplacer » et « Effacer la clé ».
4. Clique sur « Enregistrer » pour garder le choix du service : la clé, elle, est gardée tout de suite.

Pour qu’OpenAI lise aussi les réponses de tes agents, va au groupe « Conversation » du même onglet : « Voix générée par » sur « En ligne », puis « Service de la voix » sur « OpenAI ». **Une seule clé par service** : enregistrée pour la dictée, elle sert déjà à la voix.

## Ce qu’Escouade garde, et où

- **La clé** : dans le trousseau du système (le Gestionnaire d’identification sous Windows, le Trousseau sous macOS), service `dev.gagnaire.escouade`, entrée `dictation-openai`. Jamais dans un fichier d’Escouade, ni dans son journal, et elle n’est jamais réaffichée. Sous Linux, Escouade n’a pas de trousseau : la clé ne s’enregistre pas, et l’app le dit.
- **Tes choix** (service, modèles, voix, temps réel) : dans les réglages de l’app.
- **La liste des modèles et des voix** lue chez OpenAI : dans `voice/online-catalog.json`, sous le [dossier de données](https://escouade.dev/docs/data) d’Escouade, jusqu’à ce que tu la relises.
- **Le plugin ne garde rien** : il reçoit la clé à chaque appel, et n’a ni disque ni secret à lui.
