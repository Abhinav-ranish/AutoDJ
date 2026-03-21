//! autodj-core: Real-time audio engine for Auto-DJ
//!
//! This crate contains the audio engine, deck model, mixer,
//! command system, and transport clock. All code in the audio
//! callback path is designed to be allocation-free and lock-free.

pub mod buffer;
pub mod command;
pub mod deck;
pub mod delay;
pub mod engine;
pub mod eq;
pub mod filter;
pub mod mixer;
pub mod recorder;
pub mod reverb;
pub mod transition;
pub mod transport;
