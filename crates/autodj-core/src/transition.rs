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
            "drop_swap" => DropSwap.expand(params),
            "loop_roll" => LoopRoll.expand(params),
            "bass_kill_swap" => BassKillSwap.expand(params),
            "break_blend" => BreakBlend.expand(params),
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
        let bass_swap_bar = params["bass_swap_bar"].as_u64().unwrap_or(8) as u32;

        let mut commands = Vec::new();

        // 1. Initial State: Incoming deck plays silently
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

// ─── Phase 19: New DJ-Style Transition Templates ────────────────

/// DropSwap: Cut on a drop with pre-drop tension build (HP sweep outgoing,
/// hard cut to incoming at the drop point).
pub struct DropSwap;

impl TransitionTemplate for DropSwap {
    fn expand(&self, params: &Value) -> Result<Vec<ScheduledCommand>, String> {
        let buildup_bars = params["buildup_bars"].as_u64().unwrap_or(8) as u32;
        let incoming_deck_str = params["incoming_deck"].as_str().unwrap_or("B");
        let incoming_deck = if incoming_deck_str == "A" { DeckId::A } else { DeckId::B };
        let outgoing_deck = if incoming_deck == DeckId::A { DeckId::B } else { DeckId::A };

        let mut commands = Vec::new();
        let buildup_beats = buildup_bars * 4;

        // Pre-roll incoming silently at the start
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

        // Build tension: HP sweep outgoing from 200 → 4000 Hz over buildup
        for beat in 0..buildup_beats {
            let progress = beat as f32 / buildup_beats as f32;
            let log_start = 200.0_f32.ln();
            let log_end = 4000.0_f32.ln();
            let cutoff = (log_start + progress * (log_end - log_start)).exp();

            commands.push(ScheduledCommand {
                command: Command::SetFilter { deck: outgoing_deck, cutoff_hz: cutoff, resonance: 1.2 },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
        }

        // At the drop: hard cut — kill outgoing, full incoming
        let drop_beat = buildup_beats;
        let final_cf = if incoming_deck == DeckId::B { 1.0 } else { -1.0 };

        commands.push(ScheduledCommand {
            command: Command::Pause { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(drop_beat),
            target_sample: None,
        });
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
        // Reset outgoing filter
        commands.push(ScheduledCommand {
            command: Command::SetFilter { deck: outgoing_deck, cutoff_hz: 20000.0, resonance: 1.0 },
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

/// LoopRoll: 4→2→1 bar loop roll on outgoing, then cut to incoming.
pub struct LoopRoll;

impl TransitionTemplate for LoopRoll {
    fn expand(&self, params: &Value) -> Result<Vec<ScheduledCommand>, String> {
        let incoming_deck_str = params["incoming_deck"].as_str().unwrap_or("B");
        let incoming_deck = if incoming_deck_str == "A" { DeckId::A } else { DeckId::B };
        let outgoing_deck = if incoming_deck == DeckId::A { DeckId::B } else { DeckId::A };

        let mut commands = Vec::new();

        // Pre-roll incoming silently
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

        // Calculate the current beat to set loops relative to it.
        // We use AfterBars to schedule loops. The engine will resolve the absolute position.

        // Step 1: 4-bar loop on outgoing (starts at bar 0, held for 4 bars = 16 beats)
        commands.push(ScheduledCommand {
            command: Command::SetLoop { deck: outgoing_deck, start_beat: 0, length_bars: 4 },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });

        // Step 2: After 4 bars (16 beats), switch to 2-bar loop
        commands.push(ScheduledCommand {
            command: Command::SetLoop { deck: outgoing_deck, start_beat: 0, length_bars: 2 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(4),
            target_sample: None,
        });

        // Step 3: After 6 bars (24 beats), switch to 1-bar loop
        commands.push(ScheduledCommand {
            command: Command::SetLoop { deck: outgoing_deck, start_beat: 0, length_bars: 1 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(6),
            target_sample: None,
        });

        // Step 4: After 7 bars (28 beats), clear loop and cut to incoming
        let cut_beat = 28;
        let final_cf = if incoming_deck == DeckId::B { 1.0 } else { -1.0 };

        commands.push(ScheduledCommand {
            command: Command::ClearLoop { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(cut_beat),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::Pause { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(cut_beat),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetCrossfader { position: final_cf },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(cut_beat),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: incoming_deck, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(cut_beat),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: outgoing_deck, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBeats(cut_beat),
            target_sample: None,
        });

        Ok(commands)
    }
}

/// BassKillSwap: Kill outgoing bass early while blending, then full swap.
/// Classic DJ technique: swap bass ownership before volume swap.
pub struct BassKillSwap;

impl TransitionTemplate for BassKillSwap {
    fn expand(&self, params: &Value) -> Result<Vec<ScheduledCommand>, String> {
        let duration_bars = params["duration_bars"].as_u64().unwrap_or(16) as u32;
        let bass_swap_bar = params["bass_swap_bar"].as_u64().unwrap_or(4) as u32;
        let incoming_deck_str = params["incoming_deck"].as_str().unwrap_or("B");
        let incoming_deck = if incoming_deck_str == "A" { DeckId::A } else { DeckId::B };
        let outgoing_deck = if incoming_deck == DeckId::A { DeckId::B } else { DeckId::A };

        let mut commands = Vec::new();

        // Start incoming silently with bass killed
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: incoming_deck, gain_db: -96.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetEq { deck: incoming_deck, band: EqBand::Low, gain_db: -96.0 },
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

        // Bring incoming volume up over bass_swap_bar bars (mids/highs only)
        let ramp_beats = bass_swap_bar * 4;
        for beat in 0..=ramp_beats {
            let progress = beat as f32 / ramp_beats as f32;
            let in_gain = -96.0 + progress * 96.0;
            commands.push(ScheduledCommand {
                command: Command::SetDeckGain { deck: incoming_deck, gain_db: in_gain },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
        }

        // Bass swap: kill outgoing bass, restore incoming bass
        commands.push(ScheduledCommand {
            command: Command::SetEq { deck: outgoing_deck, band: EqBand::Low, gain_db: -96.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(bass_swap_bar),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetEq { deck: incoming_deck, band: EqBand::Low, gain_db: 0.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(bass_swap_bar),
            target_sample: None,
        });

        // Fade outgoing down over remaining bars
        let remaining_beats = (duration_bars - bass_swap_bar) * 4;
        let fade_start_beat = bass_swap_bar * 4;
        for beat in 0..=remaining_beats {
            let progress = beat as f32 / remaining_beats as f32;
            let out_gain = -(progress * 96.0);
            commands.push(ScheduledCommand {
                command: Command::SetDeckGain { deck: outgoing_deck, gain_db: out_gain },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(fade_start_beat + beat),
                target_sample: None,
            });
        }

        // Crossfader to incoming side
        let final_cf = if incoming_deck == DeckId::B { 1.0 } else { -1.0 };
        commands.push(ScheduledCommand {
            command: Command::SetCrossfader { position: final_cf },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });

        // Finalize: pause outgoing, reset EQ
        commands.push(ScheduledCommand {
            command: Command::Pause { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetEq { deck: outgoing_deck, band: EqBand::Low, gain_db: 0.0 },
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

/// BreakBlend: Blend during a breakdown section while avoiding vocal clash.
/// Applies a gentle HP filter to avoid bass conflict and fades smoothly.
pub struct BreakBlend;

impl TransitionTemplate for BreakBlend {
    fn expand(&self, params: &Value) -> Result<Vec<ScheduledCommand>, String> {
        let duration_bars = params["duration_bars"].as_u64().unwrap_or(16) as u32;
        let incoming_deck_str = params["incoming_deck"].as_str().unwrap_or("B");
        let incoming_deck = if incoming_deck_str == "A" { DeckId::A } else { DeckId::B };
        let outgoing_deck = if incoming_deck == DeckId::A { DeckId::B } else { DeckId::A };

        let mut commands = Vec::new();
        let total_beats = duration_bars * 4;

        // Start incoming silently with HP filter engaged (no bass clash)
        commands.push(ScheduledCommand {
            command: Command::SetDeckGain { deck: incoming_deck, gain_db: -96.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::NextBar,
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetFilter { deck: incoming_deck, cutoff_hz: 400.0, resonance: 0.8 },
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

        // Gradual blend: incoming volume up, outgoing volume down
        // Incoming filter opens gradually, outgoing gets gentle HP sweep
        for beat in 0..=total_beats {
            let progress = beat as f32 / total_beats as f32;

            // Incoming: volume ramps up, filter opens
            let in_gain = -96.0 + progress * 96.0;
            let in_cutoff_log = 400.0_f32.ln() + progress * (20000.0_f32.ln() - 400.0_f32.ln());
            let in_cutoff = in_cutoff_log.exp();

            // Outgoing: gentle volume fade + mid/high EQ reduction in second half
            let out_gain = -(progress * progress * 24.0); // quadratic fade, gentler start
            let out_mid_db = if progress > 0.5 { -((progress - 0.5) * 2.0 * 12.0) } else { 0.0 };

            commands.push(ScheduledCommand {
                command: Command::SetDeckGain { deck: incoming_deck, gain_db: in_gain },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
            commands.push(ScheduledCommand {
                command: Command::SetFilter { deck: incoming_deck, cutoff_hz: in_cutoff, resonance: 0.8 },
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
                command: Command::SetEq { deck: outgoing_deck, band: EqBand::Mid, gain_db: out_mid_db },
                source: CommandSource::System,
                schedule: SchedulePoint::AfterBeats(beat),
                target_sample: None,
            });
        }

        // Finalize
        let final_cf = if incoming_deck == DeckId::B { 1.0 } else { -1.0 };
        commands.push(ScheduledCommand {
            command: Command::Pause { deck: outgoing_deck },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetCrossfader { position: final_cf },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });
        // Reset filters, EQ, gains
        commands.push(ScheduledCommand {
            command: Command::SetFilter { deck: outgoing_deck, cutoff_hz: 20000.0, resonance: 1.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetFilter { deck: incoming_deck, cutoff_hz: 20000.0, resonance: 1.0 },
            source: CommandSource::System,
            schedule: SchedulePoint::AfterBars(duration_bars),
            target_sample: None,
        });
        commands.push(ScheduledCommand {
            command: Command::SetEq { deck: outgoing_deck, band: EqBand::Mid, gain_db: 0.0 },
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
