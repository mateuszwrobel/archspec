//! Scenario 02 — canonical-producer declaration face and shared-constant
//! class (workplan `events_value_join`, ADR-069 clause 4.4a(iv),
//! decision-check F-W3/F-W5). The done-when cells:
//!
//! - (cell A) a `PROTOCOL_VERSION`-shape shared-constant declaration — one
//!   exported constant, several readers, no edge to name — draws ZERO async
//!   rows and exactly ONE canonical-producer census row (the F-W3 proving
//!   test; the corpus-battery leg rides this same declaring fixture, whose
//!   records land only in the suite's temp dir — no witness tree is written
//!   and none needs to be: the declaration face is a spec-schema/verify
//!   surface, read-only against any tree it adjudicates);
//! - (cell B) a constant-class declaration naming an emit/subscribe
//!   pattern-pair key is refused by name, in its own message class, never
//!   folded into the unknown-key echo — and the frozen `[[async_depends]]`
//!   four-key refusal stays byte-identical (the frozen row schema stands);
//! - (cell C) a declaration carrying the bare per-driver unit name `"."`
//!   where the tree root is no unit is refused by name, and the
//!   tree-root-canonical form (the owning unit's canonicalized directory)
//!   is accepted; a spelling that resolves through another vocabulary is
//!   refused as not canonical; one class with two producers is refused as
//!   a conflict, while two spellings of ONE producer are one producer;
//! - (cell D) the generated JSON schema and the validator agree on the new
//!   tables from the ONE refused-key constant: the schema carries exactly
//!   the shape-table keys, never expresses the pair on either table, and
//!   the validator accepts and refuses exactly what the schema states
//!   (the cannot-drift cell — mutating one side reddens the other, welded
//!   in `src/archspec/commands/spec.rs` beside the generator).
//!
//! The census law is the report-only half of the face: one
//! `canonical producer:` row per declared entry, the shared-constant class
//! riding the same row with a ` — shared constant` marker — a
//! shared-constant declaration IS a canonical-producer declaration over a
//! constant fact class (ADR-069 4.4a(iv)) and never an async row;
//! `--strict` never promotes the row, and a non-declaring run renders no
//! census bytes at all (D8).

mod common;

use common::{Fixture, stderr, stdout};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn link(from: &std::path::Path, to_dir: &std::path::Path) {
    std::fs::create_dir_all(from.parent().expect("link parent")).expect("link parent dir");
    std::os::unix::fs::symlink(to_dir, from).expect("symlink");
}

/// The `PROTOCOL_VERSION` witness shape (the survey's row-D class, spelled
/// generically): a linked ts workspace whose root is a unit, one constant
/// package exporting an integer constant, and TWO reader packages importing
/// it — a shared constant with several readers and no edge to name. The
/// spec body is appended verbatim after the module claims, which keep the
/// verdict bytes clean (every unit claimed, every derived edge allowed).
fn protocol_workspace(spec_body: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-monorepo\",\n  \"version\": \"0.1.0\",\n  \"workspaces\": [\"packages/*\", \"apps/*\"]\n}\n",
    );
    tree.write(
        "packages/gateway-protocol/package.json",
        "{\n  \"name\": \"@demo/gateway-protocol\",\n  \"version\": \"0.1.0\",\n  \"main\": \"src/index.ts\"\n}\n",
    );
    tree.write(
        "packages/gateway-protocol/src/index.ts",
        "export const PROTOCOL_VERSION = 4 as const;\n",
    );
    for app in ["api", "worker"] {
        let manifest = "{\n  \"name\": \"@demo/app-placeholder\",\n  \"version\": \"0.1.0\",\n  \"dependencies\": { \"@demo/gateway-protocol\": \"workspace:*\" }\n}\n"
            .replace("app-placeholder", app);
        tree.write(&format!("apps/{app}/package.json"), &manifest);
        tree.write(
            &format!("apps/{app}/src/index.ts"),
            "import { PROTOCOL_VERSION } from \"@demo/gateway-protocol\";\nexport const version = PROTOCOL_VERSION;\n",
        );
    }
    link(
        &tree.path("node_modules/@demo/gateway-protocol"),
        &tree.path("packages/gateway-protocol"),
    );
    link(&tree.path("node_modules/@demo/api"), &tree.path("apps/api"));
    link(
        &tree.path("node_modules/@demo/worker"),
        &tree.path("apps/worker"),
    );
    tree.write(
        "architecture.spec.toml",
        &protocol_workspace_spec(spec_body),
    );
    tree
}

