// US 05 (workplan_archspec_fanout_and_output): `--output -` means stdout.
// The rust blind run asked for stdout with `inspect tree --output -` and got a
// 36 KB file literally named `./-`. Now the dash is the pipe convention it reads
// as one: the body goes to stdout and nothing else (not even the destination
// statement of US 04), no file named `-` appears anywhere, the configured
// destination keeps its bytes, exit 0 — and every help surface that documents
// `--output` says so (owner decision C).

mod common;

use std::path::Path;

use common::{stderr, stdout};

/// One render command that accepts `--output`, the body marker stdout must
/// carry, and the `[output]` key + path its destination is configured to (none
/// for `depgraph`, which has no `[output]` key).
struct Render {
    args: &'static [&'static str],
    body_marker: &'static str,
    key: Option<&'static str>,
    destination: Option<&'static str>,
}

const RENDERERS: &[Render] = &[
    Render {
        args: &["report"],
        body_marker: "Architecture report",
        key: Some("report"),
        destination: Some("docs/archspec/report.md"),
    },
    Render {
        args: &["scan"],
        body_marker: "\"language\"",
        key: Some("scan"),
        destination: Some("docs/archspec/scan.json"),
    },
    Render {
        args: &["diagram"],
        body_marker: "graph",
        key: Some("diagram"),
        destination: Some("docs/archspec/diagram.mmd"),
    },
    Render {
        args: &["inspect"],
        body_marker: "graph",
        key: Some("inspect"),
        destination: Some("docs/archspec/inspect.mmd"),
    },
    Render {
        args: &["depgraph", "modules"],
        body_marker: "graph",
        key: None,
        destination: None,
    },
];

/// A single-crate rust tree whose sources carry a module tier (`app::auth` is
/// imported by `app::billing`): enough for every render command — `report` and
/// `diagram` need a spec that matches, `depgraph` needs the module tier.
fn render_fixture() -> common::Fixture {
    let fixture = common::Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "mod auth;\nmod billing;\n");
    fixture.write("src/auth.rs", "pub struct Token;\n");
    fixture.write("src/billing.rs", "use crate::auth::Token;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = { units = [\"app\"] }\n",
    );
    fixture
}

/// Every path in the tree, as fixture-relative strings — the dash guard's
/// witness list.
fn paths(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            out.push(
                path.strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned(),
            );
            if path.is_dir() {
                stack.push(path);
            }
        }
    }
    out
}

/// Scenario "dash is stdout": with `[output]` destinations configured, every
/// render command given `--output -` puts the body (and only the body) on
/// stdout, leaves the configured destination at its previous bytes, creates no
/// file named `-`, and exits 0.
#[test]
fn dash_output_is_stdout_for_every_render_command() {
    for renderer in RENDERERS {
        let fixture = render_fixture();
        let mut config = String::from("[output]\n");
        for entry in RENDERERS {
            if let (Some(key), Some(path)) = (entry.key, entry.destination) {
                config.push_str(&format!("{key} = \"{path}\"\n"));
            }
        }
        fixture.write("archspec.toml", &config);

        // First run without the dash populates each configured destination.
        let seeded = fixture.run(renderer.args);
        assert_eq!(
            seeded.status.code(),
            Some(0),
            "seeding run {:?} must exit 0 (stderr: {})",
            renderer.args,
            stderr(&seeded)
        );
        let before: Vec<(String, String)> = renderer
            .destination
            .into_iter()
            .map(|path| (path.to_string(), fixture.read(path)))
            .collect();

        let mut args = renderer.args.to_vec();
        args.extend(["--output", "-"]);
        let output = fixture.run(&args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?} must exit 0 (stderr: {})",
            stderr(&output)
        );

        let out = stdout(&output);
        assert!(
            out.contains(renderer.body_marker),
            "{args:?} must print the body on stdout:\n{out}"
        );
        assert!(
            !out.contains("wrote ") && !out.contains("not written"),
            "{args:?} under dash is the body only — no destination statement:\n{out}"
        );

        // No file named `-` in the working directory or anywhere in the tree.
        let all = paths(&fixture.root);
        let offenders: Vec<&String> = all.iter().filter(|name| name.ends_with('-')).collect();
        assert!(
            offenders.is_empty(),
            "{args:?} must create no file named -:\n{offenders:?}"
        );

        for (path, bytes) in before {
            assert_eq!(
                fixture.read(&path),
                bytes,
                "{args:?} must leave the configured destination untouched"
            );
        }
    }
}

/// The dash body is byte-identical to what a named `--output` writes — stdout
/// carries the body, not a reformatted or annotated variant of it.
#[test]
fn dash_stdout_equals_the_body_a_named_output_writes() {
    let fixture = render_fixture();
    let mut args = vec!["inspect"];
    args.extend(["--output", "body.mmd"]);
    let named = fixture.run(&args);
    assert_eq!(named.status.code(), Some(0), "named write must exit 0");
    let body = fixture.read("body.mmd");

    let dash = fixture.run(&["inspect", "--output", "-"]);
    assert_eq!(dash.status.code(), Some(0), "dash run must exit 0");
    assert_eq!(
        stdout(&dash),
        body,
        "dash stdout must equal the named-write body (stderr: {})",
        stderr(&dash)
    );
}

/// The exact command the blind run reached for, in a tree with no config at all:
/// it must not leave a junk file behind.
#[test]
fn dash_leaves_no_junk_file_in_an_unconfigured_tree() {
    let fixture = render_fixture();
    let output = fixture.run(&["inspect", "--output", "-"]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "inspect --output - must exit 0 (stderr: {})",
        stderr(&output)
    );
    assert!(stdout(&output).contains("graph"), "body must be on stdout");
    let names = paths(&fixture.root);
    assert!(
        names
            .iter()
            .all(|name| name != "-" && !name.ends_with("/-")),
        "no file named - may appear: {names:?}"
    );
}

/// Scenario "the flag's help says so": every surface that documents `--output`
/// states the dash convention — the command's own `--help` and its manual page.
#[test]
fn every_output_flag_help_states_the_dash_convention() {
    for command in [
        vec!["scan"],
        vec!["report"],
        vec!["diagram"],
        vec!["inspect"],
        vec!["depgraph", "modules"],
    ] {
        let mut help_flag = command.clone();
        help_flag.push("--help");
        let mut topic = vec!["help"];
        topic.extend(command.iter().take(1).copied());
        for (label, args) in [("--help", help_flag), ("help topic", topic)] {
            let output = common::Fixture::new().run(&args);
            assert_eq!(
                output.status.code(),
                Some(0),
                "`{args:?}` must exit 0 (stderr: {})",
                stderr(&output)
            );
            let out = stdout(&output);
            assert!(
                out.contains("`--output -` writes to stdout and creates no file"),
                "the {label} surface of {command:?} must document the dash:\n{out}"
            );
        }
    }
}
