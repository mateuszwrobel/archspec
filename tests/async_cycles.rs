//! Scenario 03 (archspec_async_depends): the async ring census and its
//! adjacent surfaces. The `[[async_depends]]` declaration gains the verdict
//! parts of issue #44: the no_cycles adjudication stays direct-only (the
//! witness-corpus-style pub→sub→pub round trip stays green at the error class), the
//! census walks direct ∪ async legs over the strongly-connected components
//! and prints one named advisory row per ring (`async cycle: <path> (mixed)`
//! or `async cycle: <path> (async)` — green without `--strict` and green
//! under it too: `--strict` never promotes an async census row, the severity
//! law of scenario 09), the declared
//! endpoints are CHECKED against the claims table (an unclaimed endpoint is a
//! named error finding, a spec typo must not buy silence), async cycles seed
//! no `[[exempt]]` rows and print no `repro:` line, and `archspec explain`
//! traces a pair with async legs through the async-edges section.
//!
//! Law source: `docs/archspec/spec.md` §Async event dependencies; the
//! witness-corpus evidence: opentelemetry's InMemoryExporter publishes `Activity` the
//! instrumentation SDK subscribes to while referencing
//! `OpenTelemetry.Instrumentation.Wip` — https://github.com/open-telemetry/opentelemetry-net-instrumentation/pull/1/files
//! (`ActivitySource.AddActivityListener` composes a delegate calling
//! `Activity.StartStopSwitch`); a grpc client in the same corpus is the direct$
//! sibling of the same shape.
//!
//! The round-trip fixture is the minimal Rust rendering of that shape: unit
//! `orders` declares `EventOne` and unit `catalog` declares `EventTwo`; the
//! table declares the publisher→subscriber leg for each event. No package
//! dependency rides between the two ring units — every ring leg is an async
//! leg; a sink unit `ledger` carries the one direct manifest edge that keeps
//! the `no_cycles` constraint non-vacuous under `--strict` (scenario 09).

mod common;

use common::{Fixture, stderr, stdout};

/// The round-trip fixture: pub→sub→pub with NO manifest dependency between
/// the two ring units, so every ring leg is an async leg. A third sink unit
/// `ledger` carries one real manifest edge from `orders` — the scenario 09
/// strict cell needs the `no_cycles` constraint NON-vacuous, because the
/// vacuous-constraint warning is a direct-channel warning and `--strict`
/// still promotes it (the direct promotion law is unchanged).
fn round_trip_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"orders\", \"catalog\", \"ledger\"]\nresolver = \"2\"\n");
    fixture.write(
        "orders/Cargo.toml",
        "[package]\nname = \"orders\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
         [dependencies]\nledger = { path = \"../ledger\" }\n",
    );
    fixture.write(
        "orders/src/lib.rs",
        "pub struct EventOne;\n\npub fn publish() -> EventOne { EventOne }\n\n\
         pub mod archive;\n\nuse crate::archive::LedgerRow;\n\npub fn store(row: LedgerRow) \
         {\n    let _ = row;\n}\n",
    );
    fixture.write(
        "orders/src/archive.rs",
        "pub struct LedgerRow;\n\npub fn row() -> LedgerRow { LedgerRow }\n",
    );
    fixture.write("ledger/Cargo.toml", "[package]\nname = \"ledger\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("ledger/src/lib.rs", "pub struct LedgerEntry;\n");
    fixture.write("catalog/Cargo.toml", "[package]\nname = \"catalog\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write(
        "catalog/src/lib.rs",
        "pub struct EventTwo;\n\npub fn handle(event: EventTwo) {\n    let _ = event;\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "orders"
matches = { units = ["orders"] }

[module.allowed]
depend_on = ["ledger"]

[[module]]
name = "catalog"
matches = { units = ["catalog"] }

[[module]]
name = "ledger"
matches = { units = ["ledger"] }

[[constraint]]
type = "no_cycles"

[[async_depends]]
from = "orders"
to = "catalog"
event = "orders::EventOne"

[[async_depends]]
from = "catalog"
to = "orders"
event = "catalog::EventTwo"
"#,
    );
    fixture
}

/// The compiled-against fixture: `api` depends on `core` through a manifest
/// path dependency AND the table declares the subscriber-side leg back —
/// the ring carries a direct leg and async legs. Under the declaring-unit
/// predicate (issue #47) the `api -> core` edge is subscriber `api`'s
/// compile-against leg for `core`'s published contract; a rust-tree driver
/// derives no record census, the declaration-trust arm carves the edge, and
/// the ring labels `(async)`.
fn mixed_fixture() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"api\", \"core\"]\nresolver = \"2\"\n");
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
         [dependencies]\ncore = { path = \"../core\" }\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "pub struct OrderPlaced;\n\npub fn publish() -> OrderPlaced { OrderPlaced }\n",
    );
    fixture.write("core/Cargo.toml", "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write(
        "core/src/lib.rs",
        "pub struct AppEvent;\n\npub fn handle(event: AppEvent) {\n    let _ = event;\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        r#"[project]
language = "rust"

[[module]]
name = "api"
matches = { units = ["api"] }
allowed = { depend_on = ["core"] }

[[module]]
name = "core"
matches = { units = ["core"] }

[[constraint]]
type = "no_cycles"

[[async_depends]]
from = "core"
to = "api"
event = "core::AppEvent"

[[async_depends]]
from = "api"
to = "core"
event = "api::OrderPlaced"
"#,
    );
    fixture
}

