//! Audio engine: owns cpal stream, SPSC queues, decks, and mixer.
//!
//! The engine runs the audio callback on the cpal thread and
//! communicates with the rest of the application via lock-free
//! ring buffers. Commands can be scheduled for beat/bar-aligned
//! execution via a pre-allocated pending queue.

use crate::buffer::TrackBuffer;
use crate::command::{Command, CommandSource, CommandValidator, DeckId, SchedulePoint, ScheduledCommand};
use crate::deck::Deck;
use crate::mixer::Mixer;
use crate::recorder::{RecordProducer, RecordingFlag};
use crate::transport::TransportClock;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// Size of the command ring buffer (max commands in flight).
const COMMAND_RING_SIZE: usize = 256;

/// Size of the state snapshot ring buffer.
const STATE_RING_SIZE: usize = 4;

/// Max pending scheduled commands in the audio callback.
const MAX_PENDING_COMMANDS: usize = 256;

/// Number of spectrum analyzer bands.
const SPECTRUM_BANDS: usize = 8;

/// A read-only snapshot of engine state for the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStateSnapshot {
    pub deck_a: DeckSnapshot,
    pub deck_b: DeckSnapshot,
    pub crossfader: f32,
    pub master_bpm: f64,
    pub beat_number: u64,
    pub bar_number: u64,
    pub sample_pos: u64,
    /// RMS level of the master output (0.0 = silence, 1.0 = full scale)
    #[serde(default)]
    pub master_rms: f32,
    /// Number of consecutive silent frames (RMS < threshold)
    #[serde(default)]
    pub silence_frames: u32,
    /// Buffer underrun count (audio callback received empty data)
    #[serde(default)]
    pub underrun_count: u32,
    /// 8-band spectrum analyzer levels (0.0–1.0 per band)
    #[serde(default = "default_spectrum")]
    pub spectrum: [f32; SPECTRUM_BANDS],
}

fn default_spectrum() -> [f32; SPECTRUM_BANDS] {
    [0.0; SPECTRUM_BANDS]
}

/// Snapshot of a single deck's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSnapshot {
    pub state: String,
    pub track_id: Option<String>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub position_fraction: f64,
    pub gain_db: f32,
    pub eq_low: f32,
    pub eq_mid: f32,
    pub eq_high: f32,
    pub eq_kill_low: bool,
    pub eq_kill_mid: bool,
    pub eq_kill_high: bool,
    pub eq_isolator: bool,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub loop_active: bool,
    pub loop_start_ms: u64,
    pub loop_end_ms: u64,
    pub speed: f64,
    /// Per-deck RMS level (0.0–1.0)
    #[serde(default)]
    pub rms: f32,
    /// Per-deck peak level (0.0–1.0)
    #[serde(default)]
    pub peak: f32,
}

impl DeckSnapshot {
    fn from_deck(deck: &Deck, sample_rate: u32, rms: f32, peak: f32) -> Self {
        let duration_ms = deck
            .track
            .as_ref()
            .map(|t| t.duration_ms())
            .unwrap_or(0);
        let state = match deck.state {
            crate::deck::TransportState::Empty => "empty",
            crate::deck::TransportState::Paused => "paused",
            crate::deck::TransportState::Playing => "playing",
        };
        let eq_db = deck.eq.gains_db();
        let loop_start_ms = deck
            .loop_start
            .map(|f| (f as u64 * 1000) / sample_rate as u64)
            .unwrap_or(0);
        let loop_end_ms = deck
            .loop_end
            .map(|f| (f as u64 * 1000) / sample_rate as u64)
            .unwrap_or(0);

        Self {
            state: state.to_string(),
            track_id: deck.track_id.clone(),
            position_ms: deck.position_ms(sample_rate),
            duration_ms,
            position_fraction: deck.position_fraction(),
            gain_db: if deck.gain <= 0.0 {
                -96.0
            } else {
                20.0 * deck.gain.log10()
            },
            eq_low: eq_db[0],
            eq_mid: eq_db[1],
            eq_high: eq_db[2],
            eq_kill_low: deck.eq.kill_low,
            eq_kill_mid: deck.eq.kill_mid,
            eq_kill_high: deck.eq.kill_high,
            eq_isolator: deck.eq.isolator_mode,
            filter_cutoff: deck.filter.cutoff_hz,
            filter_resonance: deck.filter.resonance,
            loop_active: deck.loop_active(),
            loop_start_ms,
            loop_end_ms,
            speed: deck.speed,
            rms,
            peak,
        }
    }
}

