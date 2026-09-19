use crate::ai::stt::SpeechToText;
use crate::ai::{is_exit_command, JarvisAgent};
use crate::audio::playback::play_wake_chime;
use crate::audio::recorder::AudioRecorder;
use crate::audio::tts::TextToSpeech;
use crate::audio::wakeword::WakeWordDetector;
use crate::cli::args::CliArgs;
use crate::core::config::Settings;
use crate::core::error::Result;
use crate::core::state::{
    clear_busy, daemon_pid_file_path, is_pid_running, pid_file_path, read_pid, read_status,
    remove_file_if_exists, set_idle, set_processing, set_recording, set_speaking, update_status,
    write_pid,
};
use crate::tools::hyprland::HyprlandController;
use crate::tools::workflow::WorkflowManager;
use crate::ui::hud::JarvisHUD;
use crate::ui::notification::TaskNotifier;
use crate::ui::signals::{DaemonSignal, SignalHandler};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{debug, info, warn};

pub fn stop_running_session() -> bool {
    let mut killed = false;

    // 1. If daemon is running and in an active conversational turn, signal SIGHUP
    let st = read_status();
    let is_turn_active = st.active
        || st.state == "recording"
        || st.state == "processing"
        || st.state == "speaking"
        || st.mic_active;

    let daemon_path = daemon_pid_file_path();
    if is_turn_active {
        if let Some(d_pid) = read_pid(&daemon_path) {
            if is_pid_running(d_pid) && d_pid != std::process::id() as i32 {
                let _ = kill(Pid::from_raw(d_pid), Signal::SIGHUP);
                killed = true;
            }
        }
    }

    // 2. If a standalone session PID exists, terminate that standalone process
    let session_path = pid_file_path();
    if let Some(pid) = read_pid(&session_path) {
        if is_pid_running(pid) && pid != std::process::id() as i32 {
            let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
            std::thread::sleep(Duration::from_millis(80));
            if is_pid_running(pid) {
                let _ = kill(Pid::from_raw(pid), Signal::SIGKILL);
            }
            killed = true;
        }
        remove_file_if_exists(&session_path);
    }

    // 3. Silence any active audio playback
    let _ = Command::new("pkill").arg("-9").arg("pw-play").output();

    // 4. Dismiss active task notification and clear busy state
    tokio::spawn(async move {
        TaskNotifier::global().cancel().await;
    });
    clear_busy();

    // 5. Hide Quickshell HUD immediately via IPC
    let hud = JarvisHUD::new();
    tokio::spawn(async move {
        hud.hide().await;
    });

    // 6. Reset status
    let daemon_alive = read_pid(&daemon_path).map(is_pid_running).unwrap_or(false);
    set_idle(daemon_alive);

    killed
}

pub fn quit_all() {
    println!("\x1b[1;33m󰚩 Stopping all Jarvis processes and services...\x1b[0m");

    // 1. Stop systemd user service
    let _ = Command::new("systemctl")
        .args(["--user", "stop", "jarvis"])
        .output();

    // 2. Stop running sessions
    stop_running_session();

    // 3. Terminate daemon PID if still running
    let daemon_path = daemon_pid_file_path();
    if let Some(d_pid) = read_pid(&daemon_path) {
        if is_pid_running(d_pid) && d_pid != std::process::id() as i32 {
            let _ = kill(Pid::from_raw(d_pid), Signal::SIGTERM);
            std::thread::sleep(Duration::from_millis(100));
            if is_pid_running(d_pid) {
                let _ = kill(Pid::from_raw(d_pid), Signal::SIGKILL);
            }
        }
        remove_file_if_exists(&daemon_path);
    }

    // 4. Silence audio
    let _ = Command::new("pkill").arg("-9").arg("pw-play").output();

    // 5. Mark state as inactive
    update_status(|s| {
        s.active = false;
        s.daemon_running = false;
        s.state = "idle".to_string();
        s.mic_active = false;
    });

    println!("\x1b[1;31m󰚩 Jarvis completely terminated.\x1b[0m Background daemon and systemd service stopped.");
}

