//! 3-band equalizer using 2nd-order biquad filters.
//!
//! Crossover frequencies:
//! - Low/Mid boundary: 250 Hz
//! - Mid/High boundary: 2500 Hz
//!
//! Each band has independent gain control (-∞ to +6 dB).
//! Implementation uses Linkwitz-Riley (LR2) crossover topology for
//! phase-coherent band splitting.
//!
//! All methods are safe for the audio callback: no allocation, no I/O.

/// Biquad filter coefficients and state (Direct Form II Transposed).
#[derive(Clone)]
struct Biquad {
    // Coefficients
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    // State (per-channel)
    z1_l: f32,
    z2_l: f32,
    z1_r: f32,
    z2_r: f32,
}

impl Biquad {
    fn new() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1_l: 0.0,
            z2_l: 0.0,
            z1_r: 0.0,
            z2_r: 0.0,
        }
    }

    /// Configure as a 2nd-order lowpass (for crossover).
    fn set_lowpass(&mut self, cutoff_hz: f32, sample_rate: f32) {
        let w0 = 2.0 * std::f32::consts::PI * cutoff_hz / sample_rate;
        let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
        // Q = 0.707 for Butterworth (Linkwitz-Riley second order)
        let alpha = sin_w0 / (2.0 * std::f32::consts::FRAC_1_SQRT_2);

        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 - cos_w0) / 2.0) / a0;
        self.b1 = (1.0 - cos_w0) / a0;
        self.b2 = ((1.0 - cos_w0) / 2.0) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Configure as a 2nd-order highpass (for crossover).
    fn set_highpass(&mut self, cutoff_hz: f32, sample_rate: f32) {
        let w0 = 2.0 * std::f32::consts::PI * cutoff_hz / sample_rate;
        let (sin_w0, cos_w0) = (w0.sin(), w0.cos());
        let alpha = sin_w0 / (2.0 * std::f32::consts::FRAC_1_SQRT_2);

        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 + cos_w0) / 2.0) / a0;
        self.b1 = (-(1.0 + cos_w0)) / a0;
        self.b2 = ((1.0 + cos_w0) / 2.0) / a0;
        self.a1 = (-2.0 * cos_w0) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    /// Process one sample (left channel).
    #[inline(always)]
    fn process_l(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1_l;
        self.z1_l = self.b1 * input - self.a1 * output + self.z2_l;
        self.z2_l = self.b2 * input - self.a2 * output;
        output
    }

    /// Process one sample (right channel).
    #[inline(always)]
    fn process_r(&mut self, input: f32) -> f32 {
        let output = self.b0 * input + self.z1_r;
        self.z1_r = self.b1 * input - self.a1 * output + self.z2_r;
        self.z2_r = self.b2 * input - self.a2 * output;
        output
    }
}

/// 3-band EQ with adjustable per-band gain and optional isolator/kill mode.
pub struct ThreeBandEq {
    /// LP filter for extracting the low band (< 250 Hz).
    lp_low: Biquad,
    /// HP filter for removing the low band from the signal.
    hp_low: Biquad,
    /// LP filter for extracting the mid band (< 2500 Hz from the HP output).
    lp_high: Biquad,
    /// HP filter for extracting the high band (> 2500 Hz).
    hp_high: Biquad,

    // Steeper isolator filters (second cascade stage for LR4 in isolator mode)
    lp_low_2: Biquad,
    hp_low_2: Biquad,
    lp_high_2: Biquad,
    hp_high_2: Biquad,

    /// Per-band gain in linear amplitude.
    pub gain_low: f32,
    pub gain_mid: f32,
    pub gain_high: f32,

    /// Per-band kill switches.
    pub kill_low: bool,
    pub kill_mid: bool,
    pub kill_high: bool,

    /// Whether to use isolator (steep LR4) or classic (LR2) mode.
    pub isolator_mode: bool,
}

impl ThreeBandEq {
    /// Create a new 3-band EQ for the given sample rate.
    pub fn new(sample_rate: f32) -> Self {
        let mut eq = Self {
            lp_low: Biquad::new(),
            hp_low: Biquad::new(),
            lp_high: Biquad::new(),
            hp_high: Biquad::new(),
            lp_low_2: Biquad::new(),
            hp_low_2: Biquad::new(),
            lp_high_2: Biquad::new(),
            hp_high_2: Biquad::new(),
            gain_low: 1.0,
            gain_mid: 1.0,
            gain_high: 1.0,
            kill_low: false,
            kill_mid: false,
            kill_high: false,
            isolator_mode: false,
        };
        eq.lp_low.set_lowpass(250.0, sample_rate);
        eq.hp_low.set_highpass(250.0, sample_rate);
        eq.lp_high.set_lowpass(2500.0, sample_rate);
        eq.hp_high.set_highpass(2500.0, sample_rate);
        // Cascade stage for isolator (LR4)
        eq.lp_low_2.set_lowpass(250.0, sample_rate);
        eq.hp_low_2.set_highpass(250.0, sample_rate);
        eq.lp_high_2.set_lowpass(2500.0, sample_rate);
        eq.hp_high_2.set_highpass(2500.0, sample_rate);
        eq
    }

