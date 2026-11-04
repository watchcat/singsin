//! SingSin 聲線 — a Cantonese voice-line practice studio.
//!
//! The crate is exposed as a library as well as a binary so that the
//! deterministic phonetic engine and the DSP can be exercised by
//! integration tests.

pub mod app;
pub mod audio;
pub mod components;
pub mod data;
pub mod dsp;
pub mod minimax;
pub mod storage;
