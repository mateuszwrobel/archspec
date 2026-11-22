//! Workplan level_views scenario 06: the `--layer` × `--level` composition
//! weld and its stderr posture.
//!
//! The compose law: `--layer` semantics stand UNCHANGED and the layer select
//! happens on the PROJECTED rows, never on a second fold path — so one
//! projection feeds every composed pair and the accounting ledger stays
//! byte-identical across the three layer values at a level (`accounting
//! exact: <d> drawn + <s> self = <r> raw` at every pair, MAJOR-4: a folded
//! async row is self-accounted, never dropped). The flag-absent render
//! replays the shipped bytes and stays `--check` green with the descriptive
//! rows present (MINOR-11 + M11 residue: the `descriptive only` note and the
//! `completeness restated` rows ride STDERR, never the render body). A
//! bucket cycle is descriptive: the render draws it and `verify` gates the
//! module tier — the exit stays 0 and no verdict byte appears. The weld
//! cells read the claim registry at run time (the canonical enumeration
//! surface), resolve every VERB+FLAG pair in every level refusal row against
//! the live help surfaces (the §Round-3/4 parity law applied fleet-wide to
//! all five landings' rows), pin the capability matrix against the new flags
//! (no matrix row names them — the flag-level claims live in the registry
//! and help), and grep the tree for restated ceiling numbers: the facts stay
//! parsed from `archspec diagram --help`, and only the owner surfaces spell
//! the constants.

mod common;
#[allow(dead_code)]
mod shared;

use common::{Fixture, stderr, stdout};

/// The three `--layer` values and the five selectable `--level` values —
/// every composed pair the surface accepts.
const LAYERS: [&str; 3] = ["direct", "indirect", "both"];
const LEVELS: [&str; 5] = ["unit", "dir:1", "dir:2", "module", "file"];

/// Run the built binary in the fixture dir and return (code, out, err).
fn run(fixture: &Fixture, args: &[&str]) -> (i32, String, String) {
    let output = fixture.run(args);
    (
        output.status.code().unwrap_or(-1),
        stdout(&output),
        stderr(&output),
    )
}

/// The composition fixture: structural cross rows, an intra-bucket row, and
/// two async rows — one between units whose paths share the first directory
/// (folds to ONE bucket at `dir:1`) and one self-pair (folds at EVERY level)
/// — so each composed pair exercises cross, self, drawn-async and
/// folded-async accounting at once. Raw rows per fold: 5, level-invariant.
fn composed_artefact() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"alpha","kind":"lib","path":"pkg/alpha"},{"name":"beta","kind":"lib","path":"pkg/beta"},{"name":"gamma","kind":"lib","path":"svc/gamma"}],"edges":[{"from":"alpha","to":"beta"},{"from":"gamma","to":"alpha"}],"module_edges":[{"unit":"alpha","from":"pkg/alpha/gateway","to":"pkg/beta/store","symbols":[]},{"unit":"gamma","from":"svc/gamma/store","to":"pkg/alpha/gateway","symbols":[]},{"unit":"alpha","from":"pkg/alpha/gateway","to":"pkg/alpha/gateway","symbols":[]}],"async_edges":[{"from":"alpha","to":"beta","event":"ping::PingEvent","kind":"async"},{"from":"beta","to":"beta","event":"loop::LoopEvent","kind":"async"}]}"#,
    );
    fixture
}

/// The ledger row of an accounting block: (drawn, self, raw, level, row).
fn ledger(err: &str) -> (usize, usize, usize, String, String) {
    let row = err
        .lines()
        .find(|line| line.starts_with("accounting exact: "))
        .unwrap_or_else(|| panic!("the ledger row is missing from:\n{err}"));
    let middle = row
        .trim_start_matches("accounting exact: ")
        .split(" raw ")
        .next()
        .expect("the raw clause");
    let mut numbers = middle
        .split([' ', '+', '='])
        .filter_map(|token| token.parse::<usize>().ok());
    let parsed = (
        numbers.next().expect("drawn"),
        numbers.next().expect("self"),
        numbers.next().expect("raw"),
    );
    let level = row
        .rsplit(" at level ")
        .next()
        .expect("the level clause")
        .to_string();
    assert_eq!(
        parsed.0 + parsed.1,
        parsed.2,
        "the ledger must be exact — drawn + self = raw: {row}"
    );
    (parsed.0, parsed.1, parsed.2, level, row.to_string())
}

