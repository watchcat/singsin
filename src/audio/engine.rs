//! Web Audio engine: fetching reference syllables, stitching them into one
//! phrase buffer, and scheduling the four comparison playback modes.
//!
//! # A note on CORS
//!
//! The reference recordings live on [words.hk](https://words.hk), one MP3 per
//! syllable. That site does **not** send an `Access-Control-Allow-Origin`
//! header, so a browser cannot `fetch()` those bytes directly.
//!
//! Rather than depend on one public relay — which may be down, slow or
//! rate-limited — the engine keeps a list of sources and races them: every
//! candidate is requested at once, each with its own timeout, and the first
//! one to answer wins. The source that worked is remembered, so the next
//! syllable goes straight to it.
//!
//! Sources, in order:
//!
//! 1. the direct `words.hk` URL (works if the site ever enables CORS, or
//!    when the app is served from a host that proxies it);
//! 2. the bundled local relay (`tools/proxy.py`) — the fastest, most reliable
//!    option for a learner running the app on their own machine;
//! 3. the learner's own proxy prefix, if they set one in Audio Settings;
//! 4. public CORS relays.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AbortController, AudioBuffer, AudioBufferSourceNode, AudioContext, GainNode, Request,
    RequestInit, RequestMode, Response, StereoPannerNode,
};

/// Base URL of the `words.hk` syllable recordings.
pub const WORDS_HK_BASE: &str = "https://words.hk/static/jyutping/";

/// The bundled local relay (see `tools/proxy.py`).
pub const LOCAL_RELAY: &str = "http://127.0.0.1:8081/?url=";

/// The same relay, deployed as a Cloudflare Worker (see `tools/worker-relay.js`).
/// It is first in the list of candidates for anyone who is not running the
/// local one.
pub const CLOUD_RELAY: &str = "https://singsin-relay.watchcat.workers.dev/?url=";

/// How long one source may take before it is abandoned, in milliseconds.
const SOURCE_TIMEOUT_MS: i32 = 7000;

thread_local! {
    static CONTEXT: RefCell<Option<AudioContext>> = const { RefCell::new(None) };

    /// The source template that answered last time.
    static PREFERRED: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The one shared `AudioContext`. Browsers cap the number of contexts, so it
/// is created once and reused for the whole session.
pub fn ctx() -> Result<AudioContext, String> {
    CONTEXT.with(|c| {
        if let Some(existing) = c.borrow().as_ref() {
            return Ok(existing.clone());
        }
        let created = AudioContext::new().map_err(|e| format!("AudioContext: {e:?}"))?;
        *c.borrow_mut() = Some(created.clone());
        Ok(created)
    })
}

/// Resume a context that the browser suspended (autoplay policy).
pub fn resume(ctx: &AudioContext) {
    let _ = ctx.resume();
}

/// A phrase built out of the individual `words.hk` syllables.
#[derive(Clone)]
pub struct Reference {
    /// The stitched phrase buffer.
    pub buffer: AudioBuffer,
    /// Start / end time of every syllable inside `buffer`, in seconds.
    pub syllables: Vec<(f64, f64)>,
}

/// The source templates, most trusted first.
fn templates(user_proxy: &str) -> Vec<String> {
    let mut out = vec![
        "{direct}".to_string(),
        "{cloud}".to_string(),
        "{local}".to_string(),
    ];

    let trimmed = user_proxy.trim();
    if !trimmed.is_empty() {
        out.push(format!("{{custom}}{trimmed}"));
    }

    out.push("https://cors.eu.org/{url}".to_string());
    out.push("https://api.allorigins.win/raw?url={encoded}".to_string());
    out.push("https://api.codetabs.com/v1/proxy/?quest={encoded}".to_string());

    out
}

/// Expand one template into a concrete URL.
fn expand(template: &str, url: &str, encoded: &str) -> String {
    match template {
        "{direct}" => url.to_string(),
        "{cloud}" => format!("{CLOUD_RELAY}{encoded}"),
        "{local}" => format!("{LOCAL_RELAY}{encoded}"),
        _ => template.replace("{url}", url).replace("{encoded}", encoded),
    }
}

/// Every URL that might serve this syllable, best first.
pub fn candidate_urls(jyutping: &str, user_proxy: &str) -> Vec<String> {
    let url = format!("{WORDS_HK_BASE}{}.mp3", jyutping);
    let encoded = js_sys::encode_uri_component(&url).as_string().unwrap_or_default();
    ordered_templates(user_proxy)
        .iter()
        .map(|t| expand(t, &url, &encoded))
        .collect()
}

/// The source templates, with the one that answered last time moved to the
/// front.
fn ordered_templates(user_proxy: &str) -> Vec<String> {
    let mut list = templates(user_proxy);
    PREFERRED.with(|p| {
        if let Some(preferred) = p.borrow().as_ref() {
            if let Some(pos) = list.iter().position(|t| t == preferred) {
                let preferred = list.remove(pos);
                list.insert(0, preferred);
            }
        }
    });
    list
}

/// Fetch a single URL and return the raw bytes, giving up after `timeout_ms`.
pub async fn fetch_bytes(url: &str, timeout_ms: i32) -> Result<Vec<u8>, String> {
    let window = web_sys::window().ok_or("No window")?;

    let controller = AbortController::new().map_err(|e| format!("{e:?}"))?;
    let signal = controller.signal();

    let init = RequestInit::new();
    init.set_method("GET");
    init.set_mode(RequestMode::Cors);
    init.set_signal(Some(&signal));

    let request = Request::new_with_str_and_init(url, &init).map_err(|e| format!("{e:?}"))?;
    let promise = window.fetch_with_request(&request);

    // Abandon the request if it drags on.
    let timer = Closure::wrap(Box::new(move || controller.abort()) as Box<dyn FnMut()>);
    let timer_id = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            timer.as_ref().unchecked_ref(),
            timeout_ms,
        )
        .unwrap_or(0);

    let outcome = JsFuture::from(promise).await;
    window.clear_timeout_with_handle(timer_id);
    drop(timer);

    let value = outcome.map_err(|_| "network error".to_string())?;
    let response: Response = value.dyn_into().map_err(|_| "bad response".to_string())?;

    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }

    let buffer = JsFuture::from(response.array_buffer().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|_| "read failed".to_string())?;
    let array_buffer: js_sys::ArrayBuffer = buffer.unchecked_into();
    Ok(Uint8Array::new(&array_buffer).to_vec())
}

