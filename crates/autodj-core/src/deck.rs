//! Deck: playback unit with transport state, gain, EQ, filter, loop, and track buffer.

use crate::buffer::TrackBuffer;
use crate::command::EqBand;
use crate::eq::ThreeBandEq;
use crate::filter::SvfFilter;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Deck transport state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportState {
    Empty,
    Paused,
    Playing,
}

/// A single DJ deck.
pub struct Deck {
    /// Currently loaded track buffer (shared ownership for thread safety).
    pub track: Option<Arc<TrackBuffer>>,

    /// Playback position in frames (stereo frame index).
    pub position: usize,

    /// Transport state.
    pub state: TransportState,

    /// Deck gain in linear amplitude (1.0 = 0 dB).
    pub gain: f32,

    /// Playback speed multiplier (1.0 = native tempo).
    pub speed: f64,

    /// 3-band EQ.
    pub eq: ThreeBandEq,

    /// HP/LP sweep filter.
    pub filter: SvfFilter,

    /// Loop start position in frames (None = no loop).
    pub loop_start: Option<usize>,

    /// Loop end position in frames.
    pub loop_end: Option<usize>,
}

impl Deck {
    /// Create a new empty deck for the given sample rate.
    pub fn with_sample_rate(sample_rate: f32) -> Self {
        Self {
            track: None,
            position: 0,
            state: TransportState::Empty,
            gain: 1.0,
            speed: 1.0,
            eq: ThreeBandEq::new(sample_rate),
            filter: SvfFilter::new(sample_rate),
            loop_start: None,
            loop_end: None,
        }
    }

    /// Create a new empty deck with default sample rate (48 kHz).
    pub fn new() -> Self {
        Self::with_sample_rate(48000.0)
    }

    /// Load a track buffer into the deck.
    pub fn load_track(&mut self, track: Arc<TrackBuffer>) {
        self.position = 0;
        self.state = TransportState::Paused;
        self.loop_start = None;
        self.loop_end = None;
        self.track = Some(track);
    }

    /// Start playback.
    pub fn play(&mut self) {
        if self.track.is_some() && self.state != TransportState::Empty {
            self.state = TransportState::Playing;
        }
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        if self.state == TransportState::Playing {
            self.state = TransportState::Paused;
        }
    }

    /// Seek to a specific frame position.
    pub fn seek(&mut self, frame: usize) {
        if let Some(track) = &self.track {
            self.position = frame.min(track.total_frames().saturating_sub(1));
        }
    }

    /// Set a loop region (start and end in frames).
    pub fn set_loop(&mut self, start_frame: usize, end_frame: usize) {
        if start_frame < end_frame {
            self.loop_start = Some(start_frame);
            self.loop_end = Some(end_frame);
        }
    }

    /// Clear the active loop.
    pub fn clear_loop(&mut self) {
        self.loop_start = None;
        self.loop_end = None;
    }

    /// Returns true if a loop is currently active.
    pub fn loop_active(&self) -> bool {
        self.loop_start.is_some() && self.loop_end.is_some()
    }

    /// Read the next stereo frame and advance position.
    ///
    /// Signal chain: PCM → EQ → Filter → Gain → Output
    ///
    /// Returns (0.0, 0.0) if no track is loaded or playback is paused.
    /// This method is safe for the audio callback: no allocation, no I/O.
    #[inline(always)]
    pub fn next_frame(&mut self) -> (f32, f32) {
        if self.state != TransportState::Playing {
            return (0.0, 0.0);
        }

        let track = match &self.track {
            Some(t) => t,
            None => return (0.0, 0.0),
        };

        // Loop: if we've reached the loop end, jump back to start
        if let (Some(start), Some(end)) = (self.loop_start, self.loop_end) {
            if self.position >= end {
                self.position = start;
            }
        }

        if self.position >= track.total_frames() {
            self.state = TransportState::Paused;
            return (0.0, 0.0);
        }

        let (l, r) = track.get_frame(self.position);
        self.position += 1;

        // Signal chain: EQ → Filter → Gain
        let (l, r) = self.eq.process(l, r);
        let (l, r) = self.filter.process(l, r);
        (l * self.gain, r * self.gain)
    }

