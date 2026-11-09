# Les artefacts de Claude Code en stream-json (ESC-400)

Enregistré le 2026-11-09 sous Windows 11 avec Claude Code 2.1.319, sur le compte principal : le lancement d'un agent d'Escouade (`--output-format stream-json --verbose --input-format stream-json --include-partial-messages --replay-user-messages`, Sonnet, `bypassPermissions`), un message qui demande de charger la skill `artifact-design` puis de publier une petite page privée.

| Fichier | Lancement |
|---|---|
| `without-artifact-env.jsonl` | sans `CLAUDE_CODE_ARTIFACT` : `init` sans outil `Artifact`, `Skill` → « Unknown skill: artifact-design », repli sur Claude Docs |
| `with-artifact-env.jsonl` | avec `CLAUDE_CODE_ARTIFACT=1` : `init` avec `Artifact`, `ArtifactComments`, `ArtifactData` ; la skill se charge, `Write` puis `Artifact` publie la page et rend son lien |

Pourquoi : en stream-json, Claude Code pose `CLAUDE_CODE_ENTRYPOINT=sdk-cli` et coupe ses artefacts par défaut (« sdk_default_off ») tant que `CLAUDE_CODE_ARTIFACT` n'est pas vraie.

Gardé : la frame `init` (ses outils hors MCP, ses skills), les messages entiers, les résultats d'outils, la fin du tour ; sans les `stream_event`, ni l'usage. Anonymisé : chemins (`C:\Users\user`), identifiants de session et de message, adresses des deux artefacts. Les deux artefacts de l'essai ont été supprimés aussitôt (`Artifact`, `action: delete`, accordé sur la carte de permission).
