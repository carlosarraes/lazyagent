from __future__ import annotations

from claude_agent_sdk import (
    AssistantMessage,
    ResultMessage,
    SystemMessage,
    TextBlock,
)


TOOL_ACTIVITIES = {
    "Read": "Reading file",
    "Write": "Writing file",
    "Edit": "Editing file",
    "Bash": "Running command",
    "Glob": "Searching files",
    "Grep": "Searching content",
    "LS": "Listing directory",
    "Task": "Spawning agent",
    "WebFetch": "Fetching web",
    "WebSearch": "Searching web",
}


def tool_activity(tool_name: str) -> str:
    return TOOL_ACTIVITIES.get(tool_name, f"Using {tool_name}")


def tool_context(tool_name: str, tool_input: dict) -> str:
    """Extract a short context string from tool input."""
    if tool_name == "Bash":
        return tool_input.get("description", "") or _truncate(
            tool_input.get("command", ""), 50
        )
    if tool_name in ("Read", "Edit", "Write"):
        fp = tool_input.get("file_path", "")
        return fp.rsplit("/", 1)[-1] if "/" in fp else fp
    if tool_name in ("Grep", "Glob"):
        return _truncate(tool_input.get("pattern", ""), 40)
    if tool_name == "Task":
        return _truncate(tool_input.get("description", ""), 40)
    return ""


def format_message(message: object) -> tuple[str, str]:
    """Format an SDK message into (output_text, activity).

    Returns a tuple of (text to append to output, activity string for status bar).
    Tool invocations are handled by PreToolUse hooks, not here.
    """
    if isinstance(message, AssistantMessage):
        parts: list[str] = []
        activity = ""
        for block in message.content:
            if isinstance(block, TextBlock):
                parts.append(block.text)
                activity = "Writing"
        return "\n".join(parts), activity

    if isinstance(message, SystemMessage):
        return "", "Initializing"

    if isinstance(message, ResultMessage):
        return "", ""

    return "", ""


def _truncate(s: str, max_len: int) -> str:
    s = s.split("\n")[0]
    if len(s) > max_len:
        return s[: max_len - 1] + "…"
    return s
