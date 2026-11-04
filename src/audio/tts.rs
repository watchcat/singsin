//! Cantonese text-to-speech fallback.
//!
//! `words.hk` has no Cantonese TTS and the browser's own Cantonese voice is
//! not guaranteed to exist, so this is a *fallback*: it keeps the phrase
//! audible when the reference MP3s cannot be fetched. The voice is always
//! Cantonese (`yue-HK` / `zh-HK`) — never Mandarin.
//!
//! # The voice list arrives late
//!
//! `speechSynthesis.getVoices()` returns an **empty array** until the browser
//! has finished loading its voices. Chrome on macOS regularly takes well over
//! a second to do that, and only then fires `voiceschanged`. An empty list
//! therefore means *"not ready yet"* — **not** *"this browser has no Cantonese
//! voice"*. Reading it the other way round is how a perfectly capable machine
//! ends up claiming it has no Cantonese support, so every entry point here
//! keeps the two apart.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use yew::prelude::Callback;

/// The Cantonese locales we will accept.
pub const CANTONESE_LOCALES: [&str; 2] = ["yue-HK", "zh-HK"];

/// A voice offered by the browser.
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceInfo {
    pub name: String,
    pub lang: String,
    pub uri: String,
    pub default: bool,
}

/// What the browser's voice list looks like right now.
#[derive(Clone, Debug, PartialEq)]
pub enum VoiceStatus {
    /// The browser has not finished loading its voices yet.
    Loading,
    /// The list is in. These are the Cantonese voices — possibly none.
    Ready(Vec<VoiceInfo>),
}

/// Why a phrase could not be spoken.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SpeakError {
    /// The browser is still loading its voice list. A retry in a moment works;
    /// this is a timing problem, not a missing voice.
    VoicesNotReady,
    /// The voice list is in, and none of it is Cantonese.
    NoCantoneseVoice,
}

/// Every voice the browser currently exposes, Cantonese ones first.
pub fn voices() -> Vec<VoiceInfo> {
    let Some(window) = web_sys::window() else {
        return Vec::new();
    };
    let Ok(synth) = window.speech_synthesis() else {
        return Vec::new();
    };
    let list = synth.get_voices();

    let mut out: Vec<VoiceInfo> = Vec::new();
    for (i, v) in list.iter().enumerate() {
        let Ok(voice) = v.dyn_into::<web_sys::SpeechSynthesisVoice>() else {
            continue;
        };
        out.push(VoiceInfo {
            name: voice.name(),
            lang: voice.lang(),
            uri: voice.voice_uri(),
            default: i == 0,
        });
    }

    // Cantonese first, then everything else.
    out.sort_by(|a, b| {
        let rank = |lang: &str| -> u8 {
            let l = lang.to_ascii_lowercase();
            if l.starts_with("yue") {
                0
            } else if l.starts_with("zh-hk") || l.starts_with("zh-yue") {
                1
            } else if l.starts_with("zh") {
                3
            } else {
                2
            }
        };
        rank(&a.lang).cmp(&rank(&b.lang)).then(a.name.cmp(&b.name))
    });

    out
}

/// The Cantonese voices only.
pub fn cantonese_voices() -> Vec<VoiceInfo> {
    voices()
        .into_iter()
        .filter(|v| {
            let l = v.lang.to_ascii_lowercase();
            CANTONESE_LOCALES
                .iter()
                .any(|locale| l.starts_with(&locale.to_ascii_lowercase()))
                || l.starts_with("zh-yue")
        })
        .collect()
}

/// Does this browser expose the Web Speech API at all?
pub fn speech_available() -> bool {
    web_sys::window()
        .map(|w| w.speech_synthesis().is_ok())
        .unwrap_or(false)
}

/// Has the browser finished loading its voice list?
///
/// This is the question an empty `getVoices()` actually answers: *not yet*,
/// not *there are none*.
pub fn voices_loaded() -> bool {
    web_sys::window()
        .and_then(|w| w.speech_synthesis().ok())
        .map(|s| s.get_voices().length() > 0)
        .unwrap_or(false)
}