/// Fetch and decode one syllable from one specific URL.
async fn fetch_syllable_from(url: &str) -> Result<AudioBuffer, String> {
    let context = ctx()?;
    let bytes = fetch_bytes(url, SOURCE_TIMEOUT_MS).await?;
    if bytes.is_empty() {
        return Err("empty response".into());
    }

    let array = Uint8Array::new_with_length(bytes.len() as u32);
    array.copy_from(&bytes);
    let slice = array.buffer();

    let promise = context
        .decode_audio_data(&slice)
        .map_err(|e| format!("decode: {e:?}"))?;
    let value = JsFuture::from(promise)
        .await
        .map_err(|_| "decode rejected".to_string())?;
    value
        .dyn_into::<AudioBuffer>()
        .map_err(|_| "bad buffer".to_string())
}

/// A one-shot wake-up used to race several sources.
#[derive(Default)]
struct Signal {
    version: Cell<u64>,
    waker: RefCell<Option<Waker>>,
}

impl Signal {
    fn notify(&self) {
        self.version.set(self.version.get() + 1);
        if let Some(w) = self.waker.borrow_mut().take() {
            w.wake();
        }
    }
}

/// Future returned by [`Signal::wait`].
struct Waiter {
    signal: Rc<Signal>,
    seen: u64,
}

impl std::future::Future for Waiter {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = self.get_mut();
        if this.signal.version.get() != this.seen {
            return Poll::Ready(());
        }
        *this.signal.waker.borrow_mut() = Some(cx.waker().clone());
        if this.signal.version.get() != this.seen {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

/// Request every candidate at once and take the first that answers.
///
/// Returns the winning buffer together with the template that served it.
async fn fetch_first_success(
    candidates: Vec<(String, String)>,
) -> Result<(AudioBuffer, String), String> {
    if candidates.is_empty() {
        return Err("no source".into());
    }

    let result: Rc<RefCell<Option<(AudioBuffer, String)>>> = Rc::new(RefCell::new(None));
    let remaining = Rc::new(Cell::new(candidates.len()));
    let signal = Rc::new(Signal::default());

    for (template, url) in candidates {
        let result = result.clone();
        let remaining = remaining.clone();
        let signal = signal.clone();

        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(buffer) = fetch_syllable_from(&url).await {
                let mut slot = result.borrow_mut();
                if slot.is_none() {
                    *slot = Some((buffer, template));
                }
            }
            remaining.set(remaining.get() - 1);
            signal.notify();
        });
    }

    loop {
        let seen = signal.version.get();
        if result.borrow().is_some() {
            break;
        }
        if remaining.get() == 0 {
            break;
        }
        Waiter {
            signal: signal.clone(),
            seen,
        }
        .await;
    }

