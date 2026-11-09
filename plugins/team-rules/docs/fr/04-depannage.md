# Dépannage de Règles de l’équipe

Chaque décision est écrite dans son journal : onglet **Journal** de sa page, filtre « Décision » (le fichier entier : `~/.escouade/plugin-logs/dev.escouade.team-rules.log`). Les lignes sont en anglais.

## Les lignes du journal

**`allowed Bash npm test (1 ms)`**
Autorisée par ses règles : l’agent a lancé la commande sans te la demander.

**``refused Bash rm -rf build (2 ms): The team's rules forbid `rm -rf`.``**
Refusée : la fin est le message que l’agent a reçu.

**`left Bash cd app && npm test to the user (1 ms)`**
Ses règles ne tranchaient pas : la demande t’a été posée. C’est le cas de toute commande qu’elles ne nomment pas, d’une ligne avec une redirection, une substitution, un antislash ou `--output`, et de tout ce qui n’est pas une commande (une modification de fichier, une page web).

**`did not decide Bash cargo build: no answer in time; the user is asked`**
Il n’a pas répondu en 500 ms (ou en 3 secondes à son premier démarrage). La demande t’a été posée. Si ça se répète, regarde son état en tête du journal.

**`did not decide …: {raison}; the user is asked`**
Il a planté pendant la décision, ou sa réponse n’en était pas une. La demande t’a été posée, et Escouade le relance comme d’habitude.

## Une commande que tu croyais refusée t’est demandée

Une règle juge le début d’une commande, mot pour mot : `git push origin main --force` ne commence pas par `git push --force`, `sudo rm -rf` pas par `rm -rf`. Le plugin n’autorise pas ces lignes : il te les laisse. Pour les refuser, il te faut tes propres règles (onglet Utiliser).

## Une commande que tu croyais autorisée t’est demandée

- Une autre commande de la même ligne n’est pas dans `allow` (`cd app && npm test`).
- La ligne a un `>`, un `<`, un `$`, un accent grave, un antislash ou `--output` : jamais autorisée.
- Le mot n’est pas le même (`npm testing` n’est pas `npm test`).
- Le plugin n’est pas actif pour ce projet, ou tu lui as retiré « Décide des autorisations de tes agents » : il n’est pas interrogé, et rien n’est écrit au journal.

## Le plugin est désactivé

Sa ligne dans **Installés** dit pourquoi :

**« Ses fichiers ont changé depuis son installation (empreinte …) : installe-le à nouveau. »**
Un fichier de `~/.escouade/plugins/dev.escouade.team-rules` a été modifié, `rules.json` le plus souvent. Réinstalle-le depuis le store. Pour d’autres règles, fais ta propre copie (onglet Utiliser).

**« Arrêté après {n} plantages »**
Son programme s’est arrêté plusieurs fois de suite : tes demandes te reviennent toutes. Lis les dernières lignes du journal, puis **Relancer**. Si ça recommence, signale-le sur sa page du store.
