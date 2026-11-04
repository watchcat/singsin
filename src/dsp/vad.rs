//! Voice activity detection — automatic trimming of dead silence.
//!
//! When a learner presses record, the take almost always starts and ends
//! with silence. Trimming it means the user's `t = 0%` and `t = 100%` land
//! on the same two points of speech as the reference phrase, which is what
//! makes the two pitch curves comparable at all.

use super::pitch::rms_envelope;

/// What the VAD decided.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrimInfo {
    /// Samples removed from the front.
    pub trimmed_front_ms: f64,
    /// Samples removed from the back.
    pub trimmed_back_ms: f64,
    /// Duration before trimming.
    pub original_ms: f64,
    /// Duration after trimming.
    pub kept_ms: f64,
    /// The energy threshold that was used.
    pub threshold: f32,
}

impl TrimInfo {
    /// Whether anything was actually removed.
    pub fn changed(&self) -> bool {
        self.trimmed_front_ms > 0.5 || self.trimmed_back_ms > 0.5
    }
}

/// Trim leading and trailing silence from `samples` in place.
pub fn trim(samples: &mut Vec<f32>, sample_rate: f32) -> TrimInfo {
    let original_ms = if sample_rate > 0.0 {
        samples.len() as f64 * 1000.0 / sample_rate as f64
    } else {
        0.0
    };

    if samples.len() < sample_rate as usize / 50 || sample_rate <= 0.0 {
        return TrimInfo {
            original_ms,
            kept_ms: original_ms,
            threshold: 0.0,
            ..Default::default()
        };
    }

    let env = rms_envelope(samples, sample_rate, 10.0);
    if env.is_empty() {
        return TrimInfo {
            original_ms,
            kept_ms: original_ms,
            threshold: 0.0,
            ..Default::default()
        };
    }

    let peak = env.iter().cloned().fold(0.0f64, f64::max);

    // Noise floor: the 20th percentile of the envelope.
    let mut sorted = env.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let noise_floor = sorted[sorted.len() / 5];

    let threshold = (noise_floor * 3.0).max(peak * 0.05).max(0.004) as f32;

    let mut first: Option<usize> = None;
    let mut last: Option<usize> = None;
    for (i, v) in env.iter().enumerate() {
        if *v >= threshold as f64 {
            if first.is_none() {
                first = Some(i);
            }
            last = Some(i);
        }
    }

    let (first, last) = match (first, last) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            // Nothing crossed the threshold — leave the take untouched.
            return TrimInfo {
                original_ms,
                kept_ms: original_ms,
                threshold,
                ..Default::default()
            };
        }
    };

    let hop = ((sample_rate as f64) * 0.010).max(1.0) as usize;
    let pad = (sample_rate as f64 * 0.020) as usize; // 20 ms of breathing room

    let start = first.saturating_mul(hop).saturating_sub(pad);
    let end = ((last + 1) * hop + pad).min(samples.len());
    let end = end.max(start + 1).min(samples.len());

    let trimmed_front_ms = start as f64 * 1000.0 / sample_rate as f64;
    let trimmed_back_ms = (samples.len() - end) as f64 * 1000.0 / sample_rate as f64;

    if end > start {
        samples.drain(..start);
        samples.truncate(end - start);
    }

    let kept_ms = samples.len() as f64 * 1000.0 / sample_rate as f64;

    TrimInfo {
        trimmed_front_ms,
        trimmed_back_ms,
        original_ms,
        kept_ms,
        threshold,
    }
}
