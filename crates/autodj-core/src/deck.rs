//! Deck: playback unit with transport state, gain, EQ, filter, loop, and track buffer.

use crate::buffer::TrackBuffer;
use crate::command::{EqBand, EqMode};
use crate::delay::StereoDelay;
use crate::eq::ThreeBandEq;
use crate::filter::SvfFilter;
use crate::reverb::StereoReverb;
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

    /// ID of the currently loaded track.
    pub track_id: Option<String>,

    /// Playback position in frames (stereo frame index).
    pub position: f64,

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

    /// Tempo-synced stereo delay.
    pub delay: StereoDelay,

    /// Algorithmic reverb.
    pub reverb: StereoReverb,

    /// Current BPM (updated from transport clock each frame).
    pub bpm: f64,

    /// Loop start position in frames (None = no loop).
    pub loop_start: Option<usize>,

    /// Loop end position in frames.
    pub loop_end: Option<usize>,

    // -- Phase 30: DJ Performance Engine --
    /// Slip mode: when enabled, the "real" position continues advancing
    /// even during loops/repeats, so clearing them resumes from the right spot.
    pub slip_mode: bool,
    /// Slip position tracks where playback *would* be without loops/repeats.
    pub slip_position: f64,

    /// Beat repeat state: if active, loops a small region.
    pub beat_repeat_start: Option<usize>,
    pub beat_repeat_end: Option<usize>,

    /// Vinyl brake state: speed ramps from current to 0 over this many frames.
    pub brake_remaining: u32,
    pub brake_total: u32,
    pub brake_initial_speed: f64,

    /// Spinback state: speed ramps through 0 to negative then stops.
    pub spinback_remaining: u32,
    pub spinback_total: u32,
    pub spinback_initial_speed: f64,
}

impl Deck {
    /// Create a new empty deck for the given sample rate.
    pub fn with_sample_rate(sample_rate: f32) -> Self {
        Self {
            track: None,
            track_id: None,
            position: 0.0,
            state: TransportState::Empty,
            gain: 1.0,
            speed: 1.0,
            eq: ThreeBandEq::new(sample_rate),
            filter: SvfFilter::new(sample_rate),
            delay: StereoDelay::new(sample_rate),
            reverb: StereoReverb::new(sample_rate),
            bpm: 120.0,
            loop_start: None,
            loop_end: None,
            slip_mode: false,
            slip_position: 0.0,
            beat_repeat_start: None,
            beat_repeat_end: None,
            brake_remaining: 0,
            brake_total: 0,
            brake_initial_speed: 1.0,
            spinback_remaining: 0,
            spinback_total: 0,
            spinback_initial_speed: 1.0,
        }
    }

    /// Create a new empty deck with default sample rate (48 kHz).
    pub fn new() -> Self {
        Self::with_sample_rate(48000.0)
    }

