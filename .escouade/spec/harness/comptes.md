---
title: "Comptes"
subject: "Harness"
sources: ["docs/SPEC.md"]
tickets: ["ESC-30","ESC-52","ESC-88","ESC-120","ESC-124","ESC-128","ESC-141","ESC-147","ESC-148","ESC-162"]
decisions: []
compared: null
---

## Ce que c'est

Un compte est l'identité d'un harness sur laquelle tournent des agents. Pour Claude Code, c'est un dossier de configuration (`CLAUDE_CONFIG_DIR` : configuration, identifiants, historique des sessions), un exécutable optionnel, un état « Actif » et un rang. Chaque compte appartient à un harness. Escouade répartit agents et tickets entre les comptes actifs selon leur quota, et peut passer un agent sur un autre compte, ou sur un autre harness. Ce qu'est un compte pour chaque harness (dossier, clé, connexion) : la ligne « Compte » de la table de *Harness*. Le détail ci-dessous est celui de Claude Code quand il est seul à le faire.

## Comportement

### Seuls les comptes ajoutés

- Une première installation n'a aucun compte, pour aucun harness, Claude Code compris. Réglages › Comptes montre chaque harness installé avec « Aucun compte » (« Aucun agent ni ticket ne démarre sur Codex avant qu’un compte soit ajouté. ») et « Ajouter un compte Codex… ».
- Seuls les comptes ajoutés existent et comptent : choix du compte d'un agent ou d'un ticket, bascule et reprise sur un autre compte, harness de secours et passation, quotas et barre d'état, statistiques, serveur MCP déclaré, textes rédigés, lecture des modèles.
- Sans compte de son harness, rien ne démarre en silence : nouvel agent, ticket lancé, pilote auto, passation et textes rédigés sont refusés (« Aucun compte Codex : ajoute-en un dans Réglages › Comptes. »). La fenêtre le dit dans un toast avec « Ajouter un compte… » sur ce harness, le Kanban à côté de sa raison, l'écran d'accueil (« Aucun compte ») et l'étape « Compte et commit » de l'assistant « Nouveau projet » aussi ; la modale s'ouvre alors par-dessus l'assistant, qui garde ce qui est saisi.
- Le compte de la machine d'un harness (son Principal) a l'id `principal` pour Claude Code, `principal-<harness>` pour un autre. Le Principal de Claude Code s'appelle « Principal » / « Main » tant qu'il n'est pas renommé.
- Un harness qui a des comptes en garde au moins un actif (le premier sinon).
- Toutes les règles de cette page (réservations, compte préféré, choix, bascule, pause, reprise sur un autre compte) valent à l'intérieur d'un harness : un agent ne part jamais sur le compte d'un autre harness. Un projet qu'aucun compte actif d'un harness ne sert ne démarre rien sur lui : « Aucun compte Codex actif ne sert le projet « X » : … ».

### Ajouter un compte

« Ajouter un compte… » propose, pour le harness choisi :

- **le compte de la machine** (le Principal) : le dossier habituel du harness (`~/.claude` ou le `CLAUDE_CONFIG_DIR` de l'app, `~/.codex`, `~/.copilot`, la config d'OpenCode). Proposé quand le harness y est connecté (« Codex est connecté sur cette machine · <e-mail> », `@login` pour Copilot CLI, « OpenCode est installé sur cette machine » pour OpenCode, qui ne se connecte à aucun compte) et qu'il n'est pas déjà ajouté (« Déjà ajouté ») ; grisé sinon (« … n’est pas connecté sur cette machine »). Il s'ajoute tel quel, sans connexion à faire. Il est lancé sans dossier d'Escouade, sert de source aux éléments partagés d'un nouveau compte, et son nom par défaut suit la langue tant qu'il n'est pas renommé. Il se retire comme les autres, et le retrait ne déconnecte pas la machine, ce que dit la confirmation : « Retirer le compte de la machine « Principal » ? … Claude Code reste connecté sur cette machine : le retirer ne déconnecte rien. »
- **un nouveau compte** (isolé) : son dossier sous `~/.escouade`, sa connexion.

#### Nouveau compte Claude Code

