use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::language;
use crate::archspec::spec;

/// Annotated reference for `architecture.spec.toml` — every section, with
/// inline comments. The single source shared by `archspec spec` and the
/// command's `--help` output, so the two cannot drift.
pub const HELP: &str = "\
usage: archspec spec [--schema]
print the architecture.spec.toml JSON schema (--schema) or annotated reference

  --schema   print a JSON Schema (draft-07) for architecture.spec.toml to stdout

# architecture.spec.toml — annotated reference
# Boundaries and relations, not tree shape. Matching rules: archspec help glob.
# Constraint keys and severity defaults: archspec help constraints.

[project]
language = \"rust\"   # required; the accepted driver names: archspec spec --schema
# framework = \"sveltekit\"  # optional; a driver-registered framework name (the driver's registry: archspec spec --schema); framework semantics never auto-discover
excluded_sources = []  # optional; entries are { path, reason, mode } — path and reason are mandatory per entry; mode is the optional dial: \"abort\" (the default — a resolution hit on the entry fails loud) or \"warn\" (the hit becomes a warning-severity finding naming entry and target: --strict fails, without it the run stays green); matching is component-wise — the entry `web` matches `web` and `web/app.ts`, never `webapp/app.ts`; overlapping entries resolve by specificity (exact match over the longest prefix), never declaration order, ties are an invalid-spec error; every active entry prints in every run's report, and an entry that matched nothing prints one liveness line
# a key the spec model does not define is an invalid-spec error naming the key and its location — at every table, in every verb that reads the spec

