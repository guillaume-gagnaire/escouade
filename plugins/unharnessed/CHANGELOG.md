# Changelog

## 1.0.0 (2026-11-16)

- Its session taken back after a restart (`acp.resume`: `--resume <id>`, under the id it saves it under); its turns of its own (a wake) shown as turns (`acp.turnEnd`); what its context holds read from its stream (`acp.usage`). Needs Escouade 2.2.2, which refuses its agents when Bypass mode is not allowed; these three need 2.2.3 and are ignored before (ESC-491).

- Unharnessed as a harness of Escouade, described over ACP without code (`unharnessed --mode acp`), served by the ACP
  plugin (`dev.escouade.acp`): its one mode, Bypass, said in clear; its Python cell shown as « Python » with its code;
  its model its own default; its questions in text; an account by `UNHARNESSED_CODING_AGENT_DIR`; no installer, no
  read-only discussion, no private project. Tried with Unharnessed 0.9.8 built from source.
