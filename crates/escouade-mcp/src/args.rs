//! The arguments of the MCP server's tools: what the model sends, and the schema it reads (each
//! field's `#[schemars(description)]` its argument's). The tools that take them are the app's
//! (`mcp::tools`).

use escouade_model::model::Column;
use rmcp::schemars;
use serde::Deserialize;

// The arguments' descriptions are the model's (English, one line each: a doc comment's line
// breaks would stay in them).

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListAgentsArgs {
    #[schemars(
        description = "Only the agents of this project: its id, or its name in any case. Leave it out for every project's."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListFoldersArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out for its own."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateFolderArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out for its own."
    )]
    pub project: Option<String>,
    #[schemars(description = "The folder's name, as the sidebar shows it.")]
    pub name: String,
    #[schemars(
        description = "The agents to put in it, in this order: each one's id, or its name in any case. They leave their place."
    )]
    pub agents: Option<Vec<String>>,
    #[schemars(
        description = "The folder's principal, shown first and standing for the folder: one of its agents, by id or name."
    )]
    pub principal: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct UpdateFolderArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out for its own."
    )]
    pub project: Option<String>,
    #[schemars(description = "The folder: its id, or its name in any case.")]
    pub folder: String,
    #[schemars(description = "The new name of the folder.")]
    pub name: Option<String>,
    #[schemars(
        description = "The new principal: an agent of the folder, by id or name, or an empty string for none."
    )]
    pub principal: Option<String>,
    #[schemars(
        description = "Its rank among the folders and agents the read tools list, from 0, not counting the agents that wait (they come first) nor archived ones; past the end, the bottom."
    )]
    pub position: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteFolderArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out for its own."
    )]
    pub project: Option<String>,
    #[schemars(description = "The folder: its id, or its name in any case.")]
    pub folder: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveAgentArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The folder to put it in: its id, or its name in any case; an empty string for outside any folder. Left out, it stays where it is."
    )]
    pub folder: Option<String>,
    #[schemars(
        description = "Its rank among the folders and agents the read tools list in that place, in the folder or in the project's list, from 0, not counting the agents that wait (they come first, or under the folder's principal), the principal nor archived ones. Left out, the bottom."
    )]
    pub position: Option<u32>,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListTicketsArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
    #[schemars(
        description = "Only the tickets of this column of the board: todo (to do), doing (in progress), review (to test) or done. Leave it out for every column's."
    )]
    pub column: Option<ColumnArg>,
}

/// A column of the board, as a tool's argument names it.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum ColumnArg {
    Todo,
    Doing,
    Review,
    Done,
}

impl From<ColumnArg> for Column {
    fn from(c: ColumnArg) -> Self {
        match c {
            ColumnArg::Todo => Column::Todo,
            ColumnArg::Doing => Column::Doing,
            ColumnArg::Review => Column::Review,
            ColumnArg::Done => Column::Done,
        }
    }
}

impl PositionArg {
    pub fn name(self) -> &'static str {
        match self {
            PositionArg::Top => "top",
            PositionArg::Bottom => "bottom",
            PositionArg::Before => "before",
        }
    }
}

