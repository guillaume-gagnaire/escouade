# Reprendre une session copiée sur un autre poste (TS-0c, ESC-581)

L'essai qui décide du choix C16 (`docs/TEAM-SERVER.md` § 11 et § 15) : quand un agent passe à un collègue, peut-on copier la session du harness et la reprendre là-bas, chemins réécrits (C16-B), plutôt que de seulement relancer une session sur le fichier de passation d'ESC-88 (C16-A) ?

Essai fait le 2026-11-21 sous Windows 11, avec `claude` 2.1.330 et `codex-cli` 0.144.5 installés et déjà connectés. Le produit ne change pas. Les scripts de l'essai (`rewrite.js`, `paths.js`) sont restés dans le dossier temporaire de la session et ne sont pas versionnés.

## En bref

- **Claude Code : ça marche.** Une session créée dans un dossier A, copiée seule dans le dossier encodé d'un dossier B, avec ses chemins réécrits (26 remplacements), se reprend par `--resume` depuis B, alors que A et sa session d'origine ont été effacés. Le contexte est gardé, les outils marchent dans B, le `cwd` vaut B, et rien de A ne reste.
- **Codex : à moitié essayé.** Le compte refuse tous les modèles essayés. Mais `codex exec resume` retrouve une session copiée, chemins réécrits, **sous un nouvel identifiant que sa base ne connaît pas**, et il lance un tour dans B (`turn_context.cwd` = B). Ce qui manque : la réponse du modèle.
- **Recommandation : C16-B pour Claude Code**, sous garde : la passation d'ESC-88 reste là en repli, la version de `claude` doit être la même ou plus récente chez qui reprend, et la copie part chiffrée comme le journal. Codex reste en C16-A. Coût : un ticket, environ 2 h d'agent, plus un essai à deux comptes qui attend l'utilisateur.

## 1. Claude Code

### Ce qui a été fait

