---
number: 222
id: "la-2-2-4-reversee-dans-la-2-3-l-ecran-et"
title: "La 2.2.4 reversée dans la 2.3 : l'écran et les vues de la boîte servis à distance, ses fichiers ouverts sur le poste"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-540"
pages: ["harness/boite-a-toi"]
tickets: ["ESC-540","ESC-510","ESC-513","ESC-521","ESC-538"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/boite-a-toi","added":6,"removed":0}]
accepted_at: "2026-11-20T07:28:41+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La 2.2.4 (étiquette v2.2.4) livre sur main l'écran de la boîte « À toi » (ESC-510), ce qu'Escouade y verse de lui-même (ESC-513), l'arrêt en Bypass (ESC-521), les points réservés du manifeste (ESC-524), les sauvegardes à la sortie (ESC-535) et le hook NSIS (ESC-533). La 2.3 a une liste unique des commandes, servies ou non à un client distant, une fenêtre qui peut parler au cœur d'un serveur (ESC-537), et son propre jugement des modes (ESC-497). docs/INBOX.md § 18 proposait de classer inbox_save_file et inbox_open_file « du poste » et de ne pas servir inbox_track.

## Décision

Une vraie fusion de v2.2.4 dans release/2.3. inbox_views, inbox_view_act, inbox_view_snooze et inbox_track sont servies à un client distant : les vues sont lues dans l'état du cœur, leurs gestes sont ceux de leurs sources, et ce que la fenêtre voit de la boîte est compté sur le cœur dont c'est la boîte. inbox_save_file change de machine, refusée à distance (not-served-yet) comme inbox_open_file, plutôt que « du poste » : une commande du poste part au cœur du poste, qui n'a pas la demande d'un serveur. Une fenêtre sur un serveur ouvre un fichier de la boîte comme ceux d'un agent (ESC-538) : ses octets téléchargés par inbox_file_bytes, la copie gardée et ouverte sur le poste. Le résumé des notifications passe par l'hôte du cœur (Host::notify). L'arrêt en Bypass reste celui de la 2.3 (judge_start, settle_mode, avec les stratégies) ; le jugement d'un mode inchangé n'émet plus l'agent ni ne sauvegarde ; la notice d'arrêt prend le niveau « warn », qui a un style.

## Conséquences

Une fenêtre connectée à un serveur lit et règle la boîte du serveur, vues comprises, et en ouvre les fichiers sur le poste. « Enregistrer sous… » y reste refusé jusqu'à un ticket de suite (le dialogue du poste sur les octets téléchargés). Restent aussi : l'Overseer et la délégation, les relectures de PR de la 2.3, Word et CSV par les aperçus de l'éditeur, les réglages imposés de la boîte, la station de l'onboarding (docs/INBOX.md § 16).
