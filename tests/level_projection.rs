//! Workplan level_views scenario 01: the `--level` projection fold and its
//! accounting block on the `diagram --source scan` render surface.
//!
//! The fold reads the artefact's `module_edges` rows (REAL relation, plan
//! §Relations): an aggregated edge is a distinct ordered bucket pair drawn
//! as ONE row with its raw-row count; a raw row whose buckets coincide is a
//! SELF row counted in the accounting block. The ledger prints
//! `accounting exact: <d> drawn + <s> self = <r> raw rows at level <lv>`
//! exactly — async folded rows ride the self term, never a dropped count
//! (decision-check MAJOR-4; the async cell lives in `tests/async_render.rs`
//! beside the pinned fold-to-self cell it extends). A zero-row artefact
//! prints `empty model: 0 raw rows at level <lv>` instead (MINOR-10). Every
//! level render prints `note: descriptive only — verify gates the module
//! tier` (MINOR-11) and restates the completeness census row —
//! `completeness restated:` — whenever the gap census is non-empty
//! (MAJOR-6). Files at the tree root bucket into ONE named class at every
//! level — never `.` (MINOR-9). Unknown values refuse with
//! `for --level — supported levels: unit, dir:<k>, module, file` and the
//! floor with `for --level — dir:<k> needs an integer k of at least 1`;
//! with the spec source `--level requires --source scan` refuses, and
//! beside `--tiers` — `--level replaces the tier selection` (all exit 64).
//!
//! The fold is descriptive-only: advisory rows ride STDERR, never the
//! render body; a render with the flag absent stays byte-identical (the
//! pre-existing diagram byte-pins cover that arm).

mod common;

use common::{Fixture, stdout};

/// Aggregation fixture: three raw rows onto one cross-bucket pair
/// (gateway→store) and one row folding onto the self bucket (gateway→gateway).
fn cross_and_self_artefact() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"alpha","kind":"lib","path":"src/alpha"},{"name":"beta","kind":"lib","path":"src/beta"}],"edges":[],"module_edges":[{"unit":"alpha","from":"alpha::gateway","to":"beta::store","symbols":["a"]},{"unit":"alpha","from":"alpha::gateway","to":"beta::store","symbols":["b"]},{"unit":"alpha","from":"alpha::gateway","to":"beta::store","symbols":[]},{"unit":"alpha","from":"alpha::gateway","to":"beta::gateway","symbols":[]}]}"#,
    );
    fixture
}

/// Zero-row fixture: a unit-tier model with no module-edge rows.
fn zero_row_artefact() -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"auth","kind":"crate","path":"crates/auth"}],"edges":[]}"#,
    );
    fixture
}

/// The ledger row's numbers: `accounting exact: <d> drawn + <s> self = <r> raw ...`.
fn ledger(stderr: &str) -> (usize, usize, usize) {
    let row = stderr
        .lines()
        .find(|line| line.starts_with("accounting exact: "))
        .unwrap_or_else(|| panic!("the ledger row is missing from:\n{stderr}"));
    let middle = row
        .trim_start_matches("accounting exact: ")
        .split(" raw ")
        .next()
        .expect("the raw clause");
    let mut numbers = middle
        .split([' ', '+', '='])
        .filter_map(|token| token.parse::<usize>().ok());
    let (drawn, self_rows, raw) = (
        numbers.next().expect("drawn"),
        numbers.next().expect("self"),
        numbers.next().expect("raw"),
    );
    assert_eq!(
        drawn + self_rows,
        raw,
        "the ledger must be exact — drawn + self = raw: {row}"
    );
    (drawn, self_rows, raw)
}

