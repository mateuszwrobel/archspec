//! The views × markers matrix: where roles appear in every render surface.
//!
//! One home for the statement "which views mark roles, per fixture" — the
//! table `archspec help roles` mirrors and the skill pointer sentence names.
//! Cells are not prose opinions: the machinery runs every view over every
//! fixture with the built binary and derives each cell from the recorded
//! output (a role marker in the rendered text marks; a view registered as
//! designed-silent reads none by decision; anything else reads none). The
//! pinned table below is the reviewed derivation — a view changing its
//! marking behaviour fails here before any document can drift.
//!
//! Fixture identities (each a whole codebase the matrix drives through every
//! view):
//! - `rust-probe`: the rust driver's canonical probe tree — two units with a
//!   hard edge, module structure and an external dependency.
//! - `rust-libbin`: a single manifest with `src/lib.rs` and `src/main.rs` and
//!   no explicit `[[bin]]`/`[lib]` section — the tool-shaped tree where the
//!   bin entrypoint node is a model node the fold views render or drop.
//! - `csharp-probe`: the c# driver's probe tree, composition-root shape
//!   included (the DI-wired `Program.cs` states the composition role).
//! - `go-probe`: the go driver's probe tree, composition-root shape included
//!   (the `package main` clause states the composition role; go has no
//!   facade role — ADR-017).
use crate::common::{self, stdout, Fixture};
use crate::shared::driver::{Driver, Language};

/// One cell of the matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// The view's output carries at least one role marker for this fixture.
    Marks,
    /// The view's output carries no role marker for this fixture.
    None,
    /// The view is silent about roles by stated decision, not oversight —
    /// the registry carries the decision text.
    NoneByDecision,
}

impl Cell {
    pub fn label(&self) -> &'static str {
        match self {
            Cell::Marks => "marks",
            Cell::None => "none",
            Cell::NoneByDecision => "none by decision",
        }
    }
}

/// A render surface the matrix enumerates.
#[derive(Debug, Clone, Copy)]
pub struct View {
    /// Display name, used verbatim in the table and the `roles` help topic.
    pub name: &'static str,
    /// The decision text for a view designed silent (its matrix cell reads
    /// `none by decision` without running it), `None` otherwise.
    pub decision: Option<&'static str>,
    /// The exact line the view emits to state its own silence (US 06's
    /// self-stating note). When set, the cell is derived from the line's
    /// presence in the output, welding table and emitted note together.
    pub self_statement: Option<&'static str>,
}

/// The views, in display order.
pub const VIEWS: &[View] = &[
    View {
        name: "inspect tree",
        decision: None,
        self_statement: None,
    },
    View {
        // The scanner alias of the structural mode: same model rendering,
        // unit-tier edges included (blind5 probe: identical role labels to
        // `inspect tree`, one unit-edge line of difference — a rendering
        // detail, not a roles fact).
        name: "inspect scanner",
        decision: None,
        self_statement: None,
    },
    View {
        // The default file-level import map (no sub-command): roles attach
        // to model nodes, not files, so this view carries no markers —
        // plain `none`, not `none by decision` (no decision text is
        // registered for it).
        name: "inspect (file level)",
        decision: None,
        self_statement: None,
    },
    View {
        name: "depgraph modules",
        decision: None,
        self_statement: None,
    },
    View {
        name: "depgraph api-usage",
        decision: Some("a role is not a usage fact"),
        // US 06's weld: the cell only reads none by decision while the
        // emitted table carries this exact line; rewording either surface
        // fails the derivation naming both.
        self_statement: Some(
            "note: this view shows no roles by decision (a role is not a usage fact)",
        ),
    },
    View {
        name: "diagram (spec mode)",
        decision: Some("declared components are not model paths"),
        self_statement: None,
    },
    View {
        name: "diagram --source scan",
        decision: None,
        self_statement: None,
    },
];

/// The fixtures, in display order.
pub const FIXTURES: &[&str] = &["rust-probe", "rust-libbin", "csharp-probe", "go-probe"];

