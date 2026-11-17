//! GitHub issue #28 — "For each runtime cycle, emit a runnable import-order
//! repro". `verify --emit-repro` renders, after the verdict, one greppable
//! `repro:` block per violating runtime file-tier cycle: the ring path and
//! BOTH import orders as runnable node one-liners, each stating its static
//! expectation (TDZ ReferenceError in the file whose value access reads the
//! entry's uninitialized binding). The flag is pure output: with it off the
//! shipped bytes do not move; type-assisted rings have no import-order
//! semantics and print nothing; a `--json` run keeps the machine body clean;
//! a runtime cycle on a language without the one-liner form states one
//! named skip row instead of silence.

mod common;

use common::Fixture;

fn ts_tree(spec: &str, files: &[(&str, &str)]) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "package.json",
        "{\n  \"name\": \"demo-ui\",\n  \"dependencies\": {}\n}\n",
    );
    tree.write("architecture.spec.toml", spec);
    for (path, content) in files {
        tree.write(path, content);
    }
    tree
}

/// One claim over `src/**`, a `no_cycles` constraint: every file ring below
/// attributes to the `ui` claim and violates the constraint (error tier).
const TS_SPEC: &str = "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n";

/// The planted TDZ pair: `a` reads `b`'s value at module-init, `b` reads
/// `a`'s value at module-init — every leg a value edge, so the ring is
/// runtime (issue #22's classification, ccd0f5487).
fn tdz_pair() -> Fixture {
    ts_tree(
        TS_SPEC,
        &[
            (
                "src/a.ts",
                "import { b } from \"./b\";\nexport const a = b;\n",
            ),
            (
                "src/b.ts",
                "import { a } from \"./a\";\nexport const b = a;\n",
            ),
        ],
    )
}

fn repro_lines(output: &std::process::Output) -> Vec<String> {
    common::stdout(output)
        .lines()
        .filter(|line| line.starts_with("repro:"))
        .map(str::to_string)
        .collect()
}

