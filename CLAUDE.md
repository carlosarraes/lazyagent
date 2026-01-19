# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Lazyagent is a single-binary TUI application built with Rust + ratatui that orchestrates multiple AI coding agents across multiple projects simultaneously. Think "lazygit but for autonomous task execution and PR reviews."

**Core concept**: Multiple Claude-based agents work concurrently on different tasks, each in its own git worktree, with live progress monitoring in a clean terminal UI controlled via vim-like keybindings.

## Build Commands

This is a new codebase that will be bootstrapped soon. Once initialized:

```bash
# Build the project
cargo build

# Run the application
cargo run

# Run in release mode
./target/release/lazyagent

# Run tests
cargo test

# Run tests for a specific module
cargo test <module_name>

# Run clippy (linting)
cargo clippy

# Format code
cargo fmt
```

## Architecture

### Module Structure

The codebase should be organized into these core modules:

- **config**: Configuration file parsing and defaults (TOML from `~/.config/lazyagent/config.toml`)
- **state**: Runtime state management for projects and agents
- **ui**: TUI layout and rendering (ratatui + crossterm)
- **git**: Git operations (worktrees, branches, status checks)
- **agent**: Agent lifecycle management (spawn, monitor, kill)
- **persist**: State persistence (`~/.local/state/lazyagent/`)

### Key Dependencies

- `ratatui` + `crossterm` - Terminal UI
- `serde` + `serde_yaml` + `toml` - Config/task file parsing
- `clap` - CLI argument parsing
- `anyhow` + `thiserror` - Error handling
- `chrono` - Timing and timestamps

### Data Flow

1. **Config loading** → Parse TOML config, validate project paths and settings
2. **State restoration** → Load persisted agent state from disk on startup
3. **TUI rendering** → Display projects header, agent list (left), agent output (right)
4. **Agent spawning** → Create git worktree → Spawn Claude CLI → Stream output to log file
5. **State updates** → Agent status transitions trigger UI re-renders and state persistence
6. **Agent completion** → Optional PR creation via `gh` → Cleanup or preserve worktree

### Critical Design Patterns

#### Git Worktrees Per Agent
Each agent gets its own isolated worktree to avoid conflicts. Branch naming follows `lazyagent/<task-id-or-description>`.

```rust
// Pseudocode example
let worktree_path = create_worktree(&project.repo_path, &base_branch)?;
let agent = spawn_claude_process(&worktree_path, &prompt)?;
```

#### Process Management
Store PIDs for all agents. Graceful shutdown on cancel (SIGTERM before SIGKILL). Stream stdout/stderr to per-agent log files for live display.

#### State Persistence
All runtime state (agent list, logs, worktrees, PR info) persists to `~/.local/state/lazyagent/`. On restart, reload and display all agents/logs.

#### Task YAML Schema
```yaml
tasks:
  - title: "Task description"
    completed: false
    parallel_group: 1  # Optional: tasks in same group can run concurrently
```

Agents update `completed: true` in place. Orchestrator only reads for display.

### Claude CLI Invocation

Use this exact command pattern for spawning agents:

```bash
claude --dangerously-skip-permissions \
       --verbose \
       --output-format stream-json \
       -p "<prompt>"
```

Parse `stream-json` events to extract:
- Agent progress indicators
- Tool usage (for step detection like "Implementing", "Testing", "Committing")
- Token usage from final result event
- Error conditions

### UI Layout

```
┌──Projects──[1 2/3]─[2 1/3]─[3 0/3]──┬───────────────────────────────┐
│ Agents:                             │ Agent Output (selected)       │
│ [R] #12 running 02:14               │                               │
│ [D] #11 done    14:22               │  ... live stream + history... │
│ [E] #10 error   00:32               │                               │
│                                     │                               │
└─────────────────────────────────────┴───────────────────────────────┘
```

**Left pane**: Agent list with status icon, ID, state, elapsed time
**Right pane**: Live-streaming output from selected agent's log file
**Header**: Numbered project tabs showing `current/max` agent count per project

### Keybindings

| Key     | Action                                      |
|---------|---------------------------------------------|
| `j`/`k` | Navigate agent list                         |
| `h`/`l` | Switch focus between panes                  |
| `1`-`9` | Switch current project                      |
| `n`     | Spawn new task agent (if below limit)       |
| `r`     | Spawn review agent (max iterations)         |
| `R`     | Spawn review agent (1 iteration)            |
| `d`     | Delete selected agent + history/logs        |
| `x`     | Cancel/stop selected agent (keep history)   |
| `l`     | Adjust per-project concurrency limit        |
| `?`     | Show help/shortcuts                         |
| `q`     | Quit (preserves state)                      |

## Development Guidelines

### Agent State Machine

Agent states: `queued` → `running` → `done` / `error` / `canceled`

Transitions:
- Spawn (`n`) → queued → running (when slot available)
- Cancel (`x`) → canceled (graceful SIGTERM)
- Delete (`d`) → remove from state, cleanup worktree/logs
- Process exit → done/error based on exit code

### Review Workflow

Review agents iterate: `review → fix → review` up to `max_iterations` (default: 3).

They operate on the PR created by the task agent, parsing review feedback and applying fixes in the same worktree.

### Per-Project Concurrency

Enforce `max_parallel` (default: 3) per project. Queue agents when at limit. Update header to show `current/max` for each project.

### Error Handling

- API errors → retry with exponential backoff
- Git failures → surface error in agent status, preserve logs
- Process crashes → mark agent as error state, keep logs for debugging

### Testing Strategy

Focus tests on:
- Config parsing (valid/invalid TOML)
- Tasks YAML parsing and task selection
- Agent state transitions
- Git worktree creation/cleanup
- State persistence and restoration

### File Size Limits

**No file should exceed 500 lines.** Exception: test files can exceed this limit.

When a file approaches this limit, refactor by:
- Extracting related functions into new modules
- Moving complex logic into separate helper modules
- Splitting UI components into smaller, focused files
- Creating trait implementations in separate files

This keeps the codebase maintainable and easier to navigate.

## Configuration

Config lives at `~/.config/lazyagent/config.toml`:

```toml
[ui]
refresh_ms = 200

[agent]
engine = "claude"           # Only Claude supported in v1
max_iterations = 3          # For review loops
auto_pr = true              # Auto-create PRs via gh
draft_pr = false            # Create as draft

[[projects]]
name = "project-1"
repo_path = "/path/to/repo"
tasks_yaml = "/path/to/repo/tasks.yaml"
base_branch = "main"
max_parallel = 3
```

## Bootstrap Context

This codebase is brand new. The `tasks.yaml` file contains the full implementation roadmap organized into parallel groups. Use `ralphy.sh --yaml tasks.yaml --parallel` to bootstrap the initial implementation with multiple concurrent agents.

The PRD (PRD.md) provides comprehensive product requirements. Reference it for design decisions and feature scope.

## Non-Goals (v1)

- Multi-engine support (only Claude)
- Complex scheduling beyond per-project limits
- Web UI or graphical interface
- Automatic task prioritization (agent decides)
