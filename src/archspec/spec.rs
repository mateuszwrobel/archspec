// Shared spec model for the landed consumers: diagram, verify, update and
// report each read what they need (see `verify/compare.rs` and `commands/`).

use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

pub const SPEC_FILE: &str = "architecture.spec.toml";

#[derive(Debug, Clone, Deserialize)]
pub struct Spec {
    pub project: Project,
    #[serde(default)]
    pub stereotype: Vec<Stereotype>,
    #[serde(rename = "module", default)]
    pub modules: Vec<Module>,
    #[serde(default)]
    pub constraint: Vec<Constraint>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    pub language: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Stereotype {
    pub name: String,
    #[serde(rename = "match", default)]
    pub match_set: MatchSet,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MatchSet {
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub units: Vec<String>,
    #[serde(default)]
    pub modules: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Module {
    pub name: String,
    #[serde(rename = "matches", default)]
    pub matches: MatchSet,
    #[serde(default)]
    pub contract: Contract,
    #[serde(default)]
    pub allowed: Allowed,
    #[serde(default)]
    pub submodules: Vec<Module>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Contract {
    /// `Option` distinguishes an absent key from an empty list: presence —
    /// not emptiness — signals a contract the loader cannot enforce, so the
    /// schema validator rejects the key whenever it is `Some` (see
    /// `validate_contract_expose`).
    #[serde(default)]
    pub expose: Option<Vec<String>>,
    #[serde(default)]
    pub forbid: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Allowed {
    #[serde(default)]
    pub depend_on: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Constraint {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub severity: String,
    // `public_api_allowlist`
    #[serde(default)]
    pub allowed: Vec<String>,
    // `forbid_external_crates` and `forbid_submodule_dependency`
    #[serde(default)]
    pub from: Vec<String>,
    #[serde(default)]
    pub forbid: Vec<String>,
    // `manifest_integrity`
    #[serde(default)]
    pub require_publish: Option<bool>,
    #[serde(default)]
    pub forbidden_dependencies: Vec<String>,
    #[serde(default)]
    pub required_features: Vec<String>,
    // `feature_boundary`
    #[serde(default)]
    pub feature: String,
    #[serde(default)]
    pub gated_modules: Vec<String>,
    #[serde(default)]
    pub allowed_from: Vec<String>,
    // `forbid_submodule_dependency`
    #[serde(default)]
    pub parent: String,
}

pub fn load(dir: &Path) -> Result<Spec, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Err(format!("no spec found under: {}", dir.display()));
    }
    let content = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("failed to read spec: {}: {err}", spec_path.display()))?;
    toml::from_str(&content)
        .map_err(|err| format!("failed to parse spec: {}: {err}", spec_path.display()))
}

/// Verify's spec loader: same file and parsing, but verify's contract words
/// the errors differently from diagram's (`load` above). Missing spec names the
/// expected file and location; TOML/schema failures are `invalid spec: <file>`.
/// Schema validation is applied after parsing so a malformed but parseable
/// spec is rejected before extraction.
pub fn load_verify(dir: &Path) -> Result<Spec, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Err(format!(
            "no architecture.spec.toml found in: {}",
            dir.display()
        ));
    }
    let content = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    let spec: Spec = toml::from_str(&content)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    validate_schema(&spec_path, &spec)?;
    Ok(spec)
}

/// Report's spec loader: same file and parsing, but report's contract words the
/// errors differently (commands/report/errors.md). A missing spec names the
/// expected file path; a malformed spec is `invalid spec: <file> (malformed
/// TOML)`. Schema validation is applied after parsing, matching load_verify.
pub fn load_report(dir: &Path) -> Result<Spec, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Err(format!("spec file not found: {}", spec_path.display()));
    }
    let content = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    let spec: Spec = toml::from_str(&content)
        .map_err(|_| format!("invalid spec: {} (malformed TOML)", spec_path.display()))?;
    validate_schema(&spec_path, &spec)?;
    Ok(spec)
}

/// Per-constraint-type list of required non-empty fields. Single source of
/// truth consumed by both the runtime validator (`validate_constraint`) and the
/// JSON Schema generator (`commands::spec`), so the two cannot drift. The empty
/// arrays (`no_cycles`, `manifest_integrity`) declare that no field is required.
pub const CONSTRAINT_REQUIRED_FIELDS: &[(&str, &[&str])] = &[
    ("no_cycles", &[]),
    ("manifest_integrity", &[]),
    ("public_api_allowlist", &["allowed"]),
    ("forbid_external_crates", &["from", "forbid"]),
    ("forbid_submodule_dependency", &["parent", "from", "forbid"]),
    ("feature_boundary", &["feature", "gated_modules"]),
    ("external_free", &["from"]),
];

/// Whether a single required field is empty for the given constraint. List
/// fields are empty when the vector is empty; scalar fields when trimmed empty.
fn required_field_empty(constraint: &Constraint, field: &str) -> bool {
    match field {
        "allowed" => constraint.allowed.is_empty(),
        "from" => constraint.from.is_empty(),
        "forbid" => constraint.forbid.is_empty(),
        "gated_modules" => constraint.gated_modules.is_empty(),
        "feature" => constraint.feature.trim().is_empty(),
        "parent" => constraint.parent.trim().is_empty(),
        _ => false,
    }
}

