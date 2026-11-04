//! Persistence layer — everything lives in `localStorage`.
//!
//! * `singsin_phrases`  — the user's phrasebook (custom / edited / deleted)
//! * `singsin_practiced` — the ids of phrases already practised
//! * `singsin_settings`  — API key, proxy, audio preferences

use serde::{Deserialize, Serialize};

use crate::data::phrases::Phrase;

pub const PHRASES_KEY: &str = "singsin_phrases";
pub const PRACTICED_KEY: &str = "singsin_practiced";
pub const SETTINGS_KEY: &str = "singsin_settings";

/// User preferences.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// MiniMax API key used for the optional LLM enrichment.
    #[serde(default)]
    pub mini_max_api_key: String,
    /// Optional CORS proxy prefix for the `words.hk` audio.
    #[serde(default)]
    pub audio_proxy: String,
    /// Voice URI of the preferred Cantonese TTS voice.
    #[serde(default)]
    pub tts_voice: String,
    /// Play the browser TTS at 0.72×.
    #[serde(default)]
    pub slow_tts: bool,
    /// Pan the two voices apart instead of centring them.
    #[serde(default)]
    pub stereo_split: bool,
    /// Balance between Original and Mine (0..1).
    #[serde(default = "half")]
    pub balance: f32,
    /// `"percent"` or `"seconds"`.
    #[serde(default)]
    pub time_mode: String,
    /// `"phrase"` or `"syllable"`.
    #[serde(default)]
    pub zoom: String,
    /// Stretch the user's take so it matches the reference duration.
    #[serde(default = "yes")]
    pub time_sync: bool,
    /// Show the chart y-axis in absolute Hz instead of relative semitones.
    #[serde(default)]
    pub absolute_hz: bool,
}

fn half() -> f32 {
    0.5
}
fn yes() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            mini_max_api_key: String::new(),
            audio_proxy: String::new(),
            tts_voice: String::new(),
            slow_tts: false,
            stereo_split: false,
            balance: 0.5,
            time_mode: "percent".into(),
            zoom: "phrase".into(),
            time_sync: true,
            absolute_hz: false,
        }
    }
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()
        .and_then(|w| w.local_storage().ok().flatten())
}

fn read_raw(key: &str) -> Option<String> {
    storage()?.get_item(key).ok().flatten()
}

fn write_raw(key: &str, value: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(key, value);
    }
}

/// The phrasebook the user has customised, or `None` when untouched.
pub fn load_phrases() -> Option<Vec<Phrase>> {
    let raw = read_raw(PHRASES_KEY)?;
    serde_json::from_str(&raw).ok()
}

/// Persist the user's phrasebook.
pub fn save_phrases(phrases: &[Phrase]) {
    if let Ok(json) = serde_json::to_string(phrases) {
        write_raw(PHRASES_KEY, &json);
    }
}

/// Remove the custom phrasebook and fall back to the seed library.
pub fn clear_phrases() {
    if let Some(s) = storage() {
        let _ = s.remove_item(PHRASES_KEY);
    }
}

/// The ids of the phrases already practised.
pub fn load_practiced() -> Vec<i64> {
    read_raw(PRACTICED_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Persist the practised ids.
pub fn save_practiced(ids: &[i64]) {
    if let Ok(json) = serde_json::to_string(ids) {
        write_raw(PRACTICED_KEY, &json);
    }
}

/// Load the user settings.
pub fn load_settings() -> Settings {
    read_raw(SETTINGS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// Persist the user settings.
pub fn save_settings(settings: &Settings) {
    if let Ok(json) = serde_json::to_string(settings) {
        write_raw(SETTINGS_KEY, &json);
    }
}
