//! The experimental features of Claude Code (ESC-553): what the settings ask, what each launch of the interface gets, and
//! why what is not passed is not.

use super::*;
use serde_json::json;

fn set(v: Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}

fn params() -> TerminalParams {
    TerminalParams {
        agent: "tui-a1".into(),
        cwd: "/x/repo".into(),
        model: "sonnet".into(),
        mode: "default".into(),
        lang: "fr".into(),
        private_file: "/data/tui/a1.json".into(),
        ..TerminalParams::default()
    }
}

fn context(settings: Value) -> Context {
    Context {
        settings: set(settings),
        version: Some("2.1.329".into()),
        ..Context::default()
    }
}

#[test]
fn nothing_is_asked_by_default() {
    assert_eq!(wanted(&Map::new()), Wanted::default());
    let j = judge(&params(), &Context::default());
    assert!(j.env.is_empty() && j.warns.is_empty(), "{j:?}");
}

#[test]
fn every_project_then_each_one_decides() {
    let all = json!({ key::CODE_MODE: true, key::AGENT_TEAMS: true });
    let w = wanted(&set(all.clone()));
    assert!(w.code_mode && w.agent_teams);
    // A project turns one off, the other on.
    let mut here = all.clone();
    here[key::HERE_CODE_MODE] = json!("off");
    assert!(!wanted(&set(here.clone())).code_mode);
    here[key::HERE_AGENT_TEAMS] = json!("inherit");
    assert!(wanted(&set(here)).agent_teams);
    let only_here = json!({ key::HERE_CODE_MODE: "on" });
    assert!(wanted(&set(only_here)).code_mode);
}

#[test]
fn the_others_are_names_of_claude_code_experimental_alone() {
    let w = wanted(&set(json!({
        key::OTHERS: "CLAUDE_CODE_EXPERIMENTAL_FOO, CLAUDE_CODE_EXPERIMENTAL_BAR_2;CLAUDE_CODE_EXPERIMENTAL_FOO \
                      ANTHROPIC_BASE_URL claude_code_experimental_low CLAUDE_CODE_EXPERIMENTAL_ CLAUDE_CODE_EXPERIMENTAL_X=1 \
                      CLAUDE_CODE_EXPERIMENTAL_CODE_MODE PATH",
    })));
    assert_eq!(
        w.others,
        [
            "CLAUDE_CODE_EXPERIMENTAL_FOO",
            "CLAUDE_CODE_EXPERIMENTAL_BAR_2"
        ]
    );
    // Never another variable, nor a value, nor a flag that has its own switch (it would pass its checks by).
    assert_eq!(
        w.refused,
        [
            "ANTHROPIC_BASE_URL",
            "claude_code_experimental_low",
            "CLAUDE_CODE_EXPERIMENTAL_",
            "CLAUDE_CODE_EXPERIMENTAL_X=1",
            "CLAUDE_CODE_EXPERIMENTAL_CODE_MODE",
            "PATH",
        ]
    );
    assert!(!w.code_mode);
    let j = judge(
        &params(),
        &context(json!({ key::OTHERS: "CLAUDE_CODE_EXPERIMENTAL_FOO PATH" })),
    );
    assert_eq!(j.env, ["CLAUDE_CODE_EXPERIMENTAL_FOO"]);
    assert_eq!(
        j.warns,
        ["Pas passés à l’interface : PATH (seuls des noms de CLAUDE_CODE_EXPERIMENTAL_* le sont, et le Code Mode et les Agent Teams ont leur interrupteur)."]
    );
}

#[test]
fn a_project_replaces_the_others_or_takes_none() {
    let all = json!({ key::OTHERS: "CLAUDE_CODE_EXPERIMENTAL_FOO" });
    assert_eq!(
        wanted(&set(all.clone())).others,
        ["CLAUDE_CODE_EXPERIMENTAL_FOO"]
    );
    let mut here = all.clone();
    here[key::HERE_OTHERS] = json!("CLAUDE_CODE_EXPERIMENTAL_BAR");
    assert_eq!(
        wanted(&set(here.clone())).others,
        ["CLAUDE_CODE_EXPERIMENTAL_BAR"]
    );
    for none in ["aucun", "none", "-", "Aucun"] {
        here[key::HERE_OTHERS] = json!(none);
        assert!(wanted(&set(here.clone())).others.is_empty(), "{none}");
    }
    // Empty: those of every project.
    here[key::HERE_OTHERS] = json!("  ");
    assert_eq!(wanted(&set(here)).others, ["CLAUDE_CODE_EXPERIMENTAL_FOO"]);
}

