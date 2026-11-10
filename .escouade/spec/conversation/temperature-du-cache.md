---
title: "Température du cache"
subject: "conversation"
sources: ["docs/FEATURES.md §3 (conception du 2026-10-28)","design/features/temperature.png","design/features/temperature-echelle.png","design/features/maquettes-temperature.html","tests/fixtures/claude/long-turn.jsonl","tests/fixtures/codex/full-turn.jsonl","tests/fixtures/copilot/full-turn.jsonl"]
tickets: ["ESC-398"]
decisions: [140]
compared: null
---

# Température du cache

Sur chaque agent, quel que soit son harness, un chiffre en degrés dit combien de temps son cache tient encore : un message parti au chaud relit le contexte pour ~5 à 10 % du prix, un message parti à froid le relit au plein tarif.

## Le chiffre

- **100 °C** au dernier tour qui a servi le cache (la dernière requête de l'agent qui l'a lu ou écrit), puis il baisse **en ligne droite** sur la durée de vie du cache, jusqu'à **0 °C** à l'expiration. Arrondi à l'entier supérieur : 0 °C seulement une fois expiré. La ligne droite dit la part de vie qui reste ; le fournisseur jette le cache d'un coup à l'échéance, il ne s'use pas.
- **0 °C** aussi quand le cache est cassé : modèle changé, compte changé, consigne système ajoutée par Escouade modifiée, contexte compacté (compaction de Claude Code, ou `/compact` demandé au harness), nouvelle conversation. Il le reste jusqu'au tour suivant qui lit ou écrit le cache.
- **« — °C »** quand le harness ne dit rien de son cache, ou pas combien de temps il vit. Rien n'est inventé.
- **Couleur du texte** : rouge-orangé de 67 à 100 °C, ambre de 34 à 66 °C, bleu en dessous (0 °C compris), gris pour « — °C ». Les trois teintes (`--heat-hot`, `--heat-mid`, `--heat-cool`) passent le contraste AA dans les deux thèmes ; un thème de plugin peut les donner.
- **Accessible** : le chiffre a un libellé en mots (« Cache : 72 °C, expire dans 3 min 40 », « Cache : 0 °C, cassé (modèle changé) : … », « Cache : — °C, Cursor CLI ne dit rien de la durée de vie de son cache »), en français et en anglais.

## Où

- **Carte de l'agent** (barre latérale) : au bout de la ligne de son modèle, à droite ; son libellé en infobulle.
- **En-tête de la conversation** : une figure « Cache » parmi Contexte, Tokens et Coût, qui part avec elles quand la conversation est trop étroite. Au survol ou au focus (Échap la referme), le détail du dernier tour : le temps qui reste et la durée de vie du cache de ce harness, une barre lus / mis en cache / plein tarif, les tokens lus depuis le cache, mis en cache et nouveaux au plein tarif (lus et mis en cache en une ligne quand le harness ne les dit qu'ensemble), ce que le cache a fait économiser sur ce tour (tokens lus × (prix d'entrée − prix d'une lecture), quand le prix est connu), et ce que coûterait le prochain message s'il partait à froid (le contexte au plein tarif, réécrit en cache pour sa durée chez Anthropic) au lieu du coût au chaud.
- **Barre d'état** : « Cache aujourd'hui : ≈ 3,10 $ économisés », tous agents confondus, depuis minuit (heure locale) ; « Cache ≈ 3,10 $ » sous 1240 px ; absente tant que le cache n'a rien économisé.

## Durée de vie, harness par harness

- **Claude Code** : l'usage de chaque requête de l'agent lui-même (pas d'un sous-agent) : `cache_read_input_tokens`, `cache_creation_input_tokens` et leur part en 5 min et en 1 h (`cache_creation.ephemeral_5m_input_tokens` / `ephemeral_1h_input_tokens`). 1 h quand la requête a écrit pour une heure (ce que fait Claude Code, vu sur ses enregistrements), 5 min quand elle a écrit pour 5 min ou sans le détail ; une requête qui n'a fait que lire garde la durée d'avant. Le tour additionne ses requêtes.
- **Les autres** disent les tokens de leur tour (lus et écrits en un chiffre, `TurnUsage.cache`) ; la durée est celle que le fournisseur du modèle annonce : 5 min chez Anthropic (une écriture d'une heure se demande, seul Claude Code le dit), 5 min chez OpenAI (« 5 à 10 minutes sans activité », la plus courte). Chez OpenAI, le cache ne s'écrit pas : ce qui est en cache a été lu, et l'économie se calcule. Ailleurs (Gemini, Mistral, `auto`…) : « — °C ».
  - Codex : `cachedInputTokens` (ses écritures valent toujours 0).
  - Copilot CLI, OpenCode, harness ACP qui le disent : lus et écrits ensemble ; pas d'économie affichée sur un modèle d'Anthropic.
  - Gemini CLI, Antigravity CLI, Mistral Vibe : « — °C » (durée non annoncée).
  - Cursor CLI : ne dit rien de son cache : « — °C ».
- Un tour sans token en cache ne chauffe rien. Un plugin ancien qui ne passe pas ces chiffres donne « — °C ».

## Ce qui est gardé

- L'état du cache est enregistré avec l'agent (heure de la dernière requête, durée, chiffres du dernier tour, économie du jour) : le cache vit chez le fournisseur, un redémarrage de l'app ne le refroidit pas.
- **Un seul minuteur** pour tous les chiffres : il bat chaque seconde tant qu'un cache est chaud et s'arrête à l'expiration du dernier ; rien ne tourne quand aucun agent n'est chaud.
- Le thermomètre montre, il n'agit pas : Escouade n'envoie jamais de message pour garder un cache chaud (ce serait un tour payé).
- La fonctionnalité « Coût » n'a pas encore d'interrupteur et aucun réglage ne cache le coût : la température s'affiche toujours. Elle partira avec « Coût » quand celle-ci se coupera.
