//! What the core still asks a `codex app-server` itself, outside any conversation: the quota and the models of an
//! account none of whose agents runs. An app-server of its own (`env`: its `CODEX_HOME`), started for that and stopped
//! once it answered; it starts no thread, nothing is spent, and it costs little (0,6 s and 62 Mo on Codex 0.163.0,
//! tried). The conversation of an agent goes through its plugin (`dev.escouade.codex`).
