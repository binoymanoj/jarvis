use crate::audio::capture::AudioCapture;
use crate::core::config::Settings;
use ndarray::{Array2, Array3, Array4};
use ort::session::Session;
use ort::value::Tensor;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn};

pub struct WakeWordDetector {
    enabled: bool,
    name: String,
    threshold: f32,
    cooldown: Duration,
    last_trigger: Instant,
    melspec_session: Option<Session>,
    embedding_session: Option<Session>,
    wakeword_session: Option<Session>,
    input_buffer: VecDeque<i16>,
    raw_buffer: VecDeque<i16>,
    melspec_buffer: Vec<[f32; 32]>,
    feature_buffer: Vec<[f32; 96]>,
}

#[allow(clippy::excessive_precision)]
const SILENCE_EMBEDDING: [f32; 96] = [
    -4.936722, 19.387594, 8.662575, -3.079512, 4.874652, 25.96701, 10.267291, -15.02655, -1.100898,
    11.771112, -26.499851, -1.849956, 9.19239, -5.976384, -5.40081, -3.406047, 7.080233, -5.844443,
    3.985872, -13.363791, 4.440428, 18.841412, -8.178735, -9.440122, -7.618305, 13.860628,
    -20.480137, -2.998387, 4.332905, 2.646651, -10.251684, 18.191359, -21.826748, -3.785842,
    -12.35635, -1.343246, 38.230148, 18.310574, -8.144447, 26.286665, -9.480198, 2.89648,
    37.794834, -16.301245, -13.528915, -7.998281, -8.230913, 6.217336, 15.473965, -7.471695,
    -9.008852, 3.090901, 11.819296, -1.033224, -20.816809, -17.792492, -1.208664, 28.544418,
    -15.447296, 0.356498, 6.248527, 8.138709, 11.195825, -3.0412, 25.106758, 14.920961, 8.971652,
    -13.14422, -10.940308, 3.817143, 0.657487, 9.037533, 8.665456, -6.888185, 12.278726, 6.570675,
    3.519481, 7.54345, -22.791733, -36.776505, 11.589973, 17.173134, 8.118751, -11.019923,
    13.788208, -5.869898, 13.466286, -10.36521, 10.326647, 37.467678, 5.638528, 21.887428,
    28.544918, -30.547234, 7.677816, 27.27882,
];

