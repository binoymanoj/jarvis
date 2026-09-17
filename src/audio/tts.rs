use crate::core::error::{JarvisError, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tracing::{info, warn};

pub struct TextToSpeech {
    engine: String,
    edge_voice: String,
    piper_bin: Option<PathBuf>,
    piper_model: PathBuf,
    piper_config: PathBuf,
    is_speaking: Arc<AtomicBool>,
}

impl Default for TextToSpeech {
    fn default() -> Self {
        Self::new("edge", "en-GB-RyanNeural", "en_GB-alan-medium")
    }
}

impl TextToSpeech {
    pub fn new(engine: &str, edge_voice: &str, piper_voice: &str) -> Self {
        let piper_bin = which::which("piper").ok();

        let base_models = crate::audio::resolve_models_dir();
        let piper_model = base_models.join(format!("{piper_voice}.onnx"));
        let piper_config = base_models.join(format!("{piper_voice}.onnx.json"));

        Self {
            engine: engine.to_lowercase().trim().to_string(),
            edge_voice: edge_voice.trim().to_string(),
            piper_bin,
            piper_model,
            piper_config,
            is_speaking: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn from_settings(settings: &crate::core::config::Settings) -> Self {
        Self::new(
            &settings.tts_engine,
            &settings.edge_voice,
            &settings.piper_voice,
        )
    }

    pub fn is_speaking(&self) -> bool {
        self.is_speaking.load(Ordering::SeqCst)
    }

    pub async fn stop(&self) {
        self.is_speaking.store(false, Ordering::SeqCst);
        let _ = Command::new("pkill")
            .arg("-f")
            .arg("pw-play")
            .status()
            .await;
    }

    pub async fn speak(&self, text: &str) -> Result<()> {
        let clean = text.trim();
        if clean.is_empty() {
            return Ok(());
        }

        info!("Jarvis Speaking: \"{clean}\"");
        self.is_speaking.store(true, Ordering::SeqCst);

        let res = if self.engine == "piper" && self.piper_bin.is_some() && self.piper_model.is_file() {
            self.speak_piper(clean).await
        } else {
            self.speak_edge(clean).await
        };

        self.is_speaking.store(false, Ordering::SeqCst);
        res
    }

    async fn speak_piper(&self, text: &str) -> Result<()> {
        let piper = self.piper_bin.as_ref().unwrap();
        let tmp_wav = PathBuf::from("/tmp/jarvis_speech.wav");

        let mut child = Command::new(piper)
            .arg("--model")
            .arg(&self.piper_model)
            .arg("--config")
            .arg(&self.piper_config)
            .arg("--length-scale")
            .arg("0.85")
            .arg("--sentence-silence")
            .arg("0.0")
            .arg("--output_file")
            .arg(&tmp_wav)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes()).await?;
            stdin.flush().await?;
        }

        let status = child.wait().await?;
        if !status.success() {
            warn!("Piper synthesis failed, falling back to pure Rust Edge TTS");
            return self.speak_edge(text).await;
        }

        self.play_audio_file(&tmp_wav).await
    }

    /// Pure Rust async Edge-TTS synthesis over WebSockets
    async fn speak_edge(&self, text: &str) -> Result<()> {
        let voice_name = if self.edge_voice.is_empty() {
            "en-GB-RyanNeural"
        } else {
            &self.edge_voice
        };

        let config = msedge_tts::tts::SpeechConfig {
            voice_name: voice_name.to_string(),
            audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
            pitch: 0,
            rate: 20,
            volume: 0,
        };

        let mut client = msedge_tts::tts::client::tokio_runtime::connect_async()
            .await
            .map_err(|e| JarvisError::Tts(format!("Failed to connect to Edge TTS: {e}")))?;

        let audio = client
            .synthesize(text, &config)
            .await
            .map_err(|e| JarvisError::Tts(format!("Edge TTS synthesis failed: {e}")))?;

        let tmp_mp3 = PathBuf::from("/tmp/jarvis_speech.mp3");
        tokio::fs::write(&tmp_mp3, &audio.audio_bytes).await?;

        self.play_audio_file(&tmp_mp3).await
    }

    async fn play_audio_file(&self, path: &Path) -> Result<()> {
        let pw_play = which::which("pw-play")
            .or_else(|_| which::which("aplay"))
            .unwrap_or_else(|_| PathBuf::from("pw-play"));

        let status = Command::new(&pw_play)
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await?;

        if status.success() {
            Ok(())
        } else {
            Err(JarvisError::Audio(format!(
                "pw-play failed with status {:?}",
                status.code()
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tts_init() {
        let tts = TextToSpeech::default();
        assert!(!tts.is_speaking());
    }
}
