//! Scenario 05 (archspec_async_depends): rendering and census output. The
//! async rows render as dashed, event-labelled arrows in the depgraph and
//! diagram renders — spec source AND scan source, mermaid and PlantUML —
//! and the renderers draw the ONE async census section in text, markdown
//! and json only when the declared table resolves at least one pair. A run
//! with no async rows renders no async bytes: every non-declaring render
//! here is byte-pinned (D8, follow-up 6; law: N2 — the renderers consume
//! the census rows, never a re-derivation).
//!
//! Scenario 08 (consumer-fix round 1, eShop verdict on 0.6.86) moves the
//! ARROW law: exactly ONE arrow line per declared entry — the projected
//! module-node-pair fan-out is gone — with the declared UNIT names as
//! endpoints and the short `unit::SimpleName` label (never the declared
//! FQN). The census cells keep scenario 05's bytes; only the arrow law and
//! the label form moved.
//!
//! Cells: the dashed + event-labelled byte cells (both verbs, both sources),
//! the PlantUML parity cells, the multi-module-endpoint one-arrow cell, the
//! no-async byte pins, the report-section present/absent cells across the
//! three renderers, the scan.json derived-row cells (the artifact side of
//! D10's split), and the pair-census numbers of `verify --json` and
//! `report --format json`.

mod common;

use common::{Fixture, stdout};

/// The round-trip declaring fixture: units `orders` and `catalog`, no
/// manifest dependency between them, one `[[async_depends]]` leg each way —
/// the same shape the scenario-03 census owns. The census renders
/// `(catalog -> orders — catalog::EventTwo)` and
/// `(orders -> catalog — orders::EventOne)`, so every fan number below is 1.
fn round_trip_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"orders\", \"catalog\"]\nresolver = \"2\"\n");
    fixture.write("orders/Cargo.toml", "[package]\nname = \"orders\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write(
        "orders/src/lib.rs",
        "pub struct EventOne;\n\npub fn publish() -> EventOne { EventOne }\n",
    );
    fixture.write("catalog/Cargo.toml", "[package]\nname = \"catalog\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write(
        "catalog/src/lib.rs",
        "pub struct EventTwo;\n\npub fn handle(event: EventTwo) {\n    let _ = event;\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "orders"
matches = { units = ["orders"] }

[[module]]
name = "catalog"
matches = { units = ["catalog"] }

[[async_depends]]
from = "orders"
to = "catalog"
event = "orders::EventOne"

[[async_depends]]
from = "catalog"
to = "orders"
event = "catalog::EventTwo"
"#,
    );
    fixture
}

/// The mixed fixture: unit `api` depends on unit `core` through a manifest
/// path dependency AND the table declares both event legs, so the direct
/// arrow and the async arrows share one graph. Both crates carry a module
/// tier (the depgraph `modules` view needs one — the refusal law), so the
/// graph projects to the `gateway`/`handler` nodes.
fn mixed_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"api\", \"core\"]\nresolver = \"2\"\n");
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
         [dependencies]\ncore = { path = \"../core\" }\n",
    );
    fixture.write("api/src/lib.rs", "pub mod gateway;\n");
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
from = "core"
to = "api"
event = "core::AppEvent"

[[async_depends]]
from = "api"
to = "core"
event = "api::OrderPlaced"
"#,
    );
    fixture
}

/// The multi-module-endpoint fixture (scenario 08): unit `api` carries TWO
/// modules (`gateway` and `admin`), so its paths fold to TWO rendered module
/// nodes — the eShop shape that made 18 entries render 26 arrow lines on
/// 0.6.86. Unit `core` folds to the one `handler` node. Three entries are
/// declared, two of them the same `api -> core` pair over different events,
/// one event given as a dotted FQN — under the one-arrow law this renders
/// EXACTLY THREE lines with unit endpoints and short labels.
fn multi_module_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"api\", \"core\"]\nresolver = \"2\"\n");
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
         [dependencies]\ncore = { path = \"../core\" }\n",
    );
    fixture.write("api/src/lib.rs", "pub mod gateway;\npub mod admin;\n");
    fixture.write(
        "api/src/gateway.rs",
        "use core::AppEvent;\n\npub fn publish(event: AppEvent) {\n    let _ = event;\n}\n",
    );
    fixture.write("api/src/admin.rs", "pub fn panel() {}\n");
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
event = "api::OrderShipped"

