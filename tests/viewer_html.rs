//! Workplan visibility_phase2 scenario 06: the single-file interactive
//! viewer emitter (`--format viewer`) and its publish channel.
//!
//! Cells (scenario §Done-when):
//! 1. emit — a fixture scan artefact (structural + async rows, names with
//!    `<`, `&`, quote chars) renders ONE `viewer.html`: zero external asset
//!    references, the embedded script passes `node --check` (`NODE` -> PATH
//!    -> named SKIP, the family probe law), every fixture name appears ONLY
//!    escaped, the emitted bytes contain `innerHTML` zero times, and the
//!    click-handler, legend and layer-toggle markup covers the inventory;
//! 2. zero-vendored — the payload tree ships no standalone `*.js` asset and
//!    no third-party library identifier (`cytoscape`, `d3`, `elk`,
//!    `mermaid.min` — word-boundary) greps ZERO across the shipped files;
//! 3. corpus probe — documented, external, non-CI: the probe script
//!    `rust-arch-test-kit/internal/viewer-corpus-probe.sh` emits the viewer
//!    over a corpus scan and checks the probe shape. Armed only when
//!    `ARCHSPEC_VIEWER_CORPUS` names the corpus tree; otherwise one named
//!    SKIP row states it ran out-of-band;
//! 4. limitation rows — the help surface names gitea-no-Pages (28.x, the
//!    404 receipt) and BOTH named-not-built alternatives (`archspec serve`,
//!    gitea-side infra hosting), greppable so silence cannot return;
//! 5. channel — the payload `.github/workflows/pages.yml` exists,
//!    references ONLY the viewer artifact, and the publish gate's
//!    leak-pattern list (read from the publish workflow, never restated)
//!    is clean over it.
//!
//! NAMED non-fleet-surface justification (§E2E-PIN): the payload suite is
//! the pin vehicle; the Pages workflow is a mirror publish concern — no
//! fleet-surface key path moves.

mod common;

use common::{stderr, stdout, Fixture};
use std::path::{Path, PathBuf};
use std::process::Command;

// ---------------------------------------------------------------------------
// cell 1 — the emit fixture: structural + async rows over names that carry
// `<`, `&`, `"` and `'` so the escape posture has teeth
// ---------------------------------------------------------------------------

const SCAN_HOSTILE: &str = r#"{
  "schema_version": 1,
  "language": "rust",
  "units": [
    { "name": "a<b>c", "kind": "lib", "path": "alpha" },
    { "name": "d&e\"f", "kind": "lib", "path": "beta" }
  ],
  "edges": [ { "from": "a<b>c", "to": "d&e\"f" }, { "from": "a<b>c", "to": "ser&de" } ],
  "external": [ "ser&de" ],
  "async_edges": [
    { "from": "d&e\"f", "to": "a<b>c", "event": "ev'<x>&y", "kind": "async" }
  ]
}
"#;

