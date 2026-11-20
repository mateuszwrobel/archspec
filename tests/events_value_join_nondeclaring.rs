//! Non-declaring law: byte identity (workplan events_value_join scenario 08).
//!
//! The law (ADR-069 4.2 arm 1, decision-check F6/F-W1): a tree declaring
//! NOTHING on any face of the fact-events slice — no `[[pattern_pair]]`, no
//! `[[canonical_producer]]`, no `[[shared_constant]]` — serializes BYTE-IDENTICAL
//! bytes to the previous release binary on every verb and every surface. The
//! not-declared arm is ONE early-exit code path at the declaration gate
//! (`events::declarations_active`, consulted at the single site in
//! `archspec/scan.rs`), never per-verb special-casing.
//!
//! The pin vehicle: `tests/fixtures/nondeclaring/<tree>/<verb>.{stdout,stderr,rc}`
//! are the bytes recorded ONCE from the previous release binary (0.6.112,
//! pre-slice commit `c757fedb048b45ecb627ae7bdde73c69bf924698`); the fixture
//! trees themselves are pure recipes below, so the recorded bytes are a pure
//! function of recipe plus binary (the `async_model_surface` digest law,
//! extended from one verb to the whole matrix). Regeneration is a NAMED,
//! DATED refresh:
//! `ARCHSPEC_GOLDEN=1 cargo test --test events_value_join_nondeclaring`
//! — a refresh must cite the releasing version and date in the landing.
//!
//! Static-reference exclusion (named, not silent): the `spec` and
//! `spec --schema` surfaces print the shipped declaration-GRAMMAR reference,
//! which scenarios 02/03 extended as part of adding the tables — that delta
//! is documentation of the grammar, never a derived output of a tree. The
//! exclusion stays honest because
//! `the_static_spec_surface_is_tree_independent` below pins that the spec
//! bytes cannot depend on any tree at all, so a future tree-dependent arm
//! there reddens a cell instead of hiding behind this comment.

mod common;

use common::{Fixture, stderr, stdout};

/// The verbs, in run order: `update` writes the seed spec the later `verify`
/// and `report` reads, so the order is part of the recorded surface.
const VERBS: &[(&str, &[&str])] = &[
    ("scan", &["scan"]),
    ("scan-pretty", &["scan", "--pretty", "--output", "-"]),
    ("update", &["update"]),
    ("verify", &["verify"]),
    ("verify-json", &["verify", "--json"]),
    ("report", &["report"]),
    ("report-json", &["report", "--format", "json"]),
    ("report-md", &["report", "--format", "markdown"]),
    ("diagram", &["diagram"]),
    ("depgraph-modules", &["depgraph", "modules"]),
    ("inspect", &["inspect"]),
];

/// The report-only census family the slice added (scenarios 04–06 plus the
/// literal-site census): every row prefix that may speak ONLY when a
/// declaration arms the events face. On a non-declaring tree every one of
/// them must speak ZERO times.
const REPORT_ONLY_FAMILIES: &[&str] = &[
    "dispatch-blinded-literals",
    "ambiguous-leg",
    "one-sided-leg",
    "shared-constant",
    "unfunded-shape",
    "unoriented",
    "driver-gap",
];

// ---------------------------------------------------------------------------
// Fixture recipes — the tree bytes are pinned here, so the recorded bytes are
// a pure function of recipe plus binary (the async_model_surface law).
// ---------------------------------------------------------------------------

/// Non-declaring rust workspace A: the `async_model_surface` D8 digest tree
/// (two members, one real manifest reference, no declarations anywhere).
fn plain_workspace(fixture: &Fixture) {
    fixture.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"api\", \"core\"]\n",
    );
    fixture.write(
        "core/Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "core/src/lib.rs",
        "pub struct OrderPlaced;\n\npub fn publish() -> OrderPlaced { OrderPlaced }\n",
    );
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore = { path = \"../core\" }\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "use core::OrderPlaced;\n\npub fn handle(event: OrderPlaced) {\n    let _ = event;\n}\n",
    );
}

/// Non-declaring rust workspace B: three members, an external dependency and
/// a submodule — a wider graph for the render surfaces.
fn rich_workspace(fixture: &Fixture) {
    fixture.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"gateway\", \"core\", \"store\"]\n",
    );
    fixture.write(
        "gateway/Cargo.toml",
        "[package]\nname = \"gateway\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore = { path = \"../core\" }\nserde = \"1\"\n",
    );
    fixture.write("gateway/src/lib.rs", "pub mod http;\n");
    fixture.write("gateway/src/http.rs", "pub fn serve() {}\n");
    fixture.write(
        "core/Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("core/src/lib.rs", "pub fn handle() {}\n");
    fixture.write(
        "store/Cargo.toml",
        "[package]\nname = \"store\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("store/src/lib.rs", "pub fn read() {}\n");
}

fn fixtures_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/nondeclaring")
}

