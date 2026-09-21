pub mod detect;

use crate::archspec::cli;
use std::fs;
use std::path::Path;

pub const HELP: &str = "usage: archspec init [path]\nscaffold a minimal architecture.spec.toml in the project directory\n\n  path    project directory to scaffold into (default: current directory)\n";

const OUTPUT_CONFIG_CONTENT: &str = "[output]\ninspect = \"docs/archspec/inspect.mmd\"\ndiagram = \"docs/archspec/diagram.mmd\"\nreport  = \"docs/archspec/report.md\"\nscan    = \"docs/archspec/scan.json\"\n";

/// Base spec body appended after `[project]`: the global cycle guard. An empty
/// (absent) `modules` list already means "all declared modules", so new
/// projects start cycle-clean without any engine-level default or schema field.
const GLOBAL_CYCLE_GUARD: &str = "[[constraint]]\ntype = \"no_cycles\"\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let args = cli::parse(args, &[])?;
    cli::exactly_one_positional(&args)?;

    let target = args.positionals.first().map(String::as_str).unwrap_or(".");
    let target_path = Path::new(target);

    if !target_path.exists() {
        return Err(format!("path does not exist: {target}"));
    }
    if !target_path.is_dir() {
        return Err(format!("path is not a directory: {target}"));
    }

    let spec_path = target_path.join("architecture.spec.toml");
    if spec_path.exists() {
        return Err(format!(
            "architecture.spec.toml already exists: {}",
            spec_path.display()
        ));
    }

    let config_path = target_path.join("archspec.toml");
    if config_path.exists() {
        return Err(format!(
            "archspec.toml already exists: {}",
            config_path.display()
        ));
    }

    let language = detect::detect(target_path).ok_or_else(|| {
        format!(
            "could not detect project language: {target} (expected Cargo.toml, go.mod, or *.csproj/*.sln)"
        )
    })?;

    let spec = format!(
        "[project]\nlanguage = \"{}\"\n\n{}",
        language.as_str(),
        GLOBAL_CYCLE_GUARD
    );

    fs::write(&spec_path, spec.as_bytes())
        .map_err(|e| format!("cannot write spec: {} ({e})", spec_path.display()))?;

    fs::write(&config_path, OUTPUT_CONFIG_CONTENT.as_bytes())
        .map_err(|e| format!("cannot write config: {} ({e})", config_path.display()))?;

    println!("created architecture.spec.toml");
    println!("created archspec.toml");
    println!("language: {}", language.as_str());
    println!("next step: run archspec update to seed boundaries from the tree, then edit, then verify");

    Ok(())
}
