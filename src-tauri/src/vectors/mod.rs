//! Local vectors (ESC-252): the texts of a project's docs become vectors computed on this machine, by a small
//! multilingual model run with ONNX Runtime, and are searched by their meaning. Nothing leaves the machine but the
//! download of the model. The model, the pieces and the index are the crate `escouade-vectors`; here, its wiring to the
//! core.

pub use escouade_vectors::{chunk, embed, index, model};
pub mod wiring;

#[cfg(test)]
mod tests;