/// The pinned table (one row per fixture, cells in `VIEWS` order), derived
/// from recorded output as the roles plan requires and re-pinned whenever a
/// run shifts an observation.
pub const TABLE: &[(&str, &[Cell; VIEWS.len()])] = &[
    (
        "rust-probe",
        &[
            Cell::Marks,
            Cell::Marks,
            Cell::None,
            Cell::None,
            Cell::NoneByDecision,
            Cell::NoneByDecision,
            Cell::Marks,
        ],
    ),
    (
        "rust-libbin",
        &[
            Cell::Marks,
            Cell::Marks,
            Cell::None,
            Cell::Marks,
            Cell::NoneByDecision,
            Cell::NoneByDecision,
            Cell::Marks,
        ],
    ),
    (
        "csharp-probe",
        &[
            Cell::Marks,
            Cell::Marks,
            Cell::None,
            Cell::None,
            Cell::NoneByDecision,
            Cell::NoneByDecision,
            Cell::Marks,
        ],
    ),
    (
        "go-probe",
        &[
            Cell::Marks,
            Cell::Marks,
            Cell::None,
            Cell::Marks,
            Cell::NoneByDecision,
            Cell::NoneByDecision,
            Cell::Marks,
        ],
    ),
];

/// The `roles` help topic's per-view lines, byte-identical to `ROLES_HELP`
/// (guarded in the roles suite). The agreement guard parses these rows
/// against the matrix cells.
pub const TOPIC_LINES: &[&str] = &[
    "  inspect tree — marks: entrypoint and facade nodes carry a role label.",
    "  inspect scanner — marks: the same structural model as inspect tree, same role labels.",
    "  inspect (file level) — none: roles attach to model nodes, not files — this view renders files.",
    "  depgraph modules — marks: module nodes carry a role label.",
    "  depgraph api-usage — none by decision: a role is not a usage fact; the view itself prints 'note: this view shows no roles by decision (a role is not a usage fact)'.",
    "  diagram (spec mode) — none by decision: declared components are not model paths.",
    "  diagram --source scan — marks: the scan model carries its roles.",
];

