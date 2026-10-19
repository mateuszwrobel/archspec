// npm-workspace registry (ts-resolution R2, R5/R6): the membership map a
// scan starts from. The registry is static — no package manager is run,
// no package-lock is parsed, no node is invoked. A workspace root is a
// `package.json` that carries a `workspaces` field, or a directory whose
// `pnpm-workspace.yaml` declares `packages`. Membership is decided by
// realpath containment, not by the spelling (R5): the registry maps each
// on-disk member directory to its package name.

use std::path::{Path, PathBuf};


/// Root spelling of a workspace member: a plain directory entry or a
/// `glob` pattern entry (and pnpm's `packages` list — the same shape).
pub(crate) fn workspace_globs(manifest: &serde_json::Value) -> Option<Vec<String>> {
    match manifest.get("workspaces")? {
        serde_json::Value::Array(entries) => Some(patterns_from(entries)),
        serde_json::Value::Object(map) => {
            let patterns = map.get("packages")?.as_array()?;
            Some(patterns_from(patterns))
        }
        _ => None,
    }
}

fn patterns_from(entries: &[serde_json::Value]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|entry| entry.as_str())
        .filter(|pattern| !pattern.starts_with('!'))
        .map(str::to_string)
        .collect()
}

/// One registered workspace member: its directory (realpath-normalized)
/// and its declared package name.
#[derive(Clone, Debug)]
pub(crate) struct Member {
    pub dir: PathBuf,
    pub name: String,
}

/// The workspace members declared by one root manifest, beside the root
/// directory that declares them: the linker-family gate of R5 case (iv)
/// (round 4) reads the lockfile NAMES at that root and nowhere else.
#[derive(Clone, Debug)]
pub(crate) struct Registry {
    pub root: PathBuf,
    pub members: Vec<Member>,
}

/// The linker-family signal of one registry root (R5 case (iv), round-4
/// gate): `yarn.lock` or `npm-shrinkwrap.json` names the HOISTING family —
/// under it a bare member-name import the manifest never declared is a
/// real runtime edge, so the membership edge and its report-only line are
/// honest. `pnpm-lock.yaml` or NO lockfile names the refusing family —
/// pnpm-strict denies undeclared imports, so the NotFound red is the
/// runtime truth there. FILE NAMES ONLY: the lockfile content is never
/// parsed (the lockfile is a family signal, never a registry — the
/// rejected lockfile-based-registry entry stands) and a lockfile below
/// the root belongs to no family signal here. The hoisting names are
/// checked first: a tree carrying one of them is hoisting regardless of
/// any other lockfile present beside it.
pub(crate) fn root_lockfile_hoists(root: &Path) -> bool {
    root.join("yarn.lock").is_file() || root.join("npm-shrinkwrap.json").is_file()
}

/// Every ancestor workspace root of `start`, nearest first (the R5 scan
/// starts at the scanned root; a scan rooted inside a member still sees
/// the workspace that declares it). The walk is bounded like the tsconfig
/// discovery bound.
pub(crate) fn scan_roots_ts(start: &Path) -> Result<Vec<Registry>, String> {
    let mut registries: Vec<Registry> = Vec::new();
    let mut current = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    let mut depth = 0usize;
    loop {
        depth += 1;
        if depth > 64 {
            return Err("workspace root search exceeded 64 ancestor directories".to_string());
        }
        let mut patterns: Vec<String> = Vec::new();
        let manifest = current.join("package.json");
        if let Ok(bytes) = std::fs::read(&manifest)
            && let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes)
            && let Some(root_patterns) = workspace_globs(&value)
        {
            patterns = root_patterns;
        }
        let pnpm = current.join("pnpm-workspace.yaml");
        if let Ok(text) = std::fs::read_to_string(&pnpm)
            && let Some(root_patterns) = pnpm_package_patterns(&text)
        {
            patterns.extend(root_patterns);
        }
        if !patterns.is_empty() {
            let mut members = Vec::new();
            expand(&current, &patterns, &mut members)?;
            members.sort_by(|left, right| left.dir.cmp(&right.dir));
            registries.push(Registry {
                root: current.clone(),
                members,
            });
        }
        if !current.pop() {
            break;
        }
    }
    Ok(registries)
}

/// Expand the glob patterns of one workspace root into registered members.
/// A pattern entry without `*` names a directory directly; entries with
/// `*` match one path segment level (the npm and pnpm convention). A
/// matched directory registers only when it carries a package.json with a
/// name; a member dir without a manifest owns its files through the
/// nearest-manifest law instead of a registry entry. Node-like
/// directories are never member candidates.
fn expand(root: &Path, patterns: &[String], members: &mut Vec<Member>) -> Result<(), String> {
    for pattern in patterns {
        let segments = pattern.split('/').collect::<Vec<_>>();
        expand_segments(root, &segments, members)?;
    }
    Ok(())
}

