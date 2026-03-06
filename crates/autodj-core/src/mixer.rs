//! Mixer: crossfader, master gain, and brick-wall limiter.

/// Crossfader curve type.
#[derive(Debug, Clone, Copy)]
pub enum CrossfaderCurve {
    Linear,
    EqualPower,
}

/// The mixer combines two decks and applies a limiter.
pub struct Mixer {
    /// Crossfader position: -1.0 (full A) to +1.0 (full B).
    pub crossfader: f32,

    /// Crossfader curve.
    pub curve: CrossfaderCurve,

    /// Master gain in linear amplitude.
    pub master_gain: f32,

    /// Limiter ceiling in linear amplitude.
    limiter_ceiling: f32,

    /// Limiter gain envelope (for look-ahead limiter).
    limiter_envelope: f32,

    /// Limiter release coefficient (per sample).
    limiter_release: f32,
}

impl Mixer {
    /// Create a new mixer with default settings.
    pub fn new(sample_rate: f64) -> Self {
        // Release time: 50 ms.
        let release_samples = (sample_rate * 0.05) as f32;
        let limiter_release = (-1.0_f32 / release_samples).exp();

        // Ceiling at -0.3 dBFS.
        let limiter_ceiling = 10.0_f32.powf(-0.3 / 20.0);

        Self {
            crossfader: 0.0,
            curve: CrossfaderCurve::EqualPower,
            master_gain: 1.0,
            limiter_ceiling,
            limiter_envelope: 1.0,
            limiter_release,
        }
    }

    /// Compute the gain for each deck based on crossfader position.
    #[inline(always)]
    pub fn crossfader_gains(&self) -> (f32, f32) {
        let norm = (self.crossfader + 1.0) / 2.0; // 0.0..1.0

        match self.curve {
            CrossfaderCurve::Linear => (1.0 - norm, norm),
            CrossfaderCurve::EqualPower => {
                let theta = norm * std::f32::consts::FRAC_PI_2;
                (theta.cos(), theta.sin())
            }
        }
    }

    /// Mix two stereo frames from deck A and deck B.
    ///
    /// Applies crossfader gains, master gain, and the limiter.
    /// This is safe for the audio callback.
    #[inline(always)]
    pub fn mix(
        &mut self,
        deck_a: (f32, f32),
        deck_b: (f32, f32),
    ) -> (f32, f32) {
        let (gain_a, gain_b) = self.crossfader_gains();

        let l = deck_a.0 * gain_a + deck_b.0 * gain_b;
        let r = deck_a.1 * gain_a + deck_b.1 * gain_b;

        // Apply master gain.
        let l = l * self.master_gain;
        let r = r * self.master_gain;

        // Brick-wall limiter.
        self.limit(l, r)
    }

    /// Apply a simple brick-wall limiter.
    #[inline(always)]
    fn limit(&mut self, l: f32, r: f32) -> (f32, f32) {
        let peak = l.abs().max(r.abs());

        if peak > self.limiter_ceiling {
            let target_gain = self.limiter_ceiling / peak;
            // Instant attack.
            self.limiter_envelope = self.limiter_envelope.min(target_gain);
        } else {
            // Release.
            self.limiter_envelope =
                self.limiter_envelope + (1.0 - self.limiter_envelope) * (1.0 - self.limiter_release);
        }

        (l * self.limiter_envelope, r * self.limiter_envelope)
    }

    /// Set crossfader position.
    pub fn set_crossfader(&mut self, position: f32) {
        self.crossfader = position.clamp(-1.0, 1.0);
    }

    /// Set master gain from dB.
    pub fn set_master_gain_db(&mut self, db: f32) {
        if db <= -96.0 {
            self.master_gain = 0.0;
        } else {
            self.master_gain = 10.0_f32.powf(db / 20.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossfader_linear() {
        let mut mixer = Mixer::new(48000.0);
        mixer.curve = CrossfaderCurve::Linear;

        // Center
        mixer.set_crossfader(0.0);
        let gains = mixer.crossfader_gains();
        assert_eq!(gains, (0.5, 0.5));

        // Full A
        mixer.set_crossfader(-1.0);
        let gains = mixer.crossfader_gains();
        assert_eq!(gains, (1.0, 0.0));

        // Full B
        mixer.set_crossfader(1.0);
        let gains = mixer.crossfader_gains();
        assert_eq!(gains, (0.0, 1.0));
    }

    #[test]
    fn test_crossfader_equal_power() {
        let mut mixer = Mixer::new(48000.0);
        mixer.curve = CrossfaderCurve::EqualPower;

        // Center: should be sqrt(0.5) ~ 0.707
        mixer.set_crossfader(0.0);
        let gains = mixer.crossfader_gains();
        assert!((gains.0 - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5);
        assert!((gains.1 - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5);

        // Full A
        mixer.set_crossfader(-1.0);
        let gains = mixer.crossfader_gains();
        assert!((gains.0 - 1.0).abs() < 1e-5);
        assert!((gains.1 - 0.0).abs() < 1e-5);

        // Full B
        mixer.set_crossfader(1.0);
        let gains = mixer.crossfader_gains();
        assert!((gains.0 - 0.0).abs() < 1e-5);
        assert!((gains.1 - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_master_gain() {
        let mut mixer = Mixer::new(48000.0);
        mixer.set_master_gain_db(0.0);
        assert_eq!(mixer.master_gain, 1.0);

        mixer.set_master_gain_db(-6.0);
        assert!((mixer.master_gain - 0.501187).abs() < 1e-4);

        mixer.set_master_gain_db(-100.0);
        assert_eq!(mixer.master_gain, 0.0);
    }

    #[test]
    fn test_limiter() {
        let mut mixer = Mixer::new(48000.0);
        mixer.limiter_ceiling = 0.5; // Artificial low ceiling for testing
        mixer.set_master_gain_db(0.0);
        mixer.set_crossfader(-1.0); // Only Deck A
        mixer.curve = CrossfaderCurve::Linear;

        // Mix a loud signal that exceeds ceiling
        let (l, r) = mixer.mix((1.0, 1.0), (0.0, 0.0));
        
        // Output should be strictly <= ceiling
        assert!(l <= 0.5001);
        assert!(r <= 0.5001);
        
        // Envelope should have dropped
        assert!(mixer.limiter_envelope < 1.0);
    }
}