impl ColumnArg {
    pub fn name(self) -> &'static str {
        match self {
            ColumnArg::Todo => "todo",
            ColumnArg::Doing => "doing",
            ColumnArg::Review => "review",
            ColumnArg::Done => "done",
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListPullRequestsArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out for its own."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetPullRequestArgs {
    #[schemars(
        description = "The project: its id, or its name in any case. An agent of Escouade may leave it out for its own."
    )]
    pub project: Option<String>,
    #[schemars(description = "The pull request's number.")]
    pub number: u64,
    #[schemars(
        description = "The repository it is in (owner/name), when it is not the project's target: as list_pull_requests gives it (repo). Only a repository an agent or a ticket of the project knows."
    )]
    pub repo: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTicketArgs {
    #[schemars(
        description = "The ticket: its key (like ATL-12, in any case) or its id. An agent of Escouade may leave it out to read its own ticket."
    )]
    pub ticket: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTicketImageArgs {
    #[schemars(
        description = "The ticket: its key (like ATL-12, in any case) or its id. An agent of Escouade may leave it out to read an image of its own ticket."
    )]
    pub ticket: Option<String>,
    #[schemars(
        description = "The image's file, as get_ticket gives it. Give this or index, not both."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "The image's rank in the list get_ticket gives, from 0. Give this or file, not both."
    )]
    pub index: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetTicketFileArgs {
    #[schemars(
        description = "The ticket: its key (like ATL-12, in any case) or its id. An agent of Escouade may leave it out to read a file of its own ticket."
    )]
    pub ticket: Option<String>,
    #[schemars(
        description = "The file's file, as get_ticket gives it (files). Give this or index, not both."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "The file's rank in the list get_ticket gives (files), from 0. Give this or file, not both."
    )]
    pub index: Option<u32>,
    #[schemars(
        description = "Where to start reading, in bytes from the start of the file (0 for its start). Leave it out to read its end."
    )]
    pub offset: Option<u64>,
    #[schemars(
        description = "The most bytes to read: 8192 by default, 1 at least and 65536 at most."
    )]
    pub bytes: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetAgentSummaryArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListBackgroundWorkArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBackgroundOutputArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The subagent or task to read: its id, as list_background_work gives it (id). A subagent may also be named by its taskId."
    )]
    pub id: String,
    #[schemars(
        description = "The most bytes of the output to give, its end: 8192 by default, 1 at least and 65536 at most (a task that still runs gives 8192 at most). Not for a subagent's report, which is given whole."
    )]
    pub bytes: Option<u64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StopBackgroundWorkArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The subagent or task to stop: its id, as list_background_work gives it (id). A subagent may also be named by its taskId."
    )]
    pub id: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MessageSubagentArgs {
    #[schemars(description = "The agent whose subagent it is: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The subagent to write to: its id, as list_background_work gives it (id). It may also be named by its taskId."
    )]
    pub subagent: String,
    #[schemars(
        description = "The message, 32,000 characters at most. The agent is asked to pass it on to the subagent, which reads it headed by your name (Message from <you>: …), as coming from you and not from the user."
    )]
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListRequestsArgs {
    #[schemars(
        description = "Only this agent's requests: its id, or its name in any case. Leave it out for every agent's."
    )]
    pub agent: Option<String>,
    #[schemars(
        description = "Only the requests of this project's agents (and the project to look for the agent's name in: two projects may each have an agent of that name): its id, or its name in any case. Leave it out for every project's."
    )]
    pub project: Option<String>,
}

/// How `answer_permission` answers. `Always` is no choice the schema offers (nor the error of a
/// decision that is none lists it): a lasting permission rule is the user's to give, in the window,
/// so it is read to be refused with the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, schemars::JsonSchema)]
#[schemars(rename_all = "lowercase", inline)]
pub enum DecisionArg {
    Allow,
    Deny,
    #[schemars(skip)]
    Always,
}

impl<'de> Deserialize<'de> for DecisionArg {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let said = String::deserialize(deserializer)?;
        match said.as_str() {
            "allow" => Ok(DecisionArg::Allow),
            "deny" => Ok(DecisionArg::Deny),
            "always" => Ok(DecisionArg::Always),
            other => Err(serde::de::Error::custom(format!(
                "unknown decision `{other}`, expected `allow` or `deny`"
            ))),
        }
    }
}

impl DecisionArg {
    pub fn name(self) -> &'static str {
        match self {
            DecisionArg::Allow => "allow",
            DecisionArg::Deny => "deny",
            DecisionArg::Always => "always",
        }
    }
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnswerPermissionArgs {
    #[schemars(description = "The agent that asks: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for the agent's name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The request to answer: its id, as list_requests gives it (request)."
    )]
    pub request: String,
    #[schemars(
        description = "allow: the tool runs (or the plan is approved); deny: it does not (or the agent keeps planning). Always allow is not offered: a lasting permission rule is the user's to give, in the window."
    )]
    pub decision: DecisionArg,
    #[schemars(
        description = "Only with deny: what the agent should know of the refusal, or do instead. Leave it out to refuse without a word."
    )]
    pub message: Option<String>,
}

