use crate::archspec::cli;
use crate::archspec::language::Language;
use crate::archspec::scan;

/// Test seam: when set, `doctor` reports that it cannot inspect its own
/// environment instead of probing. Simulates the acceptance #9 case, which is
/// otherwise hard to trigger deterministically (probe failures below degrade to
/// "absent" findings). Documented in docs/archspec/commands/doctor/acceptance.md.
const INSPECT_FAIL_ENV: &str = "ARCHSPEC_DOCTOR_FAIL";

pub const HELP: &str = "usage: archspec doctor\nreport driver capability and toolchain availability (rust, csharp, go) as separate facts\n\n  takes no arguments or flags\n";

const LANGUAGES: [Language; 3] = [Language::Rust, Language::Csharp, Language::Go];

/// `archspec doctor` (commands/doctor/cli.md). Zero args, zero flags. Per
/// supported language it reports two INDEPENDENT facts: driver capability
/// (the in-binary, parse-only driver works without any toolchain — see the
/// capability table) and toolchain availability on PATH (a diagnostic detail,
/// never a scanning gate). The summary lists the languages whose DRIVER is
/// present. A report is success even when every toolchain is absent; only
/// invalid invocations and an uninspectable environment exit non-zero
/// (commands/doctor/errors.md).
pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(args, &[])?;
    if !parsed.positionals.is_empty() {
        return Err(
            "doctor takes no path argument (diagnoses the environment, not a project)".to_string(),
        );
    }
    if std::env::var_os(INSPECT_FAIL_ENV).is_some() {
        return Err(
            "failed to inspect environment: cannot probe toolchain availability".to_string(),
        );
    }

    let mut scannable = Vec::new();
    let mut blocks = Vec::new();
    for language in LANGUAGES {
        let mut lines = vec![language.as_str().to_string()];
        if scan::driver_available(language) {
            scannable.push(language.as_str());
            lines.push("  driver: present (in-binary, parse-only)".to_string());
        } else {
            lines.push("  driver: absent".to_string());
            lines.push(format!(
                "  guidance: the {} driver is not available in this environment; restore it, \
                 then re-run archspec doctor",
                display_name(language)
            ));
        }
        match probe(language) {
            Some(version) => lines.push(format!("  toolchain: present ({version})")),
            None => {
                lines.push("  toolchain: absent".to_string());
                lines.push(format!(
                    "  note: scanning {language} does not require the toolchain; the driver \
                     parses sources in-binary",
                    language = language.as_str()
                ));
            }
        }
        blocks.push(lines.join("\n"));
    }

    let summary = if scannable.is_empty() {
        "none".to_string()
    } else {
        scannable.join(", ")
    };
    let report = format!(
        "{}\n\nScannable languages: {summary}\n",
        blocks.join("\n\n")
    );
    print!("{report}");
    Ok(())
}

/// Probe one TOOLCHAIN on PATH (toolchain availability only — driver
/// capability is a separate fact from the capability table, never a probe).
/// Binary per language: rust probes `rustc --version`
/// (the compiler itself; its output already carries the rustc/cargo detail),
/// csharp probes `dotnet --version` (the .NET SDK driver), go probes
/// `go version`. Present only if the binary runs, exits 0, and prints a
/// non-empty version; anything else (not found, non-zero, empty output, a
/// spawn error that outlives the retry in `probe_spawn`) is absent — a
/// finding, not an inspection failure.
fn probe(language: Language) -> Option<String> {
    let (binary, args): (&str, &[&str]) = match language {
        Language::Rust => ("rustc", &["--version"]),
        Language::Csharp => ("dotnet", &["--version"]),
        Language::Go => ("go", &["version"]),
    };
    let output = probe_spawn(binary, args).ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        None
    } else {
        Some(stdout)
    }
}

/// A PATH probe that fails to spawn for a reason other than "binary not on
/// PATH" (`ETXTBSY` right after a harness wrote the stub, momentary fork
/// pressure) is transient: retry a few times before degrading to "absent", so
/// the finding reflects the PATH rather than a one-off spawn error. `NotFound`
/// is the genuine absence signal and short-circuits without retry.
const SPAWN_RETRIES: u32 = 3;
const SPAWN_RETRY_DELAY: std::time::Duration = std::time::Duration::from_millis(20);

fn probe_spawn(binary: &str, args: &[&str]) -> Result<std::process::Output, std::io::Error> {
    let mut attempts = 0;
    loop {
        match std::process::Command::new(binary).args(args).output() {
            Ok(output) => return Ok(output),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Err(err),
            Err(_transient) if attempts < SPAWN_RETRIES => {
                attempts += 1;
                std::thread::sleep(SPAWN_RETRY_DELAY);
            }
            Err(err) => return Err(err),
        }
    }
}

fn display_name(language: Language) -> &'static str {
    match language {
        Language::Rust => "Rust",
        Language::Csharp => "Csharp",
        Language::Go => "Go",
    }
}