fn emit(fixture: &Fixture) -> String {
    let output = fixture.run(&[
        "diagram", "--source", "scan", "scan.json", "--format", "viewer", "--output", "viewer.html",
    ]);
    assert!(
        output.status.success(),
        "viewer emit must succeed: {}\nstderr: {}",
        stdout(&output),
        stderr(&output)
    );
    fixture.read("viewer.html")
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

#[test]
fn emit_is_one_self_contained_escaped_file() {
    let fixture = Fixture::new();
    fixture.write("scan.json", SCAN_HOSTILE);
    let doc = emit(&fixture);

    // One self-contained document: doctype, one inline script, zero
    // external asset references.
    assert!(doc.starts_with("<!DOCTYPE html>"), "one HTML document");
    assert_eq!(count(&doc, "<script>"), 1, "exactly one inline script");
    for forbidden in [
        "http://",
        "https://",
        "<link",
        "src=",
        "<img",
        "@import",
        "url(",
        "xmlns=",
    ] {
        assert!(
            !doc.contains(forbidden),
            "no external asset reference {forbidden:?} in the emitted markup"
        );
    }

    // The XSS law: `innerHTML` never ships, model data never rides it.
    assert_eq!(count(&doc, "innerHTML"), 0, "innerHTML zero times");
    assert!(doc.contains("textContent"), "data reaches the DOM via textContent");

    // Every fixture name appears ONLY escaped — the raw spellings must be
    // absent from the emitted bytes.
    for raw in ["a<b>c", "d&e\"f", "ev'<x>&y"] {
        assert!(!doc.contains(raw), "raw model string {raw:?} never ships");
    }
    for escaped in [
        "a&lt;b&gt;c",
        "d&amp;e&quot;f",
        "ev&apos;&lt;x&gt;&amp;y",
        "ser&amp;de",
    ] {
        assert!(doc.contains(escaped), "escaped model string {escaped:?} present");
    }

    // Inventory coverage: one node group and one edge line per fixture row,
    // plus the legend and the three layer radios.
    assert_eq!(count(&doc, "class=\"node unit\""), 2, "two unit nodes");
    assert_eq!(count(&doc, "class=\"node external\""), 1, "one external node");
    assert_eq!(count(&doc, "class=\"edge direct\""), 2, "two direct arrows");
    assert_eq!(count(&doc, "class=\"edge async\""), 1, "one async arrow");
    assert!(doc.contains("id=\"legend\""), "legend section");
    assert!(doc.contains("unit — project unit tier (2)"), "legend counts the unit tier");
    assert!(doc.contains("external — dependency outside the project (1)"), "legend counts externals");
    assert!(doc.contains("direct — solid structural arrow (2)"), "legend counts the direct layer");
    assert!(doc.contains("indirect — dashed event-labelled async row (1)"), "legend counts the async layer");
    assert!(doc.contains("name=\"layer\" value=\"both\" checked"), "layer toggle defaults to both");
    for value in ["direct", "indirect"] {
        assert!(doc.contains(&format!("name=\"layer\" value=\"{value}\"")), "layer radio {value}");
    }
    assert!(doc.contains("addEventListener(\"click\""), "click handler wires the nodes");

    // The embedded script parses: `node --check` over the extracted block
    // (`NODE` -> PATH -> named SKIP, the family probe law).
    let script = doc
        .split("<script>\n")
        .nth(1)
        .and_then(|rest| rest.split("\n</script>").next())
        .expect("the emitted document carries the inline script block");
    match node_binary() {
        Some(node) => {
            let script_path = fixture.path("viewer-script.js");
            std::fs::write(&script_path, script).expect("write extracted script");
            let check = Command::new(&node)
                .arg("--check")
                .arg(&script_path)
                .output()
                .expect("run node --check");
            assert!(
                check.status.success(),
                "emitted script must pass node --check:\n{}",
                stderr(&check)
            );
        }
        None => eprintln!("SKIP node-check: no node binary (NODE -> PATH -> SKIP, the family probe law)"),
    }
}

fn node_binary() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("NODE")
        && !path.trim().is_empty()
    {
        return Some(PathBuf::from(path));
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join("node"))
            .find(|candidate| candidate.is_file())
    })
}

/// The layer selector (scenario 05 semantics) gates the emitted families
/// at build time: `direct` draws no async rows, `indirect` no structural
/// ones, and the matching radio ships checked.
#[test]
fn layer_selection_gates_the_emitted_families() {
    for (selection, direct_rows, async_rows) in
        [("direct", 2, 0), ("indirect", 0, 1), ("both", 2, 1)]
    {
        let fixture = Fixture::new();
        fixture.write("scan.json", SCAN_HOSTILE);
        let output = fixture.run(&[
            "diagram",
            "--source",
            "scan",
            "scan.json",
            "--format",
            "viewer",
            "--layer",
            selection,
            "--output",
            "viewer.html",
        ]);
        assert!(output.status.success(), "--layer {selection} emits: {}", stderr(&output));
        let doc = fixture.read("viewer.html");
        assert_eq!(
            count(&doc, "data-layer=\"direct\""),
            direct_rows,
            "layer {selection}: direct-layer rows"
        );
        assert_eq!(
            count(&doc, "data-layer=\"async\""),
            async_rows,
            "layer {selection}: async rows"
        );
        assert!(
            doc.contains(&format!("name=\"layer\" value=\"{selection}\" checked")),
            "layer {selection}: the matching radio ships checked"
        );
    }
}

