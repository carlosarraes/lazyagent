from __future__ import annotations

from textual.widgets import Tree

from ..state.models import AppState, Conversation


class TreePanel(Tree[str]):
    """Project/conversation tree with vim-style navigation."""

    BORDER_TITLE = "Projects"

    def __init__(self) -> None:
        super().__init__("Projects", id="tree-panel")
        self.show_root = False

    def rebuild(self, state: AppState) -> None:
        """Full rebuild — only call on structural changes (add/remove/expand)."""
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
                    label = _build_convo_label(convo)
                    node = proj_node.add_leaf(label, data=f"convo:{convo.id}")

                    if selected_convo and convo.id == selected_convo.id:
                        self.select_node(node)

    def update_labels(self, state: AppState) -> None:
        """In-place label update — no clear(), preserves cursor and scroll."""
        for proj_node in self.root.children:
            proj_data = proj_node.data or ""
            proj_path = proj_data.split(":", 1)[1] if ":" in proj_data else ""
            proj = next((p for p in state.projects if p.path == proj_path), None)
            if proj is None:
                continue

            arrow = "▾" if proj.expanded else "▸"
            proj_node.set_label(f"{arrow} {proj.name}")

            for convo_node in proj_node.children:
                convo_data = convo_node.data or ""
                convo_id = convo_data.split(":", 1)[1] if ":" in convo_data else ""
                convo = next((c for c in proj.convos if c.id == convo_id), None)
                if convo is None:
                    continue
                convo_node.set_label(_build_convo_label(convo))

    def on_mount(self) -> None:
        self.show_guides = False


def _build_convo_label(convo: Conversation) -> str:
    status_color = {
        "R": "[green]",
        "I": "[dim]",
        "E": "[red]",
    }.get(convo.status.value, "[dim]")

    return (
        f"  {status_color}[{convo.status.value}][/] "
        f"{_truncate(convo.prompt, 20)} "
        f"[dim]({convo.elapsed()})[/]"
    )


def _truncate(s: str, max_len: int) -> str:
    s = s.split("\n")[0]
    if len(s) > max_len:
        return s[: max_len - 1] + "…"
    return s
