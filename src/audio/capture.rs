use crate::core::error::{JarvisError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamConfig;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error};

pub struct CaptureStream(pub cpal::Stream);
unsafe impl Send for CaptureStream {}
unsafe impl Sync for CaptureStream {}

pub struct AudioCapture {
    sample_rate: u32,
    channels: u16,
}

impl Default for AudioCapture {
    fn default() -> Self {
        Self::new(16000, 1)
    }
}

impl AudioCapture {
    pub fn new(sample_rate: u32, channels: u16) -> Self {
        Self {
            sample_rate,
            channels,
        }
    }

    /// Starts capturing audio and yields chunks of 16-bit PCM samples via an async channel.
    pub fn start_stream(
        &self,
        chunk_size: usize,
    ) -> Result<(CaptureStream, mpsc::Receiver<Vec<i16>>)> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| JarvisError::Audio("No default audio input device found.".into()))?;

        let device_name = device
            .description()
            .map(|d| d.name().to_string())
            .unwrap_or_else(|_| device.to_string());
        debug!("Using audio input device: {device_name}");

        let config = StreamConfig {
            channels: self.channels,
            sample_rate: self.sample_rate,
            buffer_size: cpal::BufferSize::Fixed(chunk_size as u32),
        };

        let (tx, rx) = mpsc::channel::<Vec<i16>>(100);
        let tx = Arc::new(tx);
        let err_fn = |err| error!("Audio input stream error: {err}");

        let tx_clone = tx.clone();
        let stream = device
            .build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    let _ = tx_clone.try_send(data.to_vec());
                },
                err_fn,
                None,
            )
            .or_else(|_| {
                // Fallback to float32 input format if int16 is not supported natively by the hardware
                let tx_f32 = tx.clone();
                device.build_input_stream(
                    config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let i16_data: Vec<i16> = data
                            .iter()
                            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                            .collect();
                        let _ = tx_f32.try_send(i16_data);
                    },
                    err_fn,
                    None,
                )
            })
            .map_err(|e| JarvisError::Audio(format!("Failed to build audio input stream: {e}")))?;

        stream
            .play()
            .map_err(|e| JarvisError::Audio(format!("Failed to start audio input stream: {e}")))?;

        Ok((CaptureStream(stream), rx))
    }

    /// Computes root-mean-square energy of an audio buffer
    pub fn compute_rms(samples: &[i16]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
        ((sum_sq / samples.len() as f64).sqrt()) as f32
    }

    /// Soft AGC: Amplifies low-energy voice input while preventing clipping
    pub fn apply_soft_agc(samples: &mut [i16]) {
        let rms = Self::compute_rms(samples);
        if rms > 40.0 && rms < 3500.0 {
            let gain = (5500.0 / rms.max(80.0)).min(3.5);
            for s in samples.iter_mut() {
                let scaled = (*s as f32 * gain).clamp(-32768.0, 32767.0);
                *s = scaled as i16;
            }
        }
    }
}
