//! US 01 (roles plan): the views × markers matrix is the roles contract.
//! Four scenarios: the matrix derives from recorded command output, the rust
//! lib+bin tree pins the disputed cells, `help roles` agrees with the matrix,
//! and the skill text stops outranking it.
mod common;
#[allow(dead_code)]
mod shared;

use shared::roles_matrix::{self as matrix, Cell, Observed};

/// Run one binary surface in a scratch directory (the matrix's CLI legs do
/// not read a project).
fn run(args: &[&str]) -> std::process::Output {
    common::Fixture::new().run(args)
}

/// Run every view over every fixture once, recording all output.
fn record_all() -> Vec<Observed> {
    matrix::FIXTURES
        .iter()
        .map(|fixture| matrix::observe(fixture))
        .collect()
}

/// Scenario: matrix computed, not written
#[test]
fn matrix_computed_not_written() {
    let observed = record_all();

    // One cell per (fixture × view), values from the fixed vocabulary, and
    // each cell equals the value derived from that run's recorded output.
    for row in &observed {
        let cells = matrix::observed_cells(row);
        let pinned = matrix::TABLE
            .iter()
            .find(|(name, _)| *name == row.fixture)
            .unwrap_or_else(|| panic!("fixture {} is not pinned in the matrix", row.fixture))
            .1;
        for (view, (derived, pinned)) in matrix::VIEWS.iter().zip(cells.iter().zip(pinned.iter())) {
            assert_eq!(
                derived, pinned,
                "cell {} × {} is derived from recorded output, and the pinned \
                 matrix must carry that derived value",
                row.fixture, view.name
            );
        }
    }

    // Cell values derive from the recorded output, not from prose: a
    // marking view's recorded text carries marker syntax, a none cell's
    // carries none (the decided-silent views are excluded — they are
    // designed-absent by registry, and scenario 4 guards that registry).
    for row in &observed {
        for (name, _, text) in &row.outputs {
            let view = matrix::VIEWS
                .iter()
                .find(|v| v.name == *name)
                .expect("recorded view is registered");
            if view.decision.is_some() {
                continue;
            }
            let marks = matrix::has_role_marker(text);
            let cell = matrix::derive(view, 0, text);
            assert_eq!(
                marks,
                cell == Cell::Marks,
                "cell {} × {} must equal marker presence in the recorded output",
                row.fixture,
                view.name
            );
        }
    }

    // A rerun on the same tree reports byte-identically.
    let first = matrix::table();
    let rerun = record_all();
    let mut second = String::new();
    for row in &rerun {
        for cell in matrix::observed_cells(row).iter() {
            second.push_str(cell.label());
            second.push('\n');
        }
    }
    assert!(!first.is_empty());
    let first_cells: String = observed
        .iter()
        .flat_map(matrix::observed_cells)
        .map(|cell| format!("{}\n", cell.label()))
        .collect();
    assert_eq!(
        first_cells, second,
        "rerun on the same tree must report byte-identically"
    );
}

