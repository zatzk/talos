#!/usr/bin/env bash
# Seed the talos control plane into a checkout.
#
# A control plane is a repo holding the plan and the log: registry/ (the map),
# orchestration/ (playbooks, run logs, session profiles), and the lead's
# standing context (TALOS.md). By default it seeds the code-documentation repo;
# point it anywhere with the first argument or TALOS_CONTROL_PLANE.
#
# Non-destructive: an existing file is left untouched, so a control plane you
# have already grown is never overwritten by a re-run.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
template="$here/control-plane"
target="${1:-${TALOS_CONTROL_PLANE:-$HOME/Code/code-documentation}}"

if [[ ! -d "$template" ]]; then
  echo "no control-plane template at $template" >&2
  exit 1
fi
if [[ ! -d "$target" ]]; then
  echo "control-plane target does not exist: $target" >&2
  echo "create it first (a git repo), or pass a path as the first argument" >&2
  exit 1
fi

echo "→ Seeding control plane into $target"
copied=0
while IFS= read -r rel; do
  src="$template/$rel"
  dst="$target/$rel"
  if [[ -e "$dst" ]]; then
    continue
  fi
  mkdir -p "$(dirname "$dst")"
  cp "$src" "$dst"
  copied=$((copied + 1))
done < <(cd "$template" && find . -type f | sed 's|^\./||')
echo "  $copied file(s) written"

# Generated control-plane artifacts stay out of git.
gitignore="$target/.gitignore"
for entry in "TALOS.rendered.md" "registry/repos.generated.yaml" "orchestration/session-profiles.local.yaml"; do
  if ! grep -qxF "$entry" "$gitignore" 2>/dev/null; then
    printf '%s\n' "$entry" >> "$gitignore"
  fi
done

echo "✓ Control plane seeded. The lead session opens $target."
