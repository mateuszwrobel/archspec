//! Binary-level witness for the python driver (proposal hole (a)).
//!
//! The python driver's report-only line classes were previously witnessed
//! only at lib level and in the help catalog bytes; zero binary runs asserted
//! them. This suite spawns the real binary over python corpora and asserts
//! the emitted lines, the exit class, and the body-purity law (the classes
//! are report-only and never ride inside the machine JSON).

#![allow(clippy::disallowed_methods)]

mod common;
#[allow(dead_code)]
mod shared;

use common::Fixture;
use shared::matrix;

fn importlib_fixture() -> Fixture {
    let fx = Fixture::new();
    fx.write(
        "app.py",
        "import importlib\n\
         \n\
         def load(name):\n\
         \x20   mod = importlib.import_module(name)\n\
         \x20   spec = importlib.util.spec_from_file_location(\"dyn\", \"dyn.py\")\n\
         \x20   return mod, spec\n",
    );
    fx.write("dyn.py", "x = 1\n");
    fx
}

#[test]
fn python_dynamic_sites_witnessed_at_binary_level() {
    let fx = importlib_fixture();
    let (code, _out, err) = matrix::run3(&fx.root, &["scan"]);
    assert_eq!(code, 0, "python scan must exit 0 (report-only lines do not change the verdict)\nstderr:\n{err}");
    assert!(
        err.contains("dynamic import not modelled: app.py:4 — importlib"),
        "the import_module site must emit its report-only line\nstderr:\n{err}"
    );
    assert!(
        err.contains("dynamic import not modelled: app.py:5 — importlib"),
        "the spec_from_file_location site must emit its report-only line\nstderr:\n{err}"
    );
}

#[test]
fn python_report_only_class_never_enters_the_machine_body() {
    let fx = importlib_fixture();
    let (code, out, _err) = matrix::run3(&fx.root, &["scan"]);
    assert_eq!(code, 0);
    let model: serde_json::Value =
        serde_json::from_str(&out).expect("python scan stdout must be model JSON");
    let body = model.to_string();
    assert!(
        !body.contains("dynamic import not modelled"),
        "the report-only class leaked into the machine body: {body}"
    );
    assert_eq!(model["language"], "python");
}

#[test]
fn python_scan_without_manifest_states_the_vacuous_fact() {
    let fx = Fixture::new();
    fx.write("app.py", "x = 1\n");
    let (code, _out, err) = matrix::run3(&fx.root, &["scan"]);
    assert_eq!(code, 0);
    assert!(
        err.contains("vacuous constraint: scan.unit_manifests"),
        "a manifest-less python tree must state the vacuous unit_manifests fact\nstderr:\n{err}"
    );
}
