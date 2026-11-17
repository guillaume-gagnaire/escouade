//! `escouade_team::fuzzing::journal`: see its documentation in `crates/escouade-team/src/fuzzing/journal.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::journal(data);
});