    let winner = result.borrow().clone();
    winner.ok_or_else(|| "every source failed".to_string())
}

/// Fetch and decode one syllable, trying every source at once.
pub async fn fetch_syllable(jyutping: &str, user_proxy: &str) -> Result<AudioBuffer, String> {
    let url = format!("{WORDS_HK_BASE}{}.mp3", jyutping);
    let encoded = js_sys::encode_uri_component(&url).as_string().unwrap_or_default();

    let candidates: Vec<(String, String)> = ordered_templates(user_proxy)
        .into_iter()
        .map(|t| {
            let expanded = expand(&t, &url, &encoded);
            (t, expanded)
        })
        .collect();

    let (buffer, template) = fetch_first_success(candidates).await?;
    // Remember which source won, so the next syllable goes straight to it.
    PREFERRED.with(|p| *p.borrow_mut() = Some(template));
    Ok(buffer)
}

/// Trim the silence off the front and back of one syllable.
fn trim_silence(data: &[f32], sample_rate: f32) -> (usize, usize) {
    if data.is_empty() {
        return (0, 0);
    }
    let mut peak = 0.0f32;
    for s in data {
        peak = peak.max(s.abs());
    }
    let threshold = (peak * 0.02).max(0.0015);
    let mut start = 0;
    while start < data.len() && data[start].abs() < threshold {
        start += 1;
    }
    if start == data.len() {
        return (0, data.len());
    }
    let mut end = data.len();
    while end > start && data[end - 1].abs() < threshold {
        end -= 1;
    }
    // A little breathing room either side so plosives are not clipped.
    let pad = (sample_rate * 0.015) as usize;
    let start = start.saturating_sub(pad);
    let end = (end + pad).min(data.len());
    (start, end)
}

/// Stitch decoded syllables into one mono phrase buffer.
///
/// `gap_ms` is the natural transition between syllables. The returned
/// vector holds the start / end time of each syllable inside the result, so
/// the schematic tone targets can be drawn exactly on top of the audio.
pub fn stitch(
    context: &AudioContext,
    parts: &[(String, AudioBuffer)],
    gap_ms: f64,
) -> Result<Reference, String> {
    let sample_rate = context.sample_rate();
    let gap = (sample_rate as f64 * gap_ms / 1000.0).round() as usize;

    let mut chunks: Vec<(String, Vec<f32>)> = Vec::with_capacity(parts.len());
    for (label, buffer) in parts {
        let raw: Vec<f32> = buffer
            .get_channel_data(0)
            .map_err(|e| format!("channel data: {e:?}"))?;
        let (s, e) = trim_silence(&raw, sample_rate);
        if e > s {
            chunks.push((label.clone(), raw[s..e].to_vec()));
        }
    }

    if chunks.is_empty() {
        return Err("nothing to stitch".into());
    }

    let total = chunks.iter().map(|(_, c)| c.len()).sum::<usize>()
        + gap * chunks.len().saturating_sub(1);

    let out = context
        .create_buffer(1, total.max(1) as u32, sample_rate)
        .map_err(|e| format!("create_buffer: {e:?}"))?;

    // `copy_to_channel` always writes from the start of the channel, so the
    // whole phrase is assembled in Rust first and copied across in one go.
    // Writing each syllable separately would make every one of them overwrite
    // the beginning of the buffer instead of following the previous one.
    let mut pcm = vec![0f32; total.max(1)];
    let mut offset = 0usize;
    let mut syllables = Vec::with_capacity(chunks.len());
    for (_, chunk) in &chunks {
        pcm[offset..offset + chunk.len()].copy_from_slice(chunk);
        let start = offset as f64 / sample_rate as f64;
        offset += chunk.len();
        let end = offset as f64 / sample_rate as f64;
        syllables.push((start, end));
        offset += gap;
    }
    out.copy_to_channel(&pcm, 0)
        .map_err(|e| format!("copy_to_channel: {e:?}"))?;

    Ok(Reference {
        buffer: out,
        syllables,
    })
}

/// One scheduled voice: source → gain → panner → destination.
pub struct Voice {
    pub source: AudioBufferSourceNode,
    pub gain: GainNode,
    pub panner: StereoPannerNode,
}

impl Voice {
    /// Stop this voice immediately.
    pub fn stop(&self) {
        // `AudioBufferSourceNode::stop` is deprecated in web-sys; the
        // non-deprecated form lives on the base class.
        let scheduled: &web_sys::AudioScheduledSourceNode = &self.source;
        let _ = scheduled.stop();
        let _ = self.source.disconnect();
        let _ = self.gain.disconnect();
        let _ = self.panner.disconnect();
    }
}

