//! Workplan visibility_phase2 scenario 05: the `--layer` selector on the
//! diagram and depgraph renderers plus the graphviz-SVG/dot emitter arms.
//! The model separates the layers by construction — `async_edges` rows
//! NEVER enter `edges` (src/model.rs) — so the selector is a render-side
//! read of the two collections at the per-renderer loops. The shipped-bytes
//! law for this slice: `--layer both` (and the flag absent) reproduces the
//! recorded pre-change bytes BYTE-IDENTICALLY on every text renderer;
//! `direct` draws the structural arrows, `indirect` the dashed
//! event-labelled async rows. The legend is a dot/SVG-surface feature —
//! one generated section whenever `both` draws both edge families —
//! because a legend row in the mermaid/plantuml text would move the
//! shipped bytes the first cell pins.
//!
//! Cells (scenario §Done-when):
//! 1. byte-identity — recorded pre-change bytes, flag absent and `--layer both`;
//! 2. selection — `direct` exactly the structural arrow, zero dashed;
//!    `indirect` exactly the dashed event-labelled arrows;
//! 3. legend — present on `both` dot/SVG output, absent on `direct`/`indirect`;
//! 4. SVG — container-run `dot -Tsvg` over the emitted dot body yields a
//!    parseable SVG with per-class colors and the legend group (named SKIP
//!    when no dot resolves: `DOT` -> PATH -> SKIP, the family probe law),
//!    and the hand-emitted layout carries the mermaid render's inventory;
//! 5. rationale — the help surface names the mermaid ceilings and the
//!    corpus-scale witness so the unrenderable claim stays falsifiable;
//!    one graph-assembly structural cell.

mod common;

use common::{stderr, stdout, Fixture};

// ---------------------------------------------------------------------------
// fixtures — the recorded pair: a spec source with and without async rows,
// a scan artefact with and without async rows
// ---------------------------------------------------------------------------

fn spec_basic_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"Billing\"\nmatches = { units = [\"Billing*\"] }\n\n[module.allowed]\ndepend_on = [\"Shared\"]\nforbidden = [\"Portal\"]\n\n[[module]]\nname = \"Shared\"\nmatches = { units = [\"Shared*\"] }\n\n[module.allowed]\ndepend_on = []\nforbidden = []\n\n[[module]]\nname = \"Portal\"\nmatches = { units = [\"Portal*\"] }\n\n[module.allowed]\ndepend_on = [\"Shared\"]\nforbidden = []\n",
    );
    fixture
}

fn spec_async_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"orders\"\nmatches = { units = [\"orders\"] }\n\n[module.allowed]\ndepend_on = [\"catalog\"]\nforbidden = []\n\n[[module]]\nname = \"catalog\"\nmatches = { units = [\"catalog\"] }\n\n[module.allowed]\ndepend_on = []\nforbidden = []\n\n[[async_depends]]\nfrom = \"orders\"\nto = \"catalog\"\nevent = \"orders::EventOne\"\n\n[[async_depends]]\nfrom = \"catalog\"\nto = \"orders\"\nevent = \"catalog::EventTwo\"\n",
    );
    fixture
}

fn scan_fixture(scan_json: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write("scan.json", scan_json);
    fixture
}

const SCAN_STRUCT: &str = "{\n  \"schema_version\": 1,\n  \"language\": \"rust\",\n  \"units\": [\n    { \"name\": \"alpha\", \"kind\": \"lib\", \"path\": \"alpha\" },\n    { \"name\": \"beta\", \"kind\": \"lib\", \"path\": \"beta\" }\n  ],\n  \"edges\": [ { \"from\": \"alpha\", \"to\": \"beta\" }, { \"from\": \"alpha\", \"to\": \"serde\" } ],\n  \"external\": [ \"serde\" ],\n  \"async_edges\": []\n}\n";

