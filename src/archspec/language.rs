use std::path::{Component, Path};

use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Language {
    Rust,
    Csharp,
    Go,
    Ts,
    Python,
    Kotlin,
}

impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Csharp => "csharp",
            Language::Go => "go",
            Language::Ts => "ts",
            Language::Python => "python",
            Language::Kotlin => "kotlin",
        }
    }
}

/// Directory names the DETECTION walk skips before any marker matches: a
/// dependency or build tree can never select a driver (the ts driver design
/// law — the ts arm is the fallback, so dependency-tree markers such as a
/// `package.json` inside `node_modules` must not reach the chain). The scan
/// walks apply their own per-driver exclusion sets on top of this.
/// One mechanical exclusion directory with its class (round-6 laws — the
/// ONE list every exclusion surface reads, one owner).
pub struct ExcludeDir {
    pub name: &'static str,
    /// The GENERATED-OUTPUT class (round-6 P-GEN-TSCONFIG i): an absent
    /// extends-target under one of these segments is a not-yet-built
    /// artifact riding the `generated tsconfig absent` report-only line,
    /// not broken authoring (the `.svelte-kit` precedent). The class is a
    /// MARKER ON THIS LIST — never a second whitelist.
    pub generated: bool,
    /// The VENDORED class (round-6 P-VENDOR, R11): the one NARROWED
    /// mechanical class. A vendor segment under the scan root excludes
    /// NOTHING — source-extension glue stays in scope and asset leaf
    /// edges land again (the measured `.wasm?url` red was pure vendor
    /// exclusion). Every other class stays ABSOLUTE.
    pub vendored: bool,
}

pub const DETECT_EXCLUDE_DIRS: &[ExcludeDir] = &[
    ExcludeDir { name: "node_modules", generated: false, vendored: false },
    ExcludeDir { name: "dist", generated: true, vendored: false },
    ExcludeDir { name: "target", generated: true, vendored: false },
    ExcludeDir { name: ".git", generated: false, vendored: false },
    ExcludeDir { name: "vendor", generated: false, vendored: true },
    ExcludeDir { name: "bin", generated: true, vendored: false },
    ExcludeDir { name: "obj", generated: true, vendored: false },
    // The framework-GENERATED class (round-8 four-list law L-B): these
    // build-output directories stay excluded from source scanning AND
    // detection REGARDLESS of any `[project] framework` key — excluding a
    // build-output class is a mechanical fact (the `dist` shape), never
    // auto-discovery (round-8 F12, rebutted). Only the resolution
    // SEMANTICS that name frameworks are key-gated (L-C).
    ExcludeDir { name: ".svelte-kit", generated: true, vendored: false },
    ExcludeDir { name: ".next", generated: true, vendored: false },
    ExcludeDir { name: ".nuxt", generated: true, vendored: false },
    ExcludeDir { name: ".astro", generated: true, vendored: false },
    // The python arm's no-venv law (python driver record P3): an install
    // tree's .py files are dependency artifacts, never first-party sources,
    // and a venv below the scan root can never select the python driver.
    ExcludeDir { name: ".venv", generated: false, vendored: false },
    ExcludeDir { name: "__pycache__", generated: true, vendored: false },
];

/// The one mechanical exclusion directory named by a path (VENDORED
/// names never hit — the narrowed class excludes nothing, R11).
pub(crate) fn is_excluded_dir(name: &str) -> bool {
    DETECT_EXCLUDE_DIRS
        .iter()
        .any(|dir| dir.name == name && !dir.vendored)
}

/// True when the component name is a GENERATED-OUTPUT directory (the
/// tsconfig-absence whitelist, round-6 P-GEN-TSCONFIG i).
pub(crate) fn generated_dir(name: &str) -> bool {
    DETECT_EXCLUDE_DIRS.iter().any(|dir| dir.name == name && dir.generated)
}