/// The folded-row count of the shipped fold-away note (`0` when no row
/// folded away and the note is absent).
fn folded_count(err: &str) -> usize {
    match err
        .lines()
        .find(|line| line.starts_with("note:") && line.contains("async rows folded away"))
    {
        Some(line) => line
            .split(' ')
            .nth(1)
            .and_then(|count| count.parse::<usize>().ok())
            .unwrap_or_else(|| panic!("the fold-away note names no count: {line}")),
        None => 0,
    }
}

/// The solid counted-edge rows of a mermaid level render (the structural
/// family: `-->|<n> rows|`).
fn structural_rows(body: &str) -> Vec<String> {
    body.lines()
        .filter(|line| line.contains("-->|"))
        .map(str::to_string)
        .collect()
}

/// The dashed event-labelled rows of a mermaid level render (the async
/// family: `-.->|<event>|`).
fn async_rows(body: &str) -> Vec<String> {
    body.lines()
        .filter(|line| line.contains("-.->|"))
        .map(str::to_string)
        .collect()
}

/// The SOLID edge rows of any mermaid render — the whole-model grammar
/// spells them `a --> b`, a level fold spells them `a -->|<n> rows| b`, and
/// neither contains the dashed `-.->` grammar.
fn solid_edges(body: &str) -> Vec<String> {
    body.lines()
        .filter(|line| line.contains(" -->"))
        .map(str::to_string)
        .collect()
}

/// The DASHED edge rows of any mermaid render (both `-.->|<event>|` and the
/// plain dashed grammar).
fn dashed_edges(body: &str) -> Vec<String> {
    body.lines()
        .filter(|line| line.contains("-.->"))
        .map(str::to_string)
        .collect()
}

fn render(fixture: &Fixture, layer: &str, level: &str) -> (i32, String, String) {
    run(
        fixture,
        &[
            "diagram",
            "--source",
            "scan",
            "model.json",
            "--layer",
            layer,
            "--level",
            level,
        ],
    )
}

