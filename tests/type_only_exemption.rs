//! Step-1 cells of the ts-driver design record D3 — the per-entry `type_only`
//! dependency law is LIVE: the cycle graph counts every edge regardless of
//! kind, dependency adjudication counts type-only edges like any other by
//! default (`count`), and an entry declaring `exempt` admits the pair's
//! type-only crossing while the value crossing still reddens (D3, R2-2,
//! R3-9). The exemption lives on the rule row, never on the language
//! (ADR-022 §8, ADR-023); on rust, go and csharp models — where the fact is
//! absent — every crossing is a value crossing, so an exempt entry reddens
//! them exactly as the value-import row of the scenario table states.

mod common;

use common::Fixture;
use serde_json::Value;

/// The two-claim ts tree: `views` carries the governing `[module.allowed]`
/// table, `store` is the imported claim. `entry` is the entry text of one
/// list — the list itself is picked by the caller.
fn ts_pair_tree(entry: &str, list: &str, views_source: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"views\"\nmatches = {{ units = [\"demo-ui\"], modules = [\"src/views/**\"] }}\n\n[module.allowed]\n{list} = [{entry}]\n\n[[module]]\nname = \"store\"\nmatches = {{ modules = [\"src/store/**\"] }}\n"
        ),
    );
    tree.write("src/views/v.ts", views_source);
    tree.write(
        "src/store/s.ts",
        "export interface Store { n: number }\nexport const createStore = (): Store => ({ n: 0 });\n",
    );
    tree
}

const TYPE_ONLY_IMPORT: &str =
    "import type { Store } from \"../store/s\";\nexport const v: Store = { n: 1 };\n";
const VALUE_IMPORT: &str =
    "import { createStore } from \"../store/s\";\nexport const v = createStore();\n";

fn verdict(tree: &Fixture) -> (Option<i32>, String) {
    let out = tree.run(&["verify", "."]);
    (out.status.code(), common::stdout(&out) + &common::stderr(&out))
}

/// Scenario table row "`type_only = exempt`, type-only import → OK": the
/// exempt allowed entry admits the type-only crossing — no finding, exit 0.
#[test]
fn an_exempt_allowed_entry_admits_the_type_only_crossing() {
    let (code, out) = verdict(&ts_pair_tree(
        "{ target = \"store\", type_only = \"exempt\" }",
        "depend_on",
        TYPE_ONLY_IMPORT,
    ));
    assert_eq!(code, Some(0), "the exempt entry admits the type row:\n{out}");
    assert!(
        !out.contains("disallowed cross-component dependency")
            && !out.contains("forbidden edge")
            && !out.contains("missing edge"),
        "the admitted crossing states no finding:\n{out}"
    );
}

/// Scenario table row "`type_only = exempt`, value import → red": the same
/// entry still reddens the value crossing, naming the pair.
#[test]
fn an_exempt_allowed_entry_still_reds_the_value_crossing() {
    let (code, out) = verdict(&ts_pair_tree(
        "{ target = \"store\", type_only = \"exempt\" }",
        "depend_on",
        VALUE_IMPORT,
    ));
    assert_eq!(code, Some(1), "the value crossing reddens:\n{out}");
    assert!(
        out.contains("disallowed cross-component dependency: views -> store"),
        "the finding names the value crossing:\n{out}"
    );
}

/// D3 "the default preserves the shipped behavior": a `count` entry — or its
/// plain-string spelling — adjudicates both kinds exactly as shipped, so the
/// type-only crossing of an allowed target stays green byte-identically.
#[test]
fn a_count_entry_counts_the_type_only_crossing_like_any_other() {
    let (code_type, out_type) = verdict(&ts_pair_tree(
        "{ target = \"store\", type_only = \"count\" }",
        "depend_on",
        TYPE_ONLY_IMPORT,
    ));
    assert_eq!(code_type, Some(0), "count counts the type row as an edge:\n{out_type}");
    let (code_plain, out_plain) =
        verdict(&ts_pair_tree("\"store\"", "depend_on", TYPE_ONLY_IMPORT));
    assert_eq!(code_plain, code_type);
    assert_eq!(
        out_plain, out_type,
        "the plain-string entry verdicts byte-identically to the explicit count entry"
    );
    let (code_value, out_value) = verdict(&ts_pair_tree(
        "{ target = \"store\", type_only = \"count\" }",
        "depend_on",
        VALUE_IMPORT,
    ));
    assert_eq!(code_value, Some(0), "count keeps the value crossing green:\n{out_value}");
}