/// One question's answer, for `answer_question`.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct QuestionAnswer {
    #[schemars(
        description = "The question this answers: its text, as list_requests gives it. Leave it out when the request asks a single question."
    )]
    pub question: Option<String>,
    #[schemars(
        description = "The options picked, by their labels as list_requests gives them: one, or several when the question allows it (multiSelect). Give this or text, not both."
    )]
    pub choices: Option<Vec<String>>,
    #[schemars(
        description = "A free answer, when no option fits. Give this or choices, not both."
    )]
    pub text: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct AnswerQuestionArgs {
    #[schemars(description = "The agent that asks: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for the agent's name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The request to answer: its id, as list_requests gives it (request)."
    )]
    pub request: String,
    #[schemars(
        description = "One answer for each of the request's questions: the options picked (choices) or a free text."
    )]
    pub answers: Vec<QuestionAnswer>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WaitTicketArgs {
    #[schemars(
        description = "The tickets to watch (20 at most): their keys (like ATL-12, in any case) or their ids."
    )]
    pub tickets: Vec<String>,
    #[schemars(
        description = "The most seconds to wait: 120 by default, 240 at most (a longer wait is cut to it), 1 at least."
    )]
    pub timeout: Option<u64>,
    #[schemars(
        description = "The since the previous wait_ticket answered, or that an action on the ticket gave (approve_ticket, reject_ticket, resume_ticket, resolve_conflict, dismiss_block: the ticket as it was before), as it was given, even across other tool calls: a ticket whose state differs from the one it holds is answered at once, a change made between the two calls included. Leave it out and each ticket's state at the call is the one compared."
    )]
    pub since: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateTicketArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
    #[schemars(description = "The ticket's title, on one line (200 characters at most).")]
    pub title: String,
    #[schemars(
        description = "What to do, in a few sentences or paragraphs (10,000 characters at most). Leave it out for none."
    )]
    pub description: Option<String>,
    #[schemars(
        description = "The acceptance criteria, one per item (30 at most, 500 characters each): the ticket's agent checks each one in its report. Leave them out for two default ones."
    )]
    pub criteria: Option<Vec<String>>,
    #[schemars(
        description = "The tickets of the same project this one comes after, by key (like ATL-12, in any case) or id: it starts once all are done. Leave it out for none."
    )]
    pub after: Option<Vec<String>>,
    #[schemars(
        description = "Images to attach (screenshots, mock-ups; 20 at most, 5 MB each and 18 MB in all: PNG, JPEG, GIF or WebP, read from their content), which the ticket's agent gets with its first message. Leave them out for none."
    )]
    pub images: Option<Vec<ImageArg>>,
    #[schemars(
        description = "Other files to attach, of any format (a spreadsheet, an archive, a log, a PDF…; 20 at most, each no bigger than the size the settings allow, Settings › Advanced › Attachments), whose paths the ticket's agent gets with its first message. Leave them out for none."
    )]
    pub files: Option<Vec<FileArg>>,
    #[schemars(
        description = "The account its agents start on, every time: its name (in any case) or its id, or auto (the default), the current account of the ticket's harness. Set on an account, its agents stay there: at its usage limit or past the threshold to change accounts, they wait for its reset. An account its project may not use is refused, with why."
    )]
    pub account: Option<String>,
    #[schemars(
        description = "The model of its agents, as the window's model menu offers it for the harness of the ticket, else of its account, else of its project (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives the project's models), or auto (the default): the Kanban's, else its account's."
    )]
    pub model: Option<String>,
    #[schemars(
        description = "The effort of its agents (low, medium, high, xhigh or max: those of the harness and of the model, see list_projects), or auto (the default): the Kanban's, else its account's (a model that takes none, Haiku, goes without)."
    )]
    pub effort: Option<String>,
    #[schemars(
        description = "Share it with the team (true): Escouade writes its file .escouade/tickets/<id>.yml in the project's folder and keeps its status up to date; the user commits it, Escouade never does. Left out: as the project's « Partager chaque nouveau ticket » says. Refused while the user has given no name."
    )]
    pub shared: Option<bool>,
}