pub fn restart_all() {
    println!("\x1b[1;36m󰚩 Restarting Jarvis...\x1b[0m");

    stop_running_session();

    // Restart systemd service
    match Command::new("systemctl")
        .args(["--user", "restart", "jarvis"])
        .output()
    {
        Ok(out) if out.status.success() => {
            println!("\x1b[32m✔ Background systemd service (jarvis.service) restarted.\x1b[0m");
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            println!("\x1b[33mWarning restarting systemd: {}\x1b[0m", err.trim());
        }
        Err(e) => {
            println!("\x1b[33mWarning restarting service: {e}\x1b[0m");
        }
    }

    // Rescan Omarchy plugins
    let _ = Command::new("omarchy-shell")
        .args(["shell", "rescanPlugins"])
        .output();
    println!("\x1b[32m✔ Omarchy menubar plugin rescanned.\x1b[0m");
    println!("\x1b[1;32m✔ Jarvis restart complete.\x1b[0m");
}

pub async fn run_command_headless(
    prompt: &str,
    speak_reply: bool,
    settings: &Settings,
) -> Result<()> {
    info!("Headless command received: \"{prompt}\"");
    set_processing(prompt);

    let agent = JarvisAgent::new(settings);
    let reply_res = agent.process_prompt(prompt).await;

    match reply_res {
        Ok(reply) => {
            if TaskNotifier::global().is_active() {
                TaskNotifier::global().finish_smart(&reply).await;
            }
            clear_busy();
            info!("Jarvis reply: \"{reply}\"");
            println!("\x1b[1;36m󰚩 Jarvis:\x1b[0m {reply}");

            if speak_reply {
                let tts = TextToSpeech::from_settings(settings);
                tts.speak(&reply).await?;
            }
            Ok(())
        }
        Err(e) => {
            TaskNotifier::global().cancel().await;
            clear_busy();
            Err(e)
        }
    }
}

