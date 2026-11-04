//! Pitch tracking (McLeod / YIN) and frame energy.
//!
//! The tracker runs entirely in Wasm over `f32` PCM, so the same code serves
//! both the live microphone view and the post-hoc analysis of the reference
//! recording.

/// One analysis frame.
#[derive(Clone, Copy, Debug)]
pub struct F0Frame {
    /// Time of the frame centre, in seconds.
    pub time: f64,
    /// Fundamental frequency in Hz (`0.0` when unvoiced).
    pub f0: f32,
    /// Root-mean-square amplitude of the frame.
    pub rms: f32,
    /// Whether the frame was judged voiced.
    pub voiced: bool,
}

/// Analysis window size in samples (≈43 ms at 48 kHz).
pub const WIN: usize = 2048;
/// Hop size in samples between frames.
pub const HOP: usize = 512;
/// Lowest fundamental frequency we look for (75 Hz ≈ a low male voice).
pub const F_MIN: f32 = 75.0;
/// Highest fundamental frequency we look for (500 Hz).
pub const F_MAX: f32 = 500.0;
/// YIN absolute threshold — lower is stricter.
pub const YIN_THRESHOLD: f32 = 0.15;

/// Analyse a buffer and return one frame per hop.
pub fn analyse(samples: &[f32], sample_rate: f32) -> Vec<F0Frame> {
    if samples.len() < WIN || sample_rate <= 0.0 {
        return Vec::new();
    }

    let mut frames = Vec::with_capacity(samples.len() / HOP + 1);
    let mut start = 0usize;

    while start + WIN <= samples.len() {
        let window = &samples[start..start + WIN];
        let rms = rms(window);
        let f0 = if rms > 0.0035 {
            yin(window, sample_rate, YIN_THRESHOLD).unwrap_or(0.0)
        } else {
            0.0
        };
        frames.push(F0Frame {
            time: (start as f64 + WIN as f64 * 0.5) / sample_rate as f64,
            f0,
            rms,
            voiced: f0 > 0.0,
        });
        start += HOP;
    }
    frames
}

/// Root-mean-square amplitude of a slice.
pub fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

/// YIN fundamental-frequency estimate with parabolic interpolation.
pub fn yin(window: &[f32], sample_rate: f32, threshold: f32) -> Option<f32> {
    let n = window.len();
    let tau_min = (sample_rate / F_MAX) as usize;
    let tau_max = (sample_rate / F_MIN) as usize;

    if tau_min < 2 || tau_max >= n {
        return None;
    }
    let usable = n - tau_max;
    if usable < 256 {
        return None;
    }

    // 1. Difference function d(tau).
    let mut d = vec![0.0f32; tau_max + 1];
    for (tau, slot) in d.iter_mut().enumerate().take(tau_max + 1) {
        if tau == 0 {
            *slot = 0.0;
            continue;
        }
        let mut sum = 0.0f32;
        for j in 0..usable {
            let diff = window[j] - window[j + tau];
            sum += diff * diff;
        }
        *slot = sum;
    }

    // 2. Cumulative mean normalised difference d'(tau).
    let mut cmnd = vec![0.0f32; tau_max + 1];
    cmnd[0] = 1.0;
    let mut running = 0.0f32;
    for tau in 1..=tau_max {
        running += d[tau];
        cmnd[tau] = if running > 0.0 {
            d[tau] * (tau as f32) / running
        } else {
            1.0
        };
    }

    // 3. First local minimum below the threshold.
    let mut tau = tau_min;
    while tau <= tau_max {
        if cmnd[tau] < threshold {
            while tau + 1 <= tau_max && cmnd[tau + 1] < cmnd[tau] {
                tau += 1;
            }
            break;
        }
        tau += 1;
    }
    if tau > tau_max {
        // Fall back to the global minimum of the normalised difference.
        let mut best = tau_min;
        for t in (tau_min + 1)..=tau_max {
            if cmnd[t] < cmnd[best] {
                best = t;
            }
        }
        if cmnd[best] > 0.6 {
            return None;
        }
        tau = best;
    }

    // 4. Parabolic interpolation around the chosen lag.
    let refined = if tau > tau_min && tau < tau_max {
        let a = cmnd[tau - 1];
        let b = cmnd[tau];
        let c = cmnd[tau + 1];
        let denom = 2.0 * (2.0 * b - a - c);
        if denom.abs() > 1e-9 {
            tau as f64 + (a - c) as f64 / denom as f64
        } else {
            tau as f64
        }
    } else {
        tau as f64
    };

    if refined <= 0.0 {
        return None;
    }
    let f0 = sample_rate as f64 / refined;
    if f0 < F_MIN as f64 || f0 > F_MAX as f64 {
        return None;
    }
    Some(f0 as f32)
}

/// A coarse RMS envelope, one value per `hop_ms` milliseconds.
pub fn rms_envelope(samples: &[f32], sample_rate: f32, hop_ms: f64) -> Vec<f64> {
    if samples.is_empty() || sample_rate <= 0.0 {
        return Vec::new();
    }
    let hop = ((sample_rate as f64) * hop_ms / 1000.0).max(1.0) as usize;
    let mut out = Vec::with_capacity(samples.len() / hop + 1);
    let mut i = 0;
    while i < samples.len() {
        let end = (i + hop).min(samples.len());
        out.push(rms(&samples[i..end]) as f64);
        i += hop;
    }
    out
}
