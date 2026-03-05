use anyhow::{Context, Result};
use rubato::{FftFixedIn, Resampler};

/// Resamples audio from the microphone's native rate to 16kHz mono,
/// which is what Whisper expects.
pub struct AudioResampler {
    resampler: FftFixedIn<f32>,
    input_channels: usize,
}

impl AudioResampler {
    /// Create a new resampler from the given input sample rate and channel count
    /// to 16kHz mono output.
    pub fn new(input_sample_rate: u32, input_channels: u16) -> Result<Self> {
        let resampler = FftFixedIn::<f32>::new(
            input_sample_rate as usize,
            16000,
            1024, // chunk size
            2,    // sub-chunks
            input_channels as usize,
        )
        .context("Failed to create audio resampler")?;

        Ok(Self {
            resampler,
            input_channels: input_channels as usize,
        })
    }

    /// Resample i16 audio to f32 mono at 16kHz.
    /// Input is interleaved multi-channel i16 samples at the original rate.
    pub fn resample_i16_to_f32_16khz(&mut self, samples: &[i16]) -> Result<Vec<f32>> {
        // Convert i16 to f32 and de-interleave into per-channel buffers
        let total_frames = samples.len() / self.input_channels;
        let mut channels: Vec<Vec<f32>> = (0..self.input_channels)
            .map(|_| Vec::with_capacity(total_frames))
            .collect();

        for frame in samples.chunks_exact(self.input_channels) {
            for (ch, &sample) in frame.iter().enumerate() {
                channels[ch].push(sample as f32 / 32768.0);
            }
        }

        // Process through resampler in chunks
        let chunk_size = self.resampler.input_frames_next();
        let mut output_all: Vec<f32> = Vec::new();

        let mut pos = 0;
        while pos + chunk_size <= total_frames {
            let chunk: Vec<Vec<f32>> = channels
                .iter()
                .map(|ch| ch[pos..pos + chunk_size].to_vec())
                .collect();

            let resampled = self
                .resampler
                .process(&chunk, None)
                .context("Resampler processing failed")?;

            // Mix down to mono by averaging channels
            if let Some(first_ch) = resampled.first() {
                if self.input_channels == 1 {
                    output_all.extend_from_slice(first_ch);
                } else {
                    for i in 0..first_ch.len() {
                        let sum: f32 = resampled.iter().map(|ch| ch[i]).sum();
                        output_all.push(sum / self.input_channels as f32);
                    }
                }
            }

            pos += chunk_size;
        }

        // Handle remaining samples by zero-padding
        let remaining = total_frames - pos;
        if remaining > 0 {
            let mut chunk: Vec<Vec<f32>> = channels
                .iter()
                .map(|ch| {
                    let mut v = ch[pos..].to_vec();
                    v.resize(chunk_size, 0.0);
                    v
                })
                .collect();

            let _ = chunk.as_mut_slice(); // keep borrow checker happy
            let resampled = self
                .resampler
                .process(&chunk, None)
                .context("Resampler processing failed (tail)")?;

            if let Some(first_ch) = resampled.first() {
                if self.input_channels == 1 {
                    output_all.extend_from_slice(first_ch);
                } else {
                    for i in 0..first_ch.len() {
                        let sum: f32 = resampled.iter().map(|ch| ch[i]).sum();
                        output_all.push(sum / self.input_channels as f32);
                    }
                }
            }
        }

        Ok(output_all)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resampler_creation() {
        let resampler = AudioResampler::new(48000, 1);
        assert!(resampler.is_ok());
    }

    #[test]
    fn test_resampler_creation_stereo() {
        let resampler = AudioResampler::new(44100, 2);
        assert!(resampler.is_ok());
    }

    #[test]
    fn test_resample_mono_48k() {
        let mut resampler = AudioResampler::new(48000, 1).unwrap();
        // Generate 1 second of 48kHz mono silence
        let samples = vec![0i16; 48000];
        let result = resampler.resample_i16_to_f32_16khz(&samples);
        assert!(result.is_ok());
        let output = result.unwrap();
        // Should be roughly 16000 samples (16kHz * 1 second), allow some tolerance
        assert!(output.len() > 15000 && output.len() < 17000);
    }

    #[test]
    fn test_resample_stereo_44100() {
        let mut resampler = AudioResampler::new(44100, 2).unwrap();
        // Generate 1 second of 44.1kHz stereo silence (interleaved)
        let samples = vec![0i16; 44100 * 2];
        let result = resampler.resample_i16_to_f32_16khz(&samples);
        assert!(result.is_ok());
        let output = result.unwrap();
        // Should be roughly 16000 samples mono
        assert!(output.len() > 15000 && output.len() < 17000);
    }
}