const SCAN_ASYNC: &str = "{\n  \"schema_version\": 1,\n  \"language\": \"rust\",\n  \"units\": [\n    { \"name\": \"alpha\", \"kind\": \"lib\", \"path\": \"alpha\" },\n    { \"name\": \"beta\", \"kind\": \"lib\", \"path\": \"beta\" }\n  ],\n  \"edges\": [ { \"from\": \"alpha\", \"to\": \"beta\" }, { \"from\": \"alpha\", \"to\": \"serde\" } ],\n  \"external\": [ \"serde\" ],\n  \"async_edges\": [\n    { \"from\": \"beta\", \"to\": \"alpha\", \"event\": \"beta::Ping\", \"kind\": \"async\" }\n  ]\n}\n";

/// The rust workspace whose declaring spec renders one solid arrow and two
/// dashed event arrows in the folded `depgraph modules` view (the
/// `async_render.rs` mixed shape — the loop pair this selector gates).
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

// ---------------------------------------------------------------------------
// cell 1 — the recorded pre-change bytes (base binary 0.6.123 @ origin/main,
// captured before any scenario-05 edit; replayed here with the flag absent
// and with `--layer both`, both must be byte-identical)
// ---------------------------------------------------------------------------

const SPEC_BASIC_MERMAID: &str = "graph TD\n  Billing\n  Portal\n  Shared\n  Billing --> Shared\n  Portal --> Shared\n  Billing -.->|forbidden| Portal\n";
const SPEC_BASIC_PLANTUML: &str = "@startuml\ncomponent Billing\ncomponent Portal\ncomponent Shared\nBilling --> Shared\nPortal --> Shared\nBilling -[#red]-> Portal : forbidden\n@enduml\n";
const SPEC_ASYNC_MERMAID: &str = "graph TD\n  catalog\n  orders\n  orders --> catalog\n  catalog -.->|catalog::EventTwo| orders\n  orders -.->|orders::EventOne| catalog\n";
const SPEC_ASYNC_PLANTUML: &str = "@startuml\ncomponent catalog\ncomponent orders\norders --> catalog\ncatalog ..> orders : catalog::EventTwo\norders ..> catalog : orders::EventOne\n@enduml\n";
const SCAN_STRUCT_MERMAID: &str = "graph TD\n  subgraph project\n    alpha\n    beta\n    alpha --> beta\n  end\n  subgraph external\n    serde\n  end\n  alpha --> serde\n";
const SCAN_STRUCT_PLANTUML: &str = "@startuml\npackage project {\n  component alpha\n  component beta\nalpha --> beta\n}\npackage external {\n  component serde\n}\nalpha --> serde\n@enduml\n";
const SCAN_ASYNC_MERMAID: &str = "graph TD\n  subgraph project\n    alpha\n    beta\n    alpha --> beta\n    beta -.->|beta::Ping| alpha\n  end\n  subgraph external\n    serde\n  end\n  alpha --> serde\n";
const SCAN_ASYNC_PLANTUML: &str = "@startuml\npackage project {\n  component alpha\n  component beta\nalpha --> beta\nbeta ..> alpha : beta::Ping\n}\npackage external {\n  component serde\n}\nalpha --> serde\n@enduml\n";

fn assert_recorded(fixture: &Fixture, args: &[&str], recorded: &str, label: &str) {
    let default = fixture.run(args);
    assert_eq!(
        default.status.code(),
        Some(0),
        "{label}: {label} render must exit 0:\n{}",
        stderr(&default)
    );
    assert_eq!(stdout(&default), recorded, "{label}: flag-absent bytes moved");
    let mut both: Vec<&str> = args.to_vec();
    both.extend(["--layer", "both"]);
    let explicit = fixture.run(&both);
    assert_eq!(
        explicit.status.code(),
        Some(0),
        "{label}: explicit --layer both must exit 0:\n{}",
        stderr(&explicit)
    );
    assert_eq!(
        stdout(&explicit),
        recorded,
        "{label}: --layer both bytes moved away from the recorded output"
    );
}

