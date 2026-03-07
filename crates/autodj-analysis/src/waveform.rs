//! Waveform peak generation for UI

use crate::profile::WaveformInfo;

pub fn analyze_waveform(samples: &[f32], file_hash: &str, output_dir: &std::path::Path) -> Result<WaveformInfo, std::io::Error> {
    let samples_per_pixel = 256;
    let channels = 2;
    
    let mut peaks = Vec::new();
    
    // We expect stereo interleaved, so each "pixel" is 256 stereo frames (512 f32s).
    // For the UI, we just need the max absolute amplitude per pixel block.
    // E.g. [max_L, max_R, max_L, max_R, ...]
    
    let chunk_size = samples_per_pixel as usize * channels as usize;
    for chunk in samples.chunks(chunk_size) {
        let mut max_l = 0.0_f32;
        let mut max_r = 0.0_f32;
        
        for ch in chunk.chunks(2) {
            let l = ch[0].abs();
            let r = if ch.len() > 1 { ch[1].abs() } else { l };
            if l > max_l { max_l = l; }
            if r > max_r { max_r = r; }
        }
        
        // Convert to u8 (0-255) to save space
        peaks.push((max_l * 255.0).min(255.0) as u8);
        peaks.push((max_r * 255.0).min(255.0) as u8);
    }
    
    // Save to bin file
    let filename = format!("{}_peaks.bin", file_hash);
    let path = output_dir.join(&filename);
    std::fs::write(&path, &peaks)?;

    Ok(WaveformInfo {
        peaks_file: filename,
        samples_per_pixel,
        channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_waveform_peak_generation() {
        let dir = tempdir().unwrap();
        // 512 stereo samples = 2 pixels
        let mut samples = vec![0.0f32; 1024]; 
        samples[0] = 0.5; // pixel 0 left Peak
        samples[1] = -1.0; // pixel 0 right peak
        samples[1022] = 0.8; // pixel 1 left peak
        samples[1023] = -0.2; // pixel 1 right peak

        let _info = analyze_waveform(&samples, "test_hash", dir.path()).unwrap();
        
        let path = dir.path().join("test_hash_peaks.bin");
        let data = std::fs::read(path).unwrap();
        
        assert_eq!(data.len(), 4); // 2 pixels * 2 channels
        assert_eq!(data[0], 127); // 0.5 * 255
        assert_eq!(data[1], 255); // 1.0 * 255
        assert_eq!(data[2], 204); // 0.8 * 255
        assert_eq!(data[3], 51); // 0.2 * 255
    }
}