/// Scenario: rust lib+bin pins the seven views
#[test]
fn rust_libbin_pins_the_seven_views() {
    let row = matrix::observe("rust-libbin");
    let output = |view: &str| -> String {
        row.outputs
            .iter()
            .find(|(name, _, _)| *name == view)
            .unwrap_or_else(|| panic!("view {view} ran"))
            .2
            .clone()
    };

    // The roles map carries a lib-root facade entry and a main composition
    // entry (no explicit [[bin]]/lib section in the manifest).
    let roles = &row.roles;
    assert!(
        roles.iter().any(|(name, role)| {
            role == "facade" && name == "tool"
        }),
        "lib root carries a facade entry, roles: {roles:?}"
    );
    assert!(
        roles
            .iter()
            .any(|(name, role)| { role == "composition" && name.ends_with("::main") }),
        "a <unit>::main entry carries the composition entry, roles: {roles:?}"
    );

    // inspect tree carries [composition] on the bin carrier node.
    assert!(
        output("inspect tree").contains("[composition]"),
        "inspect tree marks the composition entrypoint:\n{}",
        output("inspect tree")
    );

    // depgraph modules carries [composition] on the node the main path
    // folds onto. The plan predicted the lib root would fold to an
    // unrendered root node; the observed renderer renders it, so the cell
    // is derived from output (marks) — the matrix records what runs do.
    assert!(
        output("depgraph modules").contains("[composition]"),
        "depgraph modules marks the folded main node:\n{}",
        output("depgraph modules")
    );

    // The two designed-silent views carry no role marker syntax at all.
    for view in ["depgraph api-usage", "diagram (spec mode)"] {
        let text = output(view);
        assert!(
            !matrix::has_role_marker(&text),
            "{view} is designed silent about roles, output:\n{text}"
        );
    }

    // The matrix records exactly these values for these cells.
    let cells = matrix::observed_cells(&row);
    let pinned = matrix::TABLE
        .iter()
        .find(|(name, _)| *name == "rust-libbin")
        .expect("rust-libbin is pinned")
        .1;
    assert_eq!(&cells, pinned);
    // Cells follow `VIEWS` order: inspect tree, inspect scanner, inspect
    // (file level), depgraph modules, depgraph api-usage, diagram (spec
    // mode), diagram --source scan.
    assert_eq!(cells[4], Cell::NoneByDecision);
    assert_eq!(cells[5], Cell::NoneByDecision);
    assert_eq!(cells[0], Cell::Marks);
    assert_eq!(cells[1], Cell::Marks);
    assert_eq!(cells[2], Cell::None);
    assert_eq!(cells[3], Cell::Marks);
    
}

/// Scenario: help roles agrees with the matrix
#[test]
fn help_roles_agrees_with_the_matrix() {
    let output = run(&["help", "roles"]);
    let text = common::stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");

    // The topics index lists roles, and the unknown-topic error lists roles.
    let index = common::stdout(&run(&["help"]));
    assert!(index.contains("roles"), "help index:\n{index}");
    let error = run(&["help", "nope"]);
    assert_eq!(error.status.code(), Some(1));
    assert!(
        common::stderr(&error).contains("roles"),
        "unknown-topic error:\n{}",
        common::stderr(&error)
    );

    // Every per-view statement in the topic matches the matrix cells it
    // describes; a disagreeing sentence is named here.
    let observed = record_all();
    for view in matrix::VIEWS {
        let column: Vec<Cell> = observed
            .iter()
            .map(|row| matrix::observed_cells(row)[matrix::VIEWS
                .iter()
                .position(|v| v.name == view.name)
                .expect("view is registered")])
            .collect();
        let line = text
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("{} ", view.name))
                || line.trim_start().starts_with(&format!("{}—", view.name))
                || line.trim_start().starts_with(view.name))
            .unwrap_or_else(|| panic!("the roles topic states no line for view {}", view.name));
        if line.contains("none by decision") {
            assert!(
                column.iter().all(|cell| *cell == Cell::NoneByDecision),
                "the topic claims {} is designed silent, the matrix disagrees: {column:?}",
                view.name
            );
        } else if line.contains("marks") {
            assert!(
                column.contains(&Cell::Marks),
                "the topic claims {} marks roles, no matrix cell in its column agrees: {column:?}",
                view.name
            );
        } else {
            assert!(
                !column.contains(&Cell::Marks) && view.decision.is_none(),
                "the topic claims {} stays silent, the matrix disagrees: {column:?}",
                view.name
            );
        }
    }

    // Welded wording: the topic prints the registry's TOPIC_LINES byte-
    // for-byte, so the agreement above checks the very lines the tests ship.
    for line in matrix::TOPIC_LINES {
        assert!(
            text.contains(line),
            "the roles topic must print the registry line verbatim:\n{line}"
        );
    }

    // The topic states the none-by-decision cells with their reasons.
    for reason in ["a role is not a usage fact", "declared components are not model paths"] {
        assert!(
            text.contains(reason),
            "the roles topic states the decision reason {reason:?}:\n{text}"
        );
    }
}

/// Scenario: skill text stops outranking the matrix
#[test]
fn skill_text_stops_outranking_the_matrix() {
    let output = run(&["skill"]);
    assert_eq!(output.status.code(), Some(0));
    let skill = common::stdout(&output);

    // The falsified enumeration is gone from the binary's own surface.
    for falsified in [
        "marks in `inspect tree` alone",
        "api-usage stays silent on roles",
    ] {
        assert!(
            !skill.contains(falsified),
            "the skill text keeps no per-view marking enumeration beyond the \
             pointer; found {falsified:?}"
        );
    }

    // The pointer sentence names the roles topic as the source.
    assert!(
        skill.contains("help roles"),
        "the skill text carries the registry pointer sentence naming \
         `archspec help roles` as the source of roles-in-views facts:\n{skill}"
    );
}


