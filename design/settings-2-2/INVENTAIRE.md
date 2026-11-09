# Réglages 2.2 — inventaire

ESC-375. Chaque réglage de la fenêtre des réglages telle qu'elle est sur `main` (a5dc78f4, 2026-11-07), puis ce que la 2.2 y ajoute. Lu dans le code : `src/components/modals/SettingsModal.svelte`, `src/lib/settings.svelte.ts` (`SETTINGS_TABS`, `SETTINGS_OF`), chaque composant de `src/components/settings/` et les catalogues `src/lib/i18n/fr/`. Les libellés sont ceux de l'app, en français.

Les captures de l'existant, prises dans la vraie app avec l'outillage des e2e, sont dans [`existant/`](existant/) ; les mesures de chaque onglet (hauteur, groupes, rangées, contrôles) dans [`existant/mesures.json`](existant/mesures.json).

## Comment lire les tableaux

- **Fréquence** : `install` (une fois, à l'installation ou à l'arrivée d'un compte), `projet` (une fois par projet, à sa création), `parfois` (quelques fois par an), `souvent` (chaque semaine ou plus), `panne` (jamais, sauf quand quelque chose ne marche pas). C'est une estimation d'expert, tirée de ce que fait le réglage et de l'endroit d'où l'app y renvoie ; aucune mesure d'usage ne la confirme (les statistiques anonymes ne comptent pas les réglages).
- **Qui** : tous ceux qui ont l'app (`tous`), ou seulement ceux qui ont tel harness, telle intégration, telle plateforme.
- **Enreg.** : `brouillon` (pris par « Enregistrer », jeté par « Annuler »), `tout de suite` (appliqué et enregistré au geste, hors du brouillon). Le mélange des deux dans un même onglet est un point du diagnostic.
- **États** : ce que la rangée montre selon le cas (vide, erreur, en cours, un, plusieurs).
- **Renvois** : les endroits de l'app qui ouvrent les réglages sur ce réglage (code, `app.modal = { kind: 'settings', … }`) ou qui le nomment dans un texte (« Réglages › … »).

## La fenêtre

- 980 × 700 au plus, une colonne d'onglets de 224 px, 13 onglets en trois groupes : sans titre (Application, Harness, Comptes, Voix, Notifications), « Par projet » (Projets, Kanban, Intégrations), « Système » (Plugins, Serveur MCP, Terminaux, Réseau, À propos).
- En tête de l'onglet : son nom et une phrase. Pour les trois onglets « Par projet », une rangée de pastilles de projet sous le titre.
- En bas : « Annuler » et « Enregistrer » ; un point sur l'onglet qui a un changement pas encore enregistré ; la raison qui bloque l'enregistrement à gauche (nom de projet vide, commande sans nom, dossier des worktrees invalide).
- `Ctrl+,`, le bouton « Réglages » en bas de la barre des projets, le ⚙ de la barre d'état.

### Ce qui ouvre les réglages, et où (renvois du code)

| D'où | Ouvre | Fichier |
|---|---|---|
| `Ctrl+,`, bouton Réglages de la barre des projets | le dernier onglet | `lib/shortcuts.ts:169`, `ProjectRail.svelte:263` |
| Menu d'une tuile de projet › « Réglages du projet… » | Projets, ce projet | `lib/project-rail.ts:167`, `ProjectRail.svelte:159` |
| Barre latérale (⚙ du projet) | Projets, ce projet | `Sidebar.svelte:630` |
| Section Lancement : ⚙, « Configurer », « ✦ Proposer des commandes » | Projets › Lancement (et la lecture du projet) | `RunsSection.svelte:57,61` |
| Kanban : « ⚙ Après validation » | Kanban, ce projet | `board/Board.svelte:121` |
| Kanban : bandeau des commandes de `.escouade.yml` en attente | Projets, ce projet | `board/Board.svelte:165` |
| Fenêtre d'import d'un ticket, sans source | Intégrations › Sources | `modals/ImportModal.svelte:103` |
| Panneau PR, vue Pull requests (jeton refusé, aucun hébergeur) | Intégrations | `PrPanel.svelte:36`, `prs/PrsView.svelte:141` |
| Panneau Workflow (« Décrire », « Analyser », ⚙, « Relire ») | Projets › Workflow, ce projet | `WorkflowPanel.svelte:160` |
| Accueil (aucun harness trouvé) | Harness | `Welcome.svelte:121` |
| Fenêtre d'un compte (réserver à un projet), retour | Projets, ce projet, brouillon repris | `modals/AccountModal.svelte:440` |
| Fenêtres ouvertes depuis les réglages (compte, mise à jour, fermeture d'un projet), retour | l'onglet d'où elles viennent | `lib/accounts.ts:607`, `modals/UpdateModal.svelte:33`, `settings/ProjectTab.svelte:139`, `settings/ClosedProjects.svelte:36` |

### Ce qui nomme un réglage dans un texte

| Texte (catalogue fr) | Nomme |
|---|---|
| `conv.ts:102`, `nav.ts:60`, `shell.ts:20-24`, `wizard.ts:187` : harness introuvable | Réglages › Harness |
| `wizard.ts:121`, `settings.ts:384` : remote control pour tous | Réglages › Harness |
| `settings.ts:158` : « Le modèle, l'effort et le mode des nouveaux agents se règlent compte par compte, dans Réglages › Comptes » | Réglages › Comptes |
| `wizard.ts:129,183`, `accounts.ts:240,260`, `plugins.ts:389` | Réglages › Comptes, Réglages › Projets |
| `wizard.ts:132`, `prs.ts:252` | Réglages › Intégrations |
| `integrations.ts:94`, `prs.ts:16` : « le plugin est coupé (Réglages › Plugins) » | Réglages › Plugins (qui n'est qu'un lien vers la vue Plugins) |
| `plugins.ts:378` : « Un thème, dans Réglages › Application › Couleurs » | Réglages › Application |
| `share.ts:56,57,66,72` : ton nom pour l'équipe | Réglages › Application, Réglages › Projets |
| `voice.ts:144`, `accounts.ts:211` : modèle de dictée, clé Mistral | Réglages › Voix |
| `shell.ts:59`, `nav.ts:209` : historique d'un projet fermé | Réglages › Projets |
| `shell.ts:111` : statistiques d'usage | Réglages › À propos |
| `workflow.ts:81` : « Réglages › Projets › Workflow, « Lire les workflows » » | Réglages › Projets › Workflow |

Chacun de ces renvois doit survivre à la refonte : la [migration](PROPOSITION.md#migration) en tient la liste.

---

## 1. Application (`app`, `ApplicationTab.svelte`)

Mesuré : 2 écrans de défilement, 4 groupes, 19 contrôles.

| Groupe › réglage | À quoi il sert | Qui · fréquence | Enreg. | États | Renvois |
|---|---|---|---|---|---|
| Affichage › Thème (Système, Clair, Sombre) | thème de la fenêtre | tous · install | tout de suite | — | — |
| Affichage › Couleurs (Escouade ou un thème de plugin) | couleurs d'un thème apporté par un plugin | qui a un thème · parfois | tout de suite | vide (« Aucun thème installé », « Voir les thèmes ») ; thème disparu (plugin coupé) ; valeurs écartées | `plugins.ts:378` |
| Affichage › Zoom de l'interface | taille de toute l'interface | tous · install | tout de suite | trop grand pour la fenêtre (retenu) | raccourcis `Ctrl+=`, `Ctrl+-`, `Ctrl+0` |
| Affichage › Taille du texte des terminaux | — | tous · parfois | brouillon | — | — |
| Affichage › Taille du texte de l'éditeur | — | tous · parfois | brouillon | — | spec *Éditeur* |
| Textes rédigés › Modèle des textes rédigés (un harness) | harness qui nomme les agents, propose commits et PR, remplit les commandes | multi-harness · install | brouillon | absent si aucun harness installé | spec *Relecture d'une PR* |
| Textes rédigés › Modèle (OpenCode seulement) | modèle d'OpenCode pour ces textes | OpenCode · install | brouillon | liste pas encore lue | — |
| Langue › Langue de l'interface | Système, English, Français | tous · install | brouillon | — | — |
| Langue › Langue des textes rédigés par les agents | commits, PR, commentaires des trackers, consignes | tous · install | brouillon | — | — |
| Équipe › Ton nom | nom montré sur un Kanban partagé | qui partage un Kanban · install | brouillon | vide (repris de `git config user.name`) | `share.ts:56-72` |
| Équipe › Ton identifiant | identifiant de l'auteur des tickets partagés | qui partage · jamais | lecture seule | — | — |

## 2. Harness (`claude`, `AppTab.svelte`)

Mesuré, avec les huit harness de la démo : 4,2 écrans, 10 groupes (un par harness, puis « Tous les harness » et « Pièces jointes »).

| Groupe › réglage | À quoi il sert | Qui · fréquence | Enreg. | États | Renvois |
|---|---|---|---|---|---|
| *Harness* (un groupe par harness) › état | Prêt · version, Aucun compte, Trop ancienne, Introuvable, Non prise en charge | tous · panne | — | chacun de ces états ; WSL en NAT (note) | `Welcome.svelte:121`, `conv.ts:102`, `nav.ts:60` |
| *Harness* › Chemin de l'exécutable (Claude Code) | où est `claude` | Claude Code · panne | brouillon | vide = détection ; trouvé ; trouvé dans WSL ; introuvable | `wizard.ts:187` |
| *Harness* › Chemin (les autres) | lecture seule (trouvé, introuvable) | — · panne | — | — | — |
| *Harness* › Modèles (OpenCode) | favoris du menu des modèles, recherche | OpenCode · parfois | brouillon (`favoriteModels`) | aucun modèle lu | — |
| *Harness* › Installation | commande d'installation d'un harness absent | qui n'a pas le harness · install | — | — | — |
| *Harness* › Mettre à jour Claude Code automatiquement | `claude update` au démarrage et chaque jour | Claude Code · jamais | brouillon | en attente, en cours, à jour, mis à jour, échec, coupée, coupée dans Claude Code | — |
| *Harness* › Coût | « Estimé… » (Codex et autres sans coût) | — · jamais | lecture seule | — | — |
| *Harness* › Remote control pour tous les agents | sessions sur claude.ai | Claude Code · parfois | brouillon | — | `settings.ts:384`, `wizard.ts:121` |
| Tous les harness › Vue des nouveaux agents (Escouade, Terminal du harness) | vue par défaut d'un agent | tous · install | brouillon | — | — |
| Tous les harness › Les agents tiennent une liste de tâches | bandeau « Plan » | tous · jamais | brouillon | — | spec *Avancée d'un plan* |
| Tous les harness › Reprise automatique après la limite d'usage | « continue » après le quota | tous · install | brouillon | — | — |
| Tous les harness › Arrêter les processus des agents inactifs après (min) | mémoire de la machine | tous · jamais | brouillon | 0 = jamais | — |
| Tous les harness › Délai avant l'envoi pendant un tour | message modifiable avant l'envoi | tous · parfois | brouillon | Désactivé, N s | — |
| Pièces jointes › Taille maximale, Avertir au-delà de | — | tous · jamais | brouillon | 0 = aucune limite | — |
| Pièces jointes › Joindre le fichier d'origine | copie ou chemin | tous · jamais | brouillon | — | — |
| Pièces jointes › Texte de plus de 256 Ko (chemin, refusé) | — | tous · jamais | brouillon | — | — |
| Pièces jointes › Purger les copies après (jours) | — | tous · jamais | brouillon | 0 = tant que le propriétaire existe | — |

La note du groupe « Tous les harness » renvoie le modèle, l'effort et le mode à Réglages › Comptes.

## 3. Comptes (`accounts`, `AccountsTab.svelte`)

Mesuré avec un seul compte : 2 écrans, 3 groupes, 35 contrôles ; un groupe de plus par harness qui a des comptes.

| Groupe › réglage | À quoi il sert | Qui · fréquence | Enreg. | États | Renvois |
|---|---|---|---|---|---|
| Comptes *harness* › liste des comptes : nom, dossier, exécutable, Actif, ordre (monter, descendre) | sur quel compte partent les agents | tous · install | tout de suite | aucun (« Aucun agent ni ticket ne démarre… ») ; vérification ; connecté · e-mail ; pas connecté ; inactif ; un ; plusieurs ; dernier actif (refus) ; encore utilisé (refus de supprimer) | `settings.ts:158`, `wizard.ts:183`, `plugins.ts:389`, erreurs « Aucun compte X : ajoute-en un dans Réglages › Comptes » |
| › Se connecter…, Se reconnecter…, Changer la clé…, Usage sur *site*, Supprimer | — | tous · install, panne | tout de suite (fenêtre à part) | — | — |
| › Ajouter un compte *harness*… | fenêtre d'ajout (machine ou nouveau) | tous · install | tout de suite | déjà ajouté | Accueil, ESC-373 (installe le plugin) |
| › Nouveaux agents : Modèle, Effort, Mode | défauts par compte | tous · install | tout de suite | « Celui du harness » | `settings.ts:158` |
| › Budget par jour ($) | arrêt du pilote auto à un montant | API, Cursor · parfois | tout de suite | invalide | — |
| › Réservé aux projets | compte réservé à des projets (privés) | projets privés · projet | tout de suite | « Tous les projets » ; projet fermé | `AccountModal.svelte:440`, `wizard.ts:129` |
| Quand un quota est épuisé › Changer de compte à (seuil) | bascule vers le compte suivant | multi-comptes · install | tout de suite | « À la limite » | — |
| › Reprendre sur un autre compte un agent de ticket arrêté par la limite | — | multi-comptes · install | brouillon | — | — |
| › Continuer sur le harness suivant… (passation) | — | multi-harness · install | brouillon | — | — |
| › Ordre des harness | ordre de passation | multi-harness · parfois | brouillon | — | — |
| › Modèle de passation · *harness* | — | multi-harness · jamais | brouillon | — | — |
| Quotas › Projection des quotas (Au-delà de 80 %, Toujours, Jamais) | barre d'état | tous · parfois | tout de suite | — | spec *Barre de statut* |

## 4. Voix (`voice`, `VoiceTab.svelte`, `SpeechSettings.svelte`)

Mesuré : 2,7 écrans, 2 groupes (« Transcription », « Conversation »), 26 contrôles.

| Groupe › réglage | À quoi il sert | Qui · fréquence | Enreg. | États | Renvois |
|---|---|---|---|---|---|
| Transcription › Transcrite par (Local (Whisper), En ligne) | — | qui dicte · install | brouillon | — | — |
| › Service (Deepgram, ElevenLabs, OpenAI, Groq, Mistral), clé, modèle, temps réel | dictée en ligne, payée à l'usage | qui dicte en ligne · install | brouillon ; clé tout de suite (trousseau) | clé absente, enregistrée, empruntée au compte Vibe ; liste lue, de la doc, échec ; modèle disparu | `accounts.ts:211` |
| › Micro utilisé | — | qui dicte · install, panne | brouillon | aucun micro | — |
| › Raccourci (`Ctrl+Maj+Espace`) | — | qui dicte · parfois | brouillon | enregistrement du raccourci | spec *Raccourcis* |
| › Langue parlée | — | qui dicte · install | brouillon | — | — |
| › Modèle (Whisper, une liste) | taille et qualité du modèle local | qui dicte en local · install | brouillon ; téléchargement tout de suite | téléchargé, à télécharger, en cours, échec | `voice.ts:144` |
| Conversation › Voix générée par, service, voix de chaque langue, Écouter | conversation vocale | qui parle · install | brouillon ; voix téléchargées tout de suite | indisponible, téléchargement, échec | — |
| › Vitesse, Volume, Délai avant l'envoi, Si *service* ne répond pas | — | qui parle · parfois | brouillon | vitesse ramenée aux bornes du service | — |

## 5. Notifications (`notifications`, `AppTab.svelte`)

Mesuré : 1,6 écran, 4 groupes, 15 contrôles.

| Groupe › réglage | À quoi il sert | Qui · fréquence | Enreg. | États |
|---|---|---|---|---|
| Me prévenir pour › Questions et autorisations, Tâches terminées, Erreurs, Tickets | types qui sonnent et notifient | tous · install | brouillon | note : l'onglet et la carte signalent toujours |
| Pull requests et déploiements › CI en échec, Changements demandés, PR approuvée, PR mergée, Conflit, Relecture demandée, Déploiement réussi, Déploiement en échec | — | qui a un hébergeur · install | brouillon | — |
| Canaux › Notifications *OS* quand l'app n'est pas au premier plan | — | tous · install | brouillon | — |
| Son › Son activé, Tester le son | — | tous · install | brouillon | — |

## 6. Projets (`projects`, `ProjectTab.svelte` et ses sections), « Par projet »

Mesuré pour un projet simple (un dépôt, aucun workflow lu, aucun projet fermé) : **9 écrans**, 13 groupes, 58 contrôles, le plus long de la fenêtre ; les trois derniers groupes valent pour tous les projets.

| Groupe › réglage | À quoi il sert | Qui · fréquence | Enreg. | États | Renvois |
|---|---|---|---|---|---|
| (en tête) Plugins proposés par ce projet | plugins de son `.escouade.yml` | projet qui en propose · projet | — (lien vers le store) | un, plusieurs ; projet privé ; absent du store | — |
| Identité › Nom du projet | — | tous · projet | brouillon | vide (refus d'enregistrer) | — |
| › Harness par défaut | — | multi-harness · projet | brouillon | marque `.escouade.yml` | — |
| › Compte préféré | — | multi-comptes · projet | brouillon | Automatique, inactif | — |
| › Assistant de configuration (Ouvrir l'assistant…) | rejoue l'assistant | tous · parfois | — | — | `wizard.ts:6,191` |
| › Dossier (Ouvrir) | lecture seule | — · jamais | — | — | — |
| › Couleur, Lettres de la tuile, Logo, Libellé sous la tuile | la tuile de la barre des projets | tous · projet | brouillon | logo trop gros, format | — |
| Remote control › Remote control pour les agents de ce projet | — | Claude Code · parfois | brouillon | allumé et grisé quand « pour tous » l'est | — |
| Confidentialité › Projet privé, Ses fichiers (garde-fou, barrière par harness), comptes réservés ou partagés, Comptes partagés à la limite | isolement du projet pour les autres agents | projets privés · projet | brouillon | public ; privé ; privé sans compte réservé ; compte partagé | `accounts.ts:240,260`, `wizard.ts:129` |
| Dépôts › N dépôts, branche de chacun, Rechercher à nouveau, Oublier | projet à plusieurs dépôts | multi-dépôts · panne | tout de suite | un, plusieurs, retiré | — |
| Workflow › Workflows du projet, lecture, Affiner avec un agent, Relire quand les fichiers changent, étapes | workflows lus dans le dépôt | qui a un workflow · projet | brouillon | aucun, lu, en lecture, échec, supprimés | `WorkflowPanel.svelte:160`, `workflow.ts:81` |
| Git › Un worktree par agent, Créer le worktree au premier message, Branche de départ, Dossier des worktrees, Préfixes des branches (agents, tickets), Fichiers copiés, Commit, Branche des docs | git des agents | tous · projet | brouillon | dossier ou préfixe invalide (refus) ; marques `.escouade.yml` | spec *Git & fichiers*, *Docs* |
| Worktrees › À l'ouverture d'un worktree, Avant sa suppression (listes de commandes), Remplir automatiquement, Worktrees restés › Nettoyer | préparation et démontage | tous · projet, panne | brouillon ; Nettoyer tout de suite | aucune commande ; proposées par Claude ; nettoyage en cours, N supprimés, N gardés | — |
| Lancement › Ouvrir les pages des lancements, commandes de lancement (nom, commande, sous-dossier, shell), Remplir automatiquement | ce que lance « ▶ » | tous · projet | brouillon | aucune ; commande sans nom (refus) ; proposées | `RunsSection.svelte:57,61` |
| .escouade.yml › état du fichier, avertissements, commandes en attente (Faire confiance), Exporter | réglages partagés par le dépôt | équipes · projet, panne | tout de suite (export, confiance) | absent, lu, illisible, avertissements, commandes en attente | `Board.svelte:165` (bandeau) |
| Mémoire partagée › souvenirs, Oublier | carnet des agents | tous · parfois | tout de suite | vide, N souvenirs, illisible | spec *Mémoire partagée* |
| Kanban partagé › Ton nom dans ce projet, Tickets partagés, Partager chaque nouveau ticket | partage du Kanban | équipes · projet | brouillon | — | `share.ts:66,72` |
| Zone sensible › Fermer le projet… | — | tous · parfois | fenêtre de confirmation | — | — |
| *Tous les projets* › Recherche par le sens (modèle de vecteurs) | modèle local de la recherche | tous · install | tout de suite | à télécharger, en cours, téléchargé, échec | spec *Recherche par le sens* |
| *Nouveaux projets* › Nouveaux projets privés | défaut de chaque projet ajouté | prudents · install | brouillon | — | — |
| *Projets fermés* › Rouvrir, Effacer l'historique…, Historique illisible › Effacer | — | qui a fermé un projet · parfois | tout de suite (confirmation) | aucun (groupe absent), un, plusieurs, illisible | `shell.ts:59`, `nav.ts:209` |

## 7. Kanban (`board`, `BoardTab.svelte`), « Par projet »

Mesuré : 2,6 écrans, 4 groupes, 30 contrôles.

| Groupe › réglage | À quoi il sert | Qui · fréquence | Enreg. | États | Renvois |
|---|---|---|---|---|---|
| Quand je valide un ticket « À tester » › Merger (branche cible, stratégie), Ouvrir une PR (brouillon, relecteurs, étiquettes, Répondre sous les fils, Mise à jour de la branche), Pousser, Laisser | la validation | qui a le Kanban · projet | brouillon | — | `Board.svelte:121` |
| Avant et après › Commande de tests, Relancer les tests avant, Supprimer le worktree d'un ticket terminé, Message de commit généré, En cas de conflit | — | Kanban · projet | brouillon | marques `.escouade.yml` | — |
| Pilote auto › Attribuer automatiquement, Pause quand la CI de la cible est rouge, **Pause au-delà du quota** (« pour tous les projets ») | — | Kanban · projet ; la pause vaut pour l'app | brouillon | — | — |
| Agents › En parallèle, Modèle, Effort, Mode | agents des tickets | Kanban · projet | brouillon | — | — |

## 8. Intégrations (`integrations`, `IntegrationsTab.svelte`, `ForgesSection.svelte`), « Par projet »

Mesuré : 2,9 écrans, deux parties titrées (« Hébergeurs de code », « Sources du Kanban ») sur 5 groupes.

| Groupe › réglage | À quoi il sert | Qui · fréquence | Enreg. | États | Renvois |
|---|---|---|---|---|---|
| Hébergeurs de code › Instances connectées : Connecter un hébergeur… (type, adresse, jeton), Vérifier, Déconnecter | PR, CI, déploiements (comptes de l'app) | qui a un hébergeur · install | tout de suite | aucun ; pas encore vérifié ; connecté ; jeton expiré ; droits manquants ; injoignable ; trousseau illisible ; plugin coupé, arrêté ; sans compte pour un projet | `PrPanel.svelte:36`, `PrsView.svelte:141`, `prs.ts:16,252`, `integrations.ts:94`, `wizard.ts:132` |
| Comptes connectés › Jira, Trello, GitHub, Linear, ClickUp, GitLab, Azure Boards : Connecter…, Servi par (plugin), Déconnecter | comptes des trackers (comptes de l'app) | qui importe · install | tout de suite | connecté, non connecté, vérification, trousseau indisponible, servi par un plugin, compte de l'hébergeur | `ImportModal.svelte:103` |
| Sources liées à *projet* › une source par service | d'où viennent les tickets du projet | qui importe · projet | brouillon | « Connecte un compte ci-dessus » | `ImportModal.svelte:103` (section `sources`) |
| Correspondance des statuts › colonne × état, Commenter | — | qui importe · projet | brouillon | absent sans source liée | — |
| Synchronisation › Mettre à jour le statut externe, Publier un résumé à chaque boucle, Extraire les critères | — | qui importe · projet | brouillon (réglages **de l'app**, `integrations`) | — | — |
| Import automatique › Importer les tickets étiquetés, Étiquette, Vérifier toutes les | — | qui importe · projet | brouillon (réglages **de l'app**) | — | — |

## 9. Plugins (`plugins`), « Système »

Un paragraphe et « Ouvrir la vue Plugins » : aucun réglage. Tout est dans la vue Plugins (barre des projets) : installés, store, et sur la page d'un plugin ses onglets Présentation, Permissions, Versions, **Projets** (actif projet par projet, coupé d'office dans un projet privé), **Réglages** (ses réglages et ses secrets, déclarés par son manifeste), Journal ; « N'installer que des versions vérifiées » (`pluginsVerifiedOnly`) dans la vue.

Renvois : `integrations.ts:94` et `prs.ts:16` disent « Réglages › Plugins » pour rallumer un plugin, ce que cet onglet ne fait pas.

## 10. Serveur MCP (`mcp`, `McpSection.svelte`), « Système »

| Réglage | À quoi il sert | Qui · fréquence | Enreg. | États |
|---|---|---|---|---|
| Les agents peuvent piloter Escouade | serveur MCP pour les agents et les harness hors d'Escouade | tous · install | tout de suite | déclaration en cours ; erreur |
| Déclarations dans les harness | une ligne par compte de chaque harness | — · panne | lecture seule | déclaré, à refaire, échec |
| Activité MCP | journal des appels | curieux · parfois | lecture seule | vide, un, plusieurs |

## 11. Terminaux (`terminals`), « Système »

| Réglage | Qui · fréquence | Enreg. | États |
|---|---|---|---|
| Chemins › PowerShell 7, Git Bash (bash sous macOS et Linux) | Windows · panne | brouillon | vide = auto ; détectés : liste ou « aucun » |
| Distribution WSL | WSL · install | brouillon | vide = par défaut |

## 12. Réseau (`network`), « Système »

| Réglage | Qui · fréquence | Enreg. | États |
|---|---|---|---|
| Proxy › Proxy HTTP(S), Exclusions (`NO_PROXY`) | entreprise · install, panne | brouillon | aucun |
| › Appliquer aussi le proxy aux terminaux intégrés | entreprise · install | brouillon | — |
| Certificats › Ignorer la vérification des certificats TLS | proxy qui déchiffre · panne | brouillon | — |

Le texte de l'onglet dit que le proxy sert à « Claude Code, les intégrations et les mises à jour ». Le code le donne aussi au store et aux plugins (`plugins/store.rs`, `plugins/wasm.rs`, `plugins/socket.rs`), aux modèles de la voix (`voice/models.rs`) et aux fichiers envoyés (`sentfiles.rs`) : la phrase est à refaire (diagnostic, point 8).

## 13. À propos (`about`), « Système »

| Réglage | Qui · fréquence | Enreg. | États | Renvois |
|---|---|---|---|---|
| Escouade *version*, Harness détectés | — · panne | — | — | — |
| Rechercher une mise à jour | tous · parfois | — | recherche, aucune, disponible (téléchargement), prête (redémarrer), échec | `UpdateModal.svelte:33` |
| Installer les mises à jour automatiquement | tous · install | brouillon | — | — |
| Statistiques d'usage (En détail) | tous · install | brouillon | jamais demandé (`null`) | `shell.ts:111` |
| Crédits et licences, Site web, Discord, Soutenir Escouade | — · jamais | — | — | — |
| Données locales (ouvrir le dossier) | — · panne | — | — | — |

---

## Ce que la 2.2 ajoute

| D'où | Ce qui s'ajoute | Où ça irait aujourd'hui | Qui · fréquence | États |
|---|---|---|---|---|
| **ESC-374** | « Recevoir les versions bêta », décochée, près de la recherche de mise à jour ; « bêta » à côté de la version | Le ticket dit « Réglages › Application, près de « Vérifier les mises à jour » » : ce bouton est dans **À propos** (« Rechercher une mise à jour »). Preuve du fouillis : l'auteur du ticket ne savait pas où sont les mises à jour. | volontaires · install | cochée (vérification lancée), décochée, sur une bêta |
| **ESC-373** | Les plugins de l'amorce non installés, « Dans l'app, sans téléchargement » ; installés au geste qui les demande (« Plugin Codex installé ») | Vue Plugins ; le geste vient de Réglages › Comptes (ajouter un compte), › Intégrations (source, hébergeur), › Voix (service), d'un projet (`.isola.toml`) | tous · install | non installé, installé au geste, désinstallé par l'utilisateur, refusé |
| **Plugins** : réglages d'un plugin | Réglages et secrets déclarés par un manifeste | Page du plugin › Réglages (vue Plugins) | qui a le plugin · install | aucun, réglé, secret gardé |
| **Plugins** : comptes servis par un plugin | Harness et trackers servis par un plugin (« Servi par le plugin X ») | Réglages › Comptes, › Intégrations | tous · install | plugin coupé, arrêté |
| **Plugins** : hébergeurs | GitHub, GitLab, Bitbucket, Gitea, Azure DevOps en plugins | Réglages › Intégrations › Hébergeurs | qui a un hébergeur · install | plugin coupé, arrêté |
| **Plugins** : voix | Deepgram, ElevenLabs, OpenAI, Groq, Mistral en plugins | Réglages › Voix › Service | qui dicte en ligne · install | plugin absent (s'installe au choix, ESC-373) |
| **Plugins** : thèmes | Couleurs d'un plugin | Réglages › Application › Couleurs | parfois | thème disparu |
| **Plugins** : actif par projet | Un plugin s'active projet par projet | Page du plugin › Projets ; rien côté projet | projet | coupé d'office dans un projet privé |
| **Réseau** | Le proxy sert aussi au store, aux plugins WASM et aux modèles de voix | Réglages › Réseau, texte à refaire | entreprise · install | — |
| **Projets privés** | Comptes réservés, barrière par harness (Vibe, Antigravity, Gemini en 2.2), plugins coupés d'office | Réglages › Projets › Confidentialité ; Réglages › Comptes › Réservé ; page du plugin › Projets | projet | public, privé, privé sans compte réservé |

## Les réglages qui ne sont pas dans la fenêtre

Ils restent où ils sont et la refonte n'y touche pas : largeur de la barre des projets (`railWidth`) et de la barre latérale (`sideWidth`), glissées à la souris ; vue d'un agent (son en-tête) ; remote control d'un agent (clic droit) ; modèle, effort et mode d'un agent (composer) ; `mcpPort` et `nameAsked` (internes) ; `switchAt` est réglé par Comptes (hors brouillon).

## Total

- **Sur main** (mesuré dans la vraie app, [`existant/mesures.json`](existant/mesures.json)) : 13 onglets, 52 groupes, 237 contrôles ; **32 écrans de défilement** en tout, dont 9 pour Projets et 4,2 pour Harness. Un écran est la hauteur visible du corps de la fenêtre (565 px à la taille par défaut).
- **Fréquence** (estimation, d'après les tableaux ci-dessus) : la moitié environ sont `install` ou `projet` (réglés une fois), un tiers `jamais` ou `panne`, une poignée `souvent` (aucun n'est quotidien : les gestes quotidiens sont hors de la fenêtre, dans le composer et le menu d'un agent).
- **Portée** : 3 onglets de projet, mais des réglages d'app dedans (Pause au-delà du quota, Synchronisation et Import automatique des intégrations, comptes des trackers et des hébergeurs, Recherche par le sens, Nouveaux projets privés, Projets fermés).
