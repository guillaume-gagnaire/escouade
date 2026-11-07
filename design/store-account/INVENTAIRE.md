# Espace de l'auteur du store : inventaire (ESC-318, phase 1)

Relevé sur `store/app/components/StoreAccount.vue`, `store/app/data/account.ts`, `store/server/api/account.get.ts`
et `store/server/utils/author.ts` le 2026-10-31, avant la modale. Aperçus des trois directions : `index.html`
(`?d=a|b|c`, `&modal=0`, `&step=delete`, `&theme=light`), captures par `node design/store-account/render.mjs`.

## Ce que montre la page aujourd'hui, de haut en bas

| Endroit | États | Problème de lecture |
| --- | --- | --- |
| Titre « Mes plugins · login » | — | — |
| Notice après une action (`?notice=`) | ok, warn, bad ; 40 clés `plugin-*`, `version-*`, `email-*`… | Elle paraît en haut de page, loin du plugin touché. |
| Bandeaux | transfert offert (Accepter / Refuser), dépôt repris sur GitHub (Reprendre), version qui attend ton clic (Ignorer / Publier, permissions en plus, gel) | La version qui attend n'est pas sur la ligne de son plugin. |
| Liste « Mes plugins » | zéro (vide + « Soumettre un plugin »), un, beaucoup ; état par ligne : vérifié (sceau), publié non vérifié, en revue, retiré de la liste, supprimé, versions retirées ; première version rejetée (raison sous la ligne), en attente, revue manuelle, en contrôle, contrôles interrompus (→ « Reprendre la soumission ») | Le « ⋯ » recharge la page vers `#manage`, sans dire ce qu'on va trouver. |
| Section `#manage` du plugin choisi (`?plugin=`, sinon le premier) | publié, transféré (« Transféré par… »), gelé 7 jours, supprimé (annuler, 30 jours), transfert proposé (annuler), publication auto oui/non, dans la liste oui/non ; versions : publiée, en revue, vérifiée, changements demandés, revue refusée, retirée, en attente, rejetée, étiquette déplacée | Sous la liste, loin de la ligne ; le premier plugin est choisi sans qu'on l'ait demandé. |
| Colonne de droite : installations par semaine du plugin choisi | 12 semaines, total, 30 jours, mises à jour | Coupée de son plugin (à droite, la gestion à gauche). |
| L'App sur tes dépôts | aucune installation, une ou plusieurs, organisation, suspendue | — |
| Avis par e-mail | sans adresse, vérifiée, en attente, envoi coupé | — |
| Applications connectées (ESC-317, branche `ticket/esc-317`) | à venir : `ConnectedApps` entre « Avis par e-mail » et « Mes données » | Sa place reste dans la colonne de droite. |
| Mes données | exporter, se déconnecter, supprimer le compte (bloqué tant qu'il reste des plugins) | — |
| Jetons de la revue (administrateur) | — | — |

Plugins de première main (`dev.escouade.*`, ESC-314) : mêmes lignes, même modale ; rien à part dans l'espace de
l'auteur (le préfixe se garde à la soumission).

Téléphone (≤ 720 px) : une colonne, la colonne de droite passe sous la liste, les lignes se replient (plugin et
« ⋯ » sur une ligne, état dessous) ; la gestion d'un plugin est tout en bas d'une longue page.

## Ce que la modale change, quelle que soit la direction

- Chaque ligne a son « ⋯ » (nom accessible « Gérer <nom> ») qui ouvre la modale du plugin, sans quitter la liste.
- Tout ce qui touche un plugin va dans sa modale : ses chiffres, ses versions (retirer, publier celle qui attend),
  publication auto, liste, puis à part les gestes dangereux : transfert, suppression confirmée (identifiant
  retapé), annulation de la suppression.
- L'adresse garde l'état : `?plugin=<id>` ouvre la modale ; un formulaire (POST puis redirection avec `notice`)
  y revient, la notice dans la modale.
- La colonne de droite ne garde que le compte : e-mail, Applications connectées (ESC-317), Mes données.

## Choix et réalisation (2026-10-31)

Direction **A, la fiche**, choisie par l'utilisateur sur aperçu (`a-list.png`, `a-modal.png`, `a-mobile.png`) ;
B (onglets) et C (tiroir) écartées. Réalisée dans `store/app/components/StoreDialog.vue` (la coque : `<dialog>` rendu
ouvert par le serveur, rendu modal par le script, focus sur le titre et gardé dedans, Échap, voile, focus rendu au
« ⋯ ») et `StoreAccount.vue` (le contenu). Écarts au rendu de l'aperçu, voulus :

- la colonne « Mises à jour » des versions quitte la modale (le total est sur la ligne des chiffres) ;
- la version qui attend ton clic garde aussi son bandeau en haut de la liste ; « Publier » sur sa ligne de versions ;
- « Transférer… » et « Supprimer le plugin » ouvrent leur formulaire sur place (`<details>`, sans script), le champ
  prend le focus pour paraître au-dessus du clavier sur un téléphone.
