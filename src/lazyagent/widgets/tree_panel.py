from __future__ import annotations

from textual.widgets import Tree
from textual.widgets._tree import TreeNode

from ..state.models import AppState, Conversation, Project


class TreePanel(Tree[str]):
    """Project/conversation tree with vim-style navigation."""

    BORDER_TITLE = "Projects"

    def __init__(self) -> None:
        super().__init__("Projects", id="tree-panel")
        self.show_root = False

    def rebuild(self, state: AppState) -> None:
        self.clear()
        selected_convo = state.selected_conversation()

        for proj in state.projects:
            arrow = "▾" if proj.expanded else "▸"
            proj_node = self.root.add(
                f"{arrow} {proj.name}",
                data=f"proj:{proj.path}",
                expand=proj.expanded,
            )

            if proj.expanded:
                for convo in proj.convos:
                    status_color = {
                        "R": "[green]",
                        "I": "[dim]",
                        "E": "[red]",
                    }.get(convo.status.value, "[dim]")

                    label = (
                        f"  {status_color}[{convo.status.value}][/] "
                        f"{_truncate(convo.prompt, 20)} "
                        f"[dim]({convo.elapsed()})[/]"
                    )
                    node = proj_node.add_leaf(label, data=f"convo:{convo.id}")

                    if selected_convo and convo.id == selected_convo.id:
                        self.select_node(node)

    def on_mount(self) -> None:
        self.show_guides = False


def _truncate(s: str, max_len: int) -> str:
    s = s.split("\n")[0]
    if len(s) > max_len:
        return s[: max_len - 1] + "…"
    return s