#[test]
fn layer_both_reproduces_the_recorded_bytes_on_every_text_renderer() {
    assert_recorded(
        &spec_basic_fixture(),
        &["diagram"],
        SPEC_BASIC_MERMAID,
        "spec mermaid",
    );
    assert_recorded(
        &spec_basic_fixture(),
        &["diagram", "--format", "plantuml"],
        SPEC_BASIC_PLANTUML,
        "spec plantuml",
    );
    assert_recorded(
        &spec_async_fixture(),
        &["diagram"],
        SPEC_ASYNC_MERMAID,
        "spec async mermaid",
    );
    assert_recorded(
        &spec_async_fixture(),
        &["diagram", "--format", "plantuml"],
        SPEC_ASYNC_PLANTUML,
        "spec async plantuml",
    );
    let struct_fixture = scan_fixture(SCAN_STRUCT);
    assert_recorded(
        &struct_fixture,
        &["diagram", "--source", "scan", "scan.json"],
        SCAN_STRUCT_MERMAID,
        "scan mermaid",
    );
    assert_recorded(
        &struct_fixture,
        &["diagram", "--source", "scan", "scan.json", "--format", "plantuml"],
        SCAN_STRUCT_PLANTUML,
        "scan plantuml",
    );
    let async_fixture = scan_fixture(SCAN_ASYNC);
    assert_recorded(
        &async_fixture,
        &["diagram", "--source", "scan", "scan.json"],
        SCAN_ASYNC_MERMAID,
        "scan async mermaid",
    );
    assert_recorded(
        &async_fixture,
        &["diagram", "--source", "scan", "scan.json", "--format", "plantuml"],
        SCAN_ASYNC_PLANTUML,
        "scan async plantuml",
    );
}

// ---------------------------------------------------------------------------
// cell 2 — selection: exactly the layer named, arrow-family counts are law
// ---------------------------------------------------------------------------

/// Mermaid arrow lines: solid (` --> `) or dashed (` -.->|`); node lines
/// carry no arrow glyph, so a count through these markers is the arrow
/// inventory itself.
fn mermaid_arrows(rendered: &str) -> Vec<&str> {
    rendered
        .lines()
        .filter(|line| line.contains("-->") || line.contains("-.->"))
        .collect()
}

fn count_with(rendered: &str, needle: &str) -> usize {
    rendered.lines().filter(|line| line.contains(needle)).count()
}

#[test]
fn layer_direct_draws_only_the_structural_arrows() {
    let spec = spec_async_fixture();
    let mermaid = stdout(&spec.run(&["diagram", "--layer", "direct"]));
    assert!(
        mermaid.contains("orders --> catalog"),
        "the structural arrow keeps its bytes:\n{mermaid}"
    );
    assert_eq!(count_with(&mermaid, " --> "), 1, "exactly one solid arrow:\n{mermaid}");
    assert_eq!(count_with(&mermaid, "-.->"), 0, "zero dashed lines:\n{mermaid}");
    let plantuml = stdout(&spec.run(&["diagram", "--format", "plantuml", "--layer", "direct"]));
    assert_eq!(count_with(&plantuml, " --> "), 1, "plantuml parity:\n{plantuml}");
    assert_eq!(count_with(&plantuml, " ..> "), 0, "no plantuml async grammar:\n{plantuml}");

    let scan = scan_fixture(SCAN_ASYNC);
    let mermaid = stdout(&scan.run(&["diagram", "--source", "scan", "scan.json", "--layer", "direct"]));
    assert_eq!(count_with(&mermaid, " --> "), 2, "both structural arrows draw:\n{mermaid}");
    assert_eq!(count_with(&mermaid, "-.->"), 0, "zero dashed lines:\n{mermaid}");

    let mixed = mixed_fixture();
    let output = mixed.run(&["depgraph", "modules", "--layer", "direct"]);
    assert_eq!(output.status.code(), Some(0), "depgraph direct must exit 0");
    let rendered = stdout(&output);
    assert!(
        rendered.contains("gateway --> handler"),
        "the declaring view's structural arrow draws:\n{rendered}"
    );
    assert_eq!(count_with(&rendered, "-.->"), 0, "depgraph direct draws no async:\n{rendered}");
}