/// The current state of the voice list.
pub fn status() -> VoiceStatus {
    if !speech_available() {
        // No speech synthesis at all — that is a final answer, not a wait.
        return VoiceStatus::Ready(Vec::new());
    }
    if !voices_loaded() {
        return VoiceStatus::Loading;
    }
    VoiceStatus::Ready(cantonese_voices())
}

/// Speak `text` with the browser's Cantonese voice.
///
/// Returns an error rather than speaking in the wrong language: if the voice
/// list has not arrived yet that is a timing problem, and if it has arrived
/// without a Cantonese voice then there is genuinely nothing to speak with.
pub fn speak(text: &str, rate: f32, voice_uri: &str) -> Result<(), SpeakError> {
    let Some(window) = web_sys::window() else {
        return Err(SpeakError::NoCantoneseVoice);
    };
    let Ok(synth) = window.speech_synthesis() else {
        return Err(SpeakError::NoCantoneseVoice);
    };
    let list = synth.get_voices();
    if list.length() == 0 {
        return Err(SpeakError::VoicesNotReady);
    }

    let mut chosen: Option<web_sys::SpeechSynthesisVoice> = None;
    for v in list.iter() {
        if let Ok(voice) = v.dyn_into::<web_sys::SpeechSynthesisVoice>() {
            if !voice_uri.is_empty() && voice.voice_uri() == voice_uri {
                chosen = Some(voice);
                break;
            }
        }
    }
    if chosen.is_none() {
        for v in list.iter() {
            if let Ok(voice) = v.dyn_into::<web_sys::SpeechSynthesisVoice>() {
                let l = voice.lang().to_ascii_lowercase();
                if l.starts_with("yue") || l.starts_with("zh-hk") {
                    chosen = Some(voice);
                    break;
                }
            }
        }
    }
    let Some(voice) = chosen else {
        return Err(SpeakError::NoCantoneseVoice);
    };

    let Ok(utterance) = web_sys::SpeechSynthesisUtterance::new_with_text(text) else {
        return Err(SpeakError::NoCantoneseVoice);
    };
    utterance.set_voice(Some(&voice));
    utterance.set_lang(&voice.lang());
    utterance.set_rate(rate);
    utterance.set_pitch(1.0);

    synth.cancel();
    synth.speak(&utterance);
    Ok(())
}

/// Speak with the browser's Cantonese voice, waiting for the browser to finish
/// loading its voice list first.
///
/// Chrome can take more than a second to populate `getVoices()`, so clicking
/// "speak" straight after the page opens used to fail with a bogus "no
/// Cantonese voice" complaint. This waits for the list (up to four seconds)
/// and only then decides whether a Cantonese voice actually exists.
pub async fn speak_when_ready(
    text: String,
    rate: f32,
    voice_uri: String,
) -> Result<(), SpeakError> {
    for _ in 0..40 {
        if !speech_available() || voices_loaded() {
            break;
        }
        sleep(100).await;
    }
    speak(&text, rate, &voice_uri)
}

/// Resolve after `ms` milliseconds.
async fn sleep(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Watch for the voice list to arrive, then report it.
///
/// `on_update` fires once with the final status: `Loading` if the browser
/// still has not answered, otherwise `Ready` with the Cantonese voices. The
/// watcher stops itself as soon as the list is in.
pub fn watch_voices(on_update: Callback<VoiceStatus>) {
    let Some(window) = web_sys::window() else {
        on_update.emit(VoiceStatus::Ready(Vec::new()));
        return;
    };

    let current = status();
    if current != VoiceStatus::Loading {
        on_update.emit(current);
        return;
    }

    let handle: Rc<Cell<i32>> = Rc::new(Cell::new(0));
    let handle_for_cb = handle.clone();
    let window_for_cb = window.clone();
    let cb = Closure::wrap(Box::new(move || {
        let st = status();
        if st != VoiceStatus::Loading {
            window_for_cb.clear_interval_with_handle(handle_for_cb.get());
            on_update.emit(st);
        }
    }) as Box<dyn FnMut()>);

    let id = window
        .set_interval_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 150)
        .unwrap_or(0);
    handle.set(id);
    // The watcher stops itself once the voices arrive, so the closure can
    // outlive this call.
    cb.forget();
}