/// Handle for sending commands to the engine from other threads.
pub struct EngineCommandSender {
    producer: ringbuf::HeapProd<ScheduledCommand>,
}

impl EngineCommandSender {
    /// Send an immediate command to the engine. Returns false if the queue is full.
    pub fn send(&mut self, command: Command, source: CommandSource) -> bool {
        self.send_scheduled(command, source, SchedulePoint::Immediate)
    }

    /// Send a scheduled command. Returns false if the queue is full.
    pub fn send_scheduled(
        &mut self,
        command: Command,
        source: CommandSource,
        schedule: SchedulePoint,
    ) -> bool {
        if let Command::ExecuteTransition { template, params } = &command {
            match crate::transition::TransitionExpander::expand(template, params) {
                Ok(commands) => {
                    let mut all_ok = true;
                    for mut cmd in commands {
                        cmd.source = source;
                        // For MVP, we let the inner commands dictate their schedule 
                        // (e.g., NextBar, AfterBeats relative to NextBar)
                        if self.producer.try_push(cmd).is_err() {
                            all_ok = false;
                        }
                    }
                    return all_ok;
                }
                Err(e) => {
                    log::error!("Failed to expand transition: {}", e);
                    return false;
                }
            }
        }

        let validated = match CommandValidator::validate(command) {
            Some(cmd) => cmd,
            None => return false,
        };

        let scheduled = ScheduledCommand {
            command: validated,
            source,
            schedule,
            target_sample: None,
        };

        self.producer.try_push(scheduled).is_ok()
    }
}

/// Handle for reading state snapshots from the engine.
pub struct EngineStateReceiver {
    consumer: ringbuf::HeapCons<EngineStateSnapshot>,
}

impl EngineStateReceiver {
    /// Get the latest state snapshot, draining any older ones.
    pub fn latest(&mut self) -> Option<EngineStateSnapshot> {
        let mut latest = None;
        while let Some(snapshot) = self.consumer.try_pop() {
            latest = Some(snapshot);
        }
        latest
    }
}

/// The audio engine.
pub struct AudioEngine {
    pub command_sender: EngineCommandSender,
    pub state_receiver: EngineStateReceiver,
    pub sample_rate: u32,
}

impl AudioEngine {
    /// Create and start the audio engine with default output device.
    /// Optionally accepts a recording producer + flag for WAV capture.
    pub fn new(
        track_load_receiver: TrackLoadReceiver,
        record_producer: Option<RecordProducer>,
        recording_flag: Option<RecordingFlag>,
    ) -> Result<(Self, Stream), String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No default audio output device found")?;

        let supported_config = device
            .default_output_config()
            .map_err(|e| format!("Failed to get default output config: {e}"))?;

        let sample_rate = supported_config.sample_rate().0;
        let channels = supported_config.channels() as usize;

        // Create SPSC ring buffers
        let cmd_ring = HeapRb::<ScheduledCommand>::new(COMMAND_RING_SIZE);
        let (cmd_prod, mut cmd_cons) = cmd_ring.split();

        let state_ring = HeapRb::<EngineStateSnapshot>::new(STATE_RING_SIZE);
        let (mut state_prod, state_cons) = state_ring.split();

        // Engine state (lives on the audio thread)
        let mut deck_a = Deck::with_sample_rate(sample_rate as f32);
        let mut deck_b = Deck::with_sample_rate(sample_rate as f32);
        let mut mixer = Mixer::new(sample_rate as f64);
        let mut transport = TransportClock::new(sample_rate as f64, 128.0);
        let mut frame_counter: u64 = 0;
        let snapshot_interval = sample_rate as u64 / 30; // ~30 Hz
        let sr = sample_rate;

        // Pre-allocated pending command queue (sorted by target_sample)
        let mut pending_commands: Vec<ScheduledCommand> = Vec::with_capacity(MAX_PENDING_COMMANDS);

        // Track load receiver (moved into the audio callback)
        let mut track_loader = track_load_receiver;

        // Recording state (moved into the audio callback)
        let mut rec_producer = record_producer;
        let rec_flag = recording_flag;

        // Silence detection state
        let mut rms_accumulator: f64 = 0.0;
        let mut rms_frame_count: u64 = 0;
        let mut current_rms: f32 = 0.0;
        let mut silence_frame_counter: u32 = 0;
        let underrun_counter: u32 = 0;
        let silence_threshold: f32 = 0.0001; // ~-80 dB

