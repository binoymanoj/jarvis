import pytest
from jarvis.ui.hud import JarvisHUD

@pytest.mark.anyio
async def test_hud_initialization():
    hud = JarvisHUD()
    assert hud.qs_bin is not None
    assert (hud.ui_path / "shell.qml").exists()