fn line_count(hay: &str, needle: &str) -> usize {
    hay.lines().filter(|line| line.contains(needle)).count()
}

/// Scenario 03 done-when: the pub→sub→pub round trip stays GREEN at the
/// error class (the direct adjudication owns no async legs), and the census
/// prints ONE named advisory row for the union ring — without `--strict`.
#[test]
fn round_trip_is_green_and_names_one_async_cycle_advisory() {
    let fixture = round_trip_fixture();
    let output = fixture.run(&["verify"]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(
        out.contains("async cycle: catalog -> orders -> catalog (async)"),
        "the census must name the ring exactly once:\n{out}\n--- stderr ---\n{err}"
    );
    assert_eq!(
        line_count(&out, "async cycle:"),
        1,
        "exactly one advisory row per ring:\n{out}\n--- stderr ---\n{err}"
    );
    assert!(
        !out.contains("cycle detected in") && !out.contains("repro:"),
        "the direct adjudication owns no async legs — no cycle verdict, no repro:\n{out}"
    );
    assert!(
        out.contains("ok: architecture.spec.toml passes with 1 warning"),
        "a green run with an advisory row must print the passing headline, not a \
         mismatch headline:\n{out}\n--- stderr ---\n{err}"
    );
    assert!(
        !out.contains("does not match source model"),
        "a run that exits 0 never claims a mismatch (GitHub issue #54):\n{out}"
    );
    assert!(output.status.success(), "the async advisory stays green without --strict:\n{out}");
}

/// The severity law of scenario 09 (issue #44): `--strict` never promotes
/// the async ring advisory — the strict run of the round-trip fixture exits
/// `0`, prints the row byte-identically to the plain run, and the row keeps
/// its `warning: ` prefix.
#[test]
fn strict_mode_keeps_the_async_cycle_advisory() {
    let fixture = round_trip_fixture();
    let lax = fixture.run(&["verify"]);
    let output = fixture.run(&["verify", "--strict"]);
    assert!(
        output.status.success(),
        "`--strict` must NOT fail a run whose only findings are the async cycle row:\n{}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("warning: async cycle: catalog -> orders -> catalog (async)"),
        "the row keeps its warning: prefix under --strict:\n{}",
        stdout(&output)
    );
    assert!(
        stdout(&output).contains("ok: architecture.spec.toml passes with 1 warning"),
        "a `--strict` run whose only findings are census rows exits 0 and prints the \
         passing headline (GitHub issue #54):\n{}",
        stdout(&output)
    );
    assert_eq!(
        stdout(&lax),
        stdout(&output),
        "the strict run is byte-identical to the plain run for an async-row-only diff"
    );
}

/// The compiled-against back edge is NO ring witness when the declaration
/// is publisher-only (GitHub issue #47 — re-pinned from the pre-fix
/// `(mixed)` label): the ring survives on its async legs alone and prints
/// exactly ONE row labelled `(async)` — the direct adjudication stays green
/// and the census owns the row alone.
#[test]
fn compiled_against_back_edge_is_no_witness_and_the_ring_labels_async() {
    let fixture = mixed_fixture();
    let output = fixture.run(&["verify"]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(
        out.contains("async cycle: api -> core -> api (async)"),
        "the ring must print the labelled row with the flipped label:\n{out}\n--- stderr ---\n{err}"
    );
    assert_eq!(line_count(&out, "async cycle:"), 1, "exactly one row per ring:\n{out}");
    assert!(
        !out.contains("(mixed)"),
        "the contract-compile back edge names no `(mixed)` class here:\n{out}"
    );
    assert!(
        !out.contains("cycle detected in") && !out.contains("repro:"),
        "the direct adjudication owns no async legs:\n{out}"
    );
    assert!(output.status.success(), "the async advisory stays green without --strict:\n{out}");
}

/// Async cycles seed no `[[exempt]]` rows and print no `repro:` line — the
/// repro machinery (and its write side) stays direct-only.
#[test]
fn async_cycle_seeds_no_exempt_rows_and_no_repro() {
    let fixture = round_trip_fixture();

    let output = fixture.run(&["verify", "--emit-repro"]);
    let out = stdout(&output);
    assert!(!out.contains("repro:"), "an async ring must not print a repro block:\n{out}");

    let before = fixture.read("architecture.spec.toml");
    let output = fixture.run(&["verify", "--write-exemptions"]);
    let out = stdout(&output);
    assert!(output.status.success(), "the advisory is green, so the seeding pass exits 0:\n{out}");
    assert!(!out.contains("wrote "), "an async-only tree seeds no exempt rows:\n{out}");
    let after = fixture.read("architecture.spec.toml");
    assert_eq!(before, after, "`--write-exemptions` must not touch the spec for an async-only tree");
    assert!(!after.contains("[[exempt]]"), "no [[exempt]] rows may appear for async rings:\n{after}");
}

/// A declared endpoint the claims table never claims is a spec typo — the
/// entry is CHECKED, not claimed: a named error finding, exit 1 (D6: a spec
/// typo must not buy silence).
#[test]
fn unclaimed_async_endpoint_refuses_by_name() {
    let fixture = round_trip_fixture();
    // An entry pointing at a unit no claim names and no selector selects.
    // The event names a real record so only the territory arm fires.
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "{}\n[[async_depends]]\nfrom = \"ghost\"\nto = \"orders\"\nevent = \"catalog::EventTwo\"\n",
            fixture.read("architecture.spec.toml")
        ),
    );

    let output = fixture.run(&["verify"]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(
        out.contains("async endpoint unclaimed: ghost — ghost -> orders (entry #3)"),
        "the unclaimed endpoint must name itself with the entry index:\n{out}\n--- stderr ---\n{err}"
    );
    assert!(!output.status.success(), "an unclaimed endpoint is an error finding:\n{out}");
}

/// The `to` side is checked as well as the `from` side.
#[test]
fn unclaimed_async_to_endpoint_refuses_by_name() {
    let fixture = round_trip_fixture();
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "{}\n[[async_depends]]\nfrom = \"catalog\"\nto = \"nowhere\"\nevent = \"catalog::EventTwo\"\n",
            fixture.read("architecture.spec.toml")
        ),
    );

    let output = fixture.run(&["verify"]);
    let out = stdout(&output);
    assert!(
        out.contains("async endpoint unclaimed: nowhere — catalog -> nowhere (entry #3)"),
        "the unclaimed `to` endpoint must name itself:\n{out}"
    );
    assert!(!output.status.success(), "an unclaimed endpoint is an error finding");
}

