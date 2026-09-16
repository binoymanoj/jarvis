import pytest
from jarvis.tools.calendar import CalendarManager, parse_event_datetime
from jarvis.tools.email import EmailManager
from jarvis.tools.hyprland import HyprlandController
from jarvis.tools.notes import NoteManager
from jarvis.tools.omarchy import OmarchyBridge
from jarvis.tools.screen import ScreenPerception
from jarvis.tools.web import WebNavigator
from jarvis.tools.workflow import WorkflowManager

@pytest.mark.anyio
async def test_hyprland_controller_init():
    hl = HyprlandController()
    assert hl.sig is not None
    assert hl.cmd_socket_path is not None
    assert hl.cmd_socket_path.exists()

@pytest.mark.anyio
async def test_hyprland_json_query():
    hl = HyprlandController()
    workspaces = await hl.get_workspaces()
    assert isinstance(workspaces, list)

@pytest.mark.anyio
async def test_omarchy_bridge_commands():
    bridge = OmarchyBridge()
    theme = await bridge.get_theme()
    assert isinstance(theme, str)
    assert len(theme) > 0

@pytest.mark.anyio
async def test_screen_perception_init():
    perception = ScreenPerception()
    assert perception.grim_bin is not None
    assert perception.storage_dir.exists()

@pytest.mark.anyio
async def test_web_navigator_init():
    nav = WebNavigator()
    assert nav._browser_bin is not None

def test_web_navigator_resolve_youtube():
    res = WebNavigator._resolve_youtube_video("mkbhd latest video")
    assert res is not None
    video_id, title = res
    assert len(video_id) == 11
    assert "mkbhd" in title.lower() or "iphone" in title.lower() or len(title) > 0

def test_workflow_manager(tmp_path, monkeypatch):
    from jarvis.tools import workflow as wf_module
    test_json = tmp_path / "test_workflows.json"
    monkeypatch.setattr(wf_module, "WORKFLOWS_FILE", test_json)

    wf_mgr = WorkflowManager()
    workflows = wf_mgr.get_workflows()
    assert "coding" in workflows
    assert wf_mgr.find_workflow("dev") is not None
    assert wf_mgr.find_workflow("research") is not None
    listing = wf_mgr.list_workflows()
    assert "Available workflows:" in listing

    # 1. Save custom workflow
    steps = [
        {"workspace": 1, "launch": "code"},
        {"workspace": 2, "launch": "omarchy-launch-browser https://news.ycombinator.com"},
    ]
    res_save = wf_mgr.save_workflow("my_custom_flow", "Custom dev flow", steps, aliases=["custom_flow"], primary_workspace=1)
    assert "Successfully saved" in res_save
    assert wf_mgr.find_workflow("custom_flow") is not None

    # 2. Get details
    details = wf_mgr.get_workflow_details("my_custom_flow")
    assert "### Workflow: my_custom_flow" in details
    assert "Custom dev flow" in details

    # 3. Delete workflow
    res_del = wf_mgr.delete_workflow("my_custom_flow")
    assert "has been deleted" in res_del
    assert wf_mgr.find_workflow("my_custom_flow") is None


@pytest.mark.anyio
async def test_workflow_manager_capture(tmp_path, monkeypatch):
    from jarvis.tools import workflow as wf_module
    from unittest.mock import AsyncMock, MagicMock

    test_json = tmp_path / "test_workflows_capture.json"
    monkeypatch.setattr(wf_module, "WORKFLOWS_FILE", test_json)

    mock_hyprland = MagicMock()
    mock_hyprland.get_clients = AsyncMock(return_value=[
        {"workspace": {"id": 1}, "class": "foot", "initialClass": "foot", "title": "terminal"},
        {"workspace": {"id": 2}, "class": "google-chrome", "initialClass": "google-chrome", "title": "Chrome"},
    ])
    mock_hyprland.get_active_window = AsyncMock(return_value={"workspace": {"id": 1}})

    wf_mgr = WorkflowManager(hyprland=mock_hyprland)
    res = await wf_mgr.capture_current_setup("auto_captured")
    assert "Captured 2 applications" in res
    assert wf_mgr.find_workflow("auto_captured") is not None


def test_calendar_parser():
    dt = parse_event_datetime("tomorrow 3pm")
    assert dt.hour == 15
    assert dt.minute == 0

    dt2 = parse_event_datetime("today 18:30")
    assert dt2.hour == 18
    assert dt2.minute == 30

def test_email_manager():
    email_mgr = EmailManager()
    assert email_mgr._xdg_open_bin is not None

@pytest.mark.anyio
async def test_note_manager(tmp_path):
    note_mgr = NoteManager(notes_dir=tmp_path)
    res = await note_mgr.create_note("Test Idea", "This is an important test note.")
    assert "Created note" in res
    assert (tmp_path / "Test_Idea.md").exists()
    listing = note_mgr.list_recent_notes()
    assert "Test_Idea" in listing

