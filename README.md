# Talos 🛰️

> **Chat-Led Multi-Agent Orchestration Cockpit — conversational routing, unified memory, and automated spec-driven execution.**
>
> Built on the foundations of [thurbox](https://github.com/Thurbeen/thurbox), octomux and autonomous-cockpit: unified workspaces, persistent
> tmux panels, worktrees, universal runner (API & CLI), built-in decision engine (**Jev**), and git submodule control plane.

```
talos              the TUI — Chat (F1), Workspace Board (F2), Fleet (F3), Selector (F4/Ctrl+O)
talos-cli          headless driver + the Jev decision engine + universal runner
```

## Install

```bash
git clone git@github.com:zatzk/talos.git ~/Code/talos
cd ~/Code/talos
./install.sh          # builds talos + talos-cli, syncs submodules, ensures control plane
```

Requires `tmux >= 3.2`, Rust, and at least one agent CLI.

## Key Navigation & Shortcuts (Talos v3)

| Shortcut | Alternative | Action | Description |
|---|---|---|---|
| `F1` | `Alt+1` | **Chat / Threads** | Primary conversational interface with intent routing |
| `F2` | `Alt+2` | **Workspace Board** | Kanban view of atomic tasks for active workspace |
| `F3` | `Alt+3` | **Fleet / Terminals** | Workers and tmux terminals for human inspection |
| `F4` | `Ctrl+O` | **Model/Agent Selector** | Fast modal to select between Auto (Jev), API Models, and CLI Agents |
| `Ctrl+A` | — | **Approve Spec & Commit** | Ingest canonical tasks into Board and commit to `code-documentation` |
| `Ctrl+N` | `Ctrl+T` | **New Thread** | Open a new conversational thread in the workspace |
| `Ctrl+W` | — | **Switch Workspace** | Switch between workspaces and repositories |
| `Ctrl+P` | — | **Command Palette** | Quick action palette |
| `Ctrl+Q` | — | **Quit Talos** | Exit TUI preserving background workers |
| `Esc` | — | **Cancel / Abort** | Dismiss modal or cancel running execution stream |

## Architecture & Features

### 1. Jev — Woven Decision Engine (1200ms Timeout)

Decisions occur natively at critical pipeline stages with deterministic heuristic fallbacks:

| Lifecycle Point | What Jev Decides |
|---|---|
| Chat Message | **Intent Routing** (Persona, Model Tier, Execution Backend: API vs CLI) |
| Task Created | **Sizing** (S/M/L/EPIC, pipeline, model tier, target agent) |
| Before Dispatch | **Atomicity** (one commit/PR or needs decomposition) |
| Shell Command | **Guardrail** (SAFE / RISKY / BLOCKED static rules + model) |
| Tests Fail | **Triage** (root cause + healing strategy for retry) |
| Spec Generated | **Audit-Spec** (APPROVED / NEEDS_REVISION / REJECTED) |

### 2. Universal Runner & Unified Memory

- **API Direct Runner:** Low latency SSE streaming via OpenRouter/9Router.
- **Headless CLI Runner:** Executes local CLI agents (`claude -p`, `codex exec`, `agy --headless`).
- **Unified 4-Layer Memory:** Episodic, Semantic (Embeddings), Procedural (Facts/Preferences), Entity Graph with **Reciprocal Rank Fusion (RRF)** combining FTS5 BM25 and cosine similarity.

### 3. Control Plane Submodule (`code-documentation`)

The control plane is embedded via git submodule (`code-documentation`), preserving shared PRDs, RFCs, and task breakdowns across projects. Approving a spec (`Ctrl+A`) writes tasks, updates the SQLite Kanban board, and performs an automated git commit.

## Agent-agnostic

Talos launches the CLI unmodified — Claude Code, Codex, **antigravity** (`agy`),
**opencode**, aider, copilot, vibe, pi, omp, or anything you describe in
`~/.config/talos/agents.toml`. The status hooks that make the panes work are
wired per agent. 9Router / OpenRouter / Anthropic are backends those CLIs are
configured to use; talos is neutral about which.

## The harness (spec-harness-kit)

Agents, skills and rules come from
[spec-harness-kit](https://github.com/zatzk/spec-harness-kit), vendored as a
submodule at `spec-harness-kit/`. **Every time talos starts, it re-installs the
harness into each CLI's agent/skill/rule directories if its contents changed** —
so a `git submodule update` reaches every coding CLI with no manual step.

- Gated on a content stamp (`<data>/harness.stamp`): an unchanged harness costs
  one directory walk, not an install.
- The installer is the harness's own `scripts/install.sh`, so the harness owns
  how it is laid down.
- Run it by hand, or check what it would do:

```bash
talos-cli harness sync          # install if changed
talos-cli harness sync --force  # reinstall regardless
talos-cli harness status        # where it is, and whether a sync is pending
```

The harness's `plugs/aton` is a private corporate plug and stays uninitialised
by default; the public harness works without it.

## The lead (orchestrator)

Talos's orchestration is the control-plane pattern: a long-lived **lead**
session — **📡 Talos Mission Control** — over a control-plane checkout
(default `~/Code/code-documentation`: `registry/` + `orchestration/` playbooks,
run logs, session profiles). You talk to the lead; it dispatches **workers**,
each in its own git worktree, and coordinates them through the mailbox.

**The lead is ensured on every start**, before the interface takes the terminal:
if the control-plane checkout or the session is missing, talos seeds/spawns it,
and pins the row above the rest of the fleet. So `talos` always opens with the
orchestrator present.

`Ctrl+N` is *not* the orchestration flow — it is thurbox's "attach one agent to
one session" primitive. Orchestration happens through the lead, not by hand.

## The interface

Every pane is a Lua file under `~/.config/talos/ui/` you can edit, and **F10**
reloads it. The inherited interface (session list, agent terminal, global
search, worktrees, the `Ctrl+X` diff reviewer, themes, automations, the
inter-session mailbox) is documented in the
[thurbox docs](https://thurbox.thurbeen.eu/docs/); this fork renames the binary
and the config/state directories (`~/.config/talos/`, `~/.local/share/talos/`)
and adds the three panes above.

## Layout

```
talos/
├── src/
│   ├── jev/            the decision engine (6 cases + client + fallbacks)
│   ├── kernel/         the Lua plugin host (bundled UI, layout, commands)
│   ├── backend/        tmux / psmux / rmux session backends
│   ├── session_ops/    spawn, restore, worktrees, extensions
│   └── cli/            talos-cli (incl. `jev`)
├── ui/plugins/         85_kanban, 86_attention, 87_pipeline (the three panes)
├── control-plane/      template seeded into the control-plane checkout
├── scripts/            seed-control-plane.sh
└── install.sh
```

## License

MIT. Talos is a fork of [thurbox](https://github.com/Thurbeen/thurbox) by
Thurbeen; the original copyright and license are preserved.
