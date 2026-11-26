//! What a process of the server goes out with when the gate is on (R6, § 4.3): the gate as its one proxy, in place of
//! the one set in the space (`HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` and their lower case, `NO_PROXY` for the loopback
//! alone, which the agents' test servers keep), and `ssh` through `escouade-server via %h %p` (`GIT_SSH_COMMAND`, and
//! `core.sshCommand` in the worktrees). Set by the core alone: a plugin may set none of them
//! (`escouade-plugins::procs`).

/// The proxy's variables the gate takes the place of.
pub const PROXY_VARS: [&str; 8] = [
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "ALL_PROXY",
    "https_proxy",
    "http_proxy",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
];

/// The `NO_PROXY` of a process behind the gate: the loopback alone.
pub const NO_PROXY: &str = "127.0.0.1,localhost";

/// `ssh`'s command for git.
pub const GIT_SSH_COMMAND: &str = "GIT_SSH_COMMAND";

/// What tells our `ProxyCommand` in an ssh command.
pub const MARK: &str = " via %h %p";

/// The variables of a process given the gate's addresses (`http`, `socks`) and the ssh command (`ssh_command`).
pub fn gate_env(http: &str, socks: &str, ssh_command: &str) -> Vec<(String, String)> {
    let mut env = Vec::with_capacity(9);
    for k in ["HTTPS_PROXY", "HTTP_PROXY", "https_proxy", "http_proxy"] {
        env.push((k.to_string(), http.to_string()));
    }
    for k in ["ALL_PROXY", "all_proxy"] {
        env.push((k.to_string(), socks.to_string()));
    }
    for k in ["NO_PROXY", "no_proxy"] {
        env.push((k.to_string(), NO_PROXY.to_string()));
    }
    if !ssh_command.is_empty() {
        env.push((GIT_SSH_COMMAND.to_string(), ssh_command.to_string()));
    }
    env
}

/// `env` with the gate's `gate` in place of its proxy (and of its ssh command): what was of the space's proxy goes,
/// the rest stays.
pub fn with_gate(env: &mut Vec<(String, String)>, gate: &[(String, String)]) {
    env.retain(|(k, _)| {
        !PROXY_VARS.contains(&k.as_str())
            && k != GIT_SSH_COMMAND
            && !gate.iter().any(|(g, _)| g == k)
    });
    env.extend(gate.iter().cloned());
}

/// A word for `sh`: as it is when it is plain, else in double quotes.
fn sh_word(s: &str) -> String {
    let plain = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "/._-+:@%=,".contains(c));
    if plain {
        s.to_string()
    } else {
        format!("\"{}\"", in_quotes(s))
    }
}

/// `s` for inside double quotes of `sh`.
fn in_quotes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '"' | '$' | '`') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// `ssh`'s `ProxyCommand` through the gate: `<escouade-server> via %h %p`.
pub fn proxy_command(exe: &str) -> String {
    format!("{}{MARK}", sh_word(&exe.replace('\\', "/")))
}

/// Whether `cmd` goes through the gate already.
pub fn is_ours(cmd: &str) -> bool {
    cmd.contains(MARK)
}

/// The ssh command of git through the gate: `base` (the user's own ssh command, else `ssh`) with
/// `-o ProxyCommand="<exe> via %h %p"`. One that goes through it already stays as it is.
pub fn ssh_command(base: Option<&str>, exe: &str) -> String {
    let base = base
        .map(str::trim)
        .filter(|b| !b.is_empty())
        .unwrap_or("ssh");
    if is_ours(base) {
        return base.to_string();
    }
    format!(
        "{base} -o ProxyCommand=\"{}\"",
        in_quotes(&proxy_command(exe))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_gate_takes_the_place_of_the_proxy_and_of_the_ssh_command() {
        let mut env = vec![
            ("HTTPS_PROXY".to_string(), "http://proxy:3128".to_string()),
            ("no_proxy".to_string(), "corp".to_string()),
            ("GIT_SSH_COMMAND".to_string(), "ssh -i k".to_string()),
            ("NODE_EXTRA_CA_CERTS".to_string(), "/ca.pem".to_string()),
        ];
        let gate = gate_env(
            "http://t@127.0.0.1:5",
            "socks5h://t@127.0.0.1:5",
            &ssh_command(None, "/opt/escouade/escouade-server"),
        );
        with_gate(&mut env, &gate);
        let get = |k: &str| {
            env.iter()
                .filter(|(n, _)| n == k)
                .map(|(_, v)| v.as_str())
                .collect::<Vec<_>>()
        };
        assert_eq!(get("HTTPS_PROXY"), vec!["http://t@127.0.0.1:5"]);
        assert_eq!(get("http_proxy"), vec!["http://t@127.0.0.1:5"]);
        assert_eq!(get("ALL_PROXY"), vec!["socks5h://t@127.0.0.1:5"]);
        assert_eq!(get("NO_PROXY"), vec!["127.0.0.1,localhost"]);
        assert_eq!(get("no_proxy"), vec!["127.0.0.1,localhost"]);
        assert_eq!(get("NODE_EXTRA_CA_CERTS"), vec!["/ca.pem"]);
        assert_eq!(
            get("GIT_SSH_COMMAND"),
            vec!["ssh -o ProxyCommand=\"/opt/escouade/escouade-server via %h %p\""]
        );
    }

    #[test]
    fn the_ssh_command_keeps_the_user_s_and_quotes_a_path_with_spaces() {
        assert_eq!(
            ssh_command(Some("ssh -i ~/.ssh/deploy"), "/usr/bin/escouade-server"),
            "ssh -i ~/.ssh/deploy -o ProxyCommand=\"/usr/bin/escouade-server via %h %p\""
        );
        let ours = ssh_command(None, "/x/escouade-server");
        assert_eq!(ssh_command(Some(&ours), "/y/escouade-server"), ours);
        assert_eq!(
            ssh_command(None, "C:\\Program Files\\Escouade\\escouade-server.exe"),
            "ssh -o ProxyCommand=\"\\\"C:/Program Files/Escouade/escouade-server.exe\\\" via %h %p\""
        );
        assert!(is_ours(&ours));
    }
}
