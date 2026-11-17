//! `escouade_team::fuzzing::convergence`: see its documentation in `crates/escouade-team/src/fuzzing/convergence.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::convergence(data);
});