[[async_depends]]
from = "core"
to = "api"
event = "core::AppEvent"
"#,
    );
    fixture
}

/// The same workspace shape WITHOUT the `[[async_depends]]` table — the
/// non-declaring counter-fixture for every byte pin.
fn non_declaring_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"api\", \"core\"]\nresolver = \"2\"\n");
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
         [dependencies]\ncore = { path = \"../core\" }\n",
    );
    fixture.write("api/src/lib.rs", "pub mod gateway;\n");
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
"#,
    );
    fixture
}

/// The async section must never appear in a render.
fn assert_no_async_bytes(label: &str, text: &str) {
    for token in ["-.->", "..> ", "Async census", "async_census"] {
        assert!(
            !text.contains(token),
            "{label} must carry no async bytes ({token:?}):\n{text}"
        );
    }
}

// ---- depgraph: the dashed event-labelled arrows, both render targets ------

/// A declaring run's depgraph mermaid gains the dashed event-labelled
/// arrows after the direct arrow, in `(from, to, event)` order (N6); the
/// direct lines keep their exact bytes (D8). Scenario 08 (consumer verdict
/// on 0.6.86): ONE arrow line per declared entry with the declared UNIT
/// names as endpoints — the projected-module-node fan-out is gone — and
/// the short `unit::SimpleName` label.
#[test]
fn declaring_depgraph_mermaid_renders_dashed_event_arrows_after_the_direct_arrow() {
    let fixture = mixed_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    let direct = rendered
        .find("gateway --> handler")
        .unwrap_or_else(|| panic!("the direct arrow keeps its bytes:\n{rendered}"));
    let first_async = rendered
        .find("api -.->|api::OrderPlaced| core")
        .unwrap_or_else(|| panic!("the declaring row renders dashed with unit endpoints:\n{rendered}"));
    assert!(
        rendered.contains("core -.->|core::AppEvent| api"),
        "both declaring rows render:\n{rendered}"
    );
    assert!(
        rendered.find("core -.->|core::AppEvent| api") > Some(first_async),
        "(from, to, event) order puts api-first:\n{rendered}"
    );
    assert!(
        direct < first_async,
        "async arrows render after every direct arrow:\n{rendered}"
    );
    assert_eq!(
        rendered.matches(" -.->|").count(),
        2,
        "one arrow line per declared entry — no projected-node fan-out:\n{rendered}"
    );
}

