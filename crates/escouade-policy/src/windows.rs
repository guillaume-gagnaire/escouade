//! Windows: the `Policies` registry keys, as Group Policy and Intune write them from an ADMX.
//!
//! - `HKLM\SOFTWARE\Policies\Escouade` (the machine), then `HKCU\SOFTWARE\Policies\Escouade` (the user); for a same
//!   key, the machine wins.
//! - A value's name is the policy's key (`permissions.allowBypass`). `REG_DWORD` and `REG_QWORD` give a number (an ADMX
//!   boolean is a `REG_DWORD` 0 or 1: the list of TS-12 will say which keys are booleans), `REG_SZ` a string, or JSON
//!   when it opens an object or an array (a complex setting), `REG_MULTI_SZ` a list of strings.
//! - A list the ADMX way is a subkey named by the key, whose values are named `1`, `2`, …: a list in that order.
//! - Both keys are written by an administrator alone (their ACL gives users read only, README.md). A key this process
//!   could write without being elevated has had its ACL loosened: it is refused, and said.
//!
//! The reading of a key is apart from the registry ([`Node`]), so that the tests need no administrator: a test key
//! under `HKCU\Software\` stands for the real ones ([`read_registry`]).

use serde_json::Value;

use crate::{string_value, Policies, Source, Warning};

/// The policy keys' path, under HKLM and under HKCU.
pub const POLICY_PATH: &str = r"SOFTWARE\Policies\Escouade";

/// The type of a registry value, as Windows numbers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    None,
    Sz,
    ExpandSz,
    Binary,
    Dword,
    DwordBigEndian,
    Link,
    MultiSz,
    Qword,
    Other(u32),
}

/// A registry value: its type and its bytes, as they are stored.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawValue {
    pub kind: Kind,
    pub bytes: Vec<u8>,
}

/// A key, read: its values and its subkeys, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Node {
    pub values: Vec<(String, RawValue)>,
    pub subkeys: Vec<(String, Node)>,
}

/// The policies of the machine's key and the user's, either absent. The user's go first, the machine's over them.
pub fn from_nodes(machine: Option<&Node>, user: Option<&Node>, known: &[&str]) -> Policies {
    let mut policies = Policies::default();
    for (node, source) in [(user, Source::User), (machine, Source::Machine)] {
        if let Some(node) = node {
            read_node(node, source, known, &mut policies);
        }
    }
    policies
}

fn read_node(node: &Node, source: Source, known: &[&str], policies: &mut Policies) {
    for (name, raw) in &node.values {
        if name.is_empty() {
            // The key's default value: no policy is named so.
            continue;
        }
        match value_of(raw) {
            Ok(value) => policies.set(name, value, source.clone(), known),
            Err(reason) => policies.warn(Warning::Malformed {
                key: name.clone(),
                source: source.clone(),
                reason,
            }),
        }
    }
    for (name, list) in &node.subkeys {
        if node.values.iter().any(|(value, _)| value == name) {
            policies.warn(Warning::Malformed {
                key: name.clone(),
                source: source.clone(),
                reason: "both a value and a list subkey: the list is kept".to_owned(),
            });
        }
        let value = list_of(name, list, &source, policies);
        policies.set(name, value, source.clone(), known);
    }
}

/// A list the ADMX way: the values `1`, `2`, … in their numbers' order. Anything else in it is said and skipped.
fn list_of(key: &str, list: &Node, source: &Source, policies: &mut Policies) -> Value {
    let mut items = Vec::new();
    for (name, raw) in &list.values {
        let Ok(index) = name.parse::<u64>() else {
            policies.warn(Warning::Malformed {
                key: format!("{key}\\{name}"),
                source: source.clone(),
                reason: "an item of a list is named by its number (1, 2, …)".to_owned(),
            });
            continue;
        };
        match value_of(raw) {
            Ok(value) => items.push((index, value)),
            Err(reason) => policies.warn(Warning::Malformed {
                key: format!("{key}\\{name}"),
                source: source.clone(),
                reason,
            }),
        }
    }
    for (name, _) in &list.subkeys {
        policies.warn(Warning::Malformed {
            key: format!("{key}\\{name}"),
            source: source.clone(),
            reason: "a list holds values, not subkeys".to_owned(),
        });
    }
    items.sort_by_key(|(index, _)| *index);
    Value::Array(items.into_iter().map(|(_, value)| value).collect())
}

