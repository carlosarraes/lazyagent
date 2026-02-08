# CLAUDE.md

This file provides guidance to Claude Code when working with code in this repository.

## Project Overview

Lazyagent is a TUI application built with Python + Textual + Claude Agent SDK that orchestrates AI coding agents. Think "lazygit but for autonomous task execution."

## Build Commands

```bash
# Install dependencies
uv sync

# Run
uv run python -m lazyagent

# Run tests
uv run pytest

# Format
uv run ruff format src/
```

## Tech Stack

- `textual` — async TUI framework (CSS layout, Tree, RichLog, Input widgets)
- `claude-agent-sdk` — Claude Code as a library (typed streaming, hooks, built-in tools)

## Architecture

```
src/lazyagent/
  __init__.py
  __main__.py          # Entry point
  app.py               # Textual App, layout, global keybindings
  widgets/
    __init__.py
    tree_panel.py      # Project/conversation tree (Tree widget)
    output_panel.py    # Agent output display (RichLog widget)
    status_bar.py      # Activity/elapsed display (Static widget)
    prompt_input.py    # Prompt entry (Input widget)
  agent/
    __init__.py
    client.py          # AgentRunner wrapping ClaudeSDKClient
    messages.py        # SDK message → Rich Text formatting
  state/
    __init__.py
    models.py          # Project, Conversation, AppState dataclasses
    persist.py         # JSON save/load to ~/.local/state/lazyagent/
  styles.tcss          # Textual CSS
```

### Agent SDK Integration

- `ClaudeSDKClient` for hook support (not bare `query()`)
- `PreToolUse` hook → update activity → refresh status bar
- Messages stream via `App.run_worker()` on asyncio event loop
- Session resume via `ClaudeAgentOptions(resume=session_id)`

### Message Flow

`SDK message → format_message() → list[Rich.Text] → App.post_message(OutputUpdate) → OutputPanel.write()`

### State Persistence

JSON format in `~/.local/state/lazyagent/`. Output stored as plain text log files.

## Guidelines

- Keep files under 500 lines
- Error handling: raise exceptions, don't silently swallow
- Use dataclasses for state models
- Use `anyio` for async operations (SDK dependency)

## Non-Goals (MVP)

- Multi-project support
- Configuration files
- Git worktrees
