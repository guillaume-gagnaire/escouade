# Changelog

## 1.0.0

- Prime Agent, Prime Intellect's coding agent, as a harness of Escouade, described over ACP without code
  (`prime-agent --mode acp --no-session`), served by the ACP plugin the app gives (`dev.escouade.acp`): its one mode,
  Bypass, said in clear; its one tool, a cell of Python, shown with its code; its model by its configuration option;
  its questions in text; its official installers and `prime-agent update`. Tried with Prime Agent 0.10.3 in a sandbox. Needs Escouade
  2.2.2, which refuses its agents when Bypass mode is not allowed.
- Its effort by its option `thought_level`, for a model that reasons (`acp.efforts`); its turns of its own shown as turns (`acp.turnEnd`). After a restart, the new session it starts on is said, given its instructions again: these need Escouade 2.2.3 and are ignored before (ESC-491).