/// Determinism: the same artefact emits byte-identical documents (the
/// layout is a function of the inventory only).
#[test]
fn emit_is_deterministic() {
    let fixture = Fixture::new();
    fixture.write("scan.json", SCAN_HOSTILE);
    let first = emit(&fixture);
    let second = emit(&fixture);
    assert_eq!(first, second, "same input, same bytes");
}

// ---------------------------------------------------------------------------
// workplan viewer_usability scenario 01: the declared-deps class, the
// selection clear, the viewBox-only pan/zoom, border endpoints and the
// keyboard/ARIA affordances (the live-page review of the emitted document)
// ---------------------------------------------------------------------------

/// Artefact for the declared-edge join: `unit_manifests` legs in BOTH
/// spellings — a hyphen dep naming an underscore external, an underscore dep
/// naming a hyphen external, an exact match — beside the three legs that
/// must draw nothing: a member dep (a structural row already), a self dep,
/// and a dep naming no node at all.
const SCAN_DECLARED: &str = r#"{
  "schema_version": 1,
  "language": "rust",
  "units": [
    { "name": "app_bin", "kind": "bin", "path": "app" },
    { "name": "core_lib", "kind": "lib", "path": "core" }
  ],
  "edges": [ { "from": "app_bin", "to": "core_lib" } ],
  "external": [ "dash_dep", "under_dep", "plain", "with-hy" ],
  "unit_manifests": {
    "app_bin": {
      "publish": false,
      "dependencies": ["dash-dep", "core_lib", "app_bin"],
      "features": []
    },
    "core_lib": {
      "publish": false,
      "dependencies": ["under_dep", "plain", "with_hy", "unseen_crate"],
      "features": []
    }
  }
}
"#;

/// Done-when 1: one `declared` row per manifest leg that names an external
/// node — the union of the `unit_manifests` deps under the ONE crate-name
/// law (`depgraph::ledger_name_key`, both spellings normalized on both
/// sides) — with the legend row counting the same number and the
/// `declared deps` checkbox shipping checked (default ON).
#[test]
fn declared_edges_connect_units_to_externals() {
    let fixture = Fixture::new();
    fixture.write("scan.json", SCAN_DECLARED);
    let doc = emit(&fixture);
    for (from, to) in [
        ("app_bin", "dash_dep"),   // hyphen dep, underscore external
        ("core_lib", "under_dep"), // exact underscore match
        ("core_lib", "plain"),     // exact match
        ("core_lib", "with-hy"),   // underscore dep, hyphen external
    ] {
        assert!(
            doc.contains(&format!(
                "<line class=\"edge declared\" data-layer=\"declared\" data-from=\"{from}\" data-to=\"{to}\""
            )),
            "declared row {from} -> {to} present"
        );
    }
    assert_eq!(
        count(&doc, "class=\"edge declared\""),
        4,
        "member, self and absent-name legs draw nothing — only external-naming legs do"
    );
    assert_eq!(
        count(&doc, "data-layer=\"declared\""),
        4,
        "every declared row carries its layer marker"
    );
    assert!(
        doc.contains("declared — thin dotted manifest-declared dependency arrow (4)"),
        "legend row counts the declared class"
    );
    assert!(
        doc.contains("type=\"checkbox\" id=\"declared-deps\" checked"),
        "the declared deps checkbox ships checked (default ON)"
    );
    assert!(
        doc.contains(".edge.declared{stroke:#999;stroke-width:1;stroke-dasharray:1,3}"),
        "the stylesheet carries the distinct thin dotted grey class"
    );
    // Done-when 4: the declared fixture is deterministic too.
    assert_eq!(doc, emit(&fixture), "same input, same bytes");
}

/// Done-when 2: structural-edge endpoints stand at the nearest box borders.
/// Same-column boxes join the facing horizontal borders at the column centre
/// line (the reviewed-page defect: the unit stack drew right-side to
/// left-side); cross-column rows keep the shipped border bytes byte-for-byte.
#[test]
fn structural_edge_endpoints_stand_at_box_borders() {
    let fixture = Fixture::new();
    fixture.write("scan.json", SCAN_HOSTILE);
    let doc = emit(&fixture);
    // Units stand at x=24 rows y=40 and y=76: the downward unit -> unit row
    // runs bottom border to top border at the column centre x = 24 + 85.
    assert!(
        doc.contains("x1=\"109\" y1=\"66\" x2=\"109\" y2=\"76\""),
        "same-column edge joins the facing horizontal borders"
    );
    // The unit -> external row keeps the shipped facing-vertical-border bytes.
    assert!(
        doc.contains("x1=\"194\" y1=\"53\" x2=\"470\" y2=\"53\""),
        "cross-column edge keeps the shipped border geometry"
    );
    // The upward same-column async row: top border of the lower box to the
    // bottom border of the upper one.
    assert!(
        doc.contains("x1=\"109\" y1=\"76\" x2=\"109\" y2=\"66\""),
        "upward same-column edge joins the facing horizontal borders"
    );
}