1. Deux dossiers jetables, `essai-a` et `essai-b`, dans le dossier temporaire de la session. `essai-a/note.txt` contient `MOT: girafe` et `essai-b/note.txt` contient `MOT: zebre`, pour voir quel fichier chaque tour lit.
2. Dans A, un seul message, sur Haiku, avec `--safe-mode` (sans les hooks, MCP, CLAUDE.md ni plugins de l'utilisateur, mais avec la connexion au compte) :
   `claude -p --safe-mode --model haiku --output-format json --allowedTools Read -- "Lis note.txt avec l'outil Read et réponds seulement par le mot qu'il contient."`
   Réponse : `girafe`, session `1e97c2d5-…`, 0,0023 $.
3. La session est le seul fichier `~/.claude/projects/C--Users-…-scratchpad-essai-a/1e97c2d5-….jsonl` (313 540 octets, 43 lignes). Le dossier encodé remplace par `-` chaque caractère qui n'est ni lettre ni chiffre (`C:\Users\…\essai-a` donne `C--Users-…-essai-a`).
4. Copie de ce seul fichier, puis réécriture de chaque chaîne de chaque ligne JSON : le dossier A, sous ses deux formes (`C:\…\essai-a` et `C:/…/essai-a`), devient B. 26 remplacements, aucun reste de `essai-a`. Le fichier va dans `~/.claude/projects/C--Users-…-scratchpad-essai-b/`, sous le même identifiant.
5. Comme sur un autre poste, la session d'origine est effacée et A est renommé, pour qu'aucun tour ne puisse s'appuyer dessus.
6. Depuis B, un seul message :
   `claude -p --safe-mode --model haiku --output-format json --allowedTools Read --resume 1e97c2d5-… -- "Réponds en 3 lignes. 1) Sans outil : le mot lu au tour d'avant et le chemin absolu du fichier lu alors. 2) Relis note.txt avec Read : le mot et le chemin absolu lus maintenant. 3) Ton dossier de travail."`

### La réponse, au mot près

```
1) Sans outil, le mot lu au tour précédent était « girafe », dans C:\Users\guill\AppData\Local\Temp\claude\9332eb4730787f3f\9e15ec4e-7b54-42eb-b34f-0f2b8ccac7f1\scratchpad\essai-b\note.txt.
2) Relecture avec Read : le mot est maintenant « zebre », dans le même chemin absolu, C:\Users\guill\AppData\Local\Temp\claude\9332eb4730787f3f\9e15ec4e-7b54-42eb-b34f-0f2b8ccac7f1\scratchpad\essai-b\note.txt.
3) Mon dossier de travail est C:\Users\guill\AppData\Local\Temp\claude\9332eb4730787f3f\9e15ec4e-7b54-42eb-b34f-0f2b8ccac7f1\scratchpad\essai-b.
```

Sortie `is_error: false`, `num_turns: 2`, même `session_id`, 0,0046 $. La reprise a ajouté ses 24 lignes **au fichier copié** (67 lignes), toutes avec `cwd` = B et `version` 2.1.330, et sans aucune mention de A.

### Ce qui marche

- **Le contexte** : le tour d'avant est connu (« girafe »).
- **Les outils** : `Read` sur un chemin relatif lit le fichier de B.
- **Le `cwd`** : B. Une ligne `environment` nouvelle redonne le dossier de travail au modèle.
- **Le dossier temporaire de la session** (`scratchpadDirectory`) : il se calcule à partir du dossier de travail (`Temp\claude\<empreinte>\<session>`). Il n'a pas à être réécrit : la reprise en tire un nouveau.
- **La recherche de la session** : `--resume <id>` la trouve dans le dossier encodé du `cwd`, même quand la session n'a jamais existé à cet endroit.

### Ce qui casse, ou casserait

- **Les fichiers cités disent faux.** La réécriture change aussi l'histoire : le premier tour dit maintenant avoir lu `girafe` dans `essai-b\note.txt`, qui contient `zebre`. Le modèle l'a dit sans s'en étonner. En vrai, la branche poussée a les mêmes fichiers. Mais le travail non commité laissé en patch, ou tout ce qui vit hors du dépôt, fait mentir l'histoire de la même façon.
- **Sans réécriture, l'ancien chemin part au modèle.** Le premier message envoyé à l'API (`api-request-blob`) contient le bloc d'environnement (« Primary working directory: …essai-a »). L'aide de `claude` dit qu'à la reprise, le prompt enregistré « est renvoyé tel quel jusqu'à la compaction ». Une copie non réécrite enverrait donc A au modèle, en plus de ne pas être trouvée depuis B. Ça se lit dans le format ; ça n'a pas été essayé, pour s'en tenir à un message par essai.
- **Les chemins hors du projet ne sont pas réécrits.** Le dossier temporaire de la session (`85c8…`) et tout chemin de l'utilisateur (`C:\Users\guill\…` d'un `--add-dir`, d'un fichier lu hors du dépôt) passent tels quels. Ils ne servent à rien sur l'autre poste, et ils disent où le collègue range ses fichiers.

### Où sont les chemins dans le fichier

Pour une session d'un message et d'un appel d'outil :

| Ligne | Champ |
| --- | --- |
| chaque `user`, `assistant`, `attachment` | `cwd` |
| `attachment` `environment` | `snapshot.workingDirectory`, `snapshot.additionalWorkingDirectories` (avec des `/`), `rendered[].content` (le texte envoyé au modèle) |
| `api-request-blob` | `message.content[].text` (le premier message envoyé, environnement compris) |
| `assistant` (appel d'outil) | `message.content[].input.file_path`, `wireToolInputs.<id>.file_path` |
| `user` (résultat d'outil) | `toolUseResult.file.filePath` |

Il faut réécrire chaque chaîne d'un objet JSON lu, et pas le texte brut du fichier : dans le JSON, le `\` de Windows est doublé, et un remplacement brut le manque. Il faut aussi couvrir les deux séparateurs.

### La version de `claude`

Chaque ligne porte `version` (2.1.330 ici). La reprise a été faite avec la même version ; une autre version n'a pas été essayée. Ce que dit le format :

- La session contient des lignes que seule une version récente écrit (`api-request-shape`, `api-request-blob`, `prompt_snapshot`, `data-flow-records`, `atis-latch`). Une version plus ancienne ne les connaît pas, au mieux les ignore et reconstruit son prompt. Le résultat ne se prévoit pas.
- La règle sûre est donc : **la même version, ou une plus récente, chez qui reprend**. Sinon, la passation d'ESC-88.

### Ce qu'un autre compte changerait (par la lecture du format, sans essai)

- La session dit à qui elle est : des lignes `session-owner` (`accountUuid`, `organizationUuid`), répétées à chaque tour, et une pièce jointe `credential_org`. Le binaire range `session-owner` en « last-wins » : à la reprise sur un autre compte, une nouvelle ligne `session-owner` s'ajoute. Le binaire parle aussi de `hybrid-organization`, la marque d'une session passée par plusieurs organisations. Ce qu'il en fait (refus, avertissement, rien) ne se lit pas sans essai.
- **Le cache des prompts** est propre au compte : le premier tour repris sur un autre compte paie toute la conversation en écriture de cache. Ici, le cache était chaud (70 588 jetons lus).
- **Les blocs de réflexion signés** (un modèle qui réfléchit ; Haiku ne l'a pas fait ici) sont renvoyés à l'API avec leur signature. Qu'une autre organisation les accepte n'a pas été essayé.

## 2. Codex

### Ce qui a été fait

- Mêmes dossiers jetables (`codex-a`, `codex-b`) et même message, par `codex exec --json --skip-git-repo-check -s read-only`.
- Le modèle par défaut est refusé (« The 'gpt-5.6-sol' model is not supported when using Codex with a ChatGPT account »), puis `gpt-5.4-mini` et `gpt-5.5` de même, avec chaque fois « Model metadata for `…` not found ». Je n'ai pas lu la configuration de l'utilisateur pour en savoir plus, et je me suis arrêté là.
- Chaque essai a quand même écrit sa session, `~/.codex/sessions/2026/11/21/rollout-…-<id>.jsonl` (9 lignes). Codex range ses sessions **par date, et pas par projet** : le chemin du projet n'est que dans le contenu.
- Les chemins y sont dans `session_meta.payload.cwd`, `turn_context.payload.cwd` et `workspace_roots`, `world_state.payload.state.environments` (`local.cwd`, et le texte `filesystem`), et le texte du message d'environnement (`<cwd>…</cwd>`). 7 remplacements.
- La copie réécrite a pris **un nouvel identifiant** (dans `session_meta.id` et `session_id`, et dans le nom du fichier), pour qu'elle ne soit pas connue de la base d'état de Codex, comme sur un autre poste. Les sessions d'origine et A ont été effacés.
- Depuis B : `codex exec resume --json --skip-git-repo-check -m gpt-5.5 01a1fd96-…-48ec77394800 "Quel mot as-tu lu ?"`.

### Ce qui en sort

- `{"type":"thread.started","thread_id":"01a1fd96-dc35-7590-b3e7-48ec77394800"}` : la copie est retrouvée par son seul fichier. Pour comparer, un identifiant inconnu donne `Error: thread/resume: thread/resume failed: no rollout found for thread id 01a1fd96-0000-7000-8000-000000000000 (code -32600)`.
- Le tour a démarré avec `turn_context.cwd` = B, puis l'API a refusé le modèle, comme avant. Rien de A n'a été ajouté.

### Ce qu'il faudrait pour finir l'essai

Un modèle que le compte ChatGPT accepte avec ce `codex`. Le refus de trois modèles, avec « Model metadata … not found » à chaque fois, fait penser que le `codex` 0.144.5 installé est trop ancien pour la liste de modèles du compte. Il faudrait donc un `codex` à jour, ou un compte de Codex avec une clé d'API, dans un `CODEX_HOME` jetable : celui-ci ne lirait rien de l'utilisateur. Puis refaire les deux messages ci-dessus. Environ 15 minutes d'agent.

Ce qui resterait à suivre pour Codex : sa base d'état (la liste des sessions par `cwd` de `resume --last`) ne connaît pas la copie. `resume <id>` passe outre ; `--last` ne la montrerait sans doute pas. Le format de Codex change aussi souvent (`world_state`, `thread_settings_applied` sont récents).

## 3. Les risques

- **Un chemin oublié.** Une réécriture de texte brut, ou d'un seul séparateur, en laisse passer (le `\` doublé du JSON, les `/` de `additionalWorkingDirectories`). Un chemin oublié se voit mal : la reprise marche, le modèle lit seulement un mauvais dossier ou le cite. Il faut réécrire par chaîne d'objet JSON, puis vérifier qu'il ne reste aucune occurrence de l'ancien dossier (et refuser la reprise native s'il en reste). Les chemins hors du dépôt (`--add-dir`, dossiers de l'utilisateur) ne se réécrivent pas : ils restent ou sont vidés.
- **Un secret dans la session.** La session garde mot pour mot ce que les outils ont lu : `toolUseResult.file.content` et `message.content` contiennent ici `MOT: girafe`. Un `.env` lu, la sortie d'une commande `env` ou un jeton collé partent avec elle. Elle contient aussi les identifiants du compte et de l'organisation (`session-owner`, `credential_org`), l'identifiant de la session parente (`derived-from`), le prompt système complet (`api-request-shape`, `prompt_snapshot`, 236 Ko à eux deux), avec la liste des outils et des MCP. Dans une session normale (sans `--safe-mode`), s'y ajoutent le CLAUDE.md et la mémoire de qui passe la main. La session de Codex, elle, contient la liste des skills de l'utilisateur avec leurs chemins sous `~/.codex/skills`. Aucun jeton de connexion n'y était (aucune occurrence de `sk-ant`, `accessToken`, `refreshToken`, `Bearer`, `access_token`). C'est le même risque que le journal d'Escouade (`conv.rs`), qui passe déjà, chiffré, en C16-A, mais en plus large : le prompt et les comptes en plus.
- **Une session d'un projet privé.** Elle contient les fichiers lus, donc le code du projet privé, et le prompt de qui passe la main. Le passage à un membre de l'équipe du projet ne fait pas sortir ces données du projet. Mais la copie ne doit aller que dans le dossier du compte de qui reprend, chiffrée de bout en bout comme le journal, et être effacée du serveur une fois reprise.
- **Deux reprises en même temps.** Escouade l'empêche par la prise, qui ne passe que par le serveur : une seule personne tient le ticket. Mais la session d'origine reste chez qui passe la main, sous le même identifiant, et `claude --resume` peut encore la rouvrir à la main : deux histoires divergent sous un même identifiant. Sur un même poste (un agent passé à la machine de l'équipe puis repris), les deux fichiers peuvent même tomber dans le même dossier, si les chemins coïncident. Remède : reprendre chez le collègue par `--resume <id> --fork-session`, qui donne un nouvel identifiant à la reprise. Cette option n'a pas été essayée ici, l'essai suivant doit la vérifier.
- **Ce que l'essai a laissé hors de ses fichiers.** `claude` et `codex` écrivent eux-mêmes leur état à chaque lancement : `~/.claude.json` garde sans doute une entrée de projet pour `essai-a` et `essai-b`, et la base d'état de Codex les fils des trois essais. Je ne les ai ni lus ni modifiés (c'est interdit). Ces restes ne gênent rien.

## 4. La recommandation pour C16-B

**Tranché par l'utilisateur le 2026-11-21** (demande a-23, « C16 : a ») : la recommandation ci-dessous. Le ticket est écrit : TS-17b = ESC-607 ; l'essai à deux comptes est dans sa boîte « À toi » (a-24).

**Le faire pour Claude Code, avec la passation d'ESC-88 en repli ; s'en tenir à C16-A pour Codex et les autres harness.**

Pourquoi : la reprise d'une copie réécrite a marché du premier coup, sans rien de A, avec le contexte, les outils et le `cwd` justes. Le format se réécrit sans rien deviner (des chaînes JSON), et l'agent qui reprend garde tout son contexte, au lieu d'un résumé. Ce qui reste incertain (une autre version, un autre compte) se garde par des contrôles et un repli, plutôt qu'en renonçant.

### Le ticket à écrire

« Serveur d'équipe (TS-17b) : la reprise native d'une session de Claude Code passée à un collègue (C16-B), passation d'ESC-88 en repli »

- **Chez qui passe la main** : la session de l'agent (son `transcript_path`, déjà connu par les hooks, `src-tauri/src/agent/transcript.rs`) part avec le dépôt chiffré du passage (`crates/escouade-team/src/handoff.rs`), avec le chemin du worktree et la `version` de `claude`. Seulement quand le harness est Claude Code et que la session est entre deux tours.
- **Chez qui reprend** (`src-tauri/src/teamhandoff.rs`), quand le collègue prend avec Claude Code :
  - la version : sa version de `claude` doit être au moins celle de la session, sinon C16-A ;
  - la réécriture : par chaîne JSON, avec les deux séparateurs, du worktree de qui passe la main vers le sien ;
  - la vérification : plus aucune occurrence de l'ancien chemin, sinon C16-A ;
  - le dépôt : dans `<dossier de configuration du compte choisi>/projects/<worktree encodé>/<id>.jsonl` ;
  - le lancement : `--resume <id> --fork-session` ;
  - le repli : si le premier tour échoue (session non trouvée, erreur d'API), retour à C16-A, avec le journal au-dessus comme aujourd'hui.
- **Après** : la copie est effacée du serveur une fois reprise, comme le journal. La fenêtre du passage dit « Reprise de la session » ou « Passation », selon le cas.
- **Tests** : la réécriture et sa vérification sur une session réelle tirée de cet essai (anonymisée, en fixture), le choix entre native et passation, le repli. Un essai réel `real_session_copy` derrière une variable, comme les autres essais réels.
- **Ce qui attend l'utilisateur** : un essai à deux comptes de Claude Code (ou deux organisations), pour savoir ce que `session-owner` et `hybrid-organization` font à la reprise. Il se fait sur des comptes réels.
- **Coût** : un ticket, environ 2 h d'agent au rythme de Claude Code, plus l'essai à deux comptes.

Codex : à reprendre seulement si l'essai se finit (§ 2) et qu'on veut le même confort ; environ 1 h 30 d'agent de plus, sur le même canevas.

## 5. Le nettoyage

Effacés à la fin de l'essai, vérifiés absents :

- les deux dossiers de projet encodés de `~/.claude/projects` (`…-essai-a`, `…-essai-b`) et leur session ;
- les trois sessions de Codex de l'essai et leur copie réécrite sous `~/.codex/sessions/2026/11/21/` ;
- les dossiers temporaires que les lancements de `claude` ont créés (`Temp\claude\85c8…` et `Temp\claude\3309…`, avec les sessions de démarrage vides) ;
- les dossiers jetables `essai-a`, `essai-b`, `codex-a` et `codex-b`.

Aucun autre fichier de `~/.claude` ni de `~/.codex` n'a été lu, copié ou modifié : ni `.credentials.json`, ni le trousseau, ni les réglages, ni les autres sessions.