/// An async ring whose endpoints are fully claimed prints no territory row —
/// the check fires on typos, not on healthy specs.
#[test]
fn claimed_async_ring_endpoints_print_no_territory_row() {
    let fixture = mixed_fixture();
    let out = stdout(&fixture.run(&["verify"]));
    assert!(!out.contains("async endpoint unclaimed"), "healthy endpoints must not produce territory rows:\n{out}");
}

/// Counterweight for D8: an empty `[[async_depends]]` table keeps the whole
/// census silent — no new refusal class appears for it.
#[test]
fn empty_async_depends_table_refuses_nothing_new() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[package]\nname = \"solo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("src/lib.rs", "pub struct Widget;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"solo\"\nmatches = { units = [\"solo\"] }\n",
    );

    let output = fixture.run(&["verify"]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(out.contains("ok:"), "a green tree stays green:\n{out}");
    assert!(
        !out.contains("async endpoint unclaimed") && !out.contains("async cycle:"),
        "the async arms must stay silent when the table is empty:\n{out}\n--- stderr ---\n{err}"
    );
}

/// `archspec explain <source> <target>` gains the async arm: a pair carrying
/// async legs prints the matched `[[async_depends]]` rows in a dedicated
/// section.
#[test]
fn explain_names_async_depends_rows_for_a_pair_with_async_legs() {
    let fixture = mixed_fixture();

    let output = fixture.run(&["explain", "api", "core"]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(
        out.contains("async edges ([[async_depends]]):"),
        "explain must render the async section:\n{out}\n--- stderr ---\n{err}"
    );
    assert!(
        out.contains("core -> api — event core::AppEvent (entry #1)"),
        "the matched entry must print as a named row:\n{out}"
    );
    assert!(
        out.contains("api -> core — event api::OrderPlaced (entry #2)"),
        "the reverse-direction entry must match the same pair:\n{out}"
    );
}

/// A pair with no async legs prints no async section at all.
#[test]
fn explain_stays_quiet_about_async_for_a_direct_only_pair() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[package]\nname = \"solo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n");
    fixture.write("src/lib.rs", "pub struct Widget;\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"solo\"\nmatches = { units = [\"solo\"] }\n",
    );

    let output = fixture.run(&["explain", "solo", "solo"]);
    let out = stdout(&output);
    assert!(!out.contains("async edges"), "a pair no [[async_depends]] entry names prints no async section:\n{out}");
}
