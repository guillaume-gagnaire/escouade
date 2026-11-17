//! The decoding of the roster, the identities and the team's messages (`escouade_team::fuzzing::roster`).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::roster(data);
});