#[test]
fn layer_indirect_draws_only_the_async_rows() {
    let spec = spec_async_fixture();
    let mermaid = stdout(&spec.run(&["diagram", "--layer", "indirect"]));
    assert_eq!(count_with(&mermaid, "-.->"), 2, "exactly the two event arrows:\n{mermaid}");
    assert!(
        mermaid.contains("orders -.->|orders::EventOne| catalog")
            && mermaid.contains("catalog -.->|catalog::EventTwo| orders"),
        "both declaring rows render dashed and event-labelled:\n{mermaid}"
    );
    assert_eq!(count_with(&mermaid, " --> "), 0, "no structural arrow:\n{mermaid}");
    let plantuml = stdout(&spec.run(&["diagram", "--format", "plantuml", "--layer", "indirect"]));
    assert_eq!(count_with(&plantuml, " ..> "), 2, "plantuml parity:\n{plantuml}");
    assert_eq!(count_with(&plantuml, " --> "), 0, "no plantuml solid arrows:\n{plantuml}");

    let scan = scan_fixture(SCAN_ASYNC);
    let mermaid = stdout(&scan.run(&["diagram", "--source", "scan", "scan.json", "--layer", "indirect"]));
    assert_eq!(count_with(&mermaid, "-.->"), 1, "exactly the derived row:\n{mermaid}");
    assert!(mermaid.contains("beta -.->|beta::Ping| alpha"), "the derived row draws:\n{mermaid}");
    assert_eq!(count_with(&mermaid, " --> "), 0, "structural arrows stand down:\n{mermaid}");

    let mixed = mixed_fixture();
    let output = mixed.run(&["depgraph", "modules", "--layer", "indirect"]);
    assert_eq!(output.status.code(), Some(0), "depgraph indirect must exit 0");
    let rendered = stdout(&output);
    assert_eq!(count_with(&rendered, "-.->"), 2, "exactly the two declaring rows:\n{rendered}");
    assert!(
        rendered.contains("gateway -.->|api::OrderPlaced| handler")
            && rendered.contains("handler -.->|core::AppEvent| gateway"),
        "both declaring rows render with the folded node endpoints:\n{rendered}"
    );
    assert_eq!(count_with(&rendered, " --> "), 0, "structural arrows stand down:\n{rendered}");
}

#[test]
fn layer_must_name_a_supported_layer() {
    let spec = spec_async_fixture();
    let output = spec.run(&["diagram", "--layer", "sideways"]);
    assert_ne!(output.status.code(), Some(0), "an unknown layer value is refused");
    assert!(
        stderr(&output).contains("invalid value \"sideways\" for --layer — supported layers: direct, indirect, both"),
        "the refusal names the value and the supported layers:\n{}",
        stderr(&output)
    );
    let depgraph = mixed_fixture().run(&["depgraph", "modules", "--layer", "sideways"]);
    assert_ne!(depgraph.status.code(), Some(0), "depgraph refuses an unknown layer too");
    assert!(
        stderr(&depgraph).contains("supported layers: direct, indirect, both"),
        "depgraph refusal shape:\n{}",
        stderr(&depgraph)
    );
}

// ---------------------------------------------------------------------------
// cell 3 — the legend: one generated section exactly when `both` draws both
// families (dot/SVG surface — the text renderers stay the shipped bytes)
// ---------------------------------------------------------------------------

const LEGEND_GROUP: &str = "<g class=\"legend\">";

#[test]
fn the_legend_fires_only_when_both_draws_both_families() {
    let spec = spec_async_fixture();
    let svg_both = stdout(&spec.run(&["diagram", "--format", "svg"]));
    assert!(svg_both.contains(LEGEND_GROUP), "both draws the legend group:\n{svg_both}");
    let dot_both = stdout(&spec.run(&["diagram", "--format", "dot"]));
    assert!(dot_both.contains("cluster_legend"), "the dot arm draws the legend cluster:\n{dot_both}");
    for layer in ["direct", "indirect"] {
        let svg = stdout(&spec.run(&["diagram", "--format", "svg", "--layer", layer]));
        assert!(
            !svg.contains(LEGEND_GROUP),
            "a single-family selection stays legend-quiet (layer {layer}):\n{svg}"
        );
        let dot = stdout(&spec.run(&["diagram", "--format", "dot", "--layer", layer]));
        assert!(
            !dot.contains("cluster_legend"),
            "the dot arm stays legend-quiet too (layer {layer}):\n{dot}"
        );
    }
    // The single-family `both` render (no async rows declared) stays
    // legend-quiet too — the legend fires on families, not on the flag.
    let basic = spec_basic_fixture();
    let svg = stdout(&basic.run(&["diagram", "--format", "svg"]));
    assert!(!svg.contains(LEGEND_GROUP), "a structural-only graph draws no legend:\n{svg}");
    let dot = stdout(&basic.run(&["diagram", "--format", "dot"]));
    assert!(!dot.contains("cluster_legend"), "dot parity on the single-family graph:\n{dot}");
}

