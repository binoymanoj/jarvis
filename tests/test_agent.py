import pytest
from jarvis.core.agent import JarvisAgent, is_exit_command

@pytest.mark.anyio
async def test_agent_initialization():
    agent = JarvisAgent()
    assert agent.hyprland is not None
    assert agent.omarchy is not None
    assert agent.screen is not None
    assert agent.web is not None
    assert agent.session_ended is False

@pytest.mark.anyio
async def test_agent_system_context():
    agent = JarvisAgent()
    context = await agent.get_system_context()
    assert isinstance(context, str)
    assert "Workspace=" in context

def test_is_exit_command_positive():
    exit_phrases = [
        "that's it",
        "That's it.",
        "done",
        "Done.",
        "I'm done",
        "that's all",
        "that will be all",
        "thank you that's it",
        "Thanks, that's all!",
        "goodbye",
        "bye jarvis",
        "stop",
        "dismiss",
        "never mind",
        "cancel",
    ]
    for phrase in exit_phrases:
        assert is_exit_command(phrase) is True, f"Expected '{phrase}' to be recognized as exit command"

def test_is_exit_command_negative():
    normal_queries = [
        "is it done?",
        "what have you done?",
        "open mkbhd latest video on youtube",
        "switch to workspace 2",
        "search google for hyprland bindings",
        "that is a good video",
        "what is the weather like",
    ]
    for phrase in normal_queries:
        assert is_exit_command(phrase) is False, f"Expected '{phrase}' not to be exit command"
