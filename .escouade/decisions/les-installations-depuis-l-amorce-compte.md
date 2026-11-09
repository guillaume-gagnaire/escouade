---
number: null
id: "les-installations-depuis-l-amorce-compte"
title: "Les installations depuis l'amorce comptent au store, gardées en attente jusqu'à l'accord"
date: "2026-11-09"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-394"
pages: ["store/store-dans-l-app","plugins/livres-avec-escouade"]
tickets: ["ESC-394"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/store-dans-l-app","added":6,"removed":0},{"page":"plugins/livres-avec-escouade","added":2,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Retour sur la bêta : les plugins que l'app installe d'elle-même depuis l'amorce (ESC-338, ESC-373) ne comptaient pas dans le nombre d'installations du store. Seul « Installer » depuis le store envoyait le compte. Or l'amorce installe hors ligne, souvent avant que l'utilisateur ait répondu sur les statistiques.

## Décision

Une installation depuis l'amorce (au démarrage, pour un geste, au clic) compte en `event: "install"`, avec la même règle de consentement. Le compte est gardé en attente dans `<plugins>/.seed.json` (par id et version). Il part quand l'accord vaut oui et que le store répond, et il est oublié sur un non. Une version comptée ne compte plus. Un échec (réseau, 5xx, 408, 429) attend la prochaine occasion : le démarrage, la relecture du store toutes les 6 heures, un geste ou l'accord. Il n'y a pas de boucle serrée. Les plugins de l'amorce d'une bêta, jamais comptés, se comptent une fois. Ne comptent pas : un `builtin`, un dossier en développement, l'amorce d'un build de développement.

## Conséquences

Les chiffres du store incluent les installations de la première main faites par l'installeur. Le corps envoyé reste les trois clés, sans identifiant de la machine. Une mise à jour vers une version de l'amorce, au clic, compte en « install ».