/// A registry value as JSON, or why it cannot be.
pub fn value_of(raw: &RawValue) -> Result<Value, String> {
    match raw.kind {
        Kind::Dword => {
            let bytes: [u8; 4] = raw
                .bytes
                .as_slice()
                .try_into()
                .map_err(|_| bad_length("REG_DWORD", 4, raw))?;
            Ok(Value::from(u32::from_le_bytes(bytes)))
        }
        Kind::DwordBigEndian => {
            let bytes: [u8; 4] = raw
                .bytes
                .as_slice()
                .try_into()
                .map_err(|_| bad_length("REG_DWORD_BIG_ENDIAN", 4, raw))?;
            Ok(Value::from(u32::from_be_bytes(bytes)))
        }
        Kind::Qword => {
            let bytes: [u8; 8] = raw
                .bytes
                .as_slice()
                .try_into()
                .map_err(|_| bad_length("REG_QWORD", 8, raw))?;
            Ok(Value::from(u64::from_le_bytes(bytes)))
        }
        // REG_EXPAND_SZ is kept as written: no %VARIABLE% of the user is expanded into a policy.
        Kind::Sz | Kind::ExpandSz => string_value(&utf16(&raw.bytes)?),
        Kind::MultiSz => {
            let text = utf16(&raw.bytes)?;
            // Strings ended by NUL, the list by an empty one: everything up to the first empty string.
            let items = text.split('\0').take_while(|item| !item.is_empty());
            Ok(Value::Array(
                items.map(|item| Value::String(item.to_owned())).collect(),
            ))
        }
        Kind::None | Kind::Binary | Kind::Link => {
            Err(format!("type {:?} is not a policy's", raw.kind))
        }
        Kind::Other(n) => Err(format!("registry type {n} is not a policy's")),
    }
}

fn bad_length(kind: &str, expected: usize, raw: &RawValue) -> String {
    format!("a {kind} holds {expected} bytes, not {}", raw.bytes.len())
}

/// UTF-16 little endian, its final NULs dropped (a REG_SZ may or may not end with one).
fn utf16(bytes: &[u8]) -> Result<String, String> {
    let (pairs, rest) = bytes.as_chunks::<2>();
    if !rest.is_empty() {
        return Err("an odd number of bytes is not UTF-16".to_owned());
    }
    let mut units: Vec<u16> = pairs.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
    while units.last() == Some(&0) {
        units.pop();
    }
    String::from_utf16(&units).map_err(|_| "not valid UTF-16".to_owned())
}

#[cfg(windows)]
pub use system::*;

