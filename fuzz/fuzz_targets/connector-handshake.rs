//! `escouade_connector_proto::fuzzing::handshake`: see its documentation in `crates/escouade-connector-proto/src/fuzzing.rs`.
#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    escouade_connector_proto::fuzzing::handshake(data);
});
