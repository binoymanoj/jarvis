use crate::audio::capture::AudioCapture;
use crate::audio::vad::SileroVAD;
use crate::core::error::{JarvisError, Result};
use hound::{WavSpec, WavWriter};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

pub struct AudioRecorder {
    sample_rate: u32,
    chunk_size: usize,
    silence_timeout_secs: f32,
    max_duration_secs: f32,
    vad_threshold: f32,
    energy_threshold: f32,
    silence_chunks_limit: usize,
    stop_requested: Arc<AtomicBool>,
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new(16000, 512, 1.3, 25.0, 0.28, 0.0008)
    }
}

impl AudioRecorder {
    pub fn new(
        sample_rate: u32,
        chunk_size: usize,
        silence_timeout_secs: f32,
        max_duration_secs: f32,
        vad_threshold: f32,
        energy_threshold: f32,
    ) -> Self {
        let chunk_duration = chunk_size as f32 / sample_rate as f32;
        let silence_chunks_limit = ((silence_timeout_secs / chunk_duration) as usize).max(15);

        Self {
            sample_rate,
            chunk_size,
            silence_timeout_secs,
            max_duration_secs,
            vad_threshold,
            energy_threshold,
            silence_chunks_limit,
            stop_requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn from_settings(settings: &crate::core::config::Settings) -> Self {
        Self::new(
            settings.sample_rate,
            512,
            settings.silence_threshold_seconds,
            settings.max_recording_seconds,
            0.28,
            0.0008,
        )
    }

    pub fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::SeqCst);
    }

    pub async fn record_phrase<F>(
        &self,
        mut on_volume: Option<F>,
        initial_timeout_secs: Option<f32>,
        force_stop_signal: Option<Arc<AtomicBool>>,
    ) -> Result<Vec<u8>>
    where
        F: FnMut(f32) + Send + 'static,
    {
        self.stop_requested.store(false, Ordering::SeqCst);

        let mut vad = SileroVAD::default();
        vad.reset();

        let capture = AudioCapture::new(self.sample_rate, 1);
        let (_stream, mut rx) = capture.start_stream(self.chunk_size)?;

        info!("Listening for speech (fast VAD active)...");

        let mut recorded_samples: Vec<i16> = Vec::new();
        let mut vad_buffer: Vec<i16> = Vec::new();
        let mut speech_started = false;
        let mut consecutive_speech_chunks = 0;
        let mut consecutive_silence_chunks = 0;
        let start_time = Instant::now();
        let wait_timeout = Duration::from_secs_f32(initial_timeout_secs.unwrap_or(8.0));
        let max_duration = Duration::from_secs_f32(self.max_duration_secs);

        while !self.stop_requested.load(Ordering::SeqCst)
            && !force_stop_signal
                .as_ref()
                .map(|s| s.load(Ordering::SeqCst))
                .unwrap_or(false)
        {
            let chunk = match tokio::time::timeout(Duration::from_millis(80), rx.recv()).await {
                Ok(Some(c)) => c,
                Ok(None) => break,
                Err(_) => continue,
            };

            recorded_samples.extend_from_slice(&chunk);
            vad_buffer.extend_from_slice(&chunk);

            // Compute RMS and update HUD volume meter
            let rms = AudioCapture::compute_rms(&chunk) / 32768.0;
            let vol = (rms * 14.0).min(1.0);
            if let Some(ref mut cb) = on_volume {
                cb(vol);
            }

            // Process complete 512-sample frames through Silero VAD
            while vad_buffer.len() >= 512 {
                let frame: Vec<i16> = vad_buffer.drain(..512).collect();
                let vad_prob = vad.get_speech_prob(&frame);
                let chunk_rms = AudioCapture::compute_rms(&frame) / 32768.0;
                let is_real_speech = vad_prob >= self.vad_threshold && chunk_rms >= self.energy_threshold;

                if is_real_speech {
                    consecutive_speech_chunks += 1;
                    consecutive_silence_chunks = 0;
                    if !speech_started && consecutive_speech_chunks >= 2 {
                        speech_started = true;
                        info!("Speech actively detected (vad={vad_prob:.3}, rms={chunk_rms:.5})");
                    }
                } else {
                    consecutive_speech_chunks = 0;
                    if speech_started {
                        consecutive_silence_chunks += 1;
                        if consecutive_silence_chunks >= self.silence_chunks_limit {
                            info!("Natural pause detected (~{:.2}s). Submitting phrase.", self.silence_timeout_secs);
                            break;
                        }
                    }
                }
            }

            if speech_started && consecutive_silence_chunks >= self.silence_chunks_limit {
                break;
            }

            let elapsed = start_time.elapsed();
            if !speech_started && elapsed >= wait_timeout {
                info!("No speech detected within wait timeout. Ending listen.");
                break;
            }

            if elapsed >= max_duration {
                info!("Reached maximum recording duration ({}s).", self.max_duration_secs);
                break;
            }
        }

        if !speech_started && !self.stop_requested.load(Ordering::SeqCst) {
            let total_rms = AudioCapture::compute_rms(&recorded_samples);
            // If the user spoke anything audible (>30 RMS) for at least 0.35 seconds, do NOT discard it!
            if recorded_samples.len() >= 16000 * 35 / 100 && total_rms > 30.0 {
                info!("Audible sound detected in recording (RMS {:.1}, {} samples). Sending to transcription.", total_rms, recorded_samples.len());
            } else {
                debug!("No speech detected in audio stream (RMS {:.1}). Discarding.", total_rms);
                return Ok(Vec::new());
            }
        }

        if recorded_samples.is_empty() {
            return Ok(Vec::new());
        }

        // Apply Soft AGC to recorded audio for optimal Groq Whisper accuracy
        AudioCapture::apply_soft_agc(&mut recorded_samples);

        // Encode to WAV format
        let spec = WavSpec {
            channels: 1,
            sample_rate: self.sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut cursor = Cursor::new(Vec::new());
        let mut writer = WavWriter::new(&mut cursor, spec)
            .map_err(|e| JarvisError::Audio(format!("Failed to create WAV writer: {e}")))?;

        for sample in recorded_samples {
            writer.write_sample(sample).unwrap();
        }
        writer.finalize()
            .map_err(|e| JarvisError::Audio(format!("Failed to finalize WAV: {e}")))?;

        Ok(cursor.into_inner())
    }
}
