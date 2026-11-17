//! The decoding of a project's operation, sealed and opened (`escouade_team::fuzzing::operation`, TS-5).
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::operation(data);
});
