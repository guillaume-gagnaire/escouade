//! The user's corrections of a project's workflows (ESC-195, LOOP-4 of ESC-191): Réglages ›
//! Projets › Workflow renames, merges, removes, reorders the steps the reading found, adds one it
//! missed, keeps or removes one it guessed, or says « Ce n'est pas un workflow ». They go ahead of
//! the reading, read again or not; one that no longer matches anything is left out (the window
//! says so). Kept in the files of the workflows, beside what was read (`workflowfiles`).

use crate::workflow::{Scan, Step, StepAgent, Transition, Workflow};

// The corrections as a project keeps them, in `escouade-model` (docs/CRATES.md).
pub use escouade_model::workflowfix::{Fix, FixStep, Fixes, Signal};

/// The corrections as kept: trimmed, without those that change nothing, one per workflow (the
/// last one written).
pub fn cleaned(fixes: Fixes) -> Fixes {
    let mut out: Fixes = Vec::new();
    for mut f in fixes {
        f.workflow = f.workflow.trim().to_string();
        for s in &mut f.steps {
            s.name = s.name.trim().to_string();
            s.signals.retain(|g| !signal_value(g).trim().is_empty());
        }
        f.steps
            .retain(|s| !s.from.is_empty() || !s.signals.is_empty() || !s.name.is_empty());
        if f.workflow.is_empty() || f.is_empty() {
            continue;
        }
        out.retain(|o| o.workflow != f.workflow);
        out.push(f);
    }
    out
}

fn signal_value(s: &Signal) -> &str {
    match s {
        Signal::Command(v) | Signal::Agent(v) | Signal::File(v) => v,
    }
}

/// A step's name in the corrected workflow: the user's, else that of the first step it stands for.
fn shown(s: &FixStep) -> String {
    if s.name.is_empty() {
        s.from.first().cloned().unwrap_or_default()
    } else {
        s.name.clone()
    }
}

/// The workflow as the user corrected it; None when it is « pas un workflow ».
pub fn apply(w: &Workflow, fix: Option<&Fix>) -> Option<Workflow> {
    let Some(fix) = fix.filter(|f| !f.is_empty()) else {
        return Some(w.clone());
    };
    if fix.dismissed {
        return None;
    }
    let mut out = w.clone();
    let of = |name: &str| w.steps.iter().find(|s| s.name == name);
    // The reading's step → the corrected one's name.
    let mut renamed: Vec<(String, String)> = Vec::new();
    let mut steps: Vec<Step> = Vec::new();
    let listed: Vec<&str> = fix
        .steps
        .iter()
        .flat_map(|s| s.from.iter().map(String::as_str))
        .collect();
    for s in &fix.steps {
        let found: Vec<&Step> = s.from.iter().filter_map(|n| of(n)).collect();
        let name = shown(s);
        // A correction of steps the reading no longer has, and that says nothing else: gone.
        if found.is_empty() && !s.from.is_empty() && s.signals.is_empty() {
            continue;
        }
        let mut step = found.first().map(|f| (*f).clone()).unwrap_or(Step {
            name: String::new(),
            command: String::new(),
            order: 0,
            description: String::new(),
            path: String::new(),
            agents: Vec::new(),
            reads: Vec::new(),
            writes: Vec::new(),
            guessed: false,
            also: Vec::new(),
        });
        for more in found.iter().skip(1) {
            // What signalled the step merged in still signals this one.
            for w in [&more.command, &more.name] {
                if !w.is_empty() && !step.also.contains(w) {
                    step.also.push(w.clone());
                }
            }
            for a in &more.agents {
                if !step.agents.iter().any(|x| x.name == a.name) {
                    step.agents.push(a.clone());
                }
            }
            for r in &more.reads {
                if !step.reads.contains(r) {
                    step.reads.push(r.clone());
                }
            }
            for r in &more.writes {
                if !step.writes.contains(r) {
                    step.writes.push(r.clone());
                }
            }
        }
        for g in &s.signals {
            match g {
                Signal::Command(c) if step.command.is_empty() => step.command = c.clone(),
                Signal::Command(c) => {
                    if !step.also.contains(c) {
                        step.also.push(c.clone())
                    }
                }
                Signal::Agent(a) => {
                    if !step.agents.iter().any(|x| &x.name == a) {
                        step.agents.push(StepAgent {
                            name: a.clone(),
                            path: None,
                            description: String::new(),
                        });
                    }
                }
                Signal::File(f) => {
                    if !step.writes.contains(f) {
                        step.writes.push(f.clone());
                    }
                }
            }
        }
        // Listed by the user: kept, guessed or not.
        step.guessed = false;
        for f in &found {
            renamed.push((f.name.clone(), name.clone()));
        }
        // Renamed, its name as read still signals it (`/gsd:plan-phase`, a skill by its name).
        if let Some(first) = found.first() {
            if first.name != name && !step.also.contains(&first.name) {
                step.also.push(first.name.clone());
            }
        }
        step.name = name;
        steps.push(step);
    }
    // What the reading found since, neither listed nor taken out: after, as it found it.
    for s in &w.steps {
        if !listed.contains(&s.name.as_str()) && !fix.removed.contains(&s.name) {
            renamed.push((s.name.clone(), s.name.clone()));
            steps.push(s.clone());
        }
    }
    for (i, s) in steps.iter_mut().enumerate() {
        s.order = i;
    }
    let to = |n: &str| renamed.iter().find(|(a, _)| a == n).map(|(_, b)| b.clone());
    let mut transitions: Vec<Transition> = Vec::new();
    for t in &w.transitions {
        let (Some(from), Some(dest)) = (to(&t.from), to(&t.to)) else {
            continue;
        };
        // Two steps merged: the way from one to the other is inside it now.
        if from == dest && !t.back {
            continue;
        }
        let pos = |n: &str| steps.iter().position(|s| s.name == n);
        let t = Transition {
            back: t.back || pos(&dest) <= pos(&from) && from != dest,
            from,
            to: dest,
            condition: t.condition.clone(),
            max: t.max,
        };
        if !transitions.contains(&t) {
            transitions.push(t);
        }
    }
    // The doors that run several steps enter them by their corrected names.
    for d in &mut out.doors {
        let mut enters: Vec<String> = Vec::new();
        for e in &d.enters {
            if let Some(n) = to(e) {
                if !enters.contains(&n) {
                    enters.push(n);
                }
            }
        }
        d.enters = enters;
    }
    out.steps = steps;
    out.transitions = transitions;
    Some(out)
}

/// A reading with its workflows as the user corrected them: what the tracking of the agents
/// follows (`workflow_track`).
pub fn corrected(scan: &Scan, fixes: &[Fix]) -> Scan {
    if fixes.is_empty() {
        return scan.clone();
    }
    Scan {
        workflows: effective(&scan.workflows, fixes),
        ..scan.clone()
    }
}

/// The workflows of a reading as corrected.
pub fn effective(workflows: &[Workflow], fixes: &[Fix]) -> Vec<Workflow> {
    workflows
        .iter()
        .filter_map(|w| apply(w, fixes.iter().find(|f| f.workflow == w.id)))
        .collect()
}
