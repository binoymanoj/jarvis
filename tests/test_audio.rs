use jarvis::audio::capture::AudioCapture;
use jarvis::audio::playback::ensure_chime_file;
use jarvis::audio::vad::SileroVAD;
use jarvis::audio::wakeword::WakeWordDetector;
use jarvis::core::config::Settings;

#[test]
fn test_audio_rms_and_soft_agc() {
    let silent = vec![0i16; 1000];
    let rms_silent = AudioCapture::compute_rms(&silent);
    assert_eq!(rms_silent, 0.0);

    // Test whisper-level audio (~500 RMS)
    let mut low_audio: Vec<i16> = (0..1000)
        .map(|i| (500.0 * (i as f32 * 0.1).sin()) as i16)
        .collect();
    let initial_rms = AudioCapture::compute_rms(&low_audio);
    assert!(initial_rms > 300.0 && initial_rms < 400.0);

    AudioCapture::apply_soft_agc(&mut low_audio);
    let boosted_rms = AudioCapture::compute_rms(&low_audio);
    assert!(
        boosted_rms > initial_rms,
        "Soft AGC should amplify low volume audio"
    );
}

#[test]
fn test_vad_silence_rejection() {
    let mut vad = SileroVAD::default();
    let silence = vec![0i16; 512];
    let prob = vad.get_speech_prob(&silence);
    assert_eq!(
        prob, 0.0,
        "Absolute silence must return 0.0 speech probability"
    );
}

#[test]
fn test_vad_speech_detection() {
    if !std::path::Path::new("/tmp/hey_jarvis.wav").exists() {
        return;
    }
    let mut reader = hound::WavReader::open("/tmp/hey_jarvis.wav").unwrap();
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();

    let mut vad = SileroVAD::default();
    let mut max_prob = 0.0f32;
    for chunk in samples.chunks(512) {
        if chunk.len() == 512 {
            let p = vad.get_speech_prob(chunk);
            if p > max_prob {
                max_prob = p;
            }
        }
    }
    println!("Max Silero VAD speech prob: {max_prob}");
    assert!(
        max_prob > 0.8,
        "Silero VAD should detect clear speech with >0.8 prob, got {max_prob}"
    );
}

#[test]
fn test_wakeword_detector_initialization() {
    let settings = Settings::default();
    let mut detector = WakeWordDetector::new(&settings);
    let silence_chunk = vec![0i16; 1280];
    let (detected, score) = detector.process_chunk(&silence_chunk);
    assert!(!detected);
    assert!(
        score < 0.01,
        "Silence score should be near zero, got {score}"
    );
}

#[test]
fn test_chime_file_exists() {
    let path = ensure_chime_file();
    assert!(path.exists());
    assert!(path.extension().unwrap() == "wav");
}

#[test]
fn test_detect_hey_jarvis_wav() {
    if !std::path::Path::new("/tmp/hey_jarvis.wav").exists() {
        return;
    }
    let mut reader = hound::WavReader::open("/tmp/hey_jarvis.wav").unwrap();
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();

    let settings = Settings {
        wakeword_threshold: 0.99,
        ..Default::default()
    };
    let mut detector = WakeWordDetector::new(&settings);

    let mut max_score = 0.0f32;
    let mut detected_any = false;

    for (idx, chunk) in samples.chunks(512).enumerate() {
        let (_det, score) = detector.process_chunk(chunk);
        if score > 0.01 {
            println!("Rust Chunk {idx}: score = {score:.4}");
        }
        if score > max_score {
            max_score = score;
        }
        if score >= 0.28 {
            detected_any = true;
        }
    }

    println!("Rust max wakeword score: {max_score}, detected: {detected_any}");
    assert!(
        detected_any,
        "Expected Hey Jarvis to be detected in /tmp/hey_jarvis.wav, got max_score={max_score}"
    );
}

