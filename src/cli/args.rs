use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "jarvis",
    author = "binoymanoj <binoypm2002@gmail.com>",
    version = "0.2.0",
    about = "Omarchy Jarvis - Intelligent Voice & Desktop Automation Assistant in Rust"
)]
pub struct CliArgs {
    #[arg(
        short = 't',
        long = "trigger",
        help = "Trigger voice listening mode (push-to-talk toggle & interrupt)"
    )]
    pub trigger: bool,

    #[arg(
        short = 's',
        long = "stop-recording",
        help = "Stop recording immediately and process speech (Push-to-Talk key release)"
    )]
    pub stop_recording: bool,

    #[arg(
        short = 'k',
        long = "kill",
        help = "Instantly kill any active voice session, silence audio, and dismiss HUD"
    )]
    pub kill: bool,

    #[arg(
        short = 'q',
        long = "quit",
        help = "Completely stop all Jarvis processes, background daemon, and systemd service"
    )]
    pub quit: bool,

    #[arg(
        short = 'r',
        long = "restart",
        help = "Restart Jarvis systemd daemon and refresh Omarchy menubar plugin"
    )]
    pub restart: bool,

    #[arg(
        short = 'd',
        long = "daemon",
        help = "Run as background daemon continuously listening for 'Hey Jarvis' wake word"
    )]
    pub daemon: bool,

    #[arg(
        short = 'W',
        long = "wakeword-toggle",
        help = "Toggle background wake word detection on or off"
    )]
    pub wakeword_toggle: bool,

    #[arg(
        long = "wakeword-status",
        help = "Print current wake word detection status"
    )]
    pub wakeword_status: bool,

    #[arg(
        short = 'c',
        long = "command",
        help = "Execute a direct text command (headless mode)"
    )]
    pub command: Option<String>,

    #[arg(
        long = "no-speech",
        help = "Suppress spoken audio responses (text-only mode)"
    )]
    pub no_speech: bool,

    #[arg(
        long = "status",
        help = "Check Jarvis daemon status, API keys, and environment"
    )]
    pub status: bool,

    #[arg(
        long = "workflow-list",
        help = "List all configured multi-workspace workflows"
    )]
    pub workflow_list: bool,

    #[arg(
        long = "workflow-show",
        value_name = "NAME",
        help = "Show detailed launch steps of a workflow"
    )]
    pub workflow_show: Option<String>,

    #[arg(
        long = "workflow-capture",
        value_name = "NAME",
        help = "Capture currently open application windows into a new workflow preset"
    )]
    pub workflow_capture: Option<String>,

    #[arg(
        long = "workflow-delete",
        value_name = "NAME",
        help = "Delete a configured workflow preset"
    )]
    pub workflow_delete: Option<String>,

    #[arg(
        long = "workflow-launch",
        value_name = "NAME",
        help = "Launch a workflow preset across Hyprland workspaces"
    )]
    pub workflow_launch: Option<String>,

    #[arg(
        short = 'l',
        long = "logs",
        help = "Display recent Jarvis activity history and logs"
    )]
    pub logs: bool,

    #[arg(
        short = 'f',
        long = "follow",
        help = "Follow log stream in real time (used with -l / --logs)"
    )]
    pub follow: bool,

    #[arg(long = "logs-path", help = "Print the absolute path to jarvis.log")]
    pub logs_path: bool,

    #[arg(
        long = "confirm-tui",
        help = "Run interactive confirmation TUI",
        hide = true
    )]
    pub confirm_tui: bool,

    #[arg(long = "confirm-title", hide = true)]
    pub confirm_title: Option<String>,

    #[arg(long = "confirm-prompt", hide = true)]
    pub confirm_prompt: Option<String>,

    #[arg(long = "confirm-result-file", hide = true)]
    pub confirm_result_file: Option<String>,

    #[arg(long = "confirm-timeout", hide = true)]
    pub confirm_timeout: Option<u32>,
}
