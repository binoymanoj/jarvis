use jarvis::ai::fallback::{FallbackCoordinator, DEFAULT_FALLBACK_MODELS};
use jarvis::ai::{is_exit_command, JarvisAgent};
use jarvis::core::config::Settings;
use jarvis::tools::build_tool_registry;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[test]
fn test_is_exit_command_variations() {
    let exits = [
        "done",
        "im done",
        "I am done",
        "all done",
        "we are done",
        "were done",
        "thats it",
        "that is it",
        "thats all",
        "that is all",
        "that will be all",
        "thatll be all",
        "thats all for now",
        "that is all for now",
        "thats it for now",
        "thank you thats it",
        "thanks thats it",
        "thank you thats all",
        "thats it thank you",
        "thats it thanks",
        "thats all thank you",
        "thats all thanks",
        "no thats it",
        "no that is it",
        "nothing else",
        "nothing else thanks",
        "nothing else thank you",
        "nothing",
        "no thanks",
        "no thank you",
        "bye",
        "goodbye",
        "bye jarvis",
        "goodbye jarvis",
        "stop",
        "stop listening",
        "exit",
        "quit",
        "dismiss",
        "never mind",
        "nevermind",
        "cancel",
        "shut down",
        "go to sleep",
        "close",
        "close jarvis",
    ];

    for phrase in exits {
        assert!(
            is_exit_command(phrase),
            "Expected '{phrase}' to be detected as exit command"
        );
    }

    let non_exits = [
        "open youtube",
        "what is the battery level",
        "turn up the volume",
        "switch to workspace 2",
        "send an email to Alice",
        "write a note",
    ];

    for phrase in non_exits {
        assert!(
            !is_exit_command(phrase),
            "Expected '{phrase}' NOT to be detected as exit command"
        );
    }
}

#[test]
fn test_fallback_coordinator_full_cycle() {
    let mut coord = FallbackCoordinator::new(Some("gemini-3.5-flash-lite"), None);
    assert_eq!(coord.current_model(), "gemini-3.5-flash-lite");

    let total = coord.all_models().len();
    assert!(total >= DEFAULT_FALLBACK_MODELS.len());

    let mut visited = vec![coord.current_model().to_string()];
    while let Some(next) = coord.next_fallback() {
        visited.push(next.to_string());
    }

    assert_eq!(visited.len(), total);
    assert_eq!(coord.next_fallback(), None);

    coord.reset();
    assert_eq!(coord.current_model(), "gemini-3.5-flash-lite");
}

#[tokio::test]
async fn test_agent_initialization_and_context() {
    let settings = Settings::default();
    let agent = JarvisAgent::new(&settings);

    let context = agent.get_system_context().await;
    assert!(context.contains("CurrentTime="));
    assert!(context.contains("Workspace="));

    // Fast exit check through agent
    let res = agent.process_prompt("That's it, thank you").await.unwrap();
    assert_eq!(res, "Very well, sir. Have a wonderful day.");
    assert!(agent.is_session_ended());

    agent.reset_session().await;
    assert!(!agent.is_session_ended());
}

#[test]
fn test_gemini_tools_declarations_against_registry() {
    let settings = Settings::default();
    let flag = Arc::new(AtomicBool::new(false));
    let reg = build_tool_registry(&settings, flag);

    let declarations = reg.gemini_function_declarations();
    let list = declarations
        .as_array()
        .expect("Should be array of tool groups");
    assert_eq!(list.len(), 1);

    let decls = list[0]["functionDeclarations"]
        .as_array()
        .expect("Should have functionDeclarations");
    assert_eq!(decls.len(), 56);

    for decl in decls {
        assert!(decl["name"].is_string());
        assert!(decl["description"].is_string());
        assert!(decl["parameters"]["type"].is_string());
    }
}
