//! `archspec skill` — the agent-facing skills as a deliverable.
//!
//! The skill documents live in the payload docs (`docs/archspec/skill.md` for
//! the audit skill, `docs/archspec/skill-analysis.md` for the analysis skill)
//! and are embedded at compile time, so a single source of truth serves both
//! the printed and installed forms; tests pin the emitted bytes against the
//! documents to stop drift. Each skill is addressable by name on stdout
//! (`audit` is the default; `analysis` emits the relation-analysis
//! playbook). `install` writes the audit skill to the loading convention of
//! this repository family (`.agent/skills/archspec.md` under the project
//! path) and treats a locally modified file as user work: it refuses without
//! `--force`, mirroring the care `update` takes with spec files.

use std::io::ErrorKind;
use std::path::Path;

use crate::archspec::cli;

pub const SKILL: &str = include_str!("../../../docs/archspec/skill.md");

/// The relation-analysis playbook as its own skill, emitted by name and not
/// silently merged into the audit skill (the analysis procedure carries no
/// registered claim signatures; verdicts stay in exit codes).
pub const SKILL_ANALYSIS: &str = include_str!("../../../docs/archspec/skill-analysis.md");

/// Where the skill lands under a project root, in the agent harness's loading
/// convention.
pub const SKILL_DEST: &str = ".agent/skills/archspec.md";

pub const HELP: &str = "usage: archspec skill [name]\n       archspec skill install [path] [--force]\n\nname     emit one shipped skill on stdout by name (default: audit)\n  audit    the agent-facing audit skill\n  analysis the relation-analysis playbook (recommendations, never verdicts)\nprint    the agent-facing audit skill on stdout (same as `skill audit`)\ninstall  write the skill to <path>/.agent/skills/archspec.md (default path: the\n         current directory), creating missing parent directories\n  --force  overwrite an installed skill whose content differs from the shipped\n           one; an unchanged file is never rewritten\n\nthe embedded skills match the shipped documents byte-for-byte; reinstall with\n--force after upgrading archspec\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[cli::FlagSpec {
            name: "force",
            takes_value: false,
        }],
    )?;
    let mut positionals = parsed.positionals.iter().map(String::as_str);
    match positionals.next() {
        None | Some("print") => {
            if positionals.next().is_some() {
                return Err("skill print takes no path arguments".to_string());
            }
            print!("{SKILL}");
            Ok(())
        }
        Some("install") => {
            let rest: Vec<&str> = positionals.collect();
            let root = match rest.as_slice() {
                [] => Path::new("."),
                [path] => Path::new(*path),
                _ => return Err("expected at most one path argument".to_string()),
            };
            install(root, parsed.switches.contains("force"))
        }
        Some(name @ ("audit" | "analysis")) => {
            if positionals.next().is_some() {
                return Err(format!("skill {name} takes no path arguments"));
            }
            print!("{}", skill_document(name)?);
            Ok(())
        }
        Some(other) => Err(format!(
            "unknown skill: {other} (expected: print, install, audit, analysis)"
        )),
    }
}

/// The embedded bytes of one named skill — the closed name set is
/// {audit, analysis}; anything else fails loud, never falling back to the
/// audit skill.
fn skill_document(name: &str) -> Result<&'static str, String> {
    match name {
        "audit" => Ok(SKILL),
        "analysis" => Ok(SKILL_ANALYSIS),
        other => Err(format!(
            "unknown skill: {other} (expected: print, install, audit, analysis)"
        )),
    }
}

fn install(root: &Path, force: bool) -> Result<(), String> {
    let dest = root.join(SKILL_DEST);
    let parent = dest
        .parent()
        .ok_or_else(|| format!("invalid skill destination: {}", dest.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
    match std::fs::read(&dest) {
        Ok(existing) if existing == SKILL.as_bytes() => {
            println!("already installed: {}", dest.display());
            Ok(())
        }
        Ok(_) if !force => Err(format!(
            "{} differs from the shipped skill; pass --force to overwrite",
            dest.display()
        )),
        Ok(_) => {
            std::fs::write(&dest, SKILL)
                .map_err(|e| format!("cannot write {}: {e}", dest.display()))?;
            println!("overwrote {}", dest.display());
            Ok(())
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {
            std::fs::write(&dest, SKILL)
                .map_err(|e| format!("cannot write {}: {e}", dest.display()))?;
            println!("installed {}", dest.display());
            Ok(())
        }
        Err(e) => Err(format!("cannot read {}: {e}", dest.display())),
    }
}
