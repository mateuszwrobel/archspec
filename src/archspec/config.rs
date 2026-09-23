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

/// Where one render command's body goes, and what stdout says about it. Decided
/// once, at the point `--output` and the `[output]` table are resolved, so the
/// commands keep exactly one routing decision
/// (`workplan_archspec_fanout_and_output` D01/D02/D03/D08).
pub struct Route {
    /// `Some(path)` receives the body; `None` prints it to stdout. A command
    /// whose format skips the configured destination (`report --format`) sets
    /// this to `None`, and stdout then names the skipped destination; `--output
    /// -` sets it to `None` too, and names no destination at all.
    pub destination: Option<PathBuf>,
    /// The destination the project configures for this artefact, kept so a run
    /// that leaves it untouched can say so.
    configured: Option<PathBuf>,
    /// Whether stdout may carry a status line at all (D08).
    human: bool,
}

/// D08's split, in one place: `text`, `markdown` and `mermaid` surfaces are
/// human, so stdout may carry the one status line about the destination; `json`
/// and `plantuml` are machine, so their bytes never move on any path.
pub fn is_human(format: &str) -> bool {
    matches!(format, "text" | "markdown" | "mermaid")
}

/// Pick the route: `--output` wins (kept cwd-relative and unchanged), `-` means
/// stdout with no destination named at all (D03), else the configured default
/// resolved against the project root, else stdout.
pub fn route(flag: Option<&str>, configured: Option<&str>, root: &Path, human: bool) -> Route {
    let configured = configured.map(|path| root.join(path));
    if flag == Some("-") {
        return Route {
            destination: None,
            configured: None,
            human,
        };
    }
    let destination = match flag {
        Some(path) => Some(PathBuf::from(path)),
        None => configured.clone(),
    };
    Route {
        destination,
        configured,
        human,
    }
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

/// Terminal step of every output-producing command: write `content` to the
/// route's destination, print it to stdout when the route has none, or — under
/// `--check` — compare it byte-for-byte against the destination without writing.
/// Fresh prints `ok: <path> up to date` and exits `0`; stale/missing/
/// absent-destination exits non-zero with an actionable finding (the `regen`
/// command shows how to regenerate) and prints no ok-line (blind4 D02). Every
/// destination write — any command, any format, configured or `--output` —
/// echoes `wrote <path>` (universal echo, blind4 D01); a human body printed to
/// stdout around an untouched destination opens with `note: destination <path>
/// not written` (D01, D08).
///
/// `stale_finding` replaces the parenthetical of a stale `--check` finding.
/// Commands whose artefact has one rendering pass `None` and keep the bare
/// `regenerate with: archspec <regen>` hint byte-unchanged. A command that
/// renders one artefact in several renderings (`report`) passes the finding
/// naming the rendering it compared and carrying that rendering's exact
/// command form, so following the hint regenerates the compared body
/// byte-exactly (blind5 D01).
pub fn emit(
    content: &str,
    route: Route,
    check: bool,
    kind: &str,
    regen: &str,
    stale_finding: Option<&str>,
) -> Result<(), String> {
    let Route {
        destination,
        configured,
        human,
    } = route;
    match destination {
        Some(path) if check => {
            check_fresh(&path, content, kind, regen, stale_finding)?;
            // A green check speaks (blind4 D02): the verdict is a line, not
            // only an exit code — and only on green, never on a red check.
            println!("ok: {} up to date", spell(&path));
            Ok(())
        }
        Some(path) => {
            write(&path, content, kind)?;
            if let Some(line) = status(&configured, Some(&path), human) {
                println!("{line}");
            }
            Ok(())
        }
        None if check => Err(format!(
            "--check requires an output destination (no default configured for {kind}; use --output <path>)"
        )),
        None => {
            if let Some(line) = status(&configured, None, human) {
                println!("{line}");
            }
            print!("{content}");
            Ok(())
        }
    }
}

/// The one statement stdout makes about a destination, if any: `wrote <path>`
/// whenever a body lands in a path — an `[output]` destination or an explicit
/// `--output`, human or machine format (the universal echo, blind4 D01) — and
/// `note: destination <path> not written` when a human body went to stdout
/// while a configured destination stayed untouched. Nothing when no destination
/// is configured or the body was printed with no destination in play at all
/// (`--output -`); a machine body on stdout stays byte-clean (D02, D08). Never
/// both — a run either writes a destination or skips it.
fn status(configured: &Option<PathBuf>, written: Option<&Path>, human: bool) -> Option<String> {
    if let Some(path) = written {
        return Some(format!("wrote {}", spell(path)));
    }
    if human {
        if let Some(path) = configured.as_deref() {
            return Some(format!("note: destination {} not written", spell(path)));
        }
    }
    None
}

/// Spell a destination for stdout: the resolved path without the `./` prefix
/// `Path::join` adds for a current-directory root, so the line spells the path
/// as configured (D01).
fn spell(path: &Path) -> String {
    let text = path.display().to_string();
    text.strip_prefix("./").unwrap_or(&text).to_owned()
}

/// Compare `content` to the bytes already at `path` without touching the file.
/// A stale finding states the regeneration hint as `stale_finding` when the
/// caller supplies one — the report family names the rendering it compared
/// and hints that rendering's exact form (blind5 D01) — and otherwise keeps
/// the bare `regenerate with: archspec <regen>` hint byte-unchanged. The
/// missing finding always states the bare generation hint: nothing was
/// compared there.
fn check_fresh(
    path: &Path,
    content: &str,
    kind: &str,
    regen: &str,
    stale_finding: Option<&str>,
) -> Result<(), String> {
    match std::fs::read(path) {
        Ok(existing) => {
            if existing.as_slice() == content.as_bytes() {
                Ok(())
            } else {
                let bare = format!("regenerate with: archspec {regen}");
                let compared = stale_finding.unwrap_or(&bare);
                Err(format!(
                    "out of date {kind}: {} differs from generated output ({compared})",
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