/// Cell 1 (done-when 1) — the composed-pair matrix: every `--layer` value ×
/// every `--level` value renders rc 0 with the ledger exact, the ledger row
/// is BYTE-IDENTICAL across the layer values at a level (one fold feeds the
/// layer select — no second fold path), the raw total is level-invariant,
/// and the layer families stay exact on the projected rows: `direct` never
/// draws an async row, `indirect` never draws a structural row, and `both`
/// draws EXACTLY the union of the two families — every row once.
#[test]
fn ledger_and_layer_families_are_exact_at_every_composed_pair() {
    let fixture = composed_artefact();
    let mut raw_totals: Vec<usize> = Vec::new();
    for level in LEVELS {
        let mut rows_by_layer: Vec<(String, String, String)> = Vec::new();
        for layer in LAYERS {
            let (code, body, err) = render(&fixture, layer, level);
            assert_eq!(
                code, 0,
                "the composed pair (--layer {layer}, --level {level}) must exit 0: {err}"
            );
            let (_drawn, _self_rows, raw, stated_level, row) = ledger(&err);
            assert_eq!(
                stated_level, level,
                "the ledger names the requested level at (--layer {layer}): {row}"
            );
            raw_totals.push(raw);
            rows_by_layer.push((layer.to_string(), body, row));
            assert!(
                err.contains("note: descriptive only — verify gates the module tier"),
                "the descriptive-only row rides stderr at (--layer {layer}, --level {level}):\n{err}"
            );
        }
        let first_layer = rows_by_layer[0].0.clone();
        let first_row = rows_by_layer[0].2.clone();
        for (layer, body, row) in &rows_by_layer {
            assert_eq!(
                row, &first_row,
                "the ledger row drifted with the layer at level {level}: {layer} spells {row}, \
                 {first_layer} spells {first_row}"
            );
            if layer == "direct" {
                assert!(
                    async_rows(body).is_empty(),
                    "--layer direct must draw no async row at level {level}:\n{body}"
                );
            }
            if layer == "indirect" {
                assert!(
                    structural_rows(body).is_empty(),
                    "--layer indirect must draw no structural row at level {level}:\n{body}"
                );
            }
        }
        let direct = &rows_by_layer[0];
        let indirect = &rows_by_layer[1];
        let both = &rows_by_layer[2];
        assert_eq!(
            structural_rows(&both.1),
            structural_rows(&direct.1),
            "at level {level} the `both` structural family must equal the `direct` family"
        );
        assert_eq!(
            async_rows(&both.1),
            async_rows(&indirect.1),
            "at level {level} the `both` async family must equal the `indirect` family"
        );
        let edge_rows: Vec<String> = both
            .1
            .lines()
            .filter(|line| line.contains("-->|") || line.contains("-.->|"))
            .map(str::to_string)
            .collect();
        let mut union = structural_rows(&both.1);
        union.extend(async_rows(&both.1));
        assert_eq!(
            edge_rows.len(),
            union.len(),
            "at level {level} `both` draws every edge row in exactly one family — no row \
             counts twice:\n{both:?}"
        );
    }
    assert!(
        raw_totals.iter().all(|raw| *raw == raw_totals[0]),
        "the raw-row total is the fold's, level- and layer-invariant: {raw_totals:?}"
    );
    assert_eq!(
        raw_totals[0], 5,
        "the fixture declares five rows (three module-edge rows + two async rows) and every \
         fold accounts all five: {raw_totals:?}"
    );
    // The matrix is not vacuous: at the module pair both families draw.
    let (code, body, _err) = render(&fixture, "both", "module");
    assert_eq!(code, 0);
    assert!(
        !structural_rows(&body).is_empty() && !async_rows(&body).is_empty(),
        "the composed matrix must exercise a pair where both families draw:\n{body}"
    );
}

/// Cell 2 (MAJOR-4 composed, the extended pinned cell) — the folded async
/// rows land SELF-ACCOUNTED under `--layer both` at the non-module levels:
/// at every level an async row either draws its arrow or rides the fold-away
/// note, never both and never neither (the two declared rows are always
/// accounted), and at `dir:1` — where the cross-unit pair folds onto ONE
/// bucket — the folded rows ride that bucket's self tally and the ledger
/// keeps `drawn + self = raw` exact. The pinned `tests/async_render.rs`
/// bytes are untouched; this cell extends the coverage to the non-module
/// folds.
#[test]
fn folded_async_rows_land_self_accounted_under_layer_both_at_non_module_levels() {
    let fixture = composed_artefact();
    for level in ["unit", "dir:1", "dir:2", "file"] {
        let (code, body_both, err) = render(&fixture, "both", level);
        assert_eq!(code, 0, "the level render must exit 0: {err}");
        let (code_ind, body_ind, _err_ind) = render(&fixture, "indirect", level);
        assert_eq!(code_ind, 0);
        let drawn_async = async_rows(&body_ind).len();
        let folded = folded_count(&err);
        assert_eq!(
            drawn_async + folded,
            2,
            "at level {level} every declared async row is either drawn or counted as folded \
             (drawn {drawn_async} + folded {folded} must be the two declared rows):\n{err}"
        );
        let (drawn, self_rows, raw, _, _) = ledger(&err);
        assert_eq!(drawn + self_rows, raw, "the ledger stays exact at level {level}");
        assert!(
            self_rows >= folded,
            "at level {level} the folded rows ({folded}) must ride the self tally ({self_rows}):\n{err}"
        );
        assert!(
            body_both.contains("-.->|") || drawn_async == 0,
            "at level {level} a drawn async row renders under `--layer both`:\n{body_both}"
        );
    }
    // The dir:1 fold places both async rows on the shared first-directory
    // bucket: the self tally names that bucket and the ledger row is exact.
    let (code, _body, err) = render(&fixture, "both", "dir:1");
    assert_eq!(code, 0, "the dir:1 render must exit 0: {err}");
    assert!(
        err.contains("self rows: pkg ×"),
        "the folded rows are self-accounted under the bucket they fold onto:\n{err}"
    );
    assert!(
        err.contains("note: 2 async rows folded away in this projection"),
        "the dir:1 fold counts both folded rows with the shipped note bytes:\n{err}"
    );
    assert_eq!(folded_count(&err), 2);
    let (drawn, self_rows, raw, level, _) = ledger(&err);
    assert_eq!(
        (drawn + self_rows, raw, level.as_str()),
        (5, 5, "dir:1"),
        "the dir:1 ledger stays exact with the folded rows inside the self term:\n{err}"
    );
}

