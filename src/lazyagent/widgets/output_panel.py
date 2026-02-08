from __future__ import annotations

from rich.text import Text
from textual.widgets import RichLog


class OutputPanel(RichLog):
    """Agent output display with auto-scroll and color support."""

    BORDER_TITLE = "Output"

    def __init__(self) -> None:
        super().__init__(
            id="output-panel",
            highlight=False,
            markup=True,
            wrap=True,
            auto_scroll=True,
        )

    def set_output(self, text: str) -> None:
        self.clear()
        if text:
            self.write(Text.from_ansi(text))

    def append_text(self, text: str) -> None:
        if text:
            self.write(Text.from_ansi(text))