        // Per-deck level metering state
        let mut deck_a_rms_acc: f64 = 0.0;
        let mut deck_a_peak: f32 = 0.0;
        let mut deck_b_rms_acc: f64 = 0.0;
        let mut deck_b_peak: f32 = 0.0;
        let mut deck_a_rms: f32 = 0.0;
        let mut deck_a_pk: f32 = 0.0;
        let mut deck_b_rms: f32 = 0.0;
        let mut deck_b_pk: f32 = 0.0;

        // 8-band spectrum analyzer state (biquad bandpass filters)
        let band_freqs: [f32; SPECTRUM_BANDS] = [
            32.0, 64.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0,
        ];
        let q = 1.4f32; // ~1 octave bandwidth
        // Biquad state: [b0, b1, b2, a1, a2, x1, x2, y1, y2]
        let mut band_state: [[f32; 9]; SPECTRUM_BANDS] = [[0.0; 9]; SPECTRUM_BANDS];
        let mut band_energy: [f64; SPECTRUM_BANDS] = [0.0; SPECTRUM_BANDS];
        let mut current_spectrum: [f32; SPECTRUM_BANDS] = [0.0; SPECTRUM_BANDS];

        // Pre-compute biquad coefficients for each band
        let sr_f = sample_rate as f32;
        for (i, &freq) in band_freqs.iter().enumerate() {
            let w0 = 2.0 * std::f32::consts::PI * freq / sr_f;
            let alpha = w0.sin() / (2.0 * q);
            let a0 = 1.0 + alpha;
            band_state[i][0] = alpha / a0;       // b0
            band_state[i][1] = 0.0;               // b1
            band_state[i][2] = -alpha / a0;       // b2
            band_state[i][3] = -2.0 * w0.cos() / a0; // a1
            band_state[i][4] = (1.0 - alpha) / a0;   // a2
            // x1, x2, y1, y2 start at 0
        }

        let config = supported_config.config();

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Check for pending track loads (non-blocking)
                    while let Some((deck_id, track_id, track_buf)) = track_loader.try_recv() {
                        match deck_id {
                            DeckId::A => deck_a.load_track(track_id, track_buf),
                            DeckId::B => deck_b.load_track(track_id, track_buf),
                        }
                    }

                    // Receive new commands and either execute immediately or schedule
                    while let Some(mut cmd) = cmd_cons.try_pop() {
                        match cmd.schedule {
                            SchedulePoint::Immediate => {
                                Self::execute_command(
                                    &cmd.command,
                                    &mut deck_a,
                                    &mut deck_b,
                                    &mut mixer,
                                    &mut transport,
                                    sr,
                                );
                            }
                            _ => {
                                // Resolve schedule to target sample
                                let target = transport.resolve_schedule(&cmd.schedule);
                                cmd.target_sample = Some(target);

                                // Insert into pending queue (keep sorted by target_sample)
                                if pending_commands.len() < MAX_PENDING_COMMANDS {
                                    let pos = pending_commands
                                        .binary_search_by_key(&target, |c| {
                                            c.target_sample.unwrap_or(0)
                                        })
                                        .unwrap_or_else(|x| x);
                                    pending_commands.insert(pos, cmd);
                                }
                                // If full, drop the command (oldest non-essential eviction
                                // could be added later)
                            }
                        }
                    }

