//! Shared ts vocabulary (self-audit F1 cycle break): the exclusion classes,
//! the resolution verdict and the specifier helpers. A LEAF of the ts driver:
//! neither `ts` nor `ts::resolution` is imported here — both call down —
//! so the `ts <-> resolution` import ring the F1 census recorded is gone.

use std::path::{Component, Path, PathBuf};

use crate::archspec::{language, spec};
use crate::archspec::spec::Spec;

/// The declared exclusions: spec roots plus the mechanical set (always on).
pub(super) struct Exclusions {
    /// `excluded_sources` roots as paths, in spec order (the named exclusion
    /// an unresolvable finding prints).
    pub(super) declared: Vec<PathBuf>,
}


impl Exclusions {
    pub(super) fn new(loaded: &Spec) -> Self {
        Self {
            declared: spec::exclusion_paths(loaded),
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
/// directory component.
pub(super) fn exclusion_hit(rel: &Path, declared: Option<&Exclusions>) -> String {
    if let Some(exclusions) = declared {
        for entry in &exclusions.declared {
            if rel.starts_with(entry) {
                return entry.to_string_lossy().replace('\\', "/");
            }
        }
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


#[derive(Clone)]
pub(super) enum Resolved {
    /// A first-party target file, repo-relative git spelling.
    Edge(String),
    /// A bare specifier: the npm package identity in the external tier.
    External(String),
    /// Fails loud (D2). The payload names the exclusion when the specifier
    /// resolves ONLY into excluded paths.
    Unresolvable(String),
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
                .any(|entry| rel.starts_with(entry))
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

