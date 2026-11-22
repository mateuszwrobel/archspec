//! Derived drift census cells (workplan archspec_async_depends, scenario 04):
//! the shape both sides derive from source and the census speaks on exactly
//! three shapes — the consumer binds a field absent from the publisher
//! (ONE warning-class row, green in BOTH modes — the scenario 09 severity law
//! retired the old "`--strict` red" reading: strict arms detection depth
//! only, never an async row's severity),
//! an enum-on-one-side string projection (its OWN named class), and silence
//! everywhere else: legitimate subset consumption, dynamic or map bindings,
//! and drivers whose event-fact row reads `not-emitted`.

mod common;

use common::{Fixture, stderr, stdout};

/// The three-project C# tree of `tests/async_depends_spec.rs`, with the
/// publisher payload spelled by `events_src` (the App.Contracts declarations)
/// and the consumer handler spelled by `worker_code` (the App.Worker class
/// body facts). Both endpoints `using` the contracts namespace and reference
/// the project, so the identity arms pass and the ONLY findings a run can
/// carry are the derived drift rows.
fn drift_tree(events_src: &str, worker_code: &str, spec: &str) -> Fixture {
    let fixture = Fixture::new();
    let referenced_csproj = concat!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n",
        "    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n",
        "  <ItemGroup>\n    <ProjectReference Include=\"..\\App.Contracts\\App.Contracts.csproj\" />\n",
        "  </ItemGroup>\n</Project>\n",
    );
    fixture.write("App.Api/App.Api.csproj", referenced_csproj);
    fixture.write(
        "App.Api/OrdersController.cs",
        "using App.Contracts;\nnamespace App.Api;\npublic class OrdersController { }\n",
    );
    fixture.write("App.Worker/App.Worker.csproj", referenced_csproj);
    fixture.write(
        "App.Worker/OrdersWorker.cs",
        &format!(
            "using App.Contracts;\nnamespace App.Worker;\n{worker_code}\n"
        ),
    );
    fixture.write(
        "App.Contracts/App.Contracts.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
         <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write("App.Contracts/Events.cs", events_src);
    fixture.write("architecture.spec.toml", spec);
    fixture
}

/// The declared edge App.Api -> App.Worker over `App.Contracts.OrderPlaced`,
/// declared with identity only — the payload shape rides neither side of the
/// TOML (D2: derived, never declared).
fn declared_edge_spec() -> String {
    concat!(
        "[project]\nlanguage = \"csharp\"\n\n",
        "[[module]]\nname = \"api\"\nmatches = { units = [\"App.Api\"], modules = [\"App::Api\"] }\n\n",
        "[module.allowed]\ndepend_on = [\"contracts\"]\n\n",
        "[[module]]\nname = \"worker\"\nmatches = { units = [\"App.Worker\"], modules = [\"App::Worker\"] }\n\n",
        "[module.allowed]\ndepend_on = [\"contracts\"]\n\n",
        "[[module]]\nname = \"contracts\"\nmatches = { units = [\"App.Contracts\"], modules = [\"App::Contracts\"] }\n\n",
        "[[async_depends]]\nfrom = \"App.Api\"\nto = \"App.Worker\"\nevent = \"App.Contracts.OrderPlaced\"\n",
    )
    .to_string()
}

/// A consumer handler referencing `OrderId` (present) and `Status` (absent).
const DRIFT_WORKER: &str = "public class OrdersWorker\n{\n    \
     public void Handle(OrderPlaced order)\n    {\n        var id = order.OrderId;\n        \
     var s = order.Status;\n    }\n}";

// ---------------------------------------------------------------------------
// consumer binds field absent from publisher — the only shape that speaks
// ---------------------------------------------------------------------------

#[test]
fn absent_bound_field_warns_once_and_the_run_stays_green() {
    let tree = drift_tree(
        "namespace App.Contracts;\npublic record OrderPlaced(Guid OrderId, decimal Total);\n",
        DRIFT_WORKER,
        &declared_edge_spec(),
    );
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the drift census rides the WARNING channel — without --strict the run is green:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let report = stdout(&output);
    let drift_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift"))
        .collect();
    assert_eq!(
        drift_lines.len(),
        1,
        "exactly ONE warning-class row names the drift:\n{report}"
    );
    let line = drift_lines[0];
    assert!(
        line.contains("async drift field absent:") && line.contains("warning:"),
        "the row rides the warning channel under its own named class: {line}"
    );
    assert!(
        line.contains("App.Worker")
            && line.contains("App.Contracts.OrderPlaced")
            && line.contains(".Status"),
        "the row names consumer, event identity and absent field: {line}"
    );
    assert!(
        line.contains("(entry #1)"),
        "the row carries the entry marker: {line}"
    );
    assert!(
        !report.contains("async drift enum projection"),
        "a plain absent field never speaks as the projection class:\n{report}"
    );
}

