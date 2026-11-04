//! The Original-vs-Mine comparison chart.
//!
//! A single canvas carries four layers:
//!
//! 1. the schematic Chao tone targets (coral), optionally dashed where a
//!    colloquial sandhi lifts the tone;
//! 2. the reference recording's measured pitch curve and energy envelope
//!    (clay);
//! 3. the learner's measured pitch curve and envelope (green);
//! 4. a live playhead that sweeps while audio plays.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;
use yew::prelude::*;

use crate::audio::engine;
use crate::dsp::contour::tone_levels;
use crate::dsp::track::Contour;

// ── palette (mirrors the CSS custom properties) ─────────────────────────────
const INK: &str = "#263b37";
const MUTED: &str = "#77817a";
const GREEN: &str = "#246c5e";
const GREEN_SOFT: &str = "rgba(36,108,94,0.15)";
const CORAL: &str = "#d48164";
const CLAY: &str = "#c97b3f";
const CLAY_SOFT: &str = "rgba(201,123,63,0.14)";
/// The dictionary (pre-lift) tone. Kept a quiet grey so it never reads as
/// the Original recording, which is clay.
const DICT: &str = "#aab2ab";
const LINE: &str = "#e0e2d9";
const LINE_SOFT: &str = "#f0ebe0";

/// How the horizontal axis is scaled.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TimeMode {
    /// 0 – 100 % of the phrase.
    Percent,
    /// Real seconds.
    Seconds,
}

impl Default for TimeMode {
    fn default() -> Self {
        TimeMode::Percent
    }
}

/// How much of the phrase is shown.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Zoom {
    /// The whole phrase.
    Phrase,
    /// A single syllable.
    Syllable,
}

impl Default for Zoom {
    fn default() -> Self {
        Zoom::Phrase
    }
}

/// One schematic tone target.
#[derive(Clone, Debug, PartialEq)]
pub struct TargetSeg {
    /// Start of the syllable, as a fraction of the reference phrase.
    pub start: f64,
    /// End of the syllable, as a fraction of the reference phrase.
    pub end: f64,
    /// The dictionary tone.
    pub tone: u8,
    /// The tone as actually spoken.
    pub spoken_tone: u8,
    /// The character.
    pub han: String,
    /// The Jyutping.
    pub jyutping: String,
}

/// Everything the chart draws, shared with the studio.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChartData {
    /// Measured pitch of the reference recording.
    pub original: Option<Contour>,
    /// Measured pitch of the learner's take.
    pub mine: Option<Contour>,
    /// Live pitch while recording.
    pub live: Option<Contour>,
    /// Energy envelope of the reference: `(seconds, rms)`.
    pub original_rms: Vec<(f64, f64)>,
    /// Energy envelope of the learner's take.
    pub mine_rms: Vec<(f64, f64)>,
    /// Schematic tone targets.
    pub targets: Vec<TargetSeg>,
    /// Whether audio is currently playing.
    pub playing: bool,
    /// `AudioContext` time at which the current playback started.
    pub play_start: f64,
    /// Duration of the source that is currently playing.
    pub play_duration: f64,
    /// Total length of the current playback, including any A/B gap.
    pub total_duration: f64,
    /// Whether the current playback is the reference.
    pub play_is_original: bool,
    /// Whether this is a sequential A/B playback.
    pub ab_mode: bool,
    /// Elapsed time at which the A/B playback switches to the second voice.
    pub ab_switch_at: f64,
    /// Duration of the second voice in an A/B playback.
    pub ab_second_duration: f64,
    /// Bumped by the drill panel to ask the studio to start recording.
    pub record_request: u32,
    /// Axis scaling.
    pub time_mode: TimeMode,
    /// Zoom level.
    pub zoom: Zoom,
    /// The syllable the chart is zoomed into.
    pub focus: Option<usize>,
    /// Layer toggles.
    pub show_target: bool,
    pub show_original: bool,
    pub show_mine: bool,
    /// Durations.
    pub original_duration: f64,
    pub mine_duration: f64,
    /// Median pitch of each speaker, for the legend.
    pub original_median: f64,
    pub mine_median: f64,
    /// The longest duration on the axis, used by `Seconds` mode.
    pub axis_seconds: f64,
    /// Bumped whenever the data changes, so the chart knows to repaint.
    pub version: u32,
    /// Optional notifier: called with the new version whenever the model
    /// changes, so the surrounding UI can re-render and stay in step with
    /// the chart. The chart model is a plain `Rc<RefCell<…>>`, so without
    /// this the toolbar would keep showing the previous selection.
    pub notify: Option<Callback<u32>>,
}

