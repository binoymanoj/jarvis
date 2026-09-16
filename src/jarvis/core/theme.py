from pathlib import Path
from typing import Dict

THEME_STATE_DIR = Path.home() / ".local/state/omarchy/current"
THEME_COLORS_PATH = THEME_STATE_DIR / "theme" / "colors.toml"
THEME_NAME_PATH = THEME_STATE_DIR / "theme.name"


def load_omarchy_theme() -> Dict[str, str]:
    """Read and parse the currently active Omarchy theme colors."""
    theme_name = "Omarchy"
    if THEME_NAME_PATH.exists():
        try:
            theme_name = THEME_NAME_PATH.read_text().strip().title()
        except Exception:
            pass

    colors = {
        "name": theme_name,
        "accent": "#00f0ff",
        "foreground": "#cacccc",
        "background": "#101315",
        "dark_background": "#080a0b",
        "muted": "#707880",
        "selection": "#343d41",
        "cyan": "#00f0ff",
        "magenta": "#d946ef",
        "blue": "#38bdf8",
        "active_border": "#a8adb0",
    }

    if THEME_COLORS_PATH.exists():
        try:
            text = THEME_COLORS_PATH.read_text()
            for line in text.splitlines():
                line = line.strip()
                if not line or line.startswith("#") or line.startswith("["):
                    continue
                if "=" in line:
                    parts = line.split("=", 1)
                    k = parts[0].strip()
                    raw_val = parts[1].strip()
                    # Remove surrounding quotes and trailing inline comments
                    if raw_val.startswith('"') or raw_val.startswith("'"):
                        q = raw_val[0]
                        end_q = raw_val.find(q, 1)
                        val = raw_val[1:end_q] if end_q != -1 else raw_val.strip(q)
                    else:
                        val = raw_val.split()[0].strip()

                    if k in ("accent", "foreground", "background", "cyan", "magenta", "blue", "muted", "selection"):
                        colors[k] = val
                    elif k == "active_border_color":
                        colors["active_border"] = val
                    elif k == "bright_cyan" and colors["cyan"] == "#00f0ff":
                        colors["cyan"] = val
                    elif k == "bright_magenta" and colors["magenta"] == "#d946ef":
                        colors["magenta"] = val
                    elif k == "bright_blue" and colors["blue"] == "#38bdf8":
                        colors["blue"] = val
                    elif k in ("darker_background", "dark_background"):
                        colors["dark_background"] = val
        except Exception:
            pass

    return colors
