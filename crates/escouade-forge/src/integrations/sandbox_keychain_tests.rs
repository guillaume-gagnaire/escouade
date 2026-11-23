use super::*;
use crate::integrations::secrets::keychain_service;

const DAY: Duration = MAX_AGE;

fn found(target: &str, comment: &str, written: SystemTime) -> Found {
    Found {
        target: target.into(),
        comment: comment.into(),
        written,
    }
}

#[test]
fn a_runs_entries_are_named_after_its_folder_and_end_with_the_sandbox() {
    let service = keychain_service(true);
    let a = Run::of(Path::new("C:/Temp/escouade-e2e/abc/data"));
    let again = Run::of(Path::new("C:/Temp/escouade-e2e/abc/data/"));
    let other = Run::of(Path::new("C:/Temp/escouade-e2e/xyz/data"));
    assert_eq!(a, again, "the same folder, the same run");
    assert_ne!(a.tag(), other.tag());
    let t = a.target(&service, "plugin:dev.test.fake:token#a1");
    assert_eq!(
        t,
        format!(
            "plugin:dev.test.fake:token#a1.run-{}.dev.gagnaire.escouade.sandbox",
            a.tag()
        )
    );
    assert!(t.ends_with(SUFFIX));
    assert_eq!(tag_of(&t), Some(a.tag()));
    assert!(a.comment().starts_with(MARK));
    // A folder too long for the comment: the mark alone, swept by age.
    let long = Run::of(&PathBuf::from("C:/").join("d".repeat(300)));
    assert_eq!(long.comment(), MARK);
}

#[test]
fn only_a_runs_entries_under_the_sandbox_are_ever_touched() {
    let tag = "0123456789abcdef";
    assert!(ours(&format!(
        "jira.run-{tag}.dev.gagnaire.escouade.sandbox"
    )));
    for not in [
        // The user's own entries.
        "jira.dev.gagnaire.escouade".to_string(),
        format!("jira.run-{tag}.dev.gagnaire.escouade"),
        // A sandbox's from before ESC-584: the user's to clear (a-18).
        "plugin:dev.test.fake:token.dev.gagnaire.escouade.sandbox".to_string(),
        // Not ending with the sandbox.
        format!("jira.run-{tag}.dev.gagnaire.escouade.sandbox.x"),
        format!("jira.run-{tag}.escouade.sandboxed"),
        // A tag that is not one.
        "jira.run-0123.dev.gagnaire.escouade.sandbox".to_string(),
        "jira.run-0123456789ABCDEF.dev.gagnaire.escouade.sandbox".to_string(),
        "jira.run-0123456789abcdefg.dev.gagnaire.escouade.sandbox".to_string(),
        "git:https://github.com".to_string(),
        String::new(),
    ] {
        assert!(!ours(&not), "{not}");
        let f = found(&not, &format!("{MARK}C:/gone"), SystemTime::UNIX_EPOCH);
        assert!(!f.dead(SystemTime::now()), "{not}");
    }
}

#[test]
fn a_dead_run_is_one_whose_folder_is_gone_or_older_than_a_day() {
    let now = SystemTime::now();
    let live = crate::paths::test_dir("sandbox-keychain-live");
    let gone = live.join("gone");
    let t = |run: &Run| run.target(&keychain_service(true), "jira");
    let (l, g) = (Run::of(&live), Run::of(&gone));
    let list = [
        found(&t(&l), &l.comment(), now),
        found(&t(&g), &g.comment(), now),
        found(&t(&l), &l.comment(), now - DAY - Duration::from_secs(60)),
        found(&t(&l), MARK, now),
        found(&t(&l), MARK, now - DAY - Duration::from_secs(60)),
        // Written by keyring (no run's comment): never swept.
        found(&t(&g), "keyring v3.6.3", now - DAY * 30),
    ];
    let dead: Vec<bool> = list.iter().map(|f| f.dead(now)).collect();
    assert_eq!(dead, [false, true, true, false, true, false]);
    assert_eq!(of_dead(&list, now).len(), 3);
    assert_eq!(of_runs(&list, std::slice::from_ref(&g)).len(), 2);
}