fn protocol_workspace_spec(spec_body: &str) -> String {
    format!(
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"protocol\"\nmatches = {{ units = [\"@demo/gateway-protocol\"], modules = [\"packages/gateway-protocol/src/index.ts\"] }}\n\n[[module]]\nname = \"api\"\nmatches = {{ units = [\"@demo/api\"], modules = [\"apps/api/src/index.ts\"] }}\n\n[module.allowed]\ndepend_on = [\"protocol\"]\n\n[[module]]\nname = \"worker\"\nmatches = {{ units = [\"@demo/worker\"], modules = [\"apps/worker/src/index.ts\"] }}\n\n[module.allowed]\ndepend_on = [\"protocol\"]\n\n[[module]]\nname = \"root\"\nmatches = {{ units = [\"demo-monorepo\"] }}\n{spec_body}"
    )
}

/// A rust workspace whose ROOT states no unit — the members register, the
/// root directory owns nothing: the witness shape for a bare per-driver
/// `"."` reaching a declaration that has no producer directory to anchor
/// it (a declaration is spec prose, not a row).
fn member_only_workspace(spec_body: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"packages/gateway-protocol\"]\n",
    );
    tree.write(
        "packages/gateway-protocol/Cargo.toml",
        "[package]\nname = \"gateway_protocol\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "packages/gateway-protocol/src/lib.rs",
        "pub const PROTOCOL_VERSION: u32 = 4;\n",
    );
    tree.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"protocol\"\nmatches = {{ units = [\"gateway_protocol\"] }}\n{spec_body}"
        ),
    );
    tree
}

fn find<'a>(json: &'a serde_json::Value, category: &str) -> Option<&'a serde_json::Value> {
    json["findings"]
        .as_array()?
        .iter()
        .find(|finding| finding["category"].as_str() == Some(category))
}

// ---------------------------------------------------------------------------
// cell A — the PROTOCOL_VERSION witness: ZERO async rows, ONE census row
// ---------------------------------------------------------------------------

