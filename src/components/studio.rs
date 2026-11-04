//! The recording + comparison studio.
//!
//! This is where the reference recording, the learner's microphone take and
//! the four playback modes come together over one chart.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::AudioBuffer;
use yew::prelude::*;

use crate::audio::engine::{self, Voice};
use crate::audio::recorder::Recorder;
use crate::audio::tts;
use crate::audio::SyllableCache;
use crate::data::phonetics::{Analysis, Syllable};
use crate::data::phrases::Phrase;
use crate::dsp::track::Contour;
use crate::dsp::vad::{self, TrimInfo};
use crate::storage::Settings;

use super::chart;
use super::chart::{Chart, ChartModel, TargetSeg, TimeMode, Zoom};
use super::{Icon, Segmented, Toggle};

// ── state ───────────────────────────────────────────────────────────────────

/// Where the reference audio is coming from.
#[derive(Clone, PartialEq, Debug)]
pub enum RefStatus {
    Idle,
    Loading,
    Ready(String),
    Failed(String),
}

/// The non-reactive audio resources.
#[derive(Default)]
struct AudioState {
    take: Option<AudioBuffer>,
    reference: Option<AudioBuffer>,
    voices: Vec<Voice>,
    recorder: Option<Recorder>,
}

#[derive(Properties, PartialEq)]
pub struct Props {
    pub phrase: Phrase,
    pub analysis: Analysis,
    pub model: ChartModel,
    pub cache: SyllableCache,
    pub settings: Settings,
    pub on_settings: Callback<Settings>,
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// Lay the schematic tone targets over the reference timeline.
fn build_targets(sylls: &[Syllable], ref_times: &[(f64, f64)], total: f64) -> Vec<TargetSeg> {
    let real: Vec<&Syllable> = sylls.iter().filter(|s| !s.foreign).collect();
    let mut out = Vec::with_capacity(real.len());

    if ref_times.len() == real.len() && total > 0.0 {
        for (i, s) in real.iter().enumerate() {
            let (a, b) = ref_times[i];
            out.push(TargetSeg {
                start: (a / total).clamp(0.0, 1.0),
                end: (b / total).clamp(0.0, 1.0),
                tone: s.tone,
                spoken_tone: s.spoken_tone,
                han: s.han.clone(),
                jyutping: s.jyutping.clone(),
            });
        }
    } else {
        let n = real.len().max(1) as f64;
        for (i, s) in real.iter().enumerate() {
            out.push(TargetSeg {
                start: i as f64 / n,
                end: (i + 1) as f64 / n,
                tone: s.tone,
                spoken_tone: s.spoken_tone,
                han: s.han.clone(),
                jyutping: s.jyutping.clone(),
            });
        }
    }
    out
}

/// The message shown when a phrase could not be spoken by the browser.
fn tts_error_message(err: tts::SpeakError) -> String {
    match err {
        tts::SpeakError::VoicesNotReady => "The browser is still loading its Cantonese voice — \
             try again in a moment."
            .into(),
        tts::SpeakError::NoCantoneseVoice => {
            "This browser has no Cantonese (yue-HK / zh-HK) voice installed.".into()
        }
    }
}

/// Turn the recorder's live pitch points into a contour.
fn build_live(points: &[(f64, f32)], duration: f64) -> Contour {
    let mut f0s: Vec<f64> = points
        .iter()
        .filter(|(_, f)| *f > 0.0)
        .map(|(_, f)| *f as f64)
        .collect();
    f0s.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = if f0s.is_empty() { 0.0 } else { f0s[f0s.len() / 2] };

    let mut times = Vec::with_capacity(points.len());
    let mut semis = Vec::with_capacity(points.len());
    let mut rms = Vec::with_capacity(points.len());
    let mut voiced = Vec::with_capacity(points.len());

    for (t, f) in points {
        times.push(*t);
        if *f > 0.0 && median > 0.0 {
            semis.push(12.0 * (*f as f64 / median).log2());
            voiced.push(true);
        } else {
            semis.push(f64::NAN);
            voiced.push(false);
        }
        rms.push(0.0);
    }

    Contour {
        times,
        semis,
        rms,
        voiced,
        median_f0: median,
        duration,
    }
}

/// Tell the chart that the shared data changed.
fn bump(model: &ChartModel) {
    chart::bump(model);
}

/// Gains for the two voices from the balance slider (0 = all Original).
fn mix_gains(balance: f32) -> (f32, f32) {
    let b = balance.clamp(0.0, 1.0);
    (1.0 - 0.7 * b, 0.3 + 0.7 * b)
}

/// The class for one playback-mode button, so the running mode is visible.
fn mode_class(mode: u8, active: u8) -> &'static str {
    if mode == active {
        "mode active"
    } else {
        "mode"
    }
}

