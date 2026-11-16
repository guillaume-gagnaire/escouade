---
number: 190
id: "la-relecture-de-l-agent-en-fils-sous-leu"
title: "La relecture de l'agent en fils sous leur ligne : écarter résout, publier envoie une remarque sous le compte de l'utilisateur"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-490"
pages: ["git-fichiers/relecture-par-un-agent","git-fichiers/pull-requests"]
tickets: ["ESC-490"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/relecture-par-un-agent","added":38,"removed":27},{"page":"git-fichiers/pull-requests","added":20,"removed":12}]
accepted_at: "2026-11-16T17:56:06+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Depuis ESC-66, la relecture d'une PR par un agent est une liste à part (PrReviewView) : chaque commentaire se garde, se corrige ou s'écarte, puis tout part d'un coup comme une review avec un verdict. Le chantier des PR de la 2.3 (design/pr-2-3, direction A) veut que l'agent commente comme un humain, à côté du code, dans l'onglet Fichiers de la vue de la PR (PR-4 y a mis les fils de l'hébergeur et l'écriture par pr_write). Les « Questions à trancher » du design supposent que les remarques restent locales tant que l'utilisateur ne les publie pas.

## Décision

- Les remarques de l'agent relecteur sont des fils de l'agent sous leur ligne (ou leur plage, leur fichier, la PR) dans l'onglet Fichiers, marqués « agent, pas publié », gardés dans Escouade ; une remarque dont la ligne n'est plus dans le diff ou dont le code a changé va à part, comme un fil périmé.
- Chaque remarque a une sévérité : Sécurité, Bogue, Spec, Détail. « Bogue » s'ajoute aux trois du design, un bogue n'étant ni un écart de spec ni un détail. Comptées par sévérité dans l'en-tête de l'onglet, filtrables, triables dans le résumé.
- Une remarque peut porter une suggestion de code (le texte qui remplace ses lignes), montrée en diff, copiée, jamais exécutée ; l'appliquer est PR-6.
- Le résumé (commit lu, remarques par sévérité, avis d'ensemble, lien vers chaque fil) est en tête de la Conversation.
- Garder/écarter devient résoudre/publier : « Écarter » résout le fil de l'agent ici (replié, gardé, « Reprendre ») ; « Publier » envoie la remarque, son texte relu et corrigé, comme un commentaire de l'utilisateur par pr_write (sur sa ligne quand elle y est, sinon dans la conversation, sa suggestion en bloc suggestion de GitHub), une à une ou « Publier les n remarques gardées » après une confirmation qui les liste. Publiée, elle laisse la place au fil de l'hébergeur ; une remarque publiée ou en train de partir n'est jamais envoyée deux fois.
- La publication d'une relecture entière avec verdict (pr_review_publish, pr_review_summary) et la vue PrReviewView disparaissent : approuver ou demander des changements vient avec « Ta relecture » (PR-7). « Voir la relecture » ouvre la vue de la PR sur Fichiers.
- Relancer après un commit garde les remarques d'avant : celles qui tiennent suivent leur code, celles dont le code a changé sont périmées, ce qui a été écarté ou publié le reste, les nouvelles s'ajoutent sans doublon (une remarque écartée ou publiée ne revient que si son code a changé).
- Rien n'est publié par l'agent, le serveur MCP ou tout seul : l'agent relecteur reste sans outil, seule la fenêtre (commande pr_review_send, classée « served » de l'API distante comme pr_write) publie ; de la réponse de l'agent, seules ses remarques et son avis sont lus, toutes ouvertes.

## Conséquences

- Un tri par commentaire remplace la publication groupée : chaque remarque publiée est un commentaire séparé chez l'hébergeur (plus de review unique avec verdict jusqu'à PR-7).
- Les relectures gardées d'avant se reprennent : une relecture publiée en entier par ESC-66 revient « prête », ses commentaires gardés marqués publiés.
- Ailleurs qu'à GitHub, une remarque part dans la conversation tant que les plugins ne déclarent pas le commentaire de ligne.
- Tests : Rust (modèle, fusion après un commit, publication par pr_write sans doublon, PR piégée, aucun chemin MCP), Vitest (fils, sévérités, suggestion, résumé, écarter, publier une et plusieurs, relance), e2e prs-review sur le faux GitHub et le faux claude aux deux tailles.
