//! Tempo-synced stereo delay with feedback and ping-pong mode.
//!
//! Designed for the audio callback: zero allocation, no I/O.
//! Delay time is derived from BPM and a subdivision value (in beats).

/// Maximum delay time in seconds (2 seconds at any sample rate).
const MAX_DELAY_SECONDS: f32 = 2.0;

/// Maximum feedback to prevent runaway oscillation.
const MAX_FEEDBACK: f32 = 0.85;

/// Stereo delay with circular buffer.
pub struct StereoDelay {
    buffer_l: Vec<f32>,
    buffer_r: Vec<f32>,
    write_head: usize,
    buffer_size: usize,
    sample_rate: f32,

    /// Delay time in beats (0.25 = 1/16th, 1.0 = quarter note).
    pub subdivision: f32,
    /// Feedback amount [0.0, 0.85].
    pub feedback: f32,
    /// Dry/wet mix [0.0, 1.0].
    pub wet: f32,
    /// Ping-pong mode: alternates taps between L and R.
    pub ping_pong: bool,

    /// Whether the delay is active.
    pub active: bool,

    /// Cached delay in frames (updated when BPM changes).
    cached_delay_frames: usize,
    cached_bpm: f64,
}

impl StereoDelay {
    /// Create a new delay, pre-allocating the circular buffer.
    pub fn new(sample_rate: f32) -> Self {
        let buffer_size = (sample_rate * MAX_DELAY_SECONDS) as usize;
        Self {
            buffer_l: vec![0.0; buffer_size],
            buffer_r: vec![0.0; buffer_size],
            write_head: 0,
            buffer_size,
            sample_rate,
            subdivision: 1.0,
            feedback: 0.0,
            wet: 0.0,
            ping_pong: false,
            active: false,
            cached_delay_frames: 0,
            cached_bpm: 0.0,
        }
    }

    /// Set delay parameters. Activates the delay.
    pub fn set_params(&mut self, subdivision: f32, feedback: f32, wet: f32, ping_pong: bool) {
        self.subdivision = subdivision.clamp(0.0625, 4.0);
        self.feedback = feedback.clamp(0.0, MAX_FEEDBACK);
        self.wet = wet.clamp(0.0, 1.0);
        self.ping_pong = ping_pong;
        self.active = true;
        self.cached_bpm = 0.0; // force recalculation
    }

    /// Clear the delay buffer and deactivate.
    pub fn clear(&mut self) {
        self.buffer_l.fill(0.0);
        self.buffer_r.fill(0.0);
        self.active = false;
        self.wet = 0.0;
        self.feedback = 0.0;
    }

    /// Process one stereo frame. BPM is passed per-frame for tempo tracking.
    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32, bpm: f64) -> (f32, f32) {
        if !self.active || self.wet <= 0.0 {
            // Still advance write head to keep buffer clean
            self.buffer_l[self.write_head] = in_l;
            self.buffer_r[self.write_head] = in_r;
            self.write_head = (self.write_head + 1) % self.buffer_size;
            return (in_l, in_r);
        }

        // Recalculate delay frames when BPM changes
        if (bpm - self.cached_bpm).abs() > 0.01 {
            self.cached_bpm = bpm;
            let beat_duration_s = 60.0 / bpm;
            let delay_s = beat_duration_s * self.subdivision as f64;
            self.cached_delay_frames = (delay_s * self.sample_rate as f64) as usize;
            self.cached_delay_frames = self.cached_delay_frames.min(self.buffer_size - 1).max(1);
        }

        let delay_frames = self.cached_delay_frames;

        // Read from delay buffer
        let read_pos = if self.write_head >= delay_frames {
            self.write_head - delay_frames
        } else {
            self.buffer_size - (delay_frames - self.write_head)
        };

        let delayed_l = self.buffer_l[read_pos];
        let delayed_r = self.buffer_r[read_pos];

        // Apply ping-pong panning
        let (tap_l, tap_r) = if self.ping_pong {
            let pan = if (self.write_head / delay_frames.max(1)) % 2 == 0 {
                (1.0_f32, 0.3_f32) // pan left
            } else {
                (0.3_f32, 1.0_f32) // pan right
            };
            (delayed_l * pan.0, delayed_r * pan.1)
        } else {
            (delayed_l, delayed_r)
        };

        // Mix: dry + wet
        let dry = 1.0 - self.wet;
        let out_l = in_l * dry + tap_l * self.wet;
        let out_r = in_r * dry + tap_r * self.wet;

        // Write input + feedback into buffer
        self.buffer_l[self.write_head] = in_l + tap_l * self.feedback;
        self.buffer_r[self.write_head] = in_r + tap_r * self.feedback;

        self.write_head = (self.write_head + 1) % self.buffer_size;

        (out_l, out_r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bypass_when_wet_zero() {
        let mut delay = StereoDelay::new(48000.0);
        // With wet=0, output should equal input
        let (l, r) = delay.process(1.0, 1.0, 120.0);
        assert!((l - 1.0).abs() < 0.001);
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_delay_produces_output_after_delay_time() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.0, 1.0, false); // 1 beat, no feedback, full wet
        let bpm = 120.0; // 1 beat = 0.5s = 24000 frames

        // Feed an impulse
        delay.process(1.0, 1.0, bpm);
        // Fill with silence until just before delay time
        for _ in 1..24000 {
            delay.process(0.0, 0.0, bpm);
        }
        // At exactly delay_frames, the impulse should appear
        let (l, _r) = delay.process(0.0, 0.0, bpm);
        assert!(l.abs() > 0.5, "Expected delayed impulse, got {l}");
    }

    #[test]
    fn test_feedback_produces_multiple_taps() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.5, 1.0, false); // feedback=0.5
        let bpm = 120.0;

        // Feed impulse
        delay.process(1.0, 1.0, bpm);
        // Silence for 2 full delay periods (48000 frames)
        for _ in 1..48000 {
            delay.process(0.0, 0.0, bpm);
        }
        // Second tap should be at ~0.5 amplitude (feedback=0.5)
        let (l, _) = delay.process(0.0, 0.0, bpm);
        assert!(l.abs() > 0.2, "Expected feedback tap, got {l}");
    }

    #[test]
    fn test_feedback_clamped_to_max() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.99, 1.0, false); // try to exceed max
        assert!(delay.feedback <= 0.85);
    }

    #[test]
    fn test_clear_resets_buffer() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.5, 1.0, false);
        delay.process(1.0, 1.0, 120.0);
        delay.clear();
        // After clear, no delayed output
        for _ in 0..48000 {
            let (l, r) = delay.process(0.0, 0.0, 120.0);
            assert!(l.abs() < 0.001 && r.abs() < 0.001);
        }
    }

    #[test]
    fn test_ping_pong_alternates_channels() {
        let mut delay = StereoDelay::new(48000.0);
        delay.set_params(1.0, 0.5, 1.0, true); // ping_pong=true
        let bpm = 120.0;

        // Feed impulse into both channels
        delay.process(1.0, 1.0, bpm);
        // First tap: should favor one channel
        for _ in 1..24000 {
            delay.process(0.0, 0.0, bpm);
        }
        let (l1, r1) = delay.process(0.0, 0.0, bpm);
        // One channel should be louder than the other
        assert!(
            (l1.abs() - r1.abs()).abs() > 0.1,
            "Expected ping-pong panning, got L={l1} R={r1}"
        );
    }
}
