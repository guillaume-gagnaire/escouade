---
number: 10
title: "Les réponses et l'économie de l'index du code, comme tokensave"
date: "2026-10-30"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-298"
pages: ["docs/index-du-code"]
tickets: ["ESC-298"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"docs/index-du-code","added":28,"removed":9}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Les sondes d'ESC-294 ont montré que l'index était moins juste que grep et que son économie annoncée (96 %) cachait un coût réel 2,5 fois plus élevé. L'utilisateur a demandé le 2026-10-29 de faire « exactement comme tokensave » (commit 580936b, MIT), puis de refaire les sondes. IDX-1 et IDX-2 ont porté la recherche, le contexte et les liens ; restaient les réponses, le compte de l'économie et les consignes des outils. Le 2026-10-30, l'utilisateur a ajouté qu'il fallait mesurer si la réponse donne à l'agent le code dont il a besoin, pour qu'il n'ait pas à relire puis re-grep.

## Décision

Les outils de l'index répondent comme tokensave : du Markdown pour index_context (5 blocs de code de 1 500 octets au plus, un bloc coupé renvoyant à index_body node_id=…, seen_node_ids pour exclude_node_ids, filtres path/path_include/path_exclude), une ligne par symbole pour index_search (ids sur demande, mode literal pour un texte exact), le corps en texte pour le nouvel outil index_body, du JSON pour les appelants, appelés et l'impact ; .escouade/queryignore écarte des chemins des recherches. L'économie est comptée comme tokensave : before = fichiers touchés ÷ 4 plafonnés à 4 fois la réponse (à la réponse pour une recherche littérale), after = réponse + requête + schémas des outils une fois par session + ligne de mesure, déficit reporté sur les appels suivants ; les Stats suivent ce qui est crédité. Les descriptions des outils sont fixes et demandent les mots du code et keywords. Écarts gardés : le code des points d'entrée est donné par défaut (include_code vrai, faux chez tokensave), la ligne de mesure est toujours envoyée, la session est celle de l'appelant, et le hook ne refuse toujours rien (tokensave bloque grep et les agents Explore : pas repris sans décision de l'utilisateur).

## Conséquences

L'économie affichée baisse et devient honnête ; une réponse comptée avant garde son ancien chiffre dans les Stats (colonne saved_tokens ajoutée). Les agents reçoivent moins de JSON et plus de code lisible. Le banc des sondes compte la réponse telle que le serveur l'envoie et mesure la suffisance des 17 sondes « tâche » (plages écrites à la main, docs/codeindex-probes-sufficiency.json), rejouée sur la référence d'ESC-294, IDX-1, IDX-2 et IDX-3. La licence MIT de tokensave et les fichiers portés sont dans THIRD-PARTY-NOTICES.md.
