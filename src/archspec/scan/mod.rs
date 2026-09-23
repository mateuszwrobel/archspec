use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::language::{self, Language};
use crate::archspec::model::Model;
use std::path::Path;

pub(crate) mod csharp;
pub(crate) mod go;
mod rust;

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[
            FlagSpec {
                name: "output",
                takes_value: true,
            },
            FlagSpec {
                name: "check",
                takes_value: false,
            },
            FlagSpec {
                name: "pretty",
                takes_value: false,
            },
        ],
    )?;
    cli::exactly_one_positional(&parsed)?;

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

    let language = language::detect(root_path)
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    if !driver_available(language) {
        return Err(driver_unavailable(language));
    }

    let model = extract(language, root_path)?;
    // `--pretty` is a serialization switch only: same facts, indented layout
    // for readers and diffs; the default bytes are untouched (blind4 D03).
    let json = if parsed.switches.contains("pretty") {
        format!("{}\n", model.to_json_pretty()?)
    } else {
        format!("{}\n", model.to_json()?)
    };

    let config = config::load(root_path)?;
    // The model JSON is a machine body: a body on stdout carries no status
    // line (wave-3 D02 purity); a destination write echoes the universal
    // `wrote <path>` line like every path write (blind4 D01).
    let route = config::route(
        parsed.values.get("output").map(String::as_str),
        config.scan.as_deref(),
        root_path,
        false,
    );
    let check = parsed.switches.contains("check");
    config::emit(&json, route, check, "model", "scan", None)
}

pub fn extract(language: Language, root: &Path) -> Result<Model, String> {
    match language {
        Language::Rust => rust::extract(root),
        Language::Csharp => csharp::extract(root),
        Language::Go => go::extract(root),
    }
}

/// Extract with the driver-availability gate applied. Every consumer that
/// extracts through a language driver must go through this so the
/// env-controlled test invariant cannot be silently bypassed.
pub fn extract_checked(language: Language, root: &Path) -> Result<Model, String> {
    if !driver_available(language) {
        return Err(driver_unavailable(language));
    }
    extract(language, root)
}

/// Phase-1 ships all drivers in the binary, so they are available unless a
/// test controls the environment (acceptance #11) by disabling one via
/// `ARCHSPEC_DISABLE_DRIVERS` (comma-separated, e.g. "go").
pub fn driver_available(language: Language) -> bool {
    std::env::var("ARCHSPEC_DISABLE_DRIVERS")
        .map(|value| {
            !value
                .split(',')
                .any(|part| part.trim() == language.as_str())
        })
        .unwrap_or(true)
}

fn driver_unavailable(language: Language) -> String {
    format!(
        "detected language '{}' but the {} driver is not available (run 'archspec doctor')",
        language.as_str(),
        language.as_str()
    )
}
