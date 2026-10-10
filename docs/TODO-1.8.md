# Escouade 1.8 — à faire

Notes prises pendant la 1.7, à détailler avant de lancer le chantier.

## Avancée d'un agent qui exécute un plan

But : quand un agent déroule un plan (liste de tâches, sous-agents), Escouade montre où il en est, sans avoir à lire sa conversation.

- [ ] Les agents peuvent remonter les détails de leur avancée pendant l'exécution d'un plan : la liste des tâches (faites, en cours, à venir), le nombre de sous-agents lancés et en cours, ce qui se passe en ce moment.
- [ ] Design de référence : https://claude.ai/design/p/9047fbff-8fa3-4f27-bf04-68c6672302d1?file=Claude+Code+Manager.dc.html (à relire avec DesignSync au début du chantier : le design en ligne évolue plus vite que `design/Claude Code Manager.dc.html`).
- [ ] Pistes à évaluer au début du chantier : ce que Claude Code donne déjà dans le flux stream-json (outil TodoWrite et ses listes, sous-agents via l'outil Task / Agent et leurs `parent_tool_use_id`), et ce qui demanderait un outil du serveur MCP d'Escouade (la 1.7 a `report_progress`, une seule ligne d'état).
