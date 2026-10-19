// Shared spec model for the landed consumers: diagram, verify, update and
// report each read what they need (see `verify/compare/` and `commands/`).

use crate::archspec::language;
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
    /// Declared framework activation (round-8 framework-config doctrine):
    /// a SINGLE optional name from the selected driver's registry
    /// (`language::frameworks`) — never auto-discovered, never a list.
    /// Absent keeps every driver at its framework-less semantics; an
    /// unknown name fails the load (the `validate_schema` arm).
    #[serde(default)]
    pub framework: Option<String>,
    /// Declared source exclusions (ts-driver record, D14): an exclusion that
    /// hides language-bearing files must say why, in the report, every run.
    /// Empty by default — specs that never use the key are unaffected.
    #[serde(default)]
    pub excluded_sources: Vec<ExcludedSource>,
}

/// One `[project] excluded_sources` entry: `path` and `reason` are both
/// mandatory — serde defaults them to `None` so the schema validator can
/// name the entry (1-indexed) and the missing key, the same message class
/// `validate_constraint` prints.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ExcludedSource {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
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
    pub depend_on: Vec<AllowedEntry>,
    pub forbidden: Vec<AllowedEntry>,
}

/// One entry of `allowed.depend_on`/`allowed.forbidden`. The shipped shape
/// is a plain target string, which takes the `count` default; the table
/// shape `{ target, type_only }` carries the entry-level exemption key
/// (D3 of the ts-driver design record). The value domain is validated at
/// spec load and the dependency adjudication reads the declaration: `count`
/// counts every crossing like any other edge — byte-identical to the
/// shipped law — while `exempt` admits the pair's type-only crossing and
/// still reddens the value crossing. On models without the kind fact
/// (rust, go, csharp) every crossing is a value crossing. The cycle tier
/// knows no exemption: it counts every edge regardless of kind.
#[derive(Debug, Clone)]
pub struct AllowedEntry {
    pub target: String,
    /// The per-entry dependency law selector, stored at load so adjudication
    /// reads what the spec declared instead of re-parsing the file
    /// (`TYPE_ONLY_VALUES`, D3).
    pub type_only: &'static str,
}

/// The value domain of the entry-level `type_only` key.
pub const TYPE_ONLY_VALUES: [&str; 2] = ["count", "exempt"];

impl AllowedEntry {
    /// The plain-string shape: the `count` default.
    fn plain(target: String) -> Self {
        Self {
            target,
            type_only: "count",
        }
    }
}

