//! Command types, validation, and ownership model.
//!
//! All interactions with the audio engine go through commands.
//! Commands are validated, clamped, and placed into the SPSC ring buffer.

use serde::{Deserialize, Serialize};

/// Identifies which deck a command targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeckId {
    A,
    B,
}

/// When a command should execute, relative to the transport clock.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SchedulePoint {
    /// Execute on the next audio buffer.
    Immediate,
    /// Execute at the next beat boundary.
    NextBeat,
    /// Execute at the next bar (downbeat).
    NextBar,
    /// Execute at a specific beat number.
    AtBeat(u64),
    /// Execute N beats from now.
    AfterBeats(u32),
    /// Execute N bars from now.
    AfterBars(u32),
}

/// Who issued the command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandSource {
    User,
    AI,
    System,
}

/// Who currently owns a parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Owner {
    System,
    AI,
    User,
}

/// EQ band identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EqBand {
    Low,
    Mid,
    High,
}

/// EQ processing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EqMode {
    /// Classic 3-band EQ with smooth crossovers.
    Classic,
    /// DJ-style isolator with steep slopes for hard kills.
    Isolator,
}

/// The DJ operating mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DjMode {
    Manual,
    Assisted,
    AutoDJ,
}

/// The core command payload sent to the audio engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    // -- Deck transport --
    LoadTrack { deck: DeckId, track_id: String },
    Play { deck: DeckId },
    Pause { deck: DeckId },
    Seek { deck: DeckId, beat: u64 },

    // -- Loop --
    SetLoop { deck: DeckId, start_beat: u64, length_bars: u32 },
    ClearLoop { deck: DeckId },

    // -- Mixer --
    SetCrossfader { position: f32 },
    SetDeckGain { deck: DeckId, gain_db: f32 },
    SetEq { deck: DeckId, band: EqBand, gain_db: f32 },
    SetEqKill { deck: DeckId, band: EqBand, enabled: bool },
    SetEqMode { deck: DeckId, mode: EqMode },
    SetFilter { deck: DeckId, cutoff_hz: f32, resonance: f32 },
    SetMasterGain { gain_db: f32 },

    // -- Tempo --
    SetTempo { deck: DeckId, bpm: f64 },
    SetSpeed { deck: DeckId, speed: f64 },
    SyncToMaster { deck: DeckId },

    // -- Mode --
    SetMode { mode: DjMode },
    ResumeAI,

    // -- Transition --
    ExecuteTransition { template: String, params: serde_json::Value },

    // -- Phase 19: DJ-style live mixing --
    /// Seek to a position in milliseconds (for section-aware transitions).
    SeekMs { deck: DeckId, position_ms: u64 },

    // -- Phase 30: DJ Performance Engine --
    /// Beat repeat: loop the current beat for `beats` beats (1/4, 1/2, 1, 2, 4).
    BeatRepeat { deck: DeckId, beats: f32 },
    /// Stop beat repeat and resume normal playback.
    BeatRepeatOff { deck: DeckId },
    /// Enable/disable slip mode (stutters/repeats don't derail track position).
    SetSlipMode { deck: DeckId, enabled: bool },
    /// Vinyl brake effect: gradually slow to stop over `duration_ms`.
    VinylBrake { deck: DeckId, duration_ms: u32 },
    /// Spinback effect: reverse playback with deceleration over `duration_ms`.
    Spinback { deck: DeckId, duration_ms: u32 },
    /// Tempo-synced delay effect.
    SetDelay { deck: DeckId, subdivision: f32, feedback: f32, wet: f32, ping_pong: bool },
    /// Clear delay effect.
    ClearDelay { deck: DeckId },
    /// Jump to a named marker position (intro, drop, break, outro, first_beat).
    JumpToMarker { deck: DeckId, marker: String, position_ms: u64 },
}

/// A command wrapped with scheduling and source metadata.
#[derive(Debug, Clone)]
pub struct ScheduledCommand {
    pub command: Command,
    pub source: CommandSource,
    pub schedule: SchedulePoint,
    /// Target sample position (resolved by the transport clock).
    pub target_sample: Option<u64>,
}

/// Validates and clamps command parameters.
pub struct CommandValidator;

