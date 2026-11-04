//! The measured-contour model.
//!
//! A contour is the result of running the YIN tracker over a buffer. It is
//! stored in **semitones relative to the speaker's own median pitch**, which
//! makes two different voices directly comparable: the shape of the contour
//! carries the information, not the absolute register.

use super::pitch::{analyse, F0Frame};

/// Repair the octave errors a pitch tracker makes now and then.
///
/// YIN occasionally locks onto the second harmonic for a frame or two, or
/// onto the sub-harmonic, which shows up as a wild spike in the contour and
/// wrecks the vertical scale of the chart. Any frame that sits more than
/// about 1.8× away from the median of its neighbours is snapped back to that
/// median.
fn correct_octaves(frames: &[F0Frame]) -> Vec<f32> {
    let raw: Vec<f32> = frames
        .iter()
        .map(|f| if f.voiced { f.f0 } else { 0.0 })
        .collect();
    let n = raw.len();
    let mut out = raw.clone();

    for i in 0..n {
        if raw[i] <= 0.0 {
            continue;
        }
        let mut window: Vec<f32> = Vec::with_capacity(9);
        for j in i.saturating_sub(4)..(i + 5).min(n) {
            if raw[j] > 0.0 {
                window.push(raw[j]);
            }
        }
        if window.len() < 3 {
            continue;
        }
        window.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = window[window.len() / 2];
        if median <= 0.0 {
            continue;
        }
        let octaves = (raw[i] / median).log2();
        if octaves.abs() > 0.85 {
            out[i] = median;
        }
    }

    out
}

/// A measured pitch + energy contour.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Contour {
    /// Frame times in seconds.
    pub times: Vec<f64>,
    /// Pitch in semitones relative to `median_f0`.
    pub semis: Vec<f64>,
    /// Frame RMS amplitude (0..1).
    pub rms: Vec<f64>,
    /// Whether each frame was voiced.
    pub voiced: Vec<bool>,
    /// The speaker's median fundamental frequency in Hz.
    pub median_f0: f64,
    /// Duration of the analysed buffer in seconds.
    pub duration: f64,
}

impl Contour {
    /// Analyse a PCM buffer.
    pub fn from_pcm(samples: &[f32], sample_rate: f32) -> Self {
        let frames: Vec<F0Frame> = analyse(samples, sample_rate);
        Self::from_frames(&frames, samples, sample_rate)
    }

    /// Build a contour from pre-computed frames.
    pub fn from_frames(frames: &[F0Frame], samples: &[f32], sample_rate: f32) -> Self {
        let f0 = correct_octaves(frames);
        if frames.is_empty() {
            return Contour {
                duration: if sample_rate > 0.0 {
                    samples.len() as f64 / sample_rate as f64
                } else {
                    0.0
                },
                ..Default::default()
            };
        }

        let mut voiced_f0: Vec<f64> = f0
            .iter()
            .zip(frames.iter())
            .filter(|(f, frame)| frame.voiced && **f > 0.0)
            .map(|(f, _)| *f as f64)
            .collect();
        voiced_f0.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median_f0 = if voiced_f0.is_empty() {
            0.0
        } else {
            voiced_f0[voiced_f0.len() / 2]
        };

        let mut times = Vec::with_capacity(frames.len());
        let mut semis = Vec::with_capacity(frames.len());
        let mut rms = Vec::with_capacity(frames.len());
        let mut voiced = Vec::with_capacity(frames.len());

        for (i, f) in frames.iter().enumerate() {
            times.push(f.time);
            if f.voiced && f0[i] > 0.0 && median_f0 > 0.0 {
                semis.push(12.0 * (f0[i] as f64 / median_f0).log2());
            } else {
                semis.push(f64::NAN);
            }
            rms.push(f.rms as f64);
            voiced.push(f.voiced);
        }

        Contour {
            times,
            semis,
            rms,
            voiced,
            median_f0,
            duration: if sample_rate > 0.0 {
                samples.len() as f64 / sample_rate as f64
            } else {
                0.0
            },
        }
    }

}
