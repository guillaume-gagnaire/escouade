//! `escouade_team::fuzzing::invite`: see its documentation in `crates/escouade-team/src/fuzzing/invite.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::invite(data);
});
