---
title: "Commandes interactives"
subject: "conversation"
sources: ["src/lib/interactive.ts","essai réel de Claude Code 2.1.319 en stream-json (2026-11-09)"]
tickets: ["ESC-401"]
decisions: [137]
compared: null
---

## Ce que c'est

Les commandes d'un harness qui ouvrent un écran à elles dans son interface (`/login`, `/permissions`, `/hooks`…). Le process qu'Escouade pilote ne les exécute pas : Claude Code 2.1.319 répond « /login isn't available in this environment. » en stream-json, et `initialize` ne les liste pas. Tapée dans le champ d'un agent ou choisie dans le menu « / », une telle commande ne part jamais comme un message : elle ouvre ce qu'Escouade a pour elle, sinon la vue du harness, où elle est tapée.

## Comportement

### Claude Code

| Commande | Dans Escouade |
|---|---|
| `/login` | La fenêtre « Se connecter » du compte de l'agent (le geste de Réglages › Comptes et harness : le programme du harness dans un terminal, avec le dossier du compte). Une fois connecté, la ligne « Compte <compte> connecté : l’agent reprend sur ce compte, avec sa session. » ; son process, s'il est inactif, s'arrête (il garde l'ancienne connexion en mémoire) et le message suivant en démarre un sur sa session ; un tour qui attendait une connexion reprend aussitôt (« continue »). Fermée, la fenêtre ramène à la conversation. |
| `/logout` | Une confirmation : « Déconnecter le compte <compte> ? », qui dit que le harness se déconnecte pour tous les agents de ce compte et que le compte reste dans Escouade. Puis `claude auth logout` avec le dossier du compte, sans fenêtre ; le process inactif de l'agent s'arrête ; la ligne « Compte <compte> déconnecté. /login le reconnecte. ». Un refus du harness est dit dans un toast. |
| `/model`, `/effort` seuls | Le menu du champ. Avec un argument (`/model opus`), la commande part au process, qui l'exécute. |
| `/plan [texte]` | Le mode Plan ; le texte, s'il y en a, part ensuite seul. |
| `/usage`, `/extra-usage`, `/usage-credits` | Réglages › Comptes et harness (les quotas). |
| `/remote-control` | Le remote control de l'agent allumé ; déjà allumé, un toast le dit. |
| `/config`, `/mcp` seuls, et `/permissions`, `/hooks`, `/plugin`, `/skills`, `/memory`, `/status`, `/help`, `/ide`, `/vim`, `/theme`, `/terminal-setup`, `/statusline`, `/keybindings`, `/privacy-settings`, `/install-github-app`, `/feedback`, `/bug`, `/release-notes`, `/upgrade`, `/add-dir`, `/export`, `/rewind`, `/tasks`, `/bashes`, `/copy`, `/btw`, `/sandbox`, `/advisor`, `/import`, `/voice`, `/ultrareview`, `/web-setup`, `/workflows`, `/background`, `/schedule`, `/mobile`, `/desktop`, `/chrome`, `/powerup`, `/passes`, `/tui`, `/focus` | L'agent passe dans la vue Claude Code ; une fois son interface prête (essayé toutes les 500 ms, 20 s au plus), la commande y est tapée telle quelle, puis Entrée. Un toast : « <commande> s’ouvre dans la vue Claude Code. « Vue Escouade » ramène à la conversation, qui continue. ». Un agent de ticket (sans vue Claude Code) garde la commande dans son champ, avec « <commande> ne marche que dans la vue Claude Code, qu’un agent de ticket n’a pas. » ; un changement de vue refusé (tour en cours) aussi, avec la raison. |
| `/resume`, `/branch`, `/fork`, `/teleport` | Rien : « <commande> n’est pas proposé ici : l’agent garde sa session. Un nouvel agent en ouvre une autre. » |
| `/exit` | Rien : « <commande> n’est pas proposé ici : rien à quitter, l’agent reste dans Escouade. » |

Les autres commandes partent comme avant : celles que le process exécute (`/clear`, `/compact`, `/context`, `/cost`, `/stats`, `/color`, `/rename`, `/output-style`, `/autocompact`, `/goal`, `/reload-plugins`, `/agents`, `/config clé=valeur`, `/mcp <action>`), les skills et les commandes de l'utilisateur.

### Les autres harness

- `/login` et `/logout` : comme pour Claude Code, avec le geste de leur compte (`codex login`, `copilot login`…). `/logout` passe par `codex logout` pour Codex ; les autres harness n'ont pas de commande de déconnexion qu'Escouade puisse lancer sans rien demander (Copilot CLI n'en a pas, celle d'OpenCode demande un fournisseur) : un toast le dit, « <harness> ne se déconnecte pas depuis Escouade : fais-le dans son terminal. ».
- `/model` et `/effort` seuls : le menu du champ (effort seulement si le modèle en prend).
- Leurs autres commandes partent comme avant : elles n'ont pas été essayées une à une.

### Menu « / »

- Le menu propose, avec les commandes que le harness liste, ses commandes interactives, même pour un harness qui n'en liste aucune. Chacune porte, à droite, une marque discrète qui dit où elle s'ouvre : « s’ouvre dans Comptes », « s’ouvre dans le menu », « passe en mode Plan », « s’ouvre dans Réglages », « Remote Control d’Escouade », « s’ouvre dans la vue Claude Code », « pas depuis Escouade ». Celles que le harness ne décrit pas en ont une description d'Escouade (`/login` : « Connecter le compte de l’agent »…) ou aucune.
- Choisie seule dans le menu (Entrée, Tab ou clic), une commande interactive s'ouvre aussitôt, sauf `/plan`, qui attend son texte.

## Limites

- La déconnexion ne passe que par Claude Code et Codex.
- La liste suit Claude Code 2.1.319 ; une commande qu'une version suivante ajoute part comme un message tant que la liste ne la connaît pas.
