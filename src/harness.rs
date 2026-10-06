//! spec-harness-kit sync: keep the CLI agent directories in step with the
//! harness on every start.
//!
//! The harness (agents, skills, rules, workspace plugs) is the source of truth
//! for the workforce. talos ships it as a submodule and re-installs it into
//! each coding CLI's global agent/skill/rule directories whenever its contents
//! changed — so a `git submodule update` (or a `git pull` inside it) reaches
//! every CLI without a manual install step.
//!
//! The installer is the harness's own `scripts/install.sh`, not a copy of its
//! logic: the harness owns how it is laid down, and re-implementing that here
//! would drift. What this module owns is *when* — gated on a content stamp so
//! an unchanged harness costs one directory walk and no subprocess.
//!
//! Best-effort by design, like the extension self-heal it sits beside: a
//! missing harness, a missing `bash`, or a failing installer is a notice, never
//! a reason the interface fails to start.

use std::path::{Path, PathBuf};

/// The subdirectories whose contents define "the harness changed". `plugin.json`
/// is hashed too (it names the supported CLIs), but these four carry the
/// workforce.
const WATCHED: &[&str] = &["agents", "skills", "rules", "plugs"];

/// Where the harness lives, first match wins:
///
/// 1. `TALOS_HARNESS_DIR` — an explicit override, for tests and odd layouts.
/// 2. `<data dir>/harness` — a copy installed alongside talos (a prebuilt
///    binary has no source checkout to point at).
/// 3. `<crate>/spec-harness-kit` — the git submodule, for a source build.
fn harness_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("TALOS_HARNESS_DIR") {
        let p = PathBuf::from(dir);
        if is_harness(&p) {
            return Some(p);
        }
    }
    if let Some(data) = crate::paths::data_directory() {
        let p = data.join("harness");
        if is_harness(&p) {
            return Some(p);
        }
    }
    let submodule = Path::new(env!("CARGO_MANIFEST_DIR")).join("spec-harness-kit");
    if is_harness(&submodule) {
        return Some(submodule);
    }
    None
}

/// A directory is a harness if it carries the installer and at least the agents.
fn is_harness(dir: &Path) -> bool {
    dir.join("scripts/install.sh").is_file() && dir.join("agents").is_dir()
}

/// The stamp recording the harness contents last installed.
fn stamp_file() -> Option<PathBuf> {
    crate::paths::data_directory().map(|d| d.join("harness.stamp"))
}

/// A content stamp over the watched directories: path, size and mtime of every
/// file, folded with FNV-1a. Content-hash would be stricter but reads every
/// byte; mtime+size is what `make` has always used and catches every edit and
/// checkout that matters, at a directory walk's cost.
fn stamp(dir: &Path) -> u64 {
    fn fold(hash: &mut u64, bytes: &[u8]) {
        for b in bytes {
            *hash ^= *b as u64;
            *hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    fn walk(hash: &mut u64, base: &Path, dir: &Path, fold: &mut dyn FnMut(&mut u64, &[u8])) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        // Sorted so the stamp is stable across readdir order.
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            fold(hash, rel.to_string_lossy().as_bytes());
            if path.is_dir() {
                walk(hash, base, &path, fold);
            } else if let Ok(meta) = path.metadata() {
                let secs = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                fold(hash, &meta.len().to_le_bytes());
                fold(hash, &secs.to_le_bytes());
            }
        }
    }

    let mut hash: u64 = 0xcbf29ce484222325;
    for sub in WATCHED {
        let p = dir.join(sub);
        fold(&mut hash, sub.as_bytes());
        walk(&mut hash, dir, &p, &mut fold);
    }
    if let Ok(meta) = dir.join("plugin.json").metadata() {
        fold(&mut hash, b"plugin.json");
        fold(&mut hash, &meta.len().to_le_bytes());
    }
    hash
}