#[cfg(windows)]
mod system {
    use std::io;

    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE, KEY_WOW64_64KEY,
        REG_BINARY, REG_DWORD, REG_DWORD_BIG_ENDIAN, REG_EXPAND_SZ, REG_LINK, REG_MULTI_SZ,
        REG_NONE, REG_QWORD, REG_SZ,
    };
    use winreg::{RegKey, HKEY};

    use super::{from_nodes, Kind, Node, RawValue, POLICY_PATH};
    use crate::{Policies, Warning};

    /// A key of the registry: its hive and its path under it.
    #[derive(Clone, Copy, Debug)]
    pub struct Location<'a> {
        pub hive: HKEY,
        pub path: &'a str,
    }

    impl Location<'_> {
        fn name(&self) -> String {
            let hive = if self.hive == HKEY_LOCAL_MACHINE {
                "HKLM"
            } else if self.hive == HKEY_CURRENT_USER {
                "HKCU"
            } else {
                "HK?"
            };
            format!(r"{hive}\{}", self.path)
        }
    }

    /// `HKLM\SOFTWARE\Policies\Escouade`.
    pub const MACHINE: Location<'static> = Location {
        hive: HKEY_LOCAL_MACHINE,
        path: POLICY_PATH,
    };
    /// `HKCU\SOFTWARE\Policies\Escouade`.
    pub const USER: Location<'static> = Location {
        hive: HKEY_CURRENT_USER,
        path: POLICY_PATH,
    };

    /// The policies of this machine and its user.
    pub fn read_system(known: &[&str]) -> Policies {
        read_registry(MACHINE, USER, true, known)
    }

    /// The policies of two keys, the machine's and the user's. With `admin_only`, a key that this process could
    /// write without being elevated is refused: its ACL no longer keeps it to the administrators.
    pub fn read_registry(
        machine: Location,
        user: Location,
        admin_only: bool,
        known: &[&str],
    ) -> Policies {
        let mut warnings = Vec::new();
        let machine_node = load(machine, admin_only, &mut warnings);
        let user_node = load(user, admin_only, &mut warnings);
        let mut policies = from_nodes(machine_node.as_ref(), user_node.as_ref(), known);
        for warning in warnings {
            policies.warn(warning);
        }
        policies
    }

    fn load(location: Location, admin_only: bool, warnings: &mut Vec<Warning>) -> Option<Node> {
        let hive = RegKey::predef(location.hive);
        let key = match hive.open_subkey_with_flags(location.path, KEY_READ | KEY_WOW64_64KEY) {
            Ok(key) => key,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return None,
            Err(e) => {
                warnings.push(Warning::Unreadable {
                    place: location.name(),
                    reason: e.to_string(),
                });
                return None;
            }
        };
        if admin_only
            && hive
                .open_subkey_with_flags(location.path, KEY_SET_VALUE | KEY_WOW64_64KEY)
                .is_ok()
            && !elevated()
        {
            warnings.push(Warning::Refused {
                place: location.name(),
                reason: "this user may change it without being an administrator: its ACL has been loosened".to_owned(),
            });
            return None;
        }
        match node(&key, true) {
            Ok(node) => Some(node),
            Err(e) => {
                warnings.push(Warning::Unreadable {
                    place: location.name(),
                    reason: e.to_string(),
                });
                None
            }
        }
    }

    /// A key's values, and with `lists` its subkeys one level down (the ADMX lists hold no deeper subkey: one that
    /// does is said, its own content not read).
    fn node(key: &RegKey, lists: bool) -> io::Result<Node> {
        let mut node = Node::default();
        for item in key.enum_values() {
            let (name, value) = item?;
            node.values.push((
                name,
                RawValue {
                    kind: kind(value.vtype),
                    bytes: value.bytes,
                },
            ));
        }
        for name in key.enum_keys() {
            let name = name?;
            let sub = if lists {
                node_of(key.open_subkey_with_flags(&name, KEY_READ | KEY_WOW64_64KEY)?)?
            } else {
                Node::default()
            };
            node.subkeys.push((name, sub));
        }
        Ok(node)
    }

    fn node_of(key: RegKey) -> io::Result<Node> {
        node(&key, false)
    }

    fn kind(vtype: winreg::enums::RegType) -> Kind {
        match vtype {
            REG_NONE => Kind::None,
            REG_SZ => Kind::Sz,
            REG_EXPAND_SZ => Kind::ExpandSz,
            REG_BINARY => Kind::Binary,
            REG_DWORD => Kind::Dword,
            REG_DWORD_BIG_ENDIAN => Kind::DwordBigEndian,
            REG_LINK => Kind::Link,
            REG_MULTI_SZ => Kind::MultiSz,
            REG_QWORD => Kind::Qword,
            other => Kind::Other(other as u32),
        }
    }

    /// Whether this process runs elevated (an administrator's full token).
    fn elevated() -> bool {
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
        use windows_sys::Win32::Security::{
            GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        let mut token: HANDLE = std::ptr::null_mut();
        // SAFETY: the pseudo-handle of this process, and a token handle closed below.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut length = 0u32;
        // SAFETY: a TOKEN_ELEVATION of the size given, for a token this process opened.
        let ok = unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut length,
            )
        };
        // SAFETY: the token opened above.
        unsafe { CloseHandle(token) };
        ok != 0 && elevation.TokenIsElevated != 0
    }

    #[cfg(test)]
    mod tests {
        use serde_json::Value;
        use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};
        use winreg::{RegKey, RegValue};

        use super::*;
        use crate::{Source, PROVISIONAL_KEYS};

        /// A test key under `HKCU\Software\` (not `Policies`), deleted when dropped.
        struct TestKey {
            path: String,
        }

        impl TestKey {
            fn new(name: &str) -> (TestKey, RegKey) {
                let path = format!(r"Software\EscouadePolicyTest-{}-{name}", std::process::id());
                let hkcu = RegKey::predef(HKEY_CURRENT_USER);
                let _ = hkcu.delete_subkey_all(&path);
                let (key, _) = hkcu
                    .create_subkey_with_flags(&path, KEY_ALL_ACCESS)
                    .unwrap();
                (TestKey { path }, key)
            }
        }

        impl Drop for TestKey {
            fn drop(&mut self) {
                let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.path);
            }
        }

        fn location(key: &TestKey) -> Location<'_> {
            Location {
                hive: HKEY_CURRENT_USER,
                path: &key.path,
            }
        }

        #[test]
        fn a_real_test_key_under_hkcu_is_read_with_its_types_its_lists_and_its_json() {
            let (machine, machine_key) = TestKey::new("machine");
            let (user, user_key) = TestKey::new("user");

            machine_key.set_value("telemetry.enabled", &0u32).unwrap();
            machine_key
                .set_value("proxy.url", &"http://proxy.acme:3128")
                .unwrap();
            machine_key.set_value("updates.channel", &"stable").unwrap();
            machine_key
                .set_value(
                    "harness.allowedModels",
                    &r#"{"claude": ["opus", "sonnet"]}"#,
                )
                .unwrap();
            machine_key
                .set_value(
                    "proxy.exclusions",
                    &vec!["localhost".to_owned(), "*.acme.corp".to_owned()],
                )
                .unwrap();
            let (list, _) = machine_key.create_subkey("harness.allowed").unwrap();
            list.set_value("2", &"codex").unwrap();
            list.set_value("1", &"claude").unwrap();
            list.set_value("10", &"opencode").unwrap();

            user_key.set_value("telemetry.enabled", &1u32).unwrap();
            user_key.set_value("voice.onlineAllowed", &0u32).unwrap();
            user_key
                .set_value("permissions.allowedModes", &r#"["default", "plan""#)
                .unwrap();
            user_key
                .set_raw_value(
                    "remoteControl.allowed",
                    &RegValue {
                        bytes: vec![1, 2, 3],
                        vtype: REG_BINARY,
                    },
                )
                .unwrap();

            let policies =
                read_registry(location(&machine), location(&user), false, PROVISIONAL_KEYS);

            let value = |key: &str| {
                policies
                    .get(key)
                    .map(|p| (p.value.clone(), p.source.clone()))
            };
            assert_eq!(
                value("telemetry.enabled"),
                Some((Value::from(0), Source::Machine))
            );
            assert_eq!(
                value("voice.onlineAllowed"),
                Some((Value::from(0), Source::User))
            );
            assert_eq!(
                value("proxy.url"),
                Some((Value::from("http://proxy.acme:3128"), Source::Machine))
            );
            assert_eq!(
                value("harness.allowedModels"),
                Some((
                    serde_json::json!({"claude": ["opus", "sonnet"]}),
                    Source::Machine
                ))
            );
            assert_eq!(
                value("proxy.exclusions"),
                Some((
                    serde_json::json!(["localhost", "*.acme.corp"]),
                    Source::Machine
                ))
            );
            assert_eq!(
                value("harness.allowed"),
                Some((
                    serde_json::json!(["claude", "codex", "opencode"]),
                    Source::Machine
                ))
            );
            assert_eq!(value("permissions.allowedModes"), None);
            assert_eq!(value("remoteControl.allowed"), None);
            let malformed: Vec<&str> = policies
                .warnings()
                .iter()
                .filter_map(|w| match w {
                    Warning::Malformed { key, .. } => Some(key.as_str()),
                    _ => None,
                })
                .collect();
            let mut malformed = malformed;
            malformed.sort();
            assert_eq!(
                malformed,
                ["permissions.allowedModes", "remoteControl.allowed"]
            );
        }

        #[test]
        fn a_key_this_user_may_write_is_refused_when_only_administrators_should() {
            if elevated() {
                // An elevated process may write the real keys too: nothing to tell apart.
                return;
            }
            let (machine, machine_key) = TestKey::new("loose");
            machine_key.set_value("telemetry.enabled", &0u32).unwrap();
            let absent = Location {
                hive: HKEY_CURRENT_USER,
                path: r"Software\EscouadePolicyTest-absent-key",
            };

            let policies = read_registry(location(&machine), absent, true, PROVISIONAL_KEYS);

            assert!(policies.is_empty());
            assert!(
                matches!(policies.warnings(), [Warning::Refused { place, .. }] if place.contains("loose"))
            );
        }

        #[test]
        fn the_real_policy_keys_are_not_writable_by_this_user() {
            if elevated() {
                return;
            }
            // Under HKCU, SOFTWARE\Policies gives this user read only (README.md): even the user's own hive.
            let hkcu = RegKey::predef(HKEY_CURRENT_USER);
            assert!(hkcu
                .open_subkey_with_flags(r"SOFTWARE\Policies", KEY_SET_VALUE | KEY_WOW64_64KEY)
                .is_err());
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            assert!(hklm
                .open_subkey_with_flags(r"SOFTWARE\Policies", KEY_SET_VALUE | KEY_WOW64_64KEY)
                .is_err());
        }

        #[test]
        fn absent_keys_give_nothing_and_say_nothing() {
            let absent = Location {
                hive: HKEY_CURRENT_USER,
                path: r"Software\EscouadePolicyTest-absent-key",
            };
            let policies = read_registry(absent, absent, true, PROVISIONAL_KEYS);
            assert!(policies.is_empty());
            assert!(policies.warnings().is_empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::PROVISIONAL_KEYS;

    fn sz(text: &str) -> RawValue {
        let mut bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
        bytes.extend([0, 0]);
        RawValue {
            kind: Kind::Sz,
            bytes,
        }
    }

    fn dword(n: u32) -> RawValue {
        RawValue {
            kind: Kind::Dword,
            bytes: n.to_le_bytes().to_vec(),
        }
    }

    fn values(items: &[(&str, RawValue)]) -> Node {
        Node {
            values: items
                .iter()
                .map(|(n, v)| (n.to_string(), v.clone()))
                .collect(),
            subkeys: vec![],
        }
    }

    #[test]
    fn values_of_each_type() {
        assert_eq!(value_of(&dword(7)), Ok(json!(7)));
        assert_eq!(
            value_of(&RawValue {
                kind: Kind::DwordBigEndian,
                bytes: vec![0, 0, 1, 0]
            }),
            Ok(json!(256))
        );
        assert_eq!(
            value_of(&RawValue {
                kind: Kind::Qword,
                bytes: 5u64.to_le_bytes().to_vec()
            }),
            Ok(json!(5))
        );
        assert_eq!(value_of(&sz("stable")), Ok(json!("stable")));
        assert_eq!(
            value_of(&RawValue {
                kind: Kind::ExpandSz,
                ..sz("%TEMP%")
            }),
            Ok(json!("%TEMP%"))
        );
        assert_eq!(value_of(&sz(r#"{"a":1}"#)), Ok(json!({"a": 1})));
        let multi: Vec<u8> = "a\0b\0\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(
            value_of(&RawValue {
                kind: Kind::MultiSz,
                bytes: multi
            }),
            Ok(json!(["a", "b"]))
        );
        // A REG_SZ without its final NUL, as some tools write it.
        let bare: Vec<u8> = "x".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(
            value_of(&RawValue {
                kind: Kind::Sz,
                bytes: bare
            }),
            Ok(json!("x"))
        );
    }

    #[test]
    fn malformed_values_are_errors_not_panics() {
        assert!(value_of(&RawValue {
            kind: Kind::Dword,
            bytes: vec![1, 2]
        })
        .is_err());
        assert!(value_of(&RawValue {
            kind: Kind::Qword,
            bytes: vec![1; 4]
        })
        .is_err());
        assert!(value_of(&RawValue {
            kind: Kind::Sz,
            bytes: vec![b'a']
        })
        .is_err());
        assert!(value_of(&RawValue {
            kind: Kind::Sz,
            bytes: vec![0x00, 0xD8, 0x41, 0x00]
        })
        .is_err());
        assert!(value_of(&RawValue {
            kind: Kind::Binary,
            bytes: vec![1]
        })
        .is_err());
        assert!(value_of(&RawValue {
            kind: Kind::Other(42),
            bytes: vec![]
        })
        .is_err());
        assert!(value_of(&sz("[1, 2")).is_err());
    }

    #[test]
    fn the_machine_wins_over_the_user_key_by_key() {
        let machine = values(&[("telemetry.enabled", dword(0))]);
        let user = values(&[
            ("telemetry.enabled", dword(1)),
            ("voice.dictation", dword(0)),
        ]);
        let policies = from_nodes(Some(&machine), Some(&user), PROVISIONAL_KEYS);
        assert_eq!(
            policies.get("telemetry.enabled").unwrap().source,
            Source::Machine
        );
        assert_eq!(policies.get("telemetry.enabled").unwrap().value, json!(0));
        assert_eq!(
            policies.get("voice.dictation").unwrap().source,
            Source::User
        );
        assert!(policies.warnings().is_empty());
    }

    #[test]
    fn an_admx_list_is_ordered_by_number_and_its_strays_are_said() {
        let mut list = values(&[
            ("3", sz("c")),
            ("1", sz("a")),
            ("x", sz("?")),
            ("2", sz("b")),
            ("4", dword(9)),
        ]);
        list.subkeys.push(("deeper".into(), Node::default()));
        let machine = Node {
            values: vec![("", sz("default value"))]
                .into_iter()
                .map(|(n, v)| (n.into(), v))
                .collect(),
            subkeys: vec![("plugins.allowed".into(), list)],
        };
        let policies = from_nodes(Some(&machine), None, PROVISIONAL_KEYS);
        assert_eq!(
            policies.get("plugins.allowed").unwrap().value,
            json!(["a", "b", "c", 9])
        );
        let said: Vec<String> = policies.warnings().iter().map(|w| w.to_string()).collect();
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0].contains(r"plugins.allowed\x"));
        assert!(said[1].contains(r"plugins.allowed\deeper"));
    }

    #[test]
    fn a_value_and_a_list_of_the_same_name_keep_the_list_and_say_so() {
        let machine = Node {
            values: vec![("harness.allowed".into(), sz("claude"))],
            subkeys: vec![("harness.allowed".into(), values(&[("1", sz("codex"))]))],
        };
        let policies = from_nodes(Some(&machine), None, PROVISIONAL_KEYS);
        assert_eq!(
            policies.get("harness.allowed").unwrap().value,
            json!(["codex"])
        );
        assert_eq!(policies.warnings().len(), 1);
    }

    #[test]
    fn unknown_keys_are_kept_and_reported() {
        let user = values(&[("future.setting", dword(1))]);
        let policies = from_nodes(None, Some(&user), PROVISIONAL_KEYS);
        assert_eq!(policies.get("future.setting").unwrap().value, json!(1));
        assert!(matches!(policies.warnings(), [Warning::UnknownKey { .. }]));
    }
}
