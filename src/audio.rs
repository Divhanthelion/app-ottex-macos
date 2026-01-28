use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::{Arc, Mutex};

/// Audio recorder that captures microphone input
pub struct AudioRecorder {
    device: Device,
    config: StreamConfig,
    sample_format: SampleFormat,
}

/// Recorded audio data
pub struct RecordedAudio {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl AudioRecorder {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("No input device available")?;

        log::info!("Using input device: {}", device.name().unwrap_or_default());

        let supported_config = device
            .default_input_config()
            .context("Failed to get default input config")?;

        log::info!(
            "Default input config: {} Hz, {} channels, {:?}",
            supported_config.sample_rate().0,
            supported_config.channels(),
            supported_config.sample_format()
        );

        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        Ok(Self {
            device,
            config,
            sample_format,
        })
    }

    /// Start recording and return a handle to control the recording
    pub fn start_recording(&self) -> Result<RecordingHandle> {
        let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = Arc::clone(&samples);

        let err_fn = |err| {
            log::error!("Audio stream error: {}", err);
        };

        let stream = match self.sample_format {
            SampleFormat::I16 => {
                let samples = samples_clone;
                self.device.build_input_stream(
                    &self.config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buffer) = samples.lock() {
                            buffer.extend_from_slice(data);
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            SampleFormat::F32 => {
                let samples = samples_clone;
                self.device.build_input_stream(
                    &self.config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buffer) = samples.lock() {
                            // Convert f32 samples to i16
                            for &sample in data {
                                let clamped = sample.clamp(-1.0, 1.0);
                                let converted = (clamped * i16::MAX as f32) as i16;
                                buffer.push(converted);
                            }
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            SampleFormat::U16 => {
                let samples = samples_clone;
                self.device.build_input_stream(
                    &self.config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buffer) = samples.lock() {
                            // Convert u16 samples to i16
                            for &sample in data {
                                let converted = (sample as i32 - 32768) as i16;
                                buffer.push(converted);
                            }
                        }
                    },
                    err_fn,
                    None,
                )?
            }
            _ => {
                anyhow::bail!("Unsupported sample format: {:?}", self.sample_format);
            }
        };

        stream.play().context("Failed to start audio stream")?;
        log::info!("Recording started");

        Ok(RecordingHandle {
            stream,
            samples,
            sample_rate: self.config.sample_rate.0,
            channels: self.config.channels,
        })
    }
}

/// Handle to an active recording
pub struct RecordingHandle {
    #[allow(dead_code)]
    stream: Stream,
    samples: Arc<Mutex<Vec<i16>>>,
    sample_rate: u32,
    channels: u16,
}

impl RecordingHandle {
    /// Stop recording and return the recorded audio
    pub fn stop(self) -> Result<RecordedAudio> {
        // Stream is stopped when dropped
        drop(self.stream);
        log::info!("Recording stopped");

        let samples = self
            .samples
            .lock()
            .map_err(|_| anyhow::anyhow!("Failed to lock samples buffer"))?
            .clone();

        log::info!(
            "Recorded {} samples ({:.2} seconds)",
            samples.len(),
            samples.len() as f32 / self.sample_rate as f32 / self.channels as f32
        );

        Ok(RecordedAudio {
            samples,
            sample_rate: self.sample_rate,
            channels: self.channels,
        })
    }
}

impl RecordedAudio {
    /// Encode the recorded audio as WAV and return the bytes
    pub fn to_wav(&self) -> Result<Vec<u8>> {
        let spec = WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = WavWriter::new(&mut cursor, spec)
                .context("Failed to create WAV writer")?;

            for &sample in &self.samples {
                writer.write_sample(sample).context("Failed to write sample")?;
            }

            writer.finalize().context("Failed to finalize WAV")?;
        }

        let wav_bytes = cursor.into_inner();
        log::info!("Encoded WAV: {} bytes", wav_bytes.len());

        Ok(wav_bytes)
    }

    /// Check if the recording has meaningful audio content
    pub fn has_audio(&self) -> bool {
        // Check if there are enough samples and they're not all silent
        if self.samples.len() < 1000 {
            return false;
        }

        // Calculate RMS to check for silence
        let sum_squares: f64 = self.samples.iter().map(|&s| (s as f64).powi(2)).sum();
        let rms = (sum_squares / self.samples.len() as f64).sqrt();

        // Threshold for "not silent" - adjust as needed
        rms > 100.0
    }
}
