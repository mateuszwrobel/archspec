//! Phase B cells of the ts-driver design record — file-tier cycle adjudication
//! at scope `both` (D5/D6/D7): a claim-free `no_cycles` on a ts tree derives
//! the `both` posture and reddens file cycles; findings attribute to the
//! nearest common claim ancestor over the claims union a synthetic root;
//! a files-tier SCC is suppressed iff its claim projection is a cycle already
//! reported at the claim tier (R3-4, non-vacuously per R3-7); the cycle graph
//! counts type-only edges (D3) and asset edges never close a cycle (R2-6);
//! claims scope stays byte-identical to the shipped adjudication.

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

fn cycle_lines(out: &std::process::Output) -> Vec<String> {
    let mut lines: Vec<String> = common::stdout(out)
        .lines()
        .chain(common::stderr(out).lines())
        .filter_map(|line| {
            let line = line.trim_start();
            if let Some(rest) = line.strip_prefix("warning: cycle: ") {
                Some(format!("warning: cycle: {}", rest.trim()))
            } else {
                line.strip_prefix("cycle: ")
                    .map(|rest| format!("cycle: {}", rest.trim()))
            }
        })
        .collect();
    lines.sort();
    lines
}

/// Scenario "file-level cycles inside one claim go red": a posture-derived
/// (no `scope`) run reddens the intra-claim file cycle attributed to the one
/// covering claim, once, with no shallower repeat.
#[test]
fn a_file_cycle_inside_one_claim_reds_attributed_to_the_covering_claim() {
    let tree = ts_tree(
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
        &[
            (
                "src/views/a.ts",
                "import { b } from \"../data/b\";\nexport const a = b;\n",
            ),
            (
                "src/data/b.ts",
                "import { a } from \"../views/a\";\nexport const b = a;\n",
            ),
        ],
    );

    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(1),
        "the ts posture derives `both` — the file cycle is red: {}",
        common::stdout(&verify)
    );
    assert_eq!(
        cycle_lines(&verify),
        vec!["cycle: ui: src/data/b.ts -> src/views/a.ts -> src/data/b.ts"],
        "one finding attributed to the nearest common ancestor (D6), files spelled as the repo spells them (D4)"
    );
}

/// Scenario "scope selects the adjudication tier": claims scope reports the
/// claim-tier cycle only (byte-unchanged behavior); both scope reports the
/// intra-claim file cycle beside it — one violation one finding, and the
/// cross-claim file SCC is suppressed by its projection (R3-4).
#[test]
fn scope_selects_the_tier_and_the_projection_dedups_coarsest_first() {
    let files = [
        (
            "src/one/a.ts",
            "import { x } from \"../two/x\";\nexport const a = x;\n",
        ),
        (
            "src/two/x.ts",
            "import { a } from \"../one/a\";\nexport const x = a;\n",
        ),
        (
            "src/two/m.ts",
            "import { n } from \"./n\";\nexport const m = n;\n",
        ),
        (
            "src/two/n.ts",
            "import { m } from \"./m\";\nexport const n = m;\n",
        ),
    ];
    let spec = |scope: &str| {
        format!(
            "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"one\"\nmatches = {{ units = [\"demo-ui\"], modules = [\"src/one/**\"] }}\n[module.allowed]\ndepend_on = [\"two\"]\n\n[[module]]\nname = \"two\"\nmatches = {{ modules = [\"src/two/**\"] }}\n[module.allowed]\ndepend_on = [\"one\"]\n\n[[constraint]]\ntype = \"no_cycles\"\nscope = \"{scope}\"\n"
        )
    };

    let claims = ts_tree(&spec("claims"), &files);
    let claims_run = claims.run(&["verify", "."]);
    assert_eq!(claims_run.status.code(), Some(1));
    assert_eq!(
        cycle_lines(&claims_run),
        vec!["cycle: one -> two -> one"],
        "claims scope is today's adjudication — no file names on the cycle lines"
    );

    let both = ts_tree(&spec("both"), &files);
    let both_run = both.run(&["verify", "."]);
    assert_eq!(both_run.status.code(), Some(1));
    assert_eq!(
        cycle_lines(&both_run),
        vec![
            "cycle: one -> two -> one",
            "cycle: two: src/two/m.ts -> src/two/n.ts -> src/two/m.ts",
        ],
        "both scope reports the claim cycle and the intra-claim file cycle; the a.ts/x.ts file SCC projects to the reported claim cycle and is suppressed (R3-4)"
    );
}

