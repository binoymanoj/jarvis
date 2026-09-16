import logging
import sys
from rich.console import Console
from rich.logging import RichHandler

console = Console()

def setup_logger(name: str = "jarvis", level: int = logging.INFO) -> logging.Logger:
    """Configures and returns a rich-formatted logger matching Omarchy styling."""
    logger = logging.getLogger(name)
    if not logger.handlers:
        logger.setLevel(level)
        handler = RichHandler(
            console=console,
            show_time=True,
            show_path=False,
            rich_tracebacks=True,
            markup=True,
        )
        handler.setFormatter(logging.Formatter("[bold cyan]%(name)s[/bold cyan] › %(message)s"))
        logger.addHandler(handler)
        logger.propagate = False
    return logger

log = setup_logger()