    /// Set gain for a band from dB value.
    pub fn set_gain_db(&mut self, band: usize, db: f32) {
        let linear = if db <= -96.0 {
            0.0
        } else {
            10.0_f32.powf(db / 20.0)
        };
        match band {
            0 => self.gain_low = linear,
            1 => self.gain_mid = linear,
            2 => self.gain_high = linear,
            _ => {}
        }
    }

    /// Set a kill switch for a band.
    pub fn set_kill(&mut self, band: usize, enabled: bool) {
        match band {
            0 => self.kill_low = enabled,
            1 => self.kill_mid = enabled,
            2 => self.kill_high = enabled,
            _ => {}
        }
    }

    /// Switch between classic (LR2) and isolator (LR4) mode.
    pub fn set_isolator_mode(&mut self, enabled: bool) {
        self.isolator_mode = enabled;
    }

    /// Process one stereo frame through the 3-band EQ.
    ///
    /// Signal flow:
    ///   input → LP(250) → low band
    ///   input → HP(250) → LP(2500) → mid band
    ///   input → HP(250) → HP(2500) → high band
    ///
    /// In isolator mode, each filter is cascaded (LR4) for steeper slopes.
    #[inline(always)]
    pub fn process(&mut self, l: f32, r: f32) -> (f32, f32) {
        // Low band
        let mut low_l = self.lp_low.process_l(l);
        let mut low_r = self.lp_low.process_r(r);

        // Remove low frequencies
        let mut no_low_l = self.hp_low.process_l(l);
        let mut no_low_r = self.hp_low.process_r(r);

        if self.isolator_mode {
            low_l = self.lp_low_2.process_l(low_l);
            low_r = self.lp_low_2.process_r(low_r);
            no_low_l = self.hp_low_2.process_l(no_low_l);
            no_low_r = self.hp_low_2.process_r(no_low_r);
        }

        // Mid band (from no-low signal, take below 2500 Hz)
        let mut mid_l = self.lp_high.process_l(no_low_l);
        let mut mid_r = self.lp_high.process_r(no_low_r);

        // High band (from no-low signal, take above 2500 Hz)
        let mut high_l = self.hp_high.process_l(no_low_l);
        let mut high_r = self.hp_high.process_r(no_low_r);

        if self.isolator_mode {
            mid_l = self.lp_high_2.process_l(mid_l);
            mid_r = self.lp_high_2.process_r(mid_r);
            high_l = self.hp_high_2.process_l(high_l);
            high_r = self.hp_high_2.process_r(high_r);
        }

        // Apply gains (kill = gain 0.0)
        let g_low = if self.kill_low { 0.0 } else { self.gain_low };
        let g_mid = if self.kill_mid { 0.0 } else { self.gain_mid };
        let g_high = if self.kill_high { 0.0 } else { self.gain_high };

        let out_l = low_l * g_low + mid_l * g_mid + high_l * g_high;
        let out_r = low_r * g_low + mid_r * g_mid + high_r * g_high;

        (out_l, out_r)
    }

    /// Returns current gains as [low, mid, high] in dB.
    pub fn gains_db(&self) -> [f32; 3] {
        let to_db = |g: f32| -> f32 {
            if g <= 0.0 { -96.0 } else { 20.0 * g.log10() }
        };
        [to_db(self.gain_low), to_db(self.gain_mid), to_db(self.gain_high)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_passthrough() {
        let mut eq = ThreeBandEq::new(48000.0);
        // With all gains at 1.0, a DC signal should pass through approximately unchanged
        // (after filter transient settles).
        let mut last = (0.0, 0.0);
        for _ in 0..1000 {
            last = eq.process(1.0, 1.0);
        }
        // Should be close to 1.0 after settling
        assert!((last.0 - 1.0).abs() < 0.05, "Left: {}", last.0);
        assert!((last.1 - 1.0).abs() < 0.05, "Right: {}", last.1);
    }

    #[test]
    fn test_eq_kill_low() {
        let mut eq = ThreeBandEq::new(48000.0);
        eq.set_gain_db(0, -96.0); // kill low

        // DC signal should be mostly removed (only low band carries DC)
        let mut last = (0.0, 0.0);
        for _ in 0..2000 {
            last = eq.process(1.0, 1.0);
        }
        assert!(last.0.abs() < 0.1, "Low kill didn't attenuate DC: {}", last.0);
    }

    #[test]
    fn test_eq_set_gain() {
        let mut eq = ThreeBandEq::new(48000.0);
        eq.set_gain_db(0, 6.0);
        assert!((eq.gain_low - 10.0_f32.powf(6.0 / 20.0)).abs() < 0.001);

        eq.set_gain_db(1, -96.0);
        assert!(eq.gain_mid < 0.001);
    }
}