/// Validate one `depend_on`/`forbidden` array element (1-indexed `ordinal`
/// in the error message) into an entry. Plain strings take the `count`
/// default; tables carry a mandatory string `target` and an optional
/// `type_only` from the named value domain.
fn parse_allowed_entry(list: &str, ordinal: usize, value: toml::Value) -> Result<AllowedEntry, String> {
    match value {
        toml::Value::String(target) => Ok(AllowedEntry::plain(target)),
        toml::Value::Table(table) => {
            let mut keys = table.keys().collect::<Vec<_>>();
            keys.retain(|key| key.as_str() != "target" && key.as_str() != "type_only");
            if !keys.is_empty() {
                keys.sort();
                return Err(format!(
                    "{list} entry #{ordinal}: unknown key(s) {} — supported keys are \"target\" and \"type_only\"",
                    keys.iter()
                        .map(|key| format!("\"{key}\""))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            let Some(target) = table.get("target").and_then(toml::Value::as_str) else {
                return Err(format!(
                    "{list} entry #{ordinal}: missing mandatory key \"target\" — a table entry \
                     states the target and an optional \"type_only\""
                ));
            };
            let type_only = match table.get("type_only") {
                None => "count",
                Some(toml::Value::String(value)) if TYPE_ONLY_VALUES.contains(&value.as_str()) => {
                    if value == "exempt" {
                        "exempt"
                    } else {
                        "count"
                    }
                }
                Some(other) => {
                    return Err(format!(
                        "{list} entry #{ordinal}: type_only value {other} is not supported — \
                         supported values are \"count\" and \"exempt\""
                    ));
                }
            };
            Ok(AllowedEntry {
                target: target.to_string(),
                type_only,
            })
        }
        other => Err(format!(
            "{list} entry #{ordinal}: expected a target string or a {{ target, type_only }} \
             table, found {other}"
        )),
    }
}

/// Deserialize with the `declared` marker: a present table is declared even
/// when both lists are empty; an absent key is not. Entry elements are
/// validated per `parse_allowed_entry`.
impl<'de> serde::Deserialize<'de> for Allowed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Fields {
            #[serde(default)]
            depend_on: Vec<toml::Value>,
            #[serde(default)]
            forbidden: Vec<toml::Value>,
        }
        let fields = Fields::deserialize(deserializer)?;
        let mut depend_on = Vec::with_capacity(fields.depend_on.len());
        for (index, value) in fields.depend_on.into_iter().enumerate() {
            depend_on.push(parse_allowed_entry("allowed.depend_on", index + 1, value).map_err(serde::de::Error::custom)?);
        }
        let mut forbidden = Vec::with_capacity(fields.forbidden.len());
        for (index, value) in fields.forbidden.into_iter().enumerate() {
            forbidden.push(parse_allowed_entry("allowed.forbidden", index + 1, value).map_err(serde::de::Error::custom)?);
        }
        Ok(Self {
            declared: true,
            depend_on,
            forbidden,
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
    /// `no_cycles` adjudication scope (ts-driver design record D5):
    /// `"claims"` | `"both"`. Optional — an absent value derives from the
    /// per-language posture table. Validation rejects the field on every
    /// other row kind and rejects any value outside the two above.
    #[serde(default)]
    pub scope: Option<String>,
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

/// Scan's root-only spec discovery (ts-driver record, D14(4)): the spec file
/// is read ONLY when it sits at the scanned root — never an upward walk,
/// never a config indirection. A missing spec is not an error for `scan`
/// (the verb never requires one); a present spec is validated exactly like
/// `load_verify` validates, so an invalid spec fails `scan` with the shipped
/// invalid-spec class — untagged in `cli::classify`, keeping exit 1.
pub fn load_scan(dir: &Path) -> Result<Option<Spec>, String> {
    let spec_path = dir.join(SPEC_FILE);
    if !spec_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&spec_path)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    let spec: Spec = toml::from_str(&content)
        .map_err(|err| format!("invalid spec: {}: {err}", spec_path.display()))?;
    validate_schema(&spec_path, &spec)?;
    Ok(Some(spec))
}

/// The `excluded sources` report line (D13 owner: the scan/verify report
/// surface): every active entry, one line per entry, exact bytes
/// `excluded sources: <path> — <reason>` — a run with an active exclusion
/// that prints no line is red-by-test (R3-7). Printed on stderr so the
/// stdout machine body stays pure (wave-3 D02) and rust stdout verdict
/// bytes stay byte-identical.
pub fn excluded_sources_report(spec: &Spec) -> String {
    let mut out = String::new();
    for entry in &spec.project.excluded_sources {
        // The schema validator admits only entries with both keys present
        // and non-empty; the fallbacks keep this line total for callers
        // that skipped validation.
        let path = entry.path.as_deref().unwrap_or_default();
        let reason = entry.reason.as_deref().unwrap_or_default();
        out.push_str(&format!("excluded sources: {path} — {reason}\n"));
    }
    out
}

/// The declared exclusion roots as paths, for the counted walks (D14: the
/// agreement check consumes the entries the report line prints). The schema
/// validator has admitted only non-empty paths by the time this runs;
/// entries without a path contribute nothing.
pub fn exclusion_paths(spec: &Spec) -> Vec<std::path::PathBuf> {
    spec.project
        .excluded_sources
        .iter()
        .filter_map(|entry| entry.path.as_deref())
        .map(|path| std::path::PathBuf::from(path.trim()))
        .filter(|path| !path.as_os_str().is_empty())
        .collect()
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
    // The `scope` field of the no_cycles arm (ts-driver design record D5):
    // accepted only on `no_cycles` rows; the two supported values are named
    // in the invalid-value message, same shipped invalid-spec class as the
    // severity check above. The round-1 `files` value is deleted (B1) — it
    // lands in the same invalid-value class.
    if let Some(scope) = &constraint.scope {
        if constraint.kind != "no_cycles" {
            return Err(format!(
                "invalid spec: {}: [constraint] #{} declares `scope` on a \"{}\" row; `scope` is supported only on `no_cycles` rows",
                spec_path.display(),
                index + 1,
                constraint.kind
            ));
        }
        if scope != "claims" && scope != "both" {
            return Err(format!(
                "invalid spec: {}: constraint #{} has invalid scope \"{}\" (supported: claims, both)",
                spec_path.display(),
                index + 1,
                scope
            ));
        }
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
    // The `[project] framework` gate (round-8 P-FW1): validated against the
    // declared driver's registry through the ONE validator owner. An
    // unparseable language name skips this arm — the shipped unknown-
    // language red owns that value at the verb sites.
    if let Some(name) = spec.project.framework.as_deref()
        && let Some(driver) = language::from_name(&spec.project.language)
        && let Some(finding) = language::framework_error(driver, name)
    {
        return Err(format!("invalid spec: {}: {finding}", spec_path.display()));
    }
    for (index, entry) in spec.project.excluded_sources.iter().enumerate() {
        let name = |key: &str| {
            Err(format!(
                "invalid spec: {}: [project] excluded_sources entry #{} requires a non-empty `{key}` key",
                spec_path.display(),
                index + 1,
            ))
        };
        match (&entry.path, &entry.reason) {
            (None, _) => name("path")?,
            (Some(path), _) if path.trim().is_empty() => name("path")?,
            (_, None) => name("reason")?,
            (_, Some(reason)) if reason.trim().is_empty() => name("reason")?,
            _ => {}
        }
    }
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
            scope: None,
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