def test_antigravity_manager_resolve_dir(tmp_path):
    from jarvis.tools.antigravity import AntigravityManager
    from pathlib import Path
    mgr = AntigravityManager()
    
    # Custom directory
    custom = mgr.resolve_target_dir("my_app", str(tmp_path))
    assert custom == tmp_path / "my_app"

    # Home directory indicator
    home_dir = mgr.resolve_target_dir("demo_project", "home directory")
    assert home_dir == Path.home() / "demo_project"

def test_antigravity_manager_prompt():
    from jarvis.tools.antigravity import AntigravityManager
    mgr = AntigravityManager()
    prompt = mgr.build_scaffold_prompt("task_tracker", "A CLI task manager in Rust", project_type="rust CLI")
    assert "full autonomous permissions" in prompt
    assert "task_tracker" in prompt
    assert "A CLI task manager in Rust" in prompt
    assert "README.md" in prompt

def test_coding_cli_manager_tools(tmp_path):
    from jarvis.tools.antigravity import CodingCLIManager
    from pathlib import Path

    # 1. Claude
    claude_mgr = CodingCLIManager(cli_tool="claude")
    assert claude_mgr.display_name == "Claude Code"
    cmd_claude = claude_mgr._build_autonomous_command(tmp_path, "test prompt", tmp_path / "log.txt", "echo ok", "echo fail")
    assert "claude" in cmd_claude
    assert "--dangerously-skip-permissions" in cmd_claude

    # 2. Codex
    codex_mgr = CodingCLIManager(cli_tool="codex")
    assert codex_mgr.display_name == "Codex"
    cmd_codex = codex_mgr._build_autonomous_command(tmp_path, "test prompt", tmp_path / "log.txt", "echo ok", "echo fail")
    assert "codex" in cmd_codex
    assert "--dangerously-bypass-approvals-and-sandbox" in cmd_codex

    # 3. Antigravity (agy)
    agy_mgr = CodingCLIManager(cli_tool="agy")
    assert agy_mgr.display_name == "Antigravity"
    cmd_agy = agy_mgr._build_autonomous_command(tmp_path, "test prompt", tmp_path / "log.txt", "echo ok", "echo fail")
    assert "agy" in cmd_agy
    assert "--add-dir" in cmd_agy
    assert "--dangerously-skip-permissions" in cmd_agy


@pytest.mark.anyio
async def test_virtual_input_manager():
    from jarvis.tools.virtual_input import VirtualInputManager
    mgr = VirtualInputManager()
    assert mgr._wtype_bin is not None
    # Empty typing handled gracefully
    res = await mgr.type_text("")
    assert "No text provided" in res


@pytest.mark.anyio
async def test_shell_executor():
    from jarvis.tools.shell import ShellExecutor
    executor = ShellExecutor(default_timeout=5.0, max_output_chars=500)
    
    # 1. Simple command
    res = await executor.execute_command("echo 'jarvis hands-free'")
    assert "jarvis hands-free" in res

    # 2. Timeout handling
    res_timeout = await executor.execute_command("sleep 2", timeout=0.1)
    assert "timed out" in res_timeout.lower()

    # 3. Output truncation
    res_trunc = await executor.execute_command("python3 -c \"print('A' * 1000)\"")
    assert "truncated" in res_trunc.lower()


@pytest.mark.anyio
async def test_media_manager():
    from jarvis.tools.media import MediaManager
    media = MediaManager()
    assert media._dbus_send_bin is not None
    players = await media.get_active_players()
    assert isinstance(players, list)
    now_playing = await media.get_now_playing()
    assert isinstance(now_playing, str)


@pytest.mark.anyio
async def test_clipboard_manager():
    from jarvis.tools.clipboard import ClipboardManager
    cb = ClipboardManager()
    assert cb._wl_paste_bin is not None
    assert cb._wl_copy_bin is not None
    # Empty set handled gracefully
    res = await cb.set_clipboard("")
    assert "No text provided" in res


@pytest.mark.anyio
async def test_system_power_manager():
    from jarvis.tools.system_power import SystemPowerManager
    from unittest.mock import AsyncMock, MagicMock

    mock_bridge = MagicMock()
    mock_bridge.run = AsyncMock(return_value="mocked output")

    power = SystemPowerManager(omarchy=mock_bridge)
    
    lock_res = await power.lock_screen()
    assert "mocked output" in lock_res or "locked" in lock_res.lower()
    mock_bridge.run.assert_called_with("system lock")

    stats_res = await power.get_system_stats()
    assert stats_res == "mocked output"
    mock_bridge.run.assert_called_with("system stats")

    bt_res = await power.toggle_bluetooth("toggle")
    assert "Bluetooth power set to toggle" in bt_res
    mock_bridge.run.assert_called_with("bluetooth power", "toggle")

