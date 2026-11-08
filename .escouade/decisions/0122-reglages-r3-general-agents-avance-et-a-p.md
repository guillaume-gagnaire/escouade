---
number: 122
id: "reglages-r3-general-agents-avance-et-a-p"
title: "Réglages R3 : Général, Agents, Avancé et À propos ont leur contenu propre"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-381"
pages: ["fenetre/reglages","docs/recherche-par-le-sens"]
tickets: ["ESC-381"]
documents: ["design/settings-2-2/PROPOSITION.md"]
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages","added":29,"removed":23},{"page":"docs/recherche-par-le-sens","added":1,"removed":1}]
accepted_at: "2026-11-08T05:59:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Jusqu'à R2b, les pages d'Escouade de la fenêtre des réglages montraient encore les onglets de la 2.0 (Application, Harness, Serveur MCP, Réseau, Terminaux, À propos), tous dessinés par AppTab.svelte. La direction A (PROPOSITION.md §2 et annexe A) range chaque réglage par la question qu'on se pose.

## Décision

Général : Apparence (thème, couleurs, zoom, terminaux), Langue, Mises à jour (version et « Rechercher une mise à jour », automatiques, bêta, mise à jour trouvée), Toi (ton nom, statistiques d'usage) ; Avancé : taille du texte de l'éditeur, ton identifiant. Agents : Nouveaux agents (vue, liste de tâches, délai avant l'envoi), Textes rédigés, Remote control pour tous les agents, Piloter Escouade (serveur MCP) ; Avancé : arrêt des process inactifs, Activité MCP. Avancé : Réseau (proxy et certificats), Terminaux, Pièces jointes, Recherche par le sens (« Modèle local »), Données locales. À propos : logo, version, harness détectés, crédits, site, Discord, soutenir. Les harness et la reprise automatique restent dans Comptes et harness jusqu'à R4. Aucune clé enregistrée ne change ; AppTab.svelte est découpé en une page par fichier puis supprimé.

## Conséquences

« Plus tard » de la fenêtre d'une mise à jour revient sur Général. Les textes « Réglages › Application », « Réglages › À propos » (statistiques) et « Réglages › Harness » (remote control) disent désormais Général et Agents. Les pages Conversation (pièces jointes) et Avancée d'un plan (liste de tâches) de la spec nomment encore l'ancien onglet Harness : à reprendre avec la réécriture de fenetre/reglages prévue après R8.