[[module]]
name = \"domain\"                                   # required
matches = { units = [\"core\"], modules = [\"Billing::domain\"] }
# a named module claims its WHOLE subtree: \"Billing::domain\" already folds every
# submodule under one boundary, no '*' needed; \"Billing::domain::*\" states that
# same claim as a pinned path prefix; overlaps resolve by specificity, ties fail
contract = { forbid = [\"entity\"], expose = [\"Report\"] }  # forbid bans stereotypes; expose declares the surface that may cross (chain law, any depth)
[module.allowed]
depend_on = [\"domain\", \"ports\"]                   # DECLARED module names — exact, not globs
forbidden = [\"infrastructure\"]                    # same: declared names, never globs

[module.submodules]                                 # recurses: same shape as [[module]]
name = \"domain::entities\"
matches = { modules = [\"Billing::domain::entities\"] }
# a submodule with [module.allowed] owns its territory's dependency checks
# (most-specific claim wins at any depth); without it the nearest declaring
# ancestor's policy applies; matches.units on a submodule is a parse error

[[constraint]]
type = \"no_cycles\"                                # one of 7 types
modules = [\"domain\", \"ports\", \"adapters\"]      # optional
severity = \"error\"                                # error | warning (optional)

[[constraint]]
type = \"public_api_allowlist\"
allowed = [\"domain\"]

[[constraint]]
type = \"forbid_external_crates\"
from = [\"app::ui\"]                              # model-path globs, case-sensitive — not module names
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
/// cannot diverge from what `validate_constraint` enforces, and shaped so its
/// per-table `properties` + `additionalProperties: false` mirror the shared
/// `SPEC_KEY_SHAPES` table the runtime unknown-key validator reads — the unit
/// test `schema_key_sets_match_shared_shape_table` asserts the agreement per
/// shape, so schema and validator cannot drift (issue-8 phase 1, BLOCKER-3).
/// Serialized with `serde_json` (default BTreeMap ordering) → deterministic
/// byte-identical output across runs.
fn schema_json() -> String {
    let mut constraint_items = Vec::new();
    for (kind, required) in spec::CONSTRAINT_REQUIRED_FIELDS {
        constraint_items.push(serde_json::json!({
            "properties": { "type": { "const": kind } },
            "required": required.to_vec(),
        }));
    }
    let string_array = || serde_json::json!({ "type": "array", "items": { "type": "string" } });
    let schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "architecture.spec.toml",
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "project": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "language": {
                        "type": "string",
                        // The ONE driver-name table (`language::LANGUAGES`) —
                        // the emitted enum and the parser cannot drift
                        // (round-8 F11: the stale three-name copy is deleted).
                        "enum": language::LANGUAGES.iter().map(|(name, _)| *name).collect::<Vec<_>>()
                    },
                    "framework": {
                        "type": "string",
                        // The framework registry across drivers
                        // (`language::frameworks_all`): names registered
                        // SOMEWHERE; the per-driver split is validated at
                        // spec load against the selected driver (round-8
                        // P-FW1 — a key, never a discovery).
                        "enum": language::frameworks_all()
                    },
                    "excluded_sources": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": {
                                "path": { "type": "string", "minLength": 1 },
                                "reason": { "type": "string", "minLength": 1 },
                                "mode": { "enum": spec::EXCLUSION_MODE_VALUES }
                            },
                            "required": ["path", "reason"],
                            "description": "declared source exclusion (D14): path is a project-relative source tree matched component-wise (the entry `web` matches `web` and `web/app.ts`, never `webapp/app.ts`), reason states why it is excluded; both keys are mandatory and non-empty per entry (the runtime refuses an empty value as invalid spec); the optional mode dial is \"abort\" (default — a resolution hit on the entry fails loud naming it) or \"warn\" (the hit becomes a warning-severity finding naming entry and target, failing --strict and green without it); overlapping entries resolve by specificity, never declaration order, and equal specificity is an invalid-spec tie; every active entry prints in every run's report and an entry that matched nothing prints one liveness line"
                        }
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
                    "additionalProperties": false,
                    "properties": {
                        "type": { "enum": ALL_TYPES },
                        "severity": { "enum": ["error", "warning"] },
                        "scope": { "enum": ["claims", "both"] },
                        "modules": string_array(),
                        "kinds": { "type": "array", "items": { "enum": ["runtime", "type-assisted"] }, "description": "ring classes a no_cycles row reports; absent means every kind — runtime means every leg carries a value edge, type-assisted means some leg is backed only by type-only rows" },
                        "allowed": string_array(),
                        "from": string_array(),
                        "forbid": string_array(),
                        "require_publish": { "type": "boolean" },
                        "forbidden_dependencies": string_array(),
                        "required_features": string_array(),
                        "feature": { "type": "string" },
                        "gated_modules": string_array(),
                        "allowed_from": string_array(),
                        "parent": { "type": "string" }
                    },
                    "description": "a row's accepted keys are exactly these; a key outside them is refused at load as invalid spec, naming it — the oneOf branches carry per-type required fields, the outer additionalProperties governs unknown keys",
                    "oneOf": constraint_items
                }
            },
            "stereotype": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "name": { "type": "string" },
                        "match": match_schema()
                    },
                    "required": ["name"]
                }
            }
        }
    });
    serde_json::to_string_pretty(&schema).expect("schema serializes")
}

/// The shared match vocabulary (`MatchSet`): `units`/`modules` resolve the
/// two model tiers; `names`/`paths` are the stereotype vocabulary — they
/// deserialize on a module claim's `matches` but verify resolves a module
/// claim on units and modules only.
fn match_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "names": { "type": "array", "items": { "type": "string" }, "description": "name patterns; the stereotype vocabulary — meaningful on a [[stereotype]] match, parsed but not resolved on a module claim's matches" },
            "paths": { "type": "array", "items": { "type": "string" }, "description": "path patterns; the stereotype vocabulary — meaningful on a [[stereotype]] match, parsed but not resolved on a module claim's matches" },
            "units": { "type": "array", "items": { "type": "string" }, "description": "unit names; declarable only on top-level modules — a submodule declares the module tier only, matches.units on a submodule is rejected at parse" },
            "modules": { "type": "array", "items": { "type": "string" }, "description": "dotted module-path patterns; a trailing '*' claims a path prefix (grouping many submodules under one boundary); ownership resolves by specificity and equal-specificity conflicts are reported as ambiguous module matches" }
        }
    })
}

