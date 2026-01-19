# Lazyagent PRD

## Overview
Lazyagent is a single-binary TUI (Rust + ratatui) that orchestrates multiple AI coding agents across multiple projects, similar to lazygit but for autonomous task execution and PR reviews.

The UI shows agent status and timing on the left, and live agent output on the right. Users control everything with vim-like keybindings and can spawn multiple agents per project, switch projects by number keys, and trigger review loops.

## Goals
- Orchestrate multiple Claude-based agents concurrently across multiple repos.
- Provide a clean TUI with live logs, per-agent history, and fast keyboard control.
- Use git worktrees per agent, one task per branch, optional auto-PR via `gh`.
- Persist project list and agent history across restarts until user deletes an agent.
- Keep tasks in a YAML file that the agent updates in place (`completed: true`).

## Non-goals (v1)
- Support engines other than Claude.
- Complex scheduling beyond per-project concurrency limits.
- Graphical UI; no web UI.
- Automatic task prioritization by the orchestrator (LLM decides).

## Assumptions & Dependencies
- `claude` CLI available in PATH.
- `gh` CLI available for PR creation.
- Git repo per project.
- Network access as needed by the agent for its own workflow.

## User Experience
### Layout
- Header row: projects bar with numbered slots and per-project usage/limit, e.g. `[1 2/3] [2 1/3] [3 0/3]`, with current project highlight.
- Left pane: agent list (status, project, branch/worktree, elapsed time).
- Right pane: live output for selected agent; scrollable history.
- Minimal status hints in the header/footer.

```
┌──Projects──[1 2/3]─[2 1/3]─[3 0/3]──┬───────────────────────────────┐
│ Agents:                 │ Agent Output (selected)                  │
│ [R] #12 running 02:14   │                                          │
│ [D] #11 done    14:22   │  ... live stream + history ...           │
│ [E] #10 error   00:32   │                                          │
│                         │                                          │
│                         │                                          │
│                         │                                          │
│                         │                                          │
└─────────────────────────┴──────────────────────────────────────────┘
```

### Keybindings (vim-style)
- `j`/`k`: move selection up/down in agent list.
- `h`/`l`: switch focus between panes (list/log).
- `1`-`9`: switch current project.
- `n`: spawn a new agent for current project.
- `r`: spawn review agent (uses default max iterations).
- `R`: spawn review agent for 1 iteration.
- `d`: delete selected agent (kills if running, removes history).
- `x`: cancel/stop selected agent (keeps history until `d`).
- `l`: adjust per-project concurrency limit.
- `?`: show help/shortcuts.
- `q`: quit (preserves state).

## Functional Requirements
### Projects
- Config-driven list of projects with:
  - repo path
  - tasks YAML path
  - base branch (default: current)
  - per-project max parallel (default: 3)
  - auto-PR toggle and draft flag
  - Claude prompt template path (optional)
- Project switching via number keys.

### Tasks YAML
Only YAML is supported in v1.
```
tasks:
  - title: "Create User model"
    completed: false
    parallel_group: 1
```
- Agent updates `completed: true` in place.
- Orchestrator reads for display only.

### Agent Lifecycle
States: queued, running, done, error, canceled.
- `n` spawns a new agent if below per-project limit.
- Each agent uses a git worktree and its own branch.
- Branch naming defaults to `lazyagent/<task-id-or-description>` as decided by the agent.
- The agent is responsible for selecting the next task and updating YAML.
- `d` removes agent and deletes its history/logs; if running, stop gracefully first.

### Review Workflow
- `r` spawns a review agent that loops: review -> fix -> review, up to max iterations (default: 3).
- `R` runs a single review iteration.
- Review agents operate on the PR created by the task agent.

### Process Management
- Each agent is a separate process with a stored PID.
- Output is streamed to per-agent log files and shown live in the UI.
- Graceful shutdown on cancel or delete.

### Claude Invocation (v1)
- Use Claude CLI only:
  - `claude --dangerously-skip-permissions --verbose --output-format stream-json -p "<prompt>"`
- Store stdout/stderr to log file.
- Parse stream-json events where helpful (optional in v1).

### PR Creation
- Auto-create PR via `gh` when enabled in config.
- Store PR link/number in agent state for display and review workflow.

### Persistence
- Store runtime state in `~/.local/state/lazyagent/`.
- Persist:
  - project list and last active project
  - agents (status, log path, worktree path, branch, PR info)
- On startup, reload state and show existing agents/logs.
- `d` removes agent state and deletes its logs/worktree as configured.

## Config (TOML)
Path: `~/.config/lazyagent/config.toml`
Example:
```
[ui]
refresh_ms = 200

[agent]
engine = "claude"
max_iterations = 3
auto_pr = true
draft_pr = false

[[projects]]
name = "project-1"
repo_path = "/path/to/repo"
tasks_yaml = "/path/to/repo/tasks.yaml"
base_branch = "main"
max_parallel = 3
```

## Data Model (high level)
- Project: id, name, repo_path, tasks_yaml, base_branch, max_parallel, auto_pr.
- Agent: id, project_id, status, pid, started_at, elapsed, log_path, worktree_path, branch, pr_number, is_review, iteration, max_iterations.

## Success Criteria
- Spawn multiple concurrent agents per project; observe logs live.
- Switch projects instantly and monitor agent statuses.
- Agents create branches/worktrees and (optionally) PRs.
- Review loop runs to completion or max iterations.
- Restarting TUI restores agent list and logs.

## Open Questions
- None for v1.