- On donne son nom, puis ce qui est repris du dossier de Principal, parmi ce qui existe : `settings.json` et `CLAUDE.md` (fichiers), `skills`, `agents`, `commands`, `plugins`, `hooks`, `output-styles` (dossiers). Chacun : « Lier (un changement vaut pour les deux comptes) » (défaut) ou « Copier » (chacun a ensuite le sien).
- Le compte reçoit le dossier `~/.escouade/claude/<nom>`, jamais un dossier déjà présent : celui d'un compte supprimé garde sa connexion et ses sessions.
- Les dossiers sont liés par une jonction sous Windows (`mklink /J`, sans droits d'administrateur ; un chemin avec `%` n'est pas lié, la fenêtre propose « Copier ») et par un lien symbolique sous macOS. Les fichiers sont liés par un lien symbolique sous macOS, copiés sous Windows (un lien de fichier y demande des droits d'administrateur, ce que la fenêtre dit).
- **Jamais partagés** : la connexion (`.credentials.json` ou entrée du trousseau), `.claude.json` (où vit aussi l'entrée du serveur MCP d'Escouade, propre à chaque compte), et tout le reste du dossier, dont l'historique des conversations (`projects/`). Une session vit dans le dossier de son compte, et deux process ne reprennent jamais la même session en même temps.

#### Connexion

- « Créer et se connecter » et « Se connecter… » lancent le `claude` du compte, directement (sans shell), dans un terminal de la fenêtre, depuis le dossier personnel, avec le `CLAUDE_CONFIG_DIR` du compte. Échap reste au terminal.
- Escouade lit la connexion toutes les 2 s : « Connecté au compte <e-mail> » (l'e-mail est `oauthAccount.emailAddress` du `.claude.json` du compte). 1,5 s plus tard, le terminal est fermé, le process avec, et le quota lu.
- Sur un compte déjà connecté, ou dont la connexion a expiré, elle attend une vraie nouvelle connexion : la lecture d'avant est refaite une fois quand la première échoue, pour qu'une connexion expirée ne passe pas pour celle qu'on vient de faire.
- Fermée avant : le compte reste « Pas connecté ». « Claude Code s'est arrêté avant la connexion. » → « Relancer ».
- Sous macOS, l'état d'un compte dont le trousseau est refusé reste « Pas connecté » pendant 10 minutes. Le compte reste utilisable : Claude Code lit son trousseau lui-même.

### Principal et identité

- **Principal sans `CLAUDE_CONFIG_DIR`.** Il se lance sans la variable, même vers `~/.claude`, même si un dossier a été écrit à la main dans les réglages (il est vidé à chaque chargement et enregistrement). Sous macOS, Claude Code range sa connexion dans le trousseau sous le service `Claude Code-credentials` quand la variable n'est pas définie, et `Claude Code-credentials-<h>` sinon, `<h>` étant les 8 premiers caractères hexadécimaux du SHA-256 de sa valeur normalisée en NFC (`CLAUDE_SECURESTORAGE_CONFIG_DIR`, s'il est défini, prend le pas) : la définir changerait le nom de l'entrée et déconnecterait l'utilisateur. Chaque dossier de config a donc sa connexion, et les comptes multiples marchent sous macOS. Un autre compte reçoit exactement la chaîne enregistrée, dont Escouade calcule le hachage.
- Le dossier de Principal est le `CLAUDE_CONFIG_DIR` de l'environnement de l'app s'il en a un (ses process l'héritent), sinon `~/.claude`. Son `.claude.json` est `~/.claude.json` (à côté de ce dossier), sauf si la variable est définie (`<dossier>/.claude.json`). Sous Windows, Claude Code écrit `<dossier>/.credentials.json`.
- **Identité héritée.** Un `CLAUDE_CODE_OAUTH_TOKEN`, un `ANTHROPIC_API_KEY` ou un `ANTHROPIC_AUTH_TOKEN` de l'environnement de l'app passe devant la connexion du dossier, et un `CLAUDE_SECURESTORAGE_CONFIG_DIR` fait partager une seule connexion : tous les comptes tourneraient sur la même identité. Un compte avec un dossier à lui reçoit donc ces trois variables vides quand l'app les a, et `CLAUDE_SECURESTORAGE_CONFIG_DIR` égal à son dossier. Principal, qui se connecte comme l'utilisateur de la machine, les garde.
- Le dossier d'un compte est celui que tient le cœur d'Escouade, jamais celui que la fenêtre envoie.

### Un agent garde son compte

