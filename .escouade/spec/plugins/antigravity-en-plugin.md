---
title: "Antigravity CLI en plugin"
subject: "plugins"
sources: ["docs/PLUGINS.md §4 (INT-10.8)","docs/PLUGINS.md §8.24 (ESC-348)","docs/PROTOCOL.md, Antigravity CLI, « Règles et jugement »","docs/TODO-2.2.md, INT-10.8","Ticket ESC-283","Ticket ESC-348","Ticket ESC-371","Ticket ESC-376","Ticket ESC-388","Ticket ESC-389","Essai real_agy_rules sur agy 1.5.3 (2026-11-08)","Revue de dev.escouade.antigravity-cli@2.2.0 (~/.escouade-review)","Revue de dev.escouade.antigravity-cli@2.2.1 (~/.escouade-review)","Revue de dev.escouade.antigravity-cli@2.2.3 (~/.escouade-review)","plugins/antigravity/README.en.md"]
tickets: ["ESC-283","ESC-348","ESC-352","ESC-369","ESC-371","ESC-376","ESC-388","ESC-389"]
decisions: [68,94,112,116,125,127]
compared: null
---

# Antigravity CLI en plugin

La conversation des agents d'Antigravity CLI passe par le plugin de première main `dev.escouade.antigravity-cli` (INT-10.8) : un module WASM (`run.wasm`, sa crate `escouade-plugin-agy` dans `plugins/antigravity/`, version `2.2.4`), qui tourne dans l'hôte de l'app, sans process à lui (ESC-348). Il reste au registre des plugins livrés jusqu'à l'amorce, et sa page du store pointe vers ce dossier. Pour l'utilisateur : mêmes agents, mêmes modes, mêmes comptes, même vue terminal.

Antigravity CLI n'est pas un harness ACP : `agy -p` parle en stream-json, une ligne par message.

## Ce que fait le plugin

