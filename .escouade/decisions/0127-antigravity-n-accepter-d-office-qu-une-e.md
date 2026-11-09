---
number: 127
id: "antigravity-n-accepter-d-office-qu-une-e"
title: "Antigravity : n'accepter d'office qu'une écriture qui n'atteint qu'un fichier, dit par le cœur (fs/fileOnly)"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-389"
pages: ["plugins/antigravity-en-plugin"]
tickets: ["ESC-389"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/antigravity-en-plugin","added":24,"removed":16}]
accepted_at: "2026-11-08T15:37:35+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de dev.escouade.antigravity-cli@2.2.3 (A1, bloquant) : en « Demander » et « Édits auto », une écriture strictement sous le dossier de l'agent était acceptée d'office, même si son chemin était un sous-dossier (C:\wt\sub). Avec agy 1.5.3, write_file(<dossier>) couvre <dossier>\.git (mesuré par real_agy_rules), et guard_rules ne refusait les dossiers gardés qu'à la racine. Le modèle pouvait écrire sans carte le fichier .git d'un sous-module vers un gitdir à lui (core.fsmonitor), que le prochain git status du cœur exécute. La revue proposait deux pistes : que l'hôte dise au module si le chemin est un fichier, ou, à défaut, refuser les noms gardés sous chaque chemin accordé et mettre en carte une écriture sans extension. L'essai réel d'ESC-389 a montré en plus qu'un deny d'agy 1.5.3 ne tient que pour le nom qu'il dit : un refus de <dossier>\.git laisse passer <dossier>\GIT~1 et <dossier>\.git., qui mènent au même dossier.

## Décision

La première piste, avec les refus de la seconde en plus. (1) Le cœur sert une méthode de l'hôte, fs/fileOnly { agent, path } → { fileOnly } : vrai pour un fichier (pas un lien) du dossier de l'agent, ou un chemin absent sans sous-module de l'index git à ce chemin ni dessous (git ls-files --stage, chemin littéral, sans fsmonitor), ses liens restant dans le dossier ; faux pour tout le reste, un git qui ne répond pas en 1 s, un cœur qui ne connaît pas la méthode. Elle ne demande aucune racine de permissions.files (pas d'agentRepo) et ne rend qu'un booléen d'un chemin du dossier d'un agent du plugin. Le plugin n'accepte d'office une écriture, ou un « Pour cet agent » d'écriture, que si elle est vraie, et n'offre pas « Pour cet agent » sinon. (2) Pendant une reprise qui porte une écriture, les règles refusent aussi .git, .husky, .githooks, .agents et .gemini juste sous chaque chemin accordé, y compris sur la carte. Pourquoi pas la seule seconde piste : un dossier peut avoir un point dans son nom (vendor.d) et tenir un sous-module plus bas, et un deny ne tient pas contre un nom court ni un point final ; seule la connaissance du disque garantit qu'aucune acceptation d'office ne couvre un dossier gardé, à toute profondeur. Les refus juste sous chaque chemin restent comme garde du nom exact, surtout pour ce que l'utilisateur accorde sur la carte, dont la note dit qu'une écriture couvre aussi ce qui est sous son chemin. Aussi : un refus de denied_actions qu'aucun appel ne rattache est dit dans la conversation ; la carte et l'avis d'office ne parlent plus de « début de texte » ; la vue terminal du cœur donne à agy le dossier temporaire du plugin (<home>/tmp). La prudence d'ESC-376 sur les commandes reste.

## Conséquences

Plugin 2.2.4. Un dossier, un sous-module ou ce qui en tient un, et un lien vont en carte, sans « Pour cet agent ». Avec un cœur sans fs/fileOnly (avant la 2.2), chaque écriture en Demander et Édits auto va en carte : sûr, mais plus de cartes. Une écriture acceptée coûte un git ls-files quand le chemin est absent dans un dépôt. Un chemin absent accepté reste couvert pour ce que l'agent crée lui-même dessous pendant la reprise, .git compris par un point final : un dépôt créé là n'est pas un sous-module, et le git du cœur n'y entre pas. Une carte de dossier acceptée par l'utilisateur ouvre ce qui est dessous, un nom court du .git compris. real_agy_rules a 5 cas de plus (sous-dossier présent ou non qui couvre son .git, deny qui l'emporte, deny que GIT~1 et .git. passent) et passe en entier sur agy 1.5.3.