#[test]
fn the_command_line_is_the_fixtures_only() {
    let s = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(command(&s(&[])), None);
    assert_eq!(command(&s(&["escouade://plugin/x"])), None);
    for bad in [
        &["--sandbox-keychain"][..],
        &["--sandbox-keychain", "clean"],
        &["--sandbox-keychain", "sweep", "x"],
        &["--sandbox-keychain", "wipe"],
    ] {
        assert_eq!(command(&s(bad)).map(|(_, c)| c), Some(2), "{bad:?}");
    }
    let dir = crate::paths::test_dir("sandbox-keychain-cli");
    let d = dir.to_string_lossy().to_string();
    assert_eq!(
        command(&s(&["--sandbox-keychain", "list", &d])),
        Some((String::new(), 0))
    );
    assert_eq!(
        command(&s(&["--sandbox-keychain", "clean", &d])),
        Some(("0".into(), 0))
    );
}

/// The real keychain of the machine (Windows' Credential Manager): what a run writes, its end takes back; what a dead run
/// left, the sweep takes back, and nothing of a live run. Only entries of runs of their own.
#[cfg(windows)]
mod real {
    use super::*;
    use crate::integrations::secrets::{Chunked, SecretStore};
    use std::sync::Arc;

    /// A store of the sandbox on the run of `dir`, as a sandboxed app has.
    fn store(dir: &Path) -> Chunked {
        Chunked::new(Arc::new(crate::integrations::secrets::sandbox_store_of(
            dir,
        )))
    }

    /// Whatever happens, the runs' entries go.
    struct Clean(Vec<PathBuf>);

    impl Drop for Clean {
        fn drop(&mut self) {
            let _ = clean(&self.0);
        }
    }

    #[test]
    fn a_run_leaves_the_keychain_as_it_found_it() {
        let dir = crate::paths::test_dir("sandbox-keychain-run");
        let _clean = Clean(vec![dir.clone()]);
        let before: Vec<String> = list().unwrap().into_iter().map(|f| f.target).collect();
        assert!(entries_of(std::slice::from_ref(&dir)).unwrap().is_empty());
        let s = store(&dir);
        s.set("plugin:dev.test.esc584:token", "short-trial-value")
            .unwrap();
        // Short: a keychain filled by the runs before ESC-584 refuses a long one (Windows error code 8).
        s.set("plugin:dev.test.esc584:password", "other-trial-value")
            .unwrap();
        assert_eq!(
            s.get("plugin:dev.test.esc584:token").unwrap().as_deref(),
            Some("short-trial-value")
        );
        let written = entries_of(std::slice::from_ref(&dir)).unwrap();
        assert_eq!(written.len(), 2, "{written:?}");
        assert!(written.iter().all(|t| ours(t)), "{written:?}");
        assert!(written.iter().all(|t| !before.contains(t)));
        assert_eq!(clean(std::slice::from_ref(&dir)).unwrap(), written.len());
        assert!(entries_of(std::slice::from_ref(&dir)).unwrap().is_empty());
        let after: Vec<String> = list().unwrap().into_iter().map(|f| f.target).collect();
        assert!(after.iter().all(|t| !written.contains(t)));
        // What was there before and is not ours stays: only another run may take it back meanwhile.
        for t in before.iter().filter(|t| !ours(t)) {
            assert!(after.contains(t), "{t} went");
        }
    }

    #[test]
    fn the_sweep_takes_back_a_dead_runs_entries_and_none_of_a_live_one() {
        let live = crate::paths::test_dir("sandbox-keychain-sweep-live");
        let dead = crate::paths::test_dir("sandbox-keychain-sweep-dead");
        let _clean = Clean(vec![live.clone(), dead.clone()]);
        store(&live).set("jira", "live-trial-value").unwrap();
        store(&dead).set("jira", "dead-trial-value").unwrap();
        std::fs::remove_dir_all(&dead).unwrap();
        assert_eq!(entries_of(std::slice::from_ref(&dead)).unwrap().len(), 1);
        assert!(sweep(SystemTime::now()).unwrap() >= 1);
        assert!(entries_of(std::slice::from_ref(&dead)).unwrap().is_empty());
        assert_eq!(
            store(&live).get("jira").unwrap().as_deref(),
            Some("live-trial-value")
        );
        assert_eq!(clean(std::slice::from_ref(&live)).unwrap(), 1);
    }
}
