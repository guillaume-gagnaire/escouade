# Un premier plugin

Un plugin qui marche en dix minutes : d’abord un webhook Discord, sans une ligne de code, puis un programme en TypeScript qui réagit aux mêmes événements. Chaque étape se vérifie avant la suivante.

## Ce qu’il te faut

- Escouade, qui livre la commande `escouade-plugin` à côté de son programme (voir [tester un plugin](plugins/testing)) ;
- pour Discord, un salon et l’adresse d’un de ses webhooks (Paramètres du salon › Intégrations › Webhooks) ;
- pour la partie en TypeScript, Node 22.18 ou plus récent, pour écrire et tester le plugin. Celui qui l’installe n’en a pas besoin.

## Un webhook Discord, sans code

**1. Le manifeste.** Crée un dossier `mon-discord` et copie-y le manifeste du plugin Discord, l’un des [plugins d’exemple](plugins/examples#discord). Change son `id` pour un identifiant à toi (`io.github.<ton-login>.discord`) : `dev.escouade.*` est réservé aux plugins d’Escouade. Le cœur du plugin est là :

`plugins/discord/escouade-plugin.json` (extrait)

```json
  "permissions": {
    "events": ["ticket.column", "agent.status"],
    "network": ["discord.com"],
    "secrets": ["webhook"]
  },
  "contributes": {
    "secrets": [{ "key": "webhook", "title": { "fr": "Adresse du webhook Discord", "en": "Discord webhook URL" } }],
    "webhooks": [
      {
        "url": "{{secrets.webhook}}",
        "events": ["ticket.column"],
        "when": { "data.to": "review" },
        "body": {
          "content": "🧪 **{{ticket.key}}** to test: {{ticket.title}} ({{project.name}})",
          "allowed_mentions": { "parse": [] }
        }
      },
```

Il demande deux sortes de faits (`permissions.events`), un hôte (`permissions.network`) et un secret, l’adresse du webhook, qu’il ne verra jamais écrite dans un fichier. Le webhook part quand un ticket passe dans la colonne `review`, « À tester ».

**2. Le vérifier.**

```sh
escouade-plugin validate mon-discord
```

Une erreur est dite champ par champ, avec son chemin (« `escouade.min` manque. »).

**3. Le charger.** Dans la vue Plugins, « En développement », puis « Charger un dossier… » et ton dossier. La fenêtre dit ce qu’il pourra faire : recevoir ces événements, se connecter à `discord.com`, garder un secret. Il est relu à chaque changement de ses fichiers.

**4. Son secret.** Sur sa page, l’onglet « Réglages » : colle l’adresse du webhook. Elle va au trousseau du système.

**5. L’essayer.** Passe un ticket dans « À tester » : le message arrive dans le salon. L’onglet « Journal » de sa page note chaque appel, par son hôte et sa réponse, jamais l’adresse entière.

## Le même, en TypeScript

**1. Le gabarit.**

```sh
npm create escouade-plugin@latest mon-plugin -- --kind program
cd mon-plugin
npm install
npm test
```

Le gabarit dit quand un ticket est prêt à tester et refuse les push forcés ; son code et ses tests sont dans [le SDK](plugins/sdk#le-client). Son manifeste nomme `"run": { "js": "dist/plugin.mjs" }` : Escouade le fait tourner dans l’app, en WASM, sans Node chez celui qui l’installe.

**2. Envoyer un message.** Un programme peut faire plus qu’un webhook : choisir le salon, mettre en forme, garder un état. Le plugin Slack, par exemple, demande son secret à chaque fait et poste par `fetch`, qui passe par Escouade, vers les hôtes de `permissions.network` seulement :

`plugins/slack/src/plugin.ts` (extrait)

```ts
  return createPlugin().onEvent(['ticket.column', 'agent.status'], async (fact, p) => {
    const text = textOf(fact);
    if (!text || !fact.project) return;
    const token = await p.secret('token');
    if (!token) return p.log.warn('no Slack token kept: nothing sent (Plugins › Slack › Settings)');
    // A bot token, never anything else, goes to Slack (and never into the journal).
    if (!token.startsWith('xoxb-')) return p.log.warn('the token kept is not a Slack bot token (xoxb-…): nothing sent');
    const channel = channelOf(fact.project.name, channels);
    try {
      const res = await fetch('https://slack.com/api/chat.postMessage', {
        method: 'POST',
        headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json; charset=utf-8' },
        body: JSON.stringify({ channel, text }),
      });
```

Ajoute à ton manifeste ce que ton code demande : le fait dans `permissions.events`, l’hôte dans `permissions.network`, le secret dans `permissions.secrets` et `contributes.secrets`.

**3. Le banc.**

```sh
npm run bench
```

Le [banc de conformité](plugins/testing) joue ton programme comme Escouade : la poignée de main, les faits, les décisions, les délais, l’arrêt. Il doit passer avant toute publication.

**4. Le charger** : `npm run build` réunit ton code dans `dist/plugin.mjs`, puis charge le dossier comme le premier, depuis « En développement ». Son journal montre ses lignes en direct.

## Ensuite

- [Le manifeste](plugins/manifest), champ par champ, et [les événements](plugins/events) qu’un plugin peut recevoir.
- [Les permissions](plugins/permissions) : ce que chacune ouvre.
- [Publier sur le store](plugins/publishing), quand il est prêt.
