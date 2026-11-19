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
//! module-node-pair fan-out is gone — and the short `unit::SimpleName`
//! label (never the declared FQN). Issue #51 (P3) moves the ENDPOINT law
//! for the folded views: an async endpoint is a NODE of the view, never a
//! raw unit name (Mermaid drew phantom boxes for the raw names); a row the
//! view cannot place folds away with one counted stderr note. The units
//! tier keeps its unit-endpoint bytes exactly. The census cells keep
//! scenario 05's bytes.
//!
//! Cells: the dashed + event-labelled byte cells (both verbs, both sources),
//! the PlantUML parity cells, the multi-module fold-away cell, the
//! fold-to-self and cross-parent note cells, the folded-view subset
//! property, the no-async byte pins, the report-section present/absent
//! cells across the three renderers, the scan.json derived-row cells (the
//! artifact side of D10's split), and the pair-census numbers of
//! `verify --json` and `report --format json`.

mod common;

use common::{Fixture, stdout};
use std::collections::BTreeSet;

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

/// The eShop-shaped fixture (GitHub issue #56): the multi-module publisher of
/// the eShop verdict tree — EIGHTEEN DISTINCT subscriptions over two unit
/// pairs, several events per pair (the legal collision class), no two entries
/// identical. The exact-duplicate law must never fire on this shape: the
/// parse accepts it and the renders keep their byte-for-byte behavior.
fn eshop_shaped_fixture() -> Fixture {
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
    let mut spec = String::from(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"api\"\nmatches = { units = [\"api\"] }\nallowed = { depend_on = [\"core\"] }\n\n[[module]]\nname = \"core\"\nmatches = { units = [\"core\"] }\n",
    );
    for name in [
        "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine", "Ten",
    ] {
        spec.push_str(&format!(
            "\n[[async_depends]]\nfrom = \"api\"\nto = \"core\"\nevent = \"api::Event{name}\"\n"
        ));
    }
    for name in ["One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight"] {
        spec.push_str(&format!(
            "\n[[async_depends]]\nfrom = \"core\"\nto = \"api\"\nevent = \"core::Event{name}\"\n"
        ));
    }
    fixture.write("architecture.spec.toml", &spec);
    fixture
}

/// The fold-to-self fixture (issue #51 P3): units `alpha` and `beta` carry
/// the SAME module name, so the modules view folds both endpoints onto ONE
/// node; the declared row folds onto a self-edge the renderer drops — with
/// the mandatory counted note. `gamma` keeps the node set above the
/// collapse-to-units guard.
fn fold_self_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"alpha\", \"beta\", \"gamma\"]\nresolver = \"2\"\n",
    );
    for unit in ["alpha", "beta", "gamma"] {
        fixture.write(
            &format!("{unit}/Cargo.toml"),
            &format!("[package]\nname = \"{unit}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        );
    }
    fixture.write("alpha/src/lib.rs", "pub mod gateway;\n");
    fixture.write("alpha/src/gateway.rs", "pub fn publish() {}\n");
    fixture.write("beta/src/lib.rs", "pub mod gateway;\n");
    fixture.write("beta/src/gateway.rs", "pub fn consume() {}\n");
    fixture.write("gamma/src/lib.rs", "pub mod admin;\n");
    fixture.write("gamma/src/admin.rs", "pub fn panel() {}\n");
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "alpha"
matches = { units = ["alpha"] }

[[module]]
name = "beta"
matches = { units = ["beta"] }

[[module]]
name = "gamma"
matches = { units = ["gamma"] }

[[async_depends]]
from = "alpha"
to = "beta"
event = "ping::PingEvent"
"#,
    );
    fixture
}