/// Cell 3 (done-when 1, the compose law at the flag seam) — the flag-absent
/// render replays today's bytes: `--layer both` is byte-identical to the
/// flag-absent render (the shipped render), the flag-absent stderr carries
/// NO level rows (`--layer` alone draws the shipped render and owes no
/// accounting block), and `direct` provably selects rows against that body.
/// The pre-existing diagram byte-pins keep covering the shipped bytes; this
/// cell pins the composition seam.
#[test]
fn flag_absent_render_replays_the_both_body_and_carries_no_ledger_rows() {
    let fixture = composed_artefact();
    let (code_plain, body_plain, err_plain) =
        run(&fixture, &["diagram", "--source", "scan", "model.json"]);
    assert_eq!(code_plain, 0, "the flag-absent render must exit 0: {err_plain}");
    let (code_both, body_both, _err_both) = render(&fixture, "both", "module");
    assert_eq!(code_both, 0);
    let (code_layer, body_layer, err_layer) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--layer", "both"],
    );
    assert_eq!(code_layer, 0);
    assert_eq!(
        body_plain, body_layer,
        "`--layer both` must replay the flag-absent render byte for byte"
    );
    assert_ne!(
        body_plain, body_both,
        "the flag-absent render is the whole-model render, not a level fold"
    );
    for (surface, err) in [("flag-absent", &err_plain), ("--layer only", &err_layer)] {
        assert!(
            !err.contains("accounting exact:") && !err.contains("descriptive only"),
            "the {surface} render owes no level accounting block:\n{err}"
        );
    }
    // `--layer direct` selects rows against the shipped body: the async rows
    // the flag-absent body draws are gone, the structural family stays.
    let (code_direct, body_direct, _err_direct) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--layer", "direct"],
    );
    assert_eq!(code_direct, 0);
    assert!(
        !dashed_edges(&body_plain).is_empty() && dashed_edges(&body_direct).is_empty(),
        "`--layer direct` draws none of the async rows the shipped body draws:\n{body_plain}\n{body_direct}"
    );
    assert_eq!(
        solid_edges(&body_direct),
        solid_edges(&body_plain),
        "the layer select happens on the projected rows — the structural family is unchanged:\n{body_direct}"
    );
}

