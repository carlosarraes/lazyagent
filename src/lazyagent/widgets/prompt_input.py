from __future__ import annotations

from textual.widgets import Input


class PromptInput(Input):
    """Prompt entry widget. Submit via Enter posts PromptSubmitted message."""

    BORDER_TITLE = "Prompt (Tab: focus, Ctrl+N: new, q: quit)"

    def __init__(self) -> None:
        super().__init__(
            id="prompt-input",
            placeholder="Enter a prompt...",
        )
