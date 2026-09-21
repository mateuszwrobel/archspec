use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::spec;

/// Annotated reference for `architecture.spec.toml` — every section, with
/// inline comments. The single source shared by `archspec spec` and the
/// command's `--help` output, so the two cannot drift.
pub const HELP: &str = "\
usage: archspec spec [--schema]
print the architecture.spec.toml JSON schema (--schema) or annotated reference

  --schema   print a JSON Schema (draft-07) for architecture.spec.toml to stdout

# architecture.spec.toml — annotated reference
# Boundaries and relations, not tree shape. See docs/archspec/spec.md.

[project]
language = \"rust\"   # rust | csharp | go

[[module]]
name = \"domain\"                                   # required
matches = { units = [\"core\"], modules = [\"Billing::domain\"] }
# a trailing '*' in matches.modules claims a path prefix (grouping): \"Billing::domain::*\"
# folds every submodule under one boundary; overlaps resolve by specificity, ties fail
contract = { forbid = [\"entity\"] }                 # expose is reserved: declaring it is a schema error
[module.allowed]
depend_on = [\"domain\", \"ports\"]
forbidden = [\"infrastructure\"]

[module.submodules]                                 # recurses: same shape as [[module]]
name = \"domain::entities\"
matches = { modules = [\"Billing::domain::entities\"] }

[[constraint]]
type = \"no_cycles\"                                # one of 7 types
modules = [\"domain\", \"ports\", \"adapters\"]      # optional
severity = \"error\"                                # error | warning (optional)

[[constraint]]
type = \"public_api_allowlist\"
allowed = [\"domain\"]

[[constraint]]
type = \"forbid_external_crates\"
from = [\"app::ui\"]
forbid = [\"serde\"]

[[constraint]]
type = \"external_free\"                              # zero-dependency purity: what 'from' matches imports nothing
from = [\"app::domain\"]                              # put it on leaf layers (domain/core), not composition roots

[[constraint]]
type = \"manifest_integrity\"

[[constraint]]
type = \"feature_boundary\"
feature = \"real-time\"
gated_modules = [\"streaming\"]
allowed_from = [\"app::core\"]

[[constraint]]
type = \"forbid_submodule_dependency\"
parent = \"orchestration\"
from = [\"common\"]
forbid = [\"control_loop\"]

[[stereotype]]
name = \"domain\"
match = { names = [\"domain\"] }
";

const ALL_TYPES: [&str; 7] = [
    "no_cycles",
    "manifest_integrity",
    "public_api_allowlist",
    "forbid_external_crates",
    "forbid_submodule_dependency",
    "feature_boundary",
    "external_free",
];

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[FlagSpec {
            name: "schema",
            takes_value: false,
        }],
    )?;
    if !parsed.positionals.is_empty() {
        return Err("spec takes no path argument".to_string());
    }
    if parsed.switches.contains("schema") {
        print!("{}", schema_json());
    } else {
        print!("{HELP}");
    }
    Ok(())
}

/// Build the JSON Schema (draft-07) for `architecture.spec.toml`. Built from
/// the shared `CONSTRAINT_REQUIRED_FIELDS` table so per-type required arrays
/// cannot diverge from what `validate_constraint` enforces. Serialized with
/// `serde_json` (default BTreeMap ordering) → deterministic byte-identical
/// output across runs.
fn schema_json() -> String {
    let mut constraint_items = Vec::new();
    for (kind, required) in spec::CONSTRAINT_REQUIRED_FIELDS {
        constraint_items.push(serde_json::json!({
            "properties": { "type": { "const": kind } },
            "required": required.to_vec(),
        }));
    }
    let schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "architecture.spec.toml",
        "type": "object",
        "properties": {
            "project": {
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["rust", "csharp", "go"]
                    }
                },
                "required": ["language"]
            },
            "module": {
                "type": "array",
                "items": module_schema(2)
            },
            "constraint": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "type": { "enum": ALL_TYPES },
                        "severity": { "enum": ["error", "warning"] },
                        "modules": { "type": "array", "items": { "type": "string" } }
                    },
                    "oneOf": constraint_items
                }
            },
            "stereotype": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": { "name": { "type": "string" } }
                }
            }
        }
    });
    serde_json::to_string_pretty(&schema).expect("schema serializes")
}

/// Module schema fragment; `submodules` recurses to a bounded depth, mirroring
/// the unbounded nesting the TOML permits while keeping the schema finite.
fn module_schema(depth: usize) -> serde_json::Value {
    let mut properties = serde_json::json!({
        "name": { "type": "string" },
        "matches": {
            "type": "object",
            "properties": {
                "units": { "type": "array", "items": { "type": "string" } },
                "modules": { "type": "array", "items": { "type": "string" }, "description": "dotted module-path patterns; a trailing '*' claims a path prefix (grouping many submodules under one boundary); ownership resolves by specificity and equal-specificity conflicts are reported as ambiguous module matches" }
            }
        },
        "contract": {
            "type": "object",
            "properties": {
                "forbid": { "type": "array", "items": { "type": "string" } }
            },
            "not": { "required": ["expose"] },
            "description": "forbid is enforced (contract leaks); expose is reserved and enforced by no check, so declaring it is rejected"
        },
        "allowed": {
            "type": "object",
            "properties": {
                "depend_on": { "type": "array", "items": { "type": "string" } },
                "forbidden": { "type": "array", "items": { "type": "string" } }
            }
        }
    });
    if depth > 0 {
        properties["submodules"] =
            serde_json::json!({ "type": "array", "items": module_schema(depth - 1) });
    }
    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": ["name"]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_per_type_required_matches_shared_table() {
        let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
        let items = &schema["properties"]["constraint"]["items"];

        let enum_types: Vec<&str> = items["properties"]["type"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(enum_types.len(), spec::CONSTRAINT_REQUIRED_FIELDS.len());
        assert_eq!(enum_types, ALL_TYPES.to_vec());

        let one_of = items["oneOf"].as_array().unwrap();
        assert_eq!(one_of.len(), spec::CONSTRAINT_REQUIRED_FIELDS.len());
        for (kind, fields) in spec::CONSTRAINT_REQUIRED_FIELDS {
            let entry = one_of
                .iter()
                .find(|e| e["properties"]["type"]["const"] == serde_json::json!(kind))
                .unwrap_or_else(|| panic!("schema must contain a branch for {kind}"));
            let required: Vec<&str> = entry["required"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            assert_eq!(
                required, *fields,
                "per-type required arrays for {kind} must match the shared table"
            );
        }
    }

    #[test]
    fn schema_rejects_reserved_expose_key() {
        let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
        let contract = &schema["properties"]["module"]["items"]["properties"]["contract"];
        assert!(
            contract["properties"].get("expose").is_none(),
            "schema must not advertise the unenforced expose key"
        );
        assert!(
            contract["properties"].get("forbid").is_some(),
            "enforced forbid must stay accepted"
        );
        assert_eq!(
            contract["not"],
            serde_json::json!({ "required": ["expose"] }),
            "schema must reject presence of expose"
        );
    }

    #[test]
    fn reference_does_not_bless_expose() {
        assert!(
            !HELP.contains("expose = ["),
            "reference must not present expose as an accepted key:\n{HELP}"
        );
        assert!(
            HELP.contains("expose is reserved"),
            "reference must flag expose as reserved:\n{HELP}"
        );
    }
}