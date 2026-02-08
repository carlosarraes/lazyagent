from __future__ import annotations

from rich.text import Text
from textual.widgets import Static

from ..state.models import Conversation


class StatusBar(Static):
    """Activity and elapsed time display."""

    def __init__(self) -> None:
        super().__init__(id="status-bar")

    def update_status(self, convo: Conversation | None) -> None:
        if convo is None:
            self.update(Text("─── Ready ───", style="dim"))
            return

        elapsed = convo.elapsed()

        if convo.is_active():
            activity = convo.activity or "Working"
            line = Text()
            line.append("● ", style="green")
            line.append(activity, style="green")
            line.append(f" │ {elapsed} │ {convo.status.value}", style="")
        else:
            line = Text()
            line.append("○ Idle", style="dim")
            line.append(f" │ {elapsed} │ {convo.status.value}", style="")

        self.update(line)
