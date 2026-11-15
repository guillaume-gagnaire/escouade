//! « Affiner… » (ESC-196, LOOP-5 of ESC-191): a short agent that is given what the rules found of a
//! project's workflows and the files they stand on, and answers with the same workflows more
//! precisely, in a fixed JSON schema that the rules check. Only on request, one question to the
//! harness of « Modèle des textes rédigés », no tool, no session; its model, its account and its
//! cost estimated are said before it is asked (`RefinePlan`). In a private project (ESC-140) the
//! agent is the project's: it is asked of an account reserved to it.
//!
//! Nothing it makes up gets in unmarked: the steps and what they run that the rules found stay as
//! they are (their description may be said better); a step it adds enters only when it stands on a
//! file the reading has seen, and as « deviné » (`Step::guessed`, dotted until the user keeps or
//! removes it); a step with no file, a transition between steps that are not there, an agent no file
//! defines, are refused and told. What it gives goes into the workflow's file, as a reading does:
//! the user's corrections kept, nothing written when nothing changes, never committed.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::i18n::Lang;
use crate::workflow::{Kind, Roots, Scan, Step, StepAgent, Transition, Workflow};
use crate::workflowfiles::fingerprint_of;

/// How long the agent may take.
pub const LIMIT: Duration = Duration::from_secs(120);

/// What the user is told before it is asked.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RefinePlan {
    pub workflows: usize,
    pub files: usize,
    /// The harness' id (`claude`).
    pub harness: String,
    /// As the interface names it (`Claude Haiku`).
    pub model: String,
    pub account: String,
    pub account_id: String,
    /// What the question weighs and the answer may, in tokens (rough: a token is about 3 letters).
    pub input_tokens: u64,
    pub output_tokens: u64,
    /// In dollars, at the list price, when the model's is known (none for a plan with no price).
    pub cost: Option<f64>,
    /// An agent of the project, on the accounts reserved to it (ESC-140).
    pub private: bool,
}

/// What a workflow gained.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Refined {
    pub id: String,
    pub name: String,
    /// The steps added, all guessed.
    pub added: Vec<String>,
    /// The steps whose description was said better.
    pub described: usize,
    /// The transitions added.
    pub transitions: usize,
}

/// What an « Affiner… » did.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub scan: Scan,
    pub refined: Vec<Refined>,
    /// What the rules refused of the answer, one line each.
    pub rejected: Vec<String>,
}

// ---------- the question ----------

pub fn system(lang: Lang) -> String {
    tr_in!(
        lang,
        "Tu précises des workflows d'agents de code à partir de leurs fichiers, sans jamais réaliser de tâche. Tu réponds uniquement par un objet JSON, sans texte autour.",
        "You refine coding-agent workflows from their files, without ever carrying out a task. You answer with a JSON object only, no text around it."
    )
}

/// The workflows as the agent is told them: what is useful to refine, no machine's state.
fn told(w: &Workflow) -> Value {
    json!({
        "id": w.id,
        "name": w.name,
        "steps": w.steps.iter().map(|s| json!({
            "name": s.name, "command": s.command, "description": s.description, "path": s.path,
            "agents": s.agents.iter().map(|a| a.name.clone()).collect::<Vec<_>>(),
            "reads": s.reads, "writes": s.writes, "guessed": s.guessed,
        })).collect::<Vec<_>>(),
        "transitions": w.transitions.iter().map(|t| json!({
            "from": t.from, "to": t.to, "back": t.back, "condition": t.condition, "max": t.max,
        })).collect::<Vec<_>>(),
        "exit": w.exit,
    })
}

