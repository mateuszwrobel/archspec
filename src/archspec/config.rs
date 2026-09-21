use serde::Deserialize;
use std::path::{Path, PathBuf};

/// Per-artefact output destinations from an optional `archspec.toml` at the
/// project root. A missing config file or missing key falls back to stdout;
/// `--output` always overrides the configured default.
#[derive(Debug, Default, Deserialize)]
pub struct Output {
    #[serde(default)]
    pub inspect: Option<String>,
    #[serde(default)]
    pub diagram: Option<String>,
    #[serde(default)]
    pub report: Option<String>,
    #[serde(default)]
    pub scan: Option<String>,
}

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    output: Output,
}

/// Load `archspec.toml` from `root`, or the default (everything unset) when
/// the file is absent. Malformed TOML is an operational error naming the file.
pub fn load(root: &Path) -> Result<Output, String> {
    let path = root.join("archspec.toml");
    if !path.exists() {
        return Ok(Output::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|err| format!("failed to parse config: {}: {err}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .map_err(|err| format!("failed to parse config: {}: {err}", path.display()))?;
    Ok(config.output)
}

/// Pick the destination: `--output` wins (kept cwd-relative and unchanged);
/// otherwise the configured default resolved against the project root; else
/// stdout (`None`).
pub fn resolve(flag: Option<&str>, configured: Option<&str>, root: &Path) -> Option<PathBuf> {
    if let Some(flag) = flag {
        return Some(PathBuf::from(flag));
    }
    configured.map(|path| root.join(path))
}

/// Write `content` to `path`, creating parent directories when the destination
/// is a nested path (e.g. a configured `docs/archspec/report.md` default).
pub fn write(path: &Path, content: &str, kind: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("failed to write {kind} to {}: {err}", path.display()))?;
        }
    }
    std::fs::write(path, content)
        .map_err(|err| format!("failed to write {kind} to {}: {err}", path.display()))
}

/// Terminal step of every output-producing command: write `content` to
/// `destination`, print it to stdout when there is no destination, or — under
/// `--check` — compare it byte-for-byte against the destination without writing.
/// Fresh exits `0`; stale/missing/absent-destination exits non-zero with an
/// actionable finding (the `regen` command shows how to regenerate).
pub fn emit(
    content: &str,
    destination: Option<PathBuf>,
    check: bool,
    kind: &str,
    regen: &str,
) -> Result<(), String> {
    match destination {
        Some(path) if check => check_fresh(&path, content, kind, regen),
        Some(path) => write(&path, content, kind),
        None if check => Err(format!(
            "--check requires an output destination (no default configured for {kind}; use --output <path>)"
        )),
        None => {
            print!("{content}");
            Ok(())
        }
    }
}

/// Compare `content` to the bytes already at `path` without touching the file.
fn check_fresh(path: &Path, content: &str, kind: &str, regen: &str) -> Result<(), String> {
    match std::fs::read(path) {
        Ok(existing) => {
            if existing.as_slice() == content.as_bytes() {
                Ok(())
            } else {
                Err(format!(
                    "out of date {kind}: {} differs from generated output (regenerate with: archspec {regen})",
                    path.display()
                ))
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Err(format!(
            "out of date {kind}: {} is missing (generate with: archspec {regen})",
            path.display()
        )),
        Err(err) => Err(format!(
            "failed to read {kind} {}: {err}",
            path.display()
        )),
    }
}