/// Scenario "scope selects the adjudication tier" — the non-vacuous cell
/// (R3-7): a file cycle whose members' nearest claiming ancestors are three
/// distinct claims with no claim-level cycle between them prints ALONGSIDE
/// the claim-tier findings (here: beside the suppressed p↔q SCC's cycle).
#[test]
fn a_file_cycle_without_a_claim_cycle_projection_prints_beside_the_claim_findings() {
    let files = [
        (
            "src/root/r.ts",
            "import { part } from \"./part/p1\";\nexport const r = part;\n",
        ),
        (
            "src/root/part/p1.ts",
            "import { p } from \"../../p/p\";\nexport const part = p;\n",
        ),
        (
            "src/p/p.ts",
            "import { r } from \"../root/r\";\nexport const p = r;\n",
        ),
        (
            "src/p/p2.ts",
            "import { q } from \"../q/q\";\nexport const p2 = q;\n",
        ),
        (
            "src/q/q.ts",
            "import { p2 } from \"../p/p2\";\nexport const q = p2;\n",
        ),
    ];
    let nested = [
        "[project]\nlanguage = \"ts\"\n",
        "\n[[module]]\nname = \"root\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/root/**\"] }\n\n",
        "[[module.submodules]]\nname = \"root::part\"\nmatches = { modules = [\"src/root/part/*\"] }\nallowed = { depend_on = [\"p\"] }\n\n",
        "[[module]]\nname = \"p\"\nmatches = { modules = [\"src/p/*\"] }\nallowed = { depend_on = [\"root\", \"q\"] }\n\n",
        "[[module]]\nname = \"q\"\nmatches = { modules = [\"src/q/*\"] }\nallowed = { depend_on = [\"p\"] }\n\n",
        "[[constraint]]\ntype = \"no_cycles\"\n",
    ]
    .concat();

    let run = ts_tree(&nested, &files).run(&["verify", "."]);
    assert_eq!(run.status.code(), Some(1));
    assert_eq!(
        cycle_lines(&run),
        vec![
            "cycle: p -> q -> p",
            "cycle: src/p/p.ts -> src/root/r.ts -> src/root/part/p1.ts -> src/p/p.ts",
        ],
        "the p↔q file SCC is suppressed by its claim projection; the disjoint-ancestor cycle prints with no claiming segment, naming every member (R2-5, R3-7)"
    );
}

/// R2-5's pair-law cell: a nested spec and its flat twin emit byte-identical
/// cycle lines at the file scope.
#[test]
fn nested_and_flat_claim_spellings_emit_byte_identical_cycle_lines() {
    let files = [
        ("src/ui/x.ts", "export const x = 0;\n"),
        (
            "src/ui/views/m.ts",
            "import { n } from \"./n\";\nexport const m = n;\n",
        ),
        (
            "src/ui/views/n.ts",
            "import { m } from \"./m\";\nexport const n = m;\n",
        ),
    ];
    let nested = [
        "[project]\nlanguage = \"ts\"\n",
        "\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/ui/*\"] }\n\n",
        "[[module.submodules]]\nname = \"ui::views\"\nmatches = { modules = [\"src/ui/views/*\"] }\n\n",
        "[[constraint]]\ntype = \"no_cycles\"\n",
    ]
    .concat();
    let flat = [
        "[project]\nlanguage = \"ts\"\n",
        "\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/ui/*\"] }\n\n",
        "[[module]]\nname = \"ui::views\"\nmatches = { modules = [\"src/ui/views/*\"] }\n\n",
        "[[constraint]]\ntype = \"no_cycles\"\n",
    ]
    .concat();

    let nested_run = ts_tree(&nested, &files).run(&["verify", "."]);
    let flat_run = ts_tree(&flat, &files).run(&["verify", "."]);
    assert_eq!(nested_run.status.code(), Some(1));
    assert_eq!(
        cycle_lines(&flat_run),
        cycle_lines(&nested_run),
        "the pair law holds at the new scope: nested and flat spellings emit byte-identical cycle lines (R2-5): {} vs {}",
        common::stdout(&nested_run),
        common::stdout(&flat_run)
    );
    assert_eq!(
        cycle_lines(&nested_run),
        vec!["cycle: ui::views: src/ui/views/m.ts -> src/ui/views/n.ts -> src/ui/views/m.ts"],
        "the attributed line names the nearest common claim ancestor, one finding (D6)"
    );
}

/// Scenario "the severity ratchet rides the shipped field": a warning row
/// passes without `--strict` and fails under it — no new severity vocabulary.
#[test]
fn a_warning_severity_file_cycle_passes_and_strict_fails_it() {
    let tree = ts_tree(
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\nseverity = \"warning\"\n",
        &[
            (
                "src/views/a.ts",
                "import { b } from \"../data/b\";\nexport const a = b;\n",
            ),
            (
                "src/data/b.ts",
                "import { a } from \"../views/a\";\nexport const b = a;\n",
            ),
        ],
    );

    let plain = tree.run(&["verify", "."]);
    assert_eq!(
        plain.status.code(),
        Some(0),
        "warning severity is rc-neutral without --strict: {}",
        common::stdout(&plain)
    );
    assert_eq!(
        cycle_lines(&plain),
        vec!["warning: cycle: ui: src/data/b.ts -> src/views/a.ts -> src/data/b.ts"],
        "the shipped warning prefix on the attributed line"
    );

    let strict = tree.run(&["verify", "--strict", "."]);
    assert_eq!(
        strict.status.code(),
        Some(1),
        "--strict reddens the warning"
    );
}

