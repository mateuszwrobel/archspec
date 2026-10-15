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
    /// `Option` distinguishes an absent key from a declared set: `None` means
    /// the claim makes no expose claim at all (verdict-neutral — never an
    /// allow-all and never a deny-all gate), `Some(set)` binds the surface
    /// crossing this claim's boundary under the chain law (every declaring
    /// ancestor binds; nesting may narrow, never widen). Empty `Some([])` is
    /// a declared deny-all ceiling for the declaring claim's surface.
    #[serde(default)]
    pub expose: Option<Vec<String>>,
    #[serde(default)]
    pub forbid: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Allowed {
    /// True when the spec carried the `[module.allowed]` table itself. The
    /// jurisdiction resolver reads this marker: a nested claim WITHOUT a
    /// declared table inherits the nearest ancestor that has one (D2 of the
    /// submodule-dependency-engagement design), while a declared-but-empty
    /// table is a deliberate deny-all ceiling.
    pub declared: bool,
    pub depend_on: Vec<String>,
    pub forbidden: Vec<String>,
}

/// Deserialize with the `declared` marker: a present table is declared even
/// when both lists are empty; an absent key is not.
impl<'de> serde::Deserialize<'de> for Allowed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Fields {
            #[serde(default)]
            depend_on: Vec<String>,
            #[serde(default)]
            forbidden: Vec<String>,
        }
        let fields = Fields::deserialize(deserializer)?;
        Ok(Self {
            declared: true,
            depend_on: fields.depend_on,
            forbidden: fields.forbidden,
        })
    }
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

/// Every declared claim name at every depth, depth-first. Claim names are
/// globally unique (the schema validator enforces it), so this list is also
/// the set of valid `allowed.depend_on`/`allowed.forbidden` targets (D6).
pub fn claim_names(modules: &[Module]) -> Vec<String> {
    let mut names = Vec::new();
    for module in modules {
        names.push(module.name.clone());
        names.extend(claim_names(&module.submodules));
    }
    names
}

pub fn load(dir: &Path) -> Result<Spec, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Err(crate::archspec::cli::operational_error(format!(
            "no spec found under: {}",
            dir.display()
        )));
    }
    let content = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("failed to read spec: {}: {err}", spec_path.display()))?;
    let spec: Spec = toml::from_str(&content)
        .map_err(|err| format!("failed to parse spec: {}: {err}", spec_path.display()))?;
    for module in &spec.modules {
        validate_submodule_units(&spec_path, module)?;
    }
    Ok(spec)
}

/// Verify's spec loader: same file and parsing, but verify's contract words
/// the errors differently from diagram's (`load` above). Missing spec names the
/// expected file and location; TOML/schema failures are `invalid spec: <file>`.
/// Schema validation is applied after parsing so a malformed but parseable
/// spec is rejected before extraction.
pub fn load_verify(dir: &Path) -> Result<Spec, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Err(crate::archspec::cli::operational_error(format!(
            "no architecture.spec.toml found in: {}",
            dir.display()
        )));
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
        return Err(crate::archspec::cli::operational_error(format!(
            "spec file not found: {}",
            spec_path.display()
        )));
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

/// `contract.expose` is schema at every depth (phase 2, ADR-011): entries are
/// non-empty strings and the parser never consults the tree — an entry's arm
/// (stereotype versus symbol) is decided against model facts at verify, not
/// against spec text (workplan workplan_archspec_module_public_api_diagrams,
/// D1). An empty entry is a mechanical authoring error and is rejected here;
/// semantic validation happens at verify. `contract.forbid` is enforced and
/// stays accepted.
fn validate_contract_expose(spec_path: &std::path::Path, module: &Module) -> Result<(), String> {
    if let Some(entries) = &module.contract.expose
        && let Some(index) = entries.iter().position(|entry| entry.trim().is_empty())
    {
        return Err(format!(
            "invalid spec: {}: module \"{}\" declares an empty contract.expose entry #{}; entries are non-empty globs or stereotype names",
            spec_path.display(),
            module.name,
            index + 1
        ));
    }
    for submodule in &module.submodules {
        validate_contract_expose(spec_path, submodule)?;
    }
    Ok(())
}

/// Claim names are globally unique across all depths (D6): every name also
/// addresses its territory as a `depend_on`/`forbidden` target, so a collision
/// would make a target ambiguous. Rejects at parse, naming both declarations.
fn validate_claim_uniqueness<'a>(
    spec_path: &std::path::Path,
    modules: &'a [Module],
    seen: &mut std::collections::BTreeMap<&'a str, ()>,
) -> Result<(), String> {
    for module in modules {
        if seen.contains_key(module.name.as_str()) {
            return Err(format!(
                "invalid spec: {}: claim name \"{}\" is declared twice; claim names are unique \
                 across all depths, give one of them a distinct name",
                spec_path.display(),
                module.name
            ));
        }
        seen.insert(module.name.as_str(), ());
        validate_claim_uniqueness(spec_path, &module.submodules, seen)?;
    }
    Ok(())
}

/// `matches.units` on a submodule is rejected at parse (D7): the unit tier
/// stays first-declared-wins at the top level, so a nested unit claim would
/// smuggle a second, order-dependent ownership semantics into it.
fn validate_submodule_units(spec_path: &std::path::Path, module: &Module) -> Result<(), String> {
    for submodule in &module.submodules {
        if !submodule.matches.units.is_empty() {
            return Err(format!(
                "invalid spec: {}: submodule \"{}\" declares matches.units; submodules claim \
                 the module tier only",
                spec_path.display(),
                submodule.name
            ));
        }
        validate_submodule_units(spec_path, submodule)?;
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
        validate_submodule_units(spec_path, module)?;
    }
    let mut seen: std::collections::BTreeMap<&str, ()> = std::collections::BTreeMap::new();
    validate_claim_uniqueness(spec_path, &spec.modules, &mut seen)?;
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

