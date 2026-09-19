use crate::ai::stt::SpeechToText;
use crate::audio::recorder::AudioRecorder;
use crate::audio::tts::TextToSpeech;
use crate::core::config::Settings;
use crate::ui::hud::JarvisHUD;
use crate::ui::theme::{load_omarchy_theme, ThemeColors};
use std::fs;
use std::io::{stdout, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::{debug, info, warn};

#[derive(Debug, Clone)]
pub struct ConfirmationRequest<'a> {
    pub title: &'a str,
    pub prompt: &'a str,
    pub spoken_prompt: &'a str,
    pub icon_name: &'a str,
    pub timeout_secs: u32,
}

impl<'a> Default for ConfirmationRequest<'a> {
    fn default() -> Self {
        Self {
            title: "Jarvis Confirmation",
            prompt: "Are you sure you want to proceed?",
            spoken_prompt: "Are you sure you want to proceed? Please confirm: yes or no.",
            icon_name: "dialog-question",
            timeout_secs: 15,
        }
    }
}

pub fn hex_to_rgb(hex: &str) -> (u8, u8, u8) {
    let clean = hex.trim_start_matches('#');
    if clean.len() >= 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(200);
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(200);
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(200);
        (r, g, b)
    } else {
        (200, 200, 200)
    }
}

pub fn fg_rgb(rgb: (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{};{};{}m", rgb.0, rgb.1, rgb.2)
}

pub fn bg_rgb(rgb: (u8, u8, u8)) -> String {
    format!("\x1b[48;2;{};{};{}m", rgb.0, rgb.1, rgb.2)
}

/// Parses voice transcript into Some(true) for affirmative,
/// Some(false) for cancellation/negative, or None for ambiguous/unrelated.
pub fn parse_voice_confirmation(transcript: &str) -> Option<bool> {
    let clean: String = transcript
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase();
    let words: Vec<&str> = clean.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }

    // Check negative / cancellation keywords first for safety
    const NEGATIVES: &[&str] = &[
        "no",
        "nope",
        "nah",
        "cancel",
        "cancelled",
        "abort",
        "aborted",
        "stop",
        "dont",
        "do not",
        "never",
        "nevermind",
        "never mind",
        "negative",
        "wait",
        "hold",
        "deny",
        "refuse",
        "not now",
    ];

    for neg in NEGATIVES {
        if words.contains(neg) || clean.contains(neg) {
            return Some(false);
        }
    }

    // Check affirmative / confirmation keywords
    const AFFIRMATIVES: &[&str] = &[
        "yes",
        "yeah",
        "yep",
        "yup",
        "sure",
        "confirm",
        "confirmed",
        "proceed",
        "do it",
        "go ahead",
        "affirmative",
        "ok",
        "okay",
        "please do",
        "shut down",
        "shutdown",
        "reboot",
        "restart",
        "log out",
        "logout",
    ];

    for pos in AFFIRMATIVES {
        if words.contains(pos) || clean.contains(pos) {
            return Some(true);
        }
    }

    None
}

/// Calculates visible display width of an ANSI-colored string (ignoring escape sequences).
pub fn visible_len(s: &str) -> usize {
    let mut count = 0;
    let mut in_esc = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_esc = true;
        } else if in_esc {
            if c.is_ascii_alphabetic() {
                in_esc = false;
            }
        } else {
            count += 1;
        }
    }
    count
}

/// Computes readable foreground (either dark background or crisp white) based on background luminance.
pub fn best_contrast_fg(bg: (u8, u8, u8), dark: (u8, u8, u8)) -> (u8, u8, u8) {
    let lum = 0.299 * bg.0 as f32 + 0.587 * bg.1 as f32 + 0.114 * bg.2 as f32;
    if lum > 140.0 {
        dark
    } else {
        (255, 255, 255)
    }
}

