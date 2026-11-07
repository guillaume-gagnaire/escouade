# ESC-68 — le groupe « Pull requests » de la vue d'ensemble

Un seul aperçu, dans le design existant : ce groupe et ces réglages ne cherchent pas de style à eux. Les captures viennent de la vraie app contre le faux GitHub (`tests/e2e/prs-overview.spec.ts` avec `ESCOUADE_E2E_SHOTS`).

- `esc-68-overview.png` : le groupe, après « Attend ta réponse » et avant les projets. Trois parties, la plus pressante d'abord : « À corriger », « À merger », « Relectures demandées ». Une ligne reprend les colonnes des agents : le signe de sa CI, « PR #12 », son projet, son ticket ou son agent ; son titre sous « Activité » ; ses pastilles (celles de la vue « Pull requests ») à la place du modèle, du coût et du contexte ; son âge sous « Depuis ».
- `esc-68-reglages.png` : Réglages › Notifications, un second groupe « Pull requests et déploiements », cases comme le premier.

Le groupe n'apparaît pas sans PR à traiter : la vue reste celle d'avant. Un clic ouvre l'agent, sinon le Kanban du ticket, sinon la vue « Pull requests » du projet.
