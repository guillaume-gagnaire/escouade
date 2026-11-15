use super::*;
use crate::{Policies, Source};
use serde_json::json;
use std::sync::mpsc;
use std::sync::Mutex;

fn policies(values: &[(&str, serde_json::Value)]) -> Policies {
    Policies::from_values(
        values.iter().map(|(k, v)| ((*k).to_owned(), v.clone())),
        Source::Machine,
    )
}

/// A watch on a source of the test's: what it reads, the sign to give, what it called back with.
struct Fixture {
    source: Arc<Mutex<Policies>>,
    sign: mpsc::Sender<()>,
    seen: mpsc::Receiver<Policies>,
    _watch: Watch,
}

fn fixture(period: Duration) -> Fixture {
    let source = Arc::new(Mutex::new(Policies::default()));
    let (sign, rx) = mpsc::channel();
    let (tell, seen) = mpsc::channel();
    let read = {
        let source = source.clone();
        move || source.lock().unwrap().clone()
    };
    let watch = spawn(
        read,
        Box::new(Channel(rx)),
        Policies::default(),
        period,
        move |p| {
            let _ = tell.send(p);
        },
    );
    Fixture {
        source,
        sign,
        seen,
        _watch: watch,
    }
}

#[test]
fn a_policy_changed_while_it_runs_is_told_at_the_sign_with_its_type() {
    let f = fixture(Duration::from_secs(60));
    *f.source.lock().unwrap() = policies(&[("telemetry.enabled", json!(0))]);
    f.sign.send(()).unwrap();
    let got = f.seen.recv_timeout(Duration::from_secs(5)).expect("told");
    assert_eq!(got.get("telemetry.enabled").unwrap().value, json!(false));

    // Removed: told again, with nothing.
    *f.source.lock().unwrap() = Policies::default();
    f.sign.send(()).unwrap();
    assert!(f
        .seen
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .is_empty());
}

#[test]
fn a_sign_with_nothing_changed_tells_nothing() {
    let f = fixture(Duration::from_secs(60));
    f.sign.send(()).unwrap();
    f.sign.send(()).unwrap();
    assert!(f.seen.recv_timeout(Duration::from_millis(400)).is_err());
}

#[test]
fn a_change_with_no_sign_is_seen_at_the_period() {
    let f = fixture(Duration::from_millis(100));
    *f.source.lock().unwrap() = policies(&[("updates.channel", json!("stable"))]);
    let got = f.seen.recv_timeout(Duration::from_secs(5)).expect("told");
    assert_eq!(got.get("updates.channel").unwrap().value, json!("stable"));
}

#[test]
fn a_watch_dropped_tells_no_more() {
    let f = fixture(Duration::from_secs(60));
    let Fixture {
        source,
        sign,
        seen,
        _watch,
    } = f;
    drop(_watch);
    *source.lock().unwrap() = policies(&[("telemetry.enabled", json!(false))]);
    let _ = sign.send(());
    assert!(seen.recv_timeout(Duration::from_millis(400)).is_err());
}

#[test]
fn a_file_written_removed_or_a_folder_that_appears_is_a_sign() {
    let dir = tempfile::tempdir().unwrap();
    let etc = dir.path().join("escouade");
    let mut poll = Poll::new(vec![etc.clone()], Duration::from_millis(20));
    assert!(!poll.wait(Duration::from_millis(100)), "nothing yet");
    std::fs::create_dir_all(etc.join("policies")).unwrap();
    assert!(poll.wait(Duration::from_secs(2)), "the folder appeared");
    std::fs::write(etc.join("policies/10-a.json"), "{}").unwrap();
    assert!(poll.wait(Duration::from_secs(2)), "a file written");
    std::fs::write(
        etc.join("policies/10-a.json"),
        r#"{"telemetry.enabled": false}"#,
    )
    .unwrap();
    assert!(poll.wait(Duration::from_secs(2)), "a file changed");
    std::fs::remove_file(etc.join("policies/10-a.json")).unwrap();
    assert!(poll.wait(Duration::from_secs(2)), "a file removed");
    assert!(!poll.wait(Duration::from_millis(100)), "then nothing");
}

