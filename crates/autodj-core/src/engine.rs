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
use crate::transport::TransportClock;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use ringbuf::{
    traits::{Consumer, Producer, Split},
    HeapRb,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Size of the command ring buffer (max commands in flight).
const COMMAND_RING_SIZE: usize = 256;

/// Size of the state snapshot ring buffer.
const STATE_RING_SIZE: usize = 4;

/// Max pending scheduled commands in the audio callback.
const MAX_PENDING_COMMANDS: usize = 256;

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
}

/// Snapshot of a single deck's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckSnapshot {
    pub state: String,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub position_fraction: f64,
    pub gain_db: f32,
    pub eq_low: f32,
    pub eq_mid: f32,
    pub eq_high: f32,
    pub filter_cutoff: f32,
    pub filter_resonance: f32,
    pub loop_active: bool,
    pub loop_start_ms: u64,
    pub loop_end_ms: u64,
}

impl DeckSnapshot {
    fn from_deck(deck: &Deck, sample_rate: u32) -> Self {
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
            filter_cutoff: deck.filter.cutoff_hz,
            filter_resonance: deck.filter.resonance,
            loop_active: deck.loop_active(),
            loop_start_ms,
            loop_end_ms,
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
    _stream: Stream,
    pub command_sender: EngineCommandSender,
    pub state_receiver: EngineStateReceiver,
    pub sample_rate: u32,
}

impl AudioEngine {
    /// Create and start the audio engine with default output device.
    pub fn new(track_load_receiver: TrackLoadReceiver) -> Result<Self, String> {
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

        let config = supported_config.config();

        let stream = device
            .build_output_stream(
                &config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Check for pending track loads (non-blocking)
                    while let Some((deck_id, track_buf)) = track_loader.try_recv() {
                        match deck_id {
                            DeckId::A => deck_a.load_track(track_buf),
                            DeckId::B => deck_b.load_track(track_buf),
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

                        transport.advance();
                        frame_counter += 1;

                        // Emit state snapshot at ~30 Hz
                        if frame_counter % snapshot_interval == 0 {
                            let snapshot = EngineStateSnapshot {
                                deck_a: DeckSnapshot::from_deck(&deck_a, sr),
                                deck_b: DeckSnapshot::from_deck(&deck_b, sr),
                                crossfader: mixer.crossfader,
                                master_bpm: transport.bpm,
                                beat_number: transport.beat_number,
                                bar_number: transport.bar_number,
                                sample_pos: transport.sample_pos,
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

        Ok(Self {
            _stream: stream,
            command_sender: EngineCommandSender { producer: cmd_prod },
            state_receiver: EngineStateReceiver { consumer: state_cons },
            sample_rate,
        })
    }

    /// Execute a command on the engine state (called from the audio callback).
    fn execute_command(
        command: &Command,
        deck_a: &mut Deck,
        deck_b: &mut Deck,
        mixer: &mut Mixer,
        transport: &mut TransportClock,
        sample_rate: u32,
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
            Command::SetEq { deck, band, gain_db } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).set_eq(*band, *gain_db);
            }
            Command::SetFilter { deck, cutoff_hz, resonance } => {
                Self::get_deck_mut(*deck, deck_a, deck_b).set_filter(*cutoff_hz, *resonance);
            }
            Command::SyncToMaster { deck } => {
                // Sync deck speed to master BPM (simple BPM match for now)
                let _ = deck; // Will need analysis BPM per track in Milestone D
                log::debug!("SyncToMaster: BPM matching deferred until track analysis");
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
    producer: ringbuf::HeapProd<(DeckId, Arc<TrackBuffer>)>,
}

/// Consumer side for the audio callback to receive track loads.
pub struct TrackLoadReceiver {
    consumer: ringbuf::HeapCons<(DeckId, Arc<TrackBuffer>)>,
}

impl TrackLoadReceiver {
    pub fn try_recv(&mut self) -> Option<(DeckId, Arc<TrackBuffer>)> {
        self.consumer.try_pop()
    }
}

/// Create a track loader pair (sender + receiver).
pub fn create_track_loader() -> (TrackLoader, TrackLoadReceiver) {
    let ring = HeapRb::<(DeckId, Arc<TrackBuffer>)>::new(4);
    let (prod, cons) = ring.split();
    (
        TrackLoader { producer: prod },
        TrackLoadReceiver { consumer: cons },
    )
}

impl TrackLoader {
    pub fn load(&mut self, deck: DeckId, track: Arc<TrackBuffer>) -> bool {
        self.producer.try_push((deck, track)).is_ok()
    }
}