pub async fn run_voice_activation(
    speak_reply: bool,
    is_daemon: bool,
    cancel_event: Arc<AtomicBool>,
    force_submit_event: Arc<AtomicBool>,
    settings: &Settings,
) -> Result<()> {
    let hud = Arc::new(JarvisHUD::new());
    let recorder = AudioRecorder::from_settings(settings);
    let stt = SpeechToText::from_settings(settings);
    let agent = JarvisAgent::with_hud(settings, hud.clone());
    let tts = if speak_reply {
        Some(TextToSpeech::from_settings(settings))
    } else {
        None
    };

    let session_path = pid_file_path();
    if !is_daemon {
        if let Some(pid) = read_pid(&session_path) {
            if is_pid_running(pid) && pid != std::process::id() as i32 {
                warn!("Another standalone session is already running (PID: {pid}).");
                return Ok(());
            }
        }
        write_pid(&session_path)?;
    }

    let mut is_first_turn = true;

    while !cancel_event.load(Ordering::SeqCst) {
        // Step 1: Open animated transparent wave HUD in Listening state
        let caption = if is_first_turn {
            "Listening..."
        } else {
            "Listening... (or say 'That's it')"
        };

        set_recording(caption);
        hud.show_listening(Some(caption)).await;

        let initial_timeout = if is_first_turn {
            settings.initial_listen_timeout
        } else {
            settings.followup_listen_timeout
        };

        // Step 2: Record speech with real-time volume callback
        let mut last_vol_time = Instant::now();

        let wav_bytes = recorder
            .record_phrase(
                Some(move |volume| {
                    if last_vol_time.elapsed() >= Duration::from_millis(50) {
                        last_vol_time = Instant::now();
                        let vol = volume;
                        tokio::spawn(async move {
                            let h = JarvisHUD::new();
                            h.update_volume(vol).await;
                        });
                    }
                }),
                Some(initial_timeout),
                Some(force_submit_event.clone()),
            )
            .await?;

        if cancel_event.load(Ordering::SeqCst) {
            info!("Voice activation cancelled during recording.");
            break;
        }

        if wav_bytes.is_empty() {
            if is_first_turn {
                info!("No initial audio detected.");
            } else {
                info!("Follow-up silence timeout elapsed. Concluding conversation session.");
            }
            break;
        }

        // Step 3: Transition HUD to Thinking state & transcribe
        set_processing("Transcribing...");
        hud.show_thinking(Some("Transcribing...")).await;

        let transcript = stt.transcribe(&wav_bytes).await?;

        if cancel_event.load(Ordering::SeqCst) {
            break;
        }

        if transcript.trim().is_empty() {
            if is_first_turn {
                let msg = "I didn't catch that, sir.";
                set_speaking(msg);
                hud.show_speaking(Some(msg)).await;
                if let Some(ref t) = tts {
                    let _ = t.speak(msg).await;
                }
            }
            break;
        }

        info!("User voice transcript: \"{transcript}\"");
        println!("\x1b[1;32mUser:\x1b[0m \"{transcript}\"");
        set_processing(&transcript);
        hud.show_thinking(Some(&format!("\"{transcript}\""))).await;

        // Step 4: Check for exit / dismissal phrase
        if is_exit_command(&transcript) {
            clear_busy();
            let farewell = "Very well, sir. Have a wonderful day.";
            println!("\x1b[1;36m󰚩 Jarvis:\x1b[0m {farewell}");
            set_speaking(farewell);
            hud.show_speaking(Some(farewell)).await;
            if let Some(ref t) = tts {
                let _ = t.speak(farewell).await;
            }
            break;
        }

        // Step 5: Process through Gemini Flash agent
        if cancel_event.load(Ordering::SeqCst) {
            TaskNotifier::global().cancel().await;
            clear_busy();
            break;
        }

        let reply_res = agent.process_prompt(&transcript).await;

        if cancel_event.load(Ordering::SeqCst) {
            TaskNotifier::global().cancel().await;
            clear_busy();
            break;
        }

        let reply = match reply_res {
            Ok(r) => {
                if TaskNotifier::global().is_active() {
                    TaskNotifier::global().finish_smart(&r).await;
                }
                clear_busy();
                r
            }
            Err(e) => {
                TaskNotifier::global().cancel().await;
                clear_busy();
                return Err(e);
            }
        };

        info!("Jarvis reply: \"{reply}\"");
        println!("\x1b[1;36m󰚩 Jarvis:\x1b[0m {reply}");

        // Step 6: Speak reply
        if !reply.is_empty() {
            set_speaking(&reply);
            hud.show_speaking(Some(&reply)).await;
            if let Some(ref t) = tts {
                let _ = t.speak(&reply).await;
            }
        }

        if agent.is_session_ended() {
            info!("Agent marked session as completed.");
            break;
        }

        is_first_turn = false;
        sleep(Duration::from_millis(50)).await;
    }

    // Cleanup session
    TaskNotifier::global().cancel().await;
    clear_busy();
    hud.hide().await;
    if !is_daemon {
        remove_file_if_exists(&session_path);
    }

    let daemon_alive = read_pid(&daemon_pid_file_path())
        .map(is_pid_running)
        .unwrap_or(false);
    set_idle(daemon_alive);

    Ok(())
}

