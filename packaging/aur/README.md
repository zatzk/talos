# Arch Linux (AUR) packaging

Two packages are provided here:

| Directory      | AUR name      | What it does                                              |
| -------------- | ------------- | -------------------------------------------------------- |
| `talos/`     | `talos`     | Builds from the release **source** tarball with `cargo`. |
| `talos-bin/` | `talos-bin` | Installs the **prebuilt** x86_64 (musl) release tarball.  |

Both install two binaries — `talos` (TUI) and `talos-cli` — plus the
MIT `LICENSE`. They `provides`/`conflicts` each other, so only one can be
installed at a time.

## Why two packages

- `talos` is the portable, idiomatic choice (any arch `cargo` targets).
  Its `build()` exports `TALOS_RELEASE_VERSION="v$pkgver"` so the TUI
  reports the real version — without it `build.rs` falls back to the
  `0.0.0-dev` marker baked into `Cargo.toml`.
- `talos-bin` is fast and needs no Rust toolchain, but is x86_64-only
  (no aarch64-Linux release artifact is published) and uses the
  statically-linked musl tarball.

> Note: upstream's `talos-cli --version` reports `0.0.0-dev` in both
> packages — that binary takes its version from clap's `CARGO_PKG_VERSION`
> rather than the build-time injected `TALOS_VERSION`. The TUI status
> bar shows the correct version. This is an upstream quirk, not a
> packaging issue.

## Runtime dependencies

- `tmux` (>= 3.2) and `git` are required.
- The `talos` (source) package links the system `sqlite` — its
  `prepare()` drops rusqlite's vendored `bundled` SQLite so the binary
  uses `libsqlite3.so` (Arch discourages bundled libs, and the vendored
  static lib also fails to link under the default rust-lld). The
  `talos-bin` package is statically linked and needs no `sqlite`.
- A coding-agent CLI (claude-code, aider, opencode, …) is user-supplied
  and listed as `optdepends`.

## Build & test locally

```bash
cd talos          # or talos-bin
makepkg -f          # build the package
makepkg -si         # build and install
namcap PKGBUILD     # lint the recipe (if namcap is installed)
namcap *.pkg.tar.zst
```

## Automated publishing (CI)

New releases publish to the AUR **automatically**. The `publish-aur` job
in [`.github/workflows/cd.yml`](../../.github/workflows/cd.yml) runs after
the GitHub Release is created and, for each package:

1. bumps `pkgver` to the release version and resets `pkgrel=1`,
2. recomputes `sha256sums` (`updpkgsums`) from the freshly released sources,
3. regenerates `.SRCINFO`, and
4. commits + pushes to the package's AUR git repo.

The `pkgver`/`sha256sums` committed in this directory are therefore just a
template/last-known-good — CI overrides them per release. The job is a
no-op if nothing changed, and is skipped entirely when the SSH secret is
absent (e.g. on forks).

### One-time setup

The job needs an SSH key that is registered on the AUR account and stored
as the `AUR_SSH_PRIVATE_KEY` repository secret:

```bash
# 1. Generate a dedicated CI key (no passphrase)
ssh-keygen -t ed25519 -C "talos-ci@aur" -f aur_ci -N ""

# 2. Store the PRIVATE key as a GitHub Actions secret
gh secret set AUR_SSH_PRIVATE_KEY < aur_ci

# 3. Add the PUBLIC key (aur_ci.pub) to your AUR account at
#    https://aur.archlinux.org/account  ->  "SSH Public Key"
#    (paste it on a new line, keeping any existing keys)
```

The package must already exist on the AUR (initial import is manual, see
below). After that, every release updates it automatically.

## Manual publishing / initial import

The AUR holds each package in its own git repo. For the first import (or a
manual update):

```bash
git clone ssh://aur@aur.archlinux.org/talos.git aur-talos
cp talos/PKGBUILD talos/.SRCINFO aur-talos/
cd aur-talos
git commit -am "upgpkg: talos <version>"
git push
```

Repeat with `talos-bin/` against the `talos-bin.git` AUR repo.

When bumping manually: set `pkgver` (drop the `v` prefix), reset
`pkgrel=1`, run `updpkgsums`, regenerate `.SRCINFO`
(`makepkg --printsrcinfo > .SRCINFO`), and confirm with `makepkg -f`.
Pick a `pkgver` that has **published release assets** for `talos-bin`.