/// The F-W3 proving test: the shared-constant declaration of the
/// PROTOCOL_VERSION shape adjudicates green, draws exactly ONE
/// canonical-producer census row (with the shared-constant marker) and
/// ZERO async rows on every machine and rendered surface — the declaration
/// face emits no async fact and the async render machinery never reads
/// these tables. `--strict` keeps the run green: the census row is
/// report-only, never a verdict.
#[test]
fn shared_constant_declaration_draws_one_census_row_and_zero_async_rows() {
    let tree = protocol_workspace(
        "\n[[shared_constant]]\nclass = \"protocol-version\"\nproducer = \"packages/gateway-protocol\"\n",
    );
    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "a declaring run with a canonical shared-constant producer verifies green: {}",
        stdout(&verify)
    );
    let strict = tree.run(&["verify", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(0),
        "the declaration census is report-only: `--strict` never promotes it: {}",
        stdout(&strict)
    );

    // The machine body: exactly one census row, constant-flagged, and no
    // async machinery anywhere in the findings (the zero-async-row law).
    let output = tree.run(&["verify", "--json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let json: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("verify --json must parse");
    let rows = json["declaration_census"]["producers"]
        .as_array()
        .expect("the declaring run carries the declaration_census member");
    assert_eq!(
        rows.len(),
        1,
        "one declared entry draws exactly one census row: {rows:?}"
    );
    assert_eq!(rows[0]["class"], "protocol-version");
    assert_eq!(rows[0]["producer"], "packages/gateway-protocol");
    assert_eq!(
        rows[0]["constant"], true,
        "a shared-constant entry is census-flagged as a constant"
    );
    assert!(
        json.get("async_census").is_none(),
        "the declaration face never earns an async census: {}",
        stdout(&output)
    );
    for category in [
        "async event unresolved",
        "async publish not derived",
        "async subscribe not derived",
        "async endpoint unclaimed",
        "async cycle",
        "async event collision",
    ] {
        assert!(
            find(&json, category).is_none(),
            "a shared-constant declaration must draw no async finding {category:?}: {}",
            stdout(&output)
        );
    }

    // The rendered census: ONE canonical-producer row with the constant
    // marker, no async section (this declaring fixture is the scenario's
    // corpus-battery leg — its record lands in the suite temp dir only).
    let report = tree.run(&["report"]);
    assert_eq!(report.status.code(), Some(0), "{}", stderr(&report));
    let out = stdout(&report);
    let row_lines: Vec<&str> = out
        .lines()
        .filter(|line| line.starts_with("canonical producer:"))
        .collect();
    assert_eq!(
        row_lines,
        ["canonical producer: protocol-version -> packages/gateway-protocol — shared constant"],
        "the shared-constant class rides the ONE canonical-producer row form: {out}"
    );
    assert!(!out.contains("Async census"), "no async section: {out}");
    assert!(
        out.contains("Declaration census"),
        "census section renders: {out}"
    );
}

/// A run with neither declaration table renders NO census bytes at all and
/// carries no `declaration_census` member (D8 — absence is the contract;
/// the byte-identity of the full non-declaring scan is pinned by scenario
/// 01's digest cell).
#[test]
fn non_declaring_run_renders_no_census_bytes() {
    let tree = protocol_workspace("");
    let report = tree.run(&["report"]);
    assert_eq!(report.status.code(), Some(0), "{}", stderr(&report));
    let out = stdout(&report);
    assert!(
        !out.contains("Declaration census") && !out.contains("canonical producer:"),
        "a non-declaring run states no declaration census: {out}"
    );
    let output = tree.run(&["verify", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("verify --json must parse");
    assert!(
        json.get("declaration_census").is_none(),
        "the non-declaring json omits the member: {}",
        stdout(&output)
    );
}

// ---------------------------------------------------------------------------
// cell B — a constant-class pattern pair is refused by name; the frozen
// [[async_depends]] four-key refusal stays byte-identical
// ---------------------------------------------------------------------------

#[test]
fn constant_class_naming_a_pattern_pair_is_refused_in_its_own_class() {
    for key in ["emit", "subscribe"] {
        let tree = protocol_workspace(&format!(
            "\n[[shared_constant]]\nclass = \"protocol-version\"\nproducer = \"packages/gateway-protocol\"\n{key} = \"publish\"\n"
        ));
        let output = tree.run(&["verify", "."]);
        assert_ne!(
            output.status.code(),
            Some(0),
            "a shared-constant entry naming {key:?} must not load: {}",
            stdout(&output)
        );
        let err = stderr(&output);
        assert!(
            err.contains("invalid spec:")
                && err.contains(&format!(
                    "[shared_constant] entry #1: key \"{key}\" — a shared constant is never a pattern pair"
                )),
            "the pattern-pair refusal must name the table, the entry, the key and the law: {err}"
        );
        assert!(
            !err.contains("unknown key(s)"),
            "the pair reaches its OWN voice, never the unknown-key echo: {err}"
        );
    }
    // The canonical-producer table takes the pair through the SHARED
    // unknown-key law — the pair is not a member of the face's shape.
    let tree = protocol_workspace(
        "\n[[canonical_producer]]\nclass = \"orders\"\nproducer = \"apps/api\"\nemit = \"publish\"\n",
    );
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0), "{}", stdout(&output));
    let err = stderr(&output);
    assert!(
        err.contains("unknown key(s) \"emit\"")
            && err.contains("in [canonical_producer] #1")
            && err.contains("supported keys are \"class\", \"producer\""),
        "the unknown-key refusal names the key, the entry and the shape: {err}"
    );
}

/// The frozen-row-schema weld: the `[[async_depends]]` four-key refusal —
/// byte-for-byte the shipped class (the same assertion `tests/
/// async_depends_spec.rs` pins against the csharp fixture; here the load
/// refusal proves the same bytes on a rust tree) — stands unchanged by the
/// declaration face.
#[test]
fn async_depends_four_key_refusal_stays_byte_identical() {
    let tree = member_only_workspace(
        "\n[[async_depends]]\nfrom = \"gateway_protocol\"\nto = \"gateway_protocol\"\nevent = \"gateway_protocol::Ping\"\npayload = \"int\"\n",
    );
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0), "{}", stdout(&output));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:")
            && err.contains("unknown key(s) \"payload\"")
            && err.contains("in [async_depends] #1")
            && err.contains("supported keys are \"event\", \"from\", \"to\""),
        "the frozen async row schema must refuse a fourth key exactly as before: {err}"
    );
}