#[test]
fn the_same_fixture_under_strict_exits_zero_with_the_warning_label() {
    let tree = drift_tree(
        "namespace App.Contracts;\npublic record OrderPlaced(Guid OrderId, decimal Total);\n",
        DRIFT_WORKER,
        &declared_edge_spec(),
    );
    let output = tree.run(&["verify", ".", "--strict"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "--strict arms census DETECTION depth only — an async drift row is never \
         promoted to an error (scenario 09 severity law):\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let report = stdout(&output);
    assert!(
        report.contains("warning: async drift field absent:"),
        "the strict run names the category WITH its warning label intact:\n{report}"
    );
}

// ---------------------------------------------------------------------------
// enum -> string projection names its own class
// ---------------------------------------------------------------------------

#[test]
fn enum_projected_to_string_names_its_own_projection_class() {
    let tree = drift_tree(
        "namespace App.Contracts;\npublic enum OrderStatus { New, Placed }\n\
         public record OrderPlaced(OrderStatus Status);\n",
        "public class OrdersWorker\n{\n    \
         public void Handle(OrderPlaced order)\n    {\n        var s = order.Status.ToString();\n \
         if (order.Status == \"Placed\") { Console.WriteLine(s); } \
         using System;\n    }\n}",
        &declared_edge_spec(),
    );
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a projection is a warning, not an error:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let report = stdout(&output);
    let projection_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift enum projection:"))
        .collect();
    assert_eq!(
        projection_lines.len(),
        1,
        "the projection speaks exactly once, as its OWN class:\n{report}"
    );
    let line = projection_lines[0];
    assert!(
        line.contains("App.Worker")
            && line.contains("App.Contracts.OrderPlaced")
            && line.contains(".Status"),
        "the projection row names consumer, event identity and field: {line}"
    );
    assert!(
        !report.contains("async drift field absent"),
        "a bound-and-present enum field is never the field-absent class — a \
         projection, not an equality failure:\n{report}"
    );
    // The scenario 09 severity law: `--strict` never promotes this class —
    // the row rides the strict run with its warning label.
    let strict = tree.run(&["verify", ".", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(0),
        "--strict gates nothing over the projection class (severity law):\n{}",
        stdout(&strict)
    );
    assert!(
        stdout(&strict).contains("warning: async drift enum projection:"),
        "the strict run names the projection category WITH its warning label:\n{}",
        stdout(&strict)
    );
}

// ---------------------------------------------------------------------------
// silence laws: subset, dynamic bind, not-emitted driver
// ---------------------------------------------------------------------------

#[test]
fn legitimate_subset_consumption_prints_nothing() {
    let tree = drift_tree(
        "namespace App.Contracts;\npublic record OrderPlaced(Guid OrderId, decimal Total);\n",
        "public class OrdersWorker\n{\n    \
         public void Handle(OrderPlaced order)\n    {\n        var id = order.OrderId;\n    }\n}",
        &declared_edge_spec(),
    );
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the consumer binds fewer fields, all present — green:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert!(
        !report.contains("async drift"),
        "subset consumption stays silent — it is the lawful shape:\n{report}"
    );
    assert!(
        !report.contains("warning:"),
        "the silent run prints no warning line of any class:\n{report}"
    );
}

#[test]
fn a_dynamic_or_map_binding_derives_nothing_and_prints_nothing() {
    // M5: the handler binds the payload as a `JsonDocument`, not as the
    // record type — the bound shape is NOT derivable, and the census never
    // guesses (M5 silence, never a guess).
    let tree = drift_tree(
        "namespace App.Contracts;\npublic record OrderPlaced(Guid OrderId, decimal Total);\n",
        "using System.Text.Json;\npublic class OrdersWorker\n{\n    \
         public void Handle(string json)\n    {\n        var doc = \
         JsonDocument.Parse(json);\n        var s = doc.RootElement.GetProperty(\"Status\");\n    \
         }\n}",
        &declared_edge_spec(),
    );
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "an unboundable consumer side must never be read as drift:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert!(
        !report.contains("async drift"),
        "a dynamic/Json.Document bind prints NOTHING:\n{report}"
    );
}

#[test]
fn a_not_emitted_driver_stays_silent_with_the_capability_named_note() {
    // Rust's event-declaration-derivation row reads not-emitted: the shape
    // facts are never derived, so the census is inert — the existing
    // capability-named inert note states the silence, and no drift byte
    // appears (never a refusal of its own).
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"driftnone\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "fn hidden() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \
         \"driftnone\"\nmatches = { units = [\"driftnone\"] }\n\n[[async_depends]]\nfrom = \
         \"driftnone\"\nto = \"driftnone\"\nevent = \"Ghost.Events.OrderPlaced\"\n",
    );
    let output = fixture.run(&["verify"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the inert driver must not be reddened by the drift census:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert!(
        !report.contains("async drift"),
        "a not-emitted driver prints no drift byte at all:\n{report}"
    );
    assert!(
        report.contains(
            "note: async_depends publish and subscribe arms inert for rust: \
             event-declaration-derivation = not-emitted"
        ),
        "the census's inertness is stated by the existing capability-named note:\n{report}"
    );
}

// ---------------------------------------------------------------------------
// consumer-fix round 1 (scenario 07): base-following, both-sides silence,
// live projection row — the witness-corpus verdict's three named classes on the
// 0.6.86 census.
// ---------------------------------------------------------------------------

/// The base the consumer's contracts tree writes: an `IntegrationEvent`-
/// style shared base whose members the publisher record inherits.
const EVENT_BASE: &str = "public record IntegrationEventBase\n{\n    \
     public Guid BuyerIdentityGuid { get; init; }\n    \
     public DateTime OccurredOn { get; init; }\n}";

/// A consumer handler referencing the BASE-declared fields and the record's
/// own field — the shape whose base-declared bindings printed 13 false
/// "the publisher record declares no such field" rows on the verdict tree.
const BASE_BINDING_WORKER: &str = "public class OrdersWorker\n{\n    \
     public void Handle(OrderPlaced integrationEvent)\n    {\n        var g = \
     integrationEvent.BuyerIdentityGuid;\n        var t = \
     integrationEvent.OccurredOn;\n        var n = integrationEvent.OrderNumber;\n var x = \
     }\n}";

#[test]
fn a_base_declared_field_resolves_present_and_prints_nothing() {
    // Scenario 07 fixture one: the publisher record inherits a shared base,
    // the consumer binds a base-declared field — the publisher side must
    // follow the base with the census's own model-wide name law, and the
    // run prints NOTHING (the base-blind class is closed).
    let events = format!(
        "namespace App.Contracts;\n{EVENT_BASE}\npublic record OrderPlaced : \
         IntegrationEventBase\n{{\n    public int OrderNumber {{ get; init; }}\n}}\n"
    );
    let tree = drift_tree(&events, BASE_BINDING_WORKER, &declared_edge_spec());
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "base-declared bindings are PRESENT facts — the run is green:\n{report}"
    );
    assert!(
        !report.contains("async drift"),
        "a base-declared field prints NOTHING — no absent row, no other drift byte:\n{report}"
    );
    assert!(
        !report.contains("warning:"),
        "the silent run prints no warning line of any class:\n{report}"
    );
    let strict = tree.run(&["verify", ".", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(0),
        "nothing was suppressed into the warning channel — --strict stays green:\n{}{}",
        stdout(&strict),
        stderr(&strict)
    );
    assert!(
        !stdout(&strict).contains("async drift"),
        "the base-declared silence is real silence under --strict too:\n{}",
        stdout(&strict)
    );
}

#[test]
fn a_field_declared_identically_on_both_sides_prints_nothing() {
    // Scenario 07 fixture two — the `BuyerIdentityGuid` shape of the verdict's
    // entries #11/#14/#15: the duplicated-contract idiom declares one
    // identical copy of the base AND the record on each side of the edge.
    // An identical-name, identical-type field declared on both sides never
    // warns — the resolution reads every same-simple-name copy and every
    // base they chain to, so the absent class cannot fire on it.
    let events = format!(
        "namespace App.Contracts;\n{EVENT_BASE}\npublic record OrderPlaced : \
         IntegrationEventBase\n{{\n    public int OrderNumber {{ get; init; }}\n}}\n"
    );
    let worker = format!(
        "{EVENT_BASE}\npublic record OrderPlaced : IntegrationEventBase\n{{\n    \
         public int OrderNumber {{ get; init; }}\n}}\n{BASE_BINDING_WORKER}"
    );
    let tree = drift_tree(&events, &worker, &declared_edge_spec());
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "a field declared identically on both sides is declared, never absent:\n{report}"
    );
    assert!(
        !report.contains("async drift"),
        "the both-sides-identical law prints NOTHING — the consumer false-positive \
         class is closed:\n{report}"
    );
    assert!(
        !report.contains("warning:"),
        "the silent run prints no warning line of any class:\n{report}"
    );
}

#[test]
fn an_enum_string_projection_prints_the_named_row_once_per_qualifying_entry() {
    // Scenario 07 fixture three — the class the verdict found DEAD: the
    // publisher's enum member rides a shared base, the consumer binds it as
    // a string. On the 0.6.86 census the member lookup missed the base and
    // the projection arm was never reached; the fix makes it live. The row
    // speaks EXACTLY ONCE per qualifying entry (two qualifying fields, one
    // row) and `--strict` never promotes it (scenario 09 severity law).
    let events = concat!(
        "namespace App.Contracts;\npublic enum OrderStatus { New, Paid }\npublic enum \
         OrderKind { Standard, Express }\n",
        "public record IntegrationEventBase\n{\n    public OrderStatus Status { get; init; }\n    \
         public OrderKind Kind { get; init; }\n}\n",
        "public record OrderPlaced : IntegrationEventBase\n{\n    public int OrderNumber { get; \
         init; }\n}\n",
    );
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced integrationEvent)\n    {\n        var s = \
                  integrationEvent.Status.ToString();\n        var k = \
                  integrationEvent.Kind.ToString();\n        if (integrationEvent.Status == \
                  \"Paid\") { Console.WriteLine(s); }\n        using System;\n    }\n}";
    let tree = drift_tree(events, worker, &declared_edge_spec());
    let output = tree.run(&["verify", "."]);
    let report = stdout(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a projection is a warning, not an error:\n{report}{}",
        stderr(&output)
    );
    let projection_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift enum projection:"))
        .collect();
    assert_eq!(
        projection_lines.len(),
        1,
        "the live projection class speaks EXACTLY ONCE per qualifying entry:\n{report}"
    );
    let line = projection_lines[0];
    assert!(
        line.contains("warning:")
            && line.contains("App.Worker")
            && line.contains("App.Contracts.OrderPlaced.")
            && line.contains("(entry #1)"),
        "the row rides the warning channel naming consumer, event identity, field and \
         entry: {line}"
    );
    assert!(
        !report.contains("async drift field absent"),
        "a base-declared, bound-and-present enum field is never the field-absent class — \
         a projection, not an equality failure:\n{report}"
    );
    let strict = tree.run(&["verify", ".", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(0),
        "--strict never promotes the live projection class (scenario 09 severity law):\n{}",
        stdout(&strict)
    );
    let strict_report = stdout(&strict);
    assert!(
        strict_report.contains("warning: async drift enum projection:")
            && strict_report.contains("(entry #1)"),
        "the strict run keeps the warning label on the projection row:\n{strict_report}"
    );
}

// ---------------------------------------------------------------------------
// consumer re-run refinement (2026-11-17, binding steering on scenario 07):
// the witness-corpus base chain runs TWO HOPS and crosses a project boundary — the
// event record lives in the unit's own namespace, its far base in a unit the
// publisher record does not share. The Id-FP class must land at EXACTLY 0.
// ---------------------------------------------------------------------------

/// The witness-corpus `IntegrationEvent` shape: declared under its own (third) unit's
/// namespace in that unit's file, reached only at the SECOND hop of the
/// publisher record's base chain.
const FAR_BASE_RECORDS: &str = "namespace App.Abstractions;\npublic record IntegrationEvent\n\
     {\n    public Guid EventId { get; init; }\n    public DateTime OccurredOn { get; init; }\n}";

/// Publisher records: `OrderPlaced` (own field) -> `BaseOrderEvent` (same
/// unit) -> `IntegrationEvent` (the far unit) — two hops, one across the
/// project boundary, the witness-corpus chain shape. `extra` prepends declarations.
fn far_chain_events(extra: &str) -> String {
    format!(
        "namespace App.Contracts;\n{extra}\
         public record OrderPlaced : BaseOrderEvent\n{{\n    \
         public int OrderNumber {{ get; init; }}\n}}\n\
         public record BaseOrderEvent : IntegrationEvent\n{{\n}}\n"
    )
}

/// The drift fixture with the far base declared in App.Api's file.
fn far_chain_tree(events_src: &str, worker_code: &str, far_base: &str) -> Fixture {
    let tree = drift_tree(events_src, worker_code, &declared_edge_spec());
    tree.write("App.Api/IntegrationEvent.cs", far_base);
    tree
}

#[test]
fn a_two_hop_base_chain_across_units_resolves_present_and_only_a_phantom_warns() {
    // Steering pin 2 + the no-over-silencing law: the census follows the
    // chain across the unit boundary — base-declared bindings print
    // NOTHING — while a field declared NOWHERE in the chain still prints
    // its one named `async drift field absent` row.
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced integrationEvent)\n    {\n        var g = \
                  integrationEvent.EventId;\n        var t = \
                  integrationEvent.OccurredOn;\n        var n = integrationEvent.OrderNumber;\n var x = \
                  integrationEvent.NotDeclaredAnywhere;\n    }\n}";
    let tree = far_chain_tree(&far_chain_events(""), worker, FAR_BASE_RECORDS);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "chain-declared bindings are PRESENT; the phantom warning keeps the run green:\n{report}"
    );
    let absent_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift field absent:"))
        .collect();
    assert_eq!(
        absent_lines.len(),
        1,
        "the no-over-silencing law: exactly one phantom field warns by name — the Id \
         class lands at EXACTLY zero:\n{report}"
    );
    assert!(
        absent_lines[0].contains("OrderPlaced.NotDeclaredAnywhere"),
        "the phantom row names the field no chain link declares: {}",
        absent_lines[0]
    );
    for field in ["EventId", "OccurredOn", "OrderNumber"] {
        assert!(
            !absent_lines[0].contains(&format!("OrderPlaced.{field}")),
            "a field declared on the chain — two hops away across the boundary — prints \
             NOTHING ({field}): {}",
            absent_lines[0]
        );
    }
}

#[test]
fn an_enum_on_the_far_base_bound_as_string_fires_the_projection_class() {
    // Steering pin 3's dead-class probe: the publisher's enum member rides
    // the far base (two hops, across the unit boundary) and the subscriber
    // binds it as a string — the class that printed ZERO rows on the
    // consumer's tree must FIRE here, exactly once — and `--strict` never
    // promotes it (scenario 09 severity law).
    let events = far_chain_events("public enum OrderStatus { New, Paid }\n");
    let far = FAR_BASE_RECORDS.replace(
        "public DateTime OccurredOn { get; init; }",
        "public DateTime OccurredOn { get; init; }\n    public OrderStatus Status { get; init; }",
    );
    let tree = far_chain_tree(
        &events,
        "public class OrdersWorker\n{\n    \
         public void Handle(OrderPlaced integrationEvent)\n    {\n        var s = \
         integrationEvent.Status.ToString();\n        using System;\n        \
         Console.WriteLine(s);\n    }\n}",
        &far,
    );
    let output = tree.run(&["verify", "."]);
    let report = stdout(&output);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the projection is a warning, not an error:\n{report}{}",
        stderr(&output)
    );
    let projection_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift enum projection:"))
        .collect();
    assert_eq!(
        projection_lines.len(),
        1,
        "the resurrected class FIRES on the far-base enum — exactly one row:\n{report}"
    );
    assert!(
        projection_lines[0].contains("OrderPlaced.Status")
            && projection_lines[0].contains("(entry #1)"),
        "the row names the event, the far-base field and the entry: {}",
        projection_lines[0]
    );
    assert!(
        !report.contains("async drift field absent"),
        "a far-base-declared, bound-and-present enum is never the absent class:\n{report}"
    );
    let strict = tree.run(&["verify", ".", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(0),
        "--strict never promotes the class on the cross-unit chain (scenario 09):\n{}",
        stdout(&strict)
    );
    assert!(
        stdout(&strict).contains("warning: async drift enum projection:"),
        "the strict run keeps the projection row advisory, label intact:\n{}",
        stdout(&strict)
    );
}

// ---------------------------------------------------------------------------
// ---------------------------------------------------------------------------
// consumer-fix round 2 (scenario 09): the severity law — `--strict` arms
// census DETECTION depth only and never promotes an async census row. The
// consumer's strict diff (issue #44, RESIDUAL 1) must be ZERO lines.
// ---------------------------------------------------------------------------

/// The spec of the three-row fixture: the drift entry (entry #1, the shape
/// `declared_edge_spec` declares) plus the reverse entry (entry #2) closing
/// the pure-async ring. The `StockUpdated` identity is declared in the shared
/// contracts unit and referenced from BOTH endpoint units, so the identity
/// arms and the territory check pass silently — the run's ONLY findings are
/// the three async census rows.
fn three_row_spec() -> String {
    format!(
        "{}\n[[async_depends]]\nfrom = \"App.Worker\"\nto = \"App.Api\"\nevent = \
         \"App.Contracts.StockUpdated\"\n",
        declared_edge_spec()
    )
}

#[test]
fn strict_parity_the_three_async_rows_exit_zero_with_identical_bytes() {
    // Scenario 09 done-when cell: ONE run carrying the RING row, the
    // ENUM-PROJECTION row and the FIELD-ABSENT row exits 0 under BOTH
    // `verify` and `verify --strict` with byte-identical stdout, every row
    // keeping its `warning: ` prefix in both modes — the strict-vs-plain
    // diff the consumer CI must now see is exactly zero lines.
    let events = concat!(
        "namespace App.Contracts;\npublic enum OrderStatus { New, Placed }\n",
        "public record OrderPlaced(OrderStatus Status, decimal Total);\n",
        "public record StockUpdated(int Count);\n",
    );
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced order)\n    {\n        var s = \
                  order.Status.ToString();\n        if (order.Status == \"Placed\") { \
                  Console.WriteLine(s); }\n        var c = order.Currency;\n        using \
                  System;\n    }\n}";
    let tree = drift_tree(events, worker, &three_row_spec());
    let lax = tree.run(&["verify", "."]);
    let strict = tree.run(&["verify", ".", "--strict"]);
    for (label, output) in [("verify", &lax), ("verify --strict", &strict)] {
        assert_eq!(
            output.status.code(),
            Some(0),
            "{label} must stay green over async census rows only:\n{}{}",
            stdout(output),
            stderr(output)
        );
    }
    let (lax_out, strict_out) = (stdout(&lax), stdout(&strict));
    assert_eq!(
        lax_out, strict_out,
        "the strict run's diff against the plain run is ZERO lines (issue #44 \
         RESIDUAL 1: the ring line lost its warning: prefix and reddened CI)"
    );
    for (label, report) in [("plain", &lax_out), ("strict", &strict_out)] {
        let ring: Vec<&str> = report.lines().filter(|l| l.contains("async cycle:")).collect();
        assert_eq!(ring.len(), 1, "{label}: exactly one ring row:\n{report}");
        assert!(
            ring[0]
                .contains("warning: async cycle: App.Api -> App.Worker -> App.Api (async)"),
            "{label}: the ring row keeps its warning: prefix: {}",
            ring[0]
        );
        assert_eq!(
            report
                .lines()
                .filter(|l| l.contains("warning: async drift enum projection:"))
                .count(),
            1,
            "{label}: the projection row keeps its warning: prefix:\n{report}"
        );
        assert_eq!(
            report
                .lines()
                .filter(|l| l.contains("warning: async drift field absent:"))
                .count(),
            1,
            "{label}: the field-absent row keeps its warning: prefix:\n{report}"
        );
        assert!(
            !report.lines().any(|l| {
                (l.contains("async cycle:") || l.contains("async drift"))
                    && !l.starts_with("  warning: ")
            }),
            "{label}: NO async census row renders without its warning: prefix:\n{report}"
        );
    }
}

#[test]
fn a_direct_warning_cycle_still_exits_one_under_strict() {
    // The control arm of the severity law: DIRECT-dependency strict promotion
    // is UNCHANGED byte-for-byte — a warning-severity `no_cycles` violation
    // still promotes under `--strict`: the line loses the `warning: ` label
    // and the run exits 1 (the shipped shape, mirrored from
    // `tests/verify.rs::verify_scenario_thirteen`).
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"crates/auth\", \"crates/billing\"]\n",
    );
    fixture.write(
        "crates/auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nbilling = { path = \"../billing\" }\n",
    );
    fixture.write("crates/auth/src/lib.rs", "pub fn auth() {}\n");
    fixture.write(
        "crates/billing/Cargo.toml",
        "[package]\nname = \"billing\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nauth = { path = \"../auth\" }\n",
    );
    fixture.write("crates/billing/src/lib.rs", "pub fn bill() {}\n");
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"auth\"\nmatches = { units = [\"auth\"] }\n\n[module.allowed]\ndepend_on = [\"billing\"]\n\n[[module]]\nname = \"billing\"\nmatches = { units = [\"billing\"] }\n\n[module.allowed]\ndepend_on = [\"auth\"]\n\n[[constraint]]\ntype = \"no_cycles\"\nmodules = [\"auth\", \"billing\"]\nseverity = \"warning\"\n",
    );
    let lax = fixture.run(&["verify"]);
    assert_eq!(
        lax.status.code(),
        Some(0),
        "a warning-severity cycle stays green without --strict:\n{}{}",
        stdout(&lax),
        stderr(&lax)
    );
    assert!(
        stdout(&lax).contains("warning: cycle: auth -> billing -> auth"),
        "the plain run labels the direct warning:\n{}",
        stdout(&lax)
    );
    let strict = fixture.run(&["verify", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(1),
        "the DIRECT strict promotion is unchanged: the warning promotes to a finding:\n{}",
        stdout(&strict)
    );
    let out = stdout(&strict);
    assert!(
        out.contains("cycle: auth -> billing -> auth") && !out.contains("warning: cycle:"),
        "the promoted direct warning renders without the warning label:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// consumer-fix round 2 (scenario 10): the duplicated-contract projection
// pairing — the enum→string class that printed ZERO rows on the consumer's
// tree because the pairing keyed on the shared-FQN binding (consumer verdict
// on archspec 0.6.89, issue #44, RESIDUAL 2; the pasync probes confirmed the
// class both ways on the same tree). Severity under `--strict` is scenario
// 09's law — these cells pin the row COUNTS.
// ---------------------------------------------------------------------------

/// The duplicated-contract drift tree: `App.Api` and `App.Worker` are
/// standalone projects — no shared contracts project, no csproj reference, no
/// cross-unit `using` — each declaring its OWN namespaced copy of
/// `OrderPlaced`. The publisher's copy types the status member `OrderStatus`,
/// the worker's copy declares the same member as a plain `string`, and the
/// handler binds it through a typed `Handle` parameter with NO projection
/// syntax — the witness-corpus shape that printed zero rows. `enum_in_third_unit`
/// moves the ENUM declaration to a THIRD project (`App.Enums`) the publisher
/// file names; the pairing must print its row whichever unit hosts the enum.
/// `worker_binds` names the single field the handler reads — the never-bound
/// silence pin binds only `OrderId`.
fn duplicated_projection_tree(enum_in_third_unit: bool, worker_binds: &str) -> Fixture {
    let fixture = Fixture::new();
    let plain_csproj = concat!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n",
        "    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write("App.Api/App.Api.csproj", plain_csproj);
    let api_enums = if enum_in_third_unit {
        "using App.Enums;\n"
    } else {
        "public enum OrderStatus { New, Paid }\n"
    };
    fixture.write(
        "App.Api/Events.cs",
        &format!(
            "{api_enums}namespace App.Api.Events;\npublic record OrderPlaced\n{{\n    \
             public Guid OrderId {{ get; init; }}\n    public OrderStatus Status {{ get; init; }}\n}}\n"
        ),
    );
    fixture.write(
        "App.Api/OrdersController.cs",
        "using App.Api.Events;\nnamespace App.Api;\npublic class OrdersController { }\n",
    );
    fixture.write("App.Worker/App.Worker.csproj", plain_csproj);
    fixture.write(
        "App.Worker/Events.cs",
        "namespace App.Worker.Events;\npublic record OrderPlaced\n{\n    \
         public Guid OrderId { get; init; }\n    public string Status { get; init; }\n}\n",
    );
    fixture.write(
        "App.Worker/OrderPlacedHandler.cs",
        &format!(
            "using App.Worker.Events;\nnamespace App.Worker;\npublic class OrderPlacedHandler : \
             IIntegrationEventHandler<OrderPlaced>\n{{\n    \
             public Task Handle(OrderPlaced integrationEvent, CancellationToken cancellationToken)\n    \
             {{\n        var bound = integrationEvent.{worker_binds};\n        return Task.CompletedTask;\n    \
             }}\n}}\n"
        ),
    );
    if enum_in_third_unit {
        fixture.write("App.Enums/App.Enums.csproj", plain_csproj);
        fixture.write(
            "App.Enums/OrderStatus.cs",
            "namespace App.Enums;\npublic enum OrderStatus { New, Paid }\n",
        );
    }
    let enums_module = if enum_in_third_unit {
        "[[module]]\nname = \"enums\"\nmatches = { units = [\"App.Enums\"], modules = [\"App::Enums\"] }\n\n"
    } else {
        ""
    };
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"api\"\nmatches = {{ \
             units = [\"App.Api\"], modules = [\"App::Api\", \"App::Api::Events\"] }}\n\n{enums_module}\
             [[module]]\nname = \"worker\"\nmatches = {{ units = [\"App.Worker\"], modules = \
             [\"App::Worker\", \"App::Worker::Events\"] }}\n\n[[async_depends]]\nfrom = \"App.Api\"\n\
             to = \"App.Worker\"\nevent = \"App.Api.Events.OrderPlaced\"\n"
        ),
    );
    fixture
}

/// Every new duplicated-contract cell asserts EXACTLY ONE named projection
/// row on a green run: the census rides the warning channel (scenario 07's
/// channel law stands), the pairing no longer depends on which unit hosts
/// the enum, and the severity dial is scenario 09's surface.
fn assert_projection_row_once(tree: &Fixture, field: &str) {
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "a projection is a warning, not an error:\n{report}"
    );
    let projection_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift enum projection:"))
        .collect();
    assert_eq!(
        projection_lines.len(),
        1,
        "the duplicated-contract pairing speaks the named row EXACTLY ONCE:\n{report}"
    );
    let line = projection_lines[0];
    assert!(
        line.contains("warning:")
            && line.contains("App.Worker")
            && line.contains("App.Api.Events.OrderPlaced.")
            && line.contains(&format!(".{field}"))
            && line.contains("(entry #1)"),
        "the row rides the warning channel naming consumer, event identity, \
         field and entry: {line}"
    );
    assert!(
        !report.contains("async drift field absent"),
        "a duplicated-contract string field is DECLARED — never the absent \
         class:\n{report}"
    );
}