fn record_mode() -> bool {
    std::env::var("ARCHSPEC_GOLDEN")
        .map(|v| v != "0")
        .unwrap_or(false)
}

/// Run the whole verb matrix on a fresh fixture and compare every surface
/// (stdout, stderr, rc, and the bytes `update` writes) against the recorded
/// base bytes; in record mode the files are rewritten instead.
fn sweep(fixture: &Fixture, tree: &str) {
    let out = fixtures_root().join(tree);
    std::fs::create_dir_all(&out).expect("fixture record dir");
    for (name, args) in VERBS {
        let run = fixture.run(args);
        check(&out.join(format!("{name}.stdout")), &run.stdout, name, tree);
        check(
            &out.join(format!("{name}.stderr")),
            &run.stderr,
            name,
            tree,
        );
        let rc = format!("{}\n", run.status.code().unwrap_or(-1));
        check(
            &out.join(format!("{name}.rc")),
            rc.as_bytes(),
            name,
            tree,
        );
        if *name == "update" {
            let seed = std::fs::read(fixture.path("architecture.spec.toml")).expect("seed spec");
            check(&out.join("update.written.toml"), &seed, name, tree);
        }
    }
}

fn check(path: &std::path::Path, got: &[u8], verb: &str, tree: &str) {
    if record_mode() {
        std::fs::write(path, got).expect("record");
        eprintln!("ARCHSPEC_GOLDEN: recorded {}", path.display());
        return;
    }
    let want = std::fs::read(path).unwrap_or_else(|e| {
        panic!(
            "{tree}/{verb}: recorded base bytes missing at {} ({e}) — \
             refresh with ARCHSPEC_GOLDEN=1 cargo test --test events_value_join_nondeclaring",
            path.display()
        )
    });
    assert_eq!(
        want,
        got,
        "{tree}/{verb}: surface bytes moved against the recorded base bytes at {} \
         ({} base bytes vs {} fresh bytes) — the non-declaring byte-identity law \
         (ADR-069 4.2 arm 1) is red; a deliberate refresh is a named, dated \
         ARCHSPEC_GOLDEN=1 re-record cited in the landing",
        path.display(),
        want.len(),
        got.len()
    );
}

// ---------------------------------------------------------------------------
// Cells
// ---------------------------------------------------------------------------

/// (a) The whole-verb byte-identity sweep on the D8 digest tree.
#[test]
fn every_verb_surface_is_byte_identical_on_the_plain_tree() {
    let fixture = Fixture::new();
    plain_workspace(&fixture);
    sweep(&fixture, "plain");
}

/// (a) The same sweep on the wider graph tree.
#[test]
fn every_verb_surface_is_byte_identical_on_the_rich_tree() {
    let fixture = Fixture::new();
    rich_workspace(&fixture);
    sweep(&fixture, "rich");
}

/// (b) Skip-when-empty on the scan face: no new collection serializes on a
/// non-declaring tree — `async_edges` and `literal_sites` carry no member,
/// the transient feeds never serialize at all (the D8 law extended, 4.2
/// arm 1; the model-level cells live in `src/model.rs`).
#[test]
fn the_new_collections_serialize_nothing_on_a_non_declaring_tree() {
    for (name, build) in [
        ("plain", plain_workspace as fn(&Fixture)),
        ("rich", rich_workspace as fn(&Fixture)),
    ] {
        let fixture = Fixture::new();
        build(&fixture);
        let run = fixture.run(&["scan"]);
        assert!(run.status.success(), "{name} scan failed: {}", stderr(&run));
        let json: serde_json::Value =
            serde_json::from_str(&stdout(&run)).expect("scan answers model JSON");
        for key in ["async_edges", "literal_sites", "literal_candidates", "report_only_lines"] {
            assert!(
                json.get(key).is_none(),
                "{name}: the non-declaring model JSON carries {key:?} — absence is the contract"
            );
        }
        let report = {
        // `report` adjudicates against the spec the `update` seed writes —
        // same sequencing the sweep matrix uses, no other meaning.
        fixture.run(&["update"]);
        fixture.run(&["report", "--format", "json"])
    };
        assert!(
            report.status.success(),
            "{name} report json failed: {}",
            stderr(&report)
        );
        let report: serde_json::Value =
            serde_json::from_str(&stdout(&report)).expect("report answers JSON");
        assert!(
            report.get("declaration_census").is_none(),
            "{name}: the non-declaring report carries a declaration_census member"
        );
    }
}