/// Finding 5 + finding 2 at the emitted-bytes level: every drawn node is a
/// keyboard-reachable button with an aria-label, Enter/Space select like a
/// click, Escape and a canvas-background click clear the selection, and the
/// stylesheet ships a focus-visible rule. (The DOM behaviors are pinned at
/// the emitted-bytes level; the document ships no JS runtime cell beyond
/// `node --check`, per the scenario-06 precedent.)
#[test]
fn nodes_are_keyboard_reachable_and_selection_clears() {
    let fixture = Fixture::new();
    fixture.write("scan.json", SCAN_DECLARED);
    let doc = emit(&fixture);
    let node_rows = count(&doc, "<g class=\"node ");
    assert!(node_rows > 0, "nodes are drawn");
    assert_eq!(
        count(&doc, "tabindex=\"0\" role=\"button\""),
        node_rows,
        "every drawn node is keyboard-focusable and carries the button role"
    );
    assert!(
        doc.contains("aria-label=\"app_bin\""),
        "aria-label states the unit name"
    );
    assert!(doc.contains("\"Enter\""), "Enter selects like a click");
    assert!(doc.contains("\"Escape\""), "Escape clears the selection");
    assert!(
        doc.contains("graph.addEventListener(\"click\""),
        "a canvas-background click clears the selection"
    );
    assert!(doc.contains(":focus-visible"), "focus-visible styling ships");
}

/// Finding 3: pan/zoom lives client-side ONLY — the emitted bytes keep the
/// initial deterministic viewBox attribute, and the script moves nothing but
/// the `viewBox` attribute through the wheel and pointer handlers.
#[test]
fn pan_zoom_lives_client_side_only() {
    let fixture = Fixture::new();
    fixture.write("scan.json", SCAN_HOSTILE);
    let doc = emit(&fixture);
    assert!(
        doc.contains("<svg id=\"graph\" viewBox=\"0 0 664 "),
        "the emitted header keeps the initial deterministic viewBox"
    );
    assert!(
        doc.contains("setAttribute(\"viewBox\""),
        "pan/zoom adjusts the viewBox attribute client-side only"
    );
    assert!(doc.contains("\"wheel\""), "wheel zoom is wired");
    assert!(
        doc.contains("\"pointerdown\"") && doc.contains("\"pointermove\""),
        "pointer drag pan is wired"
    );
    // No zoom/pan state can move the bytes: the same artefact twice (the
    // determinism cells) and the header attribute above close the claim.
}

/// The escape posture holds for the declared rows' `data-from`/`data-to` and
/// the `aria-label` too: over the hostile fixture the raw spellings never
/// ship (the fixture adds one hostile external named by a manifest leg).
#[test]
fn declared_edges_survive_hostile_names() {
    let fixture = Fixture::new();
    let doc_hostile = {
        let mut value: serde_json::Value = serde_json::from_str(SCAN_HOSTILE).expect("fixture json");
        value["external"]
            .as_array_mut()
            .expect("external array")
            .push(serde_json::json!("an&ext"));
        value["unit_manifests"] = serde_json::json!({
            "a<b>c": { "publish": false, "dependencies": ["an&ext"], "features": [] }
        });
        let scan = serde_json::to_string(&value).expect("serialise artefact");
        fixture.write("scan.json", &scan);
        emit(&fixture)
    };
    assert_eq!(
        count(&doc_hostile, "class=\"edge declared\""),
        1,
        "the hostile-named leg draws exactly one declared row"
    );
    assert!(
        doc_hostile.contains("data-to=\"an&amp;ext\""),
        "the declared endpoint is escaped"
    );
    assert!(
        !doc_hostile.contains("an&ext\""),
        "the raw hostile spelling never ships in an attribute"
    );
    assert!(
        doc_hostile.contains("aria-label=\"a&lt;b&gt;c\""),
        "the aria-label is escaped like every other model string"
    );
}