/// Cell 4 (done-when 2) — the stderr posture: the descriptive-only row and
/// the completeness restatement appear on STDERR and NEVER in the render
/// body, and the stdout body bytes are IDENTICAL with and without those
/// rows: two fixtures that differ ONLY in their `unresolvable_imports` render
/// byte-identical bodies at every level (byte-diff empty) while the gap
/// census restatement moves only the stderr.
#[test]
fn descriptive_and_restatement_rows_ride_stderr_only() {
    let fixture_plain = composed_artefact();
    let fixture_gap = Fixture::new();
    fixture_gap.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"alpha","kind":"lib","path":"pkg/alpha"},{"name":"beta","kind":"lib","path":"pkg/beta"},{"name":"gamma","kind":"lib","path":"svc/gamma"}],"edges":[{"from":"alpha","to":"beta"},{"from":"gamma","to":"alpha"}],"module_edges":[{"unit":"alpha","from":"pkg/alpha/gateway","to":"pkg/beta/store","symbols":[]},{"unit":"gamma","from":"svc/gamma/store","to":"pkg/alpha/gateway","symbols":[]},{"unit":"alpha","from":"pkg/alpha/gateway","to":"pkg/alpha/gateway","symbols":[]}],"async_edges":[{"from":"alpha","to":"beta","event":"ping::PingEvent","kind":"async"},{"from":"beta","to":"beta","event":"loop::LoopEvent","kind":"async"}],"unresolvable_imports":[{"file":"pkg/alpha/gateway.rs","specifier":"ghost::x","line":"3"},{"file":"pkg/beta/store.rs","specifier":"ghost::x","line":"7"}]}"#,
    );
    for level in LEVELS {
        let (code_a, body_a, err_a) = render(&fixture_plain, "both", level);
        let (code_b, body_b, err_b) = render(&fixture_gap, "both", level);
        assert_eq!(code_a, 0);
        assert_eq!(code_b, 0);
        assert_eq!(
            body_a, body_b,
            "the render body is byte-identical with and without the restatement rows at level {level}"
        );
        assert!(
            err_a.contains("note: descriptive only — verify gates the module tier"),
            "the descriptive-only row rides stderr at level {level}:\n{err_a}"
        );
        assert!(
            !err_a.contains("completeness restated:"),
            "a gap-free artefact restates nothing at level {level}:\n{err_a}"
        );
        assert!(
            err_b.contains("completeness restated: unresolvable imports census:"),
            "the gap census restatement rides stderr at level {level}:\n{err_b}"
        );
        for (marker, body) in [("completeness restated", &body_b), ("descriptive only", &body_b)] {
            assert!(
                !body.contains(marker),
                "the {marker:?} row entered the render body at level {level}:\n{body}"
            );
        }
    }
}

/// Cell 5 (MINOR-11 fixture) — a bucket cycle WITHOUT a module-tier cycle is
/// descriptive: the fold draws both bucket arrows, the render exits 0, the
/// descriptive-only row states the posture, and no verdict byte appears.
/// `verify` keeps gating the module tier — the render never adjudicates.
#[test]
fn a_bucket_cycle_without_a_module_cycle_carries_the_descriptive_row_and_stays_green() {
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"one","kind":"lib","path":"x/one"},{"name":"two","kind":"lib","path":"y/two"},{"name":"three","kind":"lib","path":"x/three"}],"edges":[{"from":"one","to":"two"},{"from":"two","to":"three"}],"module_edges":[{"unit":"one","from":"x/one/aa","to":"y/two/bb","symbols":[]},{"unit":"two","from":"y/two/cc","to":"x/three/dd","symbols":[]}]}"#,
    );
    // The unit-tier graph (the verify surface) is a path: one→two→three.
    // At dir:1 the units fold onto two buckets that DO form a cycle: x→y and
    // y→x — a cycle the fold draws and no gate adjudicates.
    let (code, body, err) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--level", "dir:1"],
    );
    assert_eq!(
        code, 0,
        "a bucket cycle is descriptive — the render stays green: {err}"
    );
    assert!(
        err.contains("note: descriptive only — verify gates the module tier"),
        "the descriptive-only row states the posture on stderr:\n{err}"
    );
    let solid = structural_rows(&body);
    assert_eq!(
        solid.len(),
        2,
        "both bucket arrows draw — descriptive, never adjudicated:\n{body}"
    );
    let lowered = err.to_lowercase();
    for verdict in ["violation", "cycle detected", "error:"] {
        assert!(
            !lowered.contains(verdict),
            "the level render speaks no gate verdict ({verdict:?}):\n{err}"
        );
    }
    ledger(&err);
    // The module tier of the same fold stays cycle-free — the bucket cycle
    // is the fold's reading, not the model's.
    let (code_module, body_module, _err_module) = run(
        &fixture,
        &["diagram", "--source", "scan", "model.json", "--level", "module"],
    );
    assert_eq!(code_module, 0);
    assert_eq!(
        structural_rows(&body_module).len(),
        2,
        "the module fold draws its two arrows — no bucket pair cycles twice:\n{body_module}"
    );
}