/// True when any path component names an excluded directory.
pub(crate) fn excluded_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(component, Component::Normal(name)
            if DETECT_EXCLUDE_DIRS.iter().any(|dir| dir.name == &*name.to_string_lossy()))
    })
}

/// The ONE driver-name table (round-8 F11): `from_name` parses it and the
/// `archspec spec --schema` surface enumerates it — no verb keeps a private
/// copy of the list. Order is the schema-enum spelling.
pub const LANGUAGES: &[(&str, Language)] = &[
    ("rust", Language::Rust),
    ("csharp", Language::Csharp),
    ("go", Language::Go),
    ("ts", Language::Ts),
    ("python", Language::Python),
    ("kotlin", Language::Kotlin),
];

/// Parse a spec/model language name into a driver language. The mirror of
/// `as_str`; unknown names yield `None` (consumers then fall back to the
/// granular assumption rather than guessing a driver).
pub fn from_name(value: &str) -> Option<Language> {
    LANGUAGES
        .iter()
        .find(|(name, _)| *name == value)
        .map(|(_, language)| *language)
}

/// The framework registry (round-8 P-FW1, the operator ruling: framework
/// handling belongs in DRIVERS, activates by the `[project] framework`
/// config key like `language`, and is NOT auto-discovered). Each driver
/// names the framework semantics it models; the ts driver registers
/// `sveltekit` today, every other driver registers nothing.
pub fn frameworks(language: Language) -> &'static [&'static str] {
    match language {
        Language::Ts => &["sveltekit"],
        Language::Rust
        | Language::Csharp
        | Language::Go
        | Language::Python
        | Language::Kotlin => &[],
    }
}

/// Every registered framework name across the drivers, in driver-table
/// order and deduped — the `spec schema` surface input. The schema names
/// what is registered SOMEWHERE; the per-driver split is the validator's
/// law (one owner, two readers, no second registry).
pub fn frameworks_all() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();
    for (_, language) in LANGUAGES {
        for name in frameworks(*language) {
            if !names.contains(name) {
                names.push(name);
            }
        }
    }
    names
}

/// The ONE `[project] framework` validator, read by every spec-loading
/// verb (round-8 F11). Absent or registered → `None`; an unknown name →
/// the fail-closed red naming the driver's registered alternatives
/// (`none` when the driver registers nothing) — never a warning, never an
/// ignore. Consumers wrap it in the shipped `invalid spec: <file>:` class.
pub fn framework_error(language: Language, name: &str) -> Option<String> {
    let registered = frameworks(language);
    if registered.contains(&name) {
        return None;
    }
    let alternatives = if registered.is_empty() {
        "none".to_string()
    } else {
        registered.join(", ")
    };
    Some(format!("unknown framework \"{name}\" — registered: {alternatives}"))
}

/// True if the tree contains at least one source file in the given language.
/// Used by spec-driven consumers (`report`) to reject a tree with no sources in
/// the spec's declared language before extraction (commands/report/errors.md).
/// The detection exclusion set applies: sources under dependency or build
/// trees state nothing about the tree's own language.
pub fn has_sources(root: &Path, language: Language) -> bool {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !excluded_path(entry.path()))
        .filter_map(Result::ok)
        .any(|entry| {
            if !entry.file_type().is_file() {
                return false;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            match language {
                Language::Rust => name == "Cargo.toml" || name.ends_with(".rs"),
                Language::Csharp => {
                    name.ends_with(".csproj") || name.ends_with(".sln") || name.ends_with(".cs")
                }
                Language::Go => name == "go.mod" || name.ends_with(".go"),
                Language::Ts => is_ts_source(&name) || name == "package.json",
                Language::Python => is_python_source(&name) || is_python_marker(&name),
                Language::Kotlin => is_kotlin_source(&name) || is_kotlin_marker(&name),
            }
        })
}

