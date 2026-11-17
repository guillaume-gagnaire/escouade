//! `escouade_team::fuzzing::catalog`: see its documentation in `crates/escouade-team/src/fuzzing/catalog.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::catalog(data);
});
