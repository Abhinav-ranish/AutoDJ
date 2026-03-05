//! State Variable Filter (SVF) for HP/LP sweep.
//!
//! A DJ-style sweepable filter controlled by a single knob:
//! - Fully clockwise (20 kHz) = bypass
//! - Turn counter-clockwise = lowpass, cutting highs
//! - The filter is always in lowpass mode by default
//!
//! The UI can switch between LP and HP by setting the mode.
//!
//! Implementation: SVF topology for numerical stability at all frequencies.
//! All methods are safe for the audio callback: no allocation, no I/O.

/// Filter mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMode {
    Lowpass,
    Highpass,
}

/// State Variable Filter (2nd order, stereo).
pub struct SvfFilter {
    /// Current cutoff frequency in Hz.
    pub cutoff_hz: f32,
    /// Resonance (Q factor), range 0.5–4.0.
    pub resonance: f32,
    /// Filter mode.
    pub mode: FilterMode,
    /// Sample rate.
    sample_rate: f32,

    // SVF state variables (per channel)
    ic1eq_l: f32,
    ic2eq_l: f32,
    ic1eq_r: f32,
    ic2eq_r: f32,

    // Cached coefficients
    g: f32,
    k: f32,
    a1: f32,
    a2: f32,
    a3: f32,
}

impl SvfFilter {
    /// Create a new SVF filter in bypass mode (cutoff at 20 kHz).
    pub fn new(sample_rate: f32) -> Self {
        let mut f = Self {
            cutoff_hz: 20000.0,
            resonance: 0.7,
            mode: FilterMode::Lowpass,
            sample_rate,
            ic1eq_l: 0.0,
            ic2eq_l: 0.0,
            ic1eq_r: 0.0,
            ic2eq_r: 0.0,
            g: 0.0,
            k: 0.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
        };
        f.update_coefficients();
        f
    }

    /// Set filter parameters. Automatically selects LP or HP mode:
    /// - cutoff > 1000 Hz → Lowpass (or bypass at 20 kHz)
    /// - cutoff <= 1000 Hz → Highpass
    pub fn set_params(&mut self, cutoff_hz: f32, resonance: f32) {
        self.cutoff_hz = cutoff_hz.clamp(20.0, 20000.0);
        self.resonance = resonance.clamp(0.5, 4.0);

        // Auto-select mode: low cutoff = HP, high cutoff = LP
        // The "center" of the DJ filter knob is around 1000 Hz
        if self.cutoff_hz <= 1000.0 {
            self.mode = FilterMode::Highpass;
        } else {
            self.mode = FilterMode::Lowpass;
        }

        self.update_coefficients();
    }

    fn update_coefficients(&mut self) {
        self.g = (std::f32::consts::PI * self.cutoff_hz / self.sample_rate).tan();
        self.k = 1.0 / self.resonance;
        self.a1 = 1.0 / (1.0 + self.g * (self.g + self.k));
        self.a2 = self.g * self.a1;
        self.a3 = self.g * self.a2;
    }

    /// Returns true if the filter is effectively in bypass mode.
    #[inline(always)]
    pub fn is_bypass(&self) -> bool {
        self.cutoff_hz >= 19500.0 && self.mode == FilterMode::Lowpass
    }

    /// Process one stereo frame through the SVF filter.
    #[inline(always)]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        if self.is_bypass() {
            return (l, r);
        }

        // Process left channel
        let v3_l = l - self.ic2eq_l;
        let v1_l = self.a1 * self.ic1eq_l + self.a2 * v3_l;
        let v2_l = self.ic2eq_l + self.a2 * self.ic1eq_l + self.a3 * v3_l;
        self.ic1eq_l = 2.0 * v1_l - self.ic1eq_l;
        self.ic2eq_l = 2.0 * v2_l - self.ic2eq_l;

        // Process right channel
        let v3_r = r - self.ic2eq_r;
        let v1_r = self.a1 * self.ic1eq_r + self.a2 * v3_r;
        let v2_r = self.ic2eq_r + self.a2 * self.ic1eq_r + self.a3 * v3_r;
        self.ic1eq_r = 2.0 * v1_r - self.ic1eq_r;
        self.ic2eq_r = 2.0 * v2_r - self.ic2eq_r;

        match self.mode {
            FilterMode::Lowpass => (v2_l, v2_r),
            FilterMode::Highpass => {
                let hp_l = l - self.k * v1_l - v2_l;
                let hp_r = r - self.k * v1_r - v2_r;
                (hp_l, hp_r)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_bypass() {
        let mut filter = SvfFilter::new(48000.0);
        assert!(filter.is_bypass());

        let (l, r) = filter.process(0.5, -0.3);
        assert!((l - 0.5).abs() < f32::EPSILON);
        assert!((r - (-0.3)).abs() < f32::EPSILON);
    }

    #[test]
    fn test_filter_lowpass() {
        let mut filter = SvfFilter::new(48000.0);
        filter.set_params(5000.0, 0.7); // LP mode at 5 kHz
        assert!(!filter.is_bypass());
        assert_eq!(filter.mode, FilterMode::Lowpass);

        // DC signal should pass through LP filter
        let mut last = (0.0, 0.0);
        for _ in 0..2000 {
            last = filter.process(1.0, 1.0);
        }
        assert!((last.0 - 1.0).abs() < 0.05, "LP didn't pass DC: {}", last.0);
    }

    #[test]
    fn test_filter_highpass() {
        let mut filter = SvfFilter::new(48000.0);
        filter.set_params(100.0, 0.7); // HP mode at 100 Hz
        assert!(!filter.is_bypass());
        assert_eq!(filter.mode, FilterMode::Highpass);

        // DC signal should be blocked by HP
        let mut last = (0.0, 0.0);
        for _ in 0..2000 {
            last = filter.process(1.0, 1.0);
        }
        assert!(last.0.abs() < 0.1, "HP didn't block DC: {}", last.0);
    }

    #[test]
    fn test_filter_set_params_clamping() {
        let mut filter = SvfFilter::new(48000.0);
        filter.set_params(5.0, 10.0);
        assert!((filter.cutoff_hz - 20.0).abs() < f32::EPSILON);
        assert!((filter.resonance - 4.0).abs() < f32::EPSILON);

        filter.set_params(50000.0, 0.1);
        assert!((filter.cutoff_hz - 20000.0).abs() < f32::EPSILON);
        assert!((filter.resonance - 0.5).abs() < f32::EPSILON);
    }
}
