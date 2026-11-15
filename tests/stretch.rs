//! Tests for the pitch-preserving time stretch.
//!
//! The stretch exists because the studio used to match the learner's take
//! to the reference with `playbackRate`, which changes pitch as well as
//! speed. These tests pin down the one property that matters: the pitch
//! must survive, and the length must land where it was asked to.

use singsin::dsp::pitch::yin;
use singsin::dsp::stretch::{stretch, stretch_to_duration};
use singsin::dsp::track::Contour;

const SR: f32 = 48_000.0;

/// A pure tone, the easiest case there is: any change of pitch would show
/// up immediately in the fundamental.
fn tone(freq: f32, seconds: f32) -> Vec<f32> {
    let n = (SR * seconds) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR;
            0.4 * (2.0 * std::f32::consts::PI * freq * t).sin()
        })
        .collect()
}

/// The fundamental of the middle of a signal, as a human ear would hear it.
fn heard_pitch(samples: &[f32]) -> f64 {
    let contour = Contour::from_pcm(samples, SR);
    contour.median_f0
}

#[test]
fn a_stretched_tone_keeps_its_pitch() {
    let input = tone(200.0, 1.0);
    let output = stretch(&input, SR, 1.5);

    assert!(
        (heard_pitch(&output) - 200.0).abs() < 4.0,
        "a tone stretched to 1.5x was heard at {} Hz",
        heard_pitch(&output)
    );
}

#[test]
fn a_compressed_tone_keeps_its_pitch() {
    let input = tone(200.0, 1.0);
    let output = stretch(&input, SR, 0.7);

    assert!(
        (heard_pitch(&output) - 200.0).abs() < 4.0,
        "a tone compressed to 0.7x was heard at {} Hz",
        heard_pitch(&output)
    );
}

#[test]
fn the_stretched_length_is_exact() {
    let input = tone(220.0, 0.5);
    let n = input.len();

    let longer = stretch(&input, SR, 1.5);
    assert_eq!(longer.len(), (n as f64 * 1.5).round() as usize);

    let shorter = stretch(&input, SR, 0.75);
    assert_eq!(shorter.len(), (n as f64 * 0.75).round() as usize);
}

#[test]
fn a_duration_target_is_hit() {
    let input = tone(180.0, 1.2);
    let output = stretch_to_duration(&input, SR, 2.0);

    let seconds = output.len() as f64 / SR as f64;
    assert!(
        (seconds - 2.0).abs() < 0.01,
        "asked for 2.0 s, got {} s",
        seconds
    );
}

#[test]
fn a_factor_of_one_changes_nothing() {
    let input = tone(200.0, 0.3);
    let output = stretch(&input, SR, 1.0);

    assert_eq!(output, input);
}

#[test]
fn the_level_survives_the_stretch() {
    let input = tone(200.0, 0.8);
    let output = stretch(&input, SR, 1.4);

    let peak = |s: &[f32]| s.iter().fold(0.0f32, |m, v| m.max(v.abs()));
    let before = peak(&input);
    let after = peak(&output);
    assert!(
        (after - before).abs() < before * 0.2,
        "peak went from {} to {}",
        before,
        after
    );
}

#[test]
fn the_join_carries_no_step() {
    // A click is a step in the waveform: two neighbouring samples that are
    // far apart. A sine of 200 Hz at 48 kHz moves by about 0.01 per
    // sample; a join would be orders of magnitude larger.
    let input = tone(200.0, 0.8);
    let output = stretch(&input, SR, 1.35);

    let biggest_step = output
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0f32, f32::max);
    let input_step = input
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0f32, f32::max);

    assert!(
        biggest_step < input_step * 3.0,
        "a step of {} appeared (the signal itself only moves {} per sample)",
        biggest_step,
        input_step
    );
}

#[test]
fn a_tone_stays_a_tone_after_stretching() {
    // The join has to be phase-aligned, not merely quiet: a mismatch would
    // show up as a harmonic the original did not have.
    let input = tone(200.0, 0.8);
    let output = stretch(&input, SR, 1.3);

    let window = &output[output.len() / 3..output.len() / 3 + 2048];
    let f0 = yin(window, SR, 0.15).expect("the stretched tone is still a tone");
    assert!(
        (f0 - 200.0).abs() < 4.0,
        "the fundamental moved to {} Hz",
        f0
    );
}

#[test]
fn a_signal_shorter_than_a_window_is_left_alone() {
    let input = tone(200.0, 0.01);
    let output = stretch(&input, SR, 2.0);
    assert_eq!(output, input);
}

#[test]
fn silence_stays_silent() {
    let input = vec![0.0f32; 8_000];
    let output = stretch(&input, SR, 1.5);

    assert_eq!(output.len(), 12_000);
    assert!(
        output.iter().all(|v| v.abs() < 1e-6),
        "silence grew into noise"
    );
}