pub fn prompt(lang: Lang, workflows: &[Workflow], files: &[(String, String)]) -> String {
    let told = Value::Array(workflows.iter().map(told).collect());
    let json = serde_json::to_string_pretty(&told).unwrap_or_default();
    let files: String = files
        .iter()
        .map(|(p, t)| format!("=== {p}\n{t}\n"))
        .collect();
    tr_in!(
        lang,
        "Voici les workflows que les règles ont trouvés dans ce projet, puis les fichiers sur lesquels ils reposent.\n\
Précise-les : dis mieux ce que fait chaque étape (une phrase), relie les étapes entre elles, signale les retours en arrière (et leur condition, leur maximum).\n\
Tu ne retires aucune étape. N'ajoute une étape que si un fichier ci-dessous la porte (son `path` est alors celui de ce fichier) ; elle sera marquée devinée. Rien d'inventé : une étape sans fichier est refusée.\n\
Réponds par un seul objet JSON de cette forme, et rien d'autre :\n\
{{\"workflows\":[{{\"id\":\"<id donné>\",\"steps\":[{{\"name\":\"\",\"command\":\"\",\"description\":\"\",\"path\":\"\",\"agents\":[\"\"],\"reads\":[\"\"],\"writes\":[\"\"]}}],\"transitions\":[{{\"from\":\"<étape>\",\"to\":\"<étape>\",\"condition\":null,\"max\":null}}],\"exit\":null}}]}}\n\
<workflows-json>\n{json}\n</workflows-json>\n<fichiers-workflow>\n{files}</fichiers-workflow>\n",
        "Here are the workflows the rules found in this project, then the files they stand on.\n\
Refine them: say better what each step does (one sentence), link the steps to one another, point out the ways back (with their condition and maximum).\n\
You remove no step. Add a step only when a file below carries it (its `path` is then that file's); it will be marked guessed. Nothing made up: a step with no file is refused.\n\
Answer with a single JSON object of this shape, and nothing else:\n\
{{\"workflows\":[{{\"id\":\"<id given>\",\"steps\":[{{\"name\":\"\",\"command\":\"\",\"description\":\"\",\"path\":\"\",\"agents\":[\"\"],\"reads\":[\"\"],\"writes\":[\"\"]}}],\"transitions\":[{{\"from\":\"<step>\",\"to\":\"<step>\",\"condition\":null,\"max\":null}}],\"exit\":null}}]}}\n\
<workflows-json>\n{json}\n</workflows-json>\n<files-workflow>\n{files}</files-workflow>\n",
        json = json,
        files = files
    )
}

/// What the question costs, rough, at the list price of `priced` (the model's id for the grid).
pub fn estimate(system: &str, prompt: &str, steps: usize, priced: &str) -> (u64, u64, Option<f64>) {
    let input = (system.len() + prompt.len()) as u64 / 3;
    let output = 300 + 120 * steps as u64;
    let usage = crate::pricing::Usage {
        input,
        output,
        ..Default::default()
    };
    (input, output, crate::pricing::estimate(priced, &usage))
}

// ---------- the answer ----------

