//! File cache management: PCM files, hashing, directory structure.

use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// ADJP header size in bytes.
const ADJP_HEADER_SIZE: usize = 32;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Manages the file cache directory structure.
pub struct CacheManager {
    base_dir: PathBuf,
}

impl CacheManager {
    /// Create a cache manager, ensuring directories exist.
    pub fn new(base_dir: &Path) -> Result<Self, CacheError> {
        let mgr = Self {
            base_dir: base_dir.to_path_buf(),
        };
        fs::create_dir_all(mgr.pcm_dir())?;
        fs::create_dir_all(mgr.peaks_dir())?;
        fs::create_dir_all(mgr.originals_dir())?;
        Ok(mgr)
    }

    /// Path to the PCM cache directory.
    pub fn pcm_dir(&self) -> PathBuf {
        self.base_dir.join("cache").join("pcm")
    }

    /// Path to the waveform peaks directory.
    pub fn peaks_dir(&self) -> PathBuf {
        self.base_dir.join("cache").join("peaks")
    }

    /// Path to the originals directory.
    pub fn originals_dir(&self) -> PathBuf {
        self.base_dir.join("originals")
    }

    /// Get the PCM cache file path for a track.
    pub fn pcm_path(&self, track_id: &str) -> PathBuf {
        self.pcm_dir().join(format!("{track_id}.pcm"))
    }

    /// Check if a PCM cache file exists for a track.
    pub fn pcm_exists(&self, track_id: &str) -> bool {
        self.pcm_path(track_id).exists()
    }

    /// Write PCM data in ADJP format.
    pub fn write_pcm(
        &self,
        track_id: &str,
        pcm_data: &[u8],
        total_frames: u64,
        duration_ms: u64,
    ) -> Result<PathBuf, CacheError> {
        let path = self.pcm_path(track_id);

        let mut file = fs::File::create(&path)?;

        // Write ADJP header
        let mut header = [0u8; ADJP_HEADER_SIZE];
        header[0..4].copy_from_slice(b"ADJP");
        header[4..8].copy_from_slice(&1u32.to_le_bytes()); // version
        header[8..12].copy_from_slice(&48000u32.to_le_bytes()); // sample rate
        header[12..14].copy_from_slice(&2u16.to_le_bytes()); // channels
        header[14..16].copy_from_slice(&32u16.to_le_bytes()); // bit depth
        header[16..24].copy_from_slice(&total_frames.to_le_bytes());
        header[24..32].copy_from_slice(&duration_ms.to_le_bytes());

        file.write_all(&header)?;
        file.write_all(pcm_data)?;
        file.flush()?;

        Ok(path)
    }

    /// Copy an original file to the originals directory.
    /// Returns the destination path.
    pub fn copy_original(&self, source: &Path, track_id: &str) -> Result<PathBuf, CacheError> {
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");
        let dest = self.originals_dir().join(format!("{track_id}.{ext}"));
        fs::copy(source, &dest)?;
        Ok(dest)
    }

    /// Base directory path.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Clear all files in the cache and originals directories.
    pub fn clear_all(&self) -> Result<(), CacheError> {
        let _ = fs::remove_dir_all(self.pcm_dir());
        let _ = fs::remove_dir_all(self.peaks_dir());
        let _ = fs::remove_dir_all(self.originals_dir());
        fs::create_dir_all(self.pcm_dir())?;
        fs::create_dir_all(self.peaks_dir())?;
        fs::create_dir_all(self.originals_dir())?;
        Ok(())
    }
}

/// Compute a fast hash of a file (first 1 MB + last 1 MB + file size).
pub fn hash_file(path: &Path) -> Result<String, CacheError> {
    let mut file = fs::File::open(path)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    let mut hasher = Sha256::new();

    // Hash the file size
    hasher.update(file_size.to_le_bytes());

    // Hash first 1 MB
    let mut buf = vec![0u8; 1024 * 1024.min(file_size as usize)];
    let n = file.read(&mut buf)?;
    hasher.update(&buf[..n]);

    // Hash last 1 MB (if file is large enough)
    if file_size > 1024 * 1024 {
        use std::io::Seek;
        let offset = file_size.saturating_sub(1024 * 1024);
        file.seek(std::io::SeekFrom::Start(offset))?;
        let n = file.read(&mut buf)?;
        hasher.update(&buf[..n]);
    }

    let hash = hasher.finalize();
    Ok(format!("sha256:{:x}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_cache_manager_creates_dirs() {
        let tmp = TempDir::new().unwrap();
        let cache = CacheManager::new(tmp.path()).unwrap();
        assert!(cache.pcm_dir().exists());
        assert!(cache.peaks_dir().exists());
        assert!(cache.originals_dir().exists());
    }

    #[test]
    fn test_write_and_check_pcm() {
        let tmp = TempDir::new().unwrap();
        let cache = CacheManager::new(tmp.path()).unwrap();

        let pcm_data = vec![0u8; 1024];
        cache.write_pcm("trk_test", &pcm_data, 128, 1000).unwrap();
        assert!(cache.pcm_exists("trk_test"));
    }

    #[test]
    fn test_hash_file() {
        let tmp = TempDir::new().unwrap();
        let file_path = tmp.path().join("test.mp3");
        let mut f = fs::File::create(&file_path).unwrap();
        f.write_all(b"test audio data content here").unwrap();

        let hash = hash_file(&file_path).unwrap();
        assert!(hash.starts_with("sha256:"));

        // Same file should produce same hash
        let hash2 = hash_file(&file_path).unwrap();
        assert_eq!(hash, hash2);
    }
}