/// Cell 6 (done-when 1, `--check` arm) — a fresh check is GREEN with the
/// descriptive rows present: the rows ride stderr only, so a level render
/// written once replays exactly and `--check` answers its fresh line at
/// exit 0 while the rows keep printing.
#[test]
fn check_stays_green_while_the_rows_ride_stderr() {
    let fixture = composed_artefact();
    let (code, _body, err) = run(
        &fixture,
        &[
            "diagram",
            "--source",
            "scan",
            "model.json",
            "--level",
            "module",
            "--output",
            "out.mmd",
        ],
    );
    assert_eq!(code, 0, "the first write must exit 0: {err}");
    assert!(fixture.path("out.mmd").is_file(), "the write must create out.mmd");
    let (code, out, err) = run(
        &fixture,
        &[
            "diagram",
            "--source",
            "scan",
            "model.json",
            "--level",
            "module",
            "--output",
            "out.mmd",
            "--check",
        ],
    );
    assert_eq!(
        code, 0,
        "`--check` must stay green with the descriptive rows present: {err}"
    );
    assert!(
        out.contains("ok: out.mmd up to date"),
        "the fresh check prints its ok line:\n{out}"
    );
    assert!(
        err.contains("accounting exact:")
            && err.contains("note: descriptive only — verify gates the module tier"),
        "the check run still owes its accounting block:\n{err}"
    );
}

/// Cell 7 (done-when 3, the fleet-wide parity law) — every VERB+FLAG pair
/// in EVERY level refusal row resolves in the named surface's `--help`. The
/// enumeration comes from the claim registry itself (the canonical surface,
/// read at run time): every `level-*` claim and the viewer byte-ceiling
/// claim contributes its grammar tokens, so a landing that adds a refusal
/// row without a help surface reddens HERE — the §Round-3/4 parity cell
/// applied to all five landings' rows, not just the fit refusal's.
#[test]
fn every_level_row_verb_flag_pair_resolves_in_help_fleet_wide() {
    let registry_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("claim-registry.toml");
    let text = std::fs::read_to_string(&registry_path).expect("read claim-registry.toml");
    let value: toml::Value = toml::from_str(&text).expect("registry TOML");
    let claims = value["claim"].as_array().expect("[[claim]] entries");
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut inspected = 0usize;
    for claim in claims {
        let id = claim["id"].as_str().expect("claim id");
        if !(id.starts_with("level-") || id == "viewer-byte-ceiling-refusal") {
            continue;
        }
        let Some(canonical) = claim.get("canonical").and_then(|v| v.as_str()) else {
            continue;
        };
        if !canonical.contains("archspec ") {
            continue;
        }
        inspected += 1;
        // The grammar shape (shared with the fit-refusal parity cell): a
        // step starts at the `archspec` verb-name, the next token is the
        // verb, every --flag token until the next step rides that verb.
        let mut verb = String::new();
        let mut after_archspec = false;
        for token in canonical.split_whitespace() {
            if token == "archspec" {
                after_archspec = true;
                continue;
            }
            if after_archspec {
                verb = token.trim_matches(['[', ']', ',']).to_string();
                after_archspec = false;
                continue;
            }
            if token.trim_start_matches('[').starts_with("--") {
                let flag: String = token
                    .chars()
                    .filter(|c| c.is_ascii_lowercase() || *c == '-' || *c == '[' || *c == ']')
                    .collect();
                let flag = flag.trim_matches(['-', '[', ']']);
                pairs.push((verb.clone(), format!("--{}", flag.trim_start_matches('-'))));
            }
        }
    }
    assert!(
        inspected >= 3,
        "the registry must hold the level grammar rows this cell sweeps, got {inspected}"
    );
    let unique: std::collections::BTreeSet<(String, String)> = pairs.iter().cloned().collect();
    assert!(
        unique.contains(&("scan".to_string(), "--output".to_string()))
            && unique.contains(&("diagram".to_string(), "--source".to_string()))
            && unique.contains(&("diagram".to_string(), "--level".to_string())),
        "the level rows must carry the scan --output, diagram --source and diagram --level \
         pairs, got {unique:?}"
    );
    let fixture = Fixture::new();
    for (verb, flag) in unique {
        // §Round-4 law: the `--level` pair names the post-scenario diagram
        // surface — check it against the built binary; every other pair
        // resolves in the resolved (installed-when-set) binary's help.
        let binary = if flag == "--level" {
            std::path::PathBuf::from(env!("CARGO_BIN_EXE_archspec"))
        } else {
            shared::matrix::resolve_binary()
        };
        let output = std::process::Command::new(&binary)
            .args([verb.as_str(), "--help"])
            .output()
            .expect("the help surface must be runnable");
        assert_eq!(output.status.code(), Some(0), "{verb} --help must exit 0");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            text.contains(&flag),
            "level-row pair `{verb} {flag}` never resolves in `{verb} --help` of {}:\n{text}",
            binary.display()
        );
    }
    // The narrowing grammar itself rides the diagram help verbatim — the
    // refusal's steps are documented where the refusal points.
    let (_code, help, _err) = run(&fixture, &["diagram", "--help"]);
    assert!(
        help.contains("--layer") && help.contains("--level"),
        "`archspec diagram --help` must document both composed flags:\n{help}"
    );
}