- Il décrit le lancement (`-p=`, stream-json dans les deux sens, le modèle et l'effort, `--mode plan` en Plan seulement, la conversation, le dossier de l'agent en `USERPROFILE` et `HOME`, son sous-dossier `tmp` en `TEMP`, `TMP` et `TMPDIR`, sa mise à jour automatique coupée) et les fichiers de ce dossier (ses règles de permission, le seul serveur d'Escouade et son jeton, un fichier qui fait exister `tmp`) ; le cœur trouve `agy`, vérifie la description et le lance.
- Il relance `agy` quand il le faut, sur la même conversation : un modèle, un effort ou un mode changé, une permission accordée, une règle accordée retirée en fin de tour, un tour arrêté (`agy` n'a pas de message d'arrêt), un redémarrage du plugin. Chaque relance est vérifiée par le cœur comme le premier lancement.
- L'agent est prêt quand `agy` a dit sa conversation ; une conversation à reprendre disparue en ouvre une neuve, et la conversation le dit.
- Ce qu'`agy` écrit lui arrive et repart en éléments de la conversation : textes, raisonnements, outils sous les noms de Claude Code avec leur diff, sous-agents, tokens, fin de tour. Ce que ses événements taisent est lu dans le transcript de la conversation.
- `-p` ne demande rien : à la fin d'un tour, le plugin juge chaque refus (ci-dessous), montre les cartes de ce qui reste et des questions sautées, fait écrire les règles accordées, et le tour continue par un message.
- Une discussion de l'onglet Docs est tenue en lecture seule par `write_file(*)` et `command(*)` refusés, et aucun de ses refus ne devient une carte.
- Un mode Plan finit sur « Plan à approuver », avec le plan écrit dans la conversation.

## Ce qu'`agy` refuse en `-p` (ESC-388)

- Les réglages de l'agent portent toujours `toolPermission: "request-review"`, quoi qu'ils aient contenu : ce qu'aucune règle ne permet demande un accord, et `-p` le refuse. C'est le défaut d'`agy` 1.5.3 ; le plugin l'écrit pour ne pas dépendre de ce défaut ni de ce que l'agent a pu planter (`agent-decides` ou `always-proceed` laisseraient `agy` juger seul).
- `agy` 1.5.3 écrit sans règle ni question dans son dossier temporaire. Ce dossier est donc celui de l'agent (`<dossier de l'agent>/tmp`), jamais le `%TEMP%` de l'utilisateur, où une écriture est refusée et revient au plugin comme les autres.
- `agy` 1.5.3 dit parfois fini (`DONE`, sans erreur) un appel qu'il a refusé, sans rien exécuter. Sa fin de tour liste pourtant ses refus (`denied_actions`, par action : `command`, `write_file`…), et son transcript ne dit jamais fini un appel refusé (il ne l'écrit pas). À la fin du tour, chaque refus listé que n'explique aucune étape refusée est rattaché au dernier appel fini de la même action que le transcript ne dit pas fini : sa ligne dans la conversation passe en échec (« Refusé par Antigravity »), et il est jugé comme les autres refus, carte comprise. Un refus listé qu'aucun appel ne rattache n'a pas de carte, mais une note de la conversation dit qu'un appel de cette action a été refusé (ESC-389).

## Comment `agy` 1.5.3 juge ses règles (ESC-388, ESC-389)

L'essai réel l'a mesuré :

- une règle `command(…)` tient la commande entière, pas son début : `command(echo hi)` refuse `;`, `&&`, `|`, `&`, une redirection, `$( )`, un retour à la ligne, et `command(git add)` refuse `git add .` ;
- une règle `write_file(<chemin>)` couvre ce qui est sous ce chemin, composant par composant : elle ne couvre ni `<dossier>2` ni `<fichier>\..\.git\x`, mais couvre `<dossier>\.git`, que le dossier existe ou non, et donc le `.git` d'un sous-dossier ou d'un sous-module à toute profondeur ;
- `deny` l'emporte sur `allow`, mais seulement pour le nom qu'il dit (la casse mise à part sous Windows) : un refus de `<dossier>\.git` ne tient pas contre `<dossier>\GIT~1` (nom court) ni contre `<dossier>\.git.` (point final), que Windows mène pourtant au même dossier ;
- un `*` n'est un joker que seul (`command(*)`), et crochets et accolades ne sont pas des globs.

## Ses règles et le jugement des refus (ESC-371, ESC-376, ESC-389)

Aucune règle d'`agy` ne garde rien à elle seule : hors de « Tout accepter » sans projet privé ni discussion, le plugin ne fait écrire aucune règle `allow` à lui ; `agy` refuse, et le plugin juge le refus à la fin du tour. Ce qu'il accepte devient une règle d'`agy` pour la reprise du tour, et il n'accepte d'office que ce que cette règle tient sans ouvrir plus que l'appel jugé. Comme un refus ne tient que contre le nom qu'il dit, aucune acceptation d'office ne repose sur un refus : elle ne porte que sur ce qui n'a rien dessous. La prudence d'ESC-376 sur les commandes reste, bien que l'essai montre qu'une règle tient la commande entière, jusqu'à une décision qui la relâche.

- **Refusé sans carte** : une discussion ; un appel qui nomme un chemin d'un projet privé, par son texte ou par son chemin résolu (`..`, chemin relatif joint au dossier de l'agent), un mot de sa commande compris (`type ..\prive\secret.txt`). Dans le doute (un chemin que fait le shell), une carte.
- **Accepté sans carte**, seulement si l'appel nomme son chemin déjà absolu et normal (ni relatif, ni `.`, ni `..`, ni séparateur final) : en « Tout accepter » (seul cas où un refus revient : avec un projet privé, il n'a ni `command(*)` ni autre règle large), tout sauf une commande ; une lecture dans le dossier des fichiers joints ; en « Demander » et « Édits auto », l'écriture d'un chemin strictement sous le dossier de l'agent, hors des dossiers gardés, qui n'est ni le dossier lui-même (`C:\wt`, `C:\wt\`, `C:\wt\.`, `.`, vide) ni un nom par lequel commence un dossier gardé (`.g`), **et qui n'atteint qu'un fichier** : un fichier qui est là (pas un lien), ou un chemin absent sans sous-module de l'index de git à ce chemin ni dessous. C'est le cœur qui le dit au module (`fs/fileOnly`, ci-dessous) ; un cœur qui ne sait pas le dire met l'écriture en carte. Exactement un appel accordé « Pour cet agent », s'il atteint encore un fichier seul. La conversation dit chaque acceptation d'office et ce qu'elle vaut.
- **Aucune commande acceptée d'office** : `git add` et `git commit` en « Édits auto » et chaque commande en « Tout accepter » avec un projet privé vont à une carte, comme chez Vibe sous clôture.
- **Une carte** pour le reste : en particulier l'écriture d'un dossier, d'un sous-module (même jamais initialisé), de ce qui en tient un, ou d'un lien, dont la règle couvrirait un `.git` dessous. Les **dossiers gardés** sont `.git` (toute casse, `.git.`, flux `::$…`, le fichier `.git` d'un worktree), `.husky`, `.githooks`, `.agents` et `.gemini`, à tout niveau, et tout nom avec `~` ou `:` (nom court) : une écriture y a toujours sa carte, sans « Pour cet agent ». « Pour cet agent » n'est pas non plus offert à un agent d'un projet privé, ni pour le dossier de l'agent lui-même, ni pour une écriture qui peut atteindre plus qu'un fichier, ni pour un chemin qui n'est pas absolu et normal. La carte dit qu'« Autoriser » vaut pour le tour qui suit, qu'une écriture y couvre aussi ce qui est sous son chemin, et que « Pour cet agent » redonne cette règle à chaque retour de l'appel.
- Ce qui est accordé l'est par la règle de l'appel, pour le tour qui suit seulement, puis retirée ; `agy` est relancé sur la conversation pour la lire (il ne relit pas ses règles en cours de process). Pendant une reprise qui porte une écriture, les règles refusent aussi (`deny`) l'écriture dans chaque dossier gardé à la racine du dossier de l'agent **et juste sous chaque chemin accordé** (`<chemin>\.git`, `<chemin>\.husky`…), sauf celui où l'utilisateur a permis une écriture. Ces refus n'arrêtent que le nom exact : pour un dossier que l'utilisateur accorde sur sa carte, un nom court ou un point final y passe, ce que la carte dit en disant qu'elle couvre ce qui est dessous. Elles ne refusent pas `<chemin accordé>\..` : `agy` 1.5.3 le lit comme le dossier du chemin, ce qui refuserait l'écriture même que l'utilisateur a permise, et il refuse `<fichier>\..\.git\x` sans cela. Un texte avec `*`, `?`, une parenthèse, un crochet, une accolade ou un retour à la ligne ne devient jamais une règle : le refus tient, et la conversation le dit. Un appel refusé à nouveau pendant le tour que sa règle autorisait n'est ni accordé de nouveau ni redemandé.
- **« Pour cet agent »** est gardé par le plugin, en mémoire, tant qu'il tourne : ni règle d'`agy` permanente, ni fichier du dossier de l'agent (que l'agent peut écrire). Un redémarrage du plugin l'oublie.
- **Ce que l'agent a pu écrire dans son dossier** ne lui donne aucun droit : les règles sont réécrites à chaque lancement, fin de tour, arrêt, changement de mode et re-attache ; `agentMode` est retiré de `settings.json` et `toolPermission` y est remis à `request-review` ; `mcp_config.json` ne garde que le serveur d'Escouade.
- Une écriture refusée par `agy` coûte une reprise du tour (relance d'`agy`, puis il refait l'appel).

### `fs/fileOnly`, une méthode de l'hôte (ESC-389)

Le module ne lit pas le worktree. Pour une écriture qu'il accepterait d'office, il demande au cœur `fs/fileOnly { agent, path }` : le cœur répond un seul booléen, vrai si le chemin, absolu et strictement dans le dossier de cet agent du plugin, hors de sa clôture, est un fichier (pas un lien), ou absent sans sous-module de l'index de git à ce chemin ni dessous (`git ls-files --stage`, chemin littéral, sans `core.fsmonitor`), ses liens ne sortant pas du dossier. Un dossier, un lien, autre chose, un git qui ne répond pas dans la seconde : faux, donc une carte. Elle ne demande aucune racine de `permissions.files` ; un chemin hors du dossier d'un agent du plugin est refusé. Un fichier n'a rien dessous. Un chemin absent ne couvre rien qui existe : ce que l'agent crée dessous pendant la reprise est à lui, et un dépôt qu'il y créerait (son `.git` juste dessous est refusé, mais un `.git.` passe) n'est pas un sous-module : l'index ne change que par une commande, qui a sa carte, donc le git du cœur n'y entre pas.

L'essai réel `real_agy_rules` (ignoré, `ESCOUADE_TEST_REAL_AGY` ; `ESCOUADE_TEST_REAL_AGY_MODEL` choisit un modèle, chacun a son quota ; `ESCOUADE_TEST_REAL_AGY_CASES` en choisit des cas) lance `agy` comme le plugin et dit cas par cas ce qu'il fait de ces règles. Il juge un appel refusé par l'erreur de son étape ou par les `denied_actions` de la fin du tour, refait un cas où le modèle n'a pas appelé l'outil demandé, et ses attendus sont ceux mesurés sur `agy` 1.5.3 le 2026-11-08. Il couvre le retour à la ligne, `;`, `&&`, `|`, `&`, une redirection, `$( )`, le `;` de PowerShell, `<fichier>\..\.git\x`, l'écriture du dossier lui-même, l'ordre entre `deny` et `allow`, la casse (`.GIT`), `*`, les crochets et les accolades, le dossier temporaire de l'utilisateur et celui de l'agent, et (ESC-389) l'écriture d'un sous-dossier, présent ou non, qui couvre son `.git`, le `deny` de `<sous-dossier>\.git` qui l'emporte, et ce même refus que `GIT~1` et `.git.` passent. Il finit par le client du plugin lui-même qui pilote `agy` : un appel refusé, accepté sur sa carte, fait au tour qui suit.

## Le transcript

`agy` écrit parfois le transcript juste après l'événement qui dit l'étape finie. Rien n'attend alors dans un tour du module, et aucun autre agent n'est retenu :

- ce qu'`agy` a dit à partir de cette étape est gardé, dans l'ordre ;
- le transcript est relu sur un minuteur de l'hôte, toutes les 40 ms, 600 ms au plus par lot ;
- ce qui était gardé part ensuite vers le cœur, dans le même ordre. Si le transcript n'est toujours pas là, ces lignes passent sans lui ;
- un tour arrêté lit aussitôt ce qui est gardé.

## Ce qu'il lit

Il déclare une permission : le dossier de compte de ses agents, en lecture (`agentHome`). Le dépôt de l'agent (`agentRepo`) n'y est plus depuis la 2.2.2 : il ne servait qu'à `fs/gitProtected`, pour accepter seul `git add` et `git commit`. Du worktree, il ne reçoit que la réponse de `fs/fileOnly`, un booléen par écriture jugée. Chaque agent d'Antigravity a son propre dossier, jamais le `~/.gemini` de l'utilisateur. Il n'y écrit rien lui-même : il décrit ses fichiers, le cœur les vérifie et les écrit. Le plan est lu au chemin d'écriture que le modèle a donné, seulement s'il est dans ce dossier, par la racine `agentHome`.

Du transcript, il ne lit que les 4 derniers Mio : une étape plus ancienne n'y est plus cherchée.

## Ce qui reste au cœur

Le moteur `Agy` du registre garde ce qui n'est pas la conversation : ses capacités et le sens de ses modes, les comptes (celui de la machine connecté par `agy` dans un terminal, les autres par une clé Gemini), `agy models`, les textes rédigés (`agy -p`), la vue terminal (son `--conversation`, son transcript, ses hooks), le serveur MCP d'Escouade, le retrait de son jeton du dossier de l'agent à la fin du process, et ce qui garde les outils de l'utilisateur à leur place (sa configuration git, Rust) : un module ne connaît pas son dossier.

**La vue passe par le module** pour sa traduction (`view/translate`, ESC-352) : le cœur lit le transcript et passe ses lignes au plugin. Plugin coupé, le terminal marche et la conversation dit qu'elle ne le suit plus. Le cœur garde sa copie des mots du lancement de l'interface, qui pose les mêmes règles que le plugin (ESC-376) : aucune règle `allow` sauf en « Tout accepter » sans projet privé ni discussion, les `deny` de la clôture, `toolPermission` et `agentMode` retirés (le défaut d'`agy`, `request-review`, demande dans son interface), et « Édits auto » y est le mode par défaut d'`agy`. Ce qu'aucune règle ne permet, l'interface le demande elle-même. Son dossier temporaire est celui du plugin, `<dossier de l'agent>/tmp` en `TEMP`, `TMP` et `TMPDIR`, jamais celui de l'utilisateur (ESC-389). L'ancien `escouade-always.json` de la 2.2.0 n'est plus lu : le cœur l'efface au lancement de la vue et d'`agy -p`. Voir *Le contrat des harness en process*.

## Un arrêt se dit

Coupé pour l'app ou pas chargé, un agent d'Antigravity CLI dit au lancement « Le plugin Antigravity CLI (dev.escouade.antigravity-cli) est coupé ou n'est pas chargé : rallume-le dans la vue Plugins… » et garde sa conversation ; rallumé, il reprend. Un plugin qui plante pendant un tour est relancé comme tout plugin de harness : ses règles sont réécrites aussitôt, sans ce qui avait été accordé, et `agy` relancé avant le message suivant ; une carte montrée avant ne peut plus faire continuer le tour : sa réponse le finit. Ce que le module gardait en attendant le transcript est perdu avec lui.