fn without_repro_lines(text: &str) -> String {
    text.lines()
        .filter(|line| !line.starts_with("repro:"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Scenario "flag off keeps the shipped bytes": the default output is the
/// shipped render with no `repro:` line anywhere, and stripping the
/// `repro:` lines from the flagged run reproduces the flag-off bytes whole.
#[test]
fn the_repro_flag_defaults_off_and_the_default_bytes_do_not_move() {
    let tree = tdz_pair();
    let off = tree.run(&["verify", "."]);
    assert_eq!(off.status.code(), Some(1));
    let off_out = common::stdout(&off);
    assert!(
        off_out.contains("cycle: ui: src/a.ts -> src/b.ts -> src/a.ts (runtime)"),
        "the shipped finding rides the default run (report:\n{off_out})"
    );
    assert!(
        !off_out.contains("repro:"),
        "the flag-off run prints no repro bytes (report:\n{off_out})"
    );

    let on = tree.run(&["verify", ".", "--emit-repro"]);
    assert_eq!(on.status.code(), Some(1), "the flag never changes the verdict");
    let on_out = common::stdout(&on);
    assert_eq!(
        without_repro_lines(&on_out),
        without_repro_lines(&off_out),
        "stripping the repro lines must reproduce the flag-off bytes exactly"
    );
    assert!(
        on_out.lines().count() > off_out.lines().count(),
        "the flagged run adds the repro block (off:\n{off_out}\non:\n{on_out})"
    );
}

/// Scenario "flag on emits both import orders with expectations": one
/// runtime ring yields the ring path line plus the forward and the reverse
/// node one-liners, each stating where the TDZ ReferenceError lands; the
/// reverse order names its own TDZ error.
#[test]
fn emit_repro_prints_the_ring_and_both_import_orders_with_expectations() {
    let tree = tdz_pair();
    let run = tree.run(&["verify", ".", "--emit-repro"]);
    assert_eq!(run.status.code(), Some(1));
    let out = common::stdout(&run);
    let lines = repro_lines(&run);
    for expected in [
        "repro: src/a.ts -> src/b.ts -> src/a.ts (runtime)",
        "repro: node --input-type=module -e \"import './src/a.ts'; import './src/b.ts'\" — expect: TDZ ReferenceError in src/b.ts",
        "repro: node --input-type=module -e \"import './src/b.ts'; import './src/a.ts'\" — expect: TDZ ReferenceError in src/a.ts",
    ] {
        assert!(
            lines.iter().any(|line| line == expected),
            "the repro block must carry {expected:?} verbatim (report:\n{out})"
        );
    }
    assert_eq!(
        lines.len(),
        3,
        "one ring is one path line plus two order lines (report:\n{out})"
    );
}

/// Scenario "type-assisted rings print nothing" (shape decision c): a ring
/// with a type-only leg has no import-order semantics, so the flagged run
/// keeps the finding and emits zero repro bytes.
#[test]
fn a_type_assisted_ring_prints_no_repro_under_the_flag() {
    let tree = ts_tree(
        TS_SPEC,
        &[
            ("src/a.ts", "import { b } from \"./b\";\nexport const a = b;\n"),
            (
                "src/b.ts",
                "import type { A } from \"./a\";\nexport type B = { a?: A };\nexport const b = 1;\n",
            ),
        ],
    );
    let run = tree.run(&["verify", ".", "--emit-repro"]);
    let out = common::stdout(&run);
    assert!(
        out.contains("(type-assisted)"),
        "the ring is still classified and reported (report:\n{out})"
    );
    assert!(
        repro_lines(&run).is_empty(),
        "a type-assisted-only cycle emits no repro lines (report:\n{out})"
    );
}

/// Scenario "multi-ring runs print one repro block per runtime ring": two
/// value 2-cycles in one claim yield two path lines and four order lines.
#[test]
fn two_runtime_rings_print_one_repro_block_each() {
    let tree = ts_tree(
        TS_SPEC,
        &[
            ("src/two/m.ts", "import { n } from \"./n\";\nexport const m = n;\n"),
            ("src/two/n.ts", "import { m } from \"./m\";\nexport const n = m;\n"),
            ("src/two/p.ts", "import { q } from \"./q\";\nexport const p = q;\n"),
            ("src/two/q.ts", "import { p } from \"./p\";\nexport const q = p;\n"),
        ],
    );
    let run = tree.run(&["verify", ".", "--emit-repro"]);
    assert_eq!(run.status.code(), Some(1));
    let out = common::stdout(&run);
    let lines = repro_lines(&run);
    let paths = lines
        .iter()
        .filter(|line| line.ends_with("(runtime)"))
        .count();
    let orders = lines.iter().filter(|line| line.contains("node --input-type=module")).count();
    assert_eq!(paths, 2, "one path line per runtime ring (report:\n{out})");
    assert_eq!(orders, 4, "two order lines per runtime ring (report:\n{out})");
    for expected in [
        "repro: node --input-type=module -e \"import './src/two/m.ts'; import './src/two/n.ts'\" — expect: TDZ ReferenceError in src/two/n.ts",
        "repro: node --input-type=module -e \"import './src/two/p.ts'; import './src/two/q.ts'\" — expect: TDZ ReferenceError in src/two/q.ts",
    ] {
        assert!(
            lines.iter().any(|line| line == expected),
            "each ring's own forward order must appear: {expected:?} (report:\n{out})"
        );
    }
}

/// Scenario "a runtime cycle on another language states the named skip row"
/// (shape decision: the one-liner form is the ts driver's; silence would be
/// a silent flag no-op, so one line names the missing form instead).
#[test]
fn a_runtime_cycle_on_another_language_states_the_named_skip_row() {
    let tree = Fixture::new();
    tree.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"python\"\n\n[[module]]\nname = \"app\"\nmatches = { modules = [\"**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
    );
    tree.write("a.py", "from b import b\nvalue = b\n");
    tree.write("b.py", "from a import a\nvalue = a\n");
    let run = tree.run(&["verify", ".", "--emit-repro"]);
    let out = common::stdout(&run);
    assert!(
        out.contains("(runtime)"),
        "the python ring is reported as runtime (report:\n{out})"
    );
    assert!(
        out.contains(
            "repro: import-order one-liners are implemented for the ts driver — no runnable form for python"
        ),
        "the named skip row states the missing form (report:\n{out})"
    );
    assert!(
        !out.contains("node --input-type=module"),
        "no node one-liner is emitted for a python ring (report:\n{out})"
    );
}

/// Scenario "a json run keeps the machine body clean" (the notes precedent):
/// `--json` omits the repro section; stdout stays the pure machine body.
#[test]
fn a_json_run_omits_the_repro_section_and_stays_machine_clean() {
    let tree = tdz_pair();
    let run = tree.run(&["verify", ".", "--json", "--emit-repro"]);
    assert_eq!(run.status.code(), Some(1));
    let out = common::stdout(&run);
    assert!(
        !out.contains("repro:"),
        "a --json run keeps stdout the pure machine body (report:\n{out})"
    );
    let value: serde_json::Value =
        serde_json::from_str(&out).expect("the json body parses with the flag set");
    assert!(
        value["findings"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|finding| finding["category"].as_str() == Some("cycle detected in")),
        "the json findings carry the cycle (report:\n{out})"
    );
}

/// Scenario "the flag is documented and unknown flags stay usage-class":
/// `verify --help` lists `--emit-repro`, and a near-miss flag name is the
/// usage class 64 exactly as for every other flag.
#[test]
fn the_flag_is_listed_in_the_help_and_a_near_miss_stays_usage_class() {
    let tree = Fixture::new();
    let help = tree.run(&["verify", "--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert!(
        common::stdout(&help).contains("--emit-repro"),
        "the verify help lists the flag"
    );
    assert!(
        common::stdout(&help).contains("import-order repro section"),
        "the verify help states the claim family of the flag paragraph"
    );
    let run = tree.run(&["verify", ".", "--emit-repr"]);
    assert_eq!(
        run.status.code(),
        Some(64),
        "an unknown flag is the usage class, not a verdict"
    );
}