/// The module tier rides the same inventory: under `--tiers modules` the
/// artefact's module endpoints appear as `module` nodes with a legend count.
/// (The scenario restates: the interactive population is the unit/module
/// tiers — the same tiers the ceilings starve.) The fixture is one crate
/// whose `gateway` module imports its `util` module — the shape the rust
/// driver derives a module edge from.
#[test]
fn module_tier_nodes_are_interactive() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod gateway;\npub mod util;\n");
    fixture.write("src/gateway.rs", "pub fn run() -> u32 {\n    crate::util::ping()\n}\n");
    fixture.write("src/util.rs", "pub fn ping() -> u32 {\n    1\n}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"api\"\nmatches = { units = [\"api\"] }\n",
    );
    let scan = fixture.run(&["scan", "--output", "scan.json"]);
    assert!(scan.status.success(), "scan: {}", stderr(&scan));
    let output = fixture.run(&[
        "diagram",
        "--source",
        "scan",
        "scan.json",
        "--format",
        "viewer",
        "--tiers",
        "modules",
        "--output",
        "viewer.html",
    ]);
    assert!(output.status.success(), "module-tier emit: {}", stderr(&output));
    let doc = fixture.read("viewer.html");
    assert!(doc.contains("class=\"node module\""), "module nodes are emitted:\n{doc}");
    assert!(doc.contains("module — module tier"), "legend states the module tier");
}

/// The viewer rides the scan source; `--source spec` refuses by name and
/// the territory flags join the whole-model-graph refusal.
#[test]
fn viewer_is_scan_source_only() {
    let fixture = Fixture::new();
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"m\"\nmatches = { units = [\"m\"] }\n",
    );
    let refusal = fixture.run(&["diagram", "--format", "viewer"]);
    assert_eq!(refusal.status.code(), Some(64), "spec-source viewer is a usage error");
    assert!(
        stderr(&refusal).contains("--format viewer rides the scan source"),
        "the refusal names the rule: {}",
        stderr(&refusal)
    );
    let territory = fixture.run(&["diagram", "--module", "m", "--format", "viewer"]);
    assert_eq!(territory.status.code(), Some(64), "territory viewer is a usage error");
    assert!(
        stderr(&territory).contains("--format viewer render one whole-model graph"),
        "the whole-model refusal names viewer: {}",
        stderr(&territory)
    );
}

// ---------------------------------------------------------------------------
// cell 2 — the zero-vendored structural pin over the payload tree
// ---------------------------------------------------------------------------

#[test]
fn payload_ships_no_js_and_no_third_party_identifiers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut js_assets: Vec<PathBuf> = Vec::new();
    let mut ident_hits: Vec<PathBuf> = Vec::new();
    // The forbidden class, named exactly as the scenario names it. The
    // scan is case-SENSITIVE: the identifiers are lowercase library
    // names, and an uppercase design-cite like "D3" is prose, not a
    // vendored byte.
    let third_party = ["cytoscape", "mermaid.min", "d3", "elk"];
    let mut visit = |path: &Path| {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".js") {
            js_assets.push(path.to_path_buf());
            return;
        }
        // The only exemption is THIS file: a pin that greps for a string
        // necessarily contains that string. No shipped surface is exempt.
        if name == "viewer_html.rs"
            && path.parent().and_then(|p| p.file_name()) == Some(std::ffi::OsStr::new("tests"))
        {
            return;
        }
        let Ok(text) = std::fs::read_to_string(path) else { return };
        let hit = third_party.iter().any(|p| match *p {
            "d3" | "elk" => word_hit(&text, p),
            _ => text.contains(p),
        });
        if hit {
            ident_hits.push(path.to_path_buf());
        }
    };
    walk(root, &mut visit);
    assert!(js_assets.is_empty(), "payload ships no standalone JS asset: {js_assets:?}");
    assert!(
        ident_hits.is_empty(),
        "no third-party library identifier in the payload tree: {ident_hits:?}"
    );
}

