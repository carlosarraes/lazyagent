from __future__ import annotations

from typing import TYPE_CHECKING

from textual.widgets import Input

if TYPE_CHECKING:
    from ..state.models import Conversation


class PromptInput(Input):
    """Prompt entry widget. Submit via Enter posts PromptSubmitted message."""

    def __init__(self) -> None:
        super().__init__(
            id="prompt-input",
            placeholder="Enter a prompt...",
        )
        self._mode = "build"
        self._update_title()
        self.border_subtitle = "[dim]Ready[/]"

    def set_mode(self, mode: str) -> None:
        self._mode = mode
        if mode == "plan":
            self.add_class("plan-mode")
        else:
            self.remove_class("plan-mode")
        self._update_title()

    def _update_title(self) -> None:
        tag = "[bold reverse] Plan [/] Build" if self._mode == "plan" else "Plan [bold reverse] Build [/]"
        self.border_title = f"{tag}  (Shift+Tab: toggle, Tab: focus, Ctrl+N: new)"

    def set_status(self, convo: Conversation | None) -> None:
        if convo is None:
            self.border_subtitle = "[dim]Ready[/]"
            return
        elapsed = convo.elapsed()
        if convo.is_active():
            activity = convo.activity or "Working"
            self.border_subtitle = f"[green]● {activity}[/] │ {elapsed}"
        else:
            self.border_subtitle = f"[dim]○ Idle[/] │ {elapsed}"