// ---- US 06 (roles plan): the role-less view says so ------------------------

/// The exact line the api-usage view must self-state, byte for byte.
const NOTE_LINE: &str = "note: this view shows no roles by decision (a role is not a usage fact)";

fn api_usage_text(tree: &str) -> String {
    let fx = common::Fixture::new();
    matrix::materialize(tree, &fx);
    let out = fx.run(&["depgraph", "api-usage"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "api-usage ran: {}",
        common::stderr(&out)
    );
    common::stdout(&out)
}

/// Scenario: the silent view speaks
#[test]
fn the_silent_view_speaks() {
    for tree in ["rust-libbin", "go-probe"] {
        let out = api_usage_text(tree);
        assert_eq!(
            out.matches(NOTE_LINE).count(),
            1,
            "the note line appears exactly once for {tree}:\n{out}"
        );
        // After the table (or after the emptiness statement), never inside
        // it: the table's fixed columns and rows keep their bytes.
        let note_at = out.find(NOTE_LINE).expect("note present");
        let head = &out[..note_at];
        assert!(
            head.ends_with('\n'),
            "the note starts a line, it does not trail table bytes:\n{out}"
        );
        let body = head.trim_end();
        assert!(
            body.ends_with('|')
                || body.ends_with('.')
                || body.ends_with('.'),
            "the note follows the table or the emptiness statement, not the \
             middle of one:\n{out}"
        );
        if out.contains('|') {
            assert!(
                head.contains("| Target module | Used by module | APIs used |"),
                "the human table keeps its three fixed columns:\n{out}"
            );
        } else {
            assert!(
                head.starts_with("No internal API usage details found."),
                "the emptiness statement keeps its bytes and position:\n{out}"
            );
        }
    }
}

/// Scenario: the matrix owns the sentence's meaning
#[test]
fn the_matrix_owns_the_sentences_meaning() {
    // The cell is none by decision for every fixture.
    for fixture in matrix::FIXTURES {
        let row = matrix::observe(fixture);
        let cells = matrix::observed_cells(&row);
        let index = matrix::VIEWS
            .iter()
            .position(|v| v.name == "depgraph api-usage")
            .expect("registered");
        assert_eq!(
            cells[index],
            Cell::NoneByDecision,
            "the api-usage cell for {fixture} reads none by decision"
        );
    }

    // The weld: the registry carries the emitted line, and rewording the
    // emitted line without the registry fails the guard naming both surfaces.
    let view = matrix::VIEWS
        .iter()
        .find(|v| v.name == "depgraph api-usage")
        .expect("registered");
    assert_eq!(
        view.self_statement,
        Some(NOTE_LINE),
        "the registry entry carries the emitted note verbatim"
    );
    let output = api_usage_text("rust-libbin");
    // Real output + real registry entry: derivation passes.
    assert_eq!(matrix::derive(view, 0, &output), Cell::NoneByDecision);
    // Reworded line against real output: the guard fires and names both
    // surfaces.
    let reworded = matrix::View {
        name: "depgraph api-usage",
        decision: view.decision,
        self_statement: Some("note: roles omitted or something"),
    };
    let guard = std::panic::catch_unwind(|| matrix::derive(&reworded, 0, &output));
    assert!(guard.is_err(), "the weld guard must fail on drift");
    let panic_text = guard.unwrap_err();
    let message = panic_text
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| format!("{panic_text:?}"));
    assert!(
        message.contains("roles_matrix") && message.contains("depgraph"),
        "the guard names both surfaces (registry file and emitting module): \
         {message}"
    );

    // The roles topic quotes the note verbatim (topic line and emitted line
    // name each other).
    let topic = common::stdout(&run(&["help", "roles"]));
    assert!(
        topic.contains(NOTE_LINE),
        "the roles topic quotes the emitted note verbatim:\n{topic}"
    );
}