/// The mandatory-key law of the declaration shape: an entry without either
/// key — absent or empty — is refused at load, naming the key and the
/// entry, before deserialization could fold the gap into a silent row.
#[test]
fn declaration_entry_without_a_mandatory_key_is_refused_at_load() {
    let tree = member_only_workspace("\n[[canonical_producer]]\nclass = \"orders\"\n");
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(
        stderr(&output)
            .contains("[canonical_producer] entry #1: missing mandatory key \"producer\""),
        "the missing-key refusal names the table, entry and key: {}",
        stderr(&output)
    );
    let tree = member_only_workspace(
        "\n[[shared_constant]]\nclass = \"\"\nproducer = \"packages/gateway-protocol\"\n",
    );
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("[shared_constant] entry #1: missing mandatory key \"class\""),
        "an empty class is the same refusal class: {}",
        stderr(&output)
    );
}

// ---------------------------------------------------------------------------
// cell C — bare per-driver name refused, canonical form accepted
// ---------------------------------------------------------------------------

/// The bare per-driver unit name `"."` in a declaration: a declaration is
/// spec prose, not a row, so it carries no producer directory for the
/// dot's ownership arm — where the tree root owns no unit the spelling
/// answers nothing and is refused by name; the tree-root-canonical form
/// (the owning unit's canonicalized directory) is accepted.
#[test]
fn bare_per_driver_name_is_refused_and_canonical_form_accepted() {
    let tree = member_only_workspace(
        "\n[[canonical_producer]]\nclass = \"protocol-version\"\nproducer = \".\"\n",
    );
    let output = tree.run(&["verify", "."]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "a bare dot where the root owns no unit must not verify: {}",
        stdout(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains(
            "declaration producer unresolved: protocol-version — . ([canonical_producer] entry #1)"
        ),
        "the unresolved refusal names the class, the spelling, the table and the entry: {out}"
    );

    let tree = member_only_workspace(
        "\n[[canonical_producer]]\nclass = \"protocol-version\"\nproducer = \"packages/gateway-protocol\"\n",
    );
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the tree-root-canonical id is the accepted declaration form: {}",
        stdout(&output)
    );
}

/// A spelling that resolves at the tree-root unit index through another
/// vocabulary — here the crate name, an ecosystem spelling of the owning
/// unit — is not the canonical id the declaration law demands: the run
/// refuses it by name and states the id it resolves to.
#[test]
fn non_canonical_spelling_is_refused_naming_the_resolved_id() {
    let tree = member_only_workspace(
        "\n[[shared_constant]]\nclass = \"protocol-version\"\nproducer = \"gateway_protocol\"\n",
    );
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0), "{}", stdout(&output));
    let out = stdout(&output);
    assert!(
        out.contains(
            "declaration producer not canonical: protocol-version — gateway_protocol resolves to packages/gateway-protocol ([shared_constant] entry #1)"
        ),
        "the canonicality refusal names the spelling and the resolved id: {out}"
    );
}

/// One producer per class per table: one class declaring two producers that
/// resolve to DIFFERENT units states no arbitration and is refused by name
/// with every producer and its entry numbers; two spellings of the SAME
/// unit are ONE producer — the conflict arm groups by resolved identity,
/// never by the bytes the author wrote.
#[test]
fn one_class_two_producers_is_refused_and_two_spellings_are_one_producer() {
    let tree_two = member_only_workspace(
        "\n[[canonical_producer]]\nclass = \"orders\"\nproducer = \"services/a\"\n\n[[canonical_producer]]\nclass = \"orders\"\nproducer = \"services/b\"\n",
    );
    // Two units the conflict arm can distinguish: extend the workspace.
    tree_two.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"packages/gateway-protocol\", \"services/a\"]\n",
    );
    tree_two.write(
        "services/a/Cargo.toml",
        "[package]\nname = \"orders_a\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree_two.write("services/a/src/lib.rs", "pub fn handle() {}\n");
    let output = tree_two.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0), "{}", stdout(&output));
    let out = stdout(&output);
    assert!(
        out.contains(
            "declaration producer conflict: orders — services/a (entries 1), services/b (entries 2) ([canonical_producer])"
        ),
        "the conflict row names every producer and its entries, in identity order: {out}"
    );

    // Same class, two SPELLINGS of one unit: no conflict row — the entry
    // that is not canonical carries the canonicality refusal alone.
    let tree = member_only_workspace(
        "\n[[canonical_producer]]\nclass = \"protocol-version\"\nproducer = \"packages/gateway-protocol\"\n\n[[canonical_producer]]\nclass = \"protocol-version\"\nproducer = \"gateway_protocol\"\n",
    );
    let output = tree.run(&["verify", "."]);
    let out = stdout(&output);
    assert!(
        !out.contains("declaration producer conflict"),
        "two spellings of one unit are one producer, not a conflict: {out}"
    );
}