#[test]
fn the_system_watch_starts_and_stops_on_this_device() {
    // Whatever this machine holds, a watch of its own place starts and stops without a panic.
    let watch = system_watch(crate::read(), Duration::from_millis(50), |_| {});
    std::thread::sleep(Duration::from_millis(120));
    drop(watch);
}

#[cfg(windows)]
mod windows {
    use super::*;
    use crate::windows::{read_registry, Location};
    use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};
    use winreg::RegKey;

    /// A real key under `HKCU\Software\` (not `Policies`), deleted when dropped.
    struct TestKey(String);

    impl TestKey {
        fn new(name: &str) -> (TestKey, RegKey) {
            let path = format!(
                r"Software\EscouadePolicyWatch-{}-{name}",
                std::process::id()
            );
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            let _ = hkcu.delete_subkey_all(&path);
            let (key, _) = hkcu
                .create_subkey_with_flags(&path, KEY_ALL_ACCESS)
                .unwrap();
            (TestKey(path), key)
        }
    }

    impl Drop for TestKey {
        fn drop(&mut self) {
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.0);
        }
    }

    #[test]
    fn the_registry_tells_a_value_written_and_a_list_subkey_created() {
        let (test, key) = TestKey::new("notify");
        let mut reg = registry::Registry::of(&[(
            windows_sys::Win32::System::Registry::HKEY_CURRENT_USER,
            test.0.as_str(),
        )]);
        assert_eq!(reg.len(), 1);
        assert!(!reg.wait(Duration::from_millis(100)), "nothing yet");
        // Armed by that wait: a value written now is told at the next.
        key.set_value("telemetry.enabled", &0u32).unwrap();
        assert!(reg.wait(Duration::from_secs(5)), "a value written");
        // A subkey's value (an ADMX list) is told too: the subtree is watched.
        let (list, _) = key.create_subkey("permissions.modes").unwrap();
        assert!(reg.wait(Duration::from_secs(5)), "a list created");
        list.set_value("1", &"plan").unwrap();
        assert!(reg.wait(Duration::from_secs(5)), "an item of a list");
        assert!(!reg.wait(Duration::from_millis(200)), "then nothing");
    }

    #[test]
    fn a_policy_written_in_the_registry_while_it_runs_is_read_again_and_typed() {
        let (test, key) = TestKey::new("live");
        let path = test.0.clone();
        let read = move || {
            let at = Location {
                hive: HKEY_CURRENT_USER,
                path: &path,
            };
            let absent = Location {
                hive: HKEY_CURRENT_USER,
                path: r"Software\EscouadePolicyWatch-absent",
            };
            read_registry(at, absent, false, &crate::known_keys()).typed()
        };
        let first = read();
        assert!(first.is_empty());
        let (tell, seen) = mpsc::channel();
        let reg = registry::Registry::of(&[(
            windows_sys::Win32::System::Registry::HKEY_CURRENT_USER,
            test.0.as_str(),
        )]);
        let _watch = spawn(
            read,
            Box::new(reg),
            first,
            Duration::from_secs(60),
            move |p| {
                let _ = tell.send(p);
            },
        );
        // The thread arms its notification at once; leave it the time to.
        std::thread::sleep(Duration::from_millis(200));
        key.set_value("updates.automatic", &0u32).unwrap();
        let got = seen.recv_timeout(Duration::from_secs(5)).expect("told");
        // A REG_DWORD 0 the list says Boolean: false.
        assert_eq!(got.get("updates.automatic").unwrap().value, json!(false));
        assert_eq!(
            got.get("updates.automatic").unwrap().source,
            Source::Machine
        );
    }
}
