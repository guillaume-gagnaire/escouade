//! Prints the policies of this device and the warnings, as JSON: `cargo run -p escouade-policy --example read`.
//! What TS-12's real trials run after installing a GPO, a profile or a file of `/etc/escouade/policies`.

fn main() {
    let policies = escouade_policy::read();
    println!(
        "{}",
        serde_json::to_string_pretty(&policies).expect("policies are JSON")
    );
    for warning in policies.warnings() {
        eprintln!("warning: {warning}");
    }
}
