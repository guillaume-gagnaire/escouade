//! A small set of searches in French and in English, to choose the model and to follow its quality: 24 passages of a
//! project's docs and 24 questions, each of which says what a passage says in other words (some across the languages).

/// (id, text)
pub const PASSAGES: &[(&str, &str)] = &[
    ("worktree", "Chaque agent travaille dans son propre worktree git, sur une branche à lui. Les modifications d'un agent ne touchent jamais le dossier principal du projet tant que le ticket n'est pas fusionné."),
    ("kanban", "The Kanban board has columns for backlog, in progress, review and done. A ticket moves to review when its agent reports that every acceptance criterion is met."),
    ("dictation", "La dictée vocale transcrit la parole en texte avec un modèle Whisper qui tourne sur la machine. Le modèle est téléchargé à la première utilisation, puis la dictée fonctionne hors ligne."),
    ("proxy", "Behind a corporate proxy the app reads the proxy address from its settings. Requests to local addresses are listed in the no-proxy field and never go through it."),
    ("memory", "La mémoire partagée est un carnet de travail : un fichier Markdown par souvenir, avec un index court. Les agents y notent les écueils rencontrés pour que les suivants ne les refassent pas."),
    ("release", "A release is published once the tag is pushed and the continuous integration is green. The release notes are copied from the matching section of the changelog."),
    ("hooks", "Les hooks sont des petits programmes que le harness appelle avant et après chaque outil. Escouade s'en sert pour savoir ce que fait l'agent et pour lui refuser certaines actions."),
    ("private", "A private project keeps some files out of reach: the agents of other projects cannot see them, and they are never indexed nor shown in a search result."),
    ("quota", "Le quota d'un compte se lit au fil des tours : pourcentage de la fenêtre de cinq heures et de la semaine. Au seuil réglé, Escouade peut passer l'agent sur un autre compte."),
    ("conflict", "When a merge conflicts, the ticket goes back to its agent, who resolves the conflicts in the worktree. The user can also cancel the merge and keep the branch as it was."),
    ("onnx", "Les voix de synthèse sont des modèles ONNX exécutés par une bibliothèque embarquée dans l'application. Elle est chargée au premier usage, ce qui évite de ralentir le démarrage."),
    ("terminal", "The embedded terminal shows the real interface of the agent's harness. Its size follows the area it is drawn in, so the program lays its lines out for what the user sees."),
    ("plan", "Avant de coder, l'agent peut proposer un plan. Le plan s'affiche dans un bandeau au-dessus de la conversation, et l'utilisateur l'approuve ou le renvoie avec ses remarques."),
    ("pricing", "The cost of a turn is computed from the tokens used and the price table of the model. Subscription accounts show their quota instead of a dollar amount."),
    ("wizard", "L'assistant de nouveau projet propose de créer un dépôt, de cloner un dépôt existant ou d'ouvrir un dossier. Il peut aussi copier les fichiers qui ne sont pas versionnés, comme le fichier d'environnement."),
    ("branch", "The branch an agent starts from is a setting of the project, which an agent can override. A remote branch becomes a local branch that tracks it."),
    ("shortcut", "Les raccourcis clavier se règlent dans les réglages. Ctrl+N crée un agent, Ctrl+J met le focus sur la carte, et les touches 1, 2, 3 répondent aux questions de la liste."),
    ("update", "The app checks for updates in the background and shows a banner when one is ready. On Windows the installer is signed, which avoids the SmartScreen warning."),
    ("attach", "On peut joindre des fichiers et des images à un message. Escouade garde une copie dans son dossier de données et indique à l'agent le chemin d'origine quand elle le connaît."),
    ("stats", "The statistics page charts the cost per day, the tokens per model and the time agents spent working. All of it is computed from a local database; nothing is sent anywhere."),
    ("workflow", "Un workflow décrit les étapes d'un ticket : écrire, relire, tester, ouvrir la pull request. Il est défini dans un dossier du dépôt et partagé avec toute l'équipe."),
    ("telemetry", "Anonymous usage statistics are optional. They contain no code, no file name and no prompt, and they can be turned off in the settings."),
    ("cache", "Le cache de prompt expire au bout de cinq minutes ou d'une heure selon le compte. Un agent resté inactif plus longtemps paie à nouveau tout son contexte au message suivant."),
    ("review", "The pull request view lists the checks of the continuous integration and the comments of the reviewers. An agent can be asked to fix a failing check by itself."),
];

/// (question, id of the passage it should find)
pub const QUERIES: &[(&str, &str)] = &[
    (
        "où travaillent les agents pour ne pas toucher au code principal ?",
        "worktree",
    ),
    (
        "how do I know a task is ready to be looked at by a human?",
        "kanban",
    ),
    (
        "puis-je parler au lieu de taper, sans connexion internet ?",
        "dictation",
    ),
    (
        "what to do when requests must go through the company gateway",
        "proxy",
    ),
    (
        "comment les agents se transmettent ce qu'ils ont appris ?",
        "memory",
    ),
    ("when does a new version get shipped to users", "release"),
    (
        "comment surveiller et bloquer les actions d'un agent ?",
        "hooks",
    ),
    (
        "can another team see confidential files of my project?",
        "private",
    ),
    (
        "que se passe-t-il quand un compte approche de sa limite ?",
        "quota",
    ),
    (
        "who fixes the clashing changes when two branches touch the same lines",
        "conflict",
    ),
    (
        "quand est chargé le moteur qui fait parler l'application ?",
        "onnx",
    ),
    ("why does the console adapt to the window size", "terminal"),
    (
        "l'agent peut-il soumettre sa stratégie avant d'écrire du code ?",
        "plan",
    ),
    ("how is the money spent by an agent calculated", "pricing"),
    (
        "comment démarrer un projet à partir d'un dépôt qui existe déjà ?",
        "wizard",
    ),
    ("from which branch is a new agent created", "branch"),
    (
        "quelle touche permet de lancer un nouvel agent rapidement ?",
        "shortcut",
    ),
    (
        "avoiding the Windows warning about an unknown publisher",
        "update",
    ),
    ("puis-je envoyer une capture d'écran à l'agent ?", "attach"),
    ("is my usage data uploaded to a server", "stats"),
    (
        "comment enchaîner rédaction, relecture et tests pour chaque ticket ?",
        "workflow",
    ),
    ("does the app phone home with my prompts", "telemetry"),
    (
        "pourquoi un agent endormi coûte-t-il plus cher à la reprise ?",
        "cache",
    ),
    ("how to get the robot to repair a red build", "review"),
];