/// Module schema fragment; `submodules` recurses to a bounded depth, mirroring
/// the unbounded nesting the TOML permits while keeping the schema finite.
fn module_schema(depth: usize) -> serde_json::Value {
    let mut properties = serde_json::json!({
        "name": { "type": "string" },
        "matches": match_schema(),
        "contract": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "forbid": { "type": "array", "items": { "type": "string" } },
                "expose": {
                    "type": "array",
                    "items": { "type": "string", "minLength": 1 },
                    "description": "the surface that may cross this claim's boundary, declared at any depth: entries are stereotype names or symbol globs over the fact vocabulary (nested types dotted, generics dropped); every declaring ancestor binds (chain law — nesting narrows, never widens); an absent key makes no claim, an empty list is a declared deny-all ceiling"
                }
            },
            "description": "forbid is enforced (contract leaks); expose is enforced by the expose chain law at every depth"
        },
        "allowed": {
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "depend_on": { "type": "array", "items": allowed_entry_schema() },
                "forbidden": { "type": "array", "items": allowed_entry_schema() }
            }
        }
    });
    if depth > 0 {
        properties["submodules"] =
            serde_json::json!({ "type": "array", "items": module_schema(depth - 1) });
    }
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
        "required": ["name"]
    })
}

/// One `allowed.depend_on`/`allowed.forbidden` element: a plain target string
/// or a `{ target, type_only }` table — the shape `parse_allowed_entry`
/// refuses anything else on (the entry-level unknown-key refusal predates the
/// table-level law and stays its owner).
fn allowed_entry_schema() -> serde_json::Value {
    serde_json::json!({
        "oneOf": [
            { "type": "string" },
            {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "target": { "type": "string" },
                    "type_only": { "enum": spec::TYPE_ONLY_VALUES }
                },
                "required": ["target"]
            }
        ]
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
    fn schema_accepts_expose_at_every_depth() {
        let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
        let contract = &schema["properties"]["module"]["items"]["properties"]["contract"];
        assert!(
            contract["properties"].get("expose").is_some(),
            "schema must advertise the enforced expose key"
        );
        assert!(
            contract["properties"].get("forbid").is_some(),
            "enforced forbid must stay accepted"
        );
        assert!(
            contract.get("not").is_none(),
            "schema must carry no reserved-key rejection"
        );
        let nested = &schema["properties"]["module"]["items"]["properties"]["submodules"]["items"]
            ["properties"]["contract"]["properties"]["expose"];
        assert!(
            nested.get("items").is_some(),
            "the nested contract table must accept expose like the top level"
        );
        assert_eq!(
            nested["items"]["minLength"].as_u64(),
            Some(1),
            "expose entries are non-empty strings"
        );
    }

    #[test]
    fn reference_blesses_expose() {
        assert!(
            HELP.contains("expose = ["),
            "reference must present expose as an accepted key:\n{HELP}"
        );
        assert!(
            !HELP.contains("expose is reserved"),
            "reference must not flag expose as reserved:\n{HELP}"
        );
    }

    #[test]
    fn reference_prints_excluded_sources() {
        assert!(
            HELP.contains("excluded_sources"),
            "the annotated reference must print excluded_sources (issue-8 \
             decision-check MINOR-1):\n{HELP}"
        );
    }

    /// Schema-vs-validator agreement (issue-8 phase 1, BLOCKER-3): the emitted
    /// properties of every table equal the shared `SPEC_KEY_SHAPES` entry the
    /// runtime walker reads — one table, two consumers, drift is red here.
    #[test]
    fn schema_key_sets_match_shared_shape_table() {
        let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
        let properties = |v: &serde_json::Value| -> Vec<String> {
            v["properties"]
                .as_object()
                .map(|m| m.keys().cloned().collect())
                .unwrap_or_else(|| panic!("fragment must carry properties:\n{v}"))
        };
        let assert_shape = |fragment: &serde_json::Value, shape: &str| {
            let mut got = properties(fragment);
            got.sort();
            let mut want: Vec<String> =
                spec::shape_keys(shape).iter().map(|key| (*key).to_string()).collect();
            want.sort();
            assert_eq!(
                got, want,
                "schema properties for shape {shape} must equal SPEC_KEY_SHAPES"
            );
        };
        assert_shape(&schema, "top");
        let project = &schema["properties"]["project"];
        assert_shape(project, "project");
        assert_shape(
            &project["properties"]["excluded_sources"]["items"],
            "excluded_sources",
        );
        assert_shape(&schema["properties"]["constraint"]["items"], "constraint");
        assert_shape(&schema["properties"]["stereotype"]["items"], "stereotype");
        assert_shape(
            &schema["properties"]["stereotype"]["items"]["properties"]["match"],
            "match",
        );
        let module = &schema["properties"]["module"]["items"];
        assert_shape(module, "module");
        assert_shape(&module["properties"]["matches"], "match");
        assert_shape(&module["properties"]["contract"], "contract");
        assert_shape(&module["properties"]["allowed"], "allowed");
        assert_shape(&module["properties"]["submodules"]["items"], "module");
    }

    /// The schema-level echo of the runtime refusal: every object schema that
    /// declares `properties` also declares `additionalProperties: false` —
    /// except a `oneOf` branch, which narrows its parent object (draft-07
    /// evaluates `additionalProperties` against the object that declares the
    /// properties, so the parent carries it and the branch must not).
    #[test]
    fn schema_refuses_unknown_keys_where_runtime_does() {
        let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
        let mut misses: Vec<String> = Vec::new();
        fn walk(value: &serde_json::Value, path: &str, one_of_branch: bool, misses: &mut Vec<String>) {
            if let Some(object) = value.as_object() {
                if object.contains_key("properties")
                    && !one_of_branch
                    && object.get("additionalProperties") != Some(&serde_json::json!(false))
                {
                    misses.push(path.to_string());
                }
                for (key, child) in object {
                    walk(child, &format!("{path}.{key}"), key == "oneOf", misses);
                }
            }
            if let Some(items) = value.as_array() {
                for (index, child) in items.iter().enumerate() {
                    walk(child, &format!("{path}[{index}]"), one_of_branch, misses);
                }
            }
        }
        walk(&schema, "schema", false, &mut misses);
        assert!(
            misses.is_empty(),
            "every properties-bearing schema object must carry \
             additionalProperties: false (oneOf branches excepted, per the \
             law above); misses: {misses:?}"
        );
    }

    #[test]
    fn schema_excluded_sources_shape() {
        let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
        let entry = &schema["properties"]["project"]["properties"]["excluded_sources"]["items"];
        assert_eq!(entry["type"], serde_json::json!("object"));
        assert_eq!(entry["required"], serde_json::json!(["path", "reason"]));
        assert_eq!(entry["additionalProperties"], serde_json::json!(false));
        for key in ["path", "reason"] {
            assert_eq!(entry["properties"][key]["type"], serde_json::json!("string"), "{key}");
            assert_eq!(entry["properties"][key]["minLength"], serde_json::json!(1), "{key}");
        }
        // The dial is OPTIONAL (absent from `required`, so an absent mode
        // keeps the shipped abort default) with exactly the two values the
        // runtime value-domain validator accepts — one source, no drift.
        assert_eq!(
            entry["properties"]["mode"]["enum"],
            serde_json::json!(spec::EXCLUSION_MODE_VALUES),
            "the emitted mode enum must equal EXCLUSION_MODE_VALUES"
        );
        assert_eq!(
            entry["required"].as_array().map(Vec::len),
            Some(2),
            "mode must stay optional — required would drop the default",
        );
    }
}