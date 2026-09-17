use crate::core::error::Result;
use hound::{WavSpec, WavWriter};
use std::f32::consts::PI;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;
use tokio::process::Command;
use tracing::debug;

static CHIME_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn ensure_chime_file() -> &'static PathBuf {
    CHIME_PATH.get_or_init(|| {
        let path = PathBuf::from("/tmp/jarvis_wake_chime.wav");
        if !path.exists() {
            let _ = generate_chime_wav(&path);
        }
        path
    })
}

fn generate_chime_wav(path: &PathBuf) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate: 24000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)
        .map_err(|e| crate::core::error::JarvisError::Audio(format!("Failed to create chime: {e}")))?;

    let sr = 24000.0f32;
    let n1 = (sr * 0.06) as usize;
    let n2 = (sr * 0.08) as usize;

    // Tone 1: 587.33 Hz (D5) for 60ms
    for i in 0..n1 {
        let t = i as f32 / sr;
        let fade = (PI * (i as f32 / n1 as f32)).sin();
        let sample = (2.0 * PI * 587.33 * t).sin() * 0.12 * fade;
        let int_sample = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(int_sample).unwrap();
    }

    // Tone 2: 880.00 Hz (A5) for 80ms
    for i in 0..n2 {
        let t = i as f32 / sr;
        let fade = (PI * (i as f32 / n2 as f32)).sin();
        let sample = (2.0 * PI * 880.00 * t).sin() * 0.16 * fade;
        let int_sample = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(int_sample).unwrap();
    }

    writer.finalize()
        .map_err(|e| crate::core::error::JarvisError::Audio(format!("Failed to finalize chime: {e}")))?;

    Ok(())
}

pub async fn play_wake_chime() {
    let path = ensure_chime_file();
    let pw_play = which::which("pw-play")
        .or_else(|_| which::which("aplay"))
        .unwrap_or_else(|_| PathBuf::from("pw-play"));

    let _ = Command::new(&pw_play)
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await;

    debug!("Played wake confirmation chime");
}

pub async fn play_audio_file(path: &std::path::Path) -> Result<()> {
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
        Err(crate::core::error::JarvisError::Audio(format!(
            "pw-play failed with status {:?}",
            status.code()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_generate_chime_wav() {
        let path = PathBuf::from("/tmp/test_jarvis_chime.wav");
        generate_chime_wav(&path).unwrap();
        assert!(path.exists());
        assert!(fs::metadata(&path).unwrap().len() > 100);
        let _ = fs::remove_file(path);
    }
}
