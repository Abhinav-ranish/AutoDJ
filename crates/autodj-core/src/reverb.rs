//! Simple algorithmic reverb for the audio callback.
//!
//! Uses a Schroeder-style reverb with 4 comb filters + 2 allpass filters.
//! Zero allocation after construction, safe for real-time audio.

const NUM_COMBS: usize = 4;
const NUM_ALLPASSES: usize = 2;

/// Comb filter delay line lengths (in samples at 48 kHz).
/// These are carefully chosen prime-ish numbers to avoid metallic resonance.
const COMB_LENGTHS: [usize; NUM_COMBS] = [1557, 1617, 1491, 1422];

/// Allpass filter delay line lengths.
const ALLPASS_LENGTHS: [usize; NUM_ALLPASSES] = [225, 556];

struct CombFilter {
    buffer: Vec<f32>,
    write_head: usize,
    size: usize,
    feedback: f32,
    damp: f32,
    damp_prev: f32,
}

impl CombFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            write_head: 0,
            size,
            feedback: 0.7,
            damp: 0.3,
            damp_prev: 0.0,
        }
    }

    #[inline(always)]
    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.write_head];
        // Low-pass comb filter (damping)
        let filtered = output * (1.0 - self.damp) + self.damp_prev * self.damp;
        self.damp_prev = filtered;
        self.buffer[self.write_head] = input + filtered * self.feedback;
        self.write_head = (self.write_head + 1) % self.size;
        output
    }

    fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.damp_prev = 0.0;
    }
}

struct AllpassFilter {
    buffer: Vec<f32>,
    write_head: usize,
    size: usize,
}

impl AllpassFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            write_head: 0,
            size,
        }
    }

    #[inline(always)]
    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.write_head];
        let output = -input + buffered;
        self.buffer[self.write_head] = input + buffered * 0.5;
        self.write_head = (self.write_head + 1) % self.size;
        output
    }

    fn clear(&mut self) {
        self.buffer.fill(0.0);
    }
}

/// Stereo reverb processor.
pub struct StereoReverb {
    combs_l: [CombFilter; NUM_COMBS],
    combs_r: [CombFilter; NUM_COMBS],
    allpasses_l: [AllpassFilter; NUM_ALLPASSES],
    allpasses_r: [AllpassFilter; NUM_ALLPASSES],

    /// Dry/wet mix [0.0, 1.0].
    pub wet: f32,
    /// Room size (controls feedback) [0.0, 1.0].
    pub room_size: f32,
    /// Damping (high frequency absorption) [0.0, 1.0].
    pub damping: f32,
    /// Whether the reverb is active.
    pub active: bool,

    _sample_rate_ratio: f32,
}

impl StereoReverb {
    /// Create a new reverb, pre-allocating all buffers.
    pub fn new(sample_rate: f32) -> Self {
        let ratio = sample_rate / 48000.0;

        let scale = |len: usize| -> usize {
            ((len as f32) * ratio) as usize
        };

        // Slight stereo spread: right channel comb filters are 23 samples longer
        let spread = (23.0 * ratio) as usize;

        Self {
            combs_l: [
                CombFilter::new(scale(COMB_LENGTHS[0])),
                CombFilter::new(scale(COMB_LENGTHS[1])),
                CombFilter::new(scale(COMB_LENGTHS[2])),
                CombFilter::new(scale(COMB_LENGTHS[3])),
            ],
            combs_r: [
                CombFilter::new(scale(COMB_LENGTHS[0]) + spread),
                CombFilter::new(scale(COMB_LENGTHS[1]) + spread),
                CombFilter::new(scale(COMB_LENGTHS[2]) + spread),
                CombFilter::new(scale(COMB_LENGTHS[3]) + spread),
            ],
            allpasses_l: [
                AllpassFilter::new(scale(ALLPASS_LENGTHS[0])),
                AllpassFilter::new(scale(ALLPASS_LENGTHS[1])),
            ],
            allpasses_r: [
                AllpassFilter::new(scale(ALLPASS_LENGTHS[0]) + spread),
                AllpassFilter::new(scale(ALLPASS_LENGTHS[1]) + spread),
            ],
            wet: 0.0,
            room_size: 0.7,
            damping: 0.5,
            active: false,
            _sample_rate_ratio: ratio,
        }
    }

    /// Set reverb parameters and activate.
    pub fn set_params(&mut self, room_size: f32, damping: f32, wet: f32) {
        self.room_size = room_size.clamp(0.0, 1.0);
        self.damping = damping.clamp(0.0, 1.0);
        self.wet = wet.clamp(0.0, 1.0);
        self.active = true;

        // Update comb filter parameters
        let feedback = 0.5 + self.room_size * 0.45; // 0.5..0.95
        let damp = self.damping;
        for comb in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            comb.feedback = feedback;
            comb.damp = damp;
        }
    }

    /// Clear all buffers and deactivate.
    pub fn clear(&mut self) {
        for comb in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            comb.clear();
        }
        for ap in self.allpasses_l.iter_mut().chain(self.allpasses_r.iter_mut()) {
            ap.clear();
        }
        self.active = false;
        self.wet = 0.0;
    }

    /// Process one stereo frame.
    #[inline(always)]
    pub fn process(&mut self, in_l: f32, in_r: f32) -> (f32, f32) {
        if !self.active || self.wet <= 0.0 {
            return (in_l, in_r);
        }

        // Sum all comb filter outputs (parallel combs)
        let input_scaled = (in_l + in_r) * 0.5 * 0.015; // scale down to prevent clipping
        let mut sum_l = 0.0f32;
        let mut sum_r = 0.0f32;

        for comb in &mut self.combs_l {
            sum_l += comb.process(input_scaled);
        }
        for comb in &mut self.combs_r {
            sum_r += comb.process(input_scaled);
        }

        // Series allpass filters (adds density/diffusion)
        for ap in &mut self.allpasses_l {
            sum_l = ap.process(sum_l);
        }
        for ap in &mut self.allpasses_r {
            sum_r = ap.process(sum_r);
        }

        // Mix dry + wet
        let dry = 1.0 - self.wet;
        let out_l = in_l * dry + sum_l * self.wet;
        let out_r = in_r * dry + sum_r * self.wet;

        (out_l, out_r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bypass_when_inactive() {
        let mut rev = StereoReverb::new(48000.0);
        let (l, r) = rev.process(1.0, 1.0);
        assert!((l - 1.0).abs() < 0.001);
        assert!((r - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_reverb_produces_tail() {
        let mut rev = StereoReverb::new(48000.0);
        rev.set_params(0.8, 0.3, 1.0);

        // Feed an impulse
        rev.process(1.0, 1.0);
        // After many samples, there should still be reverb tail
        let mut has_output = false;
        for _ in 0..24000 {
            let (l, r) = rev.process(0.0, 0.0);
            if l.abs() > 0.001 || r.abs() > 0.001 {
                has_output = true;
            }
        }
        assert!(has_output, "Expected reverb tail after impulse");
    }

    #[test]
    fn test_clear_kills_tail() {
        let mut rev = StereoReverb::new(48000.0);
        rev.set_params(0.8, 0.3, 1.0);
        rev.process(1.0, 1.0);
        for _ in 0..1000 {
            rev.process(0.0, 0.0);
        }
        rev.clear();
        for _ in 0..48000 {
            let (l, r) = rev.process(0.0, 0.0);
            assert!(l.abs() < 0.001 && r.abs() < 0.001);
        }
    }
}