                    // Fill the output buffer
                    for frame in data.chunks_mut(channels) {
                        // Execute any pending commands that are due
                        while let Some(cmd) = pending_commands.first() {
                            if let Some(target) = cmd.target_sample {
                                if target <= transport.sample_pos {
                                    let cmd = pending_commands.remove(0);
                                    Self::execute_command(
                                        &cmd.command,
                                        &mut deck_a,
                                        &mut deck_b,
                                        &mut mixer,
                                        &mut transport,
                                        sr,
                                    );
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }

                        let a = deck_a.next_frame();
                        let b = deck_b.next_frame();
                        let (l, r) = mixer.mix(a, b);

                        // Write to output
                        if channels >= 2 {
                            frame[0] = l;
                            frame[1] = r;
                            for ch in frame.iter_mut().skip(2) {
                                *ch = 0.0;
                            }
                        } else if channels == 1 {
                            frame[0] = (l + r) * 0.5;
                        }

                        // Per-deck level metering
                        deck_a_rms_acc += (a.0 * a.0 + a.1 * a.1) as f64 * 0.5;
                        deck_a_peak = deck_a_peak.max(a.0.abs().max(a.1.abs()));
                        deck_b_rms_acc += (b.0 * b.0 + b.1 * b.1) as f64 * 0.5;
                        deck_b_peak = deck_b_peak.max(b.0.abs().max(b.1.abs()));

                        // RMS accumulation for silence detection
                        rms_accumulator += (l * l + r * r) as f64 * 0.5;
                        rms_frame_count += 1;

                        // 8-band spectrum analyzer: run mono master through bandpass filters
                        let mono = (l + r) * 0.5;
                        for i in 0..SPECTRUM_BANDS {
                            let s = &mut band_state[i];
                            let y = s[0] * mono + s[1] * s[5] + s[2] * s[6]
                                  - s[3] * s[7] - s[4] * s[8];
                            s[6] = s[5]; // x2 = x1
                            s[5] = mono;  // x1 = x
                            s[8] = s[7]; // y2 = y1
                            s[7] = y;     // y1 = y
                            band_energy[i] += (y * y) as f64;
                        }

                        // Recording tap: push stereo samples to ring buffer
                        if let Some(ref flag) = rec_flag {
                            if flag.load(Ordering::Relaxed) {
                                if let Some(ref mut prod) = rec_producer {
                                    let _ = prod.try_push(l);
                                    let _ = prod.try_push(r);
                                }
                            }
                        }

                        transport.advance();
                        frame_counter += 1;

                        // Emit state snapshot at ~30 Hz
                        if frame_counter.is_multiple_of(snapshot_interval) {
                            // Compute RMS for this snapshot interval
                            if rms_frame_count > 0 {
                                current_rms = (rms_accumulator / rms_frame_count as f64).sqrt() as f32;
                                if current_rms < silence_threshold {
                                    silence_frame_counter = silence_frame_counter.saturating_add(1);
                                } else {
                                    silence_frame_counter = 0;
                                }

                                // Per-deck RMS (with peak hold and slow decay)
                                deck_a_rms = (deck_a_rms_acc / rms_frame_count as f64).sqrt() as f32;
                                deck_a_pk = deck_a_peak;
                                deck_b_rms = (deck_b_rms_acc / rms_frame_count as f64).sqrt() as f32;
                                deck_b_pk = deck_b_peak;

                                // Spectrum: compute RMS per band and normalize
                                let mut max_band: f64 = 0.0;
                                for i in 0..SPECTRUM_BANDS {
                                    let rms_val = (band_energy[i] / rms_frame_count as f64).sqrt();
                                    current_spectrum[i] = rms_val as f32;
                                    if rms_val > max_band {
                                        max_band = rms_val;
                                    }
                                    band_energy[i] = 0.0;
                                }
                                // Normalize spectrum to 0.0–1.0 range
                                if max_band > 0.0 {
                                    for v in &mut current_spectrum {
                                        *v = (*v as f64 / max_band).min(1.0) as f32;
                                    }
                                }

                                rms_accumulator = 0.0;
                                rms_frame_count = 0;
                                deck_a_rms_acc = 0.0;
                                deck_a_peak = 0.0;
                                deck_b_rms_acc = 0.0;
                                deck_b_peak = 0.0;
                            }

                            let snapshot = EngineStateSnapshot {
                                deck_a: DeckSnapshot::from_deck(&deck_a, sr, deck_a_rms, deck_a_pk),
                                deck_b: DeckSnapshot::from_deck(&deck_b, sr, deck_b_rms, deck_b_pk),
                                crossfader: mixer.crossfader,
                                master_bpm: transport.bpm,
                                beat_number: transport.beat_number,
                                bar_number: transport.bar_number,
                                sample_pos: transport.sample_pos,
                                master_rms: current_rms,
                                silence_frames: silence_frame_counter,
                                underrun_count: underrun_counter,
                                spectrum: current_spectrum,
                            };
                            let _ = state_prod.try_push(snapshot);
                        }
                    }
                },
                move |err| {
                    log::error!("Audio stream error: {err}");
                },
                None,
            )
            .map_err(|e| format!("Failed to build output stream: {e}"))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start audio stream: {e}"))?;

