//! Pitch-preserving time stretching (WSOLA).
//!
//! # Why this module exists
//!
//! The studio used to line the learner's take up with the reference by
//! setting `playbackRate` on the audio source. `playbackRate` is a
//! *resampling* control: it changes speed and pitch together, exactly like
//! a tape machine played at the wrong speed. A learner who said the phrase
//! a little faster than the native recording therefore heard themselves an
//! octave below — and in an app whose entire subject is pitch, that is not
//! a cosmetic wart. The chart draws the learner's real contour (it is
//! computed from the untouched take) while the speaker plays a shifted
//! copy of it, so the ear is taught the wrong shape.
//!
//! WSOLA — waveform similarity overlap-add — stretches *time* without
//! touching pitch. It cuts the signal into short overlapping windows and
//! lays them down again at a different spacing. Every window is copied
//! verbatim, so every partial keeps its frequency; only the timing of the
//! pieces changes. The one thing that has to be got right is the joins:
//! if a window begins where the previous one did not expect it to, the
//! overlap add produces a step, and a step is a click. WSOLA therefore
//! searches a small neighbourhood around the expected position for the
//! window whose beginning continues the previous one most faithfully, and
//! takes that one instead.
//!
//! The result is a take that lasts as long as the reference and sounds
//! like the learner, at the pitch they actually spoke.

/// Length of one analysis / synthesis window, in milliseconds.
///
/// 40 ms is long enough to hold a couple of pitch periods of a low voice
/// (so a window contains whole cycles and the overlap-add does not smear
/// a single period) and short enough to follow the rhythm of speech.
const FRAME_MS: f64 = 40.0;

/// How far the search may wander from the expected position, in
/// milliseconds.
///
/// The search exists to undo the drift that accumulates when a window is
/// taken slightly early or late: six milliseconds is about a tenth of a
/// syllable, enough to re-align the waveform without letting the window
/// jump to an unrelated sound.
const SEARCH_MS: f64 = 6.0;

/// How much of the window is compared when looking for the best match.
///
/// The whole overlap region is compared in principle, but a tenth of it
/// carries the same information for a fraction of the work: the search
/// runs once per window, and on a three-second take that is the difference
/// between a pause you notice and one you do not.
const TEMPLATE_MS: f64 = 10.0;

/// The coarse search steps two samples at a time and then refines.
///
/// Searching every sample would be exact and twice as slow; the coarse
/// pass finds the neighbourhood and the refinement picks the best sample
/// inside it.
const COARSE_STEP: usize = 2;

/// A periodic Hann window of `len` samples.
///
/// Two of these overlapping by half add up to exactly one, which is what
/// keeps the level of the stretched signal the same as the level of the
/// original.
fn hann(len: usize) -> Vec<f32> {
    (0..len)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / len as f32).cos())
        .collect()
}

/// The normalised correlation of two equal-length signals, in `[-1, 1]`.
///
/// Normalising by both energies means the score answers "do these two
/// waveforms have the same shape?", not "is this one louder?" — which is
/// the question that matters when the candidate window sits at a
/// different point in a syllable.
fn correlation(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut energy_a = 0.0f32;
    let mut energy_b = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        energy_a += a[i] * a[i];
        energy_b += b[i] * b[i];
    }
    if energy_a <= 1e-12 || energy_b <= 1e-12 {
        // Silence correlates with nothing; the caller keeps the expected
        // position rather than chasing an arbitrary maximum.
        return 0.0;
    }
    dot / (energy_a.sqrt() * energy_b.sqrt())
}

