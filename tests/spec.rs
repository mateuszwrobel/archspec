mod common;

use common::{stderr, stdout, Fixture};

fn schema_of(fixture: &Fixture) -> serde_json::Value {
    let output = fixture.run(&["spec", "--schema"]);
    assert_eq!(output.status.code(), Some(0), "spec --schema must exit 0");
    assert!(
        stderr(&output).is_empty(),
        "spec --schema must not write stderr"
    );
    serde_json::from_str(&stdout(&output)).expect("spec --schema stdout must be valid JSON")
}

#[test]
fn schema_exits_zero_with_valid_json_on_stdout() {
    let fixture = Fixture::new();
    let output = fixture.run(&["spec", "--schema"]);

    assert_eq!(output.status.code(), Some(0));
    assert!(
        stderr(&output).is_empty(),
        "stderr must be empty:\n{}",
        stderr(&output)
    );
    let json: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("stdout must be valid JSON");
    assert!(json.is_object());
}

#[test]
fn schema_has_root_metadata_and_top_level_sections() {
    let schema = schema_of(&Fixture::new());

    assert_eq!(
        schema["$schema"],
        serde_json::json!("http://json-schema.org/draft-07/schema#")
    );
    assert!(schema["title"].is_string(), "must have a title");
    assert_eq!(schema["type"], serde_json::json!("object"));
    for section in ["project", "module", "constraint", "stereotype"] {
        assert!(
            schema["properties"].get(section).is_some(),
            "properties must contain {section}"
        );
    }
}

#[test]
fn schema_project_language_is_required_and_enumerated() {
    let project = schema_of(&Fixture::new())["properties"]["project"].clone();

    assert_eq!(project["required"], serde_json::json!(["language"]));
    assert_eq!(
        project["properties"]["language"]["enum"],
        serde_json::json!(["rust", "csharp", "go"])
    );
}

#[test]
fn schema_module_shape() {
    let module = schema_of(&Fixture::new())["properties"]["module"]["items"].clone();

    assert_eq!(module["required"], serde_json::json!(["name"]));
    for key in ["units", "modules"] {
        assert!(
            module["properties"]["matches"]["properties"]
                .get(key)
                .is_some(),
            "matches must accept {key}"
        );
    }
    let contract = &module["properties"]["contract"];
    assert!(
        contract["properties"].get("forbid").is_some(),
        "contract must accept the enforced forbid"
    );
    assert!(
        contract["properties"].get("expose").is_none(),
        "contract must not advertise the unenforced expose key"
    );
    assert_eq!(
        contract["not"],
        serde_json::json!({ "required": ["expose"] }),
        "schema must reject presence of expose (empty list included)"
    );
    for key in ["depend_on", "forbidden"] {
        assert!(
            module["properties"]["allowed"]["properties"]
                .get(key)
                .is_some(),
            "allowed must accept {key}"
        );
    }
    let submodule_items = module["properties"]["submodules"]["items"].clone();
    assert_eq!(submodule_items["type"], serde_json::json!("object"));
    assert_eq!(
        submodule_items["required"],
        serde_json::json!(["name"]),
        "submodules must recurse to the same module shape"
    );
}

const CONSTRAINT_TYPES: [&str; 7] = [
    "no_cycles",
    "manifest_integrity",
    "public_api_allowlist",
    "forbid_external_crates",
    "forbid_submodule_dependency",
    "feature_boundary",
    "external_free",
];

#[test]
fn schema_constraint_types_severity_and_modules() {
    let items = schema_of(&Fixture::new())["properties"]["constraint"]["items"].clone();

    assert_eq!(
        items["properties"]["type"]["enum"],
        serde_json::json!(CONSTRAINT_TYPES)
    );
    assert_eq!(
        items["properties"]["severity"]["enum"],
        serde_json::json!(["error", "warning"])
    );
    assert_eq!(
        items["properties"]["modules"]["type"],
        serde_json::json!("array")
    );
}

#[test]
fn schema_per_type_required_arrays_match_shared_truth() {
    let items = schema_of(&Fixture::new())["properties"]["constraint"]["items"].clone();

    let expected: &[(&str, &[&str])] = &[
        ("no_cycles", &[]),
        ("manifest_integrity", &[]),
        ("public_api_allowlist", &["allowed"]),
        ("forbid_external_crates", &["from", "forbid"]),
        ("forbid_submodule_dependency", &["parent", "from", "forbid"]),
        ("feature_boundary", &["feature", "gated_modules"]),
        ("external_free", &["from"]),
    ];
    let one_of = items["oneOf"].as_array().unwrap();
    for (kind, required) in expected {
        let entry = one_of
            .iter()
            .find(|e| e["properties"]["type"]["const"] == serde_json::json!(kind))
            .unwrap_or_else(|| panic!("schema must contain a branch for {kind}"));
        assert_eq!(
            entry["required"],
            serde_json::json!(required),
            "per-type required array for {kind}"
        );
    }
}

#[test]
fn schema_output_is_deterministic() {
    let a = Fixture::new();
    let b = Fixture::new();
    assert_eq!(
        stdout(&a.run(&["spec", "--schema"])),
        stdout(&b.run(&["spec", "--schema"]))
    );
}

#[test]
fn spec_prints_annotated_reference_toml() {
    let fixture = Fixture::new();
    let output = fixture.run(&["spec"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        !out.trim_start().starts_with('{'),
        "must not be JSON:\n{out}"
    );
    for marker in [
        "[project]",
        "[[module]]",
        "[[constraint]]",
        "[[stereotype]]",
        "language = \"rust\"",
    ] {
        assert!(
            out.contains(marker),
            "reference must contain {marker}:\n{out}"
        );
    }
    assert!(
        stderr(&output).is_empty(),
        "stderr must be empty:\n{}",
        stderr(&output)
    );
}

#[test]
fn unknown_flag_errors_naming_it() {
    let fixture = Fixture::new();
    let output = fixture.run(&["spec", "--bogus"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(
        stderr(&output).contains("unknown flag: --bogus"),
        "must name the unknown flag:\n{}",
        stderr(&output)
    );
}

#[test]
fn help_shares_reference_material() {
    let fixture = Fixture::new();
    let output = fixture.run(&["spec", "--help"]);

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(
        out.contains("archspec spec"),
        "must show the usage line:\n{out}"
    );
    assert!(
        out.contains("--schema"),
        "help must document --schema:\n{out}"
    );
    for marker in [
        "[project]",
        "[[module]]",
        "[[constraint]]",
        "[[stereotype]]",
    ] {
        assert!(
            out.contains(marker),
            "help must carry the reference material ({marker}):\n{out}"
        );
    }
    assert!(
        stderr(&output).is_empty(),
        "stderr must be empty:\n{}",
        stderr(&output)
    );
}