/// A file to attach to a ticket, as `create_ticket` and `update_ticket` take it.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct FileArg {
    #[schemars(
        description = "Its name (only what follows the last slash is kept, made safe as the window makes it). Required with data; with path it is the file's name by default."
    )]
    pub name: Option<String>,
    #[schemars(
        description = "Its content in base64, for a small file (a request weighs 32 MiB at most: give a bigger file by its path). Give this or path, not both."
    )]
    pub data: Option<String>,
    #[schemars(
        description = "The full path of a file on this computer, which is copied. An agent of Escouade can only give a file of its working folder (a safeguard of the server, not a security boundary). Give this or data, not both."
    )]
    pub path: Option<String>,
}

/// A file of a ticket, as the tools that name one take it.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct FileRef {
    #[schemars(
        description = "The file's file, as get_ticket gives it (files). Give this or index, not both."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "The file's rank in the list get_ticket gives (files), from 0. Give this or file, not both."
    )]
    pub index: Option<u32>,
}

/// An image to attach to a ticket, as `create_ticket`, `update_ticket` and the like take it.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(inline)]
pub struct ImageArg {
    #[schemars(
        description = "Its name (100 characters at most; only what follows the last slash is kept). Required with data; with path it is the file's name by default."
    )]
    pub name: Option<String>,
    #[schemars(
        description = "Its media type, as an indication only: the image's type is the one its content has (PNG, JPEG, GIF or WebP)."
    )]
    pub media_type: Option<String>,
    #[schemars(description = "Its content in base64. Give this or path, not both.")]
    pub data: Option<String>,
    #[schemars(
        description = "The full path of an image file on this computer (5 MB at most). An agent of Escouade can only give a file of its working folder (a safeguard of the server, not a security boundary). Give this or data, not both."
    )]
    pub path: Option<String>,
}

/// An image of a ticket, as the tools that name one take it.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct ImageRef {
    #[schemars(
        description = "The image's file, as get_ticket gives it. Give this or index, not both."
    )]
    pub file: Option<String>,
    #[schemars(
        description = "The image's rank in the list get_ticket gives, from 0. Give this or file, not both."
    )]
    pub index: Option<u32>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTicketArgs {
    #[schemars(description = "The ticket: its key (like ATL-12, in any case) or its id.")]
    pub ticket: String,
    #[schemars(
        description = "The new title, on one line (200 characters at most). Leave it out to keep the current one."
    )]
    pub title: Option<String>,
    #[schemars(
        description = "The new description (10,000 characters at most, not counting the line saying who edited it, which is added after it). Leave it out to keep the current one."
    )]
    pub description: Option<String>,
    #[schemars(
        description = "The new acceptance criteria, one per item, replacing all the current ones (an empty list gives the two default ones). Leave it out to keep the current ones."
    )]
    pub criteria: Option<Vec<String>>,
    #[schemars(
        description = "The tickets of the same project it now comes after, by key or id, replacing all the current ones (an empty list takes them all away). Leave it out to keep the current ones."
    )]
    pub after: Option<Vec<String>>,
    #[schemars(
        description = "Images to attach, added after the ones the ticket has (same form and limits as create_ticket's images: 20 images in all at most). Leave it out to add none."
    )]
    pub add_images: Option<Vec<ImageArg>>,
    #[schemars(
        description = "Images of the ticket to take away, each named by its file or its rank in the list get_ticket gives. The images you do not name stay. Leave it out to take none away."
    )]
    pub remove_images: Option<Vec<ImageRef>>,
    #[schemars(
        description = "Other files to attach, added after the ones the ticket has (same form and limits as create_ticket's files: 20 files in all at most). Leave it out to add none."
    )]
    pub add_files: Option<Vec<FileArg>>,
    #[schemars(
        description = "Files of the ticket to take away, each named by its file or its rank in the list get_ticket gives (files). The files you do not name stay. Leave it out to take none away."
    )]
    pub remove_files: Option<Vec<FileRef>>,
    #[schemars(
        description = "The account its agents start on, every time: its name (in any case) or its id, or auto, the current account of the ticket's harness. Set on an account, its agents stay there: at its usage limit or past the threshold to change accounts, they wait for its reset. An account its project may not use is refused, with why. Leave it out to keep it."
    )]
    pub account: Option<String>,
    #[schemars(
        description = "The model of its agents, as the window's model menu offers it for the harness of the ticket, else of its account, else of its project (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives the project's models), or auto: the Kanban's, else its account's. Leave it out to keep it."
    )]
    pub model: Option<String>,
    #[schemars(
        description = "The effort of its agents (low, medium, high, xhigh or max: those of the harness and of the model, see list_projects), or auto: the Kanban's, else its account's (a model that takes none, Haiku, goes without). Leave it out to keep it."
    )]
    pub effort: Option<String>,
    #[schemars(
        description = "true shares it with the team (its file .escouade/tickets/<id>.yml, committed by the user), false stops sharing it (its file is deleted, to be committed). Any column. Leave it out to keep it."
    )]
    pub shared: Option<bool>,
}

