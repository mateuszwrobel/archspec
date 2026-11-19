//! GitHub issue #48 (adversarial fix plan P2): the async arrow RENDER SET
//! keys on `(from, to, full event identity)`, never on the rendered label.
//! Two declared entries over one `(from, to)` pair whose events differ in
//! full NEVER share an arrow — the collapsed single-arrow class that hid a
//! second contract behind one label is gone. The label stays the shipped
//! short `from::SimpleName` form wherever one entry owns a simple name of
//! its pair (D8 byte identity), and becomes the SHORTEST DISTINGUISHING
//! SUFFIX — a dotted suffix or the full dotted name — wherever ≥2 entries
//! of the pair share it. The verify side owes one async-census VOICE for
//! the class: `async event collision: ...`, an advisory warning `--strict`
//! never promotes, naming every colliding entry.
//!
//! This file is the dedicated guard of the `async-event-collision-advisory`
//! claim: the signature `async event collision` appears verbatim below,
//! pinned through the rendered bytes of `verify` and the diagnostics
//! catalog.
//!
//! Cells: the exact units-tier arrow list (collision fixture, both targets,
//! NO bare collapsed label), the folded modules view with both arrows on
//! the gateway/handler nodes and no fold note, the verify advisory row
//! naming both identities `(entries #1, #2)` with exit 0 in BOTH modes and
//! byte-identical strict stdout, the spec-source diagram per-entry arrows,
//! the scan.json model-path collision cell, and the unique-simple-name
//! counter-cell pinning the shipped short labels and the silent census.

mod common;

use common::{Fixture, stderr, stdout};

/// The collision fixture (issue #48): unit `api` depends on unit `core`
/// through a manifest path dependency and declares TWO `[[async_depends]]`
/// entries over the SAME `api -> core` pair whose events share the simple
/// name `OrderPlaced` but differ in full — the hidden-collapse class the
/// pre-fix binary drew as ONE collapsed arrow labelled `api::OrderPlaced`.
/// `api` carries ONE module file (the modules view must fold it onto the
/// single `gateway` node), `core` onto `handler`.
fn collision_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"api\", \"core\"]\nresolver = \"2\"\n");
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
         [dependencies]\ncore = { path = \"../core\" }\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "pub mod gateway;\n\npub struct OrderPlaced;\n",
    );
    fixture.write(
        "api/src/gateway.rs",
        "use core::AppEvent;\n\npub fn publish(event: AppEvent) {\n    let _ = event;\n}\n",
    );
    fixture.write("core/Cargo.toml", "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("core/src/lib.rs", "pub mod handler;\n\npub struct AppEvent;\n");
    fixture.write("core/src/handler.rs", "pub fn consume() {}\n");
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "api"
matches = { units = ["api"] }
allowed = { depend_on = ["core"] }

[[module]]
name = "core"
matches = { units = ["core"] }

[[async_depends]]
from = "api"
to = "core"
event = "Api.Events.OrderPlaced"

[[async_depends]]
from = "api"
to = "core"
event = "Api.Messages.OrderPlaced"
"#,
    );
    fixture
}

/// The counter-fixture: the SAME pair declares two events with DISTINCT
/// simple names — no collision exists, so the shipped short-label bytes
/// and the silent census are the contract (issue #48 D8).
fn unique_names_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"api\", \"core\"]\nresolver = \"2\"\n");
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
         [dependencies]\ncore = { path = \"../core\" }\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "pub mod gateway;\n\npub struct OrderPlaced;\npub struct OrderShipped;\n",
    );
    fixture.write(
        "api/src/gateway.rs",
        "use core::AppEvent;\n\npub fn publish(event: AppEvent) {\n    let _ = event;\n}\n",
    );
    fixture.write("core/Cargo.toml", "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("core/src/lib.rs", "pub mod handler;\n\npub struct AppEvent;\n");
    fixture.write("core/src/handler.rs", "pub fn consume() {}\n");
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "api"
matches = { units = ["api"] }
allowed = { depend_on = ["core"] }

[[module]]
name = "core"
matches = { units = ["core"] }

[[async_depends]]
from = "api"
to = "core"
event = "Api.Events.OrderPlaced"

[[async_depends]]
from = "api"
to = "core"
event = "Api.Messages.OrderShipped"
"#,
    );
    fixture
}

/// The diagram fixture: a unit-tier declaring tree (no module file tier —
/// the `diagram` spec source draws unit nodes) whose table collides on one
/// pair exactly like [`collision_fixture`].
fn diagram_collision_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"api\", \"core\"]\nresolver = \"2\"\n");
    fixture.write("api/Cargo.toml", "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write(
        "api/src/lib.rs",
        "pub struct Publisher;\n\nimpl Publisher {\n    pub fn publish(&self) {}\n}\n",
    );
    fixture.write("core/Cargo.toml", "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("core/src/lib.rs", "pub struct Handler;\n\nimpl Handler {\n    pub fn handle(&self) {}\n}\n");
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "api"
matches = { units = ["api"] }

[[module]]
name = "core"
matches = { units = ["core"] }

[[async_depends]]
from = "api"
to = "core"
event = "Api.Events.OrderPlaced"

[[async_depends]]
from = "api"
to = "core"
event = "Api.Messages.OrderPlaced"
"#,
    );
    fixture
}

