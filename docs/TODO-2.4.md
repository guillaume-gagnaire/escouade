# Escouade 2.4 — à faire

Ce qui est noté pour la version après la 2.3. Rien n'y est lancé avant la sortie de la 2.3.

## Des plugins d'interface, à la façon de VS Code

Demandé par l'utilisateur le 2026-11-24 : des plugins plus génériques, qui viennent modifier l'interface, pour permettre aux gens de faire des choses folles.

Aujourd'hui, un plugin s'intègre à des endroits fixés d'avance : un harness, un hébergeur de code, un gestionnaire de tickets, la voix, un thème. L'interface lui réserve chaque fois sa place.

- [ ] **Les contributions.** Un plugin déclare dans son manifeste ce qu'il ajoute, comme les points de contribution de VS Code : un panneau qui s'ouvre, une vue dans la barre latérale ou le panneau de droite, une commande de la palette, un bouton, un élément de la barre d'état, un menu.
- [ ] **Une étude d'abord.** Elle tranche :
  - les points de contribution offerts et leur manifeste ;
  - le rendu : du HTML dans une webview cloisonnée, ou des composants déclaratifs ;
  - l'API qu'un panneau reçoit : ce qu'il lit (agents, tickets, projet) et les gestes qu'il fait, chacun déclaré dans ses permissions ;
  - la sécurité, et ce que la revue « Vérifié par Escouade » du store doit lire de plus ;
  - ce qu'en voient le téléphone et une fenêtre connectée à un serveur.
- [ ] **Repartir des pages des agents** (ESC-661, 2.3). Une page écrite par un agent et un panneau écrit par un plugin posent le même problème : cloisonner du HTML qui n'est pas digne de confiance et ne lui ouvrir qu'un canal étroit vers l'app.

## Les bots

Demandé par l'utilisateur le 2026-11-26 : des bots, un peu comme OpenClaw ou les bots d'Orca. Chacun a une âme (*soul*) et des tâches. Il est autonome et proactif : il agit de lui-même, sans attendre qu'on lui parle.

- [ ] **Une étude d'abord.** Elle regarde comment OpenClaw et les bots d'Orca font. Puis elle tranche :
  - ce qu'est l'âme d'un bot : son rôle, sa façon d'être, ce qu'il sait et ce qu'il retient d'une fois sur l'autre ;
  - ses tâches : ce qu'il fait de lui-même, quand, et sur quels projets ;
  - comment il vient vers l'utilisateur : la boîte « À toi », les notifications, le téléphone ;
  - ses bornes : le quota, les accords, ce qu'il n'a jamais le droit de faire seul, le coupe-circuit ;
  - sa place à côté de ce qui existe : les tâches programmées, l'Overseer (2.3) et l'Overseer d'équipe.