/// Where `move_ticket` puts a ticket.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum PositionArg {
    Top,
    Bottom,
    Before,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveTicketArgs {
    #[schemars(description = "The ticket to move: its key (like ATL-12, in any case) or its id.")]
    pub ticket: String,
    #[schemars(
        description = "Where it goes in the To do column: top (first), bottom (last) or before (just before the ticket named by before)."
    )]
    pub position: PositionArg,
    #[schemars(
        description = "Only with position before: the To do ticket of the same project to put this one just before, by key or id."
    )]
    pub before: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StartTicketArgs {
    #[schemars(
        description = "The ticket to launch: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
    #[schemars(
        description = "The account this ticket's agent starts on: its name (in any case) or its id, or auto for the current account of its harness. It is the ticket's from then on, as its model and effort are, and takes precedence over the one it had. Leave it out to leave it as it is."
    )]
    pub account: Option<String>,
    #[schemars(
        description = "The model of this ticket's agent, as its harness names it (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives the project's models), for this ticket alone: it takes precedence over the Kanban's, e.g. keep opus on an architecture ticket while the Kanban is on sonnet. An empty string, or auto, goes back to the Kanban's. Leave it out to leave it as it is."
    )]
    pub model: Option<String>,
    #[schemars(
        description = "The effort of this ticket's agent (low, medium, high, xhigh or max: those of its harness and model, see list_projects), for this ticket alone: it takes precedence over the Kanban's (a model that takes none, Haiku, goes without). An empty string, or auto, goes back to the Kanban's. Leave it out to leave it as it is."
    )]
    pub effort: Option<String>,
    #[schemars(
        description = "The folder the agents of this ticket go in (their sidebar folder), kept on the ticket (also when it is resumed): its id or its name in any case, made under that name if there is none. Leave it out: your own folder, made for you (named after you) if you have none. An empty string: no folder. Left out, from another project or from outside Escouade: no automatic folder."
    )]
    pub folder: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetWorkflowArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
}

const MEMORY_PROJECT: &str = "The project: its id, or its name in any case. An agent of Escouade may leave it out for its own.";

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SearchMemoryArgs {
    #[schemars(description = MEMORY_PROJECT)]
    pub project: Option<String>,
    #[schemars(
        description = "What to look for, in a few words (any language). Empty: the memories by name, up to limit."
    )]
    pub query: String,
    #[schemars(description = "The most memories answered: 10 when left out, 50 at most.")]
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadMemoryArgs {
    #[schemars(description = MEMORY_PROJECT)]
    pub project: Option<String>,
    #[schemars(description = "The memory's name, as search_memory gives it.")]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct WriteMemoryArgs {
    #[schemars(description = MEMORY_PROJECT)]
    pub project: Option<String>,
    #[schemars(
        description = "Its name, short, in kebab-case (made so: lower case letters, digits and dashes). A memory of that name is replaced: read it first to keep what it says."
    )]
    pub name: String,
    #[schemars(
        description = "One line that says what it is about, so that a search and the index tell when it matters (300 characters at most)."
    )]
    pub description: String,
    #[serde(rename = "type")]
    #[schemars(
        description = "Its kind: feedback (a way to work, a trap to avoid; the default), project (a constraint of the work under way), reference (where to find something) or user (what the user prefers)."
    )]
    pub kind: Option<crate::memory::Kind>,
    #[schemars(
        description = "What to remember, in Markdown: the rule, then why, then how to apply it (20,000 characters at most)."
    )]
    pub body: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ForgetMemoryArgs {
    #[schemars(description = MEMORY_PROJECT)]
    pub project: Option<String>,
    #[schemars(description = "The memory's name.")]
    pub name: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GetBoardArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
}