// ---------------------------------------------------------------------------
// cell 4 — the SVG emitter: container-run dot materialization (named SKIP
// where no dot resolves) and the hand-emitted inventory law
// ---------------------------------------------------------------------------

/// The tool probe law: `DOT` -> PATH `dot` -> one named SKIP line. The
/// build container carries no graphviz by design (no vendored binary in
/// the payload, no host `dot` assumption — scenario §The design), so the
/// suite prints the SKIP row and the lane's evidence run carries the real
/// materialization receipt.
fn resolve_dot() -> Option<String> {
    if let Ok(path) = std::env::var("DOT") {
        return Some(path);
    }
    let probe = std::process::Command::new("dot").arg("-V").output();
    probe.ok().map(|_| "dot".to_string())
}

#[test]
fn the_dot_body_materializes_to_a_parseable_svg_with_colors_and_legend() {
    let Some(dot) = resolve_dot() else {
        eprintln!("svg-materialize: SKIP — no dot binary (DOT unset, none on PATH)");
        return;
    };
    let spec = spec_async_fixture();
    let body = stdout(&spec.run(&["diagram", "--format", "dot"]));
    spec.write("graph.dot", &body);
    let output = std::process::Command::new(&dot)
        .arg("-Tsvg")
        .arg(spec.path("graph.dot"))
        .output()
        .expect("dot runs once resolved");
    assert_eq!(
        output.status.code(),
        Some(0),
        "dot -Tsvg must materialize the emitted body:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let svg = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(svg.contains("<svg") && svg.contains("</svg>"), "the materialized SVG parses:\n{svg}");
    // Per-class attributes ride the materialization: node fill by class
    // and the async stroke vocabulary survive into the rendered SVG.
    assert!(svg.contains("#dbe7f3"), "the unit-family node fill renders:\n{svg}");
    assert!(svg.contains("#0066cc"), "the async edge stroke renders:\n{svg}");
    assert!(
        svg.contains("structural layer"),
        "the legend rows ride the materialized SVG:\n{svg}"
    );
}

fn svg_nodes(svg: &str) -> usize {
    svg.matches("<g class=\"node\"").count()
}

fn svg_edges(svg: &str) -> usize {
    svg.matches("<path class=\"edge\"").count()
}

/// Edge statements in a dot body: one `"from" -> "to"` line per drawn arrow;
/// legend rows are node declarations, never edges, so no legend filter rides
/// here.
fn dot_edges(dot: &str) -> usize {
    dot.lines().filter(|line| line.contains("\" -> \"")).count()
}

/// Node declarations in a dot body: one `"id" [label=` line per node; the
/// legend's plaintext nodes carry the `legend_` id prefix and stand apart.
fn dot_nodes(dot: &str) -> usize {
    dot.lines()
        .filter(|line| line.contains("[label=") && !line.contains("\"legend_"))
        .count()
}

#[test]
fn the_hand_emitted_svg_carries_the_mermaid_inventory_and_class_colors() {
    let scan = scan_fixture(SCAN_ASYNC);
    let mermaid = stdout(&scan.run(&["diagram", "--source", "scan", "scan.json"]));
    let svg = stdout(&scan.run(&["diagram", "--source", "scan", "scan.json", "--format", "svg"]));
    assert_eq!(svg_nodes(&svg), 3, "one group per assembled node (alpha, beta, serde)");
    assert_eq!(
        svg_edges(&svg),
        mermaid_arrows(&mermaid).len(),
        "the svg edge inventory is the mermaid arrow inventory"
    );
    assert!(svg.contains("#dbe7f3"), "project nodes carry the unit-class fill");
    assert!(svg.contains("#f0f0f0"), "external nodes carry the external-class fill");
    assert!(svg.contains("#0066cc"), "the async edge carries the async-class stroke");
    assert!(svg.contains(LEGEND_GROUP), "the generated legend group draws");

    let direct = stdout(&scan.run(&["diagram", "--source", "scan", "scan.json", "--format", "svg", "--layer", "direct"]));
    assert_eq!(svg_nodes(&direct), 3, "selection moves edges, never nodes");
    assert_eq!(svg_edges(&direct), 2, "the two structural arrows draw");
    assert!(!direct.contains("#0066cc"), "the async class is absent under direct");

    let spec = spec_async_fixture();
    let mermaid = stdout(&spec.run(&["diagram"]));
    let dot = stdout(&spec.run(&["diagram", "--format", "dot"]));
    assert_eq!(dot_edges(&dot), mermaid_arrows(&mermaid).len(), "the dot body edge inventory is the mermaid inventory");
    assert_eq!(dot_nodes(&dot), 2, "the dot body declares exactly the two module nodes");
}

// ---------------------------------------------------------------------------
// cell 5 — the ceiling rationale stays falsifiable on the help surface,
// and the one-graph-assembly law holds structurally
// ---------------------------------------------------------------------------

#[test]
fn help_names_the_mermaid_ceilings_and_the_corpus_witness() {
    let help = stdout(&Fixture::new().run(&["diagram", "--help"]));
    assert!(
        help.contains("ceilings of 500 edges and 50,000 characters"),
        "the pinned mermaid ceilings state themselves on the help surface:\n{help}"
    );
    assert!(
        help.contains("54,869 module-edge rows and 11,520 file nodes"),
        "the corpus-scale witness names the unrenderable claim:\n{help}"
    );
    assert!(help.contains("--layer <layer>"), "the selector documents itself:\n{help}");
    let depgraph_help = stdout(&Fixture::new().run(&["depgraph", "--help"]));
    assert!(depgraph_help.contains("--layer <layer>"), "depgraph documents the selector too:\n{depgraph_help}");
}

#[test]
fn one_graph_assembly_site_feeds_every_format() {
    // The drift law (research report §5.5): one assembler per graph, and
    // every format arm reads its output. A second assembler would have to
    // construct the graph structs away from the builder. The count below is
    // the structural pin: one struct DEFINITION and one struct LITERAL per
    // graph, the literal standing only inside the builder. The cross-format
    // parity cells are the behavioural pin.
    let source = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/archspec/diagram.rs"))
        .expect("diagram.rs reads");
    let literal_lines = |ty: &str| {
        source.lines().filter(|line| {
            let t = line.trim();
            t.starts_with(ty) && t.ends_with('{') && !line.contains("struct ") && !line.contains("-> ")
        }).count()
    };
    assert_eq!(source.matches("struct SpecGraph {").count(), 1, "one SpecGraph definition");
    assert_eq!(literal_lines("SpecGraph {"), 1, "the SpecGraph literal stands only in build_graph");
    assert_eq!(source.matches("struct ScanGraph {").count(), 1, "one ScanGraph definition");
    assert_eq!(literal_lines("ScanGraph {"), 1, "the ScanGraph literal stands only in build_scan_graph");
}

/// Cross-format parity on the scan source too — the assembly law holds on
/// every arm pair, not just the spec source.
#[test]
fn dot_and_svg_bodies_agree_with_the_mermaid_render_on_the_scan_source() {
    let scan = scan_fixture(SCAN_ASYNC);
    let mermaid = stdout(&scan.run(&["diagram", "--source", "scan", "scan.json"]));
    let dot = stdout(&scan.run(&["diagram", "--source", "scan", "scan.json", "--format", "dot"]));
    let svg = stdout(&scan.run(&["diagram", "--source", "scan", "scan.json", "--format", "svg"]));
    assert_eq!(dot_edges(&dot), mermaid_arrows(&mermaid).len(), "dot edges == mermaid arrows");
    assert_eq!(svg_edges(&svg), mermaid_arrows(&mermaid).len(), "svg edges == mermaid arrows");
    assert_eq!(svg_nodes(&svg), dot_nodes(&dot), "dot and svg carry the same node inventory");
}