/// Wraps text cleanly at word boundaries up to `max_width`.
pub fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in words {
        if cur.is_empty() {
            cur.push_str(word);
        } else if cur.chars().count() + 1 + word.chars().count() <= max_width {
            cur.push(' ');
            cur.push_str(word);
        } else {
            lines.push(cur);
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

/// Renders the styled confirmation TUI box matching the active Omarchy theme.
fn render_tui_dialog(
    title: &str,
    prompt: &str,
    selected_yes: bool,
    remaining_secs: u32,
    theme: &ThemeColors,
) {
    let c_accent = hex_to_rgb(&theme.accent);
    let c_fg = hex_to_rgb(&theme.foreground);
    let c_border = hex_to_rgb(&theme.active_border);
    let c_muted = hex_to_rgb(&theme.muted);
    let c_cyan = hex_to_rgb(&theme.cyan);
    let c_dark = hex_to_rgb(&theme.dark_background);
    let c_selection = hex_to_rgb(&theme.selection);
    let c_red = hex_to_rgb(&theme.bright_red);

    let reset = "\x1b[0m";
    let bold = "\x1b[1m";

    let col_border = fg_rgb(c_border);
    let col_accent = fg_rgb(c_accent);
    let col_fg = fg_rgb(c_fg);
    let col_muted = fg_rgb(c_muted);
    let col_cyan = fg_rgb(c_cyan);

    let w: usize = 60;
    let row = |content: &str| {
        let vis = visible_len(content);
        let pad = " ".repeat(w.saturating_sub(vis));
        println!(
            "  {}│{} {}{} {}│{}",
            col_border, reset, content, pad, col_border, reset
        );
    };

    // Clear screen and reset cursor position to top
    print!("\x1b[H\x1b[2J");
    println!();
    println!("  {}╭{}╮{}", col_border, "─".repeat(w + 2), reset);

    // 1. Header: Centered Title with Robot Icon
    let title_upper = title.to_uppercase();
    let title_clean: String = title_upper.chars().take(44).collect();
    let title_line = format!("{col_accent}{bold}󰚩  {title_clean}{reset}");
    let t_pad = (w.saturating_sub(visible_len(&title_line))) / 2;
    row(&format!("{}{title_line}", " ".repeat(t_pad)));

    // Empty spacing row
    row("");

    // 2. Prompt Text: Word-wrapped and centered
    let prompt_lines = wrap_text(prompt, 54);
    for pline in prompt_lines.iter().take(3) {
        let p_pad = (w.saturating_sub(pline.chars().count())) / 2;
        row(&format!(
            "{}{col_fg}{bold}{pline}{reset}",
            " ".repeat(p_pad)
        ));
    }

    // Empty spacing row
    row("");

    // 3. Interactive Pill Buttons with smart contrast
    let yes_fg = best_contrast_fg(c_accent, c_dark);
    let red_fg = best_contrast_fg(c_red, c_dark);

    let (b1, b2) = if selected_yes {
        (
            format!(
                "{bold}{}{}[  ✔  Yes, proceed (y)  ]{reset}",
                bg_rgb(c_accent),
                fg_rgb(yes_fg)
            ),
            format!(
                "{}{}[  ✖  Cancel (n)  ]{reset}",
                bg_rgb(c_selection),
                col_muted
            ),
        )
    } else {
        (
            format!(
                "{}{}[  ✔  Yes, proceed (y)  ]{reset}",
                bg_rgb(c_selection),
                col_muted
            ),
            format!(
                "{bold}{}{}[  ✖  Cancel (n)  ]{reset}",
                bg_rgb(c_red),
                fg_rgb(red_fg)
            ),
        )
    };

    let btn_line = format!("{b1}    {b2}");
    let b_pad = (w.saturating_sub(visible_len(&btn_line))) / 2;
    row(&format!("{}{btn_line}", " ".repeat(b_pad)));

    // Empty spacing row
    row("");

    // Divider line
    println!("  {}├{}┤{}", col_border, "─".repeat(w + 2), reset);

    // 4. Voice status line
    row(&format!(
        "   {col_cyan}󰍬  Listening for voice: \"yes\" / \"no\"{reset}"
    ));

    // 5. Timer & Keybindings line
    row(&format!(
        "   {col_muted}⏱  Auto-cancelling in {:2}s  •  [y/n]  •  [Esc] Abort{reset}",
        remaining_secs
    ));

    println!("  {}╰{}╯{}", col_border, "─".repeat(w + 2), reset);
    let _ = stdout().flush();
}

/// Runs the interactive terminal-based confirmation popup window.
pub async fn run_confirm_tui(
    title: &str,
    prompt: &str,
    result_file: Option<&str>,
    timeout_secs: u32,
) {
    let is_tty = std::io::stdin().is_terminal();
    if is_tty {
        let _ = std::process::Command::new("stty")
            .args(["cbreak", "-echo"])
            .stderr(std::process::Stdio::null())
            .status();
    }

    // Hide cursor
    print!("\x1b[?25l");
    let _ = stdout().flush();

    let theme = load_omarchy_theme();
    let mut selected_yes = true;
    let start_time = Instant::now();
    let mut remaining_secs = timeout_secs;
    let mut confirmed = false;

    render_tui_dialog(title, prompt, selected_yes, remaining_secs, &theme);

    let mut stdin_reader = tokio::io::stdin();
    let mut buf = [0u8; 16];

    loop {
        tokio::select! {
            res = stdin_reader.read(&mut buf) => {
                match res {
                    Ok(0) => break,
                    Ok(n) => {
                        let b = buf[0];
                        if b == b'y' || b == b'Y' {
                            confirmed = true;
                            break;
                        } else if b == b'n' || b == b'N' || (b == 27 && n == 1) || b == b'q' || b == b'Q' {
                            confirmed = false;
                            break;
                        } else if b == b'\n' || b == b'\r' || b == b' ' {
                            confirmed = selected_yes;
                            break;
                        } else if b == b'\t' {
                            selected_yes = !selected_yes;
                            render_tui_dialog(title, prompt, selected_yes, remaining_secs, &theme);
                        } else if n >= 3 && buf[0] == 27 && buf[1] == b'[' {
                            if buf[2] == b'D' { // Left Arrow
                                selected_yes = true;
                                render_tui_dialog(title, prompt, selected_yes, remaining_secs, &theme);
                            } else if buf[2] == b'C' { // Right Arrow
                                selected_yes = false;
                                render_tui_dialog(title, prompt, selected_yes, remaining_secs, &theme);
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(150)) => {
                let elapsed = start_time.elapsed().as_secs() as u32;
                if elapsed >= timeout_secs {
                    confirmed = false;
                    break;
                }
                let rem = timeout_secs.saturating_sub(elapsed);
                if rem != remaining_secs {
                    remaining_secs = rem;
                    render_tui_dialog(title, prompt, selected_yes, remaining_secs, &theme);
                }
            }
        }
    }

    // Restore terminal settings and show cursor
    if is_tty {
        let _ = std::process::Command::new("stty")
            .args(["-cbreak", "echo"])
            .stderr(std::process::Stdio::null())
            .status();
    }
    println!("\x1b[?25h\x1b[0m");
    let _ = stdout().flush();

    if let Some(file_path) = result_file {
        let _ = fs::write(file_path, if confirmed { "yes" } else { "no" });
    }
    std::process::exit(0);
}

/// Spawns a themed floating terminal popup confirmation with simultaneous voice recognition.
pub async fn request_confirmation(
    req: &ConfirmationRequest<'_>,
    settings: Option<&Settings>,
) -> bool {
    info!(
        "Requesting action confirmation: title='{}', prompt='{}'",
        req.title, req.prompt
    );

    let hud = JarvisHUD::new();
    hud.show_thinking(Some(&format!("Confirmation Required: {}", req.prompt)))
        .await;

    let theme = load_omarchy_theme();

    // 1. Generate unique temporary result file for the popup terminal
    let temp_dir = std::env::temp_dir();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let result_file = temp_dir.join(format!(
        "jarvis_confirm_{}_{}.txt",
        std::process::id(),
        now_ms
    ));
    let result_file_str = result_file.to_string_lossy().to_string();

    // Clean up any stale result file
    let _ = fs::remove_file(&result_file);

    // Locate jarvis binary
    let jarvis_bin = std::env::current_exe().unwrap_or_else(|_| {
        if let Ok(path) = which::which("jarvis") {
            path
        } else {
            let home = std::env::var("HOME").unwrap_or_default();
            let user_bin = PathBuf::from(&home).join(".local/bin/jarvis");
            if user_bin.is_file() {
                user_bin
            } else {
                PathBuf::from("jarvis")
            }
        }
    });

    // 2. Launch floating terminal popup (kitty, foot, or xdg-terminal-exec)
    let kitty_bin = which::which("kitty").ok();
    let foot_bin = which::which("foot").ok();
    let term_exec = which::which("xdg-terminal-exec").ok();

    let mut popup_child = if let Some(ref kitty) = kitty_bin {
        Command::new(kitty)
            .arg("--class")
            .arg("TUI.float")
            .arg("-T")
            .arg(req.title)
            .arg("-o")
            .arg("initial_window_width=640")
            .arg("-o")
            .arg("initial_window_height=300")
            .arg("-o")
            .arg("window_padding_width=18")
            .arg("-o")
            .arg("remember_window_size=no")
            .arg("-o")
            .arg("confirm_os_window_close=0")
            .arg("-o")
            .arg(format!("background={}", theme.dark_background))
            .arg("-o")
            .arg(format!("foreground={}", theme.foreground))
            .arg(&jarvis_bin)
            .arg("--confirm-tui")
            .arg("--confirm-title")
            .arg(req.title)
            .arg("--confirm-prompt")
            .arg(req.prompt)
            .arg("--confirm-result-file")
            .arg(&result_file_str)
            .arg("--confirm-timeout")
            .arg(req.timeout_secs.to_string())
            .spawn()
            .ok()
    } else if let Some(ref foot) = foot_bin {
        Command::new(foot)
            .arg("--app-id")
            .arg("TUI.float")
            .arg("-T")
            .arg(req.title)
            .arg("-w")
            .arg("620x290")
            .arg(&jarvis_bin)
            .arg("--confirm-tui")
            .arg("--confirm-title")
            .arg(req.title)
            .arg("--confirm-prompt")
            .arg(req.prompt)
            .arg("--confirm-result-file")
            .arg(&result_file_str)
            .arg("--confirm-timeout")
            .arg(req.timeout_secs.to_string())
            .spawn()
            .ok()
    } else if let Some(ref exec) = term_exec {
        Command::new(exec)
            .arg("--app-id=TUI.float")
            .arg(&jarvis_bin)
            .arg("--confirm-tui")
            .arg("--confirm-title")
            .arg(req.title)
            .arg("--confirm-prompt")
            .arg(req.prompt)
            .arg("--confirm-result-file")
            .arg(&result_file_str)
            .arg("--confirm-timeout")
            .arg(req.timeout_secs.to_string())
            .spawn()
            .ok()
    } else {
        warn!("No suitable terminal emulator found. Falling back to CLI prompt.");
        None
    };

    // 3. Play spoken prompt if TTS is available, but race against early terminal input
    if let Some(s) = settings {
        let tts = TextToSpeech::from_settings(s);
        let spoken = req.spoken_prompt.to_string();

        if let Some(ref mut child) = popup_child {
            tokio::select! {
                _ = child.wait() => {
                    let confirmed = fs::read_to_string(&result_file)
                        .map(|s| s.trim() == "yes")
                        .unwrap_or(false);
                    info!("Confirmation resolved via early popup click before TTS finished: confirmed={confirmed}");
                    let _ = fs::remove_file(&result_file);
                    hud.hide().await;
                    return confirmed;
                }
                _ = tts.speak(&spoken) => {
                    debug!("Spoken confirmation prompt completed.");
                }
            }
        } else {
            let _ = tts.speak(&spoken).await;
        }
    }

    // 4. Listen for simultaneous voice confirmation while terminal popup remains active
    hud.show_listening(Some("Confirm: Say 'Yes' or 'No'")).await;

    let force_stop = Arc::new(AtomicBool::new(false));
    let (voice_tx, mut voice_rx) = tokio::sync::mpsc::channel::<Option<bool>>(1);

    if let Some(s) = settings {
        let s_clone = s.clone();
        let force_stop_clone = force_stop.clone();

        tokio::spawn(async move {
            let recorder = AudioRecorder::from_settings(&s_clone);
            let wav_bytes = recorder
                .record_phrase(None::<fn(f32)>, Some(7.0), Some(force_stop_clone))
                .await
                .unwrap_or_default();

            if !wav_bytes.is_empty() {
                let stt = SpeechToText::from_settings(&s_clone);
                if let Ok(transcript) = stt.transcribe(&wav_bytes).await {
                    info!("Confirmation voice transcript: \"{transcript}\"");
                    let parsed = parse_voice_confirmation(&transcript);
                    let _ = voice_tx.send(parsed).await;
                    return;
                }
            }
            let _ = voice_tx.send(None).await;
        });
    }

    // Terminal fallback if popup couldn't be spawned
    let is_tty = std::io::stdin().is_terminal();
    if popup_child.is_none() && is_tty {
        print!("\n󰚩 [Jarvis Confirmation] {} (y/N): ", req.prompt);
        let _ = stdout().flush();
    }

    // 5. Concurrent wait on Voice Response OR Terminal Popup Exit OR Overall Timeout
    let mut confirmed = false;

    tokio::select! {
        // Path A: Voice response received
        Some(voice_res) = voice_rx.recv() => {
            if let Some(answer) = voice_res {
                info!("Confirmation decided via Voice: {answer}");
                confirmed = answer;
                // Close terminal popup immediately since voice answered
                if let Some(ref mut child) = popup_child {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                }
            } else {
                info!("Voice was ambiguous or silent. Waiting for terminal dialog.");
                if let Some(ref mut child) = popup_child {
                    let _ = child.wait().await;
                    confirmed = fs::read_to_string(&result_file)
                        .map(|s| s.trim() == "yes")
                        .unwrap_or(false);
                }
            }
        }

        // Path B: User confirmed/cancelled in terminal popup
        _ = async {
            if let Some(ref mut child) = popup_child {
                let _ = child.wait().await;
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            force_stop.store(true, Ordering::SeqCst);
            confirmed = fs::read_to_string(&result_file)
                .map(|s| s.trim() == "yes")
                .unwrap_or(false);
            info!("Confirmation decided via Terminal TUI: confirmed={confirmed}");
        }

        // Path C: Safety timeout
        _ = tokio::time::sleep(Duration::from_secs(req.timeout_secs as u64 + 1)) => {
            info!("Confirmation timed out after {} seconds.", req.timeout_secs);
            force_stop.store(true, Ordering::SeqCst);
            if let Some(ref mut child) = popup_child {
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            confirmed = false;
        }
    }

    let _ = fs::remove_file(&result_file);
    hud.hide().await;
    confirmed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_voice_affirmative() {
        assert_eq!(parse_voice_confirmation("yes"), Some(true));
        assert_eq!(parse_voice_confirmation("Yes please"), Some(true));
        assert_eq!(parse_voice_confirmation("YES"), Some(true));
        assert_eq!(parse_voice_confirmation("yeah, do it"), Some(true));
        assert_eq!(parse_voice_confirmation("yep"), Some(true));
        assert_eq!(parse_voice_confirmation("yup"), Some(true));
        assert_eq!(parse_voice_confirmation("sure"), Some(true));
        assert_eq!(parse_voice_confirmation("confirm"), Some(true));
        assert_eq!(parse_voice_confirmation("confirmed"), Some(true));
        assert_eq!(parse_voice_confirmation("proceed"), Some(true));
        assert_eq!(parse_voice_confirmation("go ahead"), Some(true));
        assert_eq!(parse_voice_confirmation("reboot"), Some(true));
        assert_eq!(parse_voice_confirmation("shut down"), Some(true));
    }

    #[test]
    fn test_parse_voice_negative() {
        assert_eq!(parse_voice_confirmation("no"), Some(false));
        assert_eq!(parse_voice_confirmation("No, cancel that"), Some(false));
        assert_eq!(parse_voice_confirmation("nope"), Some(false));
        assert_eq!(parse_voice_confirmation("nah"), Some(false));
        assert_eq!(parse_voice_confirmation("cancel"), Some(false));
        assert_eq!(parse_voice_confirmation("abort"), Some(false));
        assert_eq!(parse_voice_confirmation("stop"), Some(false));
        assert_eq!(parse_voice_confirmation("don't do it"), Some(false));
        assert_eq!(parse_voice_confirmation("never mind"), Some(false));
        assert_eq!(parse_voice_confirmation("nevermind"), Some(false));
        assert_eq!(parse_voice_confirmation("wait, no"), Some(false));
    }

    #[test]
    fn test_parse_voice_ambiguous() {
        assert_eq!(parse_voice_confirmation(""), None);
        assert_eq!(parse_voice_confirmation("what time is it"), None);
        assert_eq!(parse_voice_confirmation("play music"), None);
    }

    #[test]
    fn test_hex_to_rgb() {
        assert_eq!(hex_to_rgb("#798186"), (121, 129, 134));
        assert_eq!(hex_to_rgb("#101315"), (16, 19, 21));
        assert_eq!(hex_to_rgb("#ffffff"), (255, 255, 255));
    }

    #[test]
    fn test_visible_len() {
        let plain = "Hello, world!";
        assert_eq!(visible_len(plain), 13);

        let colored = "\x1b[1m\x1b[38;2;121;129;134mHello, world!\x1b[0m";
        assert_eq!(visible_len(colored), 13);

        let button = "\x1b[48;2;52;61;65m\x1b[38;2;75;78;85m[  ✖  Cancel (n)  ]\x1b[0m";
        assert_eq!(visible_len(button), 19);
    }

    #[test]
    fn test_best_contrast_fg() {
        // Very light background -> dark foreground
        let light = (240, 240, 240);
        let dark = (12, 14, 16);
        assert_eq!(best_contrast_fg(light, dark), dark);

        // Very dark background -> crisp white foreground
        let deep_dark = (16, 19, 21);
        assert_eq!(best_contrast_fg(deep_dark, dark), (255, 255, 255));
    }

    #[test]
    fn test_wrap_text() {
        let short = "Short prompt";
        assert_eq!(wrap_text(short, 50), vec!["Short prompt"]);

        let long = "Are you sure you want to delete workflow 'record_and_transcribe_audio'?";
        let wrapped = wrap_text(long, 40);
        assert_eq!(wrapped.len(), 2);
        assert_eq!(wrapped[0], "Are you sure you want to delete workflow");
        assert_eq!(wrapped[1], "'record_and_transcribe_audio'?");
    }
}
