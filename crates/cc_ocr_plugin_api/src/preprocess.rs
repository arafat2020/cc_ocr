//! Preprocess knobs supplied by a [`crate::SituationAdapter`].

/// Heuristic chat-pane crop / enhance settings.
#[derive(Debug, Clone)]
pub struct PreprocessConfig {
    pub grayscale: bool,
    pub contrast: f32,
    /// If image width >= this, apply horizontal crop.
    pub min_width_for_crop: u32,
    /// Keep pixels from this fraction of width (left).
    pub crop_left_frac: f32,
    /// Keep pixels up to this fraction of width (right).
    pub crop_right_frac: f32,
}

impl Default for PreprocessConfig {
    fn default() -> Self {
        Self {
            grayscale: true,
            contrast: 1.15,
            min_width_for_crop: u32::MAX, // no crop by default
            crop_left_frac: 0.0,
            crop_right_frac: 1.0,
        }
    }
}

impl PreprocessConfig {
    /// Full-frame enhance only (generic screenshots).
    pub fn no_crop() -> Self {
        Self::default()
    }
}