#[test]
fn a_duplicated_contract_enum_to_string_projection_prints_the_named_row_once() {
    // The enum declared in the PUBLISHER's unit: the projection rides the
    // pair of per-unit declarations, not the binding syntax — the class the
    // consumer verdict found silent on 3 qualifying witness-corpus entries.
    let tree = duplicated_projection_tree(false, "Status");
    assert_projection_row_once(&tree, "Status");
}

#[test]
fn the_projection_row_prints_once_with_the_enum_declared_in_a_third_unit() {
    // The enum declared in a THIRD unit: the publisher-side enum leg already
    // reads the model-wide census, so the pairing prints the SAME row,
    // still exactly once, regardless of which unit hosts the enum.
    let tree = duplicated_projection_tree(true, "Status");
    assert_projection_row_once(&tree, "Status");
}

#[test]
fn a_publisher_field_the_subscriber_never_binds_prints_nothing() {
    // The never-bound silence pin: the publisher declares the enum member
    // and the subscriber's copy types it as string, but the handler binds
    // only `OrderId` — an unbound field prints NOTHING.
    let tree = duplicated_projection_tree(false, "OrderId");
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "a never-bound field keeps the run green:\n{report}"
    );
    assert!(
        !report.contains("async drift"),
        "the subscriber never binds the projected field — the census stays \
         silent:\n{report}"
    );
    assert!(
        !report.contains("warning:"),
        "the silent run prints no warning line of any class:\n{report}"
    );
}

