from __future__ import annotations

import re

from rich.markdown import Markdown
from rich.text import Text
from textual.widgets import RichLog

_ANSI_RE = re.compile(r"\x1b\[")


class OutputPanel(RichLog):
    """Agent output display with markdown and ANSI support."""

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
        if not text:
            return
        for segment in _split_segments(text):
            if _has_ansi(segment):
                self.write(Text.from_ansi(segment))
            elif segment.strip():
                self.write(Markdown(segment))

    def append_text(self, text: str) -> None:
        if not text:
            return
        if _has_ansi(text):
            self.write(Text.from_ansi(text))
        else:
            self.write(Markdown(text))


def _has_ansi(text: str) -> bool:
    return bool(_ANSI_RE.search(text))


def _split_segments(text: str) -> list[str]:
    """Split output into contiguous ANSI vs plain text blocks."""
    segments: list[str] = []
    current: list[str] = []
    current_is_ansi: bool | None = None

    for line in text.split("\n"):
        is_ansi = _has_ansi(line)
        if current_is_ansi is not None and is_ansi != current_is_ansi:
            segments.append("\n".join(current))
            current = []
        current_is_ansi = is_ansi
        current.append(line)

    if current:
        segments.append("\n".join(current))
    return segments