/// Word-boundary hit for the short identifiers (`d3`, `elk`) so hex digests
/// and unrelated words cannot trip the pin.
fn word_hit(haystack: &str, word: &str) -> bool {
    haystack.match_indices(word).any(|(at, _)| {
        let before_ok = !haystack[..at].ends_with(|c: char| c.is_alphanumeric());
        let after = at + word.len();
        let after_ok = haystack[after..].starts_with(|c: char| !c.is_alphanumeric());
        before_ok && after_ok
    })
}

fn walk(dir: &Path, visit: &mut impl FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // The npm package is node installers with their own marker and is
        // excluded from the Rust module scan (architecture.spec.toml); the
        // vendored-JS law is the law of the emitted viewer surface, not of
        // that channel's installer.
        if matches!(name, "target" | ".git" | "node_modules" | "npm") {
            continue;
        }
        if path.is_dir() {
            walk(&path, visit);
        } else {
            visit(&path);
        }
    }
}

// ---------------------------------------------------------------------------
// cell 4 — the limitation rows live in the binary's own help bytes
// ---------------------------------------------------------------------------

#[test]
fn help_names_the_gitea_no_pages_limitation_and_both_not_built_rows() {
    let fixture = Fixture::new();
    let help = fixture.run(&["diagram", "--help"]);
    assert!(help.status.success());
    let text = stdout(&help);
    assert!(
        text.contains("gitea has no Pages") && text.contains("28.x") && text.contains("404"),
        "the gitea-no-Pages limitation row (28.x, the 404 receipt) must stay greppable:\n{text}"
    );
    assert!(
        text.contains("`archspec serve` is the named future local channel, not built"),
        "the `serve` row is named, not built:\n{text}"
    );
    assert!(
        text.contains("repository configuration outside the payload"),
        "the gitea-side infra-hosting row is named, not built:\n{text}"
    );
    assert!(
        text.contains("Viewer law"),
        "the no-vendored-JS viewer law row rides the help surface:\n{text}"
    );
}

/// The weld for claim `viewer-interaction-controls` (workplan viewer_usability
/// scenario 01): the `Viewer interaction:` paragraph and its signature bytes
/// ride the `diagram --help` surface, so the guard states the registered
/// canonical bytes the registry welds against.
#[test]
fn help_states_the_viewer_interaction_canonical() {
    let fixture = Fixture::new();
    let help = fixture.run(&["diagram", "--help"]);
    assert!(help.status.success());
    let text = stdout(&help);
    assert!(
        text.contains("Viewer interaction: the emitted viewer document draws one thin dotted grey declared edge per manifest-declared dependency leg from a unit to an external"),
        "the Viewer interaction paragraph opens the canonical row:\n{text}"
    );
    assert!(
        text.contains("a checkbox named declared deps toggles that class independent of the layer radios (default on)"),
        "declared deps toggles that class — the checkbox row is greppable:\n{text}"
    );
    assert!(
        text.contains("a click on the canvas background or the Escape key clears the selection"),
        "Escape key clears the selection — the clear row is greppable:\n{text}"
    );
    assert!(
        text.contains("a pointer drag pans the canvas through its viewBox only, never the emitted bytes"),
        "the pan/zoom row states the viewBox only law:\n{text}"
    );
    assert!(
        text.contains("every node is keyboard-focusable and selects on Enter or Space"),
        "the keyboard row is greppable:\n{text}"
    );
}

// ---------------------------------------------------------------------------
// cell 5 — the Pages channel: shape + the publish gate's own pattern list
// ---------------------------------------------------------------------------

fn pages_workflow() -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(".github")
        .join("workflows")
        .join("pages.yml");
    std::fs::read_to_string(path).ok()
}

#[test]
fn pages_channel_ships_and_references_only_the_viewer_artifact() {
    let text = pages_workflow().expect("the payload ships .github/workflows/pages.yml");
    assert!(text.contains("viewer.html"), "the channel references the viewer document");
    assert!(text.contains("actions/upload-pages-artifact"), "one Pages artifact upload");
    assert!(text.contains("actions/deploy-pages"), "the Pages deploy action");
    assert!(text.contains("path: _site"), "the artifact holds the emitted file only");
    for forbidden in [
        "actions/upload-artifact",
        "actions/download-artifact",
        "gh release",
        "secrets.",
    ] {
        assert!(!text.contains(forbidden), "the channel references no other artifact or secret: {forbidden}");
    }
    assert!(
        text.contains("id-token: write") && text.contains("pages: write"),
        "least-privilege deploy permissions (the Trusted-Publishing posture)"
    );
}