/// PlantUML parity: the same rows, the same order, one line per entry, the
/// dashed labelled grammar of this target (`A ..> B : label`).
#[test]
fn declaring_depgraph_plantuml_keeps_async_parity() {
    let fixture = mixed_fixture();
    let output = fixture.run(&["depgraph", "modules", "--format", "plantuml"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    assert!(
        rendered.contains("api ..> core : api::OrderPlaced"),
        "plantuml carries the same rows in its own grammar:\n{rendered}"
    );
    assert!(
        rendered.contains("core ..> api : core::AppEvent"),
        "both declaring rows render:\n{rendered}"
    );
    assert!(
        rendered.find("api ..> core : api::OrderPlaced")
            < rendered.find("core ..> api : core::AppEvent"),
        "(from, to, event) order holds across targets:\n{rendered}"
    );
    assert_eq!(
        rendered.matches(" ..> ").count(),
        2,
        "one arrow line per declared entry on both targets:\n{rendered}"
    );
}

/// Scenario 08's named cell (consumer verdict on 0.6.86, issue #44): an
/// endpoint unit whose paths fold to TWO rendered module nodes must not
/// fan its arrow out. Three declared entries — one pair twice, one event
/// as a dotted FQN — render EXACTLY THREE mermaid lines: unit endpoints,
/// short `unit::SimpleName` labels, no folded-node ids, no declared-FQN
/// bytes, and the PlantUML target prints the same three rows in the same
/// order.
#[test]
fn multi_module_endpoint_entries_render_one_arrow_each_with_unit_endpoints() {
    let fixture = multi_module_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    // The fold is real: the module view renders both `api` modules plus
    // `core`'s node — and that fold no longer repeats any arrow.
    for node in ["  gateway", "  admin", "  handler"] {
        assert!(
            rendered.lines().any(|line| line == node),
            "the module view keeps its projected nodes ({node:?}):\n{rendered}"
        );
    }
    let arrow_lines: Vec<&str> = rendered
        .lines()
        .filter(|line| line.contains(" -.->|"))
        .map(str::trim)
        .collect();
    assert_eq!(
        arrow_lines,
        [
            "api -.->|api::OrderPlaced| core",
            "api -.->|api::OrderShipped| core",
            "core -.->|core::AppEvent| api",
        ],
        "three declared entries render exactly three unit-endpoint lines in (from, to, event) order:\n{rendered}"
    );
    assert!(
        !rendered.contains("Api.Events.OrderPlaced"),
        "the declared FQN never reaches an arrow line:\n{rendered}"
    );
    assert!(
        !rendered.contains("gateway -.->") && !rendered.contains("admin -.->"),
        "no projected-module-node endpoints:\n{rendered}"
    );

    let plantuml = fixture.run(&["depgraph", "modules", "--format", "plantuml"]);
    assert_eq!(plantuml.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&plantuml);
    let arrow_lines: Vec<&str> = rendered
        .lines()
        .filter(|line| line.contains(" ..> "))
        .collect();
    assert_eq!(
        arrow_lines,
        [
            "api ..> core : api::OrderPlaced",
            "api ..> core : api::OrderShipped",
            "core ..> api : core::AppEvent",
        ],
        "plantuml parity: the same three rows, the same order:\n{rendered}"
    );
}

/// The no-async counter-pin: a non-declaring run's depgraph mermaid is the
/// exact pre-async render — one literal comparison carries the bytes.
#[test]
fn non_declaring_depgraph_mermaid_is_byte_pinned() {
    let fixture = non_declaring_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    assert_no_async_bytes("a non-declaring depgraph render", &rendered);
    for line in rendered.lines() {
        assert!(
            line == "graph TD"
                || line == "  gateway"
                || line == "  handler"
                || line == "  gateway --> handler",
            "unexpected line in the pinned no-async render: {line:?}\n{rendered}"
        );
    }
}

// ---- diagram: spec source AND scan source (N2) -----------------------------

/// Spec source: the declared table draws intent as dashed event-labelled
/// arrows after the direct arrows, like the declared forbidden marks — one
/// line per entry, unit endpoints, the short `unit::SimpleName` label
/// (scenario 08).
#[test]
fn declaring_diagram_spec_source_renders_dashed_event_arrows() {
    let fixture = round_trip_fixture();
    let output = fixture.run(&["diagram"]);
    assert_eq!(output.status.code(), Some(0), "diagram must exit 0");
    let rendered = stdout(&output);
    assert!(
        rendered.contains("orders -.->|orders::EventOne| catalog"),
        "the declared row renders in the spec-source diagram:\n{rendered}"
    );
    assert!(
        rendered.contains("catalog -.->|catalog::EventTwo| orders"),
        "both declared rows render:\n{rendered}"
    );
    assert_eq!(
        rendered.matches(" -.->|").count(),
        2,
        "one arrow line per declared entry:\n{rendered}"
    );
}

#[test]
fn declaring_diagram_spec_source_plantuml_keeps_parity() {
    let fixture = round_trip_fixture();
    let output = fixture.run(&["diagram", "--format", "plantuml"]);
    assert_eq!(output.status.code(), Some(0), "diagram must exit 0");
    let rendered = stdout(&output);
    assert!(
        rendered.contains("orders ..> catalog : orders::EventOne"),
        "plantuml parity for the declared rows:\n{rendered}"
    );
    assert!(
        rendered.contains("catalog ..> orders : catalog::EventTwo"),
        "both declared rows render:\n{rendered}"
    );
    assert_eq!(
        rendered.matches(" ..> ").count(),
        2,
        "one arrow line per declared entry on both targets:\n{rendered}"
    );
}

/// Scan source (the artefact side of D10's split): the model's derived
/// `async_edges` render inside the project cluster. No shipped driver
/// populates the collection yet (the derivation arms own that feed), so
/// the cell drives the artefact form: a scan.json carrying derived rows —
/// exactly the rows a future driver's feed serializes — renders the arrows,
/// and the same tree's unpatched scan carries no async bytes at all.
#[test]
fn scan_artefact_async_rows_render_in_the_diagram_scan_source() {
    let fixture = round_trip_fixture();
    let scan = fixture.run(&["scan", "--output", "scan.json"]);
    assert_eq!(scan.status.code(), Some(0), "scan must exit 0");
    let raw = fixture.read("scan.json");
    assert!(
        !raw.contains("async_edges"),
        "a tree whose driver emits no event facts serializes no async bytes:\n{raw}"
    );

    // Patch the derived rows in — the hand-built feed a driver's derivation
    // would serialize (rows order by (from, to, event), kind = async). The
    // second row carries a dotted FQN: the arrow line must state the SHORT
    // label, never the declared FQN (scenario 08).
    let mut model: serde_json::Value = serde_json::from_str(&raw).expect("scan.json parses");
    model["async_edges"] = serde_json::json!([
        { "from": "catalog", "to": "orders", "event": "catalog::EventTwo", "kind": "async" },
        { "from": "orders", "to": "catalog", "event": "Orders.Events.EventOne", "kind": "async" },
    ]);
    fixture.write("scan.json", &serde_json::to_string(&model).expect("model serializes"));

    let output = fixture.run(&["diagram", "--source", "scan", "scan.json"]);
    let rendered = stdout(&output);
    assert!(
        rendered.contains("    catalog -.->|catalog::EventTwo| orders"),
        "the derived rows render dashed inside the project cluster:\n{rendered}"
    );
    assert!(
        rendered.contains("    orders -.->|orders::EventOne| catalog"),
        "the dotted-FQN row renders with the short unit::SimpleName label:\n{rendered}"
    );
    assert!(
        !rendered.contains("Orders.Events.EventOne"),
        "the declared FQN never reaches an arrow line:\n{rendered}"
    );
    assert_eq!(
        rendered.matches(" -.->|").count(),
        2,
        "one arrow line per derived row:\n{rendered}"
    );

    let plantuml = fixture.run(&["diagram", "--source", "scan", "scan.json", "--format", "plantuml"]);
    let rendered = stdout(&plantuml);
    assert!(
        rendered.contains("catalog ..> orders : catalog::EventTwo"),
        "plantuml parity for the derived rows:\n{rendered}"
    );
    assert!(
        rendered.contains("orders ..> catalog : orders::EventOne"),
        "plantuml states the short label too:\n{rendered}"
    );
}

/// The no-async counter-pin for the diagram verbs: a non-declaring spec
/// render and a scan artefact without the collection carry no async bytes.
#[test]
fn non_declaring_diagram_renders_no_async_bytes() {
    let fixture = non_declaring_fixture();
    let spec_render = fixture.run(&["diagram"]);
    assert_no_async_bytes(
        "a non-declaring spec-source diagram",
        &stdout(&spec_render),
    );
    let plantuml = fixture.run(&["diagram", "--format", "plantuml"]);
    assert_no_async_bytes("a non-declaring plantuml diagram", &stdout(&plantuml));
    let scan = fixture.run(&["scan", "--output", "scan.json"]);
    assert_eq!(scan.status.code(), Some(0), "scan must exit 0");
    let scan_render = fixture.run(&["diagram", "--source", "scan", "scan.json"]);
    assert_no_async_bytes("a scan-source diagram without async rows", &stdout(&scan_render));
}

// ---- report: the ONE async census section, three renderers -----------------

/// The section block the text arm renders for the round-trip fixture —
/// pairs in `(from, to, event)` order, then the fan numbers.
const TEXT_CENSUS: &str = "Async census\n\
    ------------\n\
    catalog -> orders — event catalog::EventTwo\n\
    orders -> catalog — event orders::EventOne\n\
    per service: catalog fan-in 1, fan-out 1\n\
    per service: orders fan-in 1, fan-out 1\n\
    per event: catalog::EventTwo fan-in 1, fan-out 1\n\
    per event: orders::EventOne fan-in 1, fan-out 1\n";

/// Text renderer: exactly one section, after Roles (when present), before
/// the violations table — a table in voice, never a second diagram (D9).
#[test]
fn report_text_renders_the_one_async_census_section() {
    let fixture = round_trip_fixture();
    let output = fixture.run(&["report"]);
    assert_eq!(output.status.code(), Some(0), "report must exit 0");
    let rendered = stdout(&output);
    let census = rendered
        .find(TEXT_CENSUS)
        .unwrap_or_else(|| panic!("the census section renders verbatim:\n{rendered}"));
    assert_eq!(
        rendered.matches("Async census").count(),
        1,
        "exactly ONE census section:\n{rendered}"
    );
    if let Some(violations) = rendered.find("Violations by category") {
        assert!(
            census < violations,
            "the census sits before the violations table:\n{rendered}"
        );
    }
}

/// Markdown renderer: the same census as three tables, one section.
#[test]
fn report_markdown_renders_the_async_census_tables() {
    let fixture = round_trip_fixture();
    let output = fixture.run(&["report", "--format", "markdown"]);
    assert_eq!(output.status.code(), Some(0), "report must exit 0");
    let rendered = stdout(&output);
    assert_eq!(
        rendered.matches("## Async census").count(),
        1,
        "exactly ONE census section:\n{rendered}"
    );
    assert!(
        rendered.contains("| catalog | orders | catalog::EventTwo |"),
        "the pair table:\n{rendered}"
    );
    assert!(
        rendered.contains("| orders | catalog | orders::EventOne |"),
        "the pair table:\n{rendered}"
    );
    assert!(
        rendered.contains("| Service | Fan in | Fan out |"),
        "the per-service table:\n{rendered}"
    );
    assert!(
        rendered.contains("| Event | Fan in | Fan out |"),
        "the per-event table:\n{rendered}"
    );
    assert!(
        rendered.contains("| catalog | 1 | 1 |"),
        "the fan numbers (service rows are name-first):\n{rendered}"
    );
}

/// Json renderers: `report --format json` and `verify --json` carry the
/// adjudicated PAIR census — fan-in and fan-out per service and per event
/// type — so an audit quotes numbers, never hand-scripted censuses (D10).
#[test]
fn json_renderers_carry_the_pair_census_numbers() {
    let fixture = round_trip_fixture();
    for args in [
        vec!["report", "--format", "json"],
        vec!["verify", "--json"],
    ] {
        let output = fixture.run(&args);
        let value: serde_json::Value =
            serde_json::from_str(&stdout(&output)).unwrap_or_else(|e| {
                panic!("`archspec {}` must answer the machine body ({e}):\n{}", args.join(" "), stdout(&output))
            });
        let census = &value["async_census"];
        let pairs = census["pairs"].as_array().expect("pairs array");
        assert_eq!(pairs.len(), 2, "the declared pairs, deduplicated:\n{census}");
        assert_eq!(pairs[0]["from"].as_str(), Some("catalog"));
        assert_eq!(pairs[0]["to"].as_str(), Some("orders"));
        assert_eq!(pairs[0]["event"].as_str(), Some("catalog::EventTwo"));
        assert_eq!(pairs[1]["from"].as_str(), Some("orders"));
        assert_eq!(pairs[1]["event"].as_str(), Some("orders::EventOne"));
        assert_eq!(census["per_service"]["catalog"]["fan_in"].as_u64(), Some(1));
        assert_eq!(census["per_service"]["catalog"]["fan_out"].as_u64(), Some(1));
        assert_eq!(census["per_service"]["orders"]["fan_in"].as_u64(), Some(1));
        assert_eq!(census["per_service"]["orders"]["fan_out"].as_u64(), Some(1));
        assert_eq!(
            census["per_event"]["catalog::EventTwo"]["fan_in"].as_u64(),
            Some(1)
        );
        assert_eq!(
            census["per_event"]["orders::EventOne"]["fan_out"].as_u64(),
            Some(1)
        );
    }
}

/// The counter-cells: a non-declaring run renders no census in any of the
/// three formats and serializes no `async_census` key (D8 — absence is the
/// contract; the same verdict body the pre-async binary printed).
#[test]
fn non_declaring_reports_render_no_async_bytes() {
    let fixture = non_declaring_fixture();
    for args in [
        vec!["report"],
        vec!["report", "--format", "markdown"],
        vec!["report", "--format", "json"],
        vec!["verify", "--json"],
    ] {
        let output = fixture.run(&args);
        let rendered = stdout(&output);
        assert!(
            !rendered.contains("Async census") && !rendered.contains("async_census"),
            "`archspec {}` must render no async census bytes for a non-declaring tree:\n{rendered}",
            args.join(" ")
        );
    }
}

/// One fan cell with asymmetric numbers: one publisher addressing two
/// subscribers over one event — per service the publisher fans out twice,
/// per event the fan-out counts the DISTINCT publisher once (the census
/// definition the catalog entry states).
#[test]
fn the_pair_census_counts_fan_out_per_service_and_distinct_publishers_per_event() {
    let fixture = Fixture::new();
    for (name, body) in [
        (
            "orders",
            "pub struct EventOne;\n\npub fn publish() -> EventOne { EventOne }\n",
        ),
        ("catalog", "pub fn handle_a() {}\n"),
        ("shipping", "pub fn handle_b() {}\n"),
    ] {
        fixture.write(
            &format!("{name}/Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        );
        fixture.write(&format!("{name}/src/lib.rs"), body);
    }
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"orders\", \"catalog\", \"shipping\"]\nresolver = \"2\"\n",
    );
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "orders"
matches = { units = ["orders"] }

[[module]]
name = "catalog"
matches = { units = ["catalog"] }

[[module]]
name = "shipping"
matches = { units = ["shipping"] }

[[async_depends]]
from = "orders"
to = "catalog"
event = "orders::EventOne"

[[async_depends]]
from = "orders"
to = "shipping"
event = "orders::EventOne"
"#,
    );
    let output = fixture.run(&["verify", "--json"]);
    let value: serde_json::Value =
        serde_json::from_str(&stdout(&output)).expect("verify --json answers the machine body");
    let census = &value["async_census"];
    assert_eq!(census["pairs"].as_array().expect("pairs").len(), 2);
    assert_eq!(census["per_service"]["orders"]["fan_out"].as_u64(), Some(2));
    assert_eq!(census["per_service"]["catalog"]["fan_in"].as_u64(), Some(1));
    assert_eq!(census["per_service"]["shipping"]["fan_in"].as_u64(), Some(1));
    assert_eq!(census["per_service"]["orders"]["fan_in"].as_u64(), Some(0));
    assert_eq!(
        census["per_event"]["orders::EventOne"]["fan_out"].as_u64(),
        Some(1),
        "per event the fan-out counts the DISTINCT publisher once"
    );
    assert_eq!(
        census["per_event"]["orders::EventOne"]["fan_in"].as_u64(),
        Some(2),
        "per event the fan-in counts the DISTINCT subscribers"
    );
}