fn async_arrow_lines<'a>(text: &'a str, marker: &str) -> Vec<&'a str> {
    text.lines()
        .filter(|line| line.contains(marker))
        .map(str::trim)
        .collect()
}

/// The units tier renders ONE arrow per declared identity — two dashed
/// lines over the colliding pair, each carrying its shortest
/// distinguishing suffix — and the collapsed bare label `api::OrderPlaced`
/// appears nowhere. PlantUML states the same two rows in its own grammar.
#[test]
fn colliding_entries_render_one_arrow_each_and_never_the_collapsed_label() {
    let fixture = collision_fixture();
    let output = fixture.run(&["depgraph", "modules", "--aggregate", "units"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    let arrows = async_arrow_lines(&rendered, " -.->|");
    assert_eq!(
        arrows,
        [
            "api -.->|api::Events.OrderPlaced| core",
            "api -.->|api::Messages.OrderPlaced| core",
        ],
        "the render set keys on the full identity: two entries, two arrows, in (from, to, FQN) order:\n{rendered}"
    );
    assert!(
        !rendered.contains("|api::OrderPlaced|"),
        "the collapsed bare-simple label never renders:\n{rendered}"
    );
    assert!(
        !stderr(&output).contains("folded away"),
        "the units tier places every endpoint and prints no note:\n{}",
        stderr(&output)
    );

    let plantuml = fixture.run(&["depgraph", "modules", "--aggregate", "units", "--format", "plantuml"]);
    assert_eq!(plantuml.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&plantuml);
    let arrows = async_arrow_lines(&rendered, " ..> ");
    assert_eq!(
        arrows,
        [
            "api ..> core : api::Events.OrderPlaced",
            "api ..> core : api::Messages.OrderPlaced",
        ],
        "plantuml parity: the same two identity-keyed rows:\n{rendered}"
    );
}

/// The folded modules view keeps the law on both levels: the arrows land on
/// the view's OWN nodes (gateway/handler — never raw unit endpoints), each
/// with its distinguishing label, and a view that places every row prints
/// no fold note.
#[test]
fn the_folded_modules_view_renders_both_colliding_arrows_on_its_nodes() {
    let fixture = collision_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    assert!(
        rendered.contains("gateway -.->|api::Events.OrderPlaced| handler"),
        "the first identity draws its own arrow on the view's nodes:\n{rendered}"
    );
    assert!(
        rendered.contains("gateway -.->|api::Messages.OrderPlaced| handler"),
        "the second identity draws its own arrow — no collapse:\n{rendered}"
    );
    assert_eq!(
        rendered.matches(" -.->|").count(),
        2,
        "one arrow per declared identity — the fold draws no fan-out:\n{rendered}"
    );
    assert!(
        !stderr(&output).contains("folded away"),
        "every row placed: no fold note (issue #51 law):\n{}",
        stderr(&output)
    );
}

/// The collision voice: `verify` answers ONE async-census advisory naming
/// both full identities and both entry numbers, exits 0, and `--strict`
/// keeps the row advisory — exit 0 and a byte-identical stdout (scenario
/// 09 severity law: census DETECTION depth only, never promotion).
#[test]
fn verify_advises_the_collision_once_and_strict_never_promotes_it() {
    let fixture = collision_fixture();
    let lax = fixture.run(&["verify", "."]);
    assert_eq!(
        lax.status.code(),
        Some(0),
        "the collision is an advisory — verify exits 0:\n{}{}",
        stdout(&lax),
        stderr(&lax)
    );
    let rendered = stdout(&lax);
    assert!(
        rendered.contains(
            "warning: async event collision: api -> core declares 2 events named OrderPlaced \
             \u{2014} Api.Events.OrderPlaced, Api.Messages.OrderPlaced (entries #1, #2)"
        ),
        "the census row names both identities and both entry numbers:\n{rendered}"
    );
    assert_eq!(
        rendered.matches("async event collision").count(),
        1,
        "exactly ONE collision row for the pair:\n{rendered}"
    );
    assert!(
        rendered.contains("ok: architecture.spec.toml passes with 1 warning"),
        "the passing headline counts the advisory:\n{rendered}"
    );

    let strict = fixture.run(&["verify", "--strict", "."]);
    assert_eq!(
        strict.status.code(),
        Some(0),
        "--strict never promotes a census row (issue #48):\n{}{}",
        stdout(&strict),
        stderr(&strict)
    );
    assert_eq!(
        stdout(&strict),
        rendered,
        "strict stdout stays byte-identical while only census rows ride it"
    );
}

/// The `help diagnostics` catalog carries the row: the pattern line quotes
/// the rendered voice, and the entry states the identity-keyed law — the
/// canonical surface the claim welds to.
#[test]
fn the_diagnostics_catalog_names_the_collision_row() {
    let fixture = Fixture::new();
    let output = fixture.run(&["help", "diagnostics"]);
    assert_eq!(output.status.code(), Some(0), "help diagnostics must exit 0");
    let rendered = stdout(&output);
    assert!(
        rendered.contains("async event collision: <from> -> <to> declares"),
        "the catalog quotes the pattern:\n{rendered}"
    );
    assert!(
        rendered.contains("async event collision"),
        "the catalog names the row:\n{rendered}"
    );
}

/// The spec-source `diagram` shares the ONE implementation: the declaring
/// pair draws per-entry dashed arrows with distinguishing labels and never
/// the collapsed bare form.
#[test]
fn the_spec_source_diagram_draws_per_entry_arrows_for_the_colliding_pair() {
    let fixture = diagram_collision_fixture();
    let output = fixture.run(&["diagram"]);
    assert_eq!(output.status.code(), Some(0), "diagram must exit 0");
    let rendered = stdout(&output);
    assert_eq!(
        async_arrow_lines(&rendered, " -.->|").len(),
        2,
        "one arrow per declared identity in the spec-source diagram:\n{rendered}"
    );
    assert!(
        rendered.contains("api -.->|api::Events.OrderPlaced| core")
            && rendered.contains("api -.->|api::Messages.OrderPlaced| core"),
        "both identities draw their own labelled arrow:\n{rendered}"
    );
    assert!(
        !rendered.contains("|api::OrderPlaced|"),
        "the collapsed bare-simple label never renders:\n{rendered}"
    );
}

/// The scan-source model path (the `ModuleGraph::async_edges` feed): a
/// scan.json whose derived rows collide on one pair renders BOTH arrows
/// with their distinguishing labels — the fold projection shares the ONE
/// implementation, so the model path behaves exactly like the spec path.
#[test]
fn the_scan_artefact_model_path_renders_both_colliding_arrows() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"api\", \"core\"]\nresolver = \"2\"\n");
    for unit in ["api", "core"] {
        fixture.write(
            &format!("{unit}/Cargo.toml"),
            &format!("[package]\nname = \"{unit}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        );
        fixture.write(&format!("{unit}/src/lib.rs"), "pub struct Marker;\n");
    }
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"api\"\nmatches = { units = [\"api\"] }\n\n[[module]]\nname = \"core\"\nmatches = { units = [\"core\"] }\n",
    );
    let scan = fixture.run(&["scan", "--output", "scan.json"]);
    assert_eq!(scan.status.code(), Some(0), "scan must exit 0");
    let raw = fixture.read("scan.json");
    let mut model: serde_json::Value = serde_json::from_str(&raw).expect("scan.json parses");
    model["async_edges"] = serde_json::json!([
        { "from": "api", "to": "core", "event": "Api.Events.OrderPlaced", "kind": "async" },
        { "from": "api", "to": "core", "event": "Api.Messages.OrderPlaced", "kind": "async" },
    ]);
    fixture.write("scan.json", &serde_json::to_string(&model).expect("model serializes"));

    let output = fixture.run(&["diagram", "--source", "scan", "scan.json"]);
    assert_eq!(output.status.code(), Some(0), "diagram must exit 0");
    let rendered = stdout(&output);
    assert!(
        rendered.contains("api -.->|api::Events.OrderPlaced| core")
            && rendered.contains("api -.->|api::Messages.OrderPlaced| core"),
        "the derived rows render as two identity-keyed arrows inside the cluster:\n{rendered}"
    );
    assert!(
        !rendered.contains("|api::OrderPlaced|"),
        "the collapsed bare-simple label never renders on the model path either:\n{rendered}"
    );
}

/// The D8 counter-cell: distinct simple names over one pair keep the exact
/// shipped short-label bytes and speak NO collision row — the advisory
/// fires only on the class it names.
#[test]
fn unique_simple_names_keep_the_shipped_short_labels_and_speak_no_collision() {
    let fixture = unique_names_fixture();
    let output = fixture.run(&["depgraph", "modules", "--aggregate", "units"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    assert_eq!(
        async_arrow_lines(&rendered, " -.->|"),
        [
            "api -.->|api::OrderPlaced| core",
            "api -.->|api::OrderShipped| core",
        ],
        "no collision, no label change: the shipped short bytes stand:\n{rendered}"
    );

    let verify = fixture.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "verify passes:\n{}{}",
        stdout(&verify),
        stderr(&verify)
    );
    let rendered = stdout(&verify);
    assert!(
        !rendered.contains("async event collision"),
        "the census stays silent when every simple name of the pair is unique:\n{rendered}"
    );
}
