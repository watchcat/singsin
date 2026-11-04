//! Contour geometry and the shared tone-shape maths.
//!
//! The mini SVG contours inside the syllable cards and the big canvas chart
//! both use the same Chao grid, so a tone looks identical everywhere.

/// Vertical position of a Chao level (1 = lowest, 5 = highest).
pub fn level_y(level: f64) -> f64 {
    78.0 - (level.clamp(1.0, 5.0) - 1.0) * 16.5
}

/// The (from, to) Chao level of a Cantonese tone.
pub fn tone_levels(tone: u8) -> (f64, f64) {
    match tone {
        1 => (5.0, 5.0),
        2 => (2.0, 5.0),
        3 => (3.0, 3.0),
        4 => (2.0, 1.0),
        5 => (1.0, 3.0),
        6 => (1.0, 1.0),
        _ => (3.0, 3.0),
    }
}

/// Build a smooth SVG path for a tone shape between two Chao levels.
pub fn shape_path(from_level: f64, to_level: f64, x0: f64, x1: f64) -> String {
    let y0 = level_y(from_level);
    let y1 = level_y(to_level);
    let dx = (x1 - x0).max(1.0);
    format!(
        "M {x0:.1} {y0:.1} C {:.1} {y0:.1} {:.1} {y1:.1} {x1:.1} {y1:.1}",
        x0 + dx * 0.36,
        x0 + dx * 0.64
    )
}

/// The five grid lines of the Chao scale, as (level, y).
pub fn grid_lines() -> Vec<(f64, f64)> {
    (1..=5).map(|l| (l as f64, level_y(l as f64))).collect()
}
