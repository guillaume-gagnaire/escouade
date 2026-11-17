//! `escouade_team::fuzzing::handshake`: see its documentation in `crates/escouade-team/src/fuzzing/handshake.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::handshake(data);
});