- Il se lance avec le `claude` et le dossier de son compte, y reprend sa session (`--resume`) après un arrêt ou un redémarrage. Sa copie reste sur le même compte (la session copiée est rangée dans ce dossier).
- Le compte d'un agent ne change jamais sous son process : avant le premier message, on peut le changer (le process préchauffé repart avec) ; après, seulement en arrêtant le process et en copiant la session (« Reprendre sur <compte> », « Changer de compte à »).
- Le nom de l'agent et son message de commit passent par le compte de l'agent ; « ✦ Remplir automatiquement » par celui où partirait un nouvel agent du projet.
- Un agent dont le compte a disparu (compte supprimé, id inconnu, état d'une version plus récente) prend, au démarrage de son process, le compte où partirait un nouvel agent de son projet sur son harness. Sa session n'y est pas : il repart d'une nouvelle (« Session Claude introuvable… »). Sans compte de son harness, il est refusé et garde son compte. Un agent enregistré sans compte, d'avant les comptes, est sur Principal.

### Modèles d'un compte

- Les modèles d'un compte Claude Code sont ceux que son `claude` annonce à `initialize` (nom, efforts, mode Auto de chacun). Ils sont lus sans tour ni appel à l'API (`initialize` seul, `--no-session-persistence --strict-mcp-config`, puis le process est arrêté) au démarrage de l'app, à la création d'un compte et à la fin de sa connexion, puis à chaque démarrage d'agent. Ils sont gardés par compte.
- Le champ de saisie d'un agent montre la liste de son compte ; le Kanban, les tickets, les réglages et `create_agent` (MCP) celles des comptes actifs, chaque modèle une fois.
- L'effort et le mode Auto d'un modèle suivent ce qu'en dit son compte.
- Un agent dont le compte n'a pas son modèle (reprise sur un autre compte, changement de compte avant le premier message, passation) prend au démarrage suivant le modèle par défaut de ce compte s'il l'a, sinon celui que son Claude Code fait tourner (`default`), sinon le premier. Sa conversation le dit : « Le compte <compte> n’a pas le modèle <modèle> : l’agent passe sur <nom>, son modèle par défaut. »
- Le coût reste celui que donne Claude Code ; les statistiques par modèle prennent un modèle nouveau par son id.

### Modèle, effort et mode des nouveaux agents

- Ce avec quoi partent les nouveaux agents est un réglage de chaque compte, dans sa fiche de Réglages › Comptes (« Nouveaux agents » : « Modèle », « Effort », « Mode »), adapté à son harness : les modèles de ce compte, les efforts que son modèle prend, les modes de son harness. « Celui du harness » (vide) prend le défaut du harness : Sonnet pour Claude Code, le modèle par défaut des autres, effort `medium`, mode `auto`.
- Un nouvel agent prend ceux de son compte, sauf choix explicite. Un ticket garde son ordre : le ticket, puis le Kanban, puis le compte qui le prend. `get_board` (MCP) donne ceux du compte où partirait un nouvel agent du projet.
- Un modèle par défaut que le compte n'a plus retombe sur celui que son harness donne par défaut.
- Des réglages enregistrés avec un modèle, un effort et un mode globaux les passent à chaque compte Claude Code gardé ; un compte d'un autre harness prend les défauts de son harness.

### Quota

- Le quota d'un compte est fait de compteurs : des fenêtres libellées par leur durée (« 5h », « 7j », « 30j »), les crédits du mois, ou rien, selon le harness (voir *Harness*, ligne « Quota »). Un harness sans quota ne met jamais le pilote auto en pause. Voir docs/ENGINES.md, « Les comptes et le quota ».
- **Lecture (Claude Code).** Pour chaque compte, dans l'ordre des réglages : un process vivant du compte (même au repos) est interrogé ; sinon, pour un compte actif, l'endpoint de `/usage` avec la connexion de ce compte (sous macOS le trousseau avant le fichier `.credentials.json`, comme Claude Code ; ailleurs le fichier du dossier du compte), au plus une fois toutes les 5 minutes par compte. Un compte « Pas connecté » est relu dès la lecture suivante ; une connexion expirée reste espacée de 5 minutes. Les `rate_limit_event` du flux d'un agent mettent à jour son compte.
- États : « Pas connecté », « Connexion expirée : relance Claude Code pour ce compte. ». Une coupure du réseau garde les dernières valeurs.
- Les fenêtres sont gardées d'un lancement à l'autre (`state.json`), par compte ; celles enregistrées sans compte sont celles de Principal.
- La barre d'état montre le compte en cours du harness de l'agent affiché (sinon du harness par défaut du projet), sans barre pour un harness sans quota. Le compte en cours (barre d'état, `get_usage`) est celui où partirait un nouvel agent en « Automatique ».
- Le coût du jour est donné par compte (le panneau des quotas) et en total (la barre d'état).
- **Usage supplémentaire.** À 100 % de la fenêtre de 5 h, un compte avec l'usage supplémentaire continue de répondre, sans que le suivant serve. Escouade lit `isUsingOverage` du `rate_limit_event` : la barre d’état dit alors « <compte> : usage supplémentaire en cours, facturé à part. Les agents déjà lancés y restent, les nouveaux partent sur un autre compte. » (avec « Changer de compte à » sous 100 % : « Ses agents passent sur un autre compte à la fin de leur tour, et les nouveaux y partent. »).

#### Un compte « bon », fenêtre par fenêtre

Le choix d'un compte (nouvel agent, ticket, compte en cours, reprise, passation) et le départ d'un agent lancé jugent un seul et même compte « bon », compteur par compteur, selon la longueur de sa fenêtre. La règle est la même pour tous les harness à quota : elle va par la longueur de la fenêtre, pas par le nom du harness.

- **Fenêtre courte** (24 h ou moins : la 5 h de Claude Code et de Codex) : une pause courte. Dès qu'elle atteint le seuil (le plus bas de « Pause au-delà du quota » et « Changer de compte à »), le compte est écarté jusqu'à la remise à zéro de cette fenêtre (avec la même marge de 30 s qu'après toute remise à zéro), puis il redevient un candidat comme les autres, sans laisser de trace : une 5 h pleine ne pèse jamais sur la répartition de la semaine.
- **Fenêtre longue** (plus de 24 h : la semaine, les 30 jours de l'offre gratuite de Codex ; et les crédits du mois de Copilot CLI, pris comme une fenêtre d'un mois qui finit à leur remise à zéro) : jugée au rythme, avec la projection à la remise à zéro de la barre d'état (voir *Barre de statut*, « Projection »).
  - À 100 % (la limite réelle) : écartée jusqu'à sa remise à zéro.
  - Projection à 100 % ou moins : bonne, même au-dessus du seuil en % (une semaine à 88 % qui se remet à zéro dans 3 h ne se gaspille pas).
  - Projection au-delà de 100 % : le compte passe derrière les autres, même sous le seuil (au rythme actuel, il ne tiendrait pas jusqu'à sa remise à zéro).
  - À 95 % ou plus (`pace::NEAR_LIMIT`), projection bonne ou absente : le compte passe aussi derrière les autres, pour que les nouveaux agents ne partent pas au bord de la limite tant qu'un autre compte peut les prendre ; un agent lancé le quitte au seuil (décidé à la revue de la PR #20).
  - Pas de projection (première journée de la fenêtre, remise à zéro inconnue) : le seuil en % décide, comme avant.
- Un compte est **bon** quand aucun de ses compteurs ne l'écarte ni ne le met derrière, **derrière** quand seul le rythme d'une fenêtre longue est mauvais (aucune fenêtre n'est à sa limite, ni au seuil sans projection), **écarté** sinon. Un compte sans compteur (harness sans quota comme OpenCode, compte pas encore lu) est bon : l'ordre seul décide.
- Exemples : A (1er de la liste) à 88 % de sa semaine qui se remet à zéro dans 3 h, 5 h à 30 %, B à 10 % : A. A à 60 % au deuxième jour de sa semaine (210 % à la remise à zéro), B à 20 % (70 %) : B. A à 96 % de sa 5 h (seuil de 95 %), semaine à 20 % : B, puis A de nouveau dès la remise à zéro de sa 5 h.
- Le cœur Rust et la fenêtre calculent la projection avec les mêmes chiffres (même table de cas dans `tests/fixtures/quota-pace.json`).

### Choix du compte

- Pour un nouvel agent ou un ticket :
  1. le compte demandé, s'il est actif (puce « Compte » du champ de saisie ; une copie garde celui de son original, même désactivé depuis) ;
  2. sinon le « Compte préféré » du projet, s'il est actif ;
  3. sinon le premier compte actif, dans l'ordre des réglages, qui est **bon** au plus bas des deux seuils « Pause au-delà du quota » (Kanban) et « Changer de compte à » (voir *Un compte « bon »*), qui n'est pas sous la pause qui suit une limite d'usage, ni retenu par un agent qui attend sa remise à zéro ;
  4. aucun n'est bon : le premier dont seul le rythme est mauvais (aucune de ses fenêtres n'est à sa limite) ; sinon le premier actif, de préférence un compte que « Reprendre maintenant » a libéré.
- **L'ordre des réglages reste la priorité.** Jamais de classement au moins utilisé : demandé, puis préféré du projet, puis le premier compte bon dans l'ordre ; le choix ne remplace que ce qu'est un compte « bon ». Les réservations, le compte d'un ticket, les projets privés confinés et Principal se comportent comme avant.
- Avec « Changer de compte à : À la limite » aussi, le choix des nouveaux agents et des tickets suit cette règle : c'est toujours le compte le plus pertinent. Ce réglage ne déclenche que le départ d'un agent déjà lancé.
- La présence de Claude Code se vérifie sur le compte où partirait l'agent.
- La puce « Compte : Pro » (avec plusieurs comptes seulement, juste après le modèle) propose « Automatique (Pro) » (le compte préféré du projet, sinon le compte en cours), puis les comptes actifs (« Équipe (inactif) » pour un compte désactivé depuis). Elle est modifiable tant que l'agent n'a ni session, ni message, ni copie d'origine, ni tour en cours : son process préchauffé repart alors avec le nouveau compte. Ensuite elle est en lecture seule : « Le compte d’un agent ne change plus après son premier message (sa conversation est rangée dans ce compte). »
- Un toast le dit une fois, quand le compte en cours quitte un compte pour un compte moins écarté. Si le compte quitté est écarté par un compteur au seuil ou à sa limite : « Le compte Pro a atteint 95 % : les nouveaux agents partent sur Principal. » (le pourcentage est celui du compteur qui l'écarte, le plus utilisé). Si seul son rythme le met derrière : « Le compte Pro finirait à 140 % de sa fenêtre hebdomadaire au rythme actuel : les nouveaux agents partent sur Principal. » (« de ses crédits du mois », « de sa fenêtre de 5 h »…). Rien quand il redevient le premier bon, quand un compte est désactivé ou supprimé, ni quand tous les comptes sont écartés : c'est alors la bannière de pause qui parle.
- Le panneau des quotas marque un compte « au-delà du seuil de pause ou de changement de compte » quand le choix l'écarte, « rythme trop rapide pour sa fenêtre » quand seul son rythme le met derrière, et « près de la limite de sa fenêtre » quand il est derrière pour ses 95 % (voir *Barre de statut*).

### Pause du pilote auto, compte par compte

- Elle ne tombe que quand aucun compte n'est utilisable pour le projet (tous les comptes actifs en « Automatique », le compte préféré seul sinon), et dure jusqu'à la plus proche remise à zéro.
- Un compte est retenu par un compteur au-delà du seuil, par la pause de 30 min qui suit une limite d'usage sans reprise prévue, ou par un agent à lui qui attend sa remise à zéro. Cet agent ne retient que les tickets du compte : « Quota atteint — reprise à HH:MM » à la place des places libres, seulement quand tous les comptes du projet attendent.
- Avec plusieurs comptes, la bannière ajoute « Les comptes Principal et Pro ont passé le seuil. » (rien après une limite d'usage). « Reprendre maintenant » lève tous les comptes.
- **La pause ne suit pas le rythme.** « Pause au-delà du quota » est le plafond de l'utilisateur et se juge en pourcentage, fenêtre par fenêtre, comme avant : « Pause au-delà du quota », la pause du pilote auto, « Reprendre maintenant » et les agents qui attendent leur remise à zéro ne changent pas. Avec un plafond sous 100 %, un compte peut donc être **bon** pour le choix (une semaine à 95 % qui se remet à zéro dans 3 h, sur un plafond de 90 %) et pourtant retenu par le pilote auto : un compte retenu n'est pas choisi, le choix passe au suivant, et le pilote auto ne se met en pause que si tous le sont. Ce n'est pas corrigé : le plafond est celui de l'utilisateur.

### Changer de compte au seuil

« Changer de compte à » passe un agent sur un autre compte de son harness avant qu'il n'atteigne la limite.

- **Le seuil** : 80, 90 ou 95 %, ou « À la limite » (100, le défaut ; une autre valeur écrite à la main dans les réglages vaut 100). Un seul réglage pour tous les harness, sans second seuil. Il dit à partir de quand un agent lancé quitte son compte ; il dit aussi à partir de quand une fenêtre courte, ou une fenêtre longue sans projection, écarte un compte du choix. Un harness sans quota (OpenCode) n'est jamais concerné.
- Les nouveaux agents et les tickets de tous les harness suivent la règle du choix (*Un compte « bon »*), « À la limite » compris : le compte en cours d'un harness (barre d'état, nouveaux agents, tickets) est le premier bon au plus bas des deux seuils, et le toast « Le compte Pro a atteint 95 % : les nouveaux agents partent sur Principal. » (ou celui du rythme) suit ce choix.
- Un agent déjà lancé ne change de compte que sur un harness dont la session passe d'un compte à l'autre (ceux qui ont « Reprendre sur <compte> » : voir la ligne « Pas pris en charge » de la table de *Harness*) ; les autres restent sur leur compte.
- **Quand il part.** À la fin de son tour, un agent quitte son compte quand une **fenêtre courte** atteint le seuil, ou quand une **fenêtre longue** atteint le seuil **et** que son rythme n'est pas bon (projection au-delà de 100 %, ou pas de projection). Un agent sur une semaine à 88 % qui se remet à zéro dans 3 h ne bouge donc pas. Sur « À la limite », rien ne bouge avant 100 % (la limite est celle de « Reprendre sur… »).
- **Quand il est examiné.** À la fin de chaque tour (avant que le Kanban n'envoie le message suivant ; pas pour un tour arrêté par la limite), à chaque lecture du quota de son compte par un de ses process, et à l'envoi d'un message à un agent au repos (avant de lancer son process). L'agent passe seulement s'il est libre : pas archivé, pas en vue terminal (son process est son interface), aucun tour en cours, aucune question ni permission en attente, rien en arrière-plan dans son process (commande, surveillance, sous-agent, workflow, réveil de `/loop`), aucune reprise après une limite planifiée, une session à reprendre. Sinon il passe à la première fin de tour libre. Un seul examen à la fois par agent.
- **Où.** Le compte que donnerait le choix du compte (« Compte préféré » du projet compris : un projet qui préfère le compte de l'agent l'y garde), autre que le sien, du même harness, servant le projet (réservations), connecté, sans agent qui attend sa remise à zéro, sans pause après une limite, que l'agent ne quitterait pas aussitôt (même test que ci-dessus : ni fenêtre courte au seuil, ni fenêtre longue au seuil au rythme mauvais), du même côté de WSL. Un compte bon passe avant un compte dont seul le rythme est mauvais, sous le seuil. Un projet privé confiné ne va que sur un compte réservé à lui seul. Puis, comme « Reprendre sur » : process arrêté et attendu, session copiée dans le dossier de l'autre compte, compte de l'agent changé, process relancé tout de suite sur l'autre compte (`--resume`), sans « continue » (le tour était fini). La puce « Compte » et la carte montrent le nouveau compte une fois le process arrêté et la session copiée, jamais avant.
- **Le dire.** Une note dans la conversation (« L’agent passe sur le compte Pro : Principal est à 96 % de sa fenêtre de 5 h. » ; « de sa fenêtre hebdomadaire », « de ses crédits du mois » : le compteur qui le fait partir, le plus utilisé), un toast (« <agent> passe sur Pro : Principal à 96 % de sa fenêtre de 5 h. ») et une notification de l'agent (type « Tâches terminées »). Quand c'est une fenêtre longue au rythme mauvais, ils disent aussi où elle finirait : « … Principal est à 96 % de sa fenêtre hebdomadaire (au rythme actuel, 168 % à sa remise à zéro). »
- **Aucun compte utilisable.** L'agent reste, et sa conversation le dit une fois tant qu'il reste sur ce compte : « Principal est à 96 % de sa fenêtre de 5 h, mais aucun autre compte Claude Code ne peut prendre l’agent (sous le seuil ou à bon rythme, connecté, servant ce projet) : il y reste. » (avec le rythme entre parenthèses après le pourcentage quand c'est lui) ; pour un projet privé confiné : « … mais ce projet privé ne passe pas tout seul sur un compte qu’il partage : l’agent y reste. » Suivi de « Ce compte est en usage supplémentaire, facturé à part. » quand il l'est.
- **Pas de va-et-vient.** Le compte d'arrivée est un compte que l'agent ne quitterait pas aussitôt ; l'agent ne le quitte que quand il le passe à son tour, et le premier ne redevient un choix qu'une fois sa fenêtre remise à zéro (une fenêtre dont la fin est passée ne retient plus rien).

### Reprendre sur un autre compte

Pour un tour **arrêté** par la limite d'usage (un tour qui passe de 94 à 100 % d'un coup, ce que le seuil ne peut empêcher).

- **À la main.** La carte de fin du dernier tour (agent ni au travail ni archivé) propose « Reprendre sur <compte> » pour le premier autre compte actif, connecté (un compte pas encore lu compte comme connecté) et que l'agent ne quitterait pas aussitôt (le même test que « Changer de compte au seuil » : un compte écarté par une fenêtre pleine ou au seuil n'est pas proposé, l'agent y retrouverait la limite), et un menu « Reprendre sur un autre compte » quand il y en a plusieurs. « Reprise automatique à … / Annuler la reprise » reste à côté.
- **Le déplacement.** Le cœur vérifie seulement que le compte est connu, actif et autre que celui de l'agent. Sous le verrou de lancement de l'agent, son process est détaché, son entrée fermée et sa sortie attendue (10 s, puis il est tué et attendu 5 s de plus ; un échec est dit, rien n'est copié). **Ensuite seulement**, la session (`projects/*/<id>.jsonl` du dossier du compte et le dossier voisin `<id>/` : sous-agents, résultats d'outils) est copiée dans le dossier de l'autre compte, sous le même nom de dossier de projet : le plus récent gagne fichier par fichier (un fichier de destination au moins aussi récent est gardé), la copie reprend la date de la source et passe par un fichier `.part`. `file-history/`, `todos/` et `session-env/` ne sont pas copiés ; un identifiant de session qui n'est pas `[A-Za-z0-9_-]+` est refusé. Puis l'agent change de compte, sa reprise planifiée est effacée, une note le dit (« L’agent reprend sur le compte Équipe (il était sur Pro). ») et il reçoit « continue », comme à la reprise automatique. Jamais deux process sur la même session ; un message envoyé pendant le déplacement attend sa fin.
- **Échec.** Si le tour qui suit échoue (connexion absente, erreur de l'API), ou si Claude Code s'arrête avant de démarrer (session introuvable, connexion refusée), la carte dit « La reprise sur Pro a échoué : <erreur>. » et propose « Revenir sur Principal » (si ce compte existe et est actif). Le déplacement inverse recopie la session avec ce que le tour raté y a ajouté, puis remet l'attente de la remise à zéro du compte quitté, même quand son quota n'est plus lisible (sauf reprise automatique coupée depuis ; pour un agent de ticket sans reprise possible, le ticket est bloqué comme après toute limite sans reprise). La session de l'agent n'est jamais vidée par cet échec. Le compte d'avant est oublié à la fin du premier tour qui n'est pas une erreur.
- **Automatique**, pour un agent de ticket « En cours » arrêté par la limite (jamais un agent ordinaire, jamais un ticket bloqué), avec « Reprendre sur un autre compte un agent de ticket arrêté par la limite » : l'agent part sur le compte que choisit la règle du choix du compte (compte préféré du projet compris), s'il est autre que le sien, connecté, sans agent qui attend sa remise à zéro, sans pause, et que l'agent ne quitterait pas aussitôt sous « Pause au-delà du quota » ; sinon, ou si le projet préfère le compte de l'agent, il attend la remise à zéro. Le compte quitté est retenu jusqu'à la remise à zéro que sa reprise attendait, sinon 30 min : il ne reçoit ni nouveau ticket ni le retour de cet agent à la limite suivante, même si ses compteurs sont inconnus (sans quoi deux comptes limités se renverraient l'agent). Un déplacement qui échoue laisse l'agent attendre : « Reprise sur un autre compte impossible : … ».

### Connexion qui ne se rafraîchit pas

Voir aussi docs/PROTOCOL.md.

- Un tour fini sur « OAuth session expired and could not be refreshed » ou « Failed to refresh OAuth token: another Claude Code process is refreshing it » n'a ni notice d'erreur ni notification. Escouade relit la connexion du compte.
- Si le jeton est en date, ou si la connexion n'est plus celle avec laquelle le process a démarré, Escouade arrête le process et le relance sur sa session avec « continue », au plus deux fois par tour arrêté (après 3 s puis 10 s). La conversation dit « Session du compte rafraîchie : l’agent reprend. »
- Sinon (connexion expirée et pas rafraîchie, absente ou vidée par Claude Code, ou deux relances sans succès), l'agent attend une nouvelle connexion de son compte, attente gardée d'un lancement à l'autre : carte « Connexion à refaire », notification « Le compte <compte> doit se reconnecter : l’agent reprendra seul après. ». Toutes les 20 s, une connexion autre que celle de l'arrêt le fait repartir (« Compte <compte> reconnecté : l’agent reprend. », un process neuf, « continue »). Un message de l'utilisateur prend la main. Un agent de ticket fait pareil, son ticket n'est pas bloqué.
- Les lancements de Claude Code d'un compte au jeton expiré (agents, `claude -p`, lecture des modèles) passent un par un : le premier part et rafraîchit, les suivants attendent que la connexion change, 20 s au plus, puis partent quand même.

### Continuer sur un autre harness

Une session ne passe pas d'un harness à l'autre : c'est une passation, qui démarre une nouvelle session sur l'autre harness avec un dossier de ce qui a été fait.

- **Harness de secours** d'un projet : son harness par défaut, puis les autres dans « Ordre des harness », seulement ceux qui ont un compte actif servant le projet. Sont proposés ceux qui sont pris en charge et trouvés, sauf le harness de l'agent et celui qu'il a quitté tant que son quota n'est pas revenu.
- Le harness d'un agent est **épuisé** pour son projet quand aucun de ses comptes servant le projet ne peut le prendre (connecté, sans agent qui attend sa remise à zéro, sans pause après une limite, pas écarté par son quota sous « Pause au-delà du quota » : le même jugement « bon » que le choix du compte, un compte dont seul le rythme est mauvais pouvant le prendre), le sien compté épuisé.
- **Dans la fenêtre.** La carte de fin du dernier tour arrêté par la limite (agent ni au travail ni archivé), quand aucun compte de son harness ne permet « Reprendre sur <compte> », propose « Continuer sur <harness> » pour le premier harness de secours, et un menu ▾ (« Continuer sur un autre harness ») pour les suivants. Le menu de l'agent propose « Continuer sur un autre harness… » à tout moment : grisé pendant un tour (« Attends la fin du tour ») ; une question ou une permission en attente passe dans le dossier.
- **Mode, modèle, effort.** Le mode du harness cible est le même s'il l'a (`auto` seulement si le modèle de passation le prend, d'après le Claude Code du compte cible), sinon le plus proche qui n'autorise jamais plus (ordre `plan` < `default` < `acceptEdits` < `auto` < `bypassPermissions` ; aucun : refusé). Le modèle est le « Modèle de passation » du harness, sinon son meilleur (le premier de son menu). L'effort est gardé s'il le prend.
- **Vue terminal.** Une vue terminal est d'abord ramenée à la vue Escouade ; le dossier y part par le flux. Puis, le tour du dossier fini (sauf arrêté par la limite), l'agent passe dans la vue terminal du harness cible, sur sa nouvelle session, si ce harness en a une ; une passation suivante avant la fin du tour garde ce choix. Un agent en vue Escouade y reste.
- **Le passage.** Sous le verrou de lancement, le dossier est écrit, le process arrêté et attendu (comme au changement de compte). L'agent prend le harness, le compte que choisit la règle de ce harness pour le projet (compte préféré s'il en est, sinon le premier compte bon) et une nouvelle session ; il retient d'où il vient (harness, compte, et jusqu'à quand : sa reprise prévue, sinon la fin des compteurs qui écartaient son compte, sinon 30 min ; rien pour une passation à la main avec du quota). Sa reprise prévue est effacée, et le compte quitté est retenu jusque-là.
- **Dans la conversation.** Une ligne de passation : « Suite sur Codex · quota Claude Code épuisé jusqu'à 18:00 » (“Continued on Codex · Claude Code quota spent until 6:00 PM”), « … · l'agent était sur Codex » pour une passation demandée. Chaque tour garde son harness (stats et coût) ; une fois la conversation passée par une passation, chaque tour l'affiche (« sur Codex »).
- **Dossier de passation**, dans la langue des textes rédigés, envoyé comme premier message (replié sous « Dossier de passation ») : en-tête (harness quitté, raison, même dossier et même branche) ; ticket, et rappel que son protocole est dans les consignes ajoutées (repassées au harness cible) ; demande d'origine ; messages suivants de l'utilisateur (les plus récents qui tiennent, le nombre des omis dit ; un dossier précédent n'en est pas) ; travail fait (avancement, tâches du plan et leur état, les 8 derniers messages de l'agent) ; fichiers modifiés (`git status --short`, `git diff --stat HEAD`, `git log <base>..HEAD`) ; dernier tour coupé (limite, interruption, erreur ou process parti avant la fin) et son dernier outil ; questions et permissions sans réponse. Borné à un quart de la fenêtre de contexte du modèle cible, à trois caractères par token. Pas de résumé par un modèle.
- **Automatique**, avec « Continuer sur le harness suivant quand tous les comptes d'un harness sont épuisés » : à la fin d'un tour arrêté par la limite, après la reprise sur un autre compte d'un agent de ticket si elle n'a pas eu lieu, l'agent dont le harness est épuisé passe sur le premier harness de secours qui a un compte utilisable, jamais sur celui qu'il a quitté. Un agent de ticket garde son ticket « En cours », que le pilote auto lit sur le nouveau harness. Un échec le dit (« Suite sur un autre harness impossible : … ») et l'agent attend la remise à zéro.
- **Retour** : jamais automatique. Une fois passé le moment retenu (ou sans moment retenu), la carte du dernier tour et le menu proposent « Revenir sur <harness> » : une passation dans l'autre sens (nouvelle session, dossier de toute la conversation).

### Réservations

- « Réservé aux projets » : un compte réservé ne sert que les projets de sa liste. Un compte réservé seulement à un projet fermé ne sert donc aucun projet ouvert.
- Fermer un projet en gardant son historique le laisse dans la liste de chaque compte. Il n'en sort que quand son historique est effacé (la case « Effacer aussi son historique » à la fermeture, ou « Effacer l'historique… » d'un projet fermé). Rouvrir le projet (même id) ne remet rien : les comptes n'ont pas bougé ; son « Compte préféré », s'il n'existe plus, repasse sur Automatique (voir *Fermer un projet*).
- Dans Réglages › Comptes, la liste « Réservé aux projets » du compte garde ce projet, coché et marqué « {nom} (fermé) » (“{name} (closed)”), qu'on décoche comme un projet ouvert (le compte l'oublie alors). Un projet que ni les projets ouverts ni les fermés ne connaissent reste invisible. Le groupe s'affiche aussi sans projet ouvert quand le compte est réservé à un projet fermé, et « Tous les projets » ne s'affiche que si la liste du compte est vide.

### Un compte réservé au projet privé

- Deux projets sur le même compte partagent son dossier (`CLAUDE_CONFIG_DIR`, `CODEX_HOME`…) : sessions, mémoire, historique, réglages globaux du harness. Un compte « réservé à ce projet » est un compte dont « Réservé aux projets » ne contient que lui.
- Dans les réglages d'un projet privé (Réglages › Projets, groupe « Confidentialité », dès que l'interrupteur est allumé, avant même d'enregistrer), la rangée « Comptes » dit, pour son harness par défaut et selon son brouillon :
  - « Compte réservé à ce projet : « Pro ». Sessions, mémoire et historique de Claude Code n’y sont partagés avec aucun autre projet. » (avec, pour le compte de la machine, « Le compte de la machine reste celui de Claude Code hors d’Escouade. ») ;
  - et/ou, en couleur d'attente, les comptes partagés où ses agents peuvent encore aller : « Ses agents peuvent partir sur « Principal » et « Pro », des comptes partagés avec d’autres projets : sessions, mémoire, historique et réglages globaux de Claude Code y sont communs. »
- Pour y remédier :
  - « Ajouter un compte réservé… » : la modale « Ajouter un compte », un nouveau compte proposé d'abord, qui dit « Réservé au projet « client » : … ». Le compte créé, ou celui de la machine, est réservé à ce projet seul, puis la modale revient aux réglages du projet, brouillon intact ;
  - par compte partagé, « Réserver <compte> à ce projet » (enregistré tout de suite, comme dans Réglages › Comptes ; les autres projets n'y partent plus).
- Dans Réglages › Comptes, un compte où part encore un projet privé qui ne l'a pas à lui seul le dit (« Le projet privé « client » part encore sur ce compte, partagé avec d’autres projets. »), avec « Réserver à « client » » quand il n'y en a qu'un.
- **Pas de bascule sur un compte partagé.** Un projet privé est « confiné » tant que « Comptes partagés à la limite » est éteint. Confiné, et avec au moins un compte actif réservé à lui seul sur un harness, il ne part que sur ceux-là (et sur son « Compte préféré », le choix de l'utilisateur) sur ce harness : nouveaux agents et tickets restent sur son compte au-delà du seuil, le pilote auto attend son compte (pause du projet), la reprise automatique d'un agent de ticket ne va que sur un autre compte à lui, et le passage automatique à un autre harness seulement si le compte qu'il y prendrait est à lui. Sans compte à lui, il garde ses comptes partagés (ce que dit l'avertissement), mais aucune bascule automatique ne le fait changer de compte : il attend la remise à zéro. À la main, « Reprendre sur <compte> » et « Continuer sur <harness> » restent offerts : c'est la permission de l'utilisateur.

### Statistiques et serveur MCP

- Chaque tour et chaque prompt est enregistré avec le compte de l'agent à ce moment ; un agent repris sur un autre compte compte chaque tour pour le compte de ce tour. Les tours d'une ancienne base, sans compte, comptent pour Principal (voir *Stats*).
- Le serveur MCP d'Escouade est déclaré dans chaque compte actif (voir *Serveur MCP*).

## Réglages

### Réglages › Comptes

Enregistré à chaque changement, hors du brouillon (une création ou une suppression touche le disque, « Annuler » ne pourrait pas la reprendre). Les comptes y sont groupés par harness (« Comptes Claude Code », …), dans « Ordre des harness ».

- **Par compte** : nom modifiable (unique sans tenir compte de la casse) ; dossier (`~` pour le dossier personnel) ; exécutable (« Celui des réglages » par défaut ; un chemin propre au compte qui ne trouve rien n'a pas de repli) ; état (« Vérification… », « Connecté · <e-mail> », « Pas connecté », « Pas connecté par un compte claude.ai (clé d’API ?) » pour Principal, « Inactif ») ; « Actif » (grisé pour le dernier compte actif) ; « Monter » / « Descendre » (l'ordre est la priorité) ; « Se connecter… » / « Se reconnecter… » ; « Réservé aux projets » ; « Nouveaux agents » (« Modèle », « Effort », « Mode ») ; « Supprimer ».
- **Supprimer** est refusé tant qu'un agent non archivé se sert du compte (« Le compte sert encore à N agents. »). Sinon, après « Supprimer le compte « Pro » ? », le compte part avec son quota, est retiré des « Compte préféré » des projets et de la config de Claude Code (entrée du serveur MCP), mais son dossier reste sur le disque, avec sa connexion et ses conversations. Le compte de la machine se retire de même.
- **« Ordre des harness »** : liste réglable, affichée dès deux harness, gardée hors du brouillon. Elle ordonne les groupes de Réglages › Comptes et du panneau des quotas, et donne les harness de secours d'un projet.
- **« Quotas » › « Changer de compte à »** : 80, 90, 95 % ou « À la limite » (défaut), enregistré tout de suite. Sa description : « Un agent passe sur un autre compte de son harness à la fin de son tour quand une fenêtre courte (5 h) de son compte atteint ce seuil, ou qu’une fenêtre longue (semaine, 30 jours, crédits du mois) l’atteint sans tenir jusqu’à sa remise à zéro au rythme actuel. Les nouveaux agents et les tickets partent sur le premier compte de l’ordre dont la 5 h n’est pas pleine et dont la semaine tient son rythme, même sur « À la limite » (un agent lancé ne bouge alors pas avant 100 %, puis « Reprendre sur un autre compte… », ci-dessous, décide). « Pause au-delà du quota » (Kanban) retient en plus les tickets quand tous les comptes l’ont passée. »

### Ailleurs

- Réglages › Harness : « Reprendre sur un autre compte un agent de ticket arrêté par la limite » (désactivé par défaut, visible avec plusieurs comptes) ; « Continuer sur le harness suivant quand tous les comptes d'un harness sont épuisés » (désactivé par défaut, visible avec deux harness trouvés) ; « Modèle de passation » de chaque harness.
- Réglages › Projets : « Compte préféré » (vide = « Automatique » ; masqué avec un seul compte). Groupe « Confidentialité » d'un projet privé : rangée « Comptes », et « Comptes partagés à la limite » (éteint par défaut : « À la limite d’usage de ses comptes réservés, ses agents peuvent continuer sur un compte partagé (changement de compte, passage à un autre harness). Éteint, ils attendent la remise à zéro. »).
- Le Kanban : « Pause au-delà du quota » (voir *Pilote auto*).

## Limites

- **Reprise sur un autre compte non vérifiée avec un second compte réel.** Vérifié : `claude -p --resume <id>` lancé depuis le même dossier de travail avec un autre `CLAUDE_CONFIG_DIR` trouve une session copiée dans son `projects/<cwd encodé>/` (sans connexion, le refus porte sur la connexion, « Not logged in » ; un identifiant inconnu donne « No conversation found with session ID »). Non vérifié : que l'API accepte une conversation commencée sur un autre compte (blocs de réflexion signés). Un refus arrive comme une erreur du premier tour, le cas que la carte sait dire et défaire. C'est pourquoi la reprise automatique est éteinte par défaut.
- **macOS, second compte non vérifié.** Le nom de l'entrée du trousseau d'un compte ajouté est lu dans le binaire de Claude Code, pas observé : un compte pourrait s'afficher « Pas connecté » sous macOS alors qu'il l'est.
- Sous Windows, un passage de Claude Code au Gestionnaire d'identification (drapeau distant) est hors périmètre.
- Une session ne passe ni d'un harness à l'autre (passation seulement), ni entre Windows et WSL (voir *Claude Code dans WSL*).
- Ce qui part d'un compte partagé reste partagé, fichiers partagés à la création compris (réglages, instructions, skills…) : la modale le dit.
- **Le rythme est une projection, pas une garantie.** Le jugement d'une fenêtre longue repose sur le pourcentage utilisé rapporté au temps écoulé : une fenêtre sans projection (première journée, remise à zéro inconnue) retombe sur le seuil, et un travail qui s'arrête la nuit ou le week-end peut faire passer un bon compte derrière les autres le temps d'une journée.

## Voyant des pastilles « Réservé aux projets »

Complète *Réservations*. Dans Réglages › Comptes, chaque pastille de la liste « Réservé aux projets » porte un petit carré décoratif (`aria-hidden`) qui sert de voyant à sa case :

- case cochée : le carré est allumé, à la couleur du projet, avec un léger halo ; pour un projet fermé (« {nom} (fermé) », qui n'a pas de couleur), il est allumé en neutre (la couleur du texte) ;
- case décochée : le carré est gris éteint (le texte à 28 % d'opacité), distinct du fond de la pastille et de l'état allumé en thème clair comme en thème sombre ;
- le changement est immédiat au clic et au clavier (Espace sur la case) ; la transition est coupée sous « réduire les animations » ;
- le fond et la couleur du texte de la pastille cochée sont inchangés, et l'accessibilité aussi : l'état reste porté par la case, dans le groupe « Réservé aux projets ». L'assistant « Nouveau projet » et les réglages d'un projet privé n'ont pas ce voyant.

- **Exception : « Tous les projets ».** Tant que rien n'est réservé au compte (liste vide, « Tous les projets »), le compte sert tous les projets : tous les carrés sont allumés, cases décochées comprises. Dès qu'un projet est coché, seuls les carrés des projets réservés restent allumés, les autres passent au gris. Cette exception l'emporte sur « case décochée : gris » ci-dessus.