/// Schedule a buffer to play at a given time.
pub fn play(
    context: &AudioContext,
    buffer: &AudioBuffer,
    when: f64,
    rate: f32,
    level: f32,
    pan: f32,
) -> Result<Voice, String> {
    let source = context
        .create_buffer_source()
        .map_err(|e| format!("buffer source: {e:?}"))?;
    source.set_buffer(Some(buffer));
    source.playback_rate().set_value(rate);

    let gain = context.create_gain().map_err(|e| format!("gain: {e:?}"))?;
    gain.gain().set_value(level);

    let panner = context
        .create_stereo_panner()
        .map_err(|e| format!("panner: {e:?}"))?;
    panner.pan().set_value(pan.clamp(-1.0, 1.0));

    source.connect_with_audio_node(&gain).ok();
    gain.connect_with_audio_node(&panner).ok();
    panner.connect_with_audio_node(&context.destination()).ok();

    source.start_with_when(when.max(context.current_time())).ok();

    Ok(Voice {
        source,
        gain,
        panner,
    })
}

/// The total duration of a buffer in seconds.
pub fn buffer_duration(buffer: &AudioBuffer) -> f64 {
    buffer.duration() as f64
}

thread_local! {
    /// One-shot voices are kept alive here so the browser does not collect
    /// a node while it is still playing.
    static ACTIVE: RefCell<Vec<Voice>> = const { RefCell::new(Vec::new()) };
}

/// Play a buffer once and forget about it.
pub fn play_oneshot(context: &AudioContext, buffer: &AudioBuffer) -> Result<(), String> {
    let when = context.current_time() + 0.02;
    let voice = play(context, buffer, when, 1.0, 1.0, 0.0)?;
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();
        if active.len() > 12 {
            for v in active.drain(..) {
                v.stop();
            }
        }
        active.push(voice);
    });
    Ok(())
}

/// Copy a mono buffer out into a Rust vector.
pub fn buffer_samples(buffer: &AudioBuffer) -> Vec<f32> {
    buffer.get_channel_data(0).unwrap_or_default()
}

/// Wrap a mono sample vector back into an `AudioBuffer`.
pub fn samples_to_buffer(
    context: &AudioContext,
    samples: &[f32],
    sample_rate: f32,
) -> Result<AudioBuffer, String> {
    let buffer = context
        .create_buffer(1, samples.len().max(1) as u32, sample_rate)
        .map_err(|e| format!("create_buffer: {e:?}"))?;
    buffer
        .copy_to_channel(samples, 0)
        .map_err(|e| format!("copy_to_channel: {e:?}"))?;
    Ok(buffer)
}

/// Wrap a future into a JavaScript promise.
fn to_promise<F>(fut: F) -> js_sys::Promise
where
    F: std::future::Future<Output = Option<AudioBuffer>> + 'static,
{
    let mut slot = Some(fut);
    js_sys::Promise::new(&mut |resolve, _reject| {
        let taken = slot.take();
        let resolver = resolve.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let value = match taken {
                Some(f) => match f.await {
                    Some(buffer) => JsValue::from(buffer),
                    None => JsValue::NULL,
                },
                None => JsValue::NULL,
            };
            let _ = resolver.call1(&JsValue::NULL, &value);
        });
    })
}

/// Fetch a list of syllables in parallel.
///
/// Every syllable is fetched from the same set of sources, so a nine-syllable
/// phrase takes as long as its slowest syllable rather than the sum of all.
pub async fn fetch_syllables(
    tokens: &[String],
    proxy: &str,
) -> Vec<(String, Option<AudioBuffer>)> {
    if tokens.is_empty() {
        return Vec::new();
    }

    let promises: Vec<js_sys::Promise> = tokens
        .iter()
        .map(|t| {
            let token = t.clone();
            let proxy = proxy.to_string();
            to_promise(async move { fetch_syllable(&token, &proxy).await.ok() })
        })
        .collect();

    let array = js_sys::Array::new();
    for p in &promises {
        array.push(p.as_ref());
    }
    let all = js_sys::Promise::all(array.as_ref());

    let value = match JsFuture::from(all).await {
        Ok(v) => v,
        Err(_) => return tokens.iter().map(|t| (t.clone(), None)).collect(),
    };

    let array = js_sys::Array::from(&value);
    let mut out = Vec::with_capacity(tokens.len());
    for (i, token) in tokens.iter().enumerate() {
        let buffer = array.get(i as u32).dyn_into::<AudioBuffer>().ok();
        out.push((token.clone(), buffer));
    }
    out
}