impl WakeWordDetector {
    pub fn new(settings: &Settings) -> Self {
        let models_dir = crate::audio::resolve_models_dir();
        let mel_path = models_dir.join("melspectrogram.onnx");
        let emb_path = models_dir.join("embedding_model.onnx");

        let ww_filename = if settings.wakeword_model.ends_with(".onnx") {
            settings.wakeword_model.clone()
        } else {
            format!("{}.onnx", settings.wakeword_model)
        };

        let ww_path = if std::path::Path::new(&ww_filename).is_absolute() {
            std::path::PathBuf::from(&ww_filename)
        } else {
            models_dir.join(&ww_filename)
        };

        let build_opt_session = |path: &std::path::Path| -> Option<Session> {
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
        };

        let melspec_session = if mel_path.exists() {
            build_opt_session(&mel_path)
        } else {
            None
        };

        let embedding_session = if emb_path.exists() {
            build_opt_session(&emb_path)
        } else {
            None
        };

        let wakeword_session = if ww_path.exists() {
            build_opt_session(&ww_path)
        } else {
            // Fall back to hey_jarvis_v0.1.onnx if custom model not found
            let fallback_ww = models_dir.join("hey_jarvis_v0.1.onnx");
            build_opt_session(&fallback_ww)
        };

        let display_name = if settings.wakeword_name.starts_with("hey ") {
            let rest = settings.wakeword_name.trim_start_matches("hey ").trim();
            if let Some(first) = rest.chars().next() {
                format!("Hey {}{}", first.to_uppercase(), &rest[first.len_utf8()..])
            } else {
                "Hey Jarvis".to_string()
            }
        } else if settings.wakeword_name.eq_ignore_ascii_case("alexa") {
            "Alexa".to_string()
        } else {
            settings.wakeword_name.clone()
        };

        if melspec_session.is_some() && embedding_session.is_some() && wakeword_session.is_some() {
            info!("Local '{}' ONNX wake word engine initialized successfully (model: {:?}, threshold: {})", display_name, ww_path.file_name().unwrap_or_default(), settings.wakeword_threshold);
        } else {
            warn!("One or more wake word ONNX models not found in {:?}. Wake word detection will run in standby mode.", models_dir);
        }

        // Initialize mel buffer with 76 frames of 32 features (ones, matching openWakeWord)
        let melspec_buffer = vec![[1.0f32; 32]; 76];
        let feature_buffer = vec![SILENCE_EMBEDDING; 16];

        Self {
            enabled: settings.wakeword_enabled,
            name: display_name,
            threshold: settings.wakeword_threshold,
            cooldown: Duration::from_millis(1500),
            last_trigger: Instant::now() - Duration::from_secs(10),
            melspec_session,
            embedding_session,
            wakeword_session,
            input_buffer: VecDeque::with_capacity(4096),
            raw_buffer: VecDeque::with_capacity(16000),
            melspec_buffer,
            feature_buffer,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn reset(&mut self) {
        self.input_buffer.clear();
        self.raw_buffer.clear();
        self.melspec_buffer = vec![[1.0f32; 32]; 76];
        self.feature_buffer = vec![SILENCE_EMBEDDING; 16];
    }

    pub fn process_chunk(&mut self, chunk: &[i16]) -> (bool, f32) {
        if !self.enabled {
            return (false, 0.0);
        }

        for &s in chunk {
            self.input_buffer.push_back(s);
        }

        let mut max_score = 0.0f32;
        let mut any_detected = false;

        // openWakeWord operates on 80ms windows (exactly 1280 samples at 16kHz).
        // Drain complete 1280-sample frames sequentially to maintain perfect STFT temporal continuity.
        while self.input_buffer.len() >= 1280 {
            let mut frame = Vec::with_capacity(1280);
            for _ in 0..1280 {
                if let Some(s) = self.input_buffer.pop_front() {
                    frame.push(s);
                }
            }

            for &s in &frame {
                self.raw_buffer.push_back(s);
            }
            while self.raw_buffer.len() > 16000 {
                self.raw_buffer.pop_front();
            }

            if self.raw_buffer.len() < 1760 {
                continue;
            }

            let (detected, score) = self.run_inference_step();
            if score > max_score {
                max_score = score;
            }
            if detected {
                any_detected = true;
                break;
            }
        }

        (any_detected, max_score)
    }

    fn run_inference_step(&mut self) -> (bool, f32) {
        let mel_sess = match self.melspec_session.as_mut() {
            Some(s) => s,
            None => return (false, 0.0),
        };
        let emb_sess = match self.embedding_session.as_mut() {
            Some(s) => s,
            None => return (false, 0.0),
        };
        let ww_sess = match self.wakeword_session.as_mut() {
            Some(s) => s,
            None => return (false, 0.0),
        };

        // 1. Compute streaming melspectrogram (requires at least 1760 samples for openWakeWord STFT)
        let start_idx = self.raw_buffer.len() - 1760;
        let float_audio: Vec<f32> = self
            .raw_buffer
            .range(start_idx..)
            .map(|&s| s as f32)
            .collect();

        let mel_input_arr = match Array2::from_shape_vec((1, 1760), float_audio) {
            Ok(arr) => arr,
            Err(_) => return (false, 0.0),
        };

        let mel_tensor = match Tensor::from_array(mel_input_arr) {
            Ok(t) => t,
            Err(_) => return (false, 0.0),
        };

        let mel_inputs = ort::inputs!["input" => mel_tensor];

        if let Ok(mel_out) = mel_sess.run(mel_inputs) {
            if let Some(out_tensor) = mel_out.get("output") {
                if let Ok((_shape, data)) = out_tensor.try_extract_tensor::<f32>() {
                    let num_frames = data.len() / 32;
                    for f in 0..num_frames {
                        let mut frame = [0.0f32; 32];
                        let offset = f * 32;
                        if offset + 32 <= data.len() {
                            for i in 0..32 {
                                frame[i] = data[offset + i] / 10.0 + 2.0;
                            }
                            self.melspec_buffer.push(frame);
                        }
                    }
                }
            }
        }

        // Limit mel buffer size
        if self.melspec_buffer.len() > 120 {
            let excess = self.melspec_buffer.len() - 120;
            self.melspec_buffer.drain(0..excess);
        }

        // 2. Compute embeddings if we have at least 76 mel frames
        if self.melspec_buffer.len() >= 76 {
            let start = self.melspec_buffer.len() - 76;
            let mut emb_input_vec = Vec::with_capacity(76 * 32);
            for frame in &self.melspec_buffer[start..] {
                emb_input_vec.extend_from_slice(frame);
            }

            if let Ok(emb_arr) = Array4::from_shape_vec((1, 76, 32, 1), emb_input_vec) {
                if let Ok(emb_t) = Tensor::from_array(emb_arr) {
                    let emb_in = ort::inputs!["input_1" => emb_t];
                    if let Ok(emb_out) = emb_sess.run(emb_in) {
                        if let Some(feat_t) = emb_out.get("conv2d_19") {
                            if let Ok((_shape, f_data)) = feat_t.try_extract_tensor::<f32>() {
                                if f_data.len() >= 96 {
                                    let mut feat_96 = [0.0f32; 96];
                                    feat_96.copy_from_slice(&f_data[..96]);
                                    self.feature_buffer.push(feat_96);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Limit feature buffer size
        if self.feature_buffer.len() > 32 {
            let excess = self.feature_buffer.len() - 32;
            self.feature_buffer.drain(0..excess);
        }

        // 3. Evaluate wake word classifier with last 16 feature frames
        if self.feature_buffer.len() < 16 {
            return (false, 0.0);
        }

        let start_f = self.feature_buffer.len() - 16;
        let mut ww_input_vec = Vec::with_capacity(16 * 96);
        for frame in &self.feature_buffer[start_f..] {
            ww_input_vec.extend_from_slice(frame);
        }

        let mut detected_score: Option<f32> = None;
        if let Ok(ww_arr) = Array3::from_shape_vec((1, 16, 96), ww_input_vec) {
            if let Ok(ww_t) = Tensor::from_array(ww_arr) {
                let ww_in = ort::inputs!["x.1" => ww_t];
                if let Ok(ww_out) = ww_sess.run(ww_in) {
                    if let Some(score_t) = ww_out.get("53") {
                        if let Ok((_shape, s_data)) = score_t.try_extract_tensor::<f32>() {
                            if let Some(&score) = s_data.first() {
                                detected_score = Some(score);
                            }
                        }
                    }
                }
            }
        }

        if let Some(score) = detected_score {
            let now = Instant::now();
            if score >= self.threshold && now.duration_since(self.last_trigger) > self.cooldown {
                self.last_trigger = now;
                self.reset();
                return (true, score);
            }
            return (false, score);
        }

        (false, 0.0)
    }

    /// Background listener loop that captures audio and signals when "Hey Jarvis" is spoken
    pub async fn listen_loop<F, Fut>(
        &mut self,
        mut on_detected: F,
        stop_signal: Arc<AtomicBool>,
        pause_signal: Arc<AtomicBool>,
        manual_trigger: Arc<AtomicBool>,
    ) where
        F: FnMut() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        info!(
            "Starting background wake word listener ('{}')...",
            self.name
        );
        let capture = AudioCapture::new(16000, 1);
        let chunk_size = 1280; // 80ms at 16kHz

        while !stop_signal.load(Ordering::SeqCst) {
            // Check manual trigger hotkey / menubar signal
            if manual_trigger.swap(false, Ordering::SeqCst) {
                info!("External trigger received! Activating voice loop...");
                pause_signal.store(true, Ordering::SeqCst);
                on_detected().await;
                self.reset();
                pause_signal.store(false, Ordering::SeqCst);
                sleep(Duration::from_millis(50)).await;
                continue;
            }

            let mut paused_count = 0;
            while pause_signal.load(Ordering::SeqCst) && !stop_signal.load(Ordering::SeqCst) {
                sleep(Duration::from_millis(100)).await;
                paused_count += 1;
                if paused_count >= 450 {
                    warn!("Wake word listener pause_signal was stuck for >45s. Auto-recovering listener...");
                    pause_signal.store(false, Ordering::SeqCst);
                    crate::core::state::ensure_idle();
                    break;
                }
            }

            let (_stream, mut rx) = match capture.start_stream(chunk_size) {
                Ok(res) => res,
                Err(e) => {
                    warn!("Failed to start wake word capture stream ({e}), retrying in 2s...");
                    sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            while !stop_signal.load(Ordering::SeqCst) && !pause_signal.load(Ordering::SeqCst) {
                if manual_trigger.swap(false, Ordering::SeqCst) {
                    info!("External trigger received! Activating voice loop...");
                    drop(_stream);
                    sleep(Duration::from_millis(50)).await;
                    pause_signal.store(true, Ordering::SeqCst);
                    on_detected().await;
                    self.reset();
                    pause_signal.store(false, Ordering::SeqCst);
                    sleep(Duration::from_millis(50)).await;
                    break;
                }

                let chunk = match tokio::time::timeout(Duration::from_millis(120), rx.recv()).await
                {
                    Ok(Some(c)) => c,
                    Ok(None) => break,
                    Err(_) => continue,
                };

                let (detected, score) = self.process_chunk(&chunk);
                if detected {
                    info!(
                        "󰚩 Wake word '{}' detected! (score: {:.2})",
                        self.name, score
                    );
                    drop(_stream); // Release mic stream for conversational recording turn
                    sleep(Duration::from_millis(50)).await;
                    pause_signal.store(true, Ordering::SeqCst);
                    on_detected().await;
                    self.reset();
                    pause_signal.store(false, Ordering::SeqCst);
                    sleep(Duration::from_millis(50)).await;
                    break;
                }
            }
        }
    }
}