// ---------------------------------------------------------------------------
// cell D — generated schema and validator agree from the one constant
// ---------------------------------------------------------------------------

/// The cannot-drift cell, from the OUTSIDE: the generated JSON schema
/// carries exactly the shape-table keys for both declaration tables with
/// both mandatory, `additionalProperties: false` like every other table,
/// and NEVER expresses the pattern pair on either table; the validator
/// then accepts and refuses exactly what the schema states — a class +
/// producer entry loads green, an `emit` key dies at load — so the two
/// faces are driven by the one constant and a drift on either side reddens
/// this cell (the generator-side welds live in `commands/spec.rs`).
#[test]
fn schema_and_validator_agree_on_the_declaration_tables() {
    let tree = Fixture::new();
    let output = tree.run(&["spec", "--schema"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let schema: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("spec --schema must parse");
    for table in ["canonical_producer", "shared_constant"] {
        let items = &schema["properties"][table]["items"];
        let mut keys: Vec<&str> = items["properties"]
            .as_object()
            .expect("table item properties")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort();
        assert_eq!(
            keys,
            ["class", "producer"],
            "the schema face of {table} is the shape table verbatim"
        );
        let required: Vec<&str> = items["required"]
            .as_array()
            .expect("table item required list")
            .iter()
            .map(|value| value.as_str().expect("required key is a string"))
            .collect();
        assert!(
            required.contains(&"class") && required.contains(&"producer"),
            "both keys are mandatory on {table}: {required:?}"
        );
        assert_eq!(
            items["additionalProperties"], false,
            "{table} is fail-closed like every shipped table"
        );
        let rendered = items.to_string().to_lowercase();
        for pair_key in ["emit", "subscribe"] {
            assert!(
                !rendered.contains(&format!("\"{pair_key}\"")),
                "the schema must never make a constant expressible as the {pair_key:?} pattern key: {rendered}"
            );
        }
    }

    // The validator side of the agreement, same fixture pair: the schema's
    // accepted shape loads green (cell A's fixture adjudicates it), and
    // the schema's absent pair keys are the validator's load refusal.
    let accepted = member_only_workspace(
        "\n[[shared_constant]]\nclass = \"protocol-version\"\nproducer = \"packages/gateway-protocol\"\n",
    );
    assert_eq!(
        accepted.run(&["verify", "."]).status.code(),
        Some(0),
        "the validator accepts exactly the shape the schema states"
    );
    let refused = member_only_workspace(
        "\n[[shared_constant]]\nclass = \"protocol-version\"\nproducer = \"packages/gateway-protocol\"\nemit = \"publish\"\n",
    );
    let output = refused.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(
        stderr(&output).contains("a shared constant is never a pattern pair"),
        "the validator refuses what the schema cannot express: {}",
        stderr(&output)
    );
}

/// The census is deterministic spec arithmetic: rows come out in
/// table-then-class order no matter how the entries were written, a
/// canonical-producer row carries no constant marker, a duplicate
/// (class, producer) pair across the same table draws ONE row, and the
/// shared-constant table's rows follow the canonical-producer table's.
#[test]
fn census_rows_are_deterministic_and_declaration_order_never_matters() {
    let tree = protocol_workspace(
        "\n[[shared_constant]]\nclass = \"zeta\"\nproducer = \"apps/worker\"\n\n[[shared_constant]]\nclass = \"alpha\"\nproducer = \"apps/api\"\n\n[[shared_constant]]\nclass = \"alpha\"\nproducer = \"apps/api\"\n\n[[canonical_producer]]\nclass = \"orders\"\nproducer = \"apps/api\"\n",
    );
    let output = tree.run(&["verify", "--json"]);
    let json: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("verify --json must parse");
    let rendered: Vec<String> = json["declaration_census"]["producers"]
        .as_array()
        .expect("census member")
        .iter()
        .map(|row| {
            format!(
                "{} {} {}",
                row["class"].as_str().unwrap_or_default(),
                row["producer"].as_str().unwrap_or_default(),
                if row["constant"].as_bool().unwrap_or(false) {
                    "constant"
                } else {
                    "plain"
                }
            )
        })
        .collect();
    assert_eq!(
        rendered,
        [
            "orders apps/api plain",
            "alpha apps/api constant",
            "zeta apps/worker constant",
        ],
        "table-then-class order, deduplicated, regardless of declaration order"
    );
}