/// Mark the chart model as changed: repaint the chart and tell the UI.
pub fn bump(model: &ChartModel) {
    let (version, notify) = {
        let mut m = model.borrow_mut();
        m.version = m.version.wrapping_add(1);
        (m.version, m.notify.clone())
    };
    // Emitted outside the borrow so the listener may read the model.
    if let Some(cb) = notify {
        cb.emit(version);
    }
}

/// Shared, mutable chart state.
pub type ChartModel = Rc<RefCell<ChartData>>;

#[derive(Properties, PartialEq)]
pub struct Props {
    pub model: ChartModel,
    /// Fired when a scheduled playback runs past its end.
    pub on_finish: Callback<()>,
}

/// The audio clock, in seconds.
///
/// Read from the `AudioContext` so the playhead lines up with what is
/// actually being heard. Kept in its own function so the rest of the
/// chart logic can be tested with a fixed clock.
fn now_seconds() -> f64 {
    engine::ctx().map(|c| c.current_time() as f64).unwrap_or(0.0)
}

/// Where the playhead is, given the audio clock.
fn playhead_fraction_at(data: &ChartData, now: f64) -> Option<f64> {
    if !data.playing {
        return None;
    }
    let elapsed = (now - data.play_start).max(0.0);

    // In an A/B playback the second voice takes over after the first.
    let (source_elapsed, source_duration) = if data.ab_mode && elapsed >= data.ab_switch_at {
        (elapsed - data.ab_switch_at, data.ab_second_duration)
    } else {
        (elapsed, data.play_duration)
    };

    Some(axis_fraction(data, source_elapsed, source_duration))
}

