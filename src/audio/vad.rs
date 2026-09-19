use ndarray::{arr0, Array2, Array3};
use ort::session::Session;
use ort::value::Tensor;
use std::path::Path;
use tracing::{debug, warn};

pub struct SileroVAD {
    session: Option<Session>,
    h: Array3<f32>,
    c: Array3<f32>,
    energy_threshold: f32,
}

impl Default for SileroVAD {
    fn default() -> Self {
        Self::new(None, 0.003)
    }
}

impl SileroVAD {
    pub fn new(model_path: Option<&Path>, energy_threshold: f32) -> Self {
        let default_path = crate::audio::resolve_models_dir().join("silero_vad.onnx");
        let path = model_path.unwrap_or(&default_path);

        let session = if path.exists() {
            let res = (|| -> Option<Session> {
                let mut b = Session::builder().ok()?;
                b = b.with_intra_threads(1).ok()?;
                b = b.with_inter_threads(1).ok()?;
                b = b.with_memory_pattern(false).ok()?;
                b = b.with_intra_op_spinning(false).ok()?;
                b = b.with_inter_op_spinning(false).ok()?;
                b = b.with_parallel_execution(false).ok()?;
                b = b
                    .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level1)
                    .ok()?;
                b.commit_from_file(path).ok()
            })();
            match res {
                Some(s) => {
                    debug!("Silero VAD initialized successfully from {:?}", path);
                    Some(s)
                }
                None => {
                    warn!("Failed to load Silero VAD model, using energy-based VAD fallback");
                    None
                }
            }
        } else {
            warn!(
                "Silero VAD model not found at {:?}, using energy fallback",
                path
            );
            None
        };

        Self {
            session,
            h: Array3::<f32>::zeros((2, 1, 64)),
            c: Array3::<f32>::zeros((2, 1, 64)),
            energy_threshold,
        }
    }

    pub fn reset(&mut self) {
        self.h.fill(0.0);
        self.c.fill(0.0);
    }

    /// Calculate speech probability for an audio chunk (nominally 512 samples at 16kHz).
    pub fn get_speech_prob(&mut self, chunk: &[i16]) -> f32 {
        if chunk.is_empty() {
            return 0.0;
        }

        // Fast RMS floor check
        let sum_sq: f64 = chunk.iter().map(|&s| (s as f64).powi(2)).sum();
        let rms = ((sum_sq / chunk.len() as f64).sqrt()) as f32 / 32768.0;

        if rms < self.energy_threshold {
            return 0.0;
        }

        let session = match self.session.as_mut() {
            Some(s) => s,
            None => {
                // If model is not loaded, pure energy threshold fallback
                return if rms >= self.energy_threshold * 2.5 {
                    0.8
                } else {
                    0.0
                };
            }
        };

        // Convert i16 samples to normalized float32 [-1.0, 1.0] with fixed 512-sample length for Silero VAD
        let mut float_samples = vec![0.0f32; 512];
        let copy_len = chunk.len().min(512);
        for i in 0..copy_len {
            float_samples[i] = chunk[i] as f32 / 32768.0;
        }
        let input_tensor = match Array2::from_shape_vec((1, 512), float_samples) {
            Ok(t) => t,
            Err(_) => return 0.0,
        };
        let sr_tensor = arr0(16000i64);

        let input_val = Tensor::from_array(input_tensor);
        let h_val = Tensor::from_array(self.h.clone());
        let c_val = Tensor::from_array(self.c.clone());
        let sr_val = Tensor::from_array(sr_tensor);

        if let (Ok(in_t), Ok(h_t), Ok(c_t), Ok(sr_t)) = (input_val, h_val, c_val, sr_val) {
            let inputs = ort::inputs![
                "input" => in_t,
                "sr" => sr_t,
                "h" => h_t,
                "c" => c_t,
            ];

            if let Ok(outputs) = session.run(inputs) {
                if let Some(prob_tensor) = outputs.get("output") {
                    if let Ok((_shape, data)) = prob_tensor.try_extract_tensor::<f32>() {
                        if let Some(&prob) = data.first() {
                            if let Some(hn_out) = outputs.get("hn") {
                                if let Ok((_s_shape, s_data)) = hn_out.try_extract_tensor::<f32>() {
                                    if s_data.len() == 128 {
                                        self.h.as_slice_mut().unwrap().copy_from_slice(s_data);
                                    }
                                }
                            }
                            if let Some(cn_out) = outputs.get("cn") {
                                if let Ok((_s_shape, s_data)) = cn_out.try_extract_tensor::<f32>() {
                                    if s_data.len() == 128 {
                                        self.c.as_slice_mut().unwrap().copy_from_slice(s_data);
                                    }
                                }
                            }
                            return prob;
                        }
                    }
                }
            }
        }

        // Fallback to RMS score if tensor execution fails
        if rms >= self.energy_threshold * 2.0 {
            0.7
        } else {
            0.0
        }
    }

    pub fn is_speech(&mut self, chunk: &[i16], threshold: f32) -> bool {
        self.get_speech_prob(chunk) >= threshold
    }
}
