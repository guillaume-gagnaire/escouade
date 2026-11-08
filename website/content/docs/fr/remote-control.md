# Le remote control

Le remote control rend la session d’un agent accessible depuis claude.ai et l’app Claude sur mobile, pour que tu puisses lui répondre depuis un navigateur ou ton téléphone. Cette page explique comment l’activer, pour un agent ou pour tous, ce qui s’affiche, combien de temps il reste joignable et comment le couper.

![Escouade à gauche, avec l’icône du remote control sur la carte de l’agent « refacto-auth », et à droite l’app Claude sur mobile qui montre la même conversation, nommée « demo-api · refacto-auth »](/images/remote.jpg)

## Activer le remote control

Il y a trois façons de l’activer. Le résultat est le même : la session de l’agent porte le nom `projet · agent` sur claude.ai et dans l’app Claude, par exemple `demo-api · refacto-auth` (pour un agent de ticket, son nom vient du ticket). Renommer le projet renomme les sessions distantes de ses agents ; une interface de la [vue Claude Code](claude-code-view) garde l’ancien nom jusqu’à son prochain lancement.

- **Pour un agent**, par un clic droit sur sa carte.
- **Pour tous les agents d’un projet**, par un réglage du projet.
- **Pour tous les agents de tous les projets**, par un réglage global.

### Pour un agent

1. Fais un clic droit sur la carte de l’agent dans la barre latérale.
2. Choisis « Activer le remote control » (« claude.ai, mobile »).

Escouade démarre le processus de l’agent s’il est arrêté et demande à Claude Code d’ouvrir la session à distance. Un message confirme : « refacto-auth est accessible depuis claude.ai et l’app Claude ».

Si Claude Code ne peut pas ouvrir la session, un message d’erreur commençant par « Remote control indisponible » s’affiche, et l’option reste désactivée.