/// True for the ts driver's source-extension family (`.svelte` components
/// included: their `<script>` block carries the imports the driver models).
/// `.mjs`/`.cjs` and `.mts`/`.cts` join the family (detection record, the
/// mjs/cjs indexing law): the ESM/script flavor is a Node RESOLUTION
/// property, never a grammar or driver choice.
pub fn is_ts_source(name: &str) -> bool {
    name.ends_with(".ts")
        || name.ends_with(".tsx")
        || name.ends_with(".js")
        || name.ends_with(".jsx")
        || name.ends_with(".svelte")
        || name.ends_with(".mjs")
        || name.ends_with(".cjs")
        || name.ends_with(".mts")
        || name.ends_with(".cts")
}

/// True for the python driver's source-extension family. `.pyi` stub files
/// carry no import statements the driver models and are NOT sources (python
/// driver record: the `.pyi` count on the pilot is zero, and a stub's imports
/// state nothing about runtime structure).
pub fn is_python_source(name: &str) -> bool {
    name.ends_with(".py")
}

/// The python root-marker family: the marker files that name the python
/// driver at the scan root (python driver record P1 — `pyproject.toml` is
/// the declared-name set, `requirements.txt` the pinned-name list). Both
/// name the SAME driver, so their co-occurrence is NOT an ambiguity.
pub fn is_python_marker(name: &str) -> bool {
    name == "pyproject.toml" || name == "requirements.txt"
}

/// True for the kotlin driver's source-extension family. `.kt` is the ONLY
/// source predicate (kotlin driver record K-P1): `.kts` files (settings and
/// build scripts) are FACT-LEGS — parsed for the module list and dependency
/// evidence, never indexed as source vertices (gradle config code is not
/// component code).
pub fn is_kotlin_source(name: &str) -> bool {
    name.ends_with(".kt")
}

/// The kotlin root-marker family: the four gradle settings/build scripts
/// that name the kotlin driver at the scan root (kotlin driver record K-P1).
/// All four name the SAME driver, so their co-occurrence is NOT an ambiguity
/// at detection (the same-language-markers law, the python precedent); the
/// DUAL-SETTINGS failure class lives in the module-list parse, not here.
pub fn is_kotlin_marker(name: &str) -> bool {
    matches!(
        name,
        "settings.gradle.kts" | "settings.gradle" | "build.gradle.kts" | "build.gradle"
    )
}

/// The root-marker table: a file AT the scan root (depth 1, never deeper)
/// that names the driver outright (detection record GC). `pyproject.toml`
/// and `requirements.txt` are the python rows, wired in the same commit as
/// the driver (python driver record P1 — never before, so a python-only
/// root still answered the fallback chain until the arm shipped).
const ROOT_MARKERS: &[(&str, Language)] = &[
    ("Cargo.toml", Language::Rust),
    ("package.json", Language::Ts),
    ("go.mod", Language::Go),
    ("pyproject.toml", Language::Python),
    ("requirements.txt", Language::Python),
    // The kotlin marker family (kotlin driver record K-P1): four rows, ONE
    // driver — co-occurrence at one root is BENIGN at detection (the
    // same-language-markers law), never an ambiguity line.
    ("settings.gradle.kts", Language::Kotlin),
    ("settings.gradle", Language::Kotlin),
    ("build.gradle.kts", Language::Kotlin),
    ("build.gradle", Language::Kotlin),
];

/// The root markers present at the scan root, in ROOT_MARKERS table order
/// (deterministic: the same tree never lists markers differently).
pub fn root_markers(root: &Path) -> Vec<(&'static str, Language)> {
    ROOT_MARKERS
        .iter()
        .filter(|(name, _)| root.join(name).is_file())
        .map(|(name, language)| (*name, *language))
        .collect()
}