/// What validating a ticket does, as `set_board` names it.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum ActionArg {
    Merge,
    Pr,
    Push,
    Keep,
}

impl ActionArg {
    pub fn name(self) -> &'static str {
        match self {
            ActionArg::Merge => "merge",
            ActionArg::Pr => "pr",
            ActionArg::Push => "push",
            ActionArg::Keep => "keep",
        }
    }
}

/// How a merge goes in, as `set_board` names it.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum StrategyArg {
    Merge,
    Squash,
    Rebase,
}

impl StrategyArg {
    pub fn name(self) -> &'static str {
        match self {
            StrategyArg::Merge => "merge",
            StrategyArg::Squash => "squash",
            StrategyArg::Rebase => "rebase",
        }
    }
}

/// What a merge conflict does, as `set_board` names it.
#[derive(Debug, Clone, Copy, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
#[schemars(inline)]
pub enum ConflictArg {
    Ask,
    Agent,
    Abort,
}

impl ConflictArg {
    pub fn name(self) -> &'static str {
        match self {
            ConflictArg::Ask => "ask",
            ConflictArg::Agent => "agent",
            ConflictArg::Abort => "abort",
        }
    }
}

// Named as `get_board` answers them (camelCase), so that its answer can be given back as it is.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetBoardArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
    #[schemars(
        description = "What approving a ticket does: merge (into the target branch), pr (push its branch and open a pull request), push (push its branch only) or keep (nothing: the changes stay in its worktree). Leave it out to keep the current one."
    )]
    pub action: Option<ActionArg>,
    #[schemars(
        description = "The target branch: where tickets merge, the base of their pull request and of their branch. One of the project's local branches, unless it is the current one (the Kanban's own, or the branch get_board shows while the Kanban has none): it is then left as it is. Leave it out to keep the current one."
    )]
    pub target: Option<String>,
    #[schemars(
        description = "How a merge goes in: merge (a merge commit), squash or rebase. Leave it out to keep the current one."
    )]
    pub strategy: Option<StrategyArg>,
    #[schemars(
        description = "true: the pull request opens as a draft. Leave it out to keep the current one."
    )]
    pub draft: Option<bool>,
    #[schemars(
        description = "true: the ticket's worktree is deleted once it is approved. Leave it out to keep the current one."
    )]
    pub cleanup: Option<bool>,
    #[schemars(
        description = "true: the commit messages are written by Haiku in the Conventional Commits form. Leave it out to keep the current one."
    )]
    pub conventional: Option<bool>,
    #[schemars(
        description = "What a merge conflict does: ask (the user decides), agent (the ticket's agent resolves it) or abort (cancel the merge). Leave it out to keep the current one."
    )]
    pub conflict: Option<ConflictArg>,
    #[schemars(
        description = "The most tickets in progress at once, 1 to 6. Leave it out to keep the current one."
    )]
    pub max_parallel: Option<u32>,
    #[schemars(
        description = "true: the autopilot starts the To do tickets by itself as soon as a place is free; false: they wait to be launched by hand. Leave it out to keep the current one."
    )]
    pub autopilot: Option<bool>,
    #[schemars(
        description = "The model of the tickets' agents, as the project's harness names it (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives its models). An empty string is the app's default model. Leave it out to keep the current one."
    )]
    pub model: Option<String>,
    #[schemars(
        description = "The effort of the tickets' agents: low, medium, high, xhigh or max, those of the project's harness and model (list_projects). An empty string is the app's default effort. Leave it out to keep the current one."
    )]
    pub effort: Option<String>,
    #[schemars(
        description = "The permission mode of the tickets' agents: auto, default, plan or acceptEdits. An empty string is the app's default mode. bypassPermissions (no permission request at all) is the user's to give, in the window: it is refused here unless the Kanban is already on it. auto does not apply to a model that has no auto mode (Haiku): its agents go in the default mode. Leave it out to keep the current one."
    )]
    pub mode: Option<String>,
    #[schemars(
        description = "true: the tests run again before approving a ticket, and a failing test sends it back to its agent. Refused to turn it on when the Kanban has no test command: it is set in the window, the server runs no command (it is off with none, as get_board says). Leave it out to keep the current one."
    )]
    pub tests_first: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ApproveTicketArgs {
    #[schemars(
        description = "The ticket to approve: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct RejectTicketArgs {
    #[schemars(
        description = "The ticket to send back: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
    #[schemars(
        description = "What is wrong, or what to do: the agent reads it as test feedback, followed by who sent the ticket back."
    )]
    pub comment: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResumeTicketArgs {
    #[schemars(
        description = "The blocked In progress ticket to take up: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ResolveConflictArgs {
    #[schemars(
        description = "The To review ticket stopped on a merge conflict: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DismissBlockArgs {
    #[schemars(
        description = "The blocked To review ticket: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DeleteTicketArgs {
    #[schemars(
        description = "The ticket to delete, in any column: its key (like ATL-12, in any case) or its id."
    )]
    pub ticket: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateAgentArgs {
    #[schemars(description = "The project: its id, or its name in any case.")]
    pub project: String,
    #[schemars(
        description = "The agent's first message: what it should do. The agent reads it headed by your name (Message from <you>: …), as coming from you and not from the user."
    )]
    pub message: String,
    #[schemars(
        description = "true: the agent works in a worktree of its own; false: in the project's folder. Leave it out to do as the project does by default."
    )]
    pub worktree: Option<bool>,
    #[schemars(
        description = "The model, as the project's harness names it (sonnet, opus, haiku for Claude Code; gpt-6-luna… for Codex; list_projects gives its models). Leave it out for the default model."
    )]
    pub model: Option<String>,
    #[schemars(
        description = "The folder the new agent goes in (its sidebar folder): its id or its name in any case, made under that name if there is none. Leave it out: your own folder, made for you (named after you) if you have none. An empty string: no folder. Left out, from another project or from outside Escouade: no automatic folder."
    )]
    pub folder: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SendMessageArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
    #[schemars(
        description = "The message, 32,000 characters at most. The agent reads it, and the conversation shows it, headed by your name (Message from <you>: …), as coming from you and not from the user."
    )]
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct StopAgentArgs {
    #[schemars(description = "The agent: its id, or its name in any case.")]
    pub agent: String,
    #[schemars(
        description = "The project to look for its name in (two projects may each have an agent of that name): its id, or its name in any case."
    )]
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReportProgressArgs {
    #[schemars(
        description = "What you are doing, in one line of 120 characters at most. An empty line takes the previous one off."
    )]
    pub line: String,
}

