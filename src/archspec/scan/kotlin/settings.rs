//! The settings-file module list (kotlin driver record K-P2): the settings
//! file IS the module list. Entries are the `include(":x")` STRING-LITERAL
//! arguments; identity is the GRADLE PATH (`:app` → unit `app`, `:a:b` →
//! unit `a::b`), and a `projectDir` remap moves LOCATION, never IDENTITY.
//!
//! Failure classes, exact strings (the Emitted Strings table):
//! - both settings files at one root → the fail-closed error
//!   `ambiguous settings file: settings.gradle + settings.gradle.kts — keep one`
//!   (detection has already answered kotlin — the module-list parse refuses,
//!   one owner decides: keep one file);
//! - no settings file → one root unit + the report-only line
//!   `no settings file: gradle module list unavailable — one unit at the scan root`;
//! - an include argument that is not a statically visible literal → the
//!   report-only line `unresolved settings include: <file>:<line> — module
//!   list may be incomplete` (the shipped vacuous-constraint warning is the
//!   named backstop: an absent unit can never green a claim silently).
//!
//! Parsing is static text reading (K-P2: archspec executes nothing — no
//! gradle daemon, no script interpretation): each physical line carries its
//! own 1-based number for the unresolved line; multi-line `include(` calls
//! continue until the parentheses balance, named at the opening line.

use std::path::Path;

/// One declared gradle module: identity (`name`) and location (`dir`,
/// root-relative). A remap changes `dir`, never `name`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UnitSpec {
    /// Gradle path minus the leading colon; nested paths join with `::`
    /// (the dotted module-tier spelling every `depgraph` view projects).
    pub(crate) name: String,
    /// Root-relative directory of the module (`app`, `shared/ui`, or the
    /// remapped location).
    pub(crate) dir: String,
}

/// The module-list parse outcome.
#[derive(Clone, Debug, Default)]
pub(crate) struct ModulePlan {
    /// The settings-file units in declaration order, deduplicated by name.
    pub(crate) units: Vec<UnitSpec>,
    /// True when no settings file exists (one root unit + its line).
    pub(crate) no_settings: bool,
    /// Report-only lines the parse produced (unresolved includes).
    pub(crate) lines: Vec<String>,
    /// Literal `layout.buildDirectory.set(file("…"))` remaps found in the
    /// settings file, applied to EVERY module (the K-P7 statically-visible
    /// relocation: the stated path is excluded instead of `build`).
    pub(crate) settings_build_dir: Option<String>,
}

const SETTINGS_FILES: [&str; 2] = ["settings.gradle", "settings.gradle.kts"];

/// Parse the module list at `root`; the dual-settings class is the only
/// error this leg raises.
pub(crate) fn plan(root: &Path) -> Result<ModulePlan, String> {
    let mut present: Vec<&str> = Vec::new();
    for name in SETTINGS_FILES {
        if root.join(name).is_file() {
            present.push(name);
        }
    }
    match present.len() {
        0 => Ok(ModulePlan {
            no_settings: true,
            ..ModulePlan::default()
        }),
        1 => {
            let name = present[0];
            let raw = std::fs::read_to_string(root.join(name))
                .map_err(|err| format!("failed to read {}: {err}", name))?;
            parse(&raw, name)
        }
        _ => Err(
            "ambiguous settings file: settings.gradle + settings.gradle.kts — keep one".to_string(),
        ),
    }
}

fn parse(raw: &str, file: &str) -> Result<ModulePlan, String> {
    let mut plan = ModulePlan::default();
    let lines: Vec<&str> = raw.lines().collect();
    let mut index = 0usize;
    while index < lines.len() {
        let line = strip_comment(lines[index]);
        if let Some(open) = line.find("include(") {
            let opening = index + 1;
            let arguments = gather_arguments(&lines, index, open + "include(".len());
            let mut all_literal = true;
            for token in split_arguments(&arguments) {
                let token = token.trim();
                if token.is_empty() {
                    continue;
                }
                match parse_include_argument(token) {
                    Some(path) => add_unit(&mut plan.units, path),
                    None => all_literal = false,
                }
            }
            if !all_literal {
                plan.lines.push(format!(
                    "unresolved settings include: {file}:{opening} — module list may be incomplete"
                ));
            }
            while index < lines.len() && paren_balance(&strip_comment(lines[index])) != 0 {
                index += 1;
            }
        } else if let Some(path) = project_dir_remap(&line) {
            if let Some(unit) = plan
                .units
                .iter_mut()
                .find(|unit| unit.name == path.0)
            {
                unit.dir = path.1;
            } else {
                // A remap before (or without) its include still declares
                // the unit: gradle applies remaps to the project list.
                let mut spec = unit_from_path(&path.0);
                spec.dir = path.1;
                plan.units.push(spec);
            }
        }
        if plan.settings_build_dir.is_none() {
            plan.settings_build_dir = build_directory_literal(&line);
        }
        index += 1;
    }
    plan.lines.sort();
    plan.lines.dedup();
    Ok(plan)
}

fn strip_comment(line: &str) -> String {
    match line.find("//") {
        Some(cut) => line[..cut].to_string(),
        None => line.to_string(),
    }
}

fn paren_balance(line: &str) -> i32 {
    line.bytes()
        .fold(0i32, |depth, byte| match byte {
            b'(' => depth + 1,
            b')' => depth - 1,
            _ => depth,
        })
}