#[test]
fn a_declared_string_field_the_handler_never_reads_prints_nothing() {
    // The bind-site weld (consumer verdict on 0.6.91 — the liveness-probe
    // shape of entry #14): the enum→string projection census keys on HANDLER
    // USAGE SITES ("binds"), not on the subscriber record's DECLARATION
    // site. The worker's copy of `OrderPlaced` still DECLARES `Status` as a
    // plain string, but no handler body reads it — declaration alone is
    // SILENT, the subset-consumption-silent law (a corpus-scale witness proves it: 0 rows,
    // and a one-line handler read makes exactly 1 row appear). The
    // read-armed sibling — `a_duplicated_contract_enum_to_string_projection_
    // prints_the_named_row_once` — is that probe: the same tree speaks
    // EXACTLY ONE named row once a handler body reads the member.
    let tree = duplicated_projection_tree(false, "OrderId");
    tree.write(
        "App.Worker/OrderPlacedHandler.cs",
        "using App.Worker.Events;\nnamespace App.Worker;\npublic class OrderPlacedHandler : \
         IIntegrationEventHandler<OrderPlaced>\n{\n    public Task Handle(OrderPlaced \
         integrationEvent, CancellationToken cancellationToken)\n    {\n        return \
         Task.CompletedTask;\n    }\n}\n",
    );
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "a declared-but-unread string field keeps the run green:\n{report}"
    );
    let projection_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift enum projection:"))
        .collect();
    assert!(
        projection_lines.is_empty(),
        "the subscriber record DECLARES the string field but no handler body \
         reads it — the census keys on bindsites, not declarations:\n{report}"
    );
}