fn run_level(fixture: &Fixture, level: &str, format: &str) -> (String, String) {
    let output = fixture.run(&[
        "diagram",
        "--source",
        "scan",
        "model.json",
        "--level",
        level,
        "--format",
        format,
    ]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "`diagram --level {level}` must exit 0: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        stdout(&output),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

/// Cell 1 — aggregation: the three raw rows draw ONE edge row with count
/// `3 rows`, the intra-bucket row prints the SELF count, and the ledger
/// balances (module fold: `alpha::gateway`→`gateway`, `beta::store`→`store`,
/// `beta::gateway`→`gateway`).
#[test]
fn aggregation_draws_one_counted_edge_and_self_rows_print_their_count() {
    let fixture = cross_and_self_artefact();
    let (body, err) = run_level(&fixture, "module", "dot");
    let edge_rows: Vec<&str> = body.lines().filter(|line| line.contains(" -> ")).collect();
    assert_eq!(
        edge_rows.len(),
        1,
        "three raw rows onto one bucket pair draw exactly one aggregated edge row:\n{body}"
    );
    assert!(
        edge_rows[0].contains("label=\"3 rows\""),
        "the aggregated edge row carries its raw-row count:\n{}",
        edge_rows[0]
    );
    assert!(
        err.contains("self rows: gateway ×1"),
        "the intra-bucket row is self-accounted with its count:\n{err}"
    );
    assert!(
        err.contains("accounting exact: 3 drawn + 1 self = 4 raw rows at level module"),
        "the ledger names drawn, self and raw exactly:\n{err}"
    );
    ledger(&err);
    assert!(
        err.contains("note: descriptive only — verify gates the module tier"),
        "the descriptive-only row rides the accounting block:\n{err}"
    );
}

/// Cell 2 — accounting exact on every shape: zero-self, all-self and the
/// `dir:<k>` fold of the same rows all balance, and the counts match the
/// documented fold of each shape.
#[test]
fn accounting_ledger_is_exact_on_every_shape() {
    // Zero-self shape: two rows onto two distinct pairs.
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"alpha","kind":"lib","path":"src/alpha"},{"name":"beta","kind":"lib","path":"src/beta"}],"edges":[],"module_edges":[{"unit":"alpha","from":"alpha::gateway","to":"beta::store","symbols":[]},{"unit":"alpha","from":"alpha::gateway","to":"beta::admin","symbols":[]}]}"#,
    );
    let (_body, err) = run_level(&fixture, "module", "dot");
    assert!(
        err.contains("self rows: none"),
        "a zero-self shape states the tally as none:\n{err}"
    );
    assert!(
        err.contains("accounting exact: 2 drawn + 0 self = 2 raw rows at level module"),
        "the zero-self ledger states the exact counts:\n{err}"
    );
    ledger(&err);

    // All-self shape: a row folding onto the one bucket `gateway` under the
    // module fold's last-segment node, beside a cross row that keeps the
    // fold's node set separating (a model whose endpoints all collide under
    // the trial names no separation, so the shipped fold picks deeper).
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"alpha","kind":"lib","path":"src/alpha"},{"name":"beta","kind":"lib","path":"src/beta"}],"edges":[],"module_edges":[{"unit":"alpha","from":"alpha::gateway","to":"beta::gateway","symbols":[]},{"unit":"alpha","from":"alpha::admin","to":"beta::store","symbols":[]}]}"#,
    );
    let (body, err) = run_level(&fixture, "module", "dot");
    assert!(
        err.contains("self rows: gateway ×1"),
        "the self-folded row tallies onto its bucket:\n{err}"
    );
    assert!(
        err.contains("accounting exact: 1 drawn + 1 self = 2 raw rows at level module"),
        "the self row draws no arrow and the ledger counts every row:\n{err}"
    );
    let edge_rows: Vec<&str> = body.lines().filter(|line| line.contains(" -> ")).collect();
    assert_eq!(
        edge_rows.len(),
        1,
        "only the cross pair draws an edge:\n{body}"
    );
    ledger(&err);

    // The dir:1 fold of the aggregation fixture: every endpoint's first
    // segment is the unit, so all four rows aggregate onto alpha→beta.
    let fixture = cross_and_self_artefact();
    let (body, err) = run_level(&fixture, "dir:1", "dot");
    assert!(
        err.contains("accounting exact: 4 drawn + 0 self = 4 raw rows at level dir:1"),
        "the dir:1 fold aggregates all four rows onto one pair:\n{err}"
    );
    let edge_rows: Vec<&str> = body.lines().filter(|line| line.contains(" -> ")).collect();
    assert_eq!(
        edge_rows.len(),
        1,
        "one aggregated dir fold draws one row:\n{body}"
    );
    assert!(
        edge_rows[0].contains("label=\"4 rows\""),
        "the dir fold edge row carries the raw-row count:\n{}",
        edge_rows[0]
    );
    ledger(&err);
}

