# La construction de la fenêtre des réglages (direction A)


**Commun aux lots R1 à R8 de la nouvelle fenêtre des réglages.** L'utilisateur a choisi la **direction A** le 2026-11-07 : une barre des catégories à gauche, les pages d'Escouade puis chaque projet. L'étude fait foi : `design/settings-2-2/PROPOSITION.md`, §2 pour l'organisation, §5 pour le plan, annexe A pour la place de chaque groupe, annexe B pour les renvois. L'inventaire est dans `INVENTAIRE.md`, les aperçus dans `index.html#a-*`, et les captures à atteindre dans `captures/a-*.png`, en clair et en sombre.

Les points du §7 que l'utilisateur n'a pas tranchés suivent la recommandation de l'étude :
- tout s'applique au geste (lot R2b à part, qu'on peut retirer) ;
- la mémoire partagée reste dans la page Équipe ;
- la synchronisation et l'import automatique restent des réglages d'app (page Services connectés).

Règles de tous les lots :
- **Aucune clé enregistrée ne change** : ni `settings.json`, ni les fichiers des projets, ni `.escouade.yml`. Seule la page où s'affiche un réglage change.
- **Le design reste celui de la 2.0** : `DESIGN.md` et les jetons de `src/app.css`.
- Chaque lot laisse l'app utilisable et se fusionne seul.
- Les textes « Réglages › … » de l'app sont refaits dans le lot qui déplace leur cible.
- Le CHANGELOG fr/en reçoit une ligne sous « Non publié » au lot qui change ce que voit l'utilisateur.

Consigne (décidée par l'utilisateur le 2026-10-16) : fais tout le travail toi-même, sans sous-agents, sur Opus en effort medium. Pars de main, et fais `git merge main` avant ton bilan.

Avant ton bilan, lance et lis en entier :
- svelte-check, `npx vitest run src` et `format:check` ;
- clippy `-D warnings` et `cargo fmt --check` si tu touches à Rust ;
- `npx tauri build --debug --no-bundle`, puis les e2e que le §5 nomme pour ton lot.

Mets dans ton bilan des captures de la vraie fenêtre (clair et sombre), à comparer aux `captures/a-*.png`. `npm ci`, jamais `npm install`. Ne pousse rien, ne publie rien.

## Les lots

R1 le registre des pages ; R2 la coque ; R2b tout au geste ; R3 Escouade : Général, Agents, Avancé, À propos ; R4 Comptes et harness ; R5 Services connectés et Tickets importés ; R6 les pages d’un projet ; R7 Plugins ; R8 recherche et points ambre. Le détail de chacun est au §5 de PROPOSITION.md ; R2b est décrit dans son ticket.