/// (b) The report-only census family speaks ZERO times on a non-declaring
/// run: not one slice row prefix reaches stderr on any verb (never-silent
/// becomes never-silent-because-armed: an un-armed face states nothing).
#[test]
fn the_report_only_census_family_speaks_zero_times() {
    for (name, build) in [
        ("plain", plain_workspace as fn(&Fixture)),
        ("rich", rich_workspace as fn(&Fixture)),
    ] {
        let fixture = Fixture::new();
        build(&fixture);
        for (verb, args) in VERBS {
            let run = fixture.run(args);
            for line in stderr(&run).lines() {
                for family in REPORT_ONLY_FAMILIES {
                    assert!(
                        !line.starts_with(family),
                        "{name}/{verb}: the report-only row {family:?} spoke on a \
                         non-declaring tree: {line}"
                    );
                }
            }
        }
    }
}

/// (a) Fresh-vs-base parity: on a fresh tree the whole derived matrix answers
/// byte-identically against a resolvable base binary. A base binary resolves
/// as `ARCHSPEC` -> `archspec` on PATH -> named SKIP (the global-install
/// resolution rule, the scenario-01/06 cell shape). The committed fixtures
/// above carry the law when nothing resolves.
#[test]
fn derived_surfaces_diff_empty_against_the_base_binary() {
    let Some(base) = base_binary() else {
        eprintln!("base-parity: SKIP — no base binary (ARCHSPEC unset, none on PATH)");
        return;
    };
    for (name, build) in [
        ("plain", plain_workspace as fn(&Fixture)),
        ("rich", rich_workspace as fn(&Fixture)),
    ] {
        // TWO identically-recooked fixtures, one per binary: the matrix mutates
        // the tree (`update` writes the seed once and refuses to clobber it),
        // so each binary must face its own fresh tree in the same order.
        let fresh_tree = Fixture::new();
        build(&fresh_tree);
        let base_tree = Fixture::new();
        build(&base_tree);
        for (verb, args) in VERBS {
            let fresh = fresh_tree.run(args);
            let previous = std::process::Command::new(&base)
                .args(args.iter().copied())
                .current_dir(&base_tree.root)
                .output()
                .expect("run base binary");
            assert_eq!(
                previous.status.code(),
                fresh.status.code(),
                "{name}/{verb}: rc moved against the base binary"
            );
            assert_eq!(
                previous.stdout, fresh.stdout,
                "{name}/{verb}: stdout moved against the base binary"
            );
            assert_eq!(
                stderr_of(&previous),
                stderr(&fresh),
                "{name}/{verb}: stderr moved against the base binary"
            );
        }
    }
}

fn stderr_of(output: &std::process::Output) -> String {
    let text = String::from_utf8_lossy(&output.stderr).into_owned();
    // The version footer is the only legal difference between two binaries.
    for version in [env!("CARGO_PKG_VERSION"), "0.6.112"] {
        let footer = format!("note: archspec {version}\n");
        if let Some(stripped) = text.strip_suffix(&footer) {
            return stripped.to_string();
        }
    }
    text
}

fn base_binary() -> Option<String> {
    if let Some(path) = std::env::var("ARCHSPEC").ok().filter(|p| !p.is_empty()) {
        return Some(path);
    }
    let probe = std::process::Command::new("archspec")
        .arg("--version")
        .output()
        .ok()?;
    probe.status.success().then(|| "archspec".to_string())
}

/// The named exclusion stays honest: the `spec` and `spec --schema` surfaces
/// are static grammar reference — the SAME bytes on both fixture trees and
/// on an empty directory. A tree-dependent arm there is impossible today and
/// reddens this cell if it ever appears, so the exclusion above cannot hide
/// a derivation regression.
#[test]
fn the_static_spec_surface_is_tree_independent() {
    let trees: [fn(&Fixture); 3] = [|_| {}, plain_workspace, rich_workspace];
    for args in [&["spec"][..], &["spec", "--schema"][..]] {
        let mut renders = Vec::new();
        for build in trees {
            let fixture = Fixture::new();
            build(&fixture);
            let run = fixture.run(args);
            assert!(run.status.success(), "spec failed: {}", stderr(&run));
            renders.push(stdout(&run));
        }
        assert_eq!(renders[0], renders[1], "{args:?} bytes moved between trees");
        assert_eq!(renders[1], renders[2], "{args:?} bytes moved between trees");
    }
}