// ---------------------------------------------------------------------------
// issue #46 — scope identity at the binding layer (fix plan P4): binds are
// keyed (scope frame, variable) and a chain root resolves through the SCOPE
// CHAIN of its access site, innermost declaration first — never against
// every name ever bound in the file. Adversarial cells: the issue's own
// repro shapes, the invocation tails, the pattern arms that NOW speak, and
// the two silence laws (implicit lambda parameters, `var`) pinned blind.
// ---------------------------------------------------------------------------

/// The two event records the scope cells bind: `OrderPlaced` (entry #1) and
/// `PaymentReceived` (entry #2), disjoint field sets so any cross-type
/// attribution shows up as a named absent row.
const TWO_EVENTS: &str = "namespace App.Contracts;\npublic record OrderPlaced(Guid OrderId, \
     decimal Total);\npublic record PaymentReceived(Guid PaymentId);\n";

/// The declared-edge spec over BOTH records: App.Api -> App.Worker for each
/// (entry #1 = OrderPlaced, entry #2 = PaymentReceived), identity exactly as
/// `declared_edge_spec` builds it.
fn two_event_spec() -> String {
    format!(
        "{}\n[[async_depends]]\nfrom = \"App.Api\"\nto = \"App.Worker\"\nevent = \
         \"App.Contracts.PaymentReceived\"\n",
        declared_edge_spec()
    )
}

