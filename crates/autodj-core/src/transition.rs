use crate::command::{Command, CommandSource, DeckId, EqBand, SchedulePoint, ScheduledCommand};
use serde_json::Value;

pub trait TransitionTemplate {
    /// Expand template parameters into a sequence of primitive commands.
    fn expand(&self, params: &Value) -> Result<Vec<ScheduledCommand>, String>;
}

/// Registry for transition templates.
pub struct TransitionExpander;

impl TransitionExpander {
    pub fn expand(
        template: &str,
        params: &Value,
    ) -> Result<Vec<ScheduledCommand>, String> {
        match template {
            "smooth_beatmix" => SmoothBeatmix.expand(params),
            "filter_sweep" => FilterSweep.expand(params),
            "echo_drop" => EchoDrop.expand(params),
            _ => Err(format!("Unknown transition template: {template}")),
        }
    }
}

pub struct SmoothBeatmix;

impl TransitionTemplate for SmoothBeatmix {
    fn expand(&self, params: &Value) -> Result<Vec<ScheduledCommand>, String> {
        let duration_bars = params["duration_bars"].as_u64().unwrap_or(16) as u32;
        let incoming_deck_str = params["incoming_deck"].as_str().unwrap_or("B");
        let incoming_deck = if incoming_deck_str == "A" { DeckId::A } else { DeckId::B };
        let outgoing_deck = if incoming_deck == DeckId::A { DeckId::B } else { DeckId::A };
        let incoming_start_beat = params["incoming_start_beat"].as_u64().unwrap_or(0);
        let bass_swap_bar = params["bass_swap_bar"].as_u64().unwrap_or(8) as u32;

        let mut commands = Vec::new();

        // 1. Initial State: Incoming deck plays silently
        commands.push(ScheduledCommand {
            command: Command::Seek { deck: incoming_deck, beat: incoming_start_beat },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: incoming_deck, gain_db: -96.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::Play { deck: incoming_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });
        // Reset EQ
        commands.push(ScheduledCommand {
            command: Command::SetEq { deck: incoming_deck, band: EqBand::Low, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });

        // 2. Crossfader ramp (interpolate over duration_bars)
        let total_beats = duration_bars * 4;
        for beat in 0..=total_beats {
            let progress = beat as f32 / total_beats as f32;
            let cf = if incoming_deck == DeckId::B {
                -1.0 + (progress * 2.0)
            } else {
                1.0 - (progress * 2.0)
            };
            
            // Also ramp incoming gain up from silence to 0 dB in the first half of the transition
            let in_gain = if progress < 0.5 {
                -96.0 + (progress * 2.0 * 96.0)
            } else {
                0.0
            };

            commands.push(ScheduledCommand {
                command: Command::SetCrossfader { position: cf },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
            commands.push(ScheduledCommand {
                command: Command::SetDeckGain { deck: incoming_deck, gain_db: in_gain },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
        }

        // 3. Bass swap at bass_swap_bar
        commands.push(ScheduledCommand {
            command: Command::SetEq { deck: outgoing_deck, band: EqBand::Low, gain_db: -96.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(bass_swap_bar),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetEq { deck: incoming_deck, band: EqBand::Low, gain_db: 0.0 }, // If it was cut before
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(bass_swap_bar),
            target_sample: None,
        });

        // 4. Finalize
        commands.push(ScheduledCommand {
            command: Command::Pause { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });
        // Reset outgoing EQ
        commands.push(ScheduledCommand {
            command: Command::SetEq { deck: outgoing_deck, band: EqBand::Low, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });

        Ok(commands)
    }
}

pub struct FilterSweep;

impl TransitionTemplate for FilterSweep {
    fn expand(&self, params: &Value) -> Result<Vec<ScheduledCommand>, String> {
        let duration_bars = params["duration_bars"].as_u64().unwrap_or(8) as u32;
        let incoming_deck_str = params["incoming_deck"].as_str().unwrap_or("B");
        let incoming_deck = if incoming_deck_str == "A" { DeckId::A } else { DeckId::B };
        let outgoing_deck = if incoming_deck == DeckId::A { DeckId::B } else { DeckId::A };
        
        let start_hz = params["filter_start_hz"].as_f64().unwrap_or(200.0) as f32;
        let end_hz = params["filter_end_hz"].as_f64().unwrap_or(8000.0) as f32;

        let mut commands = Vec::new();

        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: incoming_deck, gain_db: -96.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::Play { deck: incoming_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });

        let total_beats = duration_bars * 4;
        for beat in 0..=total_beats {
            let progress = beat as f32 / total_beats as f32;
            
            // Exponential sweep
            let log_start = start_hz.ln();
            let log_end = end_hz.ln();
            let cutoff = (log_start + progress * (log_end - log_start)).exp();

            // Incoming volume ramps up
            let in_gain = -96.0 + (progress * 96.0);
            
            // Outgoing volume ramps down slightly
            let out_gain = -(progress * 12.0);

            // Crossfader moves to incoming deck in the second half
            let cf = if incoming_deck == DeckId::B {
                -1.0 + (progress * 2.0)
            } else {
                1.0 - (progress * 2.0)
            };

            commands.push(ScheduledCommand {
                command: Command::SetFilter { deck: outgoing_deck, cutoff_hz: cutoff, resonance: 1.5 },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
            commands.push(ScheduledCommand {
                command: Command::SetDeckGain { deck: incoming_deck, gain_db: in_gain },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
            commands.push(ScheduledCommand {
                command: Command::SetDeckGain { deck: outgoing_deck, gain_db: out_gain },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
            commands.push(ScheduledCommand {
                command: Command::SetCrossfader { position: cf },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
        }

        // Finalize
        commands.push(ScheduledCommand {
            command: Command::Pause { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });
        // Reset filters and gains
        commands.push(ScheduledCommand {
            command: Command::SetFilter { deck: outgoing_deck, cutoff_hz: 20000.0, resonance: 1.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: outgoing_deck, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });

        Ok(commands)
    }
}

pub struct EchoDrop;

impl TransitionTemplate for EchoDrop {
    fn expand(&self, params: &Value) -> Result<Vec<ScheduledCommand>, String> {
        let incoming_deck_str = params["incoming_deck"].as_str().unwrap_or("B");
        let incoming_deck = if incoming_deck_str == "A" { DeckId::A } else { DeckId::B };
        let outgoing_deck = if incoming_deck == DeckId::A { DeckId::B } else { DeckId::A };
        
        let silence_beats = params["silence_beats"].as_u64().unwrap_or(4) as u32;
        
        // This simulates an echo drop (since we don't have an echo FX yet, 
        // we'll just cut the outgoing volume exponentially and delay the incoming start)
        let mut commands = Vec::new();

        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: outgoing_deck, gain_db: -12.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });

        // "Echo" tail simulation over 2 beats
        for beat in 1..=2 {
            let gain = -12.0 - (beat as f32 * 20.0);
            commands.push(ScheduledCommand {
                command: Command::SetDeckGain { deck: outgoing_deck, gain_db: gain },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
        }

        // Pause outgoing after the "echo"
        commands.push(ScheduledCommand {
            command: Command::Pause { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(2),
            target_sample: None,
        });

        // Drop incoming after silence gap
        let drop_beat = silence_beats;
        let final_cf = if incoming_deck == DeckId::B { 1.0 } else { -1.0 };

        commands.push(ScheduledCommand {
            command: Command::SetCrossfader { position: final_cf },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(drop_beat),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: incoming_deck, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(drop_beat),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::Play { deck: incoming_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(drop_beat),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: outgoing_deck, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(drop_beat),
            target_sample: None,
        });

        Ok(commands)
    }
}
