# Talos 🛰️

> **Agentic Engineering TUI — agent-agnostic, with a woven decision engine and a spec-driven control plane.**
>
> A fork of [thurbox](https://github.com/Thurbeen/thurbox): the same persistent
> tmux sessions, worktrees, diff reviewer and Lua interface, plus three things
> thurbox deliberately leaves to the operator — a built-in decision engine
> (**Jev**), the octomux workflow panes, and a control plane.

```
talos              the TUI — sessions, the board (F6), attention (F7), fleet (F8)
talos-cli          headless driver + the Jev decision engine
```

## Install

```bash
git clone git@github.com:zatzk/talos.git ~/Code/talos
cd ~/Code/talos
./install.sh          # builds talos + talos-cli, seeds a control plane, makes the lead session
```

Requires `tmux >= 3.2`, Rust, and at least one agent CLI. On Arch, build the
`talos-bin`/`talos` AUR package instead (see `packaging/aur/`).

## What talos adds to thurbox

### 1. Jev — the decision engine, woven into the agent lifecycle

Not a tool you remember to call: decisions happen at the points where a
judgement is needed. Every case answers JSON and falls back to a deterministic
classifier when the API is unreachable, so the TUI never blocks.

| Lifecycle point | Case | What it decides |
|---|---|---|
| Task created | **sizing** | size (S/M/L/EPIC), pipeline, model tier, target agent |
| Before dispatch | **atomicity** | one commit/PR, or needs decomposition |
| Shell command | **guardrail** | SAFE / RISKY / BLOCKED (static rules first, then model) |
| Tests fail | **triage** | root cause + healing strategy for the self-healing retry |
| Spec generated | **audit-spec** | APPROVED / NEEDS_REVISION / REJECTED for a PRD or RFC |
| Chat message | **intent** | which persona, what the cockpit should do next |

Callable directly for hooks and scripts:

```bash
talos-cli jev sizing --title "add rate limiting" | jq
talos-cli jev guardrail --command "git push --force"     # BLOCKED
talos-cli jev triage --command "npm test" --exit-code 1 --test-output "$(npm test 2>&1)"
```

Set `OPENROUTER_API_KEY` to use the model path; see `.env.example`. Without it,
the heuristics answer (`provider: heuristic-fallback`).

### 2. Octomux panes — natively in the TUI

- **F6 — board**: the six-column workflow (`backlog → planned → in_progress →
  human_review → pr → done`). Columns are *derived* from session facts, so the
  board cannot drift from reality.
- **F7 — attention**: the needs-you inbox — blocked sessions and finished work
  with unreviewed diffs, in one list.
- **F8 — fleet**: the lead/worker tree with status, branch and diff shape.

### 3. The control plane

`install.sh` seeds a **control plane** (default `~/Code/code-documentation`): a
repo holding the plan and the log — `registry/` (the map), `orchestration/`
(playbooks, run logs, session profiles) — and creates the self-contained lead
session **📡 Talos Mission Control** over it. Workers are dispatched per task,
each in its own worktree, reporting back through the mailbox.

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
