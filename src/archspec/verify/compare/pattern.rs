//! The matching vocabulary shared by every arm: glob patterns, module
//! path matching, unit stripping, and ancestor relations.



use crate::archspec::model::Model;
/// Match a `matches.modules` pattern against a dotted module path. Accepts a
/// pattern addressing the full path, its bare last segment, or the path with
/// its unit prefix stripped (so `auth::config` matches `app::auth::config`).
/// Also treats the pattern as addressing a subtree: it matches the named module
/// and every module path beneath it (any ancestor), so `commands` covers
/// `commands::start`, `commands::init`, &c. Ancestor checks apply to both the
/// full path and its unit-stripped form.
pub(super) fn module_path_matches(pattern: &str, module_path: &str) -> bool {
    if module_matches(pattern, module_path) {
        return true;
    }
    let stripped = strip_unit(module_path);
    if stripped != module_path && module_matches(pattern, stripped) {
        return true;
    }
    let mut current = module_path;
    while let Some((rest, _)) = current.rsplit_once("::") {
        current = rest;
        if module_matches(pattern, current) {
            return true;
        }
        let ancestor_stripped = strip_unit(current);
        if ancestor_stripped != current && module_matches(pattern, ancestor_stripped) {
            return true;
        }
    }
    false
}

/// Match a module-reference pattern against a dotted module path: true if the
/// pattern glob-matches the full path or its bare (last-segment) name. Lets a
/// pattern like `common` match `app::orchestration::common`.
pub(super) fn module_matches(pattern: &str, module_path: &str) -> bool {
    if matches_pattern(module_path, pattern) {
        return true;
    }
    module_path
        .rsplit("::")
        .next()
        .is_some_and(|bare| matches_pattern(bare, pattern))
}

/// True if the module path, or any of its ancestor module paths, matches one of
/// the patterns (so a `from` pattern also covers the module's submodules).
/// True when a pattern addresses the model's unit on the ts driver. Ts
/// `module_external` keys are file paths (the D10 file-identity law), so a
/// unit-name pattern cannot fold into them by prefix; the single-package law
/// (D1) instead attributes the model's one unit to every file key.
pub(super) fn pattern_names_ts_unit(model: &Model, pattern: &str) -> bool {
    matches!(model.language.as_str(), "ts" | "python" | "kotlin")
        && model
            .units
            .iter()
            .any(|unit| matches_pattern(&unit.name, pattern))
}

pub(super) fn module_listed(module_path: &str, patterns: &[String]) -> bool {
    let mut current = module_path;
    loop {
        if patterns
            .iter()
            .any(|pattern| module_matches(pattern, current))
        {
            return true;
        }
        match current.rsplit_once("::") {
            Some((rest, _)) => current = rest,
            None => return false,
        }
    }
}

/// Strip the unit (first segment) from a dotted module path for reporting, so
/// `app::billing::sub` renders as `billing::sub`.
pub(super) fn strip_unit(module_path: &str) -> &str {
    match module_path.split_once("::") {
        Some((_, rest)) if !rest.is_empty() => rest,
        _ => module_path,
    }
}

/// The longest common dotted-prefix of two module paths (their shared ancestor).
pub(super) fn common_ancestor(left: &str, right: &str) -> Option<String> {
    let mut common: Vec<&str> = Vec::new();
    for (x, y) in left.split("::").zip(right.split("::")) {
        if x == y {
            common.push(x);
        } else {
            break;
        }
    }
    if common.is_empty() {
        None
    } else {
        Some(common.join("::"))
    }
}

/// The portion of `path` after the ancestor prefix (`common` -> empty), used to
/// isolate a submodule name within a parent.
pub(super) fn rel_after(path: &str, ancestor: &str) -> String {
    match path.strip_prefix(&format!("{ancestor}::")) {
        Some(rest) => rest.to_string(),
        None => String::new(),
    }
}

/// Glob match over a unit/component name or path. Supports `*` (any run of
/// characters, including path separators) and `**` (same as `*` here). Mirrors
/// the spec.md glob conventions (`*Entity`, `**/domain/**`).
pub fn matches_pattern(value: &str, pattern: &str) -> bool {
    glob_match(value.as_bytes(), pattern.as_bytes())
}

fn glob_match(value: &[u8], pattern: &[u8]) -> bool {
    let mut v = 0;
    let mut p = 0;
    let mut star: Option<usize> = None;
    let mut star_v = 0;

    while v < value.len() {
        if p < pattern.len() && (pattern[p] == b'*') {
            star = Some(p);
            star_v = v;
            p += 1;
        } else if p < pattern.len() && pattern[p] == value[v] {
            p += 1;
            v += 1;
        } else if let Some(star) = star {
            star_v += 1;
            v = star_v;
            p = star + 1;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}