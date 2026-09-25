use chrono::{Datelike, Timelike};
use jarvis::core::config::Settings;
use jarvis::tools::build_tool_registry;
use jarvis::tools::calendar::CalendarManager;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[test]
fn test_all_56_tools_registered() {
    let settings = Settings::default();
    let flag = Arc::new(AtomicBool::new(false));
    let reg = build_tool_registry(&settings, flag);

    assert_eq!(reg.count(), 56);

    let tools = [
        "switch_workspace",
        "focus_application",
        "close_active_window",
        "toggle_layout_split",
        "toggle_fullscreen",
        "adjust_volume",
        "set_brightness",
        "set_theme",
        "get_battery",
        "launch_application",
        "notify",
        "type_text",
        "press_key",
        "send_shortcut",
        "scroll",
        "execute_command",
        "media_play_pause",
        "media_next",
        "media_previous",
        "media_stop",
        "get_now_playing",
        "play_media",
        "resume_media",
        "get_clipboard",
        "set_clipboard",
        "lock_screen",
        "logout_system",
        "reboot_system",
        "shutdown_system",
        "toggle_bluetooth",
        "get_system_stats",
        "network_speedtest",
        "launch_workflow",
        "list_workflows",
        "capture_current_workflow",
        "save_custom_workflow",
        "delete_custom_workflow",
        "get_workflow_details",
        "open_url",
        "search_web",
        "open_youtube",
        "inspect_screen",
        "schedule_event",
        "set_reminder",
        "list_reminders",
        "clear_reminders",
        "draft_email",
        "create_note",
        "list_notes",
        "create_project",
        "delegate_to_antigravity",
        "open_file_in_editor",
        "localsend_share",
        "display_research_in_neovim",
        "create_quick_meeting",
        "dismiss_session",
    ];

    for t in tools {
        let tool = reg.get(t);
        assert!(tool.is_some(), "Tool '{t}' should be registered");
        let schema = tool.unwrap().parameters_schema();
        assert_eq!(schema["type"], "OBJECT", "Tool '{t}' schema must be OBJECT");
    }
}

#[tokio::test]
async fn test_dismiss_session_tool() {
    let flag = Arc::new(AtomicBool::new(false));
    let settings = Settings::default();
    let reg = build_tool_registry(&settings, flag.clone());

    assert!(!flag.load(Ordering::SeqCst));
    let result = reg
        .execute_tool(
            "dismiss_session",
            serde_json::json!({
                "farewell": "Goodbye!"
            }),
        )
        .await
        .unwrap();

    assert_eq!(result, "Goodbye!");
    assert!(flag.load(Ordering::SeqCst));
}

#[test]
fn test_calendar_date_parsing() {
    let dt1 = CalendarManager::parse_event_datetime("tomorrow 3pm");
    assert_eq!(dt1.time().hour(), 15);
    assert_eq!(dt1.time().minute(), 0);

    let dt2 = CalendarManager::parse_event_datetime("today at 10:30 am");
    assert_eq!(dt2.time().hour(), 10);
    assert_eq!(dt2.time().minute(), 30);

    let dt3 = CalendarManager::parse_event_datetime("2026-12-25 18:00");
    assert_eq!(dt3.date().year(), 2026);
    assert_eq!(dt3.date().month(), 12);
    assert_eq!(dt3.date().day(), 25);
    assert_eq!(dt3.time().hour(), 18);
}

#[tokio::test]
async fn test_localsend_share_tool_url() {
    std::env::set_var("JARVIS_TEST_MODE", "1");
    let flag = Arc::new(AtomicBool::new(false));
    let settings = Settings::default();
    let reg = build_tool_registry(&settings, flag);

    // Test missing parameter returns error
    let err_res = reg.execute_tool("localsend_share", serde_json::json!({})).await;
    assert!(err_res.is_err());

    // Test URL sharing without opening GUI
    let res = reg
        .execute_tool(
            "localsend_share",
            serde_json::json!({
                "item": "https://github.com/localsend/localsend"
            }),
        )
        .await;

    assert!(res.is_ok());
    let msg = res.unwrap();
    assert!(msg.contains("LocalSend sending window"));
}

#[tokio::test]
async fn test_display_research_in_neovim_tool() {
    std::env::set_var("JARVIS_TEST_MODE", "1");
    let flag = Arc::new(AtomicBool::new(false));
    let settings = Settings::default();
    let reg = build_tool_registry(&settings, flag);

    // Test missing parameter returns error
    let err_res = reg
        .execute_tool("display_research_in_neovim", serde_json::json!({ "title": "Test" }))
        .await;
    assert!(err_res.is_err());

    // Test research formatting without opening terminal popup
    let res = reg
        .execute_tool(
            "display_research_in_neovim",
            serde_json::json!({
                "title": "Quantum Computing",
                "content": "## Quantum Superposition\nQubits can exist in coherent superpositions of states."
            }),
        )
        .await;

    assert!(res.is_ok());
    let msg = res.unwrap();
    assert!(msg.contains("Quantum Computing"));
}

#[tokio::test]
async fn test_set_and_get_clipboard() {
    let flag = Arc::new(AtomicBool::new(false));
    let settings = Settings::default();
    let reg = build_tool_registry(&settings, flag);

    let res = reg
        .execute_tool(
            "set_clipboard",
            serde_json::json!({ "text": "https://meet.google.com/test-abc-xyz" }),
        )
        .await;
    println!("set_clipboard res: {:?}", res);
    assert!(res.is_ok());

    let get_res = reg.execute_tool("get_clipboard", serde_json::json!({})).await;
    println!("get_clipboard res: {:?}", get_res);
    assert!(get_res.is_ok());
    assert!(get_res.unwrap().contains("test-abc-xyz"));
}

