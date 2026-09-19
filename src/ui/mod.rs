pub mod confirmation;
pub mod hud;
pub mod notification;
pub mod signals;
pub mod theme;

pub use confirmation::{
    parse_voice_confirmation, request_confirmation, run_confirm_tui, ConfirmationRequest,
};
pub use hud::JarvisHUD;
pub use notification::{
    send_desktop_notification, tool_notification_info, TaskNotifier, ToolNotificationInfo,
};
pub use signals::{DaemonSignal, SignalHandler};
pub use theme::{load_omarchy_theme, ThemeColors};
