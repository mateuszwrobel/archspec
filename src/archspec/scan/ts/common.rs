//! Shared ts vocabulary (self-audit F1 cycle break): the exclusion classes,
//! the resolution verdict and the specifier helpers. A LEAF of the ts driver:
//! neither `ts` nor `ts::resolution` is imported here — both call down —
//! so the `ts <-> resolution` import ring the F1 census recorded is gone.

use std::path::{Component, Path, PathBuf};

use crate::archspec::{language, spec};
use crate::archspec::spec::Spec;

/// The source extensions the ts driver reads and the resolution law infers.
pub(super) const TS_SOURCE_EXTS: &[&str] =
    &["ts", "tsx", "js", "jsx", "svelte", "mjs", "cjs", "mts", "cts"];

/// Extensions that mark an asset import: an ordinary edge to the exact file,
/// never extension-inferred (D2 asset law — the runtime edges the retired
/// textual guard counted stay visible).
pub(super) const ASSET_EXTS: &[&str] = &[
    "css", "scss", "sass", "less", "styl", "svg", "png", "jpg", "jpeg", "gif", "webp", "avif",
    "woff", "woff2", "ttf", "eot", "mp3", "mp4", "webm", "wav", "ogg", "json", "txt", "wasm",
];

/// The candidate spellings of the shared resolution law (see
/// `candidate_paths`): the relative arm walks these spellings, the
/// member-named subpath arm walks the paths directly (GitHub issue #32 —
/// one table, two arms, homed here so neither arm calls across the
/// `ts <-> resolution` ring the F1 census broke).
pub(super) fn candidates(base: &Path) -> Vec<String> {
    candidate_paths(base)
        .into_iter()
        .map(|path| spell(&path))
        .collect()
}

/// The candidate table as PATHS — the ONE resolution law shared by the
/// relative arm and the member-named subpath arm. The spellings: the exact
/// path when it carries a known extension (a source file or an asset), the
/// NodeNext extension substitution when the literal spelling ends in
/// `.js`/`.mjs`/`.cjs` (record R10, round 4 B — after the literal, never
/// before it), extension inference over the ts source set, and the index
/// law (`dir/x` also tries `dir/x/index.*`).
pub(super) fn candidate_paths(base: &Path) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    let has_known_extension = |path: &Path| -> bool {
        path.extension()
            .map(|extension| {
                let extension = extension.to_string_lossy().to_lowercase();
                TS_SOURCE_EXTS.contains(&extension.as_str())
                    || ASSET_EXTS.contains(&extension.as_str())
            })
            .unwrap_or(false)
    };
    if has_known_extension(base) {
        candidates.push(base.to_path_buf());
    }
    for substituted in nodext_substitutions(base) {
        if !candidates.contains(&substituted) {
            candidates.push(substituted);
        }
    }
    for extension in TS_SOURCE_EXTS {
        let mut with_extension = base.as_os_str().to_os_string();
        with_extension.push(".");
        with_extension.push(*extension);
        candidates.push(PathBuf::from(&with_extension));
    }
    for extension in TS_SOURCE_EXTS {
        let mut index = base.as_os_str().to_os_string();
        index.push("/index.");
        index.push(*extension);
        candidates.push(PathBuf::from(&index));
    }
    candidates
}

/// The NodeNext extension substitutions (record R10, round 4 B): a base
/// spelled `.js` also tries `.ts`, `.tsx` and `.d.ts` (the tsc arm order —
/// GitHub issue #11); `.mjs` tries `.mts`; `.cjs` tries `.cts`. The literal
/// spelling stays FIRST in every row — when both
/// `x.js` and `x.ts` exist, the runtime loads `x.js` and the edge lands
/// there (the mixed-tree law). `.mjs`/`.cjs` literals carry a known
/// extension since the indexing law, so the literal rides the exact-path
/// leg and the substitution rows repeat it after the dedup. No `.svelte`
/// row exists in any mapping. The oxc-side `extension_alias` table stays
/// DISABLED: it fires before the literal and errors on twin-less trees
/// (R10 Rejected, round-4 measurements).
fn nodext_substitutions(base: &Path) -> Vec<PathBuf> {
    let Some(extension) = base.extension() else {
        return Vec::new();
    };
    let extension = extension.to_string_lossy().to_lowercase();
    let row: &[&str] = match extension.as_str() {
        "js" => &["ts", "tsx", "d.ts"],
        "mjs" => &["mjs", "mts"],
        "cjs" => &["cjs", "cts"],
        _ => &[],
    };
    row.iter().map(|target| base.with_extension(target)).collect()
}

/// The declared exclusions: spec entries plus the mechanical set (always on).
pub(super) struct Exclusions {
    /// `excluded_sources` entries (trimmed root + resolved mode). Precedence
    /// at every hit site resolves by specificity through
    /// `spec::matching_exclusion` — never spec order (decision-check MAJOR-1).
    pub(super) declared: Vec<spec::ExclusionEntry>,
}


impl Exclusions {
    pub(super) fn new(loaded: &Spec) -> Self {
        Self {
            declared: spec::exclusion_entries(loaded),
        }
    }
}