/// The text between the `include(` open paren and its matching close,
/// joining continuation lines until the parentheses balance.
fn gather_arguments(lines: &[&str], start: usize, from_column: usize) -> String {
    let mut text = String::new();
    let mut depth = 0i32;
    let mut index = start;
    let mut offset = from_column;
    while index < lines.len() {
        let line = strip_comment(lines[index]);
        let tail: String = line.chars().skip(offset).collect();
        offset = 0;
        for ch in tail.chars() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    if depth == 0 {
                        return text;
                    }
                    depth -= 1;
                    if depth == 0 {
                        return text;
                    }
                    text.push(ch);
                }
                _ => text.push(ch),
            }
        }
        text.push(' ');
        index += 1;
    }
    text
}

/// Split one argument list on top-level commas (string literals respected).
fn split_arguments(text: &str) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut quote: Option<char> = None;
    for ch in text.chars() {
        match quote {
            Some(mark) if ch == mark => {
                quote = None;
                parts.last_mut().expect("one part").push(ch);
            }
            Some(_) => parts.last_mut().expect("one part").push(ch),
            None if ch == '"' || ch == '\'' => {
                quote = Some(ch);
                parts.last_mut().expect("one part").push(ch);
            }
            None if ch == ',' => parts.push(String::new()),
            None => parts.last_mut().expect("one part").push(ch),
        }
    }
    parts
}

/// The literal include argument shape: `":a"` or `':a'` (the colon-led
/// gradle path); anything else is not statically visible.
fn parse_include_argument(token: &str) -> Option<String> {
    let inner = token.trim();
    let quoted = inner
        .strip_prefix('"')
        .and_then(|tail| tail.strip_suffix('"'))
        .or_else(|| inner.strip_prefix('\'').and_then(|tail| tail.strip_suffix('\'')))?;
    quoted.strip_prefix(':').map(|path| path.to_string())
}

fn unit_from_path(path: &str) -> UnitSpec {
    UnitSpec {
        name: path.split(':').collect::<Vec<_>>().join("::"),
        dir: path.split(':').collect::<Vec<_>>().join("/"),
    }
}

fn add_unit(units: &mut Vec<UnitSpec>, path: String) {
    let spec = unit_from_path(&path);
    if !units.iter().any(|unit| unit.name == spec.name) {
        units.push(spec);
    }
}

/// The `project(":x").projectDir = file("elsewhere")` line: identity plus
/// the new location (K-P2 — location moves, identity never).
fn project_dir_remap(line: &str) -> Option<(String, String)> {
    let open = line.find("project(")?;
    let args = line[open + "project(".len()..].split(')').next()?;
    let identity = parse_include_argument(args)?;
    let file_open = line.find("file(")?;
    let file_args = line[file_open + "file(".len()..].split(')').next()?;
    let inner = file_args.trim();
    let path = inner
        .strip_prefix('"')
        .and_then(|tail| tail.strip_suffix('"'))
        .or_else(|| inner.strip_prefix('\'').and_then(|tail| tail.strip_suffix('\'')))?;
    let dir = path
        .trim_end_matches('/')
        .trim_start_matches("./")
        .to_string();
    Some((unit_from_path(&identity).name, dir))
}

/// The statically-visible relocation literal (K-P7): a literal
/// `layout.buildDirectory.set(file("out"))` states the path that replaces
/// the `build` exclusion.
fn build_directory_literal(line: &str) -> Option<String> {
    if !line.contains("layout.buildDirectory.set(") {
        return None;
    }
    let open = line.find("file(")?;
    let args = line[open + "file(".len()..].split(')').next()?;
    let inner = args.trim();
    let path = inner
        .strip_prefix('"')
        .and_then(|tail| tail.strip_suffix('"'))
        .or_else(|| inner.strip_prefix('\'').and_then(|tail| tail.strip_suffix('\'')))?;
    Some(path.trim_end_matches('/').to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse, unit_from_path, ModulePlan};

    #[test]
    fn literal_includes_become_gradle_path_units() {
        let plan = parse(
            "rootProject.name = \"pilot\"\ninclude(\":app\", \":core\")\ninclude(\":ime\")\ninclude(\":architecture-tests\")\n",
            "settings.gradle.kts",
        )
        .expect("parses");
        let names: Vec<&str> = plan.units.iter().map(|unit| unit.name.as_str()).collect();
        assert_eq!(
            names,
            ["app", "core", "ime", "architecture-tests"],
            "names minus the leading colon, in order"
        );
        assert!(plan.lines.is_empty());
        assert!(!plan.no_settings);
    }

    #[test]
    fn a_non_literal_include_names_the_site_and_keeps_the_literals() {
        let plan = parse("include(provided)\n", "settings.gradle.kts").expect("parses");
        assert!(plan.units.is_empty());
        assert_eq!(
            plan.lines,
            ["unresolved settings include: settings.gradle.kts:1 — module list may be incomplete"]
        );
    }

    #[test]
    fn a_projectdir_remap_moves_location_never_identity() {
        let plan = parse(
            "include(\":x\")\nproject(\":x\").projectDir = file(\"vendor/elsewhere\")\n",
            "settings.gradle.kts",
        )
        .expect("parses");
        assert_eq!(plan.units.len(), 1);
        assert_eq!(plan.units[0].name, "x");
        assert_eq!(plan.units[0].dir, "vendor/elsewhere");
    }

    #[test]
    fn nested_paths_join_dotted_and_slashed() {
        assert_eq!(unit_from_path("a:b").name, "a::b");
        assert_eq!(unit_from_path("a:b").dir, "a/b");
    }

    #[test]
    fn a_literal_build_directory_override_is_stated() {
        let plan = parse(
            "subprojects { layout.buildDirectory.set(file(\"out\")) }\n",
            "settings.gradle.kts",
        )
        .expect("parses");
        assert_eq!(plan.settings_build_dir.as_deref(), Some("out"));
        let plain = ModulePlan::default();
        assert_eq!(plain.settings_build_dir, None);
    }
}
