#!/usr/bin/env bash
# Talos installer — the TUI, its decision engine, and a control plane.
#
# Provisions, in order:
#   1. prerequisites: tmux >= 3.2, a Rust toolchain, at least one agent CLI
#   2. the binaries: builds talos + talos-cli, installs them to ~/.local/bin
#   3. the control plane: seeds registry/ + orchestration/ + TALOS.md into a
#      checkout (default ~/Code/code-documentation)
#   4. the lead session: creates 📡 Talos Mission Control over that checkout
#
# Idempotent: re-run after a pull to refresh the binaries; an existing control
# plane and lead session are left alone.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$here"

fail() { echo "✗ $*" >&2; exit 1; }
step() { printf '→ %s\n' "$*"; }

# --- 1. prerequisites -----------------------------------------------------------

step "checking prerequisites"

command -v tmux >/dev/null 2>&1 || fail "tmux not found (need >= 3.2)"
tmux -V | awk '{ split($2, v, "."); if (v[1] < 3 || (v[1] == 3 && v[2] < 2)) exit 1 }' \
  || fail "tmux too old: $(tmux -V) (need >= 3.2)"

command -v cargo >/dev/null 2>&1 || fail "cargo not found — install Rust from https://rustup.rs"

# --- 2. binaries -----------------------------------------------------------------

step "initialising the spec-harness-kit submodule"
# Non-recursive on purpose: the harness's own plugs/aton submodule points at a
# private corporate repo that a talos user has no access to. The public harness
# (agents, skills, rules, plugs/personal) works without it, and the harness
# installer skips a plug it cannot find.
git submodule update --init --depth 1 spec-harness-kit || true

step "building talos"
cargo build --release
mkdir -p "$HOME/.local/bin"
install -m755 target/release/talos "$HOME/.local/bin/talos"
install -m755 target/release/talos-cli "$HOME/.local/bin/talos-cli"
export PATH="$HOME/.local/bin:$PATH"

# Stage the harness beside the data, so a binary installed without this
# checkout can still find it (the startup sync looks in <data>/harness).
if [[ -d "$here/spec-harness-kit/agents" ]]; then
  step "staging the harness for the startup sync"
  data="${XDG_DATA_HOME:-$HOME/.local/share}/talos"
  mkdir -p "$data"
  rm -rf "$data/harness"
  cp -r "$here/spec-harness-kit" "$data/harness"
  # The staged copy is content, not a checkout: drop the git metadata and the
  # private plug's empty placeholder.
  rm -rf "$data/harness/.git" "$data/harness/plugs/aton"
fi

# --- 3. control plane -----------------------------------------------------------

control_plane="${TALOS_CONTROL_PLANE:-$HOME/Code/code-documentation}"
if [[ -d "$control_plane" ]]; then
  step "seeding control plane"
  ./scripts/seed-control-plane.sh "$control_plane"
else
  echo "⚠ no control-plane checkout at $control_plane — skipping."
  echo "  create a git repo there and re-run, or set TALOS_CONTROL_PLANE." >&2
fi

# --- 4. lead session -------------------------------------------------------------

if [[ -d "$control_plane" ]]; then
  lead_agent="${TALOS_LEAD_AGENT:-claude}"
  lead_name="📡 Talos Mission Control"
  if talos-cli session list --json 2>/dev/null | grep -q "Talos Mission Control"; then
    step "lead session already exists — left alone"
  else
    step "creating the lead session"
    talos-cli session create --name "$lead_name" --repo-path "$control_plane" --agent "$lead_agent" \
      || echo "⚠ lead session not created (is the '$lead_agent' CLI installed?) — create it from the TUI with Ctrl+N" >&2
  fi
fi

# --- 5. done ---------------------------------------------------------------------

cat <<'EOF'

✓ Talos installed.

  talos              the TUI: sessions, the board (F6), attention (F7), fleet (F8)
  talos-cli          headless: sessions, tasks, mailbox, watch, review,
                     and the decision engine: `talos-cli jev --help`
  lead session       📡 Talos Mission Control — opens the control plane

  Run `talos` to start. Ctrl+N creates a session; F6/F7/F8 cycle the panes.

EOF
