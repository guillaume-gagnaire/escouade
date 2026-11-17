//! `escouade_team::fuzzing::snapshot`: see its documentation in `crates/escouade-team/src/fuzzing/snapshot.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::snapshot(data);
});
