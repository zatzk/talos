# Homebrew formula for talos.
#
# This is the canonical source, kept in the main repo. CI (the
# `publish-homebrew` job in .github/workflows/cd.yml) copies it to the
# Thurbeen/homebrew-talos tap on every release, with `version` and the
# per-platform `sha256` values bumped to that release. The values below are
# a last-known-good template — CI overrides them per release.
#
#   brew install thurbeen/talos/talos
#
# Only the platforms with a published release artifact are supported:
#   - macOS arm64 (Apple Silicon) -> aarch64-apple-darwin
#   - Linux x86_64                -> x86_64-unknown-linux-musl (static)
# Intel macOS and aarch64 Linux have no release binary, so they are omitted
# (brew reports "no available formula" on those platforms).
class Talos < Formula
  desc "TUI for orchestrating multiple coding-agent CLI sessions in persistent tmux panels"
  homepage "https://github.com/Thurbeen/talos"
  version "0.79.46"
  license "MIT"

  depends_on "git"
  depends_on "tmux"

  on_macos do
    on_arm do
      url "https://github.com/Thurbeen/talos/releases/download/v#{version}/talos-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "ac7742c30cfa0e36557e2449d9b900c553e4feb30e0f154ada0b8ab2e61ae421"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/Thurbeen/talos/releases/download/v#{version}/talos-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "182ebaf3a3842bc76bbf0bbe536300bdcfb5ced2c4a302668d4590276d4d51a3"
    end
  end

  def install
    # The release tarball ships both maintained binaries plus LICENSE; install
    # only the binaries (Homebrew records the license from the formula).
    bin.install "talos"
    bin.install "talos-cli"
  end

  def caveats
    <<~EOS
      talos needs tmux >= 3.2 and a coding-agent CLI (claude, codex, antigravity,
      opencode, aider, …) on your PATH. Launch the TUI with `talos`; the
      scriptable headless interface is `talos-cli`.
    EOS
  end

  test do
    # The TUI (`talos`) has no headless mode, so only assert it is installed
    # and executable. `talos-cli` is a clap CLI: `--version` exits 0 and
    # prints a semver-shaped marker (the build-time release version is injected
    # into the TUI's status bar, not into clap's CARGO_PKG_VERSION).
    assert_predicate bin/"talos", :executable?
    assert_match(/\d+\.\d+\.\d+/, shell_output("#{bin}/talos-cli --version"))
  end
end
