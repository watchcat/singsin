//! Signal processing — pitch tracking, energy envelopes, voice activity
//! detection, the contour model and the pitch-preserving time stretch.
//! Everything here is plain Rust that runs inside Wasm.

pub mod contour;
pub mod pitch;
pub mod stretch;
pub mod track;
pub mod vad;