/// True when a repo-relative path sits under a mechanical exclusion dir.
/// The absolute classes always exclude (R11: node_modules, dist, target,
/// `.svelte-kit`, `.git`, bin and obj). VENDOR is the one NARROWED class
/// (round-6 P-VENDOR, R11): a vendor segment UNDER THE SCAN ROOT excludes
/// nothing — source-extension glue stays in scope and asset leaf edges
/// land again (the measured `.wasm?url` red was pure vendor exclusion).
/// Collection still keeps source-extension files only (the extension
/// filter); this is the ONE law shared by collect_files and is_excluded.
pub(super) fn mechanically_excluded(rel: &Path) -> bool {
    rel.components().filter_map(|component| match component {
        Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
        _ => None,
    })
    .any(|part| language::is_excluded_dir(&part))
}


/// The exclusion an excluded path hit, named exactly as the run spells it:
/// the spec entry root first (a declared exclusion), else the mechanical
/// directory component. The declared naming resolves by SPECIFICITY (the
/// exact path beats the longest prefix beats shorter prefixes, ties refused
/// at load) — TOML order decides nothing (decision-check MAJOR-1).
pub(super) fn exclusion_hit(rel: &Path, declared: Option<&Exclusions>) -> String {
    if let Some(exclusions) = declared
        && let Some(entry) = spec::matching_exclusion(&exclusions.declared, rel)
    {
        return entry.path.to_string_lossy().replace('\\', "/");
    }
    for component in rel.components() {
        if let Component::Normal(name) = component {
            let name = name.to_string_lossy().to_string();
            if language::is_excluded_dir(&name) {
                return name;
            }
        }
    }
    // Unreachable by construction: callers only ask about excluded paths.
    String::from("the exclusion set")
}


/// The specificity-resolved verdict on an excluded path: the entry (or
/// mechanical directory) the path hits plus the mode the verdict carries.
/// The mechanical classes always carry `abort` — the dial belongs to
/// declared entries only. Hit sites branch on the mode: `abort` keeps the
/// shipped unresolvable line byte-identical, `warn` routes the
/// warning-severity finding and never aborts (decision-check BLOCKER-2).
pub(super) fn exclusion_verdict(rel: &Path, declared: Option<&Exclusions>) -> (String, &'static str) {
    if let Some(exclusions) = declared
        && let Some(entry) = spec::matching_exclusion(&exclusions.declared, rel)
    {
        return (
            entry.path.to_string_lossy().replace('\\', "/"),
            entry.mode,
        );
    }
    (exclusion_hit(rel, None), "abort")
}


#[derive(Clone)]
pub(super) enum Resolved {
    /// A first-party target file, repo-relative git spelling.
    Edge(String),
    /// A bare specifier: the npm package identity in the external tier.
    External(String),
    /// Fails loud (D2). The payload names the exclusion when the specifier
    /// resolves ONLY into excluded paths.
    Unresolvable(String),
    /// The warn-mode exclusion dial (GitHub issue 8 phase 2, BLOCKER-2): the
    /// specificity-resolved entry a hit landed on declares `mode = "warn"`,
    /// so the hit NEVER aborts. The payload is the exclusion naming sentence;
    /// the consumer renders the warning-severity finding naming entry+target
    /// (importing file and specifier), which rides the warnings channel —
    /// `--strict` fails it, without `--strict` the run stays green.
    Warned(String),
    /// The virtual-specifier law (record R7, round 4 C): the report-only
    /// line names the specifier verbatim. NO edge, NO external-tier entry,
    /// exit 0 — the seam recorded the line, the model records nothing.
    Reported,
}


/// The query-suffix law (record R7, round 4 C): a `?` starts a suffix that
/// resolution never sees — the run resolves the BASE. The miss line keeps
/// naming the FULL ORIGINAL specifier: the anchor renderer reads
/// `edge.specifier`, never this stripping result.
pub(crate) fn strip_query(specifier: &str) -> &str {
    specifier.split_once('?').map_or(specifier, |(base, _query)| base)
}


/// The virtual-specifier law (record R7, round 4 C): a URI-scheme-shaped
/// specifier (`^[A-Za-z][A-Za-z0-9+.-]*:`) or one starting with a NUL byte
/// names code no file owns. NEVER delegated, NEVER stripped. The builtins
/// law runs BEFORE this test — `node:` is a scheme the builtins list
/// pre-empts into the external fact (round-2 N3, unchanged).
pub(crate) fn is_virtual_spec(specifier: &str) -> bool {
    if specifier.starts_with('\u{0}') {
        return true;
    }
    let bytes = specifier.as_bytes();
    let Some(&first) = bytes.first() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    let mut index = 1;
    while index < bytes.len()
        && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'+' | b'.' | b'-'))
    {
        index += 1;
    }
    bytes.get(index) == Some(&b':')
}


pub(super) fn spell(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}


pub(super) fn is_excluded(rel: &Path, declared: Option<&Exclusions>) -> bool {
    mechanically_excluded(rel)
        || declared.is_some_and(|exclusions| {
            exclusions
                .declared
                .iter()
                .any(|entry| rel.starts_with(&entry.path))
        })
}


/// The npm package identity of a bare specifier: the first path segment
/// (scoped names keep their scope), `node:` builtins verbatim — one
/// external vocabulary, no ts-specific spelling (D2).
pub(super) fn package_identity(specifier: &str) -> String {
    if specifier.starts_with("node:") {
        return specifier.split('/').next().unwrap_or(specifier).to_string();
    }
    let segments: Vec<&str> = specifier.split('/').collect();
    if specifier.starts_with('@') && segments.len() >= 2 {
        return format!("{}/{}", segments[0], segments[1]);
    }
    segments[0].to_string()
}


pub(super) fn relative(path: &Path, root: &Path) -> Option<String> {
    path.strip_prefix(root).ok().map(spell)
}