// ── the component ───────────────────────────────────────────────────────────

#[function_component(Studio)]
pub fn studio(props: &Props) -> Html {
    let audio = use_mut_ref(AudioState::default);

    let status = use_state(|| RefStatus::Idle);
    let recording = use_state(|| false);
    let has_take = use_state(|| false);
    let trim = use_state(|| Option::<TrimInfo>::None);
    let error = use_state(|| Option::<String>::None);
    // True while a phrase is being spoken by the browser's Cantonese voice.
    let speaking = use_state(|| false);
    let live_ms = use_state(|| 0u64);
    // Which playback mode is running (0 = none, 1–4).
    let active = use_state(|| 0u8);
    // Bumped whenever the shared chart model changes, so the toolbar keeps
    // showing the axis, zoom and layer selection that the chart is using.
    let rev = use_state(|| 0u32);
    {
        let model = props.model.clone();
        let set_rev = rev.clone();
        use_effect_with((), move |_| {
            model.borrow_mut().notify =
                Some(Callback::from(move |version: u32| set_rev.set(version)));
            || ()
        });
    }

    let base_settings = props.settings.clone();

    // ── load the reference recording ──────────────────────────────────────
    {
        let analysis = props.analysis.clone();
        let model = props.model.clone();
        let cache = props.cache.clone();
        let status = status.clone();
        let audio = audio.clone();
        let has_take = has_take.clone();
        let trim = trim.clone();
        let proxy = props.settings.audio_proxy.clone();
        let key = format!("{}|{}|{}", props.phrase.id, props.phrase.jyutping, proxy);

        use_effect_with(key, move |_| {
            status.set(RefStatus::Loading);

            {
                let mut m = model.borrow_mut();
                m.original = None;
                m.mine = None;
                m.live = None;
                m.targets.clear();
                m.original_rms.clear();
                m.mine_rms.clear();
                m.original_duration = 0.0;
                m.mine_duration = 0.0;
                m.original_median = 0.0;
                m.mine_median = 0.0;
                m.axis_seconds = 0.0;
                m.playing = false;
                m.focus = None;
            }

            // A new phrase means a new reference recording, and the old take
            // no longer belongs to it — the chart has already dropped it.
            {
                let mut a = audio.borrow_mut();
                a.reference = None;
                a.take = None;
            }
            has_take.set(false);
            trim.set(None);

            let tokens: Vec<String> = analysis
                .syllables
                .iter()
                .filter(|s| !s.foreign)
                .map(|s| s.jyutping.clone())
                .collect();
            let sylls = analysis.syllables.clone();

            spawn_local(async move {
                let fetched = engine::fetch_syllables(&tokens, &proxy).await;
                let mut parts: Vec<(String, AudioBuffer)> = Vec::new();
                for (token, buffer) in fetched {
                    if let Some(b) = buffer {
                        cache.put(&token, b.clone());
                        parts.push((token, b));
                    }
                }

                if parts.is_empty() {
                    let tried = engine::candidate_urls("nei5", &proxy)
                        .iter()
                        .filter_map(|u| {
                            u.split("://").nth(1).and_then(|r| r.split('/').next())
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    status.set(RefStatus::Failed(format!(
                        "No source could reach the words.hk recording (tried: {tried}). \
                         Run `python3 tools/proxy.py` for a local relay, or set a proxy in Audio Settings — \
                         meanwhile the browser's Cantonese voice will stand in."
                    )));
                    return;
                }

                let ctx = match engine::ctx() {
                    Ok(c) => c,
                    Err(e) => {
                        status.set(RefStatus::Failed(e));
                        return;
                    }
                };

                match engine::stitch(&ctx, &parts, 120.0) {
                    Ok(reference) => {
                        // Keep the playable buffer around: the chart only
                        // needs the contour, but the four playback modes need
                        // the audio itself.
                        audio.borrow_mut().reference = Some(reference.buffer.clone());
                        let samples = engine::buffer_samples(&reference.buffer);
                        let sr = ctx.sample_rate();
                        let contour = Contour::from_pcm(&samples, sr);
                        let total = engine::buffer_duration(&reference.buffer);
                        let targets = build_targets(&sylls, &reference.syllables, total);
                        let envelope: Vec<(f64, f64)> = contour
                            .times
                            .iter()
                            .cloned()
                            .zip(contour.rms.iter().cloned())
                            .collect();
                        let median = contour.median_f0;

                        {
                            let mut m = model.borrow_mut();
                            m.original = Some(contour);
                            m.original_rms = envelope;
                            m.original_duration = total;
                            m.original_median = median;
                            m.axis_seconds = total;
                            m.targets = targets;
                            m.play_duration = total;
                            m.total_duration = total;
                        }
                        bump(&model);
                        status.set(RefStatus::Ready("words.hk".to_string()));
                    }
                    Err(e) => status.set(RefStatus::Failed(e)),
                }
            });
            || ()
        });
    }

    // ── stop everything ───────────────────────────────────────────────────
    let stop_all = {
        let audio = audio.clone();
        let model = props.model.clone();
        let active = active.clone();
        Callback::from(move |_: MouseEvent| {
            {
                let mut a = audio.borrow_mut();
                for v in a.voices.drain(..) {
                    v.stop();
                }
                a.recorder = None;
            }
            let mut m = model.borrow_mut();
            m.playing = false;
            active.set(0);
        })
    };

    // ── mode 1: play the original ─────────────────────────────────────────
    let play_original = {
        let audio = audio.clone();
        let model = props.model.clone();
        let error = error.clone();
        let phrase = props.phrase.clone();
        let settings = base_settings.clone();
        let active = active.clone();
        Callback::from(move |_: MouseEvent| {
            let ctx = match engine::ctx() {
                Ok(c) => c,
                Err(e) => {
                    error.set(Some(e));
                    return;
                }
            };
            engine::resume(&ctx);

            let buffer = audio.borrow().reference.clone();
            let buffer = match buffer {
                Some(b) => b,
                None => {
                    // No reference audio — fall back to the browser's
                    // Cantonese voice so the phrase is still audible.
                    let rate = if settings.slow_tts { 0.72 } else { 1.0 };
                    let text = phrase.han.clone();
                    let voice = settings.tts_voice.clone();
                    let error = error.clone();
                    spawn_local(async move {
                        if let Err(e) = tts::speak_when_ready(text, rate, voice).await {
                            error.set(Some(match e {
                                tts::SpeakError::VoicesNotReady => "No reference recording, and the browser \
                                     is still loading its Cantonese voice — try again in a moment."
                                    .into(),
                                tts::SpeakError::NoCantoneseVoice => "No reference recording and no Cantonese \
                                     browser voice available."
                                    .into(),
                            }));
                        }
                    });
                    return;
                }
            };

            let dur = engine::buffer_duration(&buffer);
            let when = ctx.current_time() + 0.06;
            match engine::play(&ctx, &buffer, when, 1.0, 1.0, 0.0) {
                Ok(v) => {
                    {
                        let mut a = audio.borrow_mut();
                        for old in a.voices.drain(..) {
                            old.stop();
                        }
                        a.voices.push(v);
                    }
                    {
                        let mut m = model.borrow_mut();
                        m.playing = true;
                        m.play_start = when;
                        m.play_duration = dur;
                        m.play_is_original = true;
                        m.ab_mode = false;
                        m.total_duration = dur;
                        active.set(1);
                    }
                    bump(&model);
                }
                Err(e) => error.set(Some(e)),
            }
        })
    };

    // ── mode 2: play the learner's take ───────────────────────────────────
    let play_mine = {
        let audio = audio.clone();
        let model = props.model.clone();
        let error = error.clone();
        let settings = base_settings.clone();
        let active = active.clone();
        Callback::from(move |_: MouseEvent| {
            let ctx = match engine::ctx() {
                Ok(c) => c,
                Err(e) => {
                    error.set(Some(e));
                    return;
                }
            };
            engine::resume(&ctx);

            let (buffer, ref_dur) = {
                let a = audio.borrow();
                let dur = a.reference.as_ref().map(engine::buffer_duration).unwrap_or(0.0);
                (a.take.clone(), dur)
            };
            let buffer = match buffer {
                Some(b) => b,
                None => {
                    error.set(Some("Record a take first — there is nothing to play back.".into()));
                    return;
                }
            };
            let mine_dur = engine::buffer_duration(&buffer);
            let rate = if settings.time_sync && ref_dur > 0.0 {
                (mine_dur / ref_dur).clamp(0.5, 2.0)
            } else {
                1.0
            };
            let effective = mine_dur / rate;
            let when = ctx.current_time() + 0.06;

            match engine::play(&ctx, &buffer, when, rate as f32, 1.0, 0.0) {
                Ok(v) => {
                    {
                        let mut a = audio.borrow_mut();
                        for old in a.voices.drain(..) {
                            old.stop();
                        }
                        a.voices.push(v);
                    }
                    {
                        let mut m = model.borrow_mut();
                        m.playing = true;
                        m.play_start = when;
                        m.play_duration = effective;
                        m.play_is_original = false;
                        m.ab_mode = false;
                        m.total_duration = effective;
                        active.set(2);
                    }
                    bump(&model);
                }
                Err(e) => error.set(Some(e)),
            }
        })
    };

    // ── mode 3: both together ─────────────────────────────────────────────
    let play_both = {
        let audio = audio.clone();
        let model = props.model.clone();
        let error = error.clone();
        let settings = base_settings.clone();
        let active = active.clone();
        Callback::from(move |_: MouseEvent| {
            let ctx = match engine::ctx() {
                Ok(c) => c,
                Err(e) => {
                    error.set(Some(e));
                    return;
                }
            };
            engine::resume(&ctx);

            let (reference, take) = {
                let a = audio.borrow();
                (a.reference.clone(), a.take.clone())
            };
            let (reference, take) = match (reference, take) {
                (Some(r), Some(t)) => (r, t),
                _ => {
                    error.set(Some(
                        "You need both the reference recording and your own take for this mode.".into(),
                    ));
                    return;
                }
            };

            let ref_dur = engine::buffer_duration(&reference);
            let mine_dur = engine::buffer_duration(&take);
            let rate = if settings.time_sync && ref_dur > 0.0 {
                (mine_dur / ref_dur).clamp(0.5, 2.0)
            } else {
                1.0
            };
            let effective = mine_dur / rate;
            let (go, gm) = mix_gains(settings.balance);
            let (po, pm) = if settings.stereo_split {
                (-0.35, 0.35)
            } else {
                (0.0, 0.0)
            };
            let when = ctx.current_time() + 0.06;

            let a_voice = engine::play(&ctx, &reference, when, 1.0, go, po);
            let b_voice = engine::play(&ctx, &take, when, rate as f32, gm, pm);
            match (a_voice, b_voice) {
                (Ok(a), Ok(b)) => {
                    {
                        let mut guard = audio.borrow_mut();
                        for old in guard.voices.drain(..) {
                            old.stop();
                        }
                        guard.voices.push(a);
                        guard.voices.push(b);
                    }
                    {
                        let mut m = model.borrow_mut();
                        m.playing = true;
                        m.play_start = when;
                        m.play_duration = ref_dur.max(effective);
                        m.play_is_original = true;
                        m.ab_mode = false;
                        m.total_duration = m.play_duration;
                        active.set(3);
                    }
                    bump(&model);
                }
                (Err(e), _) | (_, Err(e)) => error.set(Some(e)),
            }
        })
    };

    // ── mode 4: sequential A/B ────────────────────────────────────────────
    let play_ab = {
        let audio = audio.clone();
        let model = props.model.clone();
        let error = error.clone();
        let settings = base_settings.clone();
        let active = active.clone();
        Callback::from(move |_: MouseEvent| {
            let ctx = match engine::ctx() {
                Ok(c) => c,
                Err(e) => {
                    error.set(Some(e));
                    return;
                }
            };
            engine::resume(&ctx);

            let (reference, take) = {
                let a = audio.borrow();
                (a.reference.clone(), a.take.clone())
            };
            let (reference, take) = match (reference, take) {
                (Some(r), Some(t)) => (r, t),
                _ => {
                    error.set(Some(
                        "You need both the reference recording and your own take for this mode.".into(),
                    ));
                    return;
                }
            };

            let ref_dur = engine::buffer_duration(&reference);
            let mine_dur = engine::buffer_duration(&take);
            let rate = if settings.time_sync && ref_dur > 0.0 {
                (mine_dur / ref_dur).clamp(0.5, 2.0)
            } else {
                1.0
            };
            let effective = mine_dur / rate;
            let (go, gm) = mix_gains(settings.balance);
            let when = ctx.current_time() + 0.06;
            let gap = 0.3;

            let a_voice = engine::play(&ctx, &reference, when, 1.0, go, 0.0);
            let b_voice = engine::play(&ctx, &take, when + ref_dur + gap, rate as f32, gm, 0.0);
            match (a_voice, b_voice) {
                (Ok(a), Ok(b)) => {
                    {
                        let mut guard = audio.borrow_mut();
                        for old in guard.voices.drain(..) {
                            old.stop();
                        }
                        guard.voices.push(a);
                        guard.voices.push(b);
                    }
                    {
                        let mut m = model.borrow_mut();
                        m.playing = true;
                        m.play_start = when;
                        m.play_duration = ref_dur;
                        m.play_is_original = true;
                        m.ab_mode = true;
                        m.ab_switch_at = ref_dur + gap;
                        m.ab_second_duration = effective;
                        m.total_duration = ref_dur + gap + effective;
                        active.set(4);
                    }
                    bump(&model);
                }
                (Err(e), _) | (_, Err(e)) => error.set(Some(e)),
            }
        })
    };

    // ── browser Cantonese TTS ─────────────────────────────────────────────
    let play_tts = {
        let phrase = props.phrase.clone();
        let settings = base_settings.clone();
        let error = error.clone();
        let speaking = speaking.clone();
        Callback::from(move |_: MouseEvent| {
            if *speaking {
                return;
            }
            speaking.set(true);
            let text = phrase.han.clone();
            let rate = if settings.slow_tts { 0.72 } else { 1.0 };
            let voice = settings.tts_voice.clone();
            let error = error.clone();
            let speaking = speaking.clone();
            spawn_local(async move {
                let result = tts::speak_when_ready(text, rate, voice).await;
                speaking.set(false);
                if let Err(e) = result {
                    error.set(Some(tts_error_message(e)));
                }
            });
        })
    };

    // ── recording ─────────────────────────────────────────────────────────
    let start_recording = {
        let audio = audio.clone();
        let model = props.model.clone();
        let recording = recording.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            let ctx = match engine::ctx() {
                Ok(c) => c,
                Err(e) => {
                    error.set(Some(e));
                    return;
                }
            };
            engine::resume(&ctx);

            let audio = audio.clone();
            let model = model.clone();
            let recording = recording.clone();
            let error = error.clone();

            spawn_local(async move {
                match Recorder::start(&ctx).await {
                    Ok(rec) => {
                        {
                            let mut a = audio.borrow_mut();
                            a.recorder = Some(rec);
                        }
                        {
                            let mut m = model.borrow_mut();
                            m.live = None;
                        }
                        recording.set(true);
                        bump(&model);
                    }
                    Err(e) => error.set(Some(e)),
                }
            });
        })
    };

    let stop_recording = {
        let audio = audio.clone();
        let model = props.model.clone();
        let recording = recording.clone();
        let has_take = has_take.clone();
        let trim = trim.clone();
        let error = error.clone();
        Callback::from(move |_: MouseEvent| {
            recording.set(false);
            let ctx = match engine::ctx() {
                Ok(c) => c,
                Err(_) => return,
            };

            let (samples, sr) = {
                let mut a = audio.borrow_mut();
                let rec = match a.recorder.take() {
                    Some(r) => r,
                    None => return,
                };
                let samples = rec.take_samples();
                let sr = rec.sample_rate;
                drop(rec); // releases the microphone
                (samples, sr)
            };

            if samples.len() < (sr as usize / 10) {
                error.set(Some(
                    "That take was too short — hold the button a little longer.".into(),
                ));
                return;
            }

            let mut trimmed = samples;
            let info = vad::trim(&mut trimmed, sr);
            trim.set(Some(info));

            let contour = Contour::from_pcm(&trimmed, sr);
            let buffer = match engine::samples_to_buffer(&ctx, &trimmed, sr) {
                Ok(b) => b,
                Err(e) => {
                    error.set(Some(e));
                    return;
                }
            };
            let duration = trimmed.len() as f64 / sr as f64;
            let envelope: Vec<(f64, f64)> = contour
                .times
                .iter()
                .cloned()
                .zip(contour.rms.iter().cloned())
                .collect();
            let median = contour.median_f0;

            {
                let mut a = audio.borrow_mut();
                a.take = Some(buffer);
            }
            {
                let mut m = model.borrow_mut();
                m.mine = Some(contour);
                m.mine_rms = envelope;
                m.mine_duration = duration;
                m.mine_median = median;
                m.live = None;
                m.axis_seconds = m.axis_seconds.max(duration);
            }
            has_take.set(true);
            bump(&model);
        })
    };

    // ── live pitch while recording ────────────────────────────────────────
    {
        let recording = recording.clone();
        let audio = audio.clone();
        let model = props.model.clone();
        let live_ms = live_ms.clone();

        use_effect_with(recording.clone(), move |is_recording| {
            let is_recording = **is_recording;
            let mut handle = None;
            if is_recording {
                live_ms.set(0);
                handle = web_sys::window().map(|w| {
                let audio = audio.clone();
                let model = model.clone();
                let live_ms = live_ms.clone();
                let cb = Closure::wrap(Box::new(move || {
                    let guard = audio.borrow();
                    if let Some(rec) = &guard.recorder {
                        let points = rec.live.borrow().clone();
                        let elapsed = rec.elapsed_ms.get() / 1000.0;
                        live_ms.set(rec.elapsed_ms.get() as u64);
                        let contour = build_live(&points, elapsed);
                        let mut m = model.borrow_mut();
                        m.live = Some(contour);
                        m.mine_duration = elapsed.max(0.001);
                        m.axis_seconds = m.axis_seconds.max(elapsed);
                    }
                }) as Box<dyn FnMut()>);
                    let id = w
                        .set_interval_with_callback_and_timeout_and_arguments_0(
                            cb.as_ref().unchecked_ref(),
                            100,
                        )
                        .unwrap_or(0);
                    (id, cb)
                });
            }

            move || {
                if let (Some(w), Some((id, _cb))) = (web_sys::window(), handle) {
                    w.clear_interval_with_handle(id);
                }
            }
        });
    }

    // ── the drill panel asked for a recording ─────────────────────────────
    {
        let model = props.model.clone();
        let recording = recording.clone();
        let error = error.clone();
        let audio = audio.clone();

        use_effect_with((), move |_| {
            let handle = web_sys::window().map(|w| {
                let model = model.clone();
                let recording = recording.clone();
                let error = error.clone();
                let audio = audio.clone();
                let cb = Closure::wrap(Box::new(move || {
                    let request = model.borrow().record_request;
                    if request == 0 || *recording {
                        return;
                    }
                    let ctx = match engine::ctx() {
                        Ok(c) => c,
                        Err(e) => {
                            error.set(Some(e));
                            return;
                        }
                    };
                    engine::resume(&ctx);

                    let audio = audio.clone();
                    let model = model.clone();
                    let recording = recording.clone();
                    let error = error.clone();

                    spawn_local(async move {
                        match Recorder::start(&ctx).await {
                            Ok(rec) => {
                                {
                                    let mut a = audio.borrow_mut();
                                    a.recorder = Some(rec);
                                }
                                {
                                    let mut m = model.borrow_mut();
                                    m.live = None;
                                    m.record_request = 0;
                                }
                                recording.set(true);
                                bump(&model);
                            }
                            Err(e) => {
                                let mut m = model.borrow_mut();
                                m.record_request = 0;
                                error.set(Some(e));
                            }
                        }
                    });
                }) as Box<dyn FnMut()>);
                let id = w
                    .set_interval_with_callback_and_timeout_and_arguments_0(
                        cb.as_ref().unchecked_ref(),
                        200,
                    )
                    .unwrap_or(0);
                (id, cb)
            });

            move || {
                if let (Some(w), Some((id, _cb))) = (web_sys::window(), handle) {
                    w.clear_interval_with_handle(id);
                }
            }
        });
    }

    // ── chart controls ────────────────────────────────────────────────────
    let set_time_mode = {
        let model = props.model.clone();
        let on_settings = props.on_settings.clone();
        let base = base_settings.clone();
        Callback::from(move |value: AttrValue| {
            let seconds = value == "seconds";
            {
                let mut m = model.borrow_mut();
                m.time_mode = if seconds {
                    TimeMode::Seconds
                } else {
                    TimeMode::Percent
                };
            }
            bump(&model);
            let mut next = base.clone();
            next.time_mode = if seconds { "seconds" } else { "percent" }.to_string();
            on_settings.emit(next);
        })
    };

    let set_zoom = {
        let model = props.model.clone();
        Callback::from(move |value: AttrValue| {
            {
                let mut m = model.borrow_mut();
                if value == "syllable" {
                    if m.focus.is_none() {
                        m.focus = Some(0);
                    }
                    m.zoom = Zoom::Syllable;
                } else {
                    m.zoom = Zoom::Phrase;
                }
            }
            bump(&model);
        })
    };

    let toggle_layer = {
        let model = props.model.clone();
        Callback::from(move |which: AttrValue| {
            {
                let mut m = model.borrow_mut();
                match which.as_str() {
                    "target" => m.show_target = !m.show_target,
                    "original" => m.show_original = !m.show_original,
                    _ => m.show_mine = !m.show_mine,
                }
            }
            bump(&model);
        })
    };

    // ── settings toggles ──────────────────────────────────────────────────
    let on_settings = props.on_settings.clone();

    let toggle_time_sync = {
        let on_settings = on_settings.clone();
        let base = base_settings.clone();
        Callback::from(move |value: bool| {
            let mut next = base.clone();
            next.time_sync = value;
            on_settings.emit(next);
        })
    };
    let toggle_split = {
        let on_settings = on_settings.clone();
        let base = base_settings.clone();
        Callback::from(move |value: bool| {
            let mut next = base.clone();
            next.stereo_split = value;
            on_settings.emit(next);
        })
    };
    let toggle_slow = {
        let on_settings = on_settings.clone();
        let base = base_settings.clone();
        Callback::from(move |value: bool| {
            let mut next = base.clone();
            next.slow_tts = value;
            on_settings.emit(next);
        })
    };
    let set_balance = {
        let on_settings = on_settings.clone();
        let base = base_settings.clone();
        Callback::from(move |e: InputEvent| {
            let target: web_sys::HtmlInputElement = e.target_unchecked_into();
            let value = target.value().parse::<f32>().unwrap_or(0.5);
            let mut next = base.clone();
            next.balance = value;
            on_settings.emit(next);
        })
    };

    // ── render ────────────────────────────────────────────────────────────
    let (status_class, status_text) = match &*status {
        RefStatus::Idle => ("status idle", "Waiting for the reference recording…".to_string()),
        RefStatus::Loading => (
            "status loading",
            "Fetching the native recording from words.hk…".to_string(),
        ),
        RefStatus::Ready(src) => ("status ready", format!("Reference ready · {src}")),
        RefStatus::Failed(e) => ("status failed", e.clone()),
    };

    let _rev = *rev;
    let model_snapshot = props.model.borrow().clone();
    let is_recording = *recording;
    let has_recording = *has_take;
    let trim_info = *trim;

    html! {
        <section class="studio card">
            <div class="studio-head">
                <div>
                    <h2 class="section-title">{ "Listen & Compare" }</h2>
                    <p class={status_class}>{ status_text }</p>
                </div>
                if let Some(err) = (*error).clone() {
                    <button type="button" class="err" onclick={{
                        let error = error.clone();
                        Callback::from(move |_| error.set(None))
                    }}>{ err.clone() }</button>
                }
            </div>

            <div class="playbar" role="group" aria-label="Playback modes">
                <button type="button" class={mode_class(1, *active)} onclick={play_original.clone()}>
                    <span class="mode-num">{ "1" }</span>
                    <span class="mode-label">{ "Play Original" }</span>
                    { Icon::Play.svg() }
                </button>
                <button type="button" class={mode_class(2, *active)} disabled={!has_recording} onclick={play_mine.clone()}>
                    <span class="mode-num">{ "2" }</span>
                    <span class="mode-label">{ "Play Mine" }</span>
                    { Icon::Play.svg() }
                </button>
                <button type="button" class={mode_class(3, *active)} disabled={!has_recording} onclick={play_both.clone()}>
                    <span class="mode-num">{ "3" }</span>
                    <span class="mode-label">{ "Play Both Together" }</span>
                    { Icon::Layers.svg() }
                </button>
                <button type="button" class={mode_class(4, *active)} disabled={!has_recording} onclick={play_ab.clone()}>
                    <span class="mode-num">{ "4" }</span>
                    <span class="mode-label">{ "Compare: Original → Mine" }</span>
                    { Icon::Wave.svg() }
                </button>
                <button type="button" class="mode stop" onclick={stop_all.clone()}>
                    { Icon::Stop.svg() }
                    <span class="mode-label">{ "Stop" }</span>
                </button>
            </div>

            <div class="studio-options">
                <Toggle label="Time-Sync Mine to Original" checked={props.settings.time_sync}
                       on_toggle={toggle_time_sync.clone()} />
                <Toggle label="Stereo split" checked={props.settings.stereo_split}
                       on_toggle={toggle_split.clone()}
                       hint="Original left, yours right" />
                <div class="balance">
                    <span class="balance-label">{ "Balance" }</span>
                    <span class="balance-ends">{ "Original" }</span>
                    <input type="range" min="0" max="100" step="1"
                           value={ (props.settings.balance * 100.0).round().to_string() }
                           oninput={set_balance.clone()}
                           aria-label="Balance between Original and Mine" />
                    <span class="balance-ends">{ "Mine" }</span>
                </div>
                <Toggle label="Browser Cantonese voice (slow 0.72×)" checked={props.settings.slow_tts}
                       on_toggle={toggle_slow.clone()} />
                <button type="button" class="btn tiny" disabled={*speaking} onclick={play_tts.clone()}>
                    { Icon::Music.svg() }
                    <span>{ if *speaking { "Getting the voice…" } else { "Speak the phrase" } }</span>
                </button>
            </div>

            <div class="chart-toolbar">
                <Segmented
                    label="Axis"
                    options={vec![AttrValue::from("percent"), AttrValue::from("seconds")]}
                    active={AttrValue::from(match model_snapshot.time_mode {
                        TimeMode::Percent => "percent",
                        TimeMode::Seconds => "seconds",
                    })}
                    on_select={set_time_mode.clone()} />
                <Segmented
                    label="Zoom"
                    options={vec![AttrValue::from("phrase"), AttrValue::from("syllable")]}
                    active={AttrValue::from(match model_snapshot.zoom {
                        Zoom::Phrase => "phrase",
                        Zoom::Syllable => "syllable",
                    })}
                    on_select={set_zoom.clone()} />
                <div class="layers">
                    <span class="layers-label">{ "Layers" }</span>
                    <button type="button"
                            class={if model_snapshot.show_target { "layer on target" } else { "layer target" }}
                            onclick={{
                                let cb = toggle_layer.clone();
                                Callback::from(move |_| cb.emit(AttrValue::from("target")))
                            }}>{ "Target" }</button>
                    <button type="button"
                            class={if model_snapshot.show_original { "layer on original" } else { "layer original" }}
                            onclick={{
                                let cb = toggle_layer.clone();
                                Callback::from(move |_| cb.emit(AttrValue::from("original")))
                            }}>{ "Original" }</button>
                    <button type="button"
                            class={if model_snapshot.show_mine { "layer on mine" } else { "layer mine" }}
                            onclick={{
                                let cb = toggle_layer.clone();
                                Callback::from(move |_| cb.emit(AttrValue::from("mine")))
                            }}>{ "Mine" }</button>
                </div>
            </div>

            <Chart model={props.model.clone()} on_finish={{
                let active = active.clone();
                Callback::from(move |_| active.set(0))
            }} />

            <div class="legend">
                <span class="key note">
                    { format!(
                        "Reference {} · {} syllable target{} · your take {}",
                        if model_snapshot.original_duration > 0.0 {
                            format!("{:.2} s", model_snapshot.original_duration)
                        } else {
                            "—".to_string()
                        },
                        model_snapshot.targets.len(),
                        if model_snapshot.targets.len() == 1 { "" } else { "s" },
                        if model_snapshot.mine_duration > 0.0 {
                            format!("{:.2} s", model_snapshot.mine_duration)
                        } else {
                            "not recorded".to_string()
                        }
                    ) }
                </span>
                <span class="key"><i class="swatch coral dash" />{ "Target — the tone contour to aim for" }</span>
                <span class="key"><i class="swatch clay" />{ "Original — native Cantonese recording" }</span>
                <span class="key"><i class="swatch green" />{ "Mine — your recording" }</span>
                if model_snapshot.targets.iter().any(|t| t.spoken_tone != t.tone) {
                    <span class="key"><i class="swatch dict dash" />{ "Dictionary tone — before the sandhi lift" }</span>
                }
                <span class="key note">
                    { "Pitch is shown in semitones relative to each speaker's own median, so the shapes are directly comparable." }
                </span>
            </div>

            <div class="recorder">
                if is_recording {
                    <button type="button" class="rec live" onclick={stop_recording.clone()}>
                        <span class="rec-dot" />
                        <span>{ "Stop — recording" }</span>
                        <span class="rec-time">{ format!("{} ms", *live_ms) }</span>
                    </button>
                } else {
                    <button type="button" class="rec" onclick={start_recording.clone()}>
                        { Icon::Mic.svg() }
                        <span>{ "Start a repeat" }</span>
                    </button>
                }
                <p class="rec-hint">
                    { if has_recording {
                        "Your take is trimmed of dead silence automatically, so it lines up with the original."
                    } else {
                        "Press record, say the phrase once, then press stop. The silence at each end is trimmed for you."
                    } }
                </p>
                if let Some(info) = trim_info {
                    if info.changed() {
                        <p class="trim-note">
                            { format!("Trimmed {:.0} ms of silence at the front and {:.0} ms at the back.",
                                      info.trimmed_front_ms, info.trimmed_back_ms) }
                        </p>
                    }
                }
            </div>
        </section>
    }
}