/// Validate a single constraint's type and required fields. A genuinely
/// unknown `type` is a schema violation naming the file, the entry (1-indexed),
/// and the offending type (#37). Each known type validates the fields its check
/// needs; a missing required field is reported the same way as an unknown type.
fn validate_constraint(
    spec_path: &std::path::Path,
    constraint: &Constraint,
    index: usize,
) -> Result<(), String> {
    let required = |field: &str| {
        Err(format!(
            "invalid spec: {}: [constraint] #{} of type \"{}\" requires a non-empty `{field}` field",
            spec_path.display(),
            index + 1,
            constraint.kind
        ))
    };
    if !CONSTRAINT_REQUIRED_FIELDS
        .iter()
        .any(|(kind, _)| *kind == constraint.kind)
    {
        return Err(format!(
            "invalid spec: {}: unknown constraint type \"{}\" in [constraint] #{}",
            spec_path.display(),
            constraint.kind,
            index + 1
        ));
    }
    for (kind, fields) in CONSTRAINT_REQUIRED_FIELDS {
        if *kind == constraint.kind {
            for field in *fields {
                if required_field_empty(constraint, field) {
                    return required(field);
                }
            }
            break;
        }
    }
    if !constraint.severity.is_empty()
        && constraint.severity != "error"
        && constraint.severity != "warning"
    {
        return Err(format!(
            "invalid spec: {}: constraint #{} has invalid severity \"{}\" (supported: error, warning)",
            spec_path.display(),
            index + 1,
            constraint.severity
        ));
    }
    Ok(())
}

/// `contract.expose` names no check that reads it, so a spec declaring it
/// would promise enforcement that does not exist. Presence of the key —
/// including an empty list — signals an unenforced contract, so the loader
/// rejects it whenever the key is present, at the top level and in any
/// submodule. `contract.forbid` is enforced and stays accepted.
fn validate_contract_expose(spec_path: &std::path::Path, module: &Module) -> Result<(), String> {
    if module.contract.expose.is_some() {
        return Err(format!(
            "invalid spec: {}: module \"{}\" declares contract.expose, which is reserved and enforced by no check; remove it (contract.forbid is enforced)",
            spec_path.display(),
            module.name
        ));
    }
    for submodule in &module.submodules {
        validate_contract_expose(spec_path, submodule)?;
    }
    Ok(())
}

/// Basic schema validation over the fields verify reads. Unknown constraint
/// types and empty module names are schema violations naming the file, the
/// offending entry (1-indexed), and why. Extend as later behaviors add checks.
fn validate_schema(spec_path: &std::path::Path, spec: &Spec) -> Result<(), String> {
    for (index, constraint) in spec.constraint.iter().enumerate() {
        validate_constraint(spec_path, constraint, index)?;
    }
    for (index, module) in spec.modules.iter().enumerate() {
        if module.name.trim().is_empty() {
            return Err(format!(
                "invalid spec: {}: module #{} has an empty name",
                spec_path.display(),
                index + 1
            ));
        }
        validate_contract_expose(spec_path, module)?;
    }
    let declared: BTreeSet<&str> = spec
        .modules
        .iter()
        .map(|module| module.name.as_str())
        .collect();
    for (index, constraint) in spec.constraint.iter().enumerate() {
        for name in &constraint.modules {
            if !declared.contains(name.as_str()) {
                return Err(format!(
                    "invalid spec: {}: constraint #{} references undeclared module \"{}\"",
                    spec_path.display(),
                    index + 1,
                    name
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn constraint(kind: &str) -> Constraint {
        Constraint {
            kind: kind.to_string(),
            modules: Vec::new(),
            severity: String::new(),
            allowed: Vec::new(),
            from: Vec::new(),
            forbid: Vec::new(),
            require_publish: None,
            forbidden_dependencies: Vec::new(),
            required_features: Vec::new(),
            feature: String::new(),
            gated_modules: Vec::new(),
            allowed_from: Vec::new(),
            parent: String::new(),
        }
    }

    fn set_field(constraint: &mut Constraint, field: &str, present: bool) {
        match field {
            "allowed" => constraint.allowed = if present { vec!["a".into()] } else { Vec::new() },
            "from" => constraint.from = if present { vec!["a".into()] } else { Vec::new() },
            "forbid" => constraint.forbid = if present { vec!["a".into()] } else { Vec::new() },
            "gated_modules" => {
                constraint.gated_modules = if present { vec!["a".into()] } else { Vec::new() };
            }
            "feature" => constraint.feature = if present { "f".into() } else { String::new() },
            "parent" => constraint.parent = if present { "p".into() } else { String::new() },
            _ => {}
        }
    }

    #[test]
    fn validator_enforces_shared_required_table() {
        for (kind, fields) in CONSTRAINT_REQUIRED_FIELDS {
            let mut full = constraint(kind);
            for field in *fields {
                set_field(&mut full, field, true);
            }
            assert!(
                validate_constraint(std::path::Path::new("x"), &full, 0).is_ok(),
                "{kind} with all required fields present must validate"
            );

            for field in *fields {
                let mut missing = constraint(kind);
                for other in *fields {
                    set_field(&mut missing, other, true);
                }
                set_field(&mut missing, field, false);
                let err = validate_constraint(std::path::Path::new("x"), &missing, 0)
                    .expect_err("missing required field must fail validation");
                assert!(
                    err.contains(&format!("`{field}`")),
                    "{kind} missing {field} must name it: {err}"
                );
            }
        }
    }
}