/// The drift tree over the two-record contract.
fn two_event_tree(worker_code: &str) -> Fixture {
    drift_tree(TWO_EVENTS, worker_code, &two_event_spec())
}

#[test]
fn two_methods_one_file_sharing_a_variable_name_bind_only_their_own_parameters() {
    // The issue's own repro (triage #46): the SAME name typed DIFFERENTLY in
    // two methods — every chain must resolve to the parameter of the method
    // it stands in. The file-global bind printed a false `field absent` row
    // on EACH side (PaymentId against OrderPlaced, OrderId and Total against
    // PaymentReceived); scoped binds print NOTHING.
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced evt)\n    {\n        var a = \
                  evt.OrderId;\n        var t = evt.Total;\n    }\n    \
                  public void HandlePayment(PaymentReceived evt)\n    {\n        var b = \
                  evt.PaymentId;\n    }\n}";
    let tree = two_event_tree(worker);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "every binding is scoped, every field is present — green:\n{report}"
    );
    assert!(
        !report.contains("async drift"),
        "a name in one method never borrows another method's binding:\n{report}"
    );
}

#[test]
fn two_classes_one_file_sharing_a_field_name_resolve_to_their_own_class_frame() {
    // M4's class-scope arm: fields bind ONE FRAME PER CLASS, not one file
    // frame — `_evt` typed differently per class must not join the other
    // type's field set on either side.
    let worker = "public class FirstWorker\n{\n    private OrderPlaced _evt;\n    \
                  public void Run()\n    {\n        var a = _evt.OrderId;\n    }\n}\n\
                  public class SecondWorker\n{\n    private PaymentReceived _evt;\n    \
                  public void Run()\n    {\n        var b = _evt.PaymentId;\n    }\n}";
    let tree = two_event_tree(worker);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "field binds are class-scoped — green:\n{report}"
    );
    assert!(
        !report.contains("async drift"),
        "a field of one class never attributes a chain of another class in the same file:\n\
         {report}"
    );
}