/// The leak-guard arm: the publish workflow's LEAK_PATTERNS list is READ
/// from the checkout's publish workflows (by their declared pattern list,
/// never by a baked-in file name — a payload file that spelled the name
/// would itself trip the guard it pins) and must grep ZERO over the Pages
/// channel. The publish job scans every assembled file with the same list,
/// so this cell pins the row the gate will print green; absent checkout
/// context prints one named SKIP.
#[test]
fn leak_guard_list_is_clean_over_the_pages_channel() {
    let workflows = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .join(".gitea")
        .join("workflows");
    let mut patterns: Option<String> = None;
    if let Ok(entries) = std::fs::read_dir(&workflows) {
        for entry in entries.flatten() {
            let Ok(text) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            if let Some(line) = text.lines().find(|l| l.contains("LEAK_PATTERNS:"))
                && let Some(quoted) = line.split('\'').nth(1)
            {
                patterns = Some(quoted.to_string());
                break;
            }
        }
    }
    let Some(patterns) = patterns else {
        eprintln!("SKIP leak-guard-arm: no publish workflow with a LEAK_PATTERNS list (standalone package context)");
        return;
    };
    let channel = pages_workflow().expect("the payload ships .github/workflows/pages.yml");
    let staged = std::env::temp_dir().join("vp206-pages-leak-scan.yml");
    std::fs::write(&staged, &channel).expect("stage channel bytes for the scan");
    let hit = Command::new("grep")
        .args(["-inE", &patterns])
        .arg(&staged)
        .output()
        .expect("run grep");
    assert!(
        hit.status.code() == Some(1) && hit.stdout.is_empty(),
        "the leak guard must be clean over the Pages channel (UNCHANGED patterns):\n{}",
        String::from_utf8_lossy(&hit.stdout)
    );
}

// ---------------------------------------------------------------------------
// cell 3 — corpus probe: external and documented; armed only by env
// ---------------------------------------------------------------------------

#[test]
fn corpus_probe_shape() {
    let Some(corpus) = std::env::var_os("ARCHSPEC_VIEWER_CORPUS") else {
        eprintln!(
            "SKIP corpus-probe: external probe leg — run rust-arch-test-kit/internal/viewer-corpus-probe.sh \
             (git-archive scratch, status-0 law); set ARCHSPEC_VIEWER_CORPUS to the corpus tree to arm this cell"
        );
        return;
    };
    let corpus = PathBuf::from(corpus);
    let scratch = tempfile::tempdir().expect("scratch dir");
    let scan_path = scratch.path().join("scan.json");
    let scan = Command::new(env!("CARGO_BIN_EXE_archspec"))
        .args(["scan", "--output"])
        .arg(&scan_path)
        .arg(&corpus)
        .output()
        .expect("scan the corpus tree");
    assert!(scan.status.success(), "corpus scan: {}", stderr(&scan));
    let viewer = scratch.path().join("viewer.html");
    let emit = Command::new(env!("CARGO_BIN_EXE_archspec"))
        .args(["diagram", "--source", "scan"])
        .arg(&scan_path)
        .args(["--format", "viewer", "--output"])
        .arg(&viewer)
        .output()
        .expect("emit the corpus viewer");
    assert!(emit.status.success(), "corpus emit: {}", stderr(&emit));
    let doc = std::fs::read_to_string(&viewer).expect("read corpus viewer");
    let units = count(&doc, "class=\"node unit\"");
    let externals = count(&doc, "class=\"node external\"");
    let direct = count(&doc, "data-layer=\"direct\"");
    let async_rows = count(&doc, "data-layer=\"async\"");
    println!(
        "corpus-probe: units={units} external={externals} direct={direct} async={async_rows} bytes={}",
        doc.len()
    );
    assert!(units >= 15, "the unit tier populates the DOM inventory");
    assert!(externals >= 20, "the external cluster populates the DOM inventory");
    assert!(direct >= 15, "the direct layer populates the DOM inventory");
    assert!(doc.len() >= 10_000, "the file rides the ~15 KB + data class");
}
