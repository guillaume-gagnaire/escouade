---
number: 125
id: "antigravity-toolpermission-ecrit-request"
title: "Antigravity sur agy 1.5.3 : request-review écrit, refus dits finis retrouvés, dossier temporaire de l'agent, plus de deny de remontée"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-388"
pages: ["plugins/antigravity-en-plugin"]
tickets: ["ESC-388"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/antigravity-en-plugin","added":16,"removed":12}]
accepted_at: "2026-11-08T11:53:51+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'essai réel du 2026-11-08 sur agy 1.5.3 semblait montrer qu'en -p une commande ou une écriture sans règle passait. Mesuré à part (journal d'agy, effets sur disque) : 1) agy refuse bien, mais son flux dit parfois l'étape refusée DONE sans erreur ; le refus n'est que dans les denied_actions du result, et l'étape n'est jamais écrite au transcript ; le plugin ne posait alors aucune carte. 2) agy écrit sans règle ni question dans son dossier temporaire, qui était le %TEMP% de l'utilisateur. 3) Un deny de `<fichier>\..` est lu comme le dossier du fichier et refusait l'écriture même que l'utilisateur avait accordée. 4) Une règle tient la commande entière, pas son début ; `*` n'est un joker que seul.

## Décision

Le plugin écrit toujours toolPermission: "request-review". En fin de tour, chaque refus de denied_actions que n'explique aucune étape refusée est rattaché au dernier appel fini de la même action que le transcript ne dit pas fini ; sa ligne passe en échec et il est jugé comme tout refus (carte comprise). TEMP, TMP et TMPDIR d'agy sont `<dossier de l'agent>/tmp`. Une reprise avec écriture accordée ne refuse plus `<chemin>\..` ni `<chemin>/..` (agy refuse déjà `<fichier>\..\.git\x`). real_agy_rules lance agy comme le plugin, juge aussi par denied_actions, prend un modèle au choix, et finit par le client du plugin qui pilote agy à travers une carte acceptée. La prudence d'ESC-376 (aucune commande acceptée d'office) n'est pas relâchée ici.

## Conséquences

Plugin 2.2.3. Une carte de trop si agy listait un refus qui n'est pas celui rattaché (non observé : un refus clôt le tour). La vue terminal du cœur garde le %TEMP% de l'utilisateur et le défaut d'agy (son interface demande). Relâcher la prudence d'ESC-376 sur les commandes reste à décider.