#[test]
fn invoking_tostring_or_gettype_on_a_bound_variable_binds_no_field() {
    // The invocation-shaped tail is a METHOD name, never a bound field:
    // `evt.ToString()` and `evt.GetType()` printed false `field absent:
    // ToString` / `field absent: GetType` rows on the file-global join
    // (issue #46's own symptom class); they now contribute NOTHING.
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced evt)\n    {\n        \
                  var s = evt.ToString();\n        var t = evt.GetType();\n    }\n    \
                  public void Pay(PaymentReceived payment) { }\n}";
    let tree = two_event_tree(worker);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "method tails bind nothing — green:\n{report}"
    );
    assert!(
        !report.contains("async drift"),
        "ToString/GetType never join the bound-field set:\n{report}"
    );
}

#[test]
fn a_switch_section_pattern_variable_binds_its_section_and_now_names_the_absent_field() {
    // Pattern arms are NEW (issue #46): chains on `case T p:` variables were
    // silent and NOW SPEAK — scoped to the declaring section, so the
    // same-named variable in the SIBLING section attributes only its own
    // type. Exactly ONE row: the OrderPlaced phantom; PaymentReceived stays
    // silent through its own section's binding.
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced known, object message)\n    {\n        \
                  switch (message)\n        {\n            case PaymentReceived p: \
                  Speak(p.PaymentId); break;\n            case OrderPlaced p: Speak(p.Nope); \
                  break;\n            default: break;\n        }\n    }\n}";
    let tree = two_event_tree(worker);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    let absent_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift field absent:"))
        .collect();
    assert_eq!(
        absent_lines.len(),
        1,
        "the switch-pattern chain speaks EXACTLY once, for the type its section declares:\n\
         {report}"
    );
    assert!(
        absent_lines[0].contains("App.Contracts.OrderPlaced.Nope")
            && absent_lines[0].contains("(entry #1)"),
        "the row names the declaring type's phantom field: {}",
        absent_lines[0]
    );
    assert!(
        absent_lines.iter().all(|line| !line.contains("PaymentReceived.")),
        "the sibling section's same-named variable never borrows the phantom:\n{report}"
    );
}