#[test]
fn code_mode_and_the_agent_teams_go_to_the_interface() {
    let j = judge(
        &params(),
        &context(json!({ key::CODE_MODE: true, key::AGENT_TEAMS: true })),
    );
    assert_eq!(j.env, [CODE_MODE, AGENT_TEAMS]);
    assert!(j.warns.is_empty(), "{j:?}");
    assert_eq!(
        env_block(&j.env),
        Some(json!({ CODE_MODE: "1", AGENT_TEAMS: "1" }))
    );
    assert_eq!(env_block(&[]), None);
}

#[test]
fn code_mode_is_not_passed_where_claude_code_refuses_it_and_the_conversation_says_why() {
    let on = json!({ key::CODE_MODE: true });
    let why = |p: TerminalParams, c: Context| {
        let j = judge(&p, &c);
        assert!(!j.env.iter().any(|v| v == CODE_MODE), "{j:?}");
        j.warns.join(" ")
    };
    let remote = why(
        TerminalParams {
            remote: Some("demo · a1".into()),
            ..params()
        },
        context(on.clone()),
    );
    assert_eq!(remote, "Code Mode non passé à l’interface : le remote control est demandé pour cet agent, et Claude Code ne prend pas les deux.");
    let gateway = why(
        params(),
        Context {
            gateway: Some("ANTHROPIC_BASE_URL".into()),
            ..context(on.clone())
        },
    );
    assert!(
        gateway.contains(
            "le compte passe par ANTHROPIC_BASE_URL (une passerelle ou un fournisseur cloud)"
        ),
        "{gateway}"
    );
    let old = why(
        params(),
        Context {
            version: Some("2.1.322".into()),
            ..context(on.clone())
        },
    );
    assert!(
        old.contains("Claude Code 2.1.322 est trop ancien (il faut la 2.1.323 au moins)"),
        "{old}"
    );
    // With the agent teams, 2.1.326; the teams themselves still go.
    let teams = judge(
        &params(),
        &Context {
            version: Some("2.1.325".into()),
            ..context(json!({ key::CODE_MODE: true, key::AGENT_TEAMS: true }))
        },
    );
    assert_eq!(teams.env, [AGENT_TEAMS]);
    assert!(
        teams.warns[0].contains("il faut la 2.1.326 au moins avec les Agent Teams"),
        "{teams:?}"
    );
    let model = why(
        TerminalParams {
            model: "claude-opus-4-8".into(),
            ..params()
        },
        context(on.clone()),
    );
    assert!(
        model.contains("le modèle claude-opus-4-8 ne le prend pas"),
        "{model}"
    );
    let discussion = why(
        TerminalParams {
            read_only: true,
            ..params()
        },
        context(on.clone()),
    );
    assert!(discussion.contains("une discussion"), "{discussion}");
    let en = why(
        TerminalParams {
            remote: Some("x".into()),
            lang: "en".into(),
            ..params()
        },
        context(on),
    );
    assert_eq!(en, "Code Mode not passed to the interface: Remote Control is asked for this agent, and Claude Code does not take both.");
}

#[test]
fn a_session_started_in_code_mode_always_resumes_with_it() {
    // Turned off since, or never on here: its session resumes only so (« This session was started in Code Mode
    // (experimental), which is off, so it can't be resumed. », 2.1.329).
    let j = judge(
        &params(),
        &Context {
            terminal_only: true,
            ..context(json!({ key::CODE_MODE: false }))
        },
    );
    assert_eq!(j.env, [CODE_MODE]);
    assert!(j.warns.is_empty());
}

#[test]
fn an_unknown_version_is_left_to_claude_code() {
    let j = judge(
        &params(),
        &Context {
            version: None,
            ..context(json!({ key::CODE_MODE: true }))
        },
    );
    assert_eq!(j.env, [CODE_MODE]);
    assert!(
        older("2.1.322", "2.1.323") && !older("2.1.323", "2.1.323") && !older("2.1.330", "2.1.323")
    );
    assert!(older("2.1.9", "2.1.32") && !older("x", "2.1.32"));
    assert!(!older("2.1.329 (Claude Code)", CODE_MODE_MIN));
}

#[test]
fn the_models_code_mode_takes() {
    for m in [
        "opus",
        "sonnet",
        "haiku",
        "fable",
        "opus[1m]",
        "claude-opus-5-5",
        "claude-sonnet-5-5",
        "claude-haiku-5-5",
        "claude-fable-5-2",
        "claude-opus-6",
        "claude-sonnet-5-5-20261101",
        "something-else",
    ] {
        assert!(takes_model(m), "{m}");
    }
    for m in [
        "claude-opus-4-8",
        "claude-sonnet-5",
        "claude-haiku-4-5",
        "claude-fable-5-1",
        "claude-opus-5-1[1m]",
    ] {
        assert!(!takes_model(m), "{m}");
    }
}
