//! Audio recorder: captures master output to WAV file.
//!
//! Uses a lock-free ring buffer to transfer samples from the audio
//! callback to a writer thread that flushes them to disk.

use ringbuf::{
    traits::{Consumer, Split},
    HeapRb,
};
#[cfg(test)]
use ringbuf::traits::Producer;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Ring buffer size: ~2 seconds of stereo 48 kHz audio
const RECORD_RING_SIZE: usize = 48_000 * 2 * 2;

/// Producer half that lives on the audio thread.
pub type RecordProducer = ringbuf::HeapProd<f32>;

/// Consumer half that lives on the writer thread.
pub type RecordConsumer = ringbuf::HeapCons<f32>;

/// Create a recording ring buffer pair.
/// Returns (producer for audio thread, consumer for writer thread).
pub fn create_record_ring() -> (RecordProducer, RecordConsumer) {
    let rb = HeapRb::<f32>::new(RECORD_RING_SIZE);
    rb.split()
}

/// Shared flag indicating whether recording is active.
pub type RecordingFlag = Arc<AtomicBool>;

/// Create a recording flag.
pub fn create_recording_flag() -> RecordingFlag {
    Arc::new(AtomicBool::new(false))
}

/// WAV file writer that runs on a background thread.
pub struct WavWriter {
    output_path: PathBuf,
    sample_rate: u32,
    channels: u16,
}

impl WavWriter {
    pub fn new(output_path: PathBuf, sample_rate: u32) -> Self {
        Self {
            output_path,
            sample_rate,
            channels: 2,
        }
    }

    /// Run the writer loop, draining samples from the consumer until
    /// the recording flag is cleared. Returns the final file path.
    pub fn run(
        &self,
        mut consumer: RecordConsumer,
        recording_flag: RecordingFlag,
    ) -> Result<PathBuf, io::Error> {
        let file = std::fs::File::create(&self.output_path)?;
        let mut writer = BufWriter::new(file);

        // Write placeholder WAV header (will be updated at the end)
        write_wav_header(&mut writer, self.sample_rate, self.channels, 0)?;

        let mut total_samples: u64 = 0;
        let mut buf = [0.0f32; 4096];

        while recording_flag.load(Ordering::Relaxed) {
            let n = consumer.pop_slice(&mut buf);
            if n > 0 {
                for &sample in &buf[..n] {
                    let bytes = sample.to_le_bytes();
                    writer.write_all(&bytes)?;
                }
                total_samples += n as u64;
            } else {
                // No data available, yield briefly
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }

        // Drain remaining samples after stop
        loop {
            let n = consumer.pop_slice(&mut buf);
            if n == 0 {
                break;
            }
            for &sample in &buf[..n] {
                let bytes = sample.to_le_bytes();
                writer.write_all(&bytes)?;
            }
            total_samples += n as u64;
        }

        writer.flush()?;

        // Update WAV header with actual data size
        let mut file = writer.into_inner().map_err(|e| e.into_error())?;
        let data_size = total_samples * 4; // 32-bit float = 4 bytes per sample
        file.seek(SeekFrom::Start(0))?;
        write_wav_header(&mut file, self.sample_rate, self.channels, data_size as u32)?;
        file.flush()?;

        log::info!(
            "Recording saved: {} ({} samples, {:.1}s)",
            self.output_path.display(),
            total_samples,
            total_samples as f64 / (self.sample_rate as f64 * self.channels as f64)
        );

        Ok(self.output_path.clone())
    }
}

/// Write a WAV header for 32-bit float PCM.
fn write_wav_header<W: Write>(
    writer: &mut W,
    sample_rate: u32,
    channels: u16,
    data_size: u32,
) -> io::Result<()> {
    let byte_rate = sample_rate * channels as u32 * 4; // 4 bytes per sample (f32)
    let block_align = channels * 4;
    let file_size = 36 + data_size; // Total file size minus 8

    // RIFF header
    writer.write_all(b"RIFF")?;
    writer.write_all(&file_size.to_le_bytes())?;
    writer.write_all(b"WAVE")?;

    // fmt sub-chunk
    writer.write_all(b"fmt ")?;
    writer.write_all(&16u32.to_le_bytes())?; // sub-chunk size
    writer.write_all(&3u16.to_le_bytes())?; // audio format: 3 = IEEE float
    writer.write_all(&channels.to_le_bytes())?;
    writer.write_all(&sample_rate.to_le_bytes())?;
    writer.write_all(&byte_rate.to_le_bytes())?;
    writer.write_all(&block_align.to_le_bytes())?;
    writer.write_all(&32u16.to_le_bytes())?; // bits per sample

    // data sub-chunk
    writer.write_all(b"data")?;
    writer.write_all(&data_size.to_le_bytes())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wav_header() {
        let mut buf = Vec::new();
        write_wav_header(&mut buf, 48000, 2, 0).unwrap();
        assert_eq!(buf.len(), 44); // Standard WAV header size
        assert_eq!(&buf[0..4], b"RIFF");
        assert_eq!(&buf[8..12], b"WAVE");
        assert_eq!(&buf[12..16], b"fmt ");
        assert_eq!(&buf[36..40], b"data");
        // Format should be IEEE float (3)
        assert_eq!(buf[20], 3);
        assert_eq!(buf[21], 0);
    }

    #[test]
    fn test_record_ring_creation() {
        let (mut prod, mut cons) = create_record_ring();
        // Push and pop some samples
        prod.try_push(0.5).unwrap();
        prod.try_push(-0.5).unwrap();
        assert_eq!(cons.try_pop(), Some(0.5));
        assert_eq!(cons.try_pop(), Some(-0.5));
        assert_eq!(cons.try_pop(), None);
    }
}