#[test]
fn an_is_pattern_variable_binds_its_block_and_names_the_absent_field() {
    // The `is` arm (issue #46): `message is OrderPlaced p` scopes p to the
    // enclosing block — its chains speak for OrderPlaced alone; the second
    // test's variable, typed PaymentReceived, contributes only its present
    // field.
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced known, object message)\n    {\n        \
                  if (message is OrderPlaced p) { var a = p.Nope; }\n        \
                  if (message is PaymentReceived q) { var b = q.PaymentId; }\n    }\n}";
    let tree = two_event_tree(worker);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    let absent_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift field absent:"))
        .collect();
    assert_eq!(
        absent_lines.len(),
        1,
        "the is-pattern chain speaks EXACTLY once:\n{report}"
    );
    assert!(
        absent_lines[0].contains("App.Contracts.OrderPlaced.Nope"),
        "the row names the pattern type's phantom field: {}",
        absent_lines[0]
    );
    assert!(
        absent_lines.iter().all(|line| !line.contains("PaymentReceived.")),
        "the is-pattern binding of the other type contributes nothing absent:\n{report}"
    );
}

#[test]
fn pattern_variables_sharing_a_name_across_two_methods_attribute_to_their_own_types() {
    // The triage v46-pattern shape: the SAME pattern-variable name in TWO
    // methods, typed differently — a file-scoped pattern bind would
    // attribute the OrderPlaced phantom to PaymentReceived too; the section-
    // scoped bind speaks exactly the one true row.
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(object message)\n    {\n        if (message is OrderPlaced \
                  p) { var a = p.Nope; }\n    }\n    \
                  public void HandlePayment(object message)\n    {\n        if (message is \
                  PaymentReceived p) { var b = p.PaymentId; }\n    }\n}";
    let tree = two_event_tree(worker);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    let absent_lines: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async drift field absent:"))
        .collect();
    assert_eq!(
        absent_lines.len(),
        1,
        "each method's pattern variable answers for its own type only:\n{report}"
    );
    assert!(
        absent_lines[0].contains("App.Contracts.OrderPlaced.Nope"),
        "the one true row names the phantom: {}",
        absent_lines[0]
    );
    assert!(
        absent_lines.iter().all(|line| !line.contains("PaymentReceived.")),
        "no phantom crosses the method boundary:\n{report}"
    );
}

#[test]
fn an_implicit_lambda_parameter_shadows_the_outer_binding_and_stays_silent() {
    // The silence pin (plan P4, unchanged law): implicit lambda parameters
    // bind NOTHING — and they SHADOW the same-named method parameter, so a
    // chain the lambda covers attributes to no type at all, not to the
    // shadowed parameter's record.
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced evt)\n    {\n        \
                  list.ForEach(evt => Speak(evt.Nope));\n    }\n    \
                  public void Pay(PaymentReceived payment) { }\n}";
    let tree = two_event_tree(worker);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "the implicit parameter binds nothing — green:\n{report}"
    );
    assert!(
        !report.contains("async drift"),
        "the lambda's own evt hides the method's evt — no attribution, no row:\n{report}"
    );
}

#[test]
fn a_var_declared_local_stays_blind_and_prints_nothing() {
    // The second silence pin (plan P4, unchanged law): `var` inference binds
    // NOTHING — every chain rooted at the implicitly typed local derives
    // nothing, in a file that binds the known record types elsewhere.
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced evt)\n    {\n        \
                  var msg = Repo.Get();\n        var a = msg.WhateverItMightBe;\n        var b = \
                  evt.OrderId;\n    }\n    public void Pay(PaymentReceived payment) { }\n}";
    let tree = two_event_tree(worker);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "the var binding is blind — green:\n{report}"
    );
    assert!(
        !report.contains("async drift"),
        "inference through `var` is M5 silence — the census never guesses a type:\n{report}"
    );
}

#[test]
fn sibling_blocks_declaring_the_same_name_resolve_innermost_first() {
    // The sibling-block arm of the scope walk: two blocks of ONE method,
    // each declaring `evt` with a different type — a chain resolves to the
    // declaration of the block it stands in, innermost-first; the other
    // block's declaration contributes NOTHING (the file-global join printed
    // a false absent row on each side).
    let worker = "public class OrdersWorker\n{\n    \
                  public void Handle(OrderPlaced order)\n    {\n        {\n            \
                  OrderPlaced evt = order;\n            var a = evt.OrderId;\n        }\n        \
                  {\n            PaymentReceived evt = null;\n            var b = evt.PaymentId;\n\
                  }\n    }\n}";
    let tree = two_event_tree(worker);
    let output = tree.run(&["verify", "."]);
    let report = format!("{}{}", stdout(&output), stderr(&output));
    assert_eq!(
        output.status.code(),
        Some(0),
        "each block reads its own declaration — green:\n{report}"
    );
    assert!(
        !report.contains("async drift"),
        "a sibling block's same-named declaration never joins a chain of the other block:\n\
         {report}"
    );
}