fn marker_names(markers: &[(&str, Language)]) -> String {
    markers
        .iter()
        .map(|(name, language)| format!("{name} ({})", language.as_str()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn marker_file(language: Language) -> Option<&'static str> {
    ROOT_MARKERS
        .iter()
        .find(|(_, marker)| *marker == language)
        .map(|(name, _)| *name)
}

/// Select the driver for a scan root (detection record GC — one law,
/// every verb). With a pin the pin SELECTS the DRIVER: its root marker
/// wins outright, a root carrying only foreign markers answers the
/// `language mismatch:` branch, a marker-less root keeps the unchanged
/// fallback walk (the pin then drives what the walk found). Without a pin
/// exactly one root marker selects; several answer the AMBIGUITY error;
/// none keeps the unchanged chain fallback byte-for-byte. `Ok(None)` is
/// the zero-hits answer — every caller maps it to its own no-sources
/// string. A pin whose language has no marker row (`csharp`) is adjudged
/// by the agreement check exactly as shipped.
pub fn select(root: &Path, pinned: Option<Language>) -> Result<Option<Language>, String> {
    let markers = root_markers(root);
    if let Some(pin) = pinned {
        // A pin whose language has no marker row is adjudged by the
        // agreement check exactly as shipped — no marker check.
        if marker_file(pin).is_some()
            && !markers.iter().any(|(_, language)| *language == pin)
            && !markers.is_empty()
        {
            return Err(format!(
                "language mismatch: {} — no root marker {} for the declared language, found: {}",
                pin.as_str(),
                marker_file(pin).expect("marker row checked above"),
                marker_names(&markers),
            ));
        }
        return Ok(detect(root).map(|_| pin));
    }
    match markers.len() {
        0 => Ok(detect(root)),
        // One driver answers a language's whole marker family: two markers of
        // the SAME language at one root are NOT an ambiguity (python driver
        // record P1 — `pyproject.toml` plus `requirements.txt`), so the
        // answer is that language whenever the marker set is one language.
        1 => Ok(markers.first().map(|(_, language)| *language)),
        _ if markers.iter().all(|(_, language)| *language == markers[0].1) => {
            Ok(markers.first().map(|(_, language)| *language))
        }
        _ => Err(format!(
            "ambiguous root markers: {} — pin [project] language or scan a single-language root",
            marker_names(&markers),
        )),
    }
}

/// The `nested foreign marker` report line (detection record GC): one
/// report-only stderr line per FOREIGN root-marker file below the scan
/// root (never at it — the root itself is adjudged by `select`), behind
/// the detection exclusion set, sorted by path. The line states that the
/// tree below that marker is not modelled — no edge, no finding, never a
/// verdict; the sources under it keep the existing loud law (D14 stands
/// tree-wide, the ruling).
pub fn marker_report(root: &Path, driver: Language) -> String {
    let mut lines: Vec<String> = Vec::new();
    for entry in WalkDir::new(root)
        .min_depth(2)
        .into_iter()
        .filter_entry(|entry| !excluded_path(entry.path()))
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some((_, language)) = ROOT_MARKERS.iter().find(|(marker, _)| *marker == name) else {
            continue;
        };
        if *language == driver {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        lines.push(format!(
            "nested foreign marker: {} ({}) — not modelled",
            rel.display(),
            language.as_str(),
        ));
    }
    lines.sort();
    lines.into_iter().map(|line| line + "\n").collect()
}

/// Classify a file NAME as a language source. Marker files (Cargo.toml,
/// go.mod, package.json, csproj/sln) state no counts — the agreement check
/// counts source-extension files only, so the numbers name actual sources.
fn source_language(name: &str) -> Option<Language> {
    if name.ends_with(".rs") {
        Some(Language::Rust)
    } else if name.ends_with(".cs") {
        Some(Language::Csharp)
    } else if name.ends_with(".go") {
        Some(Language::Go)
    } else if is_ts_source(name) {
        Some(Language::Ts)
    } else if is_python_source(name) {
        Some(Language::Python)
    } else if is_kotlin_source(name) {
        Some(Language::Kotlin)
    } else {
        None
    }
}

/// True when `rel` (a path relative to the walked root) sits at or under one
/// of the spec's declared `excluded_sources` roots. Path comparison is
/// component-wise: the entry `web` matches `web` and `web/app.ts`, never
/// `webapp/app.ts`.
fn excluded_by_declared(rel: &Path, excluded: &[std::path::PathBuf]) -> bool {
    excluded.iter().any(|entry| rel.starts_with(entry))
}

/// Counted per-language source walk for the agreement check (D14): counts
/// source-extension files per language across the tree, behind the mechanical
/// detection exclusion set AND the spec's declared `excluded_sources` roots.
/// Returns `(language, count)` pairs in fixed chain order (rust, csharp,
/// go, ts, python, kotlin) for every language with a non-zero count.
pub fn count_sources(root: &Path, excluded: &[std::path::PathBuf]) -> Vec<(Language, usize)> {
    let mut counts: [usize; 6] = [0, 0, 0, 0, 0, 0];
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !excluded_path(entry.path()))
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(rel) = entry.path().strip_prefix(root) else {
            continue;
        };
        if excluded_by_declared(rel, excluded) {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if let Some(language) = source_language(&name) {
            let index = match language {
                Language::Rust => 0,
                Language::Csharp => 1,
                Language::Go => 2,
                Language::Ts => 3,
                Language::Python => 4,
                Language::Kotlin => 5,
            };
            counts[index] += 1;
        }
    }
    [
        Language::Rust,
        Language::Csharp,
        Language::Go,
        Language::Ts,
        Language::Python,
        Language::Kotlin,
    ]
        .into_iter()
        .zip(counts)
        .filter(|(_, count)| *count > 0)
        .collect()
}

/// The D14 AGREEMENT law as a diagnostic: the spec-declared language is the
/// truth of a declared tree; source files of any other language — after the
/// mechanical exclusion set and the `excluded_sources` entries — mean the run
/// would analyze the wrong tree, so it fails loud BEFORE any verdict or
/// artifact. The message names the pair (declared vs found) and the counts of
/// both. `None` means agreement.
pub fn agreement_finding(
    root: &Path,
    declared: Language,
    excluded: &[std::path::PathBuf],
) -> Option<String> {
    let counts = count_sources(root, excluded);
    let declared_count = counts
        .iter()
        .find(|(language, _)| *language == declared)
        .map(|(_, count)| *count)
        .unwrap_or(0);
    let foreign: Vec<(Language, usize)> = counts
        .into_iter()
        .filter(|(language, _)| *language != declared)
        .collect();
    if foreign.is_empty() {
        return None;
    }
    let names = foreign
        .iter()
        .map(|(language, _)| language.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let found = foreign
        .iter()
        .map(|(language, count)| format!("{}: {count} files", language.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "language mismatch: {} vs {names} — {}: {declared_count} files, {found} \
         (declare the tree with `excluded_sources` entries or fix [project] language)",
        declared.as_str(),
        declared.as_str(),
    ))
}

/// Marker walk for the language chain rust > csharp > go > ts > python >
/// kotlin — the FALLBACK arm, consulted by [`select`] only when the scan
/// root carries no root marker of its own. The walk runs behind the
/// detection exclusion set; the ts arm is the FALLBACK within the fallback,
/// selected only when no rust, csharp or go marker survives the exclusions;
/// the python arm ranks BELOW ts (python driver record P1: on a mixed
/// sources-only root the shipped ts verdict must not move); the kotlin arm
/// ranks LOWEST (kotlin driver record K-P1: every shipped chain answer
/// stays byte-identical, only trees that answered the zero-hits string
/// change). Marker collection
/// is order-free, so the same tree never detects different languages on
/// different walks.
pub fn detect(root: &Path) -> Option<Language> {
    let mut has_rust = false;
    let mut has_csharp = false;
    let mut has_go = false;
    let mut has_ts = false;
    let mut has_python = false;
    let mut has_kotlin = false;

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| !excluded_path(entry.path()))
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "Cargo.toml" || name.ends_with(".rs") {
            has_rust = true;
        } else if name.ends_with(".csproj") || name.ends_with(".sln") || name.ends_with(".cs") {
            has_csharp = true;
        } else if name == "go.mod" || name.ends_with(".go") {
            has_go = true;
        } else if is_ts_source(&name) || name == "package.json" {
            has_ts = true;
        } else if is_python_source(&name) {
            has_python = true;
        } else if is_kotlin_source(&name) {
            has_kotlin = true;
        }
    }

    if has_rust {
        Some(Language::Rust)
    } else if has_csharp {
        Some(Language::Csharp)
    } else if has_go {
        Some(Language::Go)
    } else if has_ts {
        Some(Language::Ts)
    } else if has_python {
        Some(Language::Python)
    } else if has_kotlin {
        Some(Language::Kotlin)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct Tree(tempfile::TempDir);

    impl Tree {
        fn new() -> Self {
            Self(tempfile::tempdir().expect("tempdir"))
        }
        fn write(&self, rel: &str, content: &str) {
            let path = self.0.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        fn path(&self) -> &Path {
            self.0.path()
        }
    }

    #[test]
    fn from_name_round_trips_as_str() {
        for language in [
            Language::Rust,
            Language::Csharp,
            Language::Go,
            Language::Ts,
            Language::Python,
            Language::Kotlin,
        ] {
            assert_eq!(from_name(language.as_str()), Some(language));
        }
        assert_eq!(from_name("java"), None);
        // The table is the ONE owner: every entry parses, and its name is
        // exactly `as_str` (the schema enum and the parser cannot drift).
        assert_eq!(LANGUAGES.len(), 6);
        for (name, language) in LANGUAGES {
            assert_eq!(*name, language.as_str());
        }
    }

    #[test]
    fn framework_registry_is_per_driver_with_one_validator_owner() {
        // The ts driver registers sveltekit today; no other driver
        // registers a framework (round-8 P-FW1).
        assert_eq!(frameworks(Language::Ts), &["sveltekit"]);
        for language in [
            Language::Rust,
            Language::Csharp,
            Language::Go,
            Language::Python,
            Language::Kotlin,
        ] {
            assert!(frameworks(language).is_empty());
        }
        assert_eq!(frameworks_all(), vec!["sveltekit"]);
        // Registered names validate; unknown names answer the fail-closed
        // red naming the registered alternatives (`none` for a driver
        // without a registry).
        assert_eq!(framework_error(Language::Ts, "sveltekit"), None);
        assert_eq!(
            framework_error(Language::Ts, "nextjs"),
            Some("unknown framework \"nextjs\" — registered: sveltekit".to_string())
        );
        assert_eq!(
            framework_error(Language::Rust, "sveltekit"),
            Some("unknown framework \"sveltekit\" — registered: none".to_string())
        );
    }

    #[test]
    fn ts_tree_detects_the_fallback_arm() {
        let tree = Tree::new();
        tree.write("package.json", "{}");
        tree.write("src/main.ts", "export const x = 1;");
        assert_eq!(detect(tree.path()), Some(Language::Ts));
        assert!(has_sources(tree.path(), Language::Ts));
    }

    #[test]
    fn ts_detection_does_not_need_the_package_marker() {
        let tree = Tree::new();
        tree.write("src/app.tsx", "export const App = 1;");
        tree.write("src/ui.js", "export const ui = 1;");
        tree.write("src/View.svelte", "<script>export let x;</script>");
        assert_eq!(detect(tree.path()), Some(Language::Ts));
    }

    #[test]
    fn root_package_json_selects_ts_over_nested_rust_sources() {
        // DET-STRAY-FLIP (the flipped `rust_outranks_the_ts_fallback_in_both_orders`):
        // the root marker names the driver; a nested .rs cannot outrank it —
        // the rust driver's member discovery is manifest-driven and could
        // never extract this tree.
        let tree = Tree::new();
        tree.write("package.json", "{}");
        tree.write("src/lib.rs", "");
        assert_eq!(select(tree.path(), None), Ok(Some(Language::Ts)));
    }

    #[test]
    fn one_root_marker_selects_its_driver() {
        let rust = Tree::new();
        rust.write("Cargo.toml", "[package]\nname = \"t\"\n");
        rust.write("src/lib.rs", "");
        assert_eq!(select(rust.path(), None), Ok(Some(Language::Rust)));
        let go = Tree::new();
        go.write("go.mod", "module example.com/x\n");
        go.write("main.go", "package main\n");
        assert_eq!(select(go.path(), None), Ok(Some(Language::Go)));
    }

    #[test]
    fn two_root_markers_without_a_pin_are_ambiguous() {
        // DET-ROOT-AMBIGUOUS: the collision names EVERY marker file, in
        // table order, regardless of write order on disk.
        let tree = Tree::new();
        tree.write("package.json", "{}");
        tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
        let err = select(tree.path(), None).expect_err("ambiguity is red");
        assert_eq!(
            err,
            "ambiguous root markers: Cargo.toml (rust), package.json (ts) — pin [project] language or scan a single-language root"
        );
    }

    #[test]
    fn a_pin_resolves_the_ambiguity_when_its_marker_is_present() {
        // DET-ROOT-AMBIGUOUS-PINNED: the pin selects the driver; the D14
        // agreement counts stay the caller's check, unchanged.
        let tree = Tree::new();
        tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
        tree.write("package.json", "{}");
        assert_eq!(select(tree.path(), Some(Language::Rust)), Ok(Some(Language::Rust)));
    }

    #[test]
    fn a_pin_without_its_root_marker_names_the_markers_found() {
        // DET-PIN-NO-ROOT-MARKER: the mismatch branch keeps the
        // `language mismatch:` byte prefix and names the marker files.
        let tree = Tree::new();
        tree.write("Cargo.toml", "[package]\nname = \"t\"\n");
        tree.write("src/lib.rs", "");
        let err = select(tree.path(), Some(Language::Ts)).expect_err("pin red");
        assert_eq!(
            err,
            "language mismatch: ts — no root marker package.json for the declared language, found: Cargo.toml (rust)"
        );
    }

    #[test]
    fn a_markerless_root_keeps_the_chain_fallback_under_a_pin() {
        // The no-walk-down truth stays: a pin over a marker-less tree keeps
        // the chain walk as the presence test and the pin as the driver.
        let tree = Tree::new();
        tree.write("src/app.tsx", "export const App = 1;");
        assert_eq!(select(tree.path(), Some(Language::Ts)), Ok(Some(Language::Ts)));
        let empty = Tree::new();
        empty.write("README.md", "nothing here");
        assert_eq!(select(empty.path(), Some(Language::Go)), Ok(None));
        assert_eq!(select(empty.path(), None), Ok(None));
    }

    #[test]
    fn nested_foreign_markers_report_sorted_below_a_rooted_scan() {
        let tree = Tree::new();
        tree.write("package.json", "{}");
        tree.write("src/app.ts", "export const a = 1;");
        tree.write("crypto/go.mod", "module example.com/crypto\n");
        tree.write("side/Cargo.toml", "[package]\nname = \"side\"\n");
        let report = marker_report(tree.path(), Language::Ts);
        assert_eq!(
            report,
            "nested foreign marker: crypto/go.mod (go) — not modelled\n\
             nested foreign marker: side/Cargo.toml (rust) — not modelled\n"
        );
        // The driver's own nested markers state nothing.
        let ts = Tree::new();
        ts.write("go.mod", "module example.com/x\n");
        ts.write("main.go", "package main\n");
        ts.write("plugin/package.json", "{}");
        assert_eq!(
            marker_report(ts.path(), Language::Go),
            "nested foreign marker: plugin/package.json (ts) — not modelled\n"
        );
    }

    #[test]
    fn excluded_trees_carry_no_nested_marker_line() {
        // DET-NESTED-EXCLUDED-SILENT: the DETECT_EXCLUDE_DIRS scoping —
        // a dependency tree's marker is not a first-party fact.
        let tree = Tree::new();
        tree.write("go.mod", "module example.com/x\n");
        tree.write("main.go", "package main\n");
        tree.write("node_modules/dep/package.json", "{}");
        assert_eq!(marker_report(tree.path(), Language::Go), "");
    }

    #[test]
    fn go_marker_outranks_ts_plugin_sources() {
        let tree = Tree::new();
        tree.write("go.mod", "module example.com/tool\n");
        tree.write("cmd/tool/main.go", "package main\n");
        tree.write(
            "opencode-plugin/package.json",
            "{\"name\": \"plugin\"}",
        );
        tree.write("opencode-plugin/index.ts", "export const p = 1;");
        assert_eq!(detect(tree.path()), Some(Language::Go));
    }

    #[test]
    fn excluded_markers_never_select_a_driver() {
        // A dependency tree's package.json (and its .js sources) sit behind
        // the exclusion set: alone, they select nothing.
        let tree = Tree::new();
        tree.write("node_modules/left-pad/package.json", "{\"name\": \"left-pad\"}");
        tree.write("node_modules/left-pad/index.js", "module.exports = 1;");
        assert_eq!(detect(tree.path()), None);
        assert!(!has_sources(tree.path(), Language::Ts));
        // Build output under target/ cannot select rust either; a real go
        // marker elsewhere still wins.
        let build = Tree::new();
        build.write("target/gen/out.rs", "");
        build.write("package.json", "{}");
        assert_eq!(detect(build.path()), Some(Language::Ts));
        let go = Tree::new();
        go.write("go.mod", "module example.com/x\n");
        go.write("node_modules/pkg/package.json", "{}");
        assert_eq!(detect(go.path()), Some(Language::Go));
    }

    #[test]
    fn svelte_only_tree_is_ts() {
        let tree = Tree::new();
        tree.write("src/App.svelte", "<script lang=\"ts\"></script><h1>hi</h1>");
        assert_eq!(detect(tree.path()), Some(Language::Ts));
    }

    #[test]
    fn agreement_counts_sources_not_markers_and_skips_declared_roots() {
        let tree = Tree::new();
        tree.write("go.mod", "module example.com/tool\n");
        tree.write("cmd/tool/main.go", "package main\n");
        tree.write("opencode-plugin/package.json", "{}");
        tree.write("opencode-plugin/index.ts", "export const p = 1;");
        // Markers (go.mod, package.json) state no counts; sources do.
        let counts = count_sources(tree.path(), &[]);
        assert_eq!(counts, vec![(Language::Go, 1), (Language::Ts, 1)]);
        let finding = agreement_finding(tree.path(), Language::Go, &[]).expect("mismatch");
        assert!(finding.contains("go vs ts"), "{finding}");
        assert!(finding.contains("go: 1 files") && finding.contains("ts: 1 files"), "{finding}");
        // An declared exclusion root silences the foreign subtree.
        let excluded = [std::path::PathBuf::from("opencode-plugin")];
        assert_eq!(count_sources(tree.path(), &excluded), vec![(Language::Go, 1)]);
        assert!(agreement_finding(tree.path(), Language::Go, &excluded).is_none());
    }

    #[test]
    fn exclusion_roots_match_componentwise_never_by_prefix_text() {
        let tree = Tree::new();
        tree.write("go.mod", "module example.com/tool\n");
        tree.write("main.go", "package main\n");
        tree.write("webapp/app.ts", "export const a = 1;");
        // The entry `web` never swallows `webapp`.
        let excluded = [std::path::PathBuf::from("web")];
        let finding = agreement_finding(tree.path(), Language::Go, &excluded).expect("webapp stays foreign");
        assert!(finding.contains("ts: 1 files"), "{finding}");
    }
}
