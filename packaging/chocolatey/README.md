# Chocolatey packaging

Talos ships a [Chocolatey](https://chocolatey.org) package that installs the
**prebuilt** x86_64 Windows release binaries (`talos.exe` + `talos-cli.exe`)
from the GitHub Release and shims them onto your `PATH`.

```powershell
choco install talos
```

> **Status: live.** The package has been approved by the Chocolatey community
> moderators and resolves from the community feed:
> [community.chocolatey.org/packages/talos](https://community.chocolatey.org/packages/talos).
> New versions still go through community-repo moderation before they appear (see
> [Moderation](#automated-publishing-ci) below).

The canonical package source lives here:
[`talos.nuspec`](talos.nuspec) (metadata) plus
[`tools/chocolateyinstall.ps1`](tools/chocolateyinstall.ps1) (downloads the
release zip via `Install-ChocolateyZipPackage` and verifies its SHA256). The
`version`/`$url64`/`$checksum64` values committed here are a last-known-good
template — CI overrides them per release.

## Supported platforms

Windows x86_64 only — the single published Windows release artifact is
`talos-v<version>-x86_64-pc-windows-msvc.zip`. ARM64 Windows installs the
x86_64 build and runs it under x64 emulation (matching
[`scripts/install.ps1`](../../scripts/install.ps1)).

## Runtime dependencies

- **[psmux](https://github.com/psmux/psmux)** — the native-Windows terminal
  multiplexer talos drives (a drop-in tmux clone). There is **no Chocolatey
  package for psmux**, so it cannot be declared as a package `<dependencies>`
  entry; install it separately. This is documented in the package
  `<description>`, and `chocolateyinstall.ps1` emits a `Write-Warning` at
  install time when `psmux` isn't found on `PATH`.
- A coding-agent CLI (claude, codex, antigravity, opencode, aider, …) on your PATH.

## Test locally

On a Windows machine with Chocolatey installed:

```powershell
# Bump the template to a published release first (see below), then:
choco pack packaging\chocolatey\talos.nuspec --outputdirectory packaging\chocolatey
choco install talos -s packaging\chocolatey -y
talos-cli --version
choco uninstall talos -y
```

## Automated publishing (CI)

The `publish-chocolatey` job in
[`.github/workflows/cd.yml`](../../.github/workflows/cd.yml) runs on
`windows-latest` after the GitHub Release is created and, **when the throttle
window has elapsed** (below):

1. downloads the release `talos-<version>-checksums.txt`,
2. runs [`bump-nuspec.py`](bump-nuspec.py) to set the nuspec `<version>` and the
   install script's `$url64`/`$checksum64` from those checksums,
3. runs `choco pack` (the version comes from the bumped nuspec), then
4. `choco push`es the `.nupkg` to `https://push.chocolatey.org/`.

The `CHOCOLATEY_API_KEY` secret is **already configured** on the main talos
repo. The job is skipped only where the secret is absent (e.g. on forks). The
committed template files are not modified by CI — they stay as last-known-good,
exactly like the Homebrew formula template.

> **Throttled to one push per `THROTTLE_DAYS` (30 days).** The community repo
> moderates *and* rate-limits every push, so it can't absorb talos's
> per-`feat`/`fix`/`perf` release cadence — versions pile up in the moderation
> queue and `choco push` starts returning **403**. So the job first reads the
> community OData feed
> (`community.chocolatey.org/api/v2/Packages()`, filtered to the latest
> `talos` version) for the last-published version's age. If it is younger than
> `THROTTLE_DAYS` the job **skips the push and exits green** with a
> `::warning::`, coalescing the intervening patch releases into the next monthly
> Chocolatey version. The binary itself always ships immediately via GitHub
> Releases (and Homebrew/AUR/winget); only the Chocolatey channel lags. Tune the
> cadence via the `THROTTLE_DAYS` env in the job. A residual `403`/`409` (rate
> limit / already-pending) at push time is caught the same way — **green +
> warning** — so a backed-up channel never turns the whole release red; only a
> genuine failure (bad package, auth) fails red.
>
> **Moderation (chocolatey.org side, not CI).** The Chocolatey community
> repository holds new packages — and each new version — for human moderation
> before they go live. The automated `choco push` succeeds, but the package may
> sit *pending* on chocolatey.org until a moderator approves it; a brand-new
> package may also draw review comments to address in the chocolatey.org web UI.
> This is not a CI failure. The [`VERIFICATION.txt`](tools/VERIFICATION.txt) and
> a working checksum are moderation requirements and ship in the package.

## Manual publishing / initial import

Publishing is fully automated (above), so this is only a fallback — e.g. to
re-push a version or seed the package outside the release flow. To pack and push
by hand (on Windows):

```powershell
# Bump the local template to a published release.
$ver = "<version>"   # a tag with published release assets, e.g. 0.79.46
# curl.exe (not the `curl` alias for Invoke-WebRequest) ships on Windows 10+.
curl.exe -fsSL -o checksums.txt `
  "https://github.com/zatzk/talos/releases/download/v$ver/talos-v$ver-checksums.txt"
python packaging\chocolatey\bump-nuspec.py "v$ver" packaging\chocolatey checksums.txt

choco pack packaging\chocolatey\talos.nuspec --outputdirectory packaging\chocolatey
choco push packaging\chocolatey\talos.$ver.nupkg `
  --source https://push.chocolatey.org/ --api-key <your-api-key>
```

Pick a `<version>` that has **published release assets** (the package points at
a release zip).