/// The position, near `centre`, of the window that continues `previous`
/// most smoothly.
///
/// The template is the tail of the window that was used last: that is the
/// stretch of sound the new window will be cross-faded with, so a
/// candidate whose beginning matches it joins without a step.
fn best_match(
    samples: &[f32],
    centre: usize,
    previous: usize,
    radius: usize,
    template_len: usize,
    hop: usize,
) -> usize {
    let n = samples.len();
    let tail_start = previous + hop;
    if tail_start + template_len > n {
        return centre.min(n.saturating_sub(1));
    }
    let template = &samples[tail_start..tail_start + template_len];

    let low = centre.saturating_sub(radius);
    let high = (centre + radius).min(n.saturating_sub(1));

    let mut best = low;
    let mut best_score = f32::NEG_INFINITY;
    let mut pos = low;
    while pos <= high {
        if pos + template_len <= n {
            let score = correlation(template, &samples[pos..pos + template_len]);
            if score > best_score {
                best_score = score;
                best = pos;
            }
        }
        pos += COARSE_STEP;
    }

    // Refine around the coarse winner, one sample at a time.
    let mut refined = best;
    let mut refined_score = best_score;
    let from = best.saturating_sub(COARSE_STEP);
    let to = (best + COARSE_STEP).min(high);
    for p in from..=to {
        if p + template_len <= n {
            let score = correlation(template, &samples[p..p + template_len]);
            if score > refined_score {
                refined_score = score;
                refined = p;
            }
        }
    }
    refined
}

/// Stretch `samples` by `factor`, keeping their pitch.
///
/// A factor above one makes the signal longer (slower speech), below one
/// makes it shorter. The output is exactly `round(len * factor)` samples
/// long, except when the input is too short to be cut into windows, in
/// which case it is returned untouched.
pub fn stretch(samples: &[f32], sample_rate: f32, factor: f32) -> Vec<f32> {
    let n = samples.len();
    if n == 0 || sample_rate <= 0.0 || !factor.is_finite() || factor <= 0.0 {
        return samples.to_vec();
    }

    // A factor this close to one changes nothing worth hearing, and
    // leaving the samples alone keeps them bit-identical.
    if (factor - 1.0).abs() < 0.001 {
        return samples.to_vec();
    }

    let frame = ((sample_rate as f64 * FRAME_MS / 1000.0).round() as usize).max(4);
    if n < frame {
        return samples.to_vec();
    }
    let hop = frame / 2;

    // How far the input advances for every output hop. The output is
    // `factor` times longer, so the input has to be consumed `factor`
    // times *slower*: the hop into the input is the output hop divided by
    // the factor.
    let hop_in = ((hop as f64 / factor as f64).round() as usize).max(1);

    let radius = ((sample_rate as f64 * SEARCH_MS / 1000.0).round() as usize).max(1);
    let template_len = ((sample_rate as f64 * TEMPLATE_MS / 1000.0).round() as usize)
        .max(1)
        .min(hop);

    let out_len = ((n as f64 * factor as f64).round() as usize).max(1);
    let window = hann(frame);

    // The output is one window longer than it needs to be, so the last
    // window can be written whole and the tail cut afterwards.
    let mut out = vec![0.0f32; out_len + frame];
    let mut weights = vec![0.0f32; out_len + frame];

    let mut out_pos = 0usize;
    let mut previous = 0usize;

    while out_pos < out_len {
        let position = if out_pos == 0 {
            // Nothing has been played yet, so there is no previous window
            // to continue: start at the beginning of the signal.
            0
        } else {
            let centre = (previous + hop_in).min(n.saturating_sub(1));
            best_match(samples, centre, previous, radius, template_len, hop)
        };

        for i in 0..frame {
            let src = position + i;
            if src >= n {
                break;
            }
            out[out_pos + i] += samples[src] * window[i];
            weights[out_pos + i] += window[i];
        }

        out_pos += hop;
        previous = position;
    }

    // Divide out the window that was applied, so the level of the result
    // is the level of the input. Where the signal has run out the sum is
    // zero and stays zero: silence, not a burst of noise.
    for i in 0..out_len {
        if weights[i] > 1e-6 {
            out[i] /= weights[i];
        }
    }

    out.truncate(out_len);
    out
}

/// Stretch `samples` so that they last `seconds`.
///
/// This is the form the studio needs: the learner wants their take to
/// last exactly as long as the reference recording.
pub fn stretch_to_duration(samples: &[f32], sample_rate: f32, seconds: f64) -> Vec<f32> {
    if samples.is_empty() || sample_rate <= 0.0 || seconds <= 0.0 {
        return samples.to_vec();
    }
    let current = samples.len() as f64 / sample_rate as f64;
    if current <= 0.0 {
        return samples.to_vec();
    }
    stretch(samples, sample_rate, (seconds / current) as f32)
}