/// The most tickets one `split_ticket` makes.
pub const SPLIT_MAX: usize = 10;

/// One of the tickets `split_ticket` makes.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[schemars(inline)]
pub struct SubTicket {
    #[schemars(description = "The ticket's title, on one line (200 characters at most).")]
    pub title: String,
    #[schemars(description = "What to do in this ticket (10,000 characters at most).")]
    pub description: Option<String>,
    #[schemars(
        description = "Its acceptance criteria, one per item. Leave them out for two default ones."
    )]
    pub criteria: Option<Vec<String>>,
    #[schemars(
        description = "The images of your ticket this one gets a copy of, each named by its file or its rank in the list get_ticket gives. Leave it out for all of them, give an empty list for none."
    )]
    pub images: Option<Vec<ImageRef>>,
    #[schemars(
        description = "The other files of your ticket this one gets a copy of, each named by its file or its rank in the list get_ticket gives (files). Leave it out for all of them, give an empty list for none."
    )]
    pub files: Option<Vec<FileRef>>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SplitTicketArgs {
    #[schemars(description = "The tickets to make (10 at most), each with a title.")]
    pub tickets: Vec<SubTicket>,
    #[schemars(
        description = "true (the default): each ticket comes after the one before it, the first after yours; false: all come after yours alone and may run in parallel."
    )]
    pub chain: Option<bool>,
}