/// Cell 4 — empty model: a zero-row artefact prints the named `empty model`
/// row at every level, never the `accounting exact` row (MINOR-10).
#[test]
fn zero_row_artefact_prints_the_empty_model_row_at_every_level() {
    let fixture = zero_row_artefact();
    for level in ["unit", "dir:1", "module", "file"] {
        let (_, err) = run_level(&fixture, level, "mermaid");
        assert!(
            err.contains(&format!("empty model: 0 raw rows at level {level}")),
            "the zero-row artefact prints the named empty-model row at level {level}:\n{err}"
        );
        assert!(
            !err.contains("accounting exact:"),
            "the zero-row artefact never prints the ledger row at level {level}:\n{err}"
        );
    }
}

/// Cell 5 — root-file bucket (MINOR-9): the tree-root file `main.rs` no
/// unit owns folds into ONE named class — `root`, the word the shipped
/// fold states for paths below any named unit — identical at every level,
/// never `.`, distinct from every unit path label.
#[test]
fn root_files_bucket_into_one_named_class_at_every_level() {
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"app","kind":"lib","path":"crates/app"},{"name":"util","kind":"lib","path":"crates/util"}],"edges":[],"module_edges":[{"unit":"app","from":"main.rs","to":"crates/app/store","symbols":[]},{"unit":"app","from":"crates/app/store","to":"crates/util/util","symbols":[]}]}"#,
    );
    let unit_paths = ["crates/app", "crates/util"];
    assert!(
        !unit_paths.contains(&"root"),
        "the root class word must stay distinct from every unit path label"
    );
    for level in ["unit", "dir:1", "dir:2", "file"] {
        let (body, err) = run_level(&fixture, level, "dot");
        assert!(
            body.contains("\"root\" -> ") || body.contains("-> \"root\""),
            "the root-file row draws its edge onto the one `root` bucket at level {level}:\n{body}"
        );
        assert!(
            !body.contains("\".\" -> ") && !body.contains("-> \".\""),
            "the root class never spells `.` at level {level}:\n{body}"
        );
        assert!(
            !err.contains(". ×"),
            "the self tally never names a `.` bucket at level {level}:\n{err}"
        );
        ledger(&err);
    }
    // At the module level the root file still buckets as `root` while the
    // unit paths fold to their module-tier nodes — the root class stays
    // ONE class and draws its edges beside the folded endpoints.
    let (body, err) = run_level(&fixture, "module", "dot");
    assert!(
        body.contains("\"root\" -> ") || body.contains("-> \"root\""),
        "the root-file row keeps its one `root` bucket at the module level:\n{body}"
    );
    assert!(
        !body.contains("\".\" -> ") && !body.contains("-> \".\""),
        "the root class never spells `.` at the module level:\n{body}"
    );
    ledger(&err);
}

/// Cell 6 — restatement (MAJOR-6): an artefact whose gap census is non-empty
/// restates the scan completeness census row inside the accounting block at
/// every level.
#[test]
fn gap_census_restates_inside_the_accounting_block() {
    let fixture = Fixture::new();
    fixture.write(
        "model.json",
        r#"{"schema_version":1,"language":"rust","units":[{"name":"auth","kind":"crate","path":"crates/auth"}],"edges":[],"module_edges":[{"unit":"auth","from":"crates/auth/login","to":"crates/auth/store","symbols":[]}],"unresolvable_imports":[{"file":"crates/auth/login.rs","specifier":"ghost::x","line":"3"},{"file":"crates/auth/store.rs","specifier":"ghost::x","line":"7"}]}"#,
    );
    let census = "completeness restated: unresolvable imports census: 1 specifiers across 2 files — top: ghost::x ×2";
    for level in ["unit", "dir:1", "module", "file"] {
        let (body, err) = run_level(&fixture, level, "mermaid");
        assert!(
            err.contains(census),
            "the accounting block restates the completeness census row verbatim at level {level}:\n{err}"
        );
        assert!(
            !body.contains("completeness restated"),
            "the restatement never enters the render body at level {level}:\n{body}"
        );
        ledger(&err);
    }
}

/// The descriptive rows are advisory: they ride STDERR and never change rc,
/// and the render body stays free of accounting bytes (plan §CLI partial
/// behavior rows; the exit stays 0 across the cells above).
#[test]
fn accounting_rows_never_enter_the_render_body() {
    let fixture = cross_and_self_artefact();
    for format in ["mermaid", "plantuml", "dot", "svg", "viewer"] {
        let (body, _) = run_level(&fixture, "module", format);
        for marker in ["accounting exact", "self rows", "descriptive only"] {
            assert!(
                !body.contains(marker),
                "the {marker:?} row never enters the {format} render body:\n{body}"
            );
        }
    }
}
