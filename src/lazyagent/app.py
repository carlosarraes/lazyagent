from __future__ import annotations

import os

import anyio
from rich.syntax import Syntax
from rich.text import Text
from textual import work
from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.message import Message
from textual.theme import Theme

from claude_agent_sdk import AssistantMessage, TextBlock, query, ClaudeAgentOptions

from .agent.client import AgentRunner
from .state.models import AppState, Conversation, Status
from .state.persist import delete_log, load_state, save_state
from .widgets.output_panel import OutputPanel
from .widgets.prompt_input import PromptInput
from .widgets.tree_panel import TreePanel


class OutputUpdate(Message):
    def __init__(self, convo_id: str, text: str) -> None:
        super().__init__()
        self.convo_id = convo_id
        self.text = text


class ActivityUpdate(Message):
    def __init__(self, convo_id: str, activity: str) -> None:
        super().__init__()
        self.convo_id = convo_id
        self.activity = activity


class SessionIdUpdate(Message):
    def __init__(self, convo_id: str, session_id: str) -> None:
        super().__init__()
        self.convo_id = convo_id
        self.session_id = session_id


class AgentDone(Message):
    def __init__(self, convo_id: str) -> None:
        super().__init__()
        self.convo_id = convo_id


class LazyAgentApp(App[None]):
    """TUI for orchestrating AI coding agents."""

    CSS_PATH = "styles.tcss"

    BINDINGS = [
        Binding("ctrl+c", "quit", "Quit", show=False),
        Binding("q", "quit_if_tree", "Quit", show=False),
        Binding("tab", "switch_focus", "Switch Focus", show=False),
        Binding("ctrl+n", "new_conversation", "New Conversation", show=False),
    ]

    def __init__(self) -> None:
        super().__init__()
        self.register_theme(_CATPPUCCIN_MOCHA)
        self.theme = "catppuccin-mocha"
        self.state: AppState = load_state(os.getcwd())
        self._runners: dict[str, AgentRunner] = {}
        self._output_mode: int = 1
        self._pending_mode: str = "build"

    def compose(self) -> ComposeResult:
        yield TreePanel()
        yield OutputPanel()
        yield PromptInput()

    def on_mount(self) -> None:
        self.query_one(OutputPanel).border_title = _output_mode_title(1)
        self._refresh_all()
        self.query_one(PromptInput).focus()
        self.set_interval(1.0, self._tick)

    def _tick(self) -> None:
        self._refresh_status()
        self.query_one(TreePanel).update_labels(self.state)

    def _refresh_all(self) -> None:
        self._refresh_tree()
        self._refresh_output()
        self._refresh_status()
        self._refresh_prompt_mode()

    def _refresh_tree(self) -> None:
        self.query_one(TreePanel).rebuild(self.state)

    def _refresh_output(self) -> None:
        if self._output_mode != 1:
            return
        panel = self.query_one(OutputPanel)
        convo = self.state.selected_conversation()
        if convo is None:
            panel.set_output("")
        else:
            panel.set_output(convo.output)

    def _refresh_status(self) -> None:
        self.query_one(PromptInput).set_status(self.state.selected_conversation())

    def _refresh_prompt_mode(self) -> None:
        convo = self.state.selected_conversation()
        mode = convo.mode if convo else self._pending_mode
        self.query_one(PromptInput).set_mode(mode)


    def _set_output_mode(self, mode: int) -> None:
        self._output_mode = mode
        panel = self.query_one(OutputPanel)
        panel.border_title = _output_mode_title(mode)
        if mode == 1:
            self._refresh_output()
        elif mode == 2:
            self._show_git_diff()

    @work(thread=False)
    async def _show_git_diff(self) -> None:
        proj = self.state.current_project()
        cwd = proj.path if proj else os.getcwd()
        panel = self.query_one(OutputPanel)
        panel.clear()

        try:
            result_status = await anyio.run_process(
                ["git", "status", "--short"], cwd=cwd
            )
            result_diff = await anyio.run_process(
                ["git", "diff"], cwd=cwd
            )
            status_text = result_status.stdout.decode().rstrip("\n")
            diff_text = result_diff.stdout.decode().rstrip("\n")

            if not status_text and not diff_text:
                panel.write(Text("No changes", style="dim"))
                return

            if status_text:
                panel.write(Text.from_ansi("\x1b[36m── git status ──\x1b[0m"))
                panel.write(Text(status_text))

            if diff_text:
                panel.write(Text.from_ansi("\x1b[36m── git diff ──\x1b[0m"))
                panel.write(Syntax(diff_text, "diff", theme="monokai", line_numbers=False))
        except Exception as e:
            panel.write(Text.from_ansi(f"\x1b[31m[Error] {e}\x1b[0m"))


    def action_quit_if_tree(self) -> None:
        if self.focused and self.focused.id == "tree-panel":
            self.exit()

    def action_switch_focus(self) -> None:
        tree = self.query_one(TreePanel)
        prompt = self.query_one(PromptInput)
        if self.focused is tree:
            self.state.tree_focused = False
            prompt.focus()
        else:
            self.state.tree_focused = True
            tree.focus()

    def _toggle_mode(self) -> None:
        convo = self.state.selected_conversation()
        if convo is None:
            new_mode = "plan" if self._pending_mode == "build" else "build"
            self._pending_mode = new_mode
        else:
            convo.mode = "plan" if convo.mode == "build" else "build"
            new_mode = convo.mode
            save_state(self.state)
        self.query_one(PromptInput).set_mode(new_mode)

    def action_new_conversation(self) -> None:
        proj = self.state.current_project()
        if proj is not None:
            proj.selected = -1
        self.state.tree_focused = False
        self.query_one(PromptInput).focus()
        self._refresh_all()


    def on_tree_node_highlighted(self, event: TreePanel.NodeHighlighted) -> None:
        node = event.node
        data = node.data or ""
        if data.startswith("convo:"):
            convo_id = data.split(":", 1)[1]
            for i, proj in enumerate(self.state.projects):
                for j, convo in enumerate(proj.convos):
                    if convo.id == convo_id:
                        self.state.selected_proj = i
                        proj.selected = j
                        self._refresh_output()
                        self._refresh_status()
                        self._refresh_prompt_mode()
                        return
        elif data.startswith("proj:"):
            proj_path = data.split(":", 1)[1]
            for i, proj in enumerate(self.state.projects):
                if proj.path == proj_path:
                    self.state.selected_proj = i
                    proj.selected = -1
                    self._refresh_output()
                    self._refresh_status()
                    self._refresh_prompt_mode()
                    return

    def on_key(self, event) -> None:
        if event.key == "shift+tab":
            if self.focused and self.focused.id == "prompt-input":
                self._toggle_mode()
            else:
                self.action_switch_focus()
            event.prevent_default()
            return

        if not (self.focused and self.focused.id == "prompt-input"):
            if event.key == "1":
                self._set_output_mode(1)
                event.prevent_default()
                return
            elif event.key == "2":
                self._set_output_mode(2)
                event.prevent_default()
                return

        if self.focused and self.focused.id == "tree-panel":
            tree = self.query_one(TreePanel)
            if event.key == "j":
                tree.action_cursor_down()
                event.prevent_default()
            elif event.key == "k":
                tree.action_cursor_up()
                event.prevent_default()
            elif event.key == "enter":
                self.state.toggle_expand()
                self._refresh_all()
                event.prevent_default()
            elif event.key == "n":
                self.action_new_conversation()
                event.prevent_default()
            elif event.key == "d":
                self._delete_conversation()
                event.prevent_default()


    def on_input_submitted(self, event: PromptInput.Submitted) -> None:
        prompt = event.value.strip()
        if not prompt:
            return

        self.query_one(PromptInput).value = ""
        self._output_mode = 1
        self.query_one(OutputPanel).border_title = _output_mode_title(1)

        convo = self.state.selected_conversation()

        is_new = convo is None
        if is_new:
            convo = self.state.new_conversation(prompt)
            if convo is None:
                return
            convo.mode = self._pending_mode
            convo.output = f"\x1b[36m▶ {prompt}\x1b[0m\n"
        else:
            convo.output += f"\n\x1b[36m▶ {prompt}\x1b[0m\n"
            convo.status = Status.RUNNING

        convo.activity = "Starting"
        self._refresh_all()
        save_state(self.state)

        self._spawn_agent(convo, prompt)
        if is_new:
            self._generate_title(convo)

    @work(thread=False)
    async def _spawn_agent(self, convo: Conversation, prompt: str) -> None:
        convo_id = convo.id
        app = self.app

        def on_output(text: str) -> None:
            app.post_message(OutputUpdate(convo_id, text))

        def on_activity(activity: str) -> None:
            app.post_message(ActivityUpdate(convo_id, activity))

        def on_session_id(sid: str) -> None:
            app.post_message(SessionIdUpdate(convo_id, sid))

        def on_done() -> None:
            app.post_message(AgentDone(convo_id))

        permission_mode = "plan" if convo.mode == "plan" else "bypassPermissions"
        runner = AgentRunner(
            cwd=self.state.current_project().path if self.state.current_project() else os.getcwd(),
            session_id=convo.session_id,
            permission_mode=permission_mode,
            on_output=on_output,
            on_activity=on_activity,
            on_session_id=on_session_id,
            on_done=on_done,
        )
        self._runners[convo_id] = runner

        try:
            await runner.run(prompt)
        except Exception as e:
            app.post_message(
                OutputUpdate(convo_id, f"\x1b[31m[Error] {e}\x1b[0m\n")
            )
            app.post_message(AgentDone(convo_id))


    @work(thread=False)
    async def _generate_title(self, convo: Conversation) -> None:
        import json as _json

        try:
            options = ClaudeAgentOptions(
                max_turns=1,
                model="haiku",
                system_prompt="Generate a 1-3 word title for the user's task. Be concise.",
                output_format={
                    "type": "json_schema",
                    "schema": {
                        "type": "object",
                        "properties": {
                            "title": {"type": "string"}
                        },
                        "required": ["title"],
                    },
                },
            )
            async for message in query(
                prompt=f"Summarize: {convo.prompt}", options=options
            ):
                if isinstance(message, AssistantMessage):
                    for block in message.content:
                        if isinstance(block, TextBlock):
                            text = block.text.strip()
                            try:
                                convo.title = _json.loads(text)["title"]
                            except (ValueError, KeyError):
                                convo.title = text.split("\n")[0][:30]
                            self._refresh_tree()
                            save_state(self.state)
                            return
        except Exception:
            pass


    def on_output_update(self, event: OutputUpdate) -> None:
        convo = self._find_convo(event.convo_id)
        if convo is None:
            return
        convo.output += event.text
        if self._output_mode == 1 and self.state.selected_conversation() is convo:
            self.query_one(OutputPanel).append_text(event.text)

    def on_activity_update(self, event: ActivityUpdate) -> None:
        convo = self._find_convo(event.convo_id)
        if convo is None:
            return
        convo.activity = event.activity
        if self.state.selected_conversation() is convo:
            self._refresh_status()

    def on_session_id_update(self, event: SessionIdUpdate) -> None:
        convo = self._find_convo(event.convo_id)
        if convo and not convo.session_id:
            convo.session_id = event.session_id

    def on_agent_done(self, event: AgentDone) -> None:
        convo = self._find_convo(event.convo_id)
        if convo is not None:
            convo.status = Status.IDLE
            convo.activity = ""
        self._runners.pop(event.convo_id, None)
        self._refresh_all()
        save_state(self.state)


    def _delete_conversation(self) -> None:
        convo = self.state.selected_conversation()
        if convo is None:
            return
        self._runners.pop(convo.id, None)
        delete_log(convo.id)
        self.state.delete_selected_conversation()
        self._refresh_all()
        save_state(self.state)

    def _find_convo(self, convo_id: str) -> Conversation | None:
        for proj in self.state.projects:
            for convo in proj.convos:
                if convo.id == convo_id:
                    return convo
        return None

    def _on_exit_app(self) -> None:
        save_state(self.state)


def _output_mode_title(mode: int) -> str:
    if mode == 1:
        return "[bold reverse] Output [/] | Diff"
    return "Output | [bold reverse] Diff [/]"


_CATPPUCCIN_MOCHA = Theme(
    name="catppuccin-mocha",
    primary="#89b4fa",
    secondary="#b4befe",
    accent="#cba6f7",
    warning="#f9e2af",
    error="#f38ba8",
    success="#a6e3a1",
    foreground="#cdd6f4",
    background="#1e1e2e",
    surface="#313244",
    panel="#181825",
    dark=True,
)
