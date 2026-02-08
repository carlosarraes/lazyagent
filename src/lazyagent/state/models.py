from __future__ import annotations

import os
import secrets
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum


class Status(str, Enum):
    RUNNING = "R"
    IDLE = "I"
    ERROR = "E"


@dataclass
class Conversation:
    id: str
    prompt: str
    status: Status = Status.IDLE
    start_time: datetime = field(default_factory=lambda: datetime.now(timezone.utc))
    title: str = ""
    session_id: str = ""
    activity: str = ""
    output: str = ""

    def is_active(self) -> bool:
        return self.status == Status.RUNNING

    def elapsed(self) -> str:
        delta = datetime.now(timezone.utc) - self.start_time
        minutes = int(delta.total_seconds() // 60)
        if minutes < 1:
            return "<1m"
        if minutes < 60:
            return f"{minutes}m"
        hours = minutes // 60
        remaining = minutes % 60
        return f"{hours}h{remaining}m"


@dataclass
class Project:
    path: str
    name: str
    expanded: bool = True
    convos: list[Conversation] = field(default_factory=list)
    selected: int = -1


@dataclass
class AppState:
    projects: list[Project] = field(default_factory=list)
    selected_proj: int = 0
    tree_focused: bool = False

    def current_project(self) -> Project | None:
        if not self.projects:
            return None
        return self.projects[self.selected_proj]

    def selected_conversation(self) -> Conversation | None:
        proj = self.current_project()
        if proj is None or proj.selected < 0 or proj.selected >= len(proj.convos):
            return None
        return proj.convos[proj.selected]

    def new_conversation(self, prompt: str) -> Conversation | None:
        proj = self.current_project()
        if proj is None:
            return None
        convo = Conversation(
            id=secrets.token_hex(3),
            prompt=prompt,
            status=Status.RUNNING,
        )
        proj.convos.append(convo)
        proj.selected = len(proj.convos) - 1
        proj.expanded = True
        return convo

    def delete_selected_conversation(self) -> Conversation | None:
        proj = self.current_project()
        if proj is None or proj.selected < 0 or proj.selected >= len(proj.convos):
            return None
        idx = proj.selected
        convo = proj.convos.pop(idx)
        if not proj.convos:
            proj.selected = -1
        elif proj.selected >= len(proj.convos):
            proj.selected = len(proj.convos) - 1
        return convo


    def _tree_item_count(self) -> int:
        count = 0
        for proj in self.projects:
            count += 1
            if proj.expanded:
                count += len(proj.convos)
        return count

    def _selected_tree_index(self) -> int:
        idx = 0
        for i, proj in enumerate(self.projects):
            if i == self.selected_proj and proj.selected == -1:
                return idx
            idx += 1
            if proj.expanded:
                for j in range(len(proj.convos)):
                    if i == self.selected_proj and proj.selected == j:
                        return idx
                    idx += 1
        return 0

    def _select_tree_index(self, target: int) -> None:
        idx = 0
        for i, proj in enumerate(self.projects):
            if idx == target:
                self.selected_proj = i
                proj.selected = -1
                return
            idx += 1
            if proj.expanded:
                for j in range(len(proj.convos)):
                    if idx == target:
                        self.selected_proj = i
                        proj.selected = j
                        return
                    idx += 1

    def move_selection(self, delta: int) -> None:
        count = self._tree_item_count()
        if count == 0:
            return
        current = self._selected_tree_index()
        nxt = max(0, min(count - 1, current + delta))
        self._select_tree_index(nxt)

    def toggle_expand(self) -> None:
        proj = self.current_project()
        if proj is not None and proj.selected == -1:
            proj.expanded = not proj.expanded


def load_initial_state(cwd: str) -> AppState:
    """Create a fresh AppState with the cwd as the sole project."""
    return AppState(
        projects=[
            Project(
                path=cwd,
                name=os.path.basename(cwd),
                expanded=True,
                selected=-1,
            )
        ],
        selected_proj=0,
    )
