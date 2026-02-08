from __future__ import annotations

import json
import os
from datetime import datetime, timezone
from pathlib import Path

from .models import AppState, Conversation, Project, Status


def _state_dir() -> Path:
    home = Path.home()
    return home / ".local" / "state" / "lazyagent"


def _logs_dir() -> Path:
    return _state_dir() / "logs"


def _state_path() -> Path:
    return _state_dir() / "state.json"


def _log_path(convo_id: str) -> Path:
    return _logs_dir() / f"{convo_id}.log"


def _ensure_dirs() -> None:
    _state_dir().mkdir(parents=True, exist_ok=True)
    _logs_dir().mkdir(parents=True, exist_ok=True)


def delete_log(convo_id: str) -> None:
    path = _log_path(convo_id)
    path.unlink(missing_ok=True)


def save_state(state: AppState) -> None:
    _ensure_dirs()

    data: dict = {"selected_project": state.selected_proj, "projects": []}

    for proj in state.projects:
        proj_data: dict = {
            "path": proj.path,
            "name": proj.name,
            "expanded": proj.expanded,
            "conversations": [],
        }
        for convo in proj.convos:
            proj_data["conversations"].append(
                {
                    "id": convo.id,
                    "session_id": convo.session_id,
                    "prompt": convo.prompt,
                    "title": convo.title,
                    "status": convo.status.value,
                    "start_time": convo.start_time.isoformat(),
                }
            )
            _log_path(convo.id).write_text(convo.output)
        data["projects"].append(proj_data)

    _state_path().write_text(json.dumps(data, indent=2))


def load_state(current_path: str) -> AppState:
    state = AppState(projects=[], selected_proj=0)

    try:
        raw = json.loads(_state_path().read_text())
    except (FileNotFoundError, json.JSONDecodeError):
        return _new_state_with_project(current_path)

    for pd in raw.get("projects", []):
        proj = Project(
            path=pd["path"],
            name=pd["name"],
            expanded=pd.get("expanded", True),
            selected=-1,
        )
        for cd in pd.get("conversations", []):
            output = ""
            try:
                output = _log_path(cd["id"]).read_text()
            except FileNotFoundError:
                pass
            status = Status(cd.get("status", "I"))
            if status == Status.RUNNING:
                status = Status.IDLE
            convo = Conversation(
                id=cd["id"],
                session_id=cd.get("session_id", ""),
                prompt=cd["prompt"],
                title=cd.get("title", ""),
                output=output,
                status=status,
                start_time=datetime.fromisoformat(cd["start_time"]),
            )
            proj.convos.append(convo)
        state.projects.append(proj)

    found = -1
    for i, proj in enumerate(state.projects):
        if proj.path == current_path:
            found = i
            break

    if found == -1:
        state.projects.insert(
            0,
            Project(
                path=current_path,
                name=os.path.basename(current_path),
                expanded=True,
                selected=-1,
            ),
        )
        state.selected_proj = 0
    else:
        state.selected_proj = found

    return state


def _new_state_with_project(path: str) -> AppState:
    return AppState(
        projects=[
            Project(
                path=path,
                name=os.path.basename(path),
                expanded=True,
                selected=-1,
            )
        ],
        selected_proj=0,
    )
