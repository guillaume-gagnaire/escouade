//! `escouade_team::fuzzing::envelope`: see its documentation in `crates/escouade-team/src/fuzzing/envelope.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::envelope(data);
});