/// What one fixture produced across the views.
pub struct Observed {
    pub fixture: &'static str,
    /// (view name, exit code, recorded output) per view, in `VIEWS` order.
    pub outputs: Vec<(&'static str, i32, String)>,
    /// The recorded roles map from the scan (`name -> role`), sorted.
    pub roles: Vec<(String, String)>,
}

/// Materialize one matrix fixture into the (empty) fixture directory.
pub fn materialize(fixture: &str, fx: &Fixture) {
    match fixture {
        "rust-probe" => {
            let driver = Driver {
                language: Language::Rust,
            };
            driver.materialize(fx, &driver.probe_tree());
        }
        "csharp-probe" => {
            let driver = Driver {
                language: Language::Csharp,
            };
            driver.materialize(fx, &driver.probe_tree());
        }
        "go-probe" => {
            let driver = Driver {
                language: Language::Go,
            };
            driver.materialize(fx, &driver.probe_tree());
        }
        "rust-libbin" => {
            fx.write(
                "Cargo.toml",
                "[package]\nname = \"tool\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
            );
            fx.write("src/lib.rs", "pub mod store;\npub use store::Db;\n");
            fx.write("src/main.rs", "mod store;\nuse store::Db;\nfn main() { let _ = Db; }\n");
            fx.write("src/store.rs", "pub struct Db;\n");
        }
        other => panic!("unknown matrix fixture {other}"),
    }
}

/// Run every view over one fixture (fresh tempdir) and record the outputs.
pub fn observe(fixture: &'static str) -> Observed {
    let fx = Fixture::new();
    materialize(fixture, &fx);
    let scan = fx.run(&["scan"]);
    assert_eq!(
        scan.status.code(),
        Some(0),
        "scan of {fixture} failed: {}",
        crate::common::stderr(&scan)
    );
    let model: serde_json::Value =
        serde_json::from_str(&stdout(&scan)).expect("scan stdout must be JSON");
    let mut roles: Vec<(String, String)> = model["roles"]
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(name, role)| {
                    (
                        name.clone(),
                        role.as_str().unwrap_or_default().to_string(),
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    roles.sort();

    // The diagram views need a spec and a scan artefact on disk; the
    // scan-only views do not read either, so recording order keeps them
    // independent of spec materialization.
    let update = fx.run(&["update", "--force"]);
    assert_eq!(update.status.code(), Some(0), "update failed");
    common::write_file(&fx.root, "scan.json", &stdout(&scan));

    let mut outputs: Vec<(&'static str, i32, String)> = Vec::new();
    for view in VIEWS {
        let (code, text) = run_view(&fx, view);
        outputs.push((view.name, code, text));
    }
    Observed {
        fixture,
        outputs,
        roles,
    }
}

fn run_view(fx: &Fixture, view: &View) -> (i32, String) {
    match view.name {
        "inspect tree" => {
            let out = fx.run(&["inspect", "tree", "--format", "mermaid"]);
            (exit(&out), stdout(&out))
        }
        "inspect scanner" => {
            let out = fx.run(&["inspect", "scanner", "--format", "mermaid"]);
            (exit(&out), stdout(&out))
        }
        "inspect (file level)" => {
            // The bare command — the file-level import map, no sub-command.
            let out = fx.run(&["inspect"]);
            (exit(&out), stdout(&out))
        }
        "depgraph modules" => {
            let out = fx.run(&["depgraph", "modules", "--format", "mermaid"]);
            (exit(&out), stdout(&out))
        }
        "depgraph api-usage" => {
            let out = fx.run(&["depgraph", "api-usage"]);
            (exit(&out), stdout(&out))
        }
        "diagram (spec mode)" => {
            let out = fx.run(&["diagram", "--output", "spec-diagram.mmd"]);
            (
                exit(&out),
                std::fs::read_to_string(fx.root.join("spec-diagram.mmd")).unwrap_or_default(),
            )
        }
        "diagram --source scan" => {
            let out = fx.run(&[
                "diagram",
                "--source",
                "scan",
                "scan.json",
                "--output",
                "scan-diagram.mmd",
            ]);
            (
                exit(&out),
                std::fs::read_to_string(fx.root.join("scan-diagram.mmd")).unwrap_or_default(),
            )
        }
        other => panic!("unknown matrix view {other}"),
    }
}

fn exit(out: &std::process::Output) -> i32 {
    out.status.code().unwrap_or(-1)
}

/// The role-marker syntaxes renderers use: a ` [role]` label suffix (mermaid)
/// and a `<<role>>` stereotype (plantuml).
const ROLE_WORDS: &[&str] = &["composition", "domain", "facade", "foundation", "kernel", "leaf"];

pub fn has_role_marker(text: &str) -> bool {
    ROLE_WORDS
        .iter()
        .any(|role| text.contains(&format!(" [{role}]")) || text.contains(&format!("<<{role}>>")))
}

/// Derive the cell for one recorded output. A registered self-statement is a
/// weld, not a derivation shortcut: the recorded output must carry the exact
/// line the registry names, or the guard fails naming both surfaces (the
/// registry here and the emitting module named in the line's owning view).
pub fn derive(view: &View, code: i32, text: &str) -> Cell {
    assert_eq!(code, 0, "view {} exited {code}", view.name);
    if let Some(statement) = view.self_statement {
        if !text.contains(statement) {
            panic!(
                "the matrix cell for {} reads none by decision, but the view's \
                 recorded output omits the self-stating line {statement:?} — the \
                 matrix registry (tests/shared/roles_matrix.rs) and the emitted \
                 note (src/archspec/depgraph.rs) are welded; one cannot change \
                 without the other",
                view.name
            );
        }
    }
    if view.decision.is_some() {
        return Cell::NoneByDecision;
    }
    if has_role_marker(text) {
        Cell::Marks
    } else {
        Cell::None
    }
}

/// The observed matrix cells for one recorded fixture run, in `VIEWS` order.
pub fn observed_cells(observed: &Observed) -> [Cell; VIEWS.len()] {
    let mut cells = [Cell::None; VIEWS.len()];
    for (index, view) in VIEWS.iter().enumerate() {
        let (_, code, text) = observed
            .outputs
            .iter()
            .find(|(name, _, _)| *name == view.name)
            .expect("every view runs");
        cells[index] = derive(view, *code, text);
    }
    cells
}

/// Render the pinned matrix table, header first.
pub fn table() -> String {
    let mut out = String::new();
    out.push_str("| fixture |");
    for view in VIEWS {
        out.push_str(&format!(" {} |", view.name));
    }
    out.push_str("\n| --- |");
    for _ in VIEWS {
        out.push_str(" --- |");
    }
    out.push('\n');
    for (fixture, cells) in TABLE {
        out.push_str(&format!("| {fixture} |"));
        for cell in cells.iter() {
            out.push_str(&format!(" {} |", cell.label()));
        }
        out.push('\n');
    }
    out
}
