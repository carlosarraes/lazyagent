from __future__ import annotations

from typing import TYPE_CHECKING

from claude_agent_sdk import (
    AssistantMessage,
    ClaudeAgentOptions,
    ClaudeSDKClient,
    HookMatcher,
    ResultMessage,
)

from .messages import format_message, tool_activity, tool_context

if TYPE_CHECKING:
    from collections.abc import Callable


class AgentRunner:
    """Wraps ClaudeSDKClient to stream agent output with hooks."""

    def __init__(
        self,
        cwd: str,
        session_id: str = "",
        permission_mode: str = "bypassPermissions",
        on_output: Callable[[str], None] | None = None,
        on_activity: Callable[[str], None] | None = None,
        on_session_id: Callable[[str], None] | None = None,
        on_done: Callable[[], None] | None = None,
    ) -> None:
        self.cwd = cwd
        self.session_id = session_id
        self.permission_mode = permission_mode
        self._on_output = on_output
        self._on_activity = on_activity
        self._on_session_id = on_session_id
        self._on_done = on_done
        self._client: ClaudeSDKClient | None = None

    async def _pre_tool_hook(
        self, input_data: dict, tool_use_id: str, context: dict
    ) -> dict:
        name = input_data.get("tool_name", "")
        inp = input_data.get("tool_input", {})

        activity = tool_activity(name)
        if self._on_activity:
            self._on_activity(activity)

        ctx = tool_context(name, inp)
        if ctx:
            text = f"\x1b[33m⚡ {name}: {ctx}\x1b[0m\n"
        else:
            text = f"\x1b[33m⚡ {name}\x1b[0m\n"

        if self._on_output:
            self._on_output(text)

        return {}

    async def run(self, prompt: str) -> None:
        options = ClaudeAgentOptions(
            cwd=self.cwd,
            permission_mode=self.permission_mode,
            hooks={
                "PreToolUse": [
                    HookMatcher(matcher="", hooks=[self._pre_tool_hook]),
                ],
            },
        )

        if self.session_id:
            options.resume = self.session_id
            options.continue_conversation = True

        try:
            async with ClaudeSDKClient(options=options) as client:
                self._client = client
                await client.query(prompt)

                async for message in client.receive_response():
                    if isinstance(message, ResultMessage):
                        if message.session_id and self._on_session_id:
                            self._on_session_id(message.session_id)
                        continue

                    if isinstance(message, AssistantMessage):
                        text, activity = format_message(message)
                        if text and self._on_output:
                            self._on_output(text + "\n")
                        if activity and self._on_activity:
                            self._on_activity(activity)
        finally:
            self._client = None
            if self._on_done:
                self._on_done()