pub async fn run_daemon(speak_reply: bool, settings: &Settings) -> Result<()> {
    let pid_path = daemon_pid_file_path();
    if let Some(pid) = read_pid(&pid_path) {
        if is_pid_running(pid) && pid != std::process::id() as i32 {
            println!("\x1b[33mJarvis daemon is already running (PID: {pid}).\x1b[0m");
            return Ok(());
        }
    }
    write_pid(&pid_path)?;

    update_status(|s| {
        s.daemon_running = true;
        s.state = "wakeword".to_string();
        s.wakeword_enabled = true;
    });

    let mut detector = WakeWordDetector::new(settings);
    let mut signal_handler = SignalHandler::new()?;

    let stop_signal = Arc::new(AtomicBool::new(false));
    let pause_signal = Arc::new(AtomicBool::new(false));
    let manual_trigger = Arc::new(AtomicBool::new(false));

    let cancel_event = Arc::new(AtomicBool::new(false));
    let force_submit = Arc::new(AtomicBool::new(false));

    println!(
        "\x1b[1;36m󰚩 Jarvis Daemon active.\x1b[0m Listening for \x1b[1;32m'{}'\x1b[0m in background...",
        detector.name()
    );

    // Spawn signal processing background task
    let sig_stop = stop_signal.clone();
    let sig_force = force_submit.clone();
    let sig_cancel = cancel_event.clone();
    let sig_manual = manual_trigger.clone();

    tokio::spawn(async move {
        while let Some(sig) = signal_handler.recv().await {
            match sig {
                DaemonSignal::Shutdown => {
                    info!("Daemon received shutdown signal.");
                    sig_stop.store(true, Ordering::SeqCst);
                    break;
                }
                DaemonSignal::TogglePtt => {
                    let st = read_status();
                    if st.state == "recording" {
                        info!("SIGUSR2: Finalizing recording.");
                        sig_force.store(true, Ordering::SeqCst);
                    } else if st.state == "processing" || st.state == "speaking" {
                        info!("SIGUSR2: Interrupting active turn.");
                        sig_cancel.store(true, Ordering::SeqCst);
                        let _ = Command::new("pkill").arg("-9").arg("pw-play").output();
                        let h = JarvisHUD::new();
                        h.hide().await;
                    } else {
                        info!("SIGUSR2: Triggering voice activation.");
                        sig_manual.store(true, Ordering::SeqCst);
                    }
                }
                DaemonSignal::ForceSubmit => {
                    info!("SIGUSR1: Force submit recording.");
                    sig_force.store(true, Ordering::SeqCst);
                }
                DaemonSignal::Interrupt => {
                    info!("SIGHUP: Emergency interrupt.");
                    sig_cancel.store(true, Ordering::SeqCst);
                    let _ = Command::new("pkill").arg("-9").arg("pw-play").output();
                    let h = JarvisHUD::new();
                    h.hide().await;
                    set_idle(true);
                }
            }
        }
    });

    let settings_clone = settings.clone();
    let voice_cancel = cancel_event.clone();
    let voice_force = force_submit.clone();

    // Run wake word listener loop
    detector
        .listen_loop(
            move || {
                let s = settings_clone.clone();
                let c = voice_cancel.clone();
                let f = voice_force.clone();
                async move {
                    let st = read_status();
                    if !st.wakeword_enabled {
                        debug!("Wake word detected but disabled in settings. Ignoring.");
                        return;
                    }

                    let ww_name = &s.wakeword_name;
                    info!("󰚩 Wake word '{ww_name}' activated!");
                    c.store(false, Ordering::SeqCst);
                    f.store(false, Ordering::SeqCst);

                    let _ = play_wake_chime().await;

                    if let Err(e) = run_voice_activation(speak_reply, true, c, f, &s).await {
                        warn!("Voice interaction turn encountered error: {e}");
                    }

                    set_idle(true);
                }
            },
            stop_signal,
            pause_signal,
            manual_trigger,
        )
        .await;

    remove_file_if_exists(&pid_path);
    update_status(|s| {
        s.daemon_running = false;
        s.state = "idle".to_string();
        s.mic_active = false;
    });

    Ok(())
}

fn show_logs(follow: bool) -> Result<()> {
    let log_path = crate::core::logging::get_log_file_path();
    if !log_path.exists() {
        println!(
            "\x1b[33mNo log file found at {}.\x1b[0m",
            log_path.display()
        );
        println!("Start the daemon or execute an action with Jarvis to generate activity logs.");
        return Ok(());
    }

    if follow {
        println!(
            "\x1b[1;36m󰚩 Following Jarvis live activity logs ({})\x1b[0m",
            log_path.display()
        );
        println!("\x1b[2mPress Ctrl+C to stop following.\x1b[0m\n");
        let _ = Command::new("tail")
            .arg("-n")
            .arg("40")
            .arg("-f")
            .arg(&log_path)
            .status();
    } else {
        println!("\x1b[1;36m󰚩 Jarvis Activity History (last 40 entries):\x1b[0m");
        println!("\x1b[2mLocation: {}\x1b[0m\n", log_path.display());
        let _ = Command::new("tail")
            .arg("-n")
            .arg("40")
            .arg(&log_path)
            .status();
        println!("\n\x1b[2mTip: Run 'jarvis -l -f' to follow logs in real time, or inspect with 'less +G {}'.\x1b[0m", log_path.display());
    }
    Ok(())
}