#[derive(Debug, Deserialize)]
struct AStep {
    name: String,
    #[serde(default)]
    command: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    agents: Vec<String>,
    #[serde(default)]
    reads: Vec<String>,
    #[serde(default)]
    writes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ATransition {
    from: String,
    to: String,
    #[serde(default)]
    condition: Option<String>,
    #[serde(default)]
    max: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct AWorkflow {
    id: String,
    #[serde(default)]
    steps: Vec<AStep>,
    #[serde(default)]
    transitions: Vec<ATransition>,
    #[serde(default)]
    exit: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Answer {
    pub workflows: Vec<AWorkflow>,
}

/// The JSON object an answer holds: the whole of it, one in a code fence, or from its first `{`
/// to its last `}`.
fn json_text(answer: &str) -> &str {
    let t = answer.trim();
    if let Some(start) = t.find("```") {
        let rest = &t[start + 3..];
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        if let Some(end) = rest.find("```") {
            return rest[..end].trim();
        }
    }
    match (t.find('{'), t.rfind('}')) {
        (Some(a), Some(b)) if a < b => &t[a..=b],
        _ => t,
    }
}

/// The answer read against the schema, or why it is not one.
pub fn parse(answer: &str) -> Result<Answer, String> {
    serde_json::from_str::<Answer>(json_text(answer)).map_err(|e| e.to_string())
}

const NAME_MAX: usize = 80;
const TEXT_MAX: usize = 300;

fn clip(s: &str, max: usize) -> String {
    let t = s.trim();
    if t.chars().count() <= max {
        t.to_string()
    } else {
        t.chars().take(max).collect()
    }
}

fn norm_path(p: &str) -> String {
    p.trim().replace('\\', "/")
}

/// One workflow's answer checked against the rules.
pub struct Checked {
    pub workflow: Workflow,
    pub refined: Refined,
    pub rejected: Vec<String>,
}

/// What `a` says of `current`, kept only as far as the rules allow (see the module): `known` are the
/// files the reading has seen. The workflow returned is `current` with what stood; it carries
/// `refined` (what it was refined from) when it differs.
pub fn check(current: &Workflow, a: &AWorkflow, known: &BTreeSet<String>) -> Checked {
    let mut w = current.clone();
    let mut refined = Refined {
        id: current.id.clone(),
        name: current.name.clone(),
        ..Default::default()
    };
    let mut rejected = Vec::new();
    let who = &current.name;
    let agent_names: BTreeSet<String> = current
        .steps
        .iter()
        .flat_map(|s| s.agents.iter().map(|x| x.name.clone()))
        .collect();
    let state: BTreeSet<String> = current.state_files.iter().map(|s| s.path.clone()).collect();

    // The steps, in the order the agent gives them; those it left out follow, as they were.
    let mut steps: Vec<Step> = Vec::new();
    for s in &a.steps {
        let name = clip(&s.name, NAME_MAX);
        if name.is_empty() || steps.iter().any(|x| x.name == name) {
            continue;
        }
        if let Some(old) = current.steps.iter().find(|x| x.name == name) {
            let mut step = old.clone();
            let said = clip(&s.description, TEXT_MAX);
            if !said.is_empty() && said != old.description {
                step.description = said;
                refined.described += 1;
            }
            steps.push(step);
            continue;
        }
        // A step the rules did not find: it needs a file the reading has seen.
        let path = norm_path(&s.path);
        if path.is_empty() || !known.contains(&path) {
            rejected.push(tr_in!(
                crate::i18n::ui(),
                "{who} : l'étape « {name} » n'a pas de fichier connu, refusée.",
                "{who}: step “{name}” has no known file, refused.",
            ));
            continue;
        }
        let agents = s
            .agents
            .iter()
            .map(|n| clip(n, NAME_MAX))
            .filter(|n| agent_names.contains(n))
            .map(|name| StepAgent {
                path: current
                    .steps
                    .iter()
                    .flat_map(|x| x.agents.iter())
                    .find(|x| x.name == name)
                    .and_then(|x| x.path.clone()),
                description: String::new(),
                name,
            })
            .collect();
        let seen = |list: &[String]| -> Vec<String> {
            list.iter()
                .map(|p| norm_path(p))
                .filter(|p| known.contains(p) || state.contains(p))
                .collect()
        };
        steps.push(Step {
            name: name.clone(),
            command: clip(&s.command, NAME_MAX),
            order: 0,
            description: clip(&s.description, TEXT_MAX),
            path,
            agents,
            reads: seen(&s.reads),
            writes: seen(&s.writes),
            guessed: true,
            also: Vec::new(),
        });
        refined.added.push(name);
    }
    for old in &current.steps {
        if !steps.iter().any(|x| x.name == old.name) {
            steps.push(old.clone());
        }
    }
    for (i, s) in steps.iter_mut().enumerate() {
        s.order = i;
    }

    // The transitions: those the rules found, and the agent's between steps that are there.
    let mut transitions = current.transitions.clone();
    for t in &a.transitions {
        let (from, to) = (t.from.trim(), t.to.trim());
        let pos = |n: &str| steps.iter().position(|s| s.name == n);
        let (Some(i), Some(j)) = (pos(from), pos(to)) else {
            rejected.push(tr_in!(
                crate::i18n::ui(),
                "{who} : le passage « {from} » → « {to} » nomme une étape absente, refusé.",
                "{who}: the way “{from}” → “{to}” names a step that is not there, refused.",
            ));
            continue;
        };
        if let Some(old) = transitions
            .iter_mut()
            .find(|x| x.from == from && x.to == to)
        {
            // Already found: only its condition and maximum may be said when the rules had none.
            let (condition, max) = (old.condition.clone(), old.max);
            if old.condition.is_none() {
                old.condition = t
                    .condition
                    .as_deref()
                    .map(|c| clip(c, TEXT_MAX))
                    .filter(|c| !c.is_empty());
            }
            if old.max.is_none() {
                old.max = t.max.filter(|m| (1..=99).contains(m));
            }
            if (old.condition.clone(), old.max) != (condition, max) {
                refined.transitions += 1;
            }
            continue;
        }
        transitions.push(Transition {
            from: from.to_string(),
            to: to.to_string(),
            back: j <= i,
            condition: t
                .condition
                .as_deref()
                .map(|c| clip(c, TEXT_MAX))
                .filter(|c| !c.is_empty()),
            max: t.max.filter(|m| (1..=99).contains(m)),
        });
        refined.transitions += 1;
    }
    w.steps = steps;
    w.transitions = transitions;
    if w.exit.is_none() {
        w.exit = a
            .exit
            .as_deref()
            .map(|e| clip(e, TEXT_MAX))
            .filter(|e| !e.is_empty());
    }
    let changed = {
        let mut a = w.clone();
        a.refined = None;
        let mut b = current.clone();
        b.refined = None;
        a != b
    };
    if changed {
        w.refined = current
            .refined
            .clone()
            .or_else(|| Some(fingerprint_of(current)));
    }
    Checked {
        workflow: w,
        refined,
        rejected,
    }
}

impl Checked {
    pub fn changed(&self) -> bool {
        self.refined.refined_anything()
    }
}

impl Refined {
    fn refined_anything(&self) -> bool {
        !self.added.is_empty() || self.described > 0 || self.transitions > 0
    }
}

/// The checked workflows of an answer, one per workflow it gave that the project has; the rest
/// told as refused.
pub fn check_all(
    current: &[Workflow],
    answer: &Answer,
    known: &BTreeSet<String>,
) -> (Vec<Checked>, Vec<String>) {
    let (mut out, mut rejected) = (Vec::new(), Vec::new());
    for a in &answer.workflows {
        match current.iter().find(|w| w.id == a.id) {
            Some(w) if out.iter().all(|c: &Checked| c.workflow.id != w.id) => {
                let c = check(w, a, known);
                rejected.extend(c.rejected.iter().cloned());
                out.push(c);
            }
            Some(_) => {}
            None => {
                let id = &a.id;
                rejected.push(tr_in!(
                    crate::i18n::ui(),
                    "Le workflow « {id} » n'existe pas dans ce projet, refusé.",
                    "Workflow “{id}” does not exist in this project, refused.",
                ));
            }
        }
    }
    (out, rejected)
}

// ---------- on the core ----------

impl crate::core::Core {
    /// The workflows « Affiner… » works on: those of the project's files the rules read, as steps
    /// that name each other (a prompt run over and over has none to refine).
    fn refinable(&self, project_id: &str) -> Result<(crate::model::Project, Vec<Workflow>)> {
        let project = self.project(project_id)?;
        let places = crate::workflowfiles::places(&project, &self.data);
        let folder = crate::workflowfiles::load_all(&places);
        let list: Vec<Workflow> = folder
            .docs
            .iter()
            .filter(|(_, d)| !d.fixes.dismissed && d.workflow.kind == Kind::Cycle)
            .map(|(_, d)| d.workflow.clone())
            .collect();
        if list.is_empty() {
            bail!(tr!(
                "Aucun workflow à affiner : relis d'abord le projet.",
                "No workflow to refine: read the project again first."
            ));
        }
        Ok((project, list))
    }

    async fn refine_input(
        &self,
        project_id: &str,
    ) -> Result<(
        crate::model::Project,
        Vec<Workflow>,
        Vec<(String, String)>,
        BTreeSet<String>,
    )> {
        // Looked at again if a file changed, before the agent is given them.
        let scan = self.refresh_workflows(project_id, false).await?;
        let (project, list) = self.refinable(project_id)?;
        let (root, shown, sources) = (
            PathBuf::from(&project.path),
            list.clone(),
            scan.sources.clone(),
        );
        let files = tauri::async_runtime::spawn_blocking(move || {
            crate::workflow::files_in_cause(&root, &shown, &sources, &Roots::of_user())
        })
        .await?;
        // What a step added may stand on: the files the agent was given, no other.
        let known: BTreeSet<String> = files.iter().map(|(p, _)| p.clone()).collect();
        Ok((project, list, files, known))
    }

    /// What « Affiner… » will do, said before it is asked: the model, the account, the weight and
    /// the cost estimated. Refused when no account could answer.
    pub async fn workflow_refine_plan(&self, project_id: &str) -> Result<RefinePlan> {
        let (project, list, files, _) = self.refine_input(project_id).await?;
        let who = self.one_shot_who(&project.id).await?;
        let lang = self.lang().claude;
        let (system, prompt) = (system(lang), prompt(lang, &list, &files));
        let steps = list.iter().map(|w| w.steps.len()).sum();
        let (input_tokens, output_tokens, cost) = estimate(&system, &prompt, steps, &who.priced);
        Ok(RefinePlan {
            workflows: list.len(),
            files: files.len(),
            harness: serde_json::to_value(who.engine)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default(),
            model: who.model,
            account: who.account.name,
            account_id: who.account.id,
            input_tokens,
            output_tokens,
            cost,
            private: project.private,
        })
    }

    /// « Affiner… »: the agent is asked, its answer checked by the rules, and what stands written
    /// into the workflows' files. Nothing changes when the answer is not the schema.
    pub async fn refine_workflows(&self, project_id: &str) -> Result<Outcome> {
        // The agent may take its two minutes: the app does not restart for an update meanwhile.
        let _working = self.working();
        let (project, list, files, known) = self.refine_input(project_id).await?;
        let lang = self.lang().claude;
        let asked = self
            .one_shot_within(
                &system(lang),
                &prompt(lang, &list, &files),
                LIMIT,
                None,
                Some(&project.id),
            )
            .await?;
        let answer = parse(&asked).map_err(|why| {
            anyhow!(tr!(
                "L'agent n'a pas répondu dans le schéma attendu ({why}) : rien n'a changé.",
                "The agent did not answer in the expected schema ({why}): nothing changed."
            ))
        })?;
        let (checked, rejected) = check_all(&list, &answer, &known);
        let changed: Vec<&Checked> = checked.iter().filter(|c| c.changed()).collect();
        let places = crate::workflowfiles::places(&project, &self.data);
        let folder = crate::workflowfiles::load_all(&places);
        let written = crate::workflowfiles::set_workflows(
            &folder,
            &changed
                .iter()
                .map(|c| c.workflow.clone())
                .collect::<Vec<_>>(),
        );
        for (id, what) in &written {
            crate::workflow::mark_updated(
                project_id,
                id,
                &tr!(
                    "affiné par un agent ({what})",
                    "refined by an agent ({what})"
                ),
            );
        }
        let scan = self.refresh_workflows(project_id, false).await?;
        Ok(Outcome {
            scan,
            refined: changed.iter().map(|c| c.refined.clone()).collect(),
            rejected,
        })
    }
}