/// Spell `value` with a thousands separator `sep` every three digits — the
/// comma spelling and the underscore spelling the fleet scans for.
fn with_separators(value: usize, sep: char) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(sep);
        }
        out.push(ch);
    }
    out
}

/// The ceiling needles PARSED, never restated (the ceiling-facts law): the
/// char ceiling comes from the `archspec diagram --help` anchor "ceilings of
/// <edges> edges and <chars> characters"; the viewer emitted-byte ceiling is
/// the binary 1 MB under the byte reading, spelled arithmetically so this
/// guard file carries no number bytes of its own. All three spellings the
/// fleet uses are scanned: the comma char form, the underscore byte form, and
/// the bare byte form.
fn ceiling_needles(fixture: &Fixture) -> Vec<String> {
    let (code, out, err) = run(fixture, &["diagram", "--help"]);
    assert_eq!(code, 0);
    let help = format!("{out}{err}");
    let anchor = help
        .find("ceilings of ")
        .expect("the help states the ceiling pair")
        + "ceilings of ".len();
    let edges: String = help[anchor..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let rest = help[anchor + edges.len()..]
        .strip_prefix(" edges and ")
        .expect("the edge ceiling is followed by the char ceiling");
    let at = rest.find(" characters").expect("the char ceiling follows");
    let chars: usize = rest[..at]
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .expect("the char ceiling parses");
    let bytes: usize = 1usize << 20;
    vec![
        with_separators(chars, ','),
        with_separators(bytes, '_'),
        bytes.to_string(),
    ]
}

/// Cell 8 (done-when 3, ceiling facts) — the ceiling numbers stay PARSED
/// from `archspec diagram --help` at the seam; only owner surfaces spell the
/// constants. The grep walks the payload tree: every file restating a
/// ceiling number must be an owner (the constant declarations, the HELP
/// surfaces that own the rationale sentence, the claim registry, the guards
/// that weld the parse, and the CHANGELOG's release history). A new restat-
/// ing file — this lane's composition guard included — reddens the cell.
#[test]
fn ceiling_numbers_appear_on_owner_surfaces_only() {
    let owners: std::collections::BTreeSet<&str> = [
        "src/archspec/diagram.rs",
        "src/archspec/viewer.rs",
        "src/archspec/commands/diagram.rs",
        "src/archspec/commands/depgraph.rs",
        "claim-registry.toml",
        "CHANGELOG.md",
        "tests/mermaid_ceiling.rs",
        "tests/level_viewer_bytes.rs",
        "tests/level_fit_refusal.rs",
        "tests/e2e_matrix.rs",
        "tests/diagram_layers.rs",
    ]
    .into_iter()
    .collect();
    let fixture = Fixture::new();
    let needles = ceiling_needles(&fixture);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut hits: Vec<String> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read dir") {
            let path = entry.expect("dir entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == ".git" || name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let tracked = matches!(
                path.extension().and_then(|e| e.to_str()),
                Some("rs" | "toml" | "md" | "json" | "mmd" | "sh" | "yml" | "yaml")
            );
            if !tracked {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            if needles.iter().any(|needle| text.contains(needle)) {
                hits.push(path
                    .strip_prefix(root)
                    .expect("inside the payload")
                    .to_string_lossy()
                    .replace('\\', "/"));
            }
        }
    }
    for hit in &hits {
        assert!(
            owners.iter().any(|owner| *owner == hit.as_str()),
            "`{hit}` restates a ceiling number but is not an owner surface — parse the facts \
             from `archspec diagram --help` instead (the ceiling-facts law)"
        );
    }
    // The seam itself parses: the level machinery reads the ceilings it was
    // handed, and the help parse keeps answering them (the fit guard pins
    // the values; this cell pins the ownership).
    let (code, help, _err) = run(&fixture, &["diagram", "--help"]);
    assert_eq!(code, 0);
    assert!(
        help.contains("ceilings of "),
        "the ceiling rationale sentence stays in the diagram help:\n{help}"
    );
}

