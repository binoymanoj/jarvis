"""Jarvis system tools and integrations (Hyprland, Omarchy, screen capture)."""

from jarvis.tools.antigravity import AntigravityManager, CodingCLIManager
from jarvis.tools.calendar import CalendarManager
from jarvis.tools.clipboard import ClipboardManager
from jarvis.tools.email import EmailManager
from jarvis.tools.hyprland import HyprlandController, HyprlandEventListener
from jarvis.tools.media import MediaManager
from jarvis.tools.notes import NoteManager
from jarvis.tools.omarchy import OmarchyBridge
from jarvis.tools.screen import ScreenPerception
from jarvis.tools.shell import ShellExecutor
from jarvis.tools.system_power import SystemPowerManager
from jarvis.tools.virtual_input import VirtualInputManager
from jarvis.tools.web import WebNavigator
from jarvis.tools.workflow import WorkflowManager

__all__ = [
    "AntigravityManager",
    "CodingCLIManager",
    "CalendarManager",
    "ClipboardManager",
    "EmailManager",
    "HyprlandController",
    "HyprlandEventListener",
    "MediaManager",
    "NoteManager",
    "OmarchyBridge",
    "ScreenPerception",
    "ShellExecutor",
    "SystemPowerManager",
    "VirtualInputManager",
    "WebNavigator",
    "WorkflowManager",
]