#[test]
fn test_hey_jarvis_detected_and_jarvis_rejected_at_threshold_50() {
    let settings = Settings::default();
    assert_eq!(settings.wakeword_threshold, 0.50);
    assert_eq!(settings.wakeword_name, "hey jarvis");

    // 1. Verify "hey jarvis" audio is detected at threshold 0.50
    if std::path::Path::new("/tmp/hey_jarvis.wav").exists() {
        let mut reader = hound::WavReader::open("/tmp/hey_jarvis.wav").unwrap();
        let samples: Vec<i16> = reader
            .samples::<i16>()
            .filter_map(std::result::Result::ok)
            .collect();
        let mut detector = WakeWordDetector::new(&settings);
        let mut detected_any = false;
        let mut max_score = 0.0f32;
        for chunk in samples.chunks(512) {
            let (det, score) = detector.process_chunk(chunk);
            if score > max_score {
                max_score = score;
            }
            if det {
                detected_any = true;
            }
        }
        println!("Hey Jarvis score: {max_score}, detected: {detected_any}");
        assert!(
            detected_any,
            "Hey Jarvis must be detected at threshold 0.50, got {max_score}"
        );
    }

    // 2. Verify saying only "jarvis" is REJECTED at threshold 0.50 (eliminates false positives)
    if std::path::Path::new("/tmp/test_jarvis_padded.wav").exists() {
        let mut reader = hound::WavReader::open("/tmp/test_jarvis_padded.wav").unwrap();
        let samples: Vec<i16> = reader
            .samples::<i16>()
            .filter_map(std::result::Result::ok)
            .collect();
        let mut detector = WakeWordDetector::new(&settings);
        let mut detected_any = false;
        let mut max_score = 0.0f32;
        for chunk in samples.chunks(512) {
            let (det, score) = detector.process_chunk(chunk);
            if score > max_score {
                max_score = score;
            }
            if det {
                detected_any = true;
            }
        }
        println!("Jarvis-only score: {max_score}, detected: {detected_any}");
        assert!(
            !detected_any,
            "Standalone 'Jarvis' must NOT trigger wake word at threshold 0.50, got {max_score}"
        );
    }
}

#[test]
fn test_detect_hey_jarvis_pipewire_chunks() {
    if !std::path::Path::new("/tmp/hey_jarvis.wav").exists() {
        return;
    }
    let mut reader = hound::WavReader::open("/tmp/hey_jarvis.wav").unwrap();
    let samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();

    let settings = Settings {
        wakeword_threshold: 0.99,
        ..Default::default()
    };
    let mut detector = WakeWordDetector::new(&settings);

    let mut max_score = 0.0f32;
    let mut detected_any = false;

    // Simulate typical Linux PipeWire quantum buffer sizes: 341, 342, 485
    let chunk_sizes = [341, 341, 342, 341, 485];
    let mut idx = 0;
    let mut c_i = 0;

    while idx < samples.len() {
        let sz = chunk_sizes[c_i % chunk_sizes.len()].min(samples.len() - idx);
        let chunk = &samples[idx..idx + sz];
        let (_det, score) = detector.process_chunk(chunk);
        if score > max_score {
            max_score = score;
        }
        if score >= 0.28 {
            detected_any = true;
        }
        idx += sz;
        c_i += 1;
    }

    println!("PipeWire chunking max wakeword score: {max_score}, detected: {detected_any}");
    assert!(
        detected_any,
        "Expected Hey Jarvis to be detected under PipeWire chunking, got max_score={max_score}"
    );

    // Also test quiet speech (0.25x volume, e.g. speaking at normal distance from laptop)
    if std::path::Path::new("/tmp/quiet_hey_jarvis.wav").exists() {
        let mut reader_q = hound::WavReader::open("/tmp/quiet_hey_jarvis.wav").unwrap();
        let samples_q: Vec<i16> = reader_q.samples::<i16>().map(|s| s.unwrap()).collect();
        detector.reset();
        let mut max_q = 0.0f32;
        let mut idx_q = 0;
        let mut c_q = 0;
        while idx_q < samples_q.len() {
            let sz = chunk_sizes[c_q % chunk_sizes.len()].min(samples_q.len() - idx_q);
            let chunk = &samples_q[idx_q..idx_q + sz];
            let (_det, score) = detector.process_chunk(chunk);
            if score > max_q {
                max_q = score;
            }
            idx_q += sz;
            c_q += 1;
        }
        println!("Quiet speech wakeword max score: {max_q}");
        assert!(
            max_q >= 0.28,
            "Quiet speech should be boosted by Soft AGC and detected, got {max_q}"
        );
    }
}

#[tokio::test]
async fn test_native_rust_tts() {
    let config = msedge_tts::tts::SpeechConfig {
        voice_name: "en-GB-RyanNeural".to_string(),
        audio_format: "audio-24khz-48kbitrate-mono-mp3".to_string(),
        pitch: 0,
        rate: 20,
        volume: 0,
    };
    let mut client = msedge_tts::tts::client::tokio_runtime::connect_async()
        .await
        .unwrap();
    let audio = client
        .synthesize("Hello, this is pure Rust TTS.", &config)
        .await
        .unwrap();
    assert!(!audio.audio_bytes.is_empty());
    println!(
        "Synthesized {} bytes of pure Rust audio!",
        audio.audio_bytes.len()
    );
}
