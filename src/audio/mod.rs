//! Web Audio — reference fetching, playback scheduling, microphone capture
//! and the Cantonese TTS fallback.

pub mod engine;
pub mod recorder;
pub mod tts;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use web_sys::AudioBuffer;

/// A cache of decoded `words.hk` syllables, so a syllable can be replayed
/// without another network round-trip.
///
/// The wrapper implements `PartialEq` by identity: the cache is a mutable
/// side-channel, never a reason to re-render a component.
#[derive(Clone, Default)]
pub struct SyllableCache(pub Rc<RefCell<HashMap<String, AudioBuffer>>>);

impl PartialEq for SyllableCache {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl SyllableCache {
    /// Look a syllable up.
    pub fn get(&self, jyutping: &str) -> Option<AudioBuffer> {
        self.0.borrow().get(jyutping).cloned()
    }

    /// Store a syllable.
    pub fn put(&self, jyutping: &str, buffer: AudioBuffer) {
        self.0.borrow_mut().insert(jyutping.to_string(), buffer);
    }

}