pub async fn run_cli(args: CliArgs, settings: Settings) -> Result<()> {
    if args.logs_path {
        println!("{}", crate::core::logging::get_log_file_path().display());
        return Ok(());
    }

    if args.logs || args.follow {
        show_logs(args.follow)?;
        return Ok(());
    }

    if args.quit {
        quit_all();
        crate::ui::send_desktop_notification(
            "󰚩",
            "Jarvis Terminated",
            "Background daemon and HUD stopped.",
            2500,
            "normal",
        )
        .await;
        return Ok(());
    }

    if args.restart {
        restart_all();
        crate::ui::send_desktop_notification(
            "󰚩",
            "Jarvis Restarted",
            "Daemon restarted and menubar reloaded.",
            3000,
            "normal",
        )
        .await;
        return Ok(());
    }

    if args.kill {
        let killed = stop_running_session();
        if killed {
            println!("\x1b[1;31m󰚩 Jarvis stopped and silenced.\x1b[0m");
            crate::ui::send_desktop_notification(
                "󰚩",
                "Jarvis Stopped",
                "Active session terminated and audio silenced.",
                2500,
                "normal",
            )
            .await;
        } else {
            println!("\x1b[2mNo active Jarvis session found to stop.\x1b[0m");
        }
        return Ok(());
    }

    if args.wakeword_toggle {
        let st = read_status();
        let new_val = !st.wakeword_enabled;
        update_status(|s| {
            s.wakeword_enabled = new_val;
        });
        let status_str = if new_val { "ENABLED" } else { "DISABLED" };
        println!("\x1b[1;36m󰚩 Jarvis Wake Word:\x1b[0m \x1b[1m{status_str}\x1b[0m");
        if new_val {
            crate::ui::send_desktop_notification(
                "󰚩",
                "Jarvis Wake Word: Enabled",
                "Listening in background for 'Hey Jarvis'...",
                3000,
                "normal",
            )
            .await;
        } else {
            crate::ui::send_desktop_notification(
                "󰚩",
                "Jarvis Wake Word: Disabled",
                "Microphone listening paused.",
                3000,
                "normal",
            )
            .await;
        }
        return Ok(());
    }

    if args.wakeword_status {
        let st = read_status();
        let enabled = st.wakeword_enabled;
        let daemon_on = st.daemon_running;
        let status_str = if enabled {
            "\x1b[1;32mON\x1b[0m"
        } else {
            "\x1b[1;31mOFF\x1b[0m"
        };
        let daemon_str = if daemon_on {
            "\x1b[1;32mRunning\x1b[0m"
        } else {
            "\x1b[2mStopped\x1b[0m"
        };
        println!("Wake Word: {status_str} | Daemon: {daemon_str}");
        return Ok(());
    }

    if args.confirm_tui {
        let title = args
            .confirm_title
            .as_deref()
            .unwrap_or("Jarvis Confirmation");
        let prompt = args
            .confirm_prompt
            .as_deref()
            .unwrap_or("Are you sure you want to proceed?");
        let result_file = args.confirm_result_file.as_deref();
        let timeout = args.confirm_timeout.unwrap_or(15);
        crate::ui::confirmation::run_confirm_tui(title, prompt, result_file, timeout).await;
        return Ok(());
    }

    if args.daemon {
        return run_daemon(!args.no_speech, &settings).await;
    }

    if args.stop_recording {
        let daemon_path = daemon_pid_file_path();
        if let Some(pid) = read_pid(&daemon_path) {
            if is_pid_running(pid) {
                let _ = kill(Pid::from_raw(pid), Signal::SIGUSR1);
                return Ok(());
            }
        }
        let session_path = pid_file_path();
        if let Some(pid) = read_pid(&session_path) {
            if is_pid_running(pid) {
                let _ = kill(Pid::from_raw(pid), Signal::SIGUSR1);
            }
        }
        return Ok(());
    }

    if args.trigger {
        let daemon_path = daemon_pid_file_path();
        if let Some(pid) = read_pid(&daemon_path) {
            if is_pid_running(pid) && pid != std::process::id() as i32 {
                let _ = kill(Pid::from_raw(pid), Signal::SIGUSR2);
                return Ok(());
            }
        }

        let cancel_event = Arc::new(AtomicBool::new(false));
        let force_submit = Arc::new(AtomicBool::new(false));
        return run_voice_activation(
            !args.no_speech,
            false,
            cancel_event,
            force_submit,
            &settings,
        )
        .await;
    }

    if let Some(cmd) = args.command {
        return run_command_headless(&cmd, !args.no_speech, &settings).await;
    }

    // Workflows
    if args.workflow_list {
        let hyprland = Arc::new(HyprlandController::new());
        let mgr = WorkflowManager::new(hyprland);
        println!("{}", mgr.list_workflows()?);
        return Ok(());
    }

    if let Some(name) = args.workflow_show {
        let hyprland = Arc::new(HyprlandController::new());
        let mgr = WorkflowManager::new(hyprland);
        println!("{}", mgr.get_workflow_details(&name)?);
        return Ok(());
    }

    if let Some(name) = args.workflow_capture {
        let hyprland = Arc::new(HyprlandController::new());
        let mgr = WorkflowManager::new(hyprland);
        let res = mgr.capture_current_setup(&name, "", None).await?;
        println!("\x1b[1;32m󰚩 Workflow Captured:\x1b[0m\n{res}");
        return Ok(());
    }

    if let Some(name) = args.workflow_delete {
        let hyprland = Arc::new(HyprlandController::new());
        let mgr = WorkflowManager::with_settings(hyprland, Some(settings.clone()));
        let res = mgr.delete_workflow(&name).await?;
        println!("\x1b[1;33m󰚩 Workflow Manager:\x1b[0m {res}");
        return Ok(());
    }

    if let Some(name) = args.workflow_launch {
        let hyprland = Arc::new(HyprlandController::new());
        let mgr = WorkflowManager::new(hyprland);
        let res = mgr.launch_workflow(&name).await?;
        println!("\x1b[1;32m󰚩 Workflow Manager:\x1b[0m {res}");
        crate::ui::send_desktop_notification(
            "󰌨",
            "Workflow Activated",
            &format!("Switched to '{name}' environment"),
            3000,
            "normal",
        )
        .await;
        return Ok(());
    }

    // Status display
    println!("\x1b[1;36m󰚩 Omarchy Jarvis\x1b[0m v0.2.0 (100% Rust Native)");
    println!("\x1b[2mArchitecture:\x1b[0m     x86_64-unknown-linux-gnu");
    println!(
        "\x1b[2mAI Provider:\x1b[0m      \x1b[1m{}\x1b[0m",
        settings.ai_provider
    );
    println!("\x1b[2mReasoning Model:\x1b[0m  {}", settings.model_name);
    println!(
        "\x1b[2mWake Word:\x1b[0m        '{}' (threshold: {:.2})",
        settings.wakeword_name, settings.wakeword_threshold
    );
    println!(
        "\x1b[2mEditor / Terminal:\x1b[0m {} via {}",
        settings.editor, settings.terminal
    );
    println!(
        "\x1b[2mSTT Engine:\x1b[0m       {} ({})",
        settings.stt_engine, settings.whisper_model
    );
    println!(
        "\x1b[2mTTS Engine:\x1b[0m       {} ({})",
        settings.tts_engine,
        if settings.tts_engine == "edge" {
            &settings.edge_voice
        } else {
            &settings.piper_voice
        }
    );

    let active_key_status = match settings.require_active_provider_key() {
        Ok(_) => "\x1b[32mConfigured\x1b[0m",
        Err(_) => "\x1b[33mMissing (configure in ~/.config/jarvis/config.toml or .env)\x1b[0m",
    };
    println!("\x1b[2mActive API Key:\x1b[0m   {active_key_status}");

    let groq_status = if settings.groq_api_key.is_some() {
        "\x1b[32mConfigured\x1b[0m"
    } else {
        "\x1b[33mMissing (set GROQ_API_KEY for fast Whisper)\x1b[0m"
    };
    println!("\x1b[2mGroq Whisper Key:\x1b[0m {groq_status}");

    let st = read_status();
    let daemon_str = if st.daemon_running {
        "\x1b[32mActive\x1b[0m"
    } else {
        "\x1b[2mInactive\x1b[0m"
    };
    println!("\x1b[2mBackground Daemon:\x1b[0m {daemon_str}");

    Ok(())
}
