//! Track buffer: wraps memory-mapped PCM data for zero-copy reading.

use memmap2::Mmap;
use std::fs::File;
use std::path::Path;
use thiserror::Error;

/// Magic bytes for the ADJP format header.
const ADJP_MAGIC: &[u8; 4] = b"ADJP";

/// Current version of the ADJP format.
const ADJP_VERSION: u32 = 1;

/// Size of the ADJP header in bytes.
pub const ADJP_HEADER_SIZE: usize = 32;

/// Errors that can occur when working with track buffers.
#[derive(Error, Debug)]
pub enum BufferError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid ADJP magic bytes")]
    InvalidMagic,

    #[error("Unsupported ADJP version: {0}")]
    UnsupportedVersion(u32),

    #[error("File too small for ADJP header")]
    FileTooSmall,
}

/// Header of an ADJP (AutoDJ PCM) file.
#[derive(Debug, Clone, Copy)]
pub struct AdjpHeader {
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: u16,
    pub total_samples: u64,
    pub duration_ms: u64,
}

impl AdjpHeader {
    /// Parse an ADJP header from a byte slice.
    pub fn from_bytes(data: &[u8]) -> Result<Self, BufferError> {
        if data.len() < ADJP_HEADER_SIZE {
            return Err(BufferError::FileTooSmall);
        }

        if &data[0..4] != ADJP_MAGIC {
            return Err(BufferError::InvalidMagic);
        }

        let version = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if version != ADJP_VERSION {
            return Err(BufferError::UnsupportedVersion(version));
        }

        Ok(Self {
            sample_rate: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            channels: u16::from_le_bytes([data[12], data[13]]),
            bit_depth: u16::from_le_bytes([data[14], data[15]]),
            total_samples: u64::from_le_bytes([
                data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
            ]),
            duration_ms: u64::from_le_bytes([
                data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
            ]),
        })
    }

    /// Serialize the header to bytes.
    pub fn to_bytes(&self) -> [u8; ADJP_HEADER_SIZE] {
        let mut buf = [0u8; ADJP_HEADER_SIZE];
        buf[0..4].copy_from_slice(ADJP_MAGIC);
        buf[4..8].copy_from_slice(&ADJP_VERSION.to_le_bytes());
        buf[8..12].copy_from_slice(&self.sample_rate.to_le_bytes());
        buf[12..14].copy_from_slice(&self.channels.to_le_bytes());
        buf[14..16].copy_from_slice(&self.bit_depth.to_le_bytes());
        buf[16..24].copy_from_slice(&self.total_samples.to_le_bytes());
        buf[24..32].copy_from_slice(&self.duration_ms.to_le_bytes());
        buf
    }
}

/// A memory-mapped track buffer for zero-copy audio reading.
///
/// The audio callback reads directly from the mapped region —
/// no allocation, no I/O at read time.
pub struct TrackBuffer {
    _mmap: Mmap,
    /// Pointer to the start of sample data (after header).
    samples_ptr: *const f32,
    /// Total number of f32 samples (channels * frames).
    total_samples: usize,
    /// Header metadata.
    pub header: AdjpHeader,
}

// SAFETY: TrackBuffer holds a read-only memory map. The pointer is derived
// from it and only used for reading. Safe to send across threads.
unsafe impl Send for TrackBuffer {}
unsafe impl Sync for TrackBuffer {}

impl TrackBuffer {
    /// Open and memory-map an ADJP PCM file.
    pub fn open(path: &Path) -> Result<Self, BufferError> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        let header = AdjpHeader::from_bytes(&mmap)?;

        let sample_data = &mmap[ADJP_HEADER_SIZE..];
        let total_samples = sample_data.len() / std::mem::size_of::<f32>();
        let samples_ptr = sample_data.as_ptr() as *const f32;

        Ok(Self {
            _mmap: mmap,
            samples_ptr,
            total_samples,
            header,
        })
    }

    /// Get a sample at the given index (interleaved stereo).
    ///
    /// Returns 0.0 if the index is out of bounds (safe for audio callback).
    #[inline(always)]
    pub fn get_sample(&self, index: usize) -> f32 {
        if index < self.total_samples {
            // SAFETY: index is bounds-checked above, pointer is valid for the
            // lifetime of the mmap.
            unsafe { *self.samples_ptr.add(index) }
        } else {
            0.0
        }
    }

    /// Get a stereo frame (left, right) at the given frame index.
    ///
    /// Returns (0.0, 0.0) if out of bounds.
    #[inline(always)]
    pub fn get_frame(&self, frame: usize) -> (f32, f32) {
        let idx = frame * 2; // stereo interleaved
        (self.get_sample(idx), self.get_sample(idx + 1))
    }

    /// Total number of stereo frames.
    pub fn total_frames(&self) -> usize {
        self.total_samples / 2
    }

    /// Duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.header.duration_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_adjp(samples: &[f32]) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        let header = AdjpHeader {
            sample_rate: 48000,
            channels: 2,
            bit_depth: 32,
            total_samples: (samples.len() / 2) as u64,
            duration_ms: ((samples.len() / 2) as u64 * 1000) / 48000,
        };
        file.write_all(&header.to_bytes()).unwrap();
        for &s in samples {
            file.write_all(&s.to_le_bytes()).unwrap();
        }
        file.flush().unwrap();
        file
    }

    #[test]
    fn test_header_roundtrip() {
        let header = AdjpHeader {
            sample_rate: 48000,
            channels: 2,
            bit_depth: 32,
            total_samples: 1000,
            duration_ms: 5000,
        };
        let bytes = header.to_bytes();
        let parsed = AdjpHeader::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.sample_rate, 48000);
        assert_eq!(parsed.channels, 2);
        assert_eq!(parsed.total_samples, 1000);
        assert_eq!(parsed.duration_ms, 5000);
    }

    #[test]
    fn test_track_buffer_read() {
        let samples: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let file = create_test_adjp(&samples);

        let buf = TrackBuffer::open(file.path()).unwrap();
        assert_eq!(buf.total_frames(), 50);

        let (l, r) = buf.get_frame(0);
        assert!((l - 0.0).abs() < 1e-6);
        assert!((r - 0.01).abs() < 1e-6);
    }

    #[test]
    fn test_out_of_bounds_returns_zero() {
        let samples: Vec<f32> = vec![0.5, -0.5, 1.0, -1.0];
        let file = create_test_adjp(&samples);

        let buf = TrackBuffer::open(file.path()).unwrap();
        let (l, r) = buf.get_frame(999);
        assert_eq!(l, 0.0);
        assert_eq!(r, 0.0);
    }
}
