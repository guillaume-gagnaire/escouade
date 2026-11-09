# Utiliser Règles de l’équipe

Quand un agent d’un projet où le plugin est actif demande à lancer une commande, Escouade lui pose la question avant de te déranger. Le plugin répond selon ses règles : autoriser, refuser, ou te laisser la main.

## Ce qui lui est soumis

- **Les commandes** seulement : l’outil `Bash` (ou `PowerShell`), quel que soit le harness de l’agent. Le plugin lit la ligne de commande telle que l’agent l’a écrite.
- Une modification de fichier, une lecture, une page web, un outil MCP : il ne les juge pas, elles te sont demandées.
- Un plan à approuver et une question de l’agent ne vont jamais aux plugins.

## Ses règles

Celles du `rules.json` publié avec lui :

```json
{
  "deny": [
    "rm -rf",
    "git reset --hard",
    { "command": "git push --force", "message": "No force push in this team: push a new commit instead." },
    { "command": "git push -f", "message": "No force push in this team: push a new commit instead." }
  ],
  "allow": ["npm test", "npm run lint", "cargo test", "cargo nextest", "git status", "git diff", "git log"]
}
```

- `deny` : les commandes refusées, par leur début. Une règle est un texte, ou un texte et le message que l’agent reçoit. Sans message, l’agent reçoit ``The team's rules forbid `rm -rf`.``
- `allow` : les commandes autorisées, par leur début.

## Ce qu’elles décident

Le plugin découpe la ligne en commandes, à chaque `&&`, `||`, `;`, `|`, `&` et retour à la ligne, puis juge chacune par son **début, mot pour mot** (les espaces en trop ne comptent pas).

1. **Refusé** si une seule commande de la ligne commence par une règle de `deny`. Le refus l’emporte sur tout.
2. **Autorisé** si chaque commande de la ligne commence par une règle de `allow`, et que la ligne n’a ni substitution (`$(…)`, des accents graves, `$VAR`), ni redirection (`>`, `<`), ni antislash, ni option `--output`.
3. **Sinon, à toi** : la demande arrive sur sa carte comme sans le plugin.

| Ligne | Décision |
| --- | --- |
| `npm test` | autorisée |
| `npm test -- --watch` | autorisée (commence par `npm test`) |
| `git status && git diff` | autorisée (les deux commandes le sont) |
| `npm testing` | à toi (`npm testing` n’est pas `npm test`) |
| `cd app && npm test` | à toi (`cd app` n’est pas autorisée) |
| `rm -rf build` | refusée |
| `npm test && rm -rf dist` | refusée (une commande refusée suffit) |
| `git push --force` | refusée, avec son message |
| `git diff > out.txt` | à toi (une redirection) |
| `git log --output=x` | à toi (`--output` écrit un fichier) |
| `git diff \| less` | à toi (`less` n’est pas autorisée) |

## Ses limites

Une règle juge le **début** d’une commande, pas son sens :

- `git push origin main --force` n’est pas refusée : elle ne commence pas par `git push --force`. Elle t’est demandée.
- `sudo rm -rf /` et `rm -fr x` ne sont pas refusées non plus : elles ne commencent pas par `rm -rf`. Elles te sont demandées.
- Ce que le plugin ne refuse pas n’est pas autorisé pour autant : tout ce que ses règles ne nomment pas te revient. Il ne remplace donc pas ta relecture, il t’en épargne une partie.

Une autorisation vaut pour cette demande seulement. Un autre plugin qui refuse l’emporte sur elle.

## Ce que tu vois, ce que voit l’agent

La carte de la demande, dans la conversation, dit qui a répondu : « Autorisé par Règles de l’équipe via Escouade », ou « Refusé par Règles de l’équipe via Escouade » avec la raison. Escouade ne te notifie que si aucun plugin n’a tranché. Tu peux toujours répondre toi-même sur la carte ; la première réponse arrivée est gardée.

L’agent apprend qui a refusé, jamais que c’est toi : « Refusé par Règles de l’équipe via Escouade : No force push in this team: push a new commit instead. ». Il peut chercher une autre voie.

## Avoir tes propres règles

Les règles sont un fichier du plugin : on ne les change pas dans l’app. Escouade fixe un plugin installé par l’empreinte de ses fichiers ; un `rules.json` modifié dans `~/.escouade/plugins/dev.escouade.team-rules` le désactive (« Ses fichiers ont changé depuis son installation »).

Pour des règles à toi :

1. Copie le dossier `plugins/team-rules` du dépôt d’Escouade.
2. Donne-lui un id à toi dans `escouade-plugin.json` (`com.mon-equipe.regles`) : `dev.escouade.*` est réservé aux plugins d’Escouade.
3. Écris ton `rules.json`, de la même forme.
4. Installe-le depuis la vue Plugins, **Depuis un dossier…** ou **Depuis un dépôt git…** (pour que l’équipe relise ses règles comme son code). Il sera « Non vérifié » : personne d’autre ne l’a relu.

Désactive alors `dev.escouade.team-rules` si tu ne veux que tes règles : les deux décident ensemble, et un refus de l’un l’emporte.
