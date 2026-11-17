//! `escouade_team::fuzzing::settings`: see its documentation in `crates/escouade-team/src/fuzzing/settings.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::settings(data);
});