    /// Get current position as milliseconds.
    pub fn position_ms(&self, sample_rate: u32) -> u64 {
        (self.position as u64 * 1000) / sample_rate as u64
    }

    /// Get current position as a fraction of total duration (0.0–1.0).
    pub fn position_fraction(&self) -> f64 {
        match &self.track {
            Some(t) if t.total_frames() > 0 => {
                self.position as f64 / t.total_frames() as f64
            }
            _ => 0.0,
        }
    }

    /// Set gain from dB value.
    pub fn set_gain_db(&mut self, db: f32) {
        if db <= -96.0 {
            self.gain = 0.0;
        } else {
            self.gain = 10.0_f32.powf(db / 20.0);
        }
    }

    /// Set EQ band gain from dB value.
    pub fn set_eq(&mut self, band: EqBand, gain_db: f32) {
        let band_index = match band {
            EqBand::Low => 0,
            EqBand::Mid => 1,
            EqBand::High => 2,
        };
        self.eq.set_gain_db(band_index, gain_db);
    }

    /// Set filter parameters.
    pub fn set_filter(&mut self, cutoff_hz: f32, resonance: f32) {
        self.filter.set_params(cutoff_hz, resonance);
    }
}

impl Default for Deck {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::TrackBuffer;

    /// Create a tiny test TrackBuffer in memory.
    fn make_test_buffer(frames: usize) -> Arc<TrackBuffer> {
        use std::io::Write;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        // Write ADJP header + sample data
        let mut f = std::fs::File::create(&path).unwrap();
        // Magic "ADJP"
        f.write_all(b"ADJP").unwrap();
        // Version: 1u32 LE
        f.write_all(&1u32.to_le_bytes()).unwrap();
        // Sample rate: 48000u32 LE
        f.write_all(&48000u32.to_le_bytes()).unwrap();
        // Channels: 2u16 LE
        f.write_all(&2u16.to_le_bytes()).unwrap();
        // Reserved: 14 bytes
        f.write_all(&[0u8; 14]).unwrap();
        // Sample data: frames * 2 channels * 4 bytes
        for i in 0..frames {
            let val = (i as f32) / (frames as f32);
            f.write_all(&val.to_le_bytes()).unwrap(); // L
            f.write_all(&val.to_le_bytes()).unwrap(); // R
        }
        drop(f);

        let buf = TrackBuffer::open(&path).unwrap();
        // Keep tempfile alive by leaking it (test only)
        std::mem::forget(tmp);
        Arc::new(buf)
    }

    #[test]
    fn test_seek() {
        let mut deck = Deck::new();
        let buf = make_test_buffer(1000);
        deck.load_track(buf);

        deck.seek(500);
        assert_eq!(deck.position, 500);

        // Seek past end should clamp
        deck.seek(99999);
        assert_eq!(deck.position, 999);
    }

    #[test]
    fn test_loop_playback() {
        let mut deck = Deck::new();
        let buf = make_test_buffer(1000);
        deck.load_track(buf);
        deck.play();

        // Set a loop from frame 100 to frame 200
        deck.set_loop(100, 200);
        assert!(deck.loop_active());

        // Advance past loop end
        deck.seek(198);
        deck.next_frame(); // 198 → 199
        deck.next_frame(); // 199 → should be 200, which triggers loop → 100
        // After the loop wrap, position should be around 100-101
        assert!(
            deck.position >= 100 && deck.position <= 102,
            "Expected ~100, got {}",
            deck.position
        );
    }

    #[test]
    fn test_clear_loop() {
        let mut deck = Deck::new();
        deck.set_loop(100, 200);
        assert!(deck.loop_active());
        deck.clear_loop();
        assert!(!deck.loop_active());
    }
}
