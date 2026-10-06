# Packaging

Optional OS-level integration and distribution packages for talos.

## Distribution packages

| Directory   | Channel                  | Installs                                  |
| ----------- | ------------------------ | ----------------------------------------- |
| `homebrew/` | Homebrew tap (macOS/Linux) | prebuilt release binaries — see [`homebrew/README.md`](homebrew/README.md) |
| `aur/`      | Arch Linux (AUR)         | source + prebuilt binary — see [`aur/README.md`](aur/README.md) |
| `chocolatey/` | Chocolatey (Windows)   | prebuilt release zip — see [`chocolatey/README.md`](chocolatey/README.md) |
| `winget/`   | winget (Windows)         | prebuilt release zip (portable) — see [`winget/README.md`](winget/README.md) |
| `../flake.nix` | Nix flake (Linux/macOS) | source build of any ref, not tied to a release — see [`../nix/package.nix`](../nix/package.nix) |

All but the flake are published automatically on each release by jobs in
[`.github/workflows/cd.yml`](../.github/workflows/cd.yml). For the
distro-agnostic one-liner installer, see
[`scripts/install.sh`](../scripts/install.sh).

## Reboot-proof automations (`systemd/`, `launchd/`)

Talos automations fire from two places out of the box:

1. **The TUI tick loop** — while the TUI is open.
2. **A tmux heartbeat keeper window** (`automation-heartbeat`) — armed
   automatically on TUI startup and on `talos-cli automation create`. It runs
   `talos-cli automation tick` every 60 s and keeps the tmux server alive, so
   automations fire even after you close the TUI.

The keeper covers the common case but is **not reboot-proof** (a reboot or
`tmux kill-server` ends it until talos runs again). For guaranteed,
session-independent firing, enable one of these units, which run the same
`talos-cli automation tick` on a timer. Firing is **claim-based** (atomic
compare-and-swap on `next_run_at`), so running the TUI, the tmux keeper, and one
of these timers simultaneously never double-fires an automation.

- **Linux**: `systemd/talos-automations.{service,timer}` — see the header
  comment in the `.service` file for install steps.
- **macOS**: `launchd/dev.talos.automations.plist` — see the comment block in
  the plist for install steps.

Both default to a 1-minute cadence; adjust the timer/`StartInterval` to taste.
