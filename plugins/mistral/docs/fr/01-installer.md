# Installer

## 1. Avoir le plugin

Escouade l’installe de lui-même : il est dans l’installeur de l’app, avec les autres plugins de sa main, et s’installe sans réseau à son lancement. Tu le vois dans la vue Plugins, sous le nom « Mistral ».

- **Allumé**, il sert Mistral dans Réglages › Voix et les agents Mistral Vibe. Il démarre au premier appel qui en a besoin.
- **Coupé** (son interrupteur, dans la vue Plugins), il ne sert plus rien : une dictée ou une voix qui passent par Mistral échouent, et le message le dit (voir l’onglet Dépannage).
- **Désinstallé**, Escouade ne le remet pas sans toi : la vue Plugins le propose sous « Proposés par Escouade », avec « Installer ». Tu peux aussi l’installer depuis le store, comme un autre plugin.

## 2. Créer une clé chez Mistral

1. Connecte-toi sur console.mistral.ai, puis ouvre « API Keys » (https://console.mistral.ai/api-keys).
2. Crée une clé et copie-la entière.
3. Les appels sont facturés à l’organisation Mistral de cette clé.

## 3. La donner à Escouade

1. Ouvre les réglages (`Ctrl+,`, `⌘,` sous macOS), onglet « Voix ».
2. Dans le groupe « Transcription », mets « Transcrite par » sur « En ligne », puis « Service » sur « Mistral ».
3. Sur la ligne « Clé de Mistral », colle ta clé dans le champ (« Colle ta clé d’API »), puis « Enregistrer la clé » ou `Entrée`. La ligne dit alors « Enregistrée dans le trousseau », avec « Remplacer » et « Effacer la clé ».
4. Clique sur « Enregistrer » pour garder le choix du service : la clé, elle, est gardée tout de suite.

Pour que Mistral lise aussi les réponses de tes agents, va au groupe « Conversation » du même onglet : « Voix générée par » sur « En ligne », puis « Service de la voix » sur « Mistral ». **Une seule clé par service** : enregistrée pour la dictée, elle sert déjà à la voix.

### Le compte Mistral, partagé avec Mistral Vibe

La clé de Réglages › Voix est **le compte Mistral** : la voix et Mistral Vibe le partagent. Sa ligne le dit (« Le compte Mistral, partagé par la voix et Mistral Vibe… »).

- Sans clé ici, la voix prend celle du premier compte Vibe actif qui en a une, et la ligne dit lequel : « Celle du compte Vibe « Équipe » · … », avec « Donner une autre clé ».
- Un compte Vibe ajouté sans clé prend celle-ci (voir plus bas).

## 4. Brancher Mistral Vibe

1. Installe Mistral Vibe : `uv tool install mistral-vibe` (il faut [uv](https://docs.astral.sh/uv/)). Escouade trouve `vibe-acp` dans le `PATH`, ou là où `uv` le pose.
2. Donne-lui ta clé avec `vibe --setup`, dans un terminal : c’est **le compte de la machine**. Vibe la garde de son côté ; Escouade ne la lit jamais.
3. Pour un autre compte Mistral, ouvre Réglages › Comptes et harness, « Ajouter un compte… », harness Mistral Vibe, « Un nouveau compte » : sa clé d’API va dans le champ (« Sur console.mistral.ai, API Keys »). Laissé vide, le compte prend la clé Mistral de Réglages › Voix (« Vide : la clé Mistral de Réglages › Voix (…) »). Deux organisations Mistral restent deux comptes, chacune facturée à part.
4. Lance un agent avec le harness « Mistral Vibe ».

La page [Harness](https://escouade.dev/docs/harness#mistral-vibe) du site dit le reste : modes, coût, vue terminal.

## Ce qu’Escouade garde, et où

- **La clé** : dans le trousseau du système (le Gestionnaire d’identification sous Windows, le Trousseau sous macOS), service `dev.gagnaire.escouade`, entrée `plugin:dev.escouade.mistral:account:mistral`, et sous son ancien nom, `dictation-mistral`, lu en premier. Jamais dans un fichier d’Escouade, ni dans son journal, et elle n’est jamais réaffichée. Sous Linux, Escouade n’a pas de trousseau : la clé ne s’enregistre pas, et l’app le dit.
- **La clé d’un compte Vibe** : dans le même trousseau, entrée `plugin:dev.escouade.mistral:account:vibe-<id>`, et sous son ancien nom `vibe:<id>`, lu en premier.
- **Tes choix** (service, modèles, voix, temps réel) : dans les réglages de l’app.
- **La liste des modèles et des voix** lue chez Mistral : dans `voice/online-catalog.json`, sous le [dossier de données](https://escouade.dev/docs/data) d’Escouade, jusqu’à ce que tu la relises.
- **Le plugin ne garde rien** : il reçoit la clé à chaque appel, et n’a ni disque ni secret à lui.
