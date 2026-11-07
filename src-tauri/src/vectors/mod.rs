//! Local vectors (ESC-252): the texts of a project's docs become vectors computed on this machine, by a small
//! multilingual model run with ONNX Runtime, and are searched by their meaning. Nothing leaves the machine but the
//! download of the model.

// The API is for the tickets that follow (the Docs, the memory): no caller yet.
#![allow(dead_code)]

pub mod chunk;
pub mod embed;
#[cfg(test)]
mod eval;
pub mod index;
pub mod model;
pub mod wiring;

#[cfg(test)]
mod tests;
