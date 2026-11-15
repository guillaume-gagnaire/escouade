//! The verification of the commits (`escouade_team::fuzzing::commit`): commits the bytes build, checked by a member.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_team::fuzzing::commit(data);
});
