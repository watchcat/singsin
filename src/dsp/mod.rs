//! Signal processing — pitch tracking, energy envelopes, voice activity
//! detection and the contour model. Everything here is plain Rust that runs
//! inside Wasm.

pub mod contour;
pub mod pitch;
pub mod track;
pub mod vad;
