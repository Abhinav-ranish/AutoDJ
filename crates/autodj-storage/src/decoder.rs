//! FFmpeg decoder: spawns ffmpeg to decode audio to canonical PCM.

use crate::cache::CacheManager;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DecoderError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("FFmpeg not found. Install with: brew install ffmpeg")]
    FfmpegNotFound,

    #[error("FFmpeg failed with exit code {0}: {1}")]
    FfmpegFailed(i32, String),

    #[error("Cache error: {0}")]
    Cache(#[from] crate::cache::CacheError),
}

/// Decode an audio file to canonical PCM using FFmpeg.
///
/// Output: 48 kHz, stereo, f32 LE (ADJP format).
/// Returns the path to the cached PCM file.
pub fn decode_to_pcm(
    input_path: &Path,
    track_id: &str,
    cache: &CacheManager,
) -> Result<PathBuf, DecoderError> {
    // Check if ffmpeg is available
    let ffmpeg = find_ffmpeg()?;

    let output_path = cache.pcm_path(track_id);

    // Spawn ffmpeg to decode to raw f32le PCM via stdout
    let mut child = Command::new(&ffmpeg)
        .args([
            "-i",
            input_path.to_str().unwrap_or(""),
            "-f",
            "f32le",
            "-acodec",
            "pcm_f32le",
            "-ar",
            "48000",
            "-ac",
            "2",
            "-v",
            "error",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Read all PCM data from stdout
    let mut pcm_data = Vec::new();
    if let Some(ref mut stdout) = child.stdout {
        stdout.read_to_end(&mut pcm_data)?;
    }

    let status = child.wait()?;
    if !status.success() {
        let mut stderr = String::new();
        if let Some(ref mut err) = child.stderr {
            err.read_to_string(&mut stderr).ok();
        }
        return Err(DecoderError::FfmpegFailed(
            status.code().unwrap_or(-1),
            stderr,
        ));
    }

    // Write ADJP formatted file
    let total_samples = pcm_data.len() / std::mem::size_of::<f32>();
    let total_frames = total_samples / 2; // stereo
    let duration_ms = (total_frames as u64 * 1000) / 48000;

    cache.write_pcm(track_id, &pcm_data, total_frames as u64, duration_ms)?;

    Ok(output_path)
}

/// Find the ffmpeg binary.
fn find_ffmpeg() -> Result<PathBuf, DecoderError> {
    // Try common locations
    let candidates = [
        PathBuf::from("/opt/homebrew/bin/ffmpeg"),
        PathBuf::from("/usr/local/bin/ffmpeg"),
        PathBuf::from("/usr/bin/ffmpeg"),
    ];

    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    // Attempt Windows standard path
    if cfg!(windows) {
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            let win_path = PathBuf::from(program_files).join("ffmpeg").join("bin").join("ffmpeg.exe");
            if win_path.exists() {
                return Ok(win_path);
            }
        }
    }

    // Try PATH via `which` (Unix) or `where` (Windows)
    let cmd = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(cmd)
        .arg("ffmpeg")
        .output()
        .map_err(|_| DecoderError::FfmpegNotFound)?;

    if output.status.success() {
        let raw_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // `where` on Windows returns multiple lines if multiple executables exist; take the first one
        let path = raw_path.lines().next().unwrap_or("").to_string();
        
        if !path.is_empty() {
            return Ok(PathBuf::from(path));
        }
    }

    Err(DecoderError::FfmpegNotFound)
}