fn expand_segments(dir: &Path, segments: &[&str], members: &mut Vec<Member>) -> Result<(), String> {
    let Some((segment, rest)) = segments.split_first() else {
        register_member(dir, members)?;
        return Ok(());
    };
    if *segment == "*" {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Ok(());
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().is_dir()
                && crate::archspec::language::DETECT_EXCLUDE_DIRS
                    .iter()
                    .any(|dir| dir.name == name)
            {
                continue;
            }
            expand_segments(&entry.path(), rest, members)?;
        }
        return Ok(());
    }
    expand_segments(&dir.join(segment), rest, members)
}

fn register_member(dir: &Path, members: &mut Vec<Member>) -> Result<(), String> {
    let Ok(canonical) = dir.canonicalize() else {
        return Ok(());
    };
    let Ok(bytes) = std::fs::read(canonical.join("package.json")) else {
        return Ok(());
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return Ok(());
    };
    let Some(name) = value.get("name").and_then(|v| v.as_str()) else {
        return Ok(());
    };
    if name.is_empty() || members.iter().any(|member| member.dir == canonical) {
        return Ok(());
    }
    members.push(Member {
        dir: canonical,
        name: name.to_string(),
    });
    Ok(())
}

/// pnpm-workspace.yaml: the `packages:` list, parsed without a YAML crate
/// (the shape the file law allows is one quoted pattern per line).
pub(crate) fn pnpm_package_patterns(text: &str) -> Option<Vec<String>> {
    let mut inside = false;
    let mut patterns = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            inside = trimmed == "packages:";
            continue;
        }
        if inside && trimmed.starts_with('-') {
            let pattern = trimmed[1..].trim().trim_matches(['\'', '"']);
            if !pattern.starts_with('!') {
                patterns.push(pattern.to_string());
            }
        }
    }
    if patterns.is_empty() {
        None
    } else {
        Some(patterns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, "").expect("write");
    }

    /// The family signal is the FILE NAME only: an EMPTY `yarn.lock` or
    /// `npm-shrinkwrap.json` at the registry root names the hoisting family;
    /// an empty `pnpm-lock.yaml` names the refusing family. No content is
    /// ever read.
    #[test]
    fn family_comes_from_the_lockfile_name_only() {
        let tree = tempfile::tempdir().expect("tempdir");
        let root = tree.path();
        touch(&root.join("pnpm-lock.yaml"));
        assert!(!root_lockfile_hoists(root), "pnpm-lock.yaml is the refusing family");
        fs::remove_file(root.join("pnpm-lock.yaml")).expect("remove");
        touch(&root.join("yarn.lock"));
        assert!(root_lockfile_hoists(root), "yarn.lock names the hoisting family");
        fs::remove_file(root.join("yarn.lock")).expect("remove");
        touch(&root.join("npm-shrinkwrap.json"));
        assert!(root_lockfile_hoists(root), "npm-shrinkwrap.json names the hoisting family");
    }

    /// NO lockfile at the registry root is the refusing family: without a
    /// hoisting-family statement archspec may not treat an undeclared
    /// member import as a real runtime link (round-4 gate, exit stays red).
    #[test]
    fn an_absent_lockfile_is_the_refusing_family() {
        let tree = tempfile::tempdir().expect("tempdir");
        assert!(!root_lockfile_hoists(tree.path()), "no lockfile — refusing family");
    }

    /// NESTED lockfiles are no signal: the gate reads the REGISTRY ROOT
    /// only, so a member's own lockfile never opens the membership edge.
    /// A `package-lock.json` names no family either (the law's hoisting
    /// names are `yarn.lock` and `npm-shrinkwrap.json`).
    #[test]
    fn nested_lockfiles_and_foreign_names_are_no_signal() {
        let tree = tempfile::tempdir().expect("tempdir");
        let root = tree.path();
        touch(&root.join("packages/app/yarn.lock"));
        touch(&root.join("packages/app/npm-shrinkwrap.json"));
        assert!(
            !root_lockfile_hoists(root),
            "a lockfile below the registry root is no family signal"
        );
        touch(&root.join("package-lock.json"));
        assert!(
            !root_lockfile_hoists(root),
            "package-lock.json names neither hoisting family file"
        );
    }

    /// The hoisting name wins when lockfiles coexist: the law checks the
    /// hoisting names first — `yarn.lock` beside `pnpm-lock.yaml` is read
    /// as the hoisting family.
    #[test]
    fn a_hoisting_name_wins_over_a_coexisting_pnpm_lock() {
        let tree = tempfile::tempdir().expect("tempdir");
        let root = tree.path();
        touch(&root.join("pnpm-lock.yaml"));
        touch(&root.join("yarn.lock"));
        assert!(root_lockfile_hoists(root), "the hoisting name is checked first");
    }
}