/// D3/R2-2 with the retired guard's oracle shape: a FORBIDDEN entry at the
/// default `count` reddens both variants of the crossing — the value import
/// and the `import type` alike.
#[test]
fn a_forbidden_count_entry_reddens_both_kinds_of_crossing() {
    for source in [VALUE_IMPORT, TYPE_ONLY_IMPORT] {
        let (code, out) = verdict(&ts_pair_tree(
            "{ target = \"store\", type_only = \"count\" }",
            "forbidden",
            source,
        ));
        assert_eq!(code, Some(1), "the count prohibition binds every kind:\n{out}");
        assert!(
            out.contains("forbidden edge: views -> store"),
            "the finding names the forbidden crossing:\n{out}"
        );
    }
}

/// The exemption on a prohibition: the type-only crossing passes, the value
/// crossing stays forbidden — rule (iii)'s door, law not accident.
#[test]
fn a_forbidden_exempt_entry_admits_type_only_and_reds_value() {
    let (code_type, out_type) = verdict(&ts_pair_tree(
        "{ target = \"store\", type_only = \"exempt\" }",
        "forbidden",
        TYPE_ONLY_IMPORT,
    ));
    assert_eq!(code_type, Some(0), "the exempt prohibition admits the type row:\n{out_type}");
    let (code_value, out_value) = verdict(&ts_pair_tree(
        "{ target = \"store\", type_only = \"exempt\" }",
        "forbidden",
        VALUE_IMPORT,
    ));
    assert_eq!(code_value, Some(1), "the value crossing stays forbidden:\n{out_value}");
    assert!(
        out_value.contains("forbidden edge: views -> store"),
        "the prohibition names the value crossing:\n{out_value}"
    );
}

/// D3 fact-site + adjudication law on one tree: the mixed pair serializes
/// exactly two `(from, to, kind)` rows, and under the exempt entry the run
/// admits the type row while reddening the value row, naming the crossing.
#[test]
fn a_mixed_pair_serializes_two_rows_and_the_exempt_entry_reds_the_value_row() {
    let tree = ts_pair_tree(
        "{ target = \"store\", type_only = \"exempt\" }",
        "forbidden",
        "import { createStore } from \"../store/s\";\nimport type { Store } from \"../store/s\";\nexport const v: Store = createStore();\n",
    );
    let scan = tree.run(&["scan", "."]);
    assert!(scan.status.success(), "scan failed: {}", common::stderr(&scan));
    let model: Value =
        serde_json::from_str(&common::stdout(&scan)).expect("scan stdout is the model JSON");
    let mut rows: Vec<(String, String, bool)> = model["module_edges"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .map(|row| {
            (
                row["from"].as_str().unwrap_or_default().to_string(),
                row["to"].as_str().unwrap_or_default().to_string(),
                row["type_only"].as_bool().unwrap_or(false),
            )
        })
        .collect();
    rows.sort();
    assert_eq!(
        rows,
        vec![
            ("src/views/v.ts".to_string(), "src/store/s.ts".to_string(), false),
            ("src/views/v.ts".to_string(), "src/store/s.ts".to_string(), true),
        ],
        "the mixed pair serializes two rows in deterministic key order (D3, R3-9)"
    );

    let (code, out) = verdict(&tree);
    assert_eq!(code, Some(1), "the value row reddens under the exempt entry:\n{out}");
    assert!(
        out.contains("forbidden edge: views -> store"),
        "the finding names the value crossing:\n{out}"
    );
}

/// D3 cycle-tier law: no exemption exists at the cycle tier — a type-only
/// cycle between two exempted claims is still a cycle, red at the claim tier.
#[test]
fn a_type_only_cycle_stays_red_under_exempt_entries() {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"views\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/views/**\"] }\n\n[module.allowed]\ndepend_on = [{ target = \"store\", type_only = \"exempt\" }]\n\n[[module]]\nname = \"store\"\nmatches = { modules = [\"src/store/**\"] }\n\n[module.allowed]\ndepend_on = [{ target = \"views\", type_only = \"exempt\" }]\n\n[[constraint]]\ntype = \"no_cycles\"\nscope = \"both\"\n",
    );
    tree.write(
        "src/views/v.ts",
        "import type { S } from \"../store/s\";\nexport const v: S = { n: 1 };\n",
    );
    tree.write(
        "src/store/s.ts",
        "import type { V } from \"../views/v\";\nexport type S = V;\n",
    );
    let (code, out) = verdict(&tree);
    assert_eq!(code, Some(1), "the type-only cycle counts at the cycle tier:\n{out}");
    assert!(out.contains("cycle"), "the cycle finding is stated:\n{out}");
}