/// A JSON status of the harness: where it is, whether a sync is pending, and
/// why not if it cannot be found. Changes nothing.
pub fn status() -> serde_json::Value {
    let dir = harness_dir();
    let Some(dir) = dir else {
        return serde_json::json!({
            "found": false,
            "detail": "no spec-harness-kit found (set TALOS_HARNESS_DIR, or init the submodule)",
        });
    };
    let current = stamp(&dir);
    let recorded = stamp_file()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| s.trim().parse::<u64>().ok());
    let pending = recorded != Some(current);
    serde_json::json!({
        "found": true,
        "dir": dir.display().to_string(),
        "pending_sync": pending,
        "detail": if pending {
            format!("{} — harness changed; run `talos-cli harness sync`", dir.display())
        } else {
            format!("{} — up to date", dir.display())
        },
    })
}

/// Run the harness's own installer. Returns its combined output on success and
/// the error string on failure. `force` skips the stamp gate.
pub fn sync(force: bool) -> Result<String, String> {
    let dir = harness_dir().ok_or_else(|| {
        "no spec-harness-kit found (set TALOS_HARNESS_DIR, or init the submodule)".to_string()
    })?;

    let stamp_path = stamp_file();
    let current = stamp(&dir);
    if !force {
        if let Some(path) = &stamp_path {
            if let Ok(previous) = std::fs::read_to_string(path) {
                if previous.trim() == current.to_string() {
                    return Ok("harness unchanged; nothing to do".to_string());
                }
            }
        }
    }

    let installer = dir.join("scripts/install.sh");
    let output = std::process::Command::new("bash")
        .arg(&installer)
        .current_dir(&dir)
        .output()
        .map_err(|e| format!("could not run {}: {e}", installer.display()))?;

    // Record the stamp the installer saw, so the next start with no further
    // edits skips. Written even on a failed install would wrongly silence a
    // retry, so only on success.
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr.lines().rev().take(5).collect::<Vec<_>>().join("\n");
        return Err(format!(
            "harness install exited {}: {}",
            output.status.code().unwrap_or(-1),
            tail.trim()
        ));
    }

    if let Some(path) = &stamp_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, current.to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let copied = stdout.matches("  Copied agent").count()
        + stdout.matches("  Copied plug agent").count();
    Ok(format!("harness synced from {} ({copied} agent(s))", dir.display()))
}

/// Startup entry point: sync if the harness changed, and turn any outcome into
/// a one-line notice. Never fails; never blocks boot for long (an unchanged
/// harness does not shell out at all).
pub fn sync_on_startup() -> Vec<String> {
    if harness_dir().is_none() {
        return Vec::new();
    }
    match sync(false) {
        Ok(msg) => {
            // "unchanged" is the common case and not worth a notice.
            if msg.starts_with("harness unchanged") {
                Vec::new()
            } else {
                vec![msg]
            }
        }
        Err(e) => vec![format!("harness sync skipped: {e}")],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_harness(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("talos-harness-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("agents")).unwrap();
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("agents/dev.md"), "---\nname: dev\n---\n").unwrap();
        // A trivial installer so the test does not need the real harness.
        std::fs::write(
            dir.join("scripts/install.sh"),
            "#!/bin/bash\necho '  Copied agent: dev.md'\n",
        )
        .unwrap();
        std::fs::write(dir.join("plugin.json"), "{}\n").unwrap();
        dir
    }

    #[test]
    fn stamp_changes_when_a_file_changes() {
        let dir = temp_harness("stamp");
        let a = stamp(&dir);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(dir.join("agents/qa.md"), "---\nname: qa\n---\n").unwrap();
        let b = stamp(&dir);
        assert_ne!(a, b, "a new agent must move the stamp");

        std::thread::sleep(std::time::Duration::from_millis(1100));
        std::fs::write(dir.join("agents/dev.md"), "---\nname: dev\nreview: true\n---\n").unwrap();
        let c = stamp(&dir);
        assert_ne!(b, c, "an edited agent must move the stamp");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_non_harness_directory_is_not_picked_up() {
        let dir = std::env::temp_dir().join(format!("talos-not-harness-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(!is_harness(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
