from __future__ import annotations

import os

from textual import work
from textual.app import App, ComposeResult
from textual.binding import Binding
from textual.message import Message

from .agent.client import AgentRunner
from .state.models import AppState, Conversation, Status
from .state.persist import delete_log, load_state, save_state
from .widgets.output_panel import OutputPanel
from .widgets.prompt_input import PromptInput
from .widgets.status_bar import StatusBar
from .widgets.tree_panel import TreePanel


class OutputUpdate(Message):
    """Posted when agent produces new output."""

    def __init__(self, convo_id: str, text: str) -> None:
        super().__init__()
        self.convo_id = convo_id
        self.text = text


class ActivityUpdate(Message):
    """Posted when agent activity changes."""

    def __init__(self, convo_id: str, activity: str) -> None:
        super().__init__()
        self.convo_id = convo_id
        self.activity = activity


class SessionIdUpdate(Message):
    """Posted when session ID is received from agent."""

    def __init__(self, convo_id: str, session_id: str) -> None:
        super().__init__()
        self.convo_id = convo_id
        self.session_id = session_id


class AgentDone(Message):
    """Posted when agent finishes."""

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
        self.state: AppState = load_state(os.getcwd())
        self._runners: dict[str, AgentRunner] = {}

    def compose(self) -> ComposeResult:
        yield TreePanel()
        yield OutputPanel()
        yield StatusBar()
        yield PromptInput()

    def on_mount(self) -> None:
        self._refresh_all()
        self.query_one(PromptInput).focus()
        self.set_interval(1.0, self._tick)

    def _tick(self) -> None:
        self._refresh_status()
        self._refresh_tree()

    def _refresh_all(self) -> None:
        self._refresh_tree()
        self._refresh_output()
        self._refresh_status()

    def _refresh_tree(self) -> None:
        self.query_one(TreePanel).rebuild(self.state)

    def _refresh_output(self) -> None:
        panel = self.query_one(OutputPanel)
        convo = self.state.selected_conversation()
        if convo is None:
            panel.set_output("")
        else:
            panel.set_output(convo.output)

    def _refresh_status(self) -> None:
        self.query_one(StatusBar).update_status(self.state.selected_conversation())


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
        self._refresh_tree()

    def action_new_conversation(self) -> None:
        proj = self.state.current_project()
        if proj is not None:
            proj.selected = -1
        self.state.tree_focused = False
        self.query_one(PromptInput).focus()
        self._refresh_all()


    def on_key(self, event) -> None:
        if self.focused and self.focused.id == "tree-panel":
            if event.key == "j":
                self.state.move_selection(1)
                self._refresh_all()
                event.prevent_default()
            elif event.key == "k":
                self.state.move_selection(-1)
                self._refresh_all()
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

        convo = self.state.selected_conversation()

        if convo is None:
            convo = self.state.new_conversation(prompt)
            if convo is None:
                return
            convo.output = f"\x1b[36m▶ {prompt}\x1b[0m\n"
        else:
            convo.output += f"\n\x1b[36m▶ {prompt}\x1b[0m\n"
            convo.status = Status.RUNNING

        convo.activity = "Starting"
        self._refresh_all()
        save_state(self.state)

        self._spawn_agent(convo, prompt)

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

        runner = AgentRunner(
            cwd=self.state.current_project().path if self.state.current_project() else os.getcwd(),
            session_id=convo.session_id,
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


    def on_output_update(self, event: OutputUpdate) -> None:
        convo = self._find_convo(event.convo_id)
        if convo is None:
            return
        convo.output += event.text
        if self.state.selected_conversation() is convo:
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

        runner = self._runners.pop(convo.id, None)

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