/// Scenario "`import type` carries its kind": the cycle graph counts every
/// edge regardless of kind — a one-direction `import type` closes the cycle.
#[test]
fn the_cycle_graph_counts_a_type_only_edge_like_any_other() {
    let tree = ts_tree(
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
        &[
            (
                "src/a.ts",
                "import type { B } from \"./b\";\nexport const a: B | null = null;\n",
            ),
            (
                "src/b.ts",
                "import { a } from \"./a\";\nexport type B = { a: typeof a };\n",
            ),
        ],
    );

    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(1),
        "no type-only carve-out exists in the cycle graph (D3): {}",
        common::stdout(&verify)
    );
    assert_eq!(
        cycle_lines(&verify),
        vec!["cycle: ui: src/a.ts -> src/b.ts -> src/a.ts"],
        "one finding for the mixed-kind cycle"
    );
}

/// Scenario "an asset import is a counted edge that never closes a cycle":
/// the cycle closes between the two ts files; the asset rides an incoming
/// arrow and appears in no cycle's member set (R2-6, R3-3).
#[test]
fn an_asset_import_never_closes_a_cycle() {
    let tree = ts_tree(
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
        &[
            (
                "src/a.ts",
                "import { b } from \"./b\";\nimport \"./theme.css\";\nexport const a = b;\n",
            ),
            (
                "src/b.ts",
                "import { a } from \"./a\";\nexport const b = a;\n",
            ),
            ("src/theme.css", "body { margin: 0; }\n"),
        ],
    );

    let verify = tree.run(&["verify", "."]);
    assert_eq!(verify.status.code(), Some(1));
    assert_eq!(
        cycle_lines(&verify),
        vec!["cycle: ui: src/a.ts -> src/b.ts -> src/a.ts"],
        "the cycle closes between the two ts files; the asset joins no cycle"
    );
}

/// R2-1: same-file edges are dropped at projection entry — a file importing
/// itself states an edge but no cycle verdict.
#[test]
fn a_self_import_states_an_edge_and_no_cycle() {
    let tree = ts_tree(
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n\n[[constraint]]\ntype = \"no_cycles\"\n",
        &[(
            "src/solo.ts",
            "import { self } from \"./solo\";\nexport const self = 1;\n",
        )],
    );

    let verify = tree.run(&["verify", "."]);
    assert_eq!(
        verify.status.code(),
        Some(0),
        "no cycle from a self edge: {}",
        common::stdout(&verify)
    );
    assert!(cycle_lines(&verify).is_empty());
}

/// Scenario "file-level cycles inside one claim go red", last cell: a spec
/// with no `no_cycles` constraint stays verdict-neutral.
#[test]
fn a_spec_without_no_cycles_reports_no_cycles() {
    let tree = ts_tree(
        "[project]\nlanguage = \"ts\"\n\n[[module]]\nname = \"ui\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/**\"] }\n",
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
    );

    let verify = tree.run(&["verify", "."]);
    assert!(
        verify.status.success(),
        "verdict-neutral as today: {}",
        common::stdout(&verify)
    );
    assert!(cycle_lines(&verify).is_empty());
}

/// Scenario "an empty file adjacency prints the honest vacuity carrier" (D7):
/// the carrier keys on FACTS, not the capability row's label — a ts tree
/// whose scanned files import nothing has granular capability and an EMPTY
/// adjacency, and the file tier states its vacuity instead of passing in
/// silence. Posture-derived scope included: no `scope` key is written.
#[test]
fn a_ts_tree_without_file_edges_prints_the_file_tier_vacuity_carrier() {
    let files = [
        ("src/a/a.ts", "export const a = 1;\n"),
        ("src/b/b.ts", "export const b = 2;\n"),
    ];
    let spec = [
        "[project]\nlanguage = \"ts\"\n",
        "\n[[module]]\nname = \"a\"\nmatches = { units = [\"demo-ui\"], modules = [\"src/a/*\"] }\n\n",
        "[[module]]\nname = \"b\"\nmatches = { modules = [\"src/b/*\"] }\n\n",
        "[[constraint]]\ntype = \"no_cycles\"\n",
    ]
    .concat();

    let run = ts_tree(&spec, &files).run(&["verify", "."]);
    let out = format!("{}{}", common::stdout(&run), common::stderr(&run));
    assert_eq!(
        run.status.code(),
        Some(0),
        "vacuity is warning severity as the shipped channel (report:\n{out})"
    );
    assert!(
        out.contains("vacuous constraint: [constraint #1] no_cycles:"),
        "the shipped vacuity carrier must print (report:\n{out})"
    );
    assert!(
        out.contains("no file-grained dependency edges among the constraint's files"),
        "the carrier must state the file tier's empty adjacency, not silence (report:\n{out})"
    );
}