/// (c) The structural cell: the not-declared arm is ONE named early-exit
/// site. Source anchors (the `events_value_join_patterns` census shape):
/// the gate arm `events::declarations_active` is defined exactly once, is
/// called exactly once on the whole derived face (the `archspec/scan.rs`
/// seam — the join, the attribution census and the blindspots projection all
/// run INSIDE that one block), no site re-derives the emptiness expression,
/// and the drivers carry only the per-language candidate-FEED seam (scenario
/// 03 F8), never a second gate.
#[test]
fn the_declaration_gate_is_one_named_site() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gate = manifest.join("src/archspec/events/mod.rs");
    let gate_text = std::fs::read_to_string(&gate).expect("read events/mod.rs");
    assert_eq!(
        gate_text.matches("pub fn declarations_active").count(),
        1,
        "the gate arm must be defined exactly once"
    );
    assert_eq!(
        gate_text.matches("patterns.is_empty()").count(),
        1,
        "the emptiness expression lives only in the gate body"
    );

    let mut definitions = 0;
    let mut calls = 0;
    let mut redrivations = 0;
    let mut feed_seams = 0;
    let mut feed_files: Vec<String> = Vec::new();
    for file in rust_sources(&manifest.join("src")) {
        let text = std::fs::read_to_string(&file).expect("read source");
        definitions += text.matches("pub fn declarations_active").count();
        calls += text.matches("events::declarations_active(").count();
        redrivations += text.matches("declared.patterns.is_empty").count();
        if file.starts_with(manifest.join("src/scan")) {
            let hits = text.matches("pattern.language ==").count();
            feed_seams += hits;
            if hits > 0 {
                feed_files.push(
                    file.strip_prefix(&manifest)
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| file.display().to_string()),
                );
            }
        }
    }
    assert_eq!(definitions, 1, "the gate arm is defined exactly once");
    assert_eq!(
        calls, 1,
        "the gate arm is consulted at exactly ONE site on the derived face"
    );
    assert_eq!(
        redrivations, 0,
        "no site may re-derive the declaration-emptiness expression (F-W1)"
    );
    feed_files.sort();
    assert_eq!(
        feed_seams, 5,
        "the candidate-feed language seam exists once per funded driver \
         (workplan events_value_join scenario 09 funded go and python)"
    );
    assert_eq!(
        feed_files,
        vec![
            "src/scan/go/mod.rs",
            "src/scan/kotlin/mod.rs",
            "src/scan/python/mod.rs",
            "src/scan/rust.rs",
            "src/scan/ts/mod.rs",
        ],
        "the candidate-feed seam must live in the five funded drivers only \
         (rust, ts, go, python, kotlin — scenario 09)"
    );
}

fn rust_sources(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).expect("read source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            files.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    files.sort();
    files
}

/// (d) The reclassification's factual anchor (surface-activation record §7
/// cell 5, landed by workplan visibility_phase2 scenario 04): the six
/// `event-declaration-derivation` rows read csharp `granular`, rust, go, ts,
/// python and kotlin `not-emitted`. The not-declared arm's silence is BY
/// DESIGN for EVERY capability state (ADR-069 §4.2: three arms, no fourth),
/// and the capability row is not a trigger — this cell is what makes that
/// law falsifiable: a flip of ANY row trips it, and the trip is the named
/// re-trigger of workplan visibility_phase2 §Falsifiable re-triggers item 1
/// (it re-opens the row question through an ADR-069 amendment plus its own
/// decision-check — it never rewrites this expectation silently). The rows
/// are read through their machine projection `archspec capability matrix`,
/// the same shape `tests/capability_prose.rs` parses.
#[test]
fn the_event_declaration_derivation_rows_are_the_reclassification_anchor() {
    let fixture = Fixture::new();
    let run = fixture.run(&["capability", "matrix"]);
    assert!(
        run.status.success(),
        "`capability matrix` must answer: {}",
        stderr(&run)
    );
    let mut rows: Vec<(String, String)> = Vec::new();
    for line in stdout(&run).lines() {
        let mut words = line.split_whitespace();
        if words.next() != Some("capability") {
            continue;
        }
        let (Some(language), Some(fact)) = (words.next(), words.next()) else {
            panic!("malformed matrix row: {line}");
        };
        if fact == "event-declaration-derivation" {
            let emission = line.splitn(4, ' ').nth(3).unwrap_or("").to_string();
            rows.push((language.to_string(), emission));
        }
    }
    rows.sort();
    let expected: Vec<(String, String)> = [
        ("csharp", "granular"),
        ("go", "not-emitted"),
        ("kotlin", "not-emitted"),
        ("python", "not-emitted"),
        ("rust", "not-emitted"),
        ("ts", "not-emitted"),
    ]
    .into_iter()
    .map(|(language, emission)| (language.to_string(), emission.to_string()))
    .collect();
    assert_eq!(
        rows, expected,
        "the six event-declaration-derivation rows are the reclassification's \
         factual anchor (surface-activation record §5): the not-declared arm \
         stays silent BY DESIGN for EVERY capability state, so a flip here is \
         the workplan visibility_phase2 §Falsifiable re-triggers item 1 firing \
         — an ADR-069 amendment plus its own decision-check decides the row \
         question, never a silent edit of this cell"
    );
}
