use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::language;
use crate::archspec::scan;
use crate::archspec::spec::SPEC_FILE;
use crate::archspec::update;
use std::path::Path;

pub const HELP: &str = "usage: archspec update [path] [--force]\nsnapshot the current model as a seed architecture.spec.toml\n\n  path      project directory to scan and snapshot (default: current directory)\n  --force   overwrite an existing architecture.spec.toml\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[FlagSpec {
            name: "force",
            takes_value: false,
        }],
    )?;
    cli::exactly_one_positional(&parsed)?;

    let force = parsed.switches.contains("force");
    let root = parsed
        .positionals
        .first()
        .map(String::as_str)
        .unwrap_or(".");
    let root_path = Path::new(root);

    if !root_path.exists() {
        return Err(format!("path does not exist: {root}"));
    }
    if !root_path.is_dir() {
        return Err(format!("path is not a directory: {root}"));
    }

    let spec_path = root_path.join(SPEC_FILE);
    if spec_path.exists() && !force {
        return Err(format!(
            "spec already exists: {} (use --force to overwrite)",
            spec_path.display()
        ));
    }

    let detected = language::detect(root_path)
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    if !scan::driver_available(detected) {
        return Err(format!(
            "{} toolchain not found (run 'archspec doctor' to diagnose drivers)",
            detected.as_str()
        ));
    }

    let model = scan::extract(detected, root_path)?;

    let content = update::seed_spec(&model);
    std::fs::write(&spec_path, content)
        .map_err(|err| format!("failed to write spec: {}: {err}", spec_path.display()))?;

    println!("generated {}", spec_path.display());
    println!("review and trim the seed spec, then run archspec verify");
    Ok(())
}