impl CommandValidator {
    /// Validate and clamp a command's parameters.
    /// Returns the (possibly clamped) command, or None if the command is invalid.
    pub fn validate(command: Command) -> Option<Command> {
        match command {
            Command::SetCrossfader { position } => {
                Some(Command::SetCrossfader {
                    position: position.clamp(-1.0, 1.0),
                })
            }
            Command::SetDeckGain { deck, gain_db } => {
                Some(Command::SetDeckGain {
                    deck,
                    gain_db: gain_db.clamp(-96.0, 12.0),
                })
            }
            Command::SetEq { deck, band, gain_db } => {
                Some(Command::SetEq {
                    deck,
                    band,
                    gain_db: gain_db.clamp(-96.0, 12.0),
                })
            }
            Command::SetFilter { deck, cutoff_hz, resonance } => {
                Some(Command::SetFilter {
                    deck,
                    cutoff_hz: cutoff_hz.clamp(20.0, 20000.0),
                    resonance: resonance.clamp(0.5, 4.0),
                })
            }
            Command::SetMasterGain { gain_db } => {
                Some(Command::SetMasterGain {
                    gain_db: gain_db.clamp(-96.0, 6.0),
                })
            }
            Command::SetTempo { deck, bpm } => {
                Some(Command::SetTempo {
                    deck,
                    bpm: bpm.clamp(60.0, 200.0),
                })
            }
            // All other commands pass through unchanged.
            Command::ExecuteTransition { template, params } => {
                Some(Command::ExecuteTransition { template, params })
            }
            Command::SetLoop { deck, start_beat, length_bars } => {
                Some(Command::SetLoop {
                    deck,
                    start_beat,
                    length_bars: length_bars.clamp(1, 64),
                })
            }
            Command::BeatRepeat { deck, beats } => {
                Some(Command::BeatRepeat {
                    deck,
                    beats: beats.clamp(0.25, 8.0),
                })
            }
            Command::VinylBrake { deck, duration_ms } => {
                Some(Command::VinylBrake {
                    deck,
                    duration_ms: duration_ms.clamp(100, 5000),
                })
            }
            Command::Spinback { deck, duration_ms } => {
                Some(Command::Spinback {
                    deck,
                    duration_ms: duration_ms.clamp(100, 5000),
                })
            }
            Command::SetDelay { deck, subdivision, feedback, wet, ping_pong } => {
                Some(Command::SetDelay {
                    deck,
                    subdivision: subdivision.clamp(0.0625, 4.0),
                    feedback: feedback.clamp(0.0, 0.85),
                    wet: wet.clamp(0.0, 1.0),
                    ping_pong,
                })
            }
            Command::ClearDelay { deck } => Some(Command::ClearDelay { deck }),
            other @ (Command::SetEqKill { .. } | Command::SetEqMode { .. } | Command::SeekMs { .. }
                    | Command::BeatRepeatOff { .. } | Command::SetSlipMode { .. }
                    | Command::JumpToMarker { .. }) => {
                Some(other)
            }
            other => Some(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crossfader_clamping() {
        let cmd = CommandValidator::validate(Command::SetCrossfader { position: 5.0 });
        match cmd {
            Some(Command::SetCrossfader { position }) => {
                assert!((position - 1.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected SetCrossfader"),
        }

        let cmd = CommandValidator::validate(Command::SetCrossfader { position: -3.0 });
        match cmd {
            Some(Command::SetCrossfader { position }) => {
                assert!((position - (-1.0)).abs() < f32::EPSILON);
            }
            _ => panic!("Expected SetCrossfader"),
        }
    }

    #[test]
    fn test_gain_clamping() {
        let cmd = CommandValidator::validate(Command::SetDeckGain {
            deck: DeckId::A,
            gain_db: 100.0,
        });
        match cmd {
            Some(Command::SetDeckGain { gain_db, .. }) => {
                assert!((gain_db - 12.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected SetDeckGain"),
        }
    }

    #[test]
    fn test_eq_clamping() {
        let cmd = CommandValidator::validate(Command::SetEq {
            deck: DeckId::A,
            band: EqBand::Low,
            gain_db: 20.0,
        });
        match cmd {
            Some(Command::SetEq { gain_db, .. }) => {
                assert!((gain_db - 12.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected SetEq"),
        }
    }

    #[test]
    fn test_filter_clamping() {
        let cmd = CommandValidator::validate(Command::SetFilter {
            deck: DeckId::A,
            cutoff_hz: 0.5,
            resonance: 10.0,
        });
        match cmd {
            Some(Command::SetFilter { cutoff_hz, resonance, .. }) => {
                assert!((cutoff_hz - 20.0).abs() < f32::EPSILON);
                assert!((resonance - 4.0).abs() < f32::EPSILON);
            }
            _ => panic!("Expected SetFilter"),
        }
    }

    #[test]
    fn test_tempo_clamping() {
        let cmd = CommandValidator::validate(Command::SetTempo {
            deck: DeckId::A,
            bpm: 300.0,
        });
        match cmd {
            Some(Command::SetTempo { bpm, .. }) => {
                assert!((bpm - 200.0).abs() < f64::EPSILON);
            }
            _ => panic!("Expected SetTempo"),
        }
    }

    #[test]
    fn test_play_passes_through() {
        let cmd = CommandValidator::validate(Command::Play { deck: DeckId::A });
        assert!(matches!(cmd, Some(Command::Play { deck: DeckId::A })));
    }
}