    /// Load a track buffer into the deck.
    pub fn load_track(&mut self, track_id: String, track: Arc<TrackBuffer>) {
        self.position = 0.0;
        self.state = TransportState::Paused;
        self.loop_start = None;
        self.loop_end = None;
        self.beat_repeat_start = None;
        self.beat_repeat_end = None;
        self.brake_remaining = 0;
        self.spinback_remaining = 0;
        self.slip_position = 0.0;
        self.delay.clear();
        self.reverb.clear();
        self.track_id = Some(track_id);
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
            self.position = (frame as f64).min(track.total_frames().saturating_sub(1) as f64);
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
    /// Signal chain: PCM → EQ → Filter → Delay → Gain → Output
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

        // Vinyl brake: ramp speed to 0
        if self.brake_remaining > 0 {
            let progress = 1.0 - (self.brake_remaining as f64 / self.brake_total as f64);
            self.speed = self.brake_initial_speed * (1.0 - progress);
            self.brake_remaining -= 1;
            if self.brake_remaining == 0 {
                self.speed = 0.0;
                self.state = TransportState::Paused;
                self.speed = self.brake_initial_speed; // restore for next play
                return (0.0, 0.0);
            }
        }

        // Spinback: ramp speed through 0 to negative, then stop
        if self.spinback_remaining > 0 {
            let progress = 1.0 - (self.spinback_remaining as f64 / self.spinback_total as f64);
            // Goes from +1 to -1 over the duration  
            self.speed = self.spinback_initial_speed * (1.0 - 2.0 * progress);
            self.spinback_remaining -= 1;
            if self.spinback_remaining == 0 {
                self.speed = self.spinback_initial_speed; // restore
                self.state = TransportState::Paused;
                return (0.0, 0.0);
            }
        }
 
        // Beat repeat: short loop region (takes priority over normal loop)
        if let (Some(start), Some(end)) = (self.beat_repeat_start, self.beat_repeat_end) {
            if self.position >= end as f64 || self.position < start as f64 {
                self.position = start as f64;
            }
        } else if let (Some(start), Some(end)) = (self.loop_start, self.loop_end) {
            // Normal loop
            if self.position >= end as f64 {
                self.position = start as f64;
            }
        }
 
        let total_frames = track.total_frames();
        if self.position < 0.0 {
            self.position = 0.0;
        }
        if self.position >= total_frames as f64 {
            self.state = TransportState::Paused;
            return (0.0, 0.0);
        }
 
        // Linear Interpolation
        let idx_low = self.position.floor().max(0.0) as usize;
        let idx_high = (idx_low + 1).min(total_frames - 1);
        let frac = (self.position - idx_low as f64) as f32;
 
        let (l_low, r_low) = track.get_frame(idx_low);
        let (l_high, r_high) = track.get_frame(idx_high);
 
        let l = l_low + frac * (l_high - l_low);
        let r = r_low + frac * (r_high - r_low);
 
        self.position += self.speed;

        // Slip mode: advance the "real" position even during loops/repeats
        if self.slip_mode {
            self.slip_position += self.speed.abs(); // always forward
        }
 
        // Signal chain: EQ → Filter → Delay → Reverb → Gain
        let (l, r) = self.eq.process(l, r);
        let (l, r) = self.filter.process(l, r);
        let (l, r) = self.delay.process(l, r, self.bpm);
        let (l, r) = self.reverb.process(l, r);
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
                self.position / t.total_frames() as f64
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

    /// Set EQ kill switch for a band.
    pub fn set_eq_kill(&mut self, band: EqBand, enabled: bool) {
        let band_index = match band {
            EqBand::Low => 0,
            EqBand::Mid => 1,
            EqBand::High => 2,
        };
        self.eq.set_kill(band_index, enabled);
    }

    /// Set EQ mode (classic / isolator).
    pub fn set_eq_mode(&mut self, mode: EqMode) {
        self.eq.set_isolator_mode(mode == EqMode::Isolator);
    }

    /// Set filter parameters.
    pub fn set_filter(&mut self, cutoff_hz: f32, resonance: f32) {
        self.filter.set_params(cutoff_hz, resonance);
    }

    /// Start beat repeat: loop a region of `beats` beats at the current position.
    /// `samples_per_beat` is derived from the transport clock BPM.
    pub fn start_beat_repeat(&mut self, beats: f32, samples_per_beat: usize) {
        let repeat_length = (beats * samples_per_beat as f32) as usize;
        if repeat_length == 0 { return; }
        let start = self.position as usize;
        if self.slip_mode {
            self.slip_position = self.position;
        }
        self.beat_repeat_start = Some(start);
        self.beat_repeat_end = Some(start + repeat_length);
    }

    /// Stop beat repeat. If slip mode is on, jump to the slip position.
    pub fn stop_beat_repeat(&mut self) {
        self.beat_repeat_start = None;
        self.beat_repeat_end = None;
        if self.slip_mode {
            self.position = self.slip_position;
        }
    }

    /// Start vinyl brake effect.
    pub fn start_brake(&mut self, duration_frames: u32) {
        self.brake_initial_speed = self.speed;
        self.brake_total = duration_frames;
        self.brake_remaining = duration_frames;
    }

    /// Start spinback effect.
    pub fn start_spinback(&mut self, duration_frames: u32) {
        self.spinback_initial_speed = self.speed;
        self.spinback_total = duration_frames;
        self.spinback_remaining = duration_frames;
    }

    /// Set delay parameters and activate.
    pub fn set_delay(&mut self, subdivision: f32, feedback: f32, wet: f32, ping_pong: bool) {
        self.delay.set_params(subdivision, feedback, wet, ping_pong);
    }

    /// Clear delay effect.
    pub fn clear_delay(&mut self) {
        self.delay.clear();
    }

    /// Set reverb parameters.
    pub fn set_reverb(&mut self, room_size: f32, damping: f32, wet: f32) {
        self.reverb.set_params(room_size, damping, wet);
    }

    /// Clear reverb effect.
    pub fn clear_reverb(&mut self) {
        self.reverb.clear();
    }

    /// Enable/disable slip mode.
    pub fn set_slip_mode(&mut self, enabled: bool) {
        self.slip_mode = enabled;
        if enabled {
            self.slip_position = self.position;
        }
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
    use crate::buffer::{AdjpHeader, TrackBuffer};

    /// Create a test TrackBuffer with the given number of stereo frames.
    fn make_test_buffer(frames: usize) -> Arc<TrackBuffer> {
        use std::io::Write;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let path = tmp.path().to_path_buf();

        let total_samples = frames * 2; // stereo
        let header = AdjpHeader {
            sample_rate: 48000,
            channels: 2,
            bit_depth: 32,
            total_samples: total_samples as u64,
            duration_ms: (frames as u64 * 1000) / 48000,
        };

        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(&header.to_bytes()).unwrap();
        for i in 0..frames {
            let val = (i as f32) / (frames as f32);
            f.write_all(&val.to_le_bytes()).unwrap(); // L
            f.write_all(&val.to_le_bytes()).unwrap(); // R
        }
        drop(f);

        let buf = TrackBuffer::open(&path).unwrap();
        assert_eq!(buf.total_frames(), frames);
        std::mem::forget(tmp);
        Arc::new(buf)
    }

    #[test]
    fn test_seek() {
        let mut deck = Deck::new();
        let buf = make_test_buffer(1000);
        deck.load_track("test_track".to_string(), buf);

        deck.seek(500);
        assert_eq!(deck.position, 500.0);

        // Seek past end should clamp to last frame
        deck.seek(99999);
        assert_eq!(deck.position, 999.0);
    }

    #[test]
    fn test_loop_playback() {
        let mut deck = Deck::new();
        let buf = make_test_buffer(1000);
        deck.load_track("test_track".to_string(), buf);
        deck.play();

        // Set a loop from frame 100 to frame 200
        deck.set_loop(100, 200);
        assert!(deck.loop_active());

        // Seek right before loop end and advance past it
        deck.seek(199);
        // position = 199, next_frame reads 199 and advances to 200
        deck.next_frame();
        // Now position = 200, which is >= loop_end, so next call wraps
        deck.next_frame();
        // After wrap: position was set to 100, then read and advanced to 101
        assert!(
            deck.position >= 100.0 && deck.position <= 102.0,
            "Expected ~101, got {}",
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
