//! autodj-analysis: Track analysis (stub for Milestone D).
//!
//! This crate will contain BPM detection, key detection, beat tracking,
//! energy curves, section segmentation, and waveform peak generation.

pub mod beats;
pub mod bpm;
pub mod energy;
pub mod key;
pub mod loudness;
pub mod perceived_tempo;
pub mod pipeline;
pub mod profile;
pub mod rhythm;
pub mod sections;
pub mod spectral;
pub mod vocal_detection;
pub mod waveform;

pub use pipeline::analyze_track;
pub use profile::*;