        Ok((
            Self {
                command_sender: EngineCommandSender { producer: cmd_prod },
                state_receiver: EngineStateReceiver { consumer: state_cons },
                sample_rate,
            },
            stream,
        ))
    }

    /// Execute a command on the engine state (called from the audio callback).
    fn execute_command(
        command: &Command,
        deck_a: &mut Deck,
        deck_b: &mut Deck,
        mixer: &mut Mixer,
        transport: &mut TransportClock,
        _sample_rate: u32,
    ) {
        match command {
            Command::Play { deck } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).play();
            }
            Command::Pause { deck } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).pause();
            }
            Command::Seek { deck, beat } => {
                let frame = transport.beat_to_frame(*beat);
                Self::get_deck_mut(*deck, deck_a, deck_b).seek(frame);
            }
            Command::SetCrossfader { position } => {
                mixer.set_crossfader(*position);
            }
            Command::SetDeckGain { deck, gain_db } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).set_gain_db(*gain_db);
            }
            Command::SetMasterGain { gain_db } => {
                mixer.set_master_gain_db(*gain_db);
            }
            Command::SetTempo { bpm, .. } => {
                transport.set_bpm(*bpm);
            }
            Command::SetSpeed { deck, speed } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).speed = *speed;
            }
            Command::SetEq { deck, band, gain_db } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).set_eq(*band, *gain_db);
            }
            Command::SetEqKill { deck, band, enabled } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).set_eq_kill(*band, *enabled);
            }
            Command::SetEqMode { deck, mode } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).set_eq_mode(*mode);
            }
            Command::SetFilter { deck, cutoff_hz, resonance } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).set_filter(*cutoff_hz, *resonance);
            }
            Command::SyncToMaster { deck } => {
                // Deck track_bpm and set_speed are not available on the primitive Deck struct
                // BPM matching is handled by the higher-level AppState transitioning logic
                let _ = deck;
                log::debug!("SyncToMaster: Handled by AppState using TrackProfile");
            }
            Command::SetLoop { deck, start_beat, length_bars } => {
                let start_frame = transport.beat_to_frame(*start_beat);
                let beats = *length_bars as u64 * transport.beats_per_bar as u64;
                let end_frame = start_frame + transport.beat_to_frame(beats);
                Self::get_deck_mut(*deck, deck_a, deck_b).set_loop(start_frame, end_frame);
            }
            Command::ClearLoop { deck } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).clear_loop();
            }
            Command::SeekMs { deck, position_ms } => {
                let frame = transport.ms_to_frames(*position_ms);
                Self::get_deck_mut(*deck, deck_a, deck_b).seek(frame);
            }
            Command::BeatRepeat { deck, beats } => {
                let spb = transport.samples_per_beat as usize;
                Self::get_deck_mut(*deck, deck_a, deck_b).start_beat_repeat(*beats, spb);
            }
            Command::BeatRepeatOff { deck } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).stop_beat_repeat();
            }
            Command::SetSlipMode { deck, enabled } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).set_slip_mode(*enabled);
            }
            Command::VinylBrake { deck, duration_ms } => {
                let frames = transport.ms_to_frames(*duration_ms as u64) as u32;
                Self::get_deck_mut(*deck, deck_a, deck_b).start_brake(frames);
            }
            Command::Spinback { deck, duration_ms } => {
                let frames = transport.ms_to_frames(*duration_ms as u64) as u32;
                Self::get_deck_mut(*deck, deck_a, deck_b).start_spinback(frames);
            }
            Command::JumpToMarker { deck, position_ms, .. } => {
                let frame = transport.ms_to_frames(*position_ms);
                Self::get_deck_mut(*deck, deck_a, deck_b).seek(frame);
            }
            _ => {
                log::debug!("Command not yet implemented: {:?}", command);
            }
        }
    }

    fn get_deck_mut<'a>(
        id: DeckId,
        deck_a: &'a mut Deck,
        deck_b: &'a mut Deck,
    ) -> &'a mut Deck {
        match id {
            DeckId::A => deck_a,
            DeckId::B => deck_b,
        }
    }
}

/// Load a track into the engine from outside the audio callback.
pub struct TrackLoader {
    producer: ringbuf::HeapProd<(DeckId, String, Arc<TrackBuffer>)>,
}

/// Consumer side for the audio callback to receive track loads.
pub struct TrackLoadReceiver {
    consumer: ringbuf::HeapCons<(DeckId, String, Arc<TrackBuffer>)>,
}

impl TrackLoadReceiver {
    pub fn try_recv(&mut self) -> Option<(DeckId, String, Arc<TrackBuffer>)> {
        self.consumer.try_pop()
    }
}

/// Create a track loader pair (sender + receiver).
pub fn create_track_loader() -> (TrackLoader, TrackLoadReceiver) {
    let ring = HeapRb::<(DeckId, String, Arc<TrackBuffer>)>::new(4);
    let (prod, cons) = ring.split();
    (
        TrackLoader { producer: prod },
        TrackLoadReceiver { consumer: cons },
    )
}

impl TrackLoader {
    pub fn load(&mut self, deck: DeckId, track_id: String, track: Arc<TrackBuffer>) -> bool {
        self.producer.try_push((deck, track_id, track)).is_ok()
    }
}