/// Cell 9 (done-when 3, the matrix/help/docs weld) — facts vs surfaces: the
/// capability matrix carries NO level fact (no matrix field was invented
/// for the new flags — the flag-level claims live in the claim registry and
/// the help surfaces); every level usage refusal speaks the usage class the
/// fleet matrix declares for `diagram` (exit 64); every `archspec <verb>` a
/// level grammar row names exists in the command list; and the payload docs
/// cite the new flags where the contract lives without restating behavior.
#[test]
fn matrix_help_and_docs_agree_on_the_composed_surface() {
    let fixture = Fixture::new();
    let (code, matrix, _err) = run(&fixture, &["capability", "matrix"]);
    assert_eq!(code, 0);
    assert!(
        !matrix.to_lowercase().contains("--level") && !matrix.to_lowercase().contains("--layer"),
        "the capability matrix must not name the flags — the flag-level claims live in the \
         claim registry and the help surfaces:\n{matrix}"
    );
    // Usage-class agreement: every level usage refusal answers 64, the class
    // the fleet matrix declares for the diagram verbs' usage errors.
    for refusal in [
        vec!["diagram", "--level", "sideways"],
        vec!["diagram", "--level", "dir:0"],
        vec!["diagram", "--source", "spec", "--level", "module"],
    ] {
        let (code, _out, err) = run(&fixture, &refusal);
        assert_eq!(
            code, 64,
            "the level refusal `{refusal:?}` must speak the usage class 64: {err}"
        );
    }
    // Cross-reference agreement: every verb the narrowing grammar names is
    // a real command of the shipped surface.
    let (_code, index, _err) = run(&fixture, &["--help"]);
    for verb in ["scan", "diagram"] {
        assert!(
            index.contains(&format!("{verb} ")) || index.contains(&format!("  {verb}")),
            "the level grammar's verb `{verb}` is absent from the command list:\n{index}"
        );
    }
    // The docs cite the new flags where the contract lives (payload-docs
    // law: a citation, no behavior restatement).
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let cli = std::fs::read_to_string(
        manifest.join("docs/archspec/commands/diagram/cli.md"),
    )
    .expect("read the diagram cli doc");
    for flag in ["--layer", "--level"] {
        assert!(
            cli.contains(flag),
            "the diagram command doc must cite the `{flag}` flag"
        );
    }
    for page in ["README.md", "cli.md"] {
        assert!(
            manifest
                .join(format!("docs/archspec/commands/diagram/{page}"))
                .is_file(),
            "the diagram docs page `{page}` must exist for the cross-reference"
        );
    }
}