/// The target segment the playhead is currently over.
///
/// Falls back to the nearest segment so the window keeps moving through the
/// small gaps between syllables.
/// The target segment the playhead is over, given the audio clock.
fn active_segment_at(data: &ChartData, now: f64) -> Option<usize> {
    let frac = playhead_fraction_at(data, now)?;
    let mut best: Option<(usize, f64)> = None;
    for (i, seg) in data.targets.iter().enumerate() {
        let a = target_fraction(data, seg.start);
        let b = target_fraction(data, seg.end);
        if frac >= a && frac <= b {
            return Some(i);
        }
        let d = if frac < a { a - frac } else { frac - b };
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

/// The visible horizontal window, given the audio clock.
fn window_at(data: &ChartData, now: f64) -> (f64, f64) {
    if data.zoom == Zoom::Syllable {
        // While audio plays the zoomed window follows the playhead, so every
        // syllable gets its turn instead of the first one holding the screen.
        let idx = active_segment_at(data, now).or(data.focus);
        if let Some(seg) = idx.and_then(|i| data.targets.get(i)) {
            let pad = 0.012;
            let a = (target_fraction(data, seg.start) - pad).max(0.0);
            let b = (target_fraction(data, seg.end) + pad).min(1.0);
            if b > a {
                return (a, b);
            }
        }
    }
    (0.0, 1.0)
}

/// Convert a time in seconds to a fraction of the horizontal axis.
fn axis_fraction(data: &ChartData, seconds: f64, source_duration: f64) -> f64 {
    match data.time_mode {
        TimeMode::Percent => {
            if source_duration > 0.0 {
                seconds / source_duration
            } else {
                0.0
            }
        }
        TimeMode::Seconds => {
            if data.axis_seconds > 0.0 {
                seconds / data.axis_seconds
            } else {
                0.0
            }
        }
    }
}

/// Map a target's reference-fraction onto the current axis.
fn target_fraction(data: &ChartData, fraction: f64) -> f64 {
    match data.time_mode {
        TimeMode::Percent => fraction,
        TimeMode::Seconds => {
            if data.axis_seconds > 0.0 {
                fraction * data.original_duration / data.axis_seconds
            } else {
                fraction
            }
        }
    }
}

/// The vertical range of the chart, in semitones.
///
/// The 5th and 95th percentiles are used rather than the extremes, so one
/// stray frame cannot squash the whole contour.
fn semitone_range(data: &ChartData) -> (f64, f64) {
    let mut values: Vec<f64> = Vec::new();
    let mut consider = |c: &Option<Contour>| {
        if let Some(c) = c {
            for s in &c.semis {
                if !s.is_nan() {
                    values.push(*s);
                }
            }
        }
    };
    consider(&data.original);
    consider(&data.mine);
    consider(&data.live);

    if values.is_empty() {
        return (-6.0, 6.0);
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len();
    let lo = values[(n * 5 / 100).min(n - 1)];
    let hi = values[(n * 95 / 100).min(n - 1)];

    let span = (hi - lo).max(7.0);
    let mid = (lo + hi) / 2.0;
    (mid - span / 2.0 - 1.0, mid + span / 2.0 + 1.0)
}

/// Paint the whole chart.
fn paint(canvas: &web_sys::HtmlCanvasElement, data: &ChartData) {
    let dpr = web_sys::window()
        .map(|w| w.device_pixel_ratio())
        .unwrap_or(1.0)
        .max(1.0);
    let css_w = canvas.client_width() as f64;
    let css_h = canvas.client_height() as f64;
    if css_w < 40.0 || css_h < 40.0 {
        return;
    }

    let bw = (css_w * dpr).round() as u32;
    let bh = (css_h * dpr).round() as u32;
    if canvas.width() != bw || canvas.height() != bh {
        canvas.set_width(bw);
        canvas.set_height(bh);
    }

    let ctx: CanvasRenderingContext2d = match canvas
        .get_context("2d")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<CanvasRenderingContext2d>().ok())
    {
        Some(c) => c,
        None => return,
    };
    let _ = ctx.set_transform(dpr, 0.0, 0.0, dpr, 0.0, 0.0);
    let _ = ctx.clear_rect(0.0, 0.0, css_w, css_h);

    let pad_l = 44.0;
    let pad_r = 16.0;
    let pad_t = 16.0;
    // Syllable zoom gets a taller footer: it names the syllable on show.
    let pad_b = if data.zoom == Zoom::Syllable { 62.0 } else { 34.0 };
    let pw = (css_w - pad_l - pad_r).max(10.0);
    let ph = (css_h - pad_t - pad_b).max(10.0);

    // One clock reading per repaint, so the window, the playhead and the
    // syllable name can never disagree about where we are.
    let now = now_seconds();
    let (win_a, win_b) = window_at(data, now);
    let win = (win_b - win_a).max(1e-6);
    let (y_lo, y_hi) = semitone_range(data);
    let y_span = (y_hi - y_lo).max(0.001);

    let x_of = |frac: f64| pad_l + (((frac - win_a) / win).clamp(-0.15, 1.15)) * pw;
    let y_of = |semi: f64| pad_t + ph - (((semi - y_lo) / y_span).clamp(-0.15, 1.15)) * ph;

    // ── background ───────────────────────────────────────────────────────
    let _ = ctx.set_fill_style_str("rgba(255,254,251,0.75)");
    let _ = ctx.fill_rect(pad_l, pad_t, pw, ph);

    // ── horizontal grid (semitones) ──────────────────────────────────────
    let _ = ctx.set_line_width(1.0);
    let _ = ctx.set_font("11px 'JetBrains Mono', ui-monospace, monospace");
    let _ = ctx.set_text_baseline("middle");
    let step = if y_span > 20.0 { 6.0 } else { 3.0 };
    let mut tick = (y_lo / step).ceil() * step;
    while tick <= y_hi {
        let y = y_of(tick);
        let _ = ctx.set_stroke_style_str(if tick.abs() < 0.01 { LINE } else { LINE_SOFT });
        let _ = ctx.begin_path();
        let _ = ctx.move_to(pad_l, y);
        let _ = ctx.line_to(pad_l + pw, y);
        let _ = ctx.stroke();
        let _ = ctx.set_fill_style_str(MUTED);
        let _ = ctx.set_text_align("right");
        let label = if tick.abs() < 0.01 {
            "0".to_string()
        } else {
            format!("{:+.0}", tick)
        };
        let _ = ctx.fill_text(&label, pad_l - 8.0, y);
        tick += step;
    }

    // ── syllable boundaries ──────────────────────────────────────────────
    let _ = ctx.set_stroke_style_str(LINE_SOFT);
    let _ = ctx.set_line_width(1.0);
    for seg in &data.targets {
        let x = x_of(target_fraction(data, seg.start));
        let _ = ctx.begin_path();
        let _ = ctx.move_to(x, pad_t);
        let _ = ctx.line_to(x, pad_t + ph);
        let _ = ctx.stroke();
    }

    // ── x axis ───────────────────────────────────────────────────────────
    let _ = ctx.set_text_align("center");
    let _ = ctx.set_text_baseline("top");
    let _ = ctx.set_fill_style_str(MUTED);
    let ticks = 5;
    for i in 0..=ticks {
        let f = i as f64 / ticks as f64;
        let x = pad_l + f * pw;
        let _ = ctx.set_stroke_style_str(LINE_SOFT);
        let _ = ctx.begin_path();
        let _ = ctx.move_to(x, pad_t + ph);
        let _ = ctx.line_to(x, pad_t + ph + 4.0);
        let _ = ctx.stroke();
        // The labels follow the visible window, so a zoomed-in axis still
        // reads as real positions along the phrase rather than 0–100%.
        let label = match data.time_mode {
            TimeMode::Percent => {
                format!("{}%", ((win_a + f * win) * 100.0).round() as i32)
            }
            TimeMode::Seconds => {
                let secs = (win_a + f * win) * data.axis_seconds;
                format!("{secs:.1}s")
            }
        };
        let _ = ctx.fill_text(&label, x, pad_t + ph + 8.0);
    }

    // ── envelopes ────────────────────────────────────────────────────────
    let envelope_h = ph * 0.16;

    let draw_envelope = |ctx: &CanvasRenderingContext2d,
                        pts: &[(f64, f64)],
                        duration: f64,
                        color: &str| {
        if pts.len() < 2 {
            return;
        }
        let peak = pts.iter().cloned().fold(0.0f64, |a, (_, v)| a.max(v));
        if peak <= 0.0 {
            return;
        }
        let _ = ctx.set_fill_style_str(color);
        let _ = ctx.begin_path();
        for (i, (t, v)) in pts.iter().enumerate() {
            let frac = axis_fraction(data, *t, duration);
            let x = x_of(frac);
            let y = pad_t + ph - (v / peak).min(1.0) * envelope_h;
            if i == 0 {
                let _ = ctx.move_to(x, y);
            } else {
                let _ = ctx.line_to(x, y);
            }
        }
        let _ = ctx.line_to(x_of(axis_fraction(data, pts[pts.len() - 1].0, duration)), pad_t + ph);
        let _ = ctx.line_to(x_of(axis_fraction(data, pts[0].0, duration)), pad_t + ph);
        let _ = ctx.close_path();
        let _ = ctx.fill();
    };

    if data.show_original && !data.original_rms.is_empty() {
        draw_envelope(&ctx, &data.original_rms, data.original_duration, CLAY_SOFT);
    }
    if data.show_mine && !data.mine_rms.is_empty() {
        draw_envelope(&ctx, &data.mine_rms, data.mine_duration, GREEN_SOFT);
    }

    // ── schematic Chao targets ───────────────────────────────────────────
    // The target is dashed: it is a shape to aim at, not a measurement.
    if data.show_target {
        for seg in &data.targets {
            let x0 = x_of(target_fraction(data, seg.start));
            let x1 = x_of(target_fraction(data, seg.end));

            // The dictionary tone, dashed and grey — only meaningful when it
            // lifts, and deliberately not the colour of the Original curve.
            if seg.spoken_tone != seg.tone {
                let (f, t) = tone_levels(seg.tone);
                stroke_tone(&ctx, x0, x1, f, t, y_lo, y_hi, &y_of, DICT, true, 1.3);
            }
            let (f, t) = tone_levels(seg.spoken_tone);
            stroke_tone(&ctx, x0, x1, f, t, y_lo, y_hi, &y_of, CORAL, true, 2.4);
        }
    }

    // ── measured curves ──────────────────────────────────────────────────
    if data.show_original {
        if let Some(c) = &data.original {
            stroke_contour(&ctx, c, data.original_duration, &x_of, &y_of, CLAY, 2.8);
        }
    }
    if data.show_mine {
        if let Some(c) = &data.live {
            stroke_contour(&ctx, c, data.mine_duration.max(0.001), &x_of, &y_of, GREEN, 2.0);
        }
        if let Some(c) = &data.mine {
            stroke_contour(&ctx, c, data.mine_duration.max(0.001), &x_of, &y_of, GREEN, 2.4);
        }
    }

    // ── playhead ─────────────────────────────────────────────────────────
    if let Some(axis_frac) = playhead_fraction_at(data, now) {
        let x = x_of(axis_frac);

        let _ = ctx.set_stroke_style_str("rgba(38,59,55,0.72)");
        let _ = ctx.set_line_width(1.4);
        let _ = ctx.begin_path();
        let _ = ctx.move_to(x, pad_t - 6.0);
        let _ = ctx.line_to(x, pad_t + ph + 4.0);
        let _ = ctx.stroke();

        let _ = ctx.set_fill_style_str(INK);
        let _ = ctx.begin_path();
        let _ = ctx.move_to(x - 5.0, pad_t - 12.0);
        let _ = ctx.line_to(x + 5.0, pad_t - 12.0);
        let _ = ctx.line_to(x, pad_t - 5.0);
        let _ = ctx.close_path();
        let _ = ctx.fill();
    }

    // ── frame ────────────────────────────────────────────────────────────
    let _ = ctx.set_stroke_style_str(LINE);
    let _ = ctx.set_line_width(1.0);
    let _ = ctx.stroke_rect(pad_l, pad_t, pw, ph);

    // ── syllable labels under the axis ───────────────────────────────────
    if data.zoom == Zoom::Phrase {
        let _ = ctx.set_text_align("center");
        let _ = ctx.set_text_baseline("top");
        let _ = ctx.set_fill_style_str(MUTED);
        let _ = ctx.set_font("13px 'Noto Serif HK', serif");
        for seg in &data.targets {
            let mid = (target_fraction(data, seg.start) + target_fraction(data, seg.end)) / 2.0;
            let x = x_of(mid);
            if x < pad_l - 2.0 || x > pad_l + pw + 2.0 {
                continue;
            }
            let _ = ctx.fill_text(&seg.han, x, pad_t + ph + 20.0);
        }
    } else if let Some(idx) = active_segment_at(data, now).or(data.focus) {
        // In syllable zoom, name the syllable that is on show — while playing
        // this follows the playhead, so you always know which one it is.
        if let Some(seg) = data.targets.get(idx) {
            let x = pad_l + pw / 2.0;
            let _ = ctx.set_text_align("center");
            let _ = ctx.set_text_baseline("top");
            let _ = ctx.set_fill_style_str(INK);
            let _ = ctx.set_font("17px 'Noto Serif HK', serif");
            let _ = ctx.fill_text(&seg.han, x, pad_t + ph + 24.0);
            let _ = ctx.set_fill_style_str(MUTED);
            let _ = ctx.set_font("11px 'JetBrains Mono', ui-monospace, monospace");
            let _ = ctx.fill_text(&seg.jyutping, x, pad_t + ph + 45.0);
        }
    }
}

/// Map a Chao level (1 = lowest, 5 = highest) onto the chart's semitone range.
fn chao_to_semi(level: f64, y_lo: f64, y_hi: f64) -> f64 {
    y_lo + (level.clamp(1.0, 5.0) - 1.0) / 4.0 * (y_hi - y_lo)
}

/// Stroke one schematic tone segment.
fn stroke_tone<F>(
    ctx: &CanvasRenderingContext2d,
    x0: f64,
    x1: f64,
    from_level: f64,
    to_level: f64,
    y_lo: f64,
    y_hi: f64,
    y_of: &F,
    color: &str,
    dashed: bool,
    width: f64,
) where
    F: Fn(f64) -> f64,
{
    let ya = y_of(chao_to_semi(from_level, y_lo, y_hi));
    let yb = y_of(chao_to_semi(to_level, y_lo, y_hi));

    let _ = ctx.set_stroke_style_str(color);
    let _ = ctx.set_line_width(width);
    let _ = ctx.set_line_cap("round");
    if dashed {
        set_dash(ctx, &[4.0, 4.0]);
    } else {
        set_dash(ctx, &[]);
    }

    let dx = (x1 - x0).max(1.0);
    let _ = ctx.begin_path();
    let _ = ctx.move_to(x0, ya);
    let _ = ctx.bezier_curve_to(x0 + dx * 0.33, ya, x0 + dx * 0.66, yb, x1, yb);
    let _ = ctx.stroke();
    set_dash(ctx, &[]);
}

/// Apply a line-dash pattern to the context.
fn set_dash(ctx: &CanvasRenderingContext2d, dashes: &[f64]) {
    let array = js_sys::Array::new();
    for d in dashes {
        array.push(&JsValue::from_f64(*d));
    }
    let _ = ctx.set_line_dash(array.as_ref());
}

/// Stroke a measured pitch contour.
fn stroke_contour<F, G>(
    ctx: &CanvasRenderingContext2d,
    contour: &Contour,
    duration: f64,
    x_of: &F,
    y_of: &G,
    color: &str,
    width: f64,
) where
    F: Fn(f64) -> f64,
    G: Fn(f64) -> f64,
{
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for (i, t) in contour.times.iter().enumerate() {
        let s = contour.semis[i];
        if s.is_nan() {
            continue;
        }
        let frac = if duration > 0.0 {
            t / duration
        } else {
            0.0
        };
        pts.push((x_of(frac), y_of(s)));
    }
    if pts.len() < 2 {
        return;
    }

    let _ = ctx.set_stroke_style_str(color);
    let _ = ctx.set_line_width(width);
    let _ = ctx.set_line_join("round");
    let _ = ctx.set_line_cap("round");
    let _ = ctx.begin_path();
    let _ = ctx.move_to(pts[0].0, pts[0].1);
    for i in 1..pts.len() - 1 {
        let mid_x = (pts[i].0 + pts[i + 1].0) / 2.0;
        let mid_y = (pts[i].1 + pts[i + 1].1) / 2.0;
        let _ = ctx.quadratic_curve_to(pts[i].0, pts[i].1, mid_x, mid_y);
    }
    let last = pts[pts.len() - 1];
    let _ = ctx.line_to(last.0, last.1);
    let _ = ctx.stroke();
}

#[function_component(Chart)]
pub fn chart(props: &Props) -> Html {
    let canvas_ref = use_node_ref();
    let last_version = use_mut_ref(|| 0u32);

    // Keep the playhead moving while audio plays, and repaint whenever the
    // studio (or a syllable card) changes the data.
    {
        let model = props.model.clone();
        let canvas_ref = canvas_ref.clone();
        let last_version = last_version.clone();
        let on_finish = props.on_finish.clone();
        use_effect_with((), move |_| {
            let handle = web_sys::window().map(|w| {
                let model = model.clone();
                let canvas_ref = canvas_ref.clone();
                let last_version = last_version.clone();
                let on_finish = on_finish.clone();
                let cb = Closure::wrap(Box::new(move || {
                    let (playing, has_live, version) = {
                        let m = model.borrow();
                        (m.playing, m.live.is_some(), m.version)
                    };
                    let changed = *last_version.borrow() != version;
                    if changed {
                        *last_version.borrow_mut() = version;
                    }
                    if !playing && !has_live && !changed {
                        return;
                    }

                    // Stop the sweep once the audio has finished.
                    if playing {
                        let now = engine::ctx().map(|c| c.current_time() as f64).unwrap_or(0.0);
                        let over = {
                            let m = model.borrow();
                            m.total_duration > 0.0 && now - m.play_start > m.total_duration + 0.15
                        };
                        if over {
                            let mut m = model.borrow_mut();
                            m.playing = false;
                            on_finish.emit(());
                        }
                    }

                    if let Some(canvas) = canvas_ref.cast::<web_sys::HtmlCanvasElement>() {
                        let data = model.borrow().clone();
                        paint(&canvas, &data);
                    }
                }) as Box<dyn FnMut()>);
                let id = w
                    .set_interval_with_callback_and_timeout_and_arguments_0(
                        cb.as_ref().unchecked_ref(),
                        33,
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

    // Repaint on resize.
    {
        let model = props.model.clone();
        let canvas_ref = canvas_ref.clone();
        use_effect_with((), move |_| {
            let closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
                if let Some(canvas) = canvas_ref.cast::<web_sys::HtmlCanvasElement>() {
                    let data = model.borrow().clone();
                    paint(&canvas, &data);
                }
            }) as Box<dyn FnMut(web_sys::Event)>);
            if let Some(w) = web_sys::window() {
                let _ =
                    w.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref());
            }
            move || {
                if let Some(w) = web_sys::window() {
                    let _ = w.remove_event_listener_with_callback(
                        "resize",
                        closure.as_ref().unchecked_ref(),
                    );
                }
            }
        });
    }

    html! {
        <div class="chart-frame">
            <canvas ref={canvas_ref} class="chart-canvas" role="img"
                    aria-label="Pitch comparison chart: the target tone contour in coral, the native Cantonese recording in clay, and your own recording in green" />
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A phrase of four evenly spread syllables, as the studio builds them.
    fn four_syllables() -> Vec<TargetSeg> {
        ["你", "好", "我", "叫"]
            .iter()
            .enumerate()
            .map(|(i, han)| TargetSeg {
                start: i as f64 * 0.25,
                end: (i + 1) as f64 * 0.25,
                tone: 5,
                spoken_tone: 5,
                han: han.to_string(),
                jyutping: "x".to_string(),
            })
            .collect()
    }

    fn playing_data() -> ChartData {
        ChartData {
            targets: four_syllables(),
            playing: true,
            play_start: 0.0,
            play_duration: 2.0,
            original_duration: 2.0,
            axis_seconds: 2.0,
            time_mode: TimeMode::Percent,
            zoom: Zoom::Syllable,
            focus: Some(0),
            ..Default::default()
        }
    }

    #[test]
    fn syllable_zoom_follows_the_playhead_across_syllables() {
        let data = playing_data();
        // One reading per syllable: the window has to move every time.
        let seen: Vec<f64> = [0.1, 0.6, 1.1, 1.6]
            .iter()
            .map(|t| window_at(&data, *t).0)
            .collect();
        for pair in seen.windows(2) {
            assert!(
                pair[1] > pair[0],
                "the window must advance to the next syllable, got {seen:?}"
            );
        }
        // The first window sits at the start, the last at the end.
        assert!(seen[0] < 0.01, "first window should start at the left edge");
        assert!(seen[3] > 0.7, "last window should sit over the final syllable");
    }

    #[test]
    fn each_syllable_gets_its_own_window() {
        let data = playing_data();
        for (i, t) in [0.1, 0.6, 1.1, 1.6].iter().enumerate() {
            let (a, b) = window_at(&data, *t);
            let centre = (a + b) / 2.0;
            let expected = i as f64 * 0.25 + 0.125;
            assert!(
                (centre - expected).abs() < 0.05,
                "syllable {i}: window {a}..{b} is not over {expected}"
            );
        }
    }

    #[test]
    fn a_stopped_chart_falls_back_to_the_chosen_syllable() {
        let mut data = playing_data();
        data.playing = false;
        data.focus = Some(2);
        let (a, b) = window_at(&data, 1.1);
        assert!((a - 0.488).abs() < 0.02, "expected the third syllable, got {a}..{b}");
    }

    #[test]
    fn phrase_zoom_always_shows_the_whole_phrase() {
        let mut data = playing_data();
        data.zoom = Zoom::Phrase;
        assert_eq!(window_at(&data, 1.1), (0.0, 1.0));
    }
}