Le menu d’un agent archivé ne propose pas le remote control. Pour un agent qui tourne dans [la vue Claude Code](claude-code-view), voir [plus bas](#dans-la-vue-claude-code).

### Pour tous les agents, ou ceux d’un projet

Deux interrupteurs, éteints au départ, activent le remote control sans passer par chaque agent :

| Réglage | Où | Agents concernés |
|---|---|---|
| « Remote control pour tous les agents » | Réglages › Comptes et harness, groupe « Remote control » | Ceux de tous les projets |
| « Remote control pour les agents de ce projet » | Réglages › Projets › Confidentialité, groupe « Remote control » (sous « Identité ») | Ceux du projet choisi |

Comme les autres réglages, ils s’enregistrent avec « Enregistrer » (voir [les réglages des harness](settings-claude-code) et [ceux des projets](settings-projects)).

Allumé, un réglage vaut pour :

- chaque agent du projet (ou de tous les projets) qui existe déjà et n’est pas archivé ;
- chaque nouvel agent, ceux des tickets du Kanban et les copies de « Dupliquer la conversation » compris.

Escouade lance tout de suite le processus de chaque agent, au plus 3 à la fois, et il reste lancé : il démarre avec l’app et n’est jamais arrêté pour inactivité. Cela garde un processus `claude` en mémoire pour chaque agent, ce que dit la description du réglage.

Éteint, un réglage retire le remote control aux agents qui le tenaient de lui, et leur session distante est fermée. Il ne touche ni aux agents dont tu as activé le remote control à la main, ni à ceux qu’un autre réglage couvre encore : si tu éteins le réglage global mais que celui d’un projet est allumé, les agents de ce projet gardent le leur, grâce au projet.

Les deux réglages ne se contredisent jamais. Tant que « Remote control pour tous les agents » est allumé, l’interrupteur du projet se montre allumé et grisé, avec « Activé pour tous les projets dans Réglages › Agents ». Le choix propre du projet est gardé : une fois le réglage global éteint, l’interrupteur du projet retrouve son état.

## Ce qui s’affiche dans Escouade

Une icône d’onde apparaît sur la carte de l’agent, après son nom. Elle dit où en est la liaison, et son infobulle la détaille :

| Icône | Début de l’infobulle | Sens |
|---|---|---|
| Bleue | « Remote control : connecté » | La session est ouverte et connectée. |
| Grise | « Remote control : connexion… » ou « Remote control : en attente de connexion » | La liaison s’établit. |

Chaque infobulle contient « (accessible depuis claude.ai et l’app Claude) ». Pour un agent en [vue Claude Code](claude-code-view), l’icône dit l’état de son interface (voir [plus bas](#l-icone-d-une-interface)).

L’infobulle se termine par ce qui a activé le remote control de l’agent, après un point médian, par exemple « Remote control : connecté (accessible depuis claude.ai et l’app Claude) · activé à la main » :

| Mention | Sens |
|---|---|
| « activé à la main » | Tu l’as activé par le clic droit sur la carte. |
| « activé par le réglage du projet » | « Remote control pour les agents de ce projet » est allumé. |
| « activé par le réglage de tous les agents » | « Remote control pour tous les agents » est allumé. |

Quand l’agent est joignable, le menu de la carte change :

| Entrée | Effet |
|---|---|
| Désactiver le remote control | Coupe la liaison et met l’agent à part (voir plus bas). |
| Ouvrir sur claude.ai | Ouvre la session dans ton navigateur. |
| Copier le lien claude.ai | Copie l’adresse de la session (« Lien claude.ai copié »), pour l’ouvrir sur un autre appareil. |

Les deux dernières entrées apparaissent dès que l’adresse de la session est connue. Le clic droit reste possible quelle que soit la source du remote control.

Ce que tu envoies à l’agent depuis claude.ai ou l’app Claude s’affiche aussi dans la conversation d’Escouade, dans une bulle comme les tiennes, avec la mention « depuis claude.ai » (son infobulle dit : « Envoyé depuis claude.ai ou l’app Claude (remote control) »).

Claude, hors d’Escouade ou dans un agent, peut lire cet état par le [serveur MCP](mcp) sans pouvoir le changer : `list_agents` et `get_agent_summary` disent, pour chaque agent, si le remote control est en vigueur, ce qui l’a allumé et l’état du lien, et `list_projects` donne les deux réglages. Aucun outil ne l’allume ni ne l’éteint : cela reste ta décision, dans la fenêtre.

## Combien de temps l’agent reste joignable

- **Tant qu’Escouade tourne.** L’agent reste joignable quand tu fermes la fenêtre : elle se cache et l’app continue dans la zone de notification (voir [les agents](managing-agents#fermer-la-fenetre-quitter-et-redemarrer)). Si tu quittes l’app avec « Quitter », les agents s’arrêtent et ne sont plus joignables.
- **Sans arrêt pour inactivité.** Escouade arrête normalement le processus d’un agent inactif au bout d’un délai (30 minutes par défaut) ; un agent dont le remote control est activé garde le sien, pour rester joignable.
- **Après un redémarrage.** Escouade relance les agents dont le remote control est activé, à la main ou par un réglage, en même temps que l’app et rattache leur session à distance précédente : le lien que tu avais ouvert sur ton téléphone reste valide, et la session garde son nom `projet · agent`. Si cette session a disparu entre-temps, une nouvelle est ouverte, avec un nouveau lien. Si la liaison échoue au démarrage, un avis « Remote control indisponible : » suivi de la cause s’affiche dans la conversation de l’agent. Cet avis n’est dit qu’une fois par agent et par lancement d’Escouade, jusqu’à ce qu’un lien réussisse ou que tu rallumes le remote control à la main.

## Désactiver le remote control

Fais un clic droit sur la carte de l’agent, puis « Désactiver le remote control ». Escouade ferme la session à distance, l’icône disparaît, et l’agent redevient un agent ordinaire (que l’arrêt pour inactivité peut de nouveau arrêter), mis à part (voir ci-dessous). Archiver un agent ou le supprimer coupe aussi son remote control ; restaurer un agent archivé le lui rend si un réglage le couvre (pas s’il l’avait activé à la main).

Couper le remote control, par le clic droit, en éteignant un réglage, en archivant l’agent ou en le supprimant, ferme la session à distance : elle est archivée sur claude.ai.

Si aucun processus de l’agent ne tourne à ce moment (juste après son démarrage, après un lien raté, processus mort), Escouade ne peut pas la fermer tout de suite. Elle se souvient de la session : elle la ferme au prochain démarrage d’un processus de l’agent, ou la reprend, avec le même lien, si tu as rallumé le remote control d’ici là. Couper puis rallumer vite ne mêle pas les demandes : elles passent l’une après l’autre.

### Mettre un agent à part

« Désactiver le remote control » met toujours l’agent à part, qu’un réglage (celui de son projet ou le réglage global) le couvre ou non à ce moment. Un réglage allumé ensuite ne lui rend pas le remote control, ni au redémarrage d’Escouade. L’agent reste éteint jusqu’à ce que tu le rallumes à la main, par « Activer le remote control » dans son menu.

Une copie de l’agent (« Dupliquer la conversation ») est mise à part aussi. Le ticket d’un agent mis à part garde cet état : un ticket renvoyé en « À faire » puis relancé crée un agent lui aussi mis à part, sauf si tu as rallumé le remote control à la main.

## Dans la vue Claude Code

La [vue Claude Code](claude-code-view) suit les mêmes règles, et un agent peut y passer quand son remote control est activé. C’est l’interface de Claude Code qui tient alors la session distante, pas Escouade.

- **Le lancement.** L’interface de Claude Code d’un agent en remote control est lancée avec `--remote-control=<projet · agent>`, en un seul argument (un nom qui commence par « - » n’est donc pas pris pour une option). Claude Code affiche alors le lien de la session. Quand l’interface reprend sa session (redémarrage d’Escouade, relance), Claude Code rattache la même session distante, avec le même lien. Une interface fermée archive sa session distante.
- **Au démarrage d’Escouade.** L’interface d’un tel agent est lancée en fond, sans que tu l’ouvres, pour qu’il soit joignable. Ouvrir l’agent affiche l’interface déjà lancée. Ces lancements se font au plus 3 à la fois, comme ceux d’un réglage : un démarrage garde sa place jusqu’à ce que l’interface soit prête, 15 secondes au plus. Si le remote control de cet agent s’éteint avant que tu l’aies ouverte, elle s’arrête, sauf si l’agent est en plein tour (son travail serait perdu).
- **Un changement pendant que l’interface tourne.** Escouade ne peut pas allumer ni couper le remote control d’une interface déjà ouverte. Que le changement vienne d’un réglage ou du clic droit, il vaut au prochain lancement de son interface. Après un clic droit, un message le dit : « Pris en compte au prochain lancement de son interface Claude Code ; d’ici là, c’est sa commande /remote-control qui le fait. » Pour un réglage, c’est sa description qui le dit.
- **Le menu de la carte.** Il propose « Activer le remote control » ou « Désactiver le remote control ». « Ouvrir sur claude.ai » et « Copier le lien claude.ai » sont proposés aussi, une fois l’interface connectée : le lien vient de l’interface.
- **Le passage d’une vue à l’autre.** Passer un agent en vue Claude Code ferme la session distante de sa vue Escouade : son interface ouvre la sienne. Revenir en vue Escouade en ouvre une nouvelle, avec un nouveau lien.
- **Quand Escouade ferme l’interface.** Une interface qu’Escouade arrête pour de bon (retour en vue Escouade, archivage, suppression, interface jamais affichée dont le remote control s’éteint) est fermée en douceur, pour que Claude Code archive sa session : Escouade envoie deux `Ctrl+C`, comme toi, puis arrête l’interface de force si elle n’est pas fermée au bout de 5 secondes. Un démarrage demandé pendant ce temps attend que l’ancienne soit partie : une session n’a jamais deux processus. À la fermeture d’Escouade, elle ne l’est pas, pour que le lien revienne au prochain démarrage.
- **Le nom de la session.** Renommer le projet ne change pas le nom de la session d’une interface déjà lancée : elle garde l’ancien nom jusqu’à son prochain lancement.

### L’icône d’une interface

L’infobulle de l’icône dit où en est l’interface :

| Infobulle | Sens |
|---|---|
| « Remote control : aucune interface lancée, pas joignable pour l’instant (ouvre l’agent pour la lancer) » | Aucune interface ne tourne : l’agent n’est pas joignable. |
| « Remote control : connexion… (accessible depuis claude.ai et l’app Claude) » | L’interface est lancée avec le remote control et la liaison s’établit. |
| « Remote control : connecté (accessible depuis claude.ai et l’app Claude) » | La session est connectée. L’icône est bleue. |
| « Remote control : au prochain lancement de son interface Claude Code » | L’interface ouverte a été lancée avant que le remote control s’allume. |
| « Remote control : pas ouvert par Claude Code (compte sans abonnement claude.ai, réseau, proxy ou règle de l’organisation) » | L’interface tourne, mais Claude Code n’a pas ouvert la session distante (voir [plus bas](#quand-claude-code-n-ouvre-pas-la-session)). La conversation le dit une fois. |
| « Remote control encore actif dans l’interface ouverte, jusqu’à son prochain lancement (sa commande /remote-control le coupe) » | Le remote control est éteint pour Escouade, mais l’interface ouverte y est encore reliée. L’icône est barrée, dans la couleur d’avertissement, jusqu’au prochain lancement. Elle n’apparaît que si l’interface a vraiment eu son lien (« connecté ») : une interface lancée avec l’option mais jamais liée n’en a pas. |

### Quand Claude Code n’ouvre pas la session

Claude Code n’écrit pas dans le terminal que le remote control a échoué. Escouade le déduit, de deux façons, et l’icône dit alors « pas ouvert par Claude Code » :

- Le suivi de l’interface (son transcript) contient une entrée `bridge_status` sans lien. L’avis de la conversation est « Remote control indisponible : Claude Code n’a pas pu ouvrir la session distante », suivi, entre parenthèses, de ce que Claude Code a dit.
- Rien ne prouve un lien 20 secondes après que l’interface est prête. L’avis est « Remote control indisponible : l’interface de Claude Code ne l’a pas ouvert (compte sans abonnement claude.ai, réseau, proxy ou règle de l’organisation). »

L’interface continue sans remote control. Chaque avis n’est dit qu’une fois par agent et par lancement d’Escouade, et le remote control reste activé pour l’agent : le prochain lancement de son interface réessaie. Si un lien arrive plus tard, l’icône repasse à « connecté ».

### Quand l’interface se ferme aussitôt

Si une interface lancée avec le remote control, que tu n’as jamais affichée, se ferme dans les 10 secondes, un avis « Remote control indisponible : l’interface de Claude Code s’est fermée juste après son lancement avec le remote control ; elle est relancée sans lui. » s’affiche dans la conversation. Elle est relancée tout de suite en fond, sans l’option, et les lancements suivants de cet agent se font sans l’option, jusqu’au prochain démarrage d’Escouade ou jusqu’à ce que tu rallumes le remote control à la main.

Une interface que tu as vue et qui se ferme dans ce délai n’est jamais prise pour un échec de l’option : « No, exit » sur la boîte de confiance du dossier, `Échap`, `/exit` ou deux `Ctrl+C` ne donnent ni relance sans l’option ni avis. Si tu veux la relancer, le bouton « Relancer » du terminal est à toi.

### Un message refusé

Un message de la fenêtre ou du serveur MCP est refusé tant que l’interface n’est pas prête : « L’interface de Claude Code de cet agent n’est pas encore prête (premier lancement ou dossier à approuver) : ouvre l’agent pour la voir. » C’est le cas à l’écran du premier lancement d’un compte, et quand un dossier est à approuver (le choix par défaut y est « No, exit » : un texte tapé le choisirait). Ouvre l’agent pour voir l’interface.

Une interface est prête dès que l’un de ces signes est là : tu l’as affichée ; son transcript contient une entrée `bridge_status` ou `bridge-session` ; ou Claude Code l’a inscrite dans son registre des sessions, ce qu’il fait en moins de 2 secondes une fois prête, et pas sur l’écran du premier lancement ni sur la boîte de confiance. Cela vaut aussi pour une interface relancée sans l’option.
