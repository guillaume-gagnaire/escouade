//! Prints the policies of this device and the warnings, as JSON: `cargo run -p escouade-policy --example read`.
//! With `--watch`, prints them again each time they change, until Ctrl+C: the reading again of TS-12.
//! What the real trials run after installing a GPO, a profile or a file of `/etc/escouade/policies`
//! (`scripts/windows-trial.ps1`).

use std::time::Duration;

fn print(policies: &escouade_policy::Policies) {
    println!(
        "{}",
        serde_json::to_string_pretty(policies).expect("policies are JSON")
    );
    for warning in policies.warnings() {
        eprintln!("warning: {warning}");
    }
}

fn main() {
    let policies = escouade_policy::read();
    print(&policies);
    if std::env::args().any(|a| a == "--watch") {
        eprintln!("watching: change a policy, Ctrl+C to stop");
        let _watch = escouade_policy::watch::system_watch(policies, Duration::from_secs(30), |p| {
            eprintln!("--- changed");
            print(&p);
        });
        loop {
            std::thread::park();
        }
    }
}
