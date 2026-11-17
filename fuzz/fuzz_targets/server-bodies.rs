//! `escouade_team_server::fuzzing::bodies`: see its documentation in `crates/escouade-team-server/src/fuzzing.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team_server::fuzzing::bodies(data);
});
