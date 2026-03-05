use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use std::sync::mpsc;

/// Recorded audio data
pub struct RecordedAudio {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl RecordedAudio {
    #[allow(dead_code)]
    pub fn to_wav(&self) -> Result<Vec<u8>> {
        let spec = WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = WavWriter::new(&mut cursor, spec).context("Failed to create WAV writer")?;
            for &sample in &self.samples {
                writer.write_sample(sample).context("Failed to write sample")?;
            }
            writer.finalize().context("Failed to finalize WAV")?;
        }

        Ok(cursor.into_inner())
    }

    pub fn has_audio(&self) -> bool {
        if self.samples.len() < 1000 {
            return false;
        }
        let sum_squares: f64 = self.samples.iter().map(|&s| (s as f64).powi(2)).sum();
        let rms = (sum_squares / self.samples.len() as f64).sqrt();
        rms > 100.0
    }
}

pub struct AudioRecorder {
    stop_tx: Option<mpsc::Sender<()>>,
    result_rx: Option<mpsc::Receiver<Result<RecordedAudio>>>,
}

impl AudioRecorder {
    pub fn new() -> Self {
        Self {
            stop_tx: None,
            result_rx: None,
        }
    }

    pub fn start_recording(&mut self) -> Result<()> {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (result_tx, result_rx) = mpsc::channel::<Result<RecordedAudio>>();

        std::thread::spawn(move || {
            let res = Self::record_loop(stop_rx);
            let _ = result_tx.send(res);
        });

        self.stop_tx = Some(stop_tx);
        self.result_rx = Some(result_rx);

        Ok(())
    }

    pub fn stop(&mut self) -> Result<RecordedAudio> {
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
        if let Some(rx) = self.result_rx.take() {
            rx.recv().unwrap_or_else(|_| Err(anyhow::anyhow!("Recording thread crashed")))
        } else {
            Err(anyhow::anyhow!("No active recording"))
        }
    }

    fn record_loop(stop_rx: mpsc::Receiver<()>) -> Result<RecordedAudio> {
        let host = cpal::default_host();
        let device = host.default_input_device().context("No input device available")?;
        let supported_config = device.default_input_config().context("Failed to get default input config")?;

        let sample_format = supported_config.sample_format();
        let config: StreamConfig = supported_config.into();

        let sample_rate = config.sample_rate.0;
        let channels = config.channels;

        let samples: Arc<Mutex<Vec<i16>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = Arc::clone(&samples);

        let err_fn = |err| {
            log::error!("Audio stream error: {}", err);
        };

        let stream = match sample_format {
            SampleFormat::I16 => {
                let samples = samples_clone;
                device.build_input_stream(
                    &config,
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
                device.build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buffer) = samples.lock() {
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
                device.build_input_stream(
                    &config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        if let Ok(mut buffer) = samples.lock() {
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
                anyhow::bail!("Unsupported sample format: {:?}", sample_format);
            }
        };

        stream.play().context("Failed to start audio stream")?;

        // Block until we receive a stop signal
        let _ = stop_rx.recv();
        drop(stream); // Stop the stream

        let final_samples = samples.lock().unwrap().clone();

        Ok(RecordedAudio {
            samples: final_samples,
            sample_rate,
            channels,
        })
    }
}