/// The cross-parent fixture (issue #51 P3): unit `shopfront` nests
/// `shop::orders` and `shop::cart` under `shop` plus `billing` beside it;
/// the submodules view of `shopfront::shop` scopes ONLY the nested pair,
/// so a declared `shopfront -> messaging` row has no single node to place
/// either endpoint on — the row folds away and the note names both.
fn cross_parent_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"shopfront\", \"messaging\"]\nresolver = \"2\"\n",
    );
    fixture.write(
        "shopfront/Cargo.toml",
        "[package]\nname = \"shopfront\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("shopfront/src/lib.rs", "pub mod shop;\npub mod billing;\n");
    fixture.write("shopfront/src/shop.rs", "pub mod orders;\npub mod cart;\n");
    fixture.write(
        "shopfront/src/shop/orders.rs",
        "use messaging::client;\n\npub fn place() {\n    client::handle();\n}\n\npub struct CheckoutRequested;\n",
    );
    fixture.write("shopfront/src/shop/cart.rs", "pub fn list() {}\n");
    fixture.write("shopfront/src/billing.rs", "pub fn bill() {}\n");
    fixture.write(
        "messaging/Cargo.toml",
        "[package]\nname = \"messaging\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("messaging/src/lib.rs", "pub mod client;\n");
    fixture.write(
        "messaging/src/client.rs",
        "pub struct CheckoutRequested;\n\npub fn handle(event: CheckoutRequested) {\n    let _ = event;\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "shopfront"
matches = { units = ["shopfront"] }

[[module]]
name = "messaging"
matches = { units = ["messaging"] }

[[async_depends]]
from = "shopfront"
to = "messaging"
event = "shopfront::CheckoutRequested"
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
/// on 0.6.86): ONE arrow line per declared entry — the projected-module-node
/// fan-out is gone. GitHub issue #51 (P3) moves the ENDPOINT law: the folded
/// views print the view's NODES as endpoints, never the raw unit names, and
/// the short `unit::SimpleName` label rides from the declaring unit.
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
        .find("gateway -.->|api::OrderPlaced| handler")
        .unwrap_or_else(|| panic!("the declaring row renders dashed with the folded node endpoints:\n{rendered}"));
    assert!(
        rendered.contains("handler -.->|core::AppEvent| gateway"),
        "both declaring rows render:\n{rendered}"
    );
    assert!(
        rendered.find("handler -.->|core::AppEvent| gateway") > Some(first_async),
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
    assert!(
        !rendered.contains("api -.->"),
        "a raw unit name must not be an endpoint in the modules view (issue #51):\n{rendered}"
    );
    // Every row placed on a rendered node: nothing folded away, no note.
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("folded away"),
        "a view that places every row prints no fold note (issue #51):\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The subset property issue #51 states as a law: in a folded view every
/// async-arrow endpoint is one of the view's OWN declared node lines — no
/// phantom box can appear. The cell parses the render instead of trusting
/// one fixture's bytes.
#[test]
fn every_async_endpoint_in_a_folded_view_is_a_declared_node() {
    let fixture = mixed_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    let nodes: Vec<&str> = rendered
        .lines()
        .filter(|line| line.starts_with("  ") && !line.contains('-'))
        .map(str::trim)
        .collect();
    let mut saw_async = false;
    for line in rendered.lines().filter(|line| line.contains(" -.->|")) {
        saw_async = true;
        let (left, rest) = line.split_once(" -.->|").expect("split marker");
        let (_, to) = rest.rsplit_once('|').expect("arrow carries a label");
        for endpoint in [left.trim(), to.trim()] {
            assert!(
                nodes.contains(&endpoint),
                "async endpoint {endpoint:?} is not a declared node of the view {nodes:?}:\n{rendered}"
            );
        }
    }
    assert!(saw_async, "the declaring fixture renders async rows:\n{rendered}");
}

/// PlantUML parity: the same rows, the same order, one line per entry, the
/// dashed labelled grammar of this target (`A ..> B : label`), and the same
/// folded-node endpoints (issue #51).
#[test]
fn declaring_depgraph_plantuml_keeps_async_parity() {
    let fixture = mixed_fixture();
    let output = fixture.run(&["depgraph", "modules", "--format", "plantuml"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    assert!(
        rendered.contains("gateway ..> handler : api::OrderPlaced"),
        "plantuml carries the same rows in its own grammar:\n{rendered}"
    );
    assert!(
        rendered.contains("handler ..> gateway : core::AppEvent"),
        "both declaring rows render:\n{rendered}"
    );
    assert!(
        rendered.find("gateway ..> handler : api::OrderPlaced")
            < rendered.find("handler ..> gateway : core::AppEvent"),
        "(from, to, event) order holds across targets:\n{rendered}"
    );
    assert_eq!(
        rendered.matches(" ..> ").count(),
        2,
        "one arrow line per declared entry on both targets:\n{rendered}"
    );
}

/// Scenario 08's named cell (consumer verdict on 0.6.86, issue #44), moved
/// by issue #51 (P3): a unit whose module paths fold to TWO rendered nodes
/// has NO single node to place its async endpoint on, so its rows fold
/// AWAY in the modules view — zero async arrows there — with the mandatory
/// stderr note naming the endpoint the view could not place (never silence).
/// The units tier still renders exactly ONE arrow line per declared entry
/// with unit endpoints and short labels, byte-identical to before.
#[test]
fn multi_module_endpoint_entries_fold_away_in_modules_and_render_in_units() {
    let fixture = multi_module_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    // The fold is real: the module view renders both `api` modules plus
    // `core`'s node — and prints no arrow for a row it cannot place.
    for node in ["  gateway", "  admin", "  handler"] {
        assert!(
            rendered.lines().any(|line| line == node),
            "the module view keeps its projected nodes ({node:?}):\n{rendered}"
        );
    }
    assert!(
        !rendered.contains(" -.->|"),
        "a row whose endpoint folds to two nodes renders no arrow (issue #51):\n{rendered}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("note: 3 async rows folded away in this projection"),
        "the fold-away note names the count (issue #51):\n{stderr}"
    );
    assert!(
        stderr.contains("endpoints outside the view's nodes: api"),
        "the note names the unplaced endpoint (issue #51):\n{stderr}"
    );

    let units = fixture.run(&["depgraph", "modules", "--aggregate", "units"]);
    assert_eq!(units.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&units);
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
        "the units render keeps its exact shipped bytes — three unit-endpoint lines in (from, to, event) order:\n{rendered}"
    );
    assert!(
        !rendered.contains("Api.Events.OrderPlaced"),
        "the declared FQN never reaches an arrow line:\n{rendered}"
    );
    assert!(
        !String::from_utf8_lossy(&units.stderr).contains("folded away"),
        "the units tier places every endpoint by name and prints no note:\n{}",
        String::from_utf8_lossy(&units.stderr)
    );

    let plantuml = fixture.run(&["depgraph", "modules", "--aggregate", "units", "--format", "plantuml"]);
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
        "plantuml parity on the units tier: the same three rows, the same order:\n{rendered}"
    );
}

/// The eShop shape at scale (GitHub issue #56): 18 DISTINCT entries over two
/// unit pairs parse green and render one arrow per DISTINCT fact — the exact
/// byte behavior the pre-duplicate-law binary produced for the same shape.
/// The entry accounting (explain's row count) equals the table's entry count.
#[test]
fn eshop_shaped_eighteen_distinct_entries_parse_and_render_one_arrow_each() {
    let fixture = eshop_shaped_fixture();

    // The parse accepts 18 distinct entries — no duplicate refusal anywhere.
    let units = fixture.run(&["depgraph", "modules", "--aggregate", "units"]);
    assert_eq!(
        units.status.code(),
        Some(0),
        "18 distinct entries parse green:\n{}{}",
        stdout(&units),
        String::from_utf8_lossy(&units.stderr)
    );
    let rendered = stdout(&units);
    let arrow_lines: Vec<&str> = rendered
        .lines()
        .filter(|line| line.contains(" -.->|"))
        .map(str::trim)
        .collect();
    assert_eq!(
        arrow_lines.len(),
        18,
        "one arrow per DISTINCT fact — 18 entries, 18 lines:\n{rendered}"
    );
    assert!(
        arrow_lines.contains(&"api -.->|api::EventOne| core")
            && arrow_lines.contains(&"core -.->|core::EventOne| api"),
        "both unit pairs keep their declared rows with unit endpoints:\n{rendered}"
    );

    // The folded modules view counts all 18 rows in its fold-away note.
    let folded = fixture.run(&["depgraph", "modules"]);
    assert_eq!(folded.status.code(), Some(0), "the parse stays green:\n{}", String::from_utf8_lossy(&folded.stderr));
    let err = String::from_utf8_lossy(&folded.stderr);
    assert!(
        err.contains("note: 18 async rows folded away in this projection"),
        "the fold-away note counts the 18 distinct rows (issue #51):\n{err}"
    );

    // Entry accounting at scale: explain prints one row per entry, each
    // numbered once — the #56 double-count cannot recur on the eShop shape.
    let trace = fixture.run(&["explain", "api", "core"]);
    assert_eq!(trace.status.code(), Some(0), "{}", stdout(&trace));
    let trace_out = stdout(&trace);
    let section_rows: Vec<&str> = trace_out
        .lines()
        .skip_while(|line| !line.starts_with("async edges ([[async_depends]]):"))
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .collect();
    let numbered: Vec<&str> = section_rows
        .iter()
        .filter(|line| line.contains("(entry #"))
        .copied()
        .collect();
    assert_eq!(
        numbered.len(),
        18,
        "explain lists the same 18 entries the spec declares, each once:\n{}",
        stdout(&trace)
    );
    let distinct_numbers: BTreeSet<&str> = numbered
        .iter()
        .filter_map(|line| line.rsplit_once("(entry #").map(|(_, rest)| rest.trim_end_matches(')')))
        .collect();
    assert_eq!(
        distinct_numbers.len(),
        18,
        "entry numbers are distinct — no entry is counted twice:\n{}",
        stdout(&trace)
    );
}

/// The fold-to-self class (issue #51 P3, eShop fold-to-self shape): every
/// declaring endpoint's paths land on the SAME rendered node, so the rows
/// fold onto self-edges the renderer drops. The stdout keeps zero async
/// bytes, the units tier keeps rendering its rows, and the note still rides
/// stderr WITHOUT the endpoint tail — nothing fell outside the view.
#[test]
fn fold_to_self_rows_render_no_arrows_and_the_note_names_no_endpoint() {
    let fixture = fold_self_fixture();
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    assert!(
        !rendered.contains(" -.->|") && !rendered.contains("..> "),
        "a fully folded pair renders no async arrow:\n{rendered}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("note: 1 async rows folded away in this projection"),
        "the fold-to-self row is counted, never silenced (issue #51):\n{stderr}"
    );
    assert!(
        !stderr.contains("endpoints outside"),
        "every endpoint placed a node, so the note names none:\n{stderr}"
    );

    let units = fixture.run(&["depgraph", "modules", "--aggregate", "units"]);
    assert_eq!(units.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&units);
    assert!(
        rendered.contains("alpha -.->|alpha::PingEvent| beta"),
        "the units render keeps the row with its unit endpoints and the short label:\n{rendered}"
    );
    assert!(
        !String::from_utf8_lossy(&units.stderr).contains("folded away"),
        "the units tier folds nothing away:\n{}",
        String::from_utf8_lossy(&units.stderr)
    );
}

/// The cross-parent class (issue #51 P3): an endpoint whose paths live
/// OUTSIDE the submodules view's scope places nothing — its row never
/// prints a raw dotted unit name as an endpoint, and the note names every
/// such endpoint.
#[test]
fn submodules_cross_parent_endpoints_never_render_raw_and_the_note_names_them() {
    let fixture = cross_parent_fixture();
    let output = fixture.run(&["depgraph", "submodules", "--parent", "shopfront::shop"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    assert!(
        !rendered.contains(" -.->|"),
        "an endpoint outside the view's scope renders no raw-name arrow:\n{rendered}"
    );
    assert!(
        !rendered.contains("shopfront -.->") && !rendered.contains("messaging -.->"),
        "raw unit names must never reach an arrow line (issue #51):\n{rendered}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("note: 1 async rows folded away in this projection — endpoints outside the view's nodes: messaging, shopfront"),
        "the note names both unplaced endpoints:\n{stderr}"
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

// ---- N4 (#51 residual): endpoints land on their solid-edge module nodes ----

/// The namespace-prefixed fixture (the reference consumer's shape): c#-style
/// units whose OWN module paths do not spell the unit name first — the
/// namespaces hang under a solution-wide root (`ShopApp::Basket::…` under
/// unit `Basket.Api`) — so the pure attribution scan placed nothing and the
/// modules view folded every row away although the graph drew each unit's
/// solid arrows on exactly one node. Three classes ride one tree: a
/// singleton-unit pair (places — the arrow renders on the solid nodes), a
/// multi-module unit (`Reporting.Api`, several — folds away with the note)
/// and a name no unit or module owns (`Billing.Api`, zero — same).
fn namespace_prefixed_fixture(declaring: bool) -> Fixture {
    let fixture = Fixture::new();
    let csproj = |extra: &str| {
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
             <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n{extra}</Project>\n"
        )
    };
    fixture.write("basket/Basket.Api.csproj", &csproj(""));
    fixture.write(
        "basket/Checkout.cs",
        "namespace ShopApp.Basket;\n\npublic class Checkout { }\n",
    );
    fixture.write(
        "basket/Models/Cart.cs",
        "namespace ShopApp.Basket.Models;\n\npublic class Cart { }\n",
    );
    fixture.write(
        "ordering/Ordering.Api.csproj",
        &csproj(
            "  <ItemGroup>\n    <ProjectReference Include=\"..\\basket\\Basket.Api.csproj\" />\n  </ItemGroup>\n",
        ),
    );
    fixture.write(
        "ordering/Events.cs",
        "namespace ShopApp.Ordering.Api;\n\npublic class OrderShipped { }\n",
    );
    fixture.write("reporting/Reporting.Api.csproj", &csproj(""));
    fixture.write(
        "reporting/Report.cs",
        "namespace ShopApp.Reporting.Api;\n\npublic class Report { }\n",
    );
    fixture.write(
        "reporting/Common/Format.cs",
        "namespace ShopApp.Shared;\n\npublic class Format { }\n",
    );
    let mut spec = String::from(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"Basket\"\nmatches = { units = [\"Basket.Api\"] }\n\n[[module]]\nname = \"Ordering\"\nmatches = { units = [\"Ordering.Api\"] }\nallowed = { depend_on = [\"Basket\"] }\n\n[[module]]\nname = \"Reporting\"\nmatches = { units = [\"Reporting.Api\"] }\n",
    );
    if declaring {
        spec.push_str(
            "\n[[async_depends]]\nfrom = \"Ordering.Api\"\nto = \"Basket.Api\"\nevent = \"ShopApp.Ordering.Api.Events.CheckoutStarted\"\n\
             \n[[async_depends]]\nfrom = \"Ordering.Api\"\nto = \"Reporting.Api\"\nevent = \"ShopApp.Ordering.Api.Events.OrderShipped\"\n\
             \n[[async_depends]]\nfrom = \"Billing.Api\"\nto = \"Basket.Api\"\nevent = \"ShopApp.Billing.Api.Events.InvoiceIssued\"\n",
        );
    }
    fixture.write("architecture.spec.toml", &spec);
    fixture
}

/// Salted ids never reach the box text: every dashed endpoint of a folded
/// render must equal a node line the render itself declares, and no endpoint
/// may carry the FNV suffix (`_xxxxxxxx`) — the raw-id class the reference
/// consumer's frozen sweep artifacts show (`Ordering_API_5d46e9ea`).
fn assert_declared_endpoints(label: &str, rendered: &str) {
    let nodes: Vec<String> = rendered
        .lines()
        .filter(|line| {
            line.starts_with("  ") && !line.contains('-') && !line.trim().is_empty()
        })
        .map(|line| line.trim().split_once('[').map_or(line.trim(), |(head, _)| head).to_string())
        .collect();
    for line in rendered.lines().filter(|line| line.contains(" -.->|")) {
        let (left, rest) = line.split_once(" -.->|").expect("split marker");
        let (_, to) = rest.rsplit_once('|').expect("arrow carries a label");
        for endpoint in [left.trim(), to.trim()] {
            assert!(
                nodes.iter().any(|node| node == endpoint),
                "{label}: async endpoint {endpoint:?} is not a declared node:\n{rendered}"
            );
            assert!(
                !endpoint.contains('_'),
                "{label}: async endpoint {endpoint:?} carries a salted id:\n{rendered}"
            );
        }
    }
}

/// Mandatory cells 1 + 2 (issue #51 residual, N4): on a namespace-prefixed
/// tree the modules view renders the placed async arrow on the SAME node
/// names its solid edge uses — byte-equal endpoints, no salted id — while
/// the several-attribution and zero-attribution rows fold away with the
/// counted note naming their endpoints (the note law intact).
#[test]
fn namespace_prefixed_units_place_on_their_solid_module_nodes() {
    let fixture = namespace_prefixed_fixture(true);
    let output = fixture.run(&["depgraph", "modules"]);
    assert_eq!(output.status.code(), Some(0), "depgraph must exit 0");
    let rendered = stdout(&output);
    let solid = rendered
        .find("Ordering --> Basket")
        .unwrap_or_else(|| panic!("the solid edge renders on the folded nodes:\n{rendered}"));
    let async_line = rendered
        .find("Ordering -.->|Ordering.Api::CheckoutStarted| Basket")
        .unwrap_or_else(|| panic!("the placed row renders on the solid nodes, byte-equal endpoints:\n{rendered}"));
    assert!(solid < async_line, "async renders after the direct arrows:\n{rendered}");
    assert_eq!(
        rendered.matches(" -.->|").count(),
        1,
        "only the single-node pair renders; the several/zero rows fold away:\n{rendered}"
    );
    assert_declared_endpoints("depgraph modules", &rendered);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("note: 2 async rows folded away in this projection — endpoints outside the view's nodes: Billing.Api, Reporting.Api"),
        "the note still answers the zero and several classes (issue #51):\n{stderr}"
    );

    let plantuml = fixture.run(&["depgraph", "modules", "--format", "plantuml"]);
    let rendered = stdout(&plantuml);
    assert!(
        rendered.contains("Ordering ..> Basket : Ordering.Api::CheckoutStarted"),
        "plantuml parity — the same placed nodes in its own grammar:\n{rendered}"
    );
    assert_eq!(
        rendered.matches(" ..> ").count(),
        1,
        "one placed row on both targets:\n{rendered}"
    );
}

/// Mandatory cell 3 (issue #51 residual, N4): the spec-source diagram draws
/// the declared rows through the DECLARED MODULE names — an endpoint a
/// module claims via `matches.units` renders byte-identically to the solid
/// module edge; an endpoint no module claims gets a labelled node alias, so
/// no raw salted id ever stands as box text.
#[test]
fn spec_diagram_async_endpoints_use_module_names_and_alias_labels() {
    let fixture = namespace_prefixed_fixture(true);
    let output = fixture.run(&["diagram"]);
    assert_eq!(output.status.code(), Some(0), "diagram must exit 0");
    let rendered = stdout(&output);
    let solid = rendered
        .find("Ordering --> Basket")
        .unwrap_or_else(|| panic!("the declared edge renders:\n{rendered}"));
    let placed = rendered
        .find("Ordering -.->|Ordering.Api::CheckoutStarted| Basket")
        .unwrap_or_else(|| panic!("the claimed endpoint renders as the claiming module:\n{rendered}"));
    assert!(solid < placed, "async renders after the direct arrows:\n{rendered}");
    assert!(
        rendered.contains("Ordering -.->|Ordering.Api::OrderShipped| Reporting"),
        "the several-attribution endpoint still names its declared module here — the spec vocabulary draws claims, not folds:\n{rendered}"
    );
    // The unclaimed endpoint renders through its own label definition.
    let alias_line = rendered
        .lines()
        .find(|line| line.contains(" -.->|Billing.Api::InvoiceIssued|"))
        .unwrap_or_else(|| panic!("the unclaimed row still renders its declared fact:\n{rendered}"));
    let (head, _rest) = alias_line.split_once(" -.->|").expect("split marker");
    let id = head.trim();
    assert!(
        rendered.contains(&format!("  {id}[\"Billing.Api\"]")),
        "the unclaimed endpoint carries a label definition, never a bare salted id:\n{rendered}"
    );
    assert!(
        id != "Billing.Api" && id.starts_with("Billing_Api_"),
        "the alias id is the escaped salted form of the name:\n{alias_line}"
    );
    for line in rendered.lines().filter(|line| line.contains(" -.->|")) {
        let (left, rest) = line.split_once(" -.->|").expect("split marker");
        let (_, to) = rest.rsplit_once('|').expect("arrow carries a label");
        for endpoint in [left.trim(), to.trim()] {
            let head = rendered
                .lines()
                .map(str::trim)
                .find(|line| *line == endpoint || line.starts_with(&format!("{endpoint}[")));
            assert!(
                head.is_some(),
                "spec-diagram endpoint {endpoint:?} is not a declared node:\n{rendered}"
            );
        }
    }

    let plantuml = fixture.run(&["diagram", "--format", "plantuml"]);
    let rendered = stdout(&plantuml);
    assert!(
        rendered.contains("Ordering ..> Basket : Ordering.Api::CheckoutStarted"),
        "plantuml parity of the claimed endpoints:\n{rendered}"
    );
    assert!(
        rendered.contains("component Billing.Api"),
        "plantuml declares the unclaimed endpoint too:\n{rendered}"
    );
    assert!(
        rendered.contains("Billing.Api ..> Basket : Billing.Api::InvoiceIssued"),
        "the unclaimed row renders in plantuml's grammar:\n{rendered}"
    );
}

/// Mandatory cell 5: the same tree WITHOUT the table renders no async bytes
/// anywhere — the alias arm is a row consumer, never a standing declaration.
#[test]
fn namespace_prefixed_non_declaring_renders_no_async_bytes() {
    let fixture = namespace_prefixed_fixture(false);
    for args in [
        vec!["depgraph", "modules"],
        vec!["depgraph", "modules", "--format", "plantuml"],
        vec!["diagram"],
        vec!["diagram", "--format", "plantuml"],
    ] {
        let output = fixture.run(&args);
        let rendered = stdout(&output);
        assert_no_async_bytes(&args.join(" "), &rendered);
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("folded away"),
            "a non-declaring run prints no fold note:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
