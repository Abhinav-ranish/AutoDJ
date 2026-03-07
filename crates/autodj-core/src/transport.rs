//! Transport clock: master beat/bar counter for quantised scheduling.

use crate::command::SchedulePoint;

/// The master transport clock tracks beats and bars.
///
/// Used by the audio callback to determine when to execute
/// scheduled commands.
pub struct TransportClock {
    /// Sample rate (Hz).
    pub sample_rate: f64,
    /// Current sample position (monotonically increasing).
    pub sample_pos: u64,
    /// Master BPM.
    pub bpm: f64,
    /// Samples per beat (derived from BPM and sample rate).
    pub samples_per_beat: f64,
    /// Current beat number (since transport start).
    pub beat_number: u64,
    /// Current bar number (assuming 4/4).
    pub bar_number: u64,
    /// Beats per bar (default 4).
    pub beats_per_bar: u32,
    /// Phase within current beat (0.0 – 1.0).
    pub beat_phase: f64,
}

impl TransportClock {
    /// Create a new transport clock.
    pub fn new(sample_rate: f64, bpm: f64) -> Self {
        let samples_per_beat = sample_rate * 60.0 / bpm;
        Self {
            sample_rate,
            sample_pos: 0,
            bpm,
            samples_per_beat,
            beat_number: 0,
            bar_number: 0,
            beats_per_bar: 4,
            beat_phase: 0.0,
        }
    }

    /// Set the BPM and recalculate samples per beat.
    pub fn set_bpm(&mut self, bpm: f64) {
        self.bpm = bpm.clamp(60.0, 200.0);
        self.samples_per_beat = self.sample_rate * 60.0 / self.bpm;
    }

    /// Advance the clock by one sample. Returns true if a new beat started.
    #[inline(always)]
    pub fn advance(&mut self) -> bool {
        self.sample_pos += 1;
        let prev_phase = self.beat_phase;
        self.beat_phase = (self.sample_pos as f64 % self.samples_per_beat) / self.samples_per_beat;

        // Detect beat boundary (phase wrapped around).
        if self.beat_phase < prev_phase {
            self.beat_number += 1;
            if self.beat_number.is_multiple_of(self.beats_per_bar as u64) {
                self.bar_number += 1;
            }
            true
        } else {
            false
        }
    }

    /// Get the sample position of the next beat boundary.
    pub fn next_beat_sample(&self) -> u64 {
        let current_beat_start =
            (self.sample_pos as f64 / self.samples_per_beat).floor() * self.samples_per_beat;
        (current_beat_start + self.samples_per_beat) as u64
    }

    /// Get the sample position of the next bar boundary.
    pub fn next_bar_sample(&self) -> u64 {
        let beats_until_bar =
            self.beats_per_bar as u64 - (self.beat_number % self.beats_per_bar as u64);
        let samples_until = beats_until_bar as f64 * self.samples_per_beat;
        self.sample_pos + samples_until as u64
    }

    /// Resolve a SchedulePoint to an absolute target sample position.
    pub fn resolve_schedule(&self, schedule: &SchedulePoint) -> u64 {
        match schedule {
            SchedulePoint::Immediate => self.sample_pos,
            SchedulePoint::NextBeat => self.next_beat_sample(),
            SchedulePoint::NextBar => self.next_bar_sample(),
            SchedulePoint::AtBeat(beat) => {
                (*beat as f64 * self.samples_per_beat) as u64
            }
            SchedulePoint::AfterBeats(n) => {
                self.sample_pos + (*n as f64 * self.samples_per_beat) as u64
            }
            SchedulePoint::AfterBars(n) => {
                self.sample_pos
                    + (*n as f64 * self.beats_per_bar as f64 * self.samples_per_beat) as u64
            }
        }
    }

    /// Convert a beat number to a frame position (for seeking).
    pub fn beat_to_frame(&self, beat: u64) -> usize {
        (beat as f64 * self.samples_per_beat) as usize
    }

    /// Convert milliseconds to frames.
    pub fn ms_to_frames(&self, ms: u64) -> usize {
        (ms as f64 * self.sample_rate / 1000.0) as usize
    }

    /// Convert frames to milliseconds.
    pub fn frames_to_ms(&self, frames: usize) -> u64 {
        (frames as f64 * 1000.0 / self.sample_rate) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_samples_per_beat() {
        let clock = TransportClock::new(48000.0, 120.0);
        // At 120 BPM, 1 beat = 0.5s = 24000 samples.
        assert!((clock.samples_per_beat - 24000.0).abs() < 1.0);
    }

    #[test]
    fn test_beat_detection() {
        let mut clock = TransportClock::new(48000.0, 120.0);
        let mut beat_count = 0u64;

        // Advance through 2 seconds = 4 beats at 120 BPM.
        for _ in 0..(48000 * 2) {
            if clock.advance() {
                beat_count += 1;
            }
        }
        // Should detect ~4 beats (±1 due to boundary).
        assert!(beat_count >= 3 && beat_count <= 5, "Got {} beats", beat_count);
    }

    #[test]
    fn test_bar_detection() {
        let mut clock = TransportClock::new(48000.0, 120.0);

        // Advance through 8 seconds = 16 beats = 4 bars at 120 BPM.
        for _ in 0..(48000 * 8) {
            clock.advance();
        }
        assert!(clock.bar_number >= 3 && clock.bar_number <= 5);
    }

    #[test]
    fn test_resolve_schedule_next_beat() {
        let clock = TransportClock::new(48000.0, 120.0);
        let target = clock.resolve_schedule(&SchedulePoint::NextBeat);
        // Next beat should be samples_per_beat from start
        assert!((target as f64 - 24000.0).abs() < 2.0);
    }

    #[test]
    fn test_resolve_schedule_next_bar() {
        let clock = TransportClock::new(48000.0, 120.0);
        let target = clock.resolve_schedule(&SchedulePoint::NextBar);
        // Next bar = 4 beats from start = 96000 samples
        assert!((target as f64 - 96000.0).abs() < 10.0);
    }

    #[test]
    fn test_resolve_schedule_after_beats() {
        let mut clock = TransportClock::new(48000.0, 120.0);
        // Advance to a known position
        for _ in 0..48000 {
            clock.advance();
        }
        let target = clock.resolve_schedule(&SchedulePoint::AfterBeats(2));
        // Should be 2 beats (48000 samples) after current pos
        let expected = clock.sample_pos + 48000;
        assert!((target as i64 - expected as i64).unsigned_abs() < 2);
    }
}
