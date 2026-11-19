//! `[[async_depends]]` spec-table cells (workplan archspec_async_depends,
//! scenario 01): the event-identity table parses; every malformed entry
//! shape prints its own named refusal; the three validation arms name their
//! failing side with a refusal string of its own; an event type declared in
//! a THIRD unit — a shared contracts project — refuses nothing (M4); the
//! not-emitted driver prints the capability-named inert note; the
//! forward-compat law keeps an older binary's unknown-key refusal loud over
//! the top-level table name (M9); an absent table keeps the shipped bytes
//! (D8). The exact-duplicate law (GitHub issue #56): a verbatim re-declaration
//! of an entry is refused naming both entries, a pair over distinct events
//! stays accepted, and explain's row count equals the table's entry count.

mod common;

use common::{Fixture, stderr, stdout};

/// The three-project C# tree: `App.Api` and `App.Worker` each reference and
/// `using` the shared `App.Contracts` project, which declares the public
/// record `OrderPlaced` in the `App.Contracts` namespace. `api_using` /
/// `worker_using` drop the respective code-level `using` (the derived
/// participation fact the arms read) while the csproj reference and the
/// declared `depend_on` stay — no other class can join the verdict.
fn csharp_tree(event: &str, api_using: bool, worker_using: bool, spec: &str) -> Fixture {
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
        &format!(
            "{}namespace App.Api;\npublic class OrdersController {{ }}\n",
            if api_using { "using App.Contracts;\n" } else { "" }
        ),
    );
    fixture.write("App.Worker/App.Worker.csproj", referenced_csproj);
    fixture.write(
        "App.Worker/OrdersWorker.cs",
        &format!(
            "{}namespace App.Worker;\npublic class OrdersWorker {{ }}\n",
            if worker_using { "using App.Contracts;\n" } else { "" }
        ),
    );
    fixture.write(
        "App.Contracts/App.Contracts.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
         <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "App.Contracts/Events.cs",
        "namespace App.Contracts;\npublic record OrderPlaced;\n",
    );
    let _ = event; // the event rides the spec, never the tree
    fixture.write("architecture.spec.toml", spec);
    fixture
}

/// The green spec: three unit claims, `depend_on` covering the two csproj
/// references, and one async edge App.Api -> App.Worker over the event
/// declared in the THIRD unit (the shared contracts project, M4).
fn shared_contracts_spec(event: &str) -> String {
    format!(
        "[project]\nlanguage = \"csharp\"\n\n{}[[async_depends]]\nfrom = \"App.Api\"\nto = \"App.Worker\"\nevent = \"{event}\"\n",
        module_claims()
    )
}

/// Unit-territory claims that also OWN the units' namespace modules: the
/// `using` facts derive module edges whose endpoints are those modules, and
/// an unclaimed endpoint prints the unowned-endpoint warning class
/// (`tests/csharp_modules.rs` scenario 24) — the claims keep the verdict
/// bytes clean.
fn module_claims() -> String {
    "[[module]]\nname = \"api\"\nmatches = { units = [\"App.Api\"], modules = [\"App::Api\"] }\n\n[module.allowed]\ndepend_on = [\"contracts\"]\n\n[[module]]\nname = \"worker\"\nmatches = { units = [\"App.Worker\"], modules = [\"App::Worker\"] }\n\n[module.allowed]\ndepend_on = [\"contracts\"]\n\n[[module]]\nname = \"contracts\"\nmatches = { units = [\"App.Contracts\"], modules = [\"App::Contracts\"] }\n".to_string()
}

// ---------------------------------------------------------------------------
// parse and refusal shapes
// ---------------------------------------------------------------------------

#[test]
fn valid_table_parses_and_validates_green_on_the_shared_contracts_tree() {
    let tree = csharp_tree("App.Contracts.OrderPlaced", true, true, &shared_contracts_spec("App.Contracts.OrderPlaced"));
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "verify must pass:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(stdout(&output).starts_with("ok: "), "{}", stdout(&output));
    // The shared-contracts identity refuses nothing — no async line at all.
    let report = stdout(&output);
    assert!(
        !report.contains("async event unresolved")
            && !report.contains("async publish not derived")
            && !report.contains("async subscribe not derived"),
        "the third unit's declared type refuses nothing: {report}"
    );
}

fn assert_invalid_spec(tree: &Fixture, expected: &str) {
    let output = tree.run(&["verify", "."]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "verify must refuse:\n{}",
        stdout(&output)
    );
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:") && err.contains(expected),
        "refusal must name {expected:?} in the invalid spec class:\n{err}"
    );
}

#[test]
fn entry_missing_a_mandatory_key_is_refused_naming_the_key_and_entry() {
    for (entry, key) in [
        ("to = \"App.Worker\"\nevent = \"App.Contracts.OrderPlaced\"", "from"),
        ("from = \"App.Api\"\nevent = \"App.Contracts.OrderPlaced\"", "to"),
        ("from = \"App.Api\"\nto = \"App.Worker\"", "event"),
    ] {
        let spec = format!(
            "[project]\nlanguage = \"csharp\"\n\n{}[[async_depends]]\n{entry}\n",
            module_claims()
        );
        let tree = csharp_tree("App.Contracts.OrderPlaced", true, true, &spec);
        assert_invalid_spec(
            &tree,
            &format!("[async_depends] entry #1: missing mandatory key \"{key}\""),
        );
    }
}

#[test]
fn entry_with_an_empty_mandatory_key_is_refused_in_the_same_class() {
    let spec = "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"api\"\nmatches = { units = [\"App.Api\"] }\n\n[module.allowed]\ndepend_on = [\"contracts\"]\n\n[[module]]\nname = \"worker\"\nmatches = { units = [\"App.Worker\"] }\n\n[module.allowed]\ndepend_on = [\"contracts\"]\n\n[[module]]\nname = \"contracts\"\nmatches = { units = [\"App.Contracts\"] }\n\n[[async_depends]]\nfrom = \"\"\nto = \"App.Worker\"\nevent = \"App.Contracts.OrderPlaced\"\n";
    let tree = csharp_tree("App.Contracts.OrderPlaced", true, true, spec);
    assert_invalid_spec(
        &tree,
        "[async_depends] entry #1: missing mandatory key \"from\"",
    );
}

#[test]
fn fourth_key_is_refused_by_the_shared_unknown_key_law() {
    let spec = shared_contracts_spec("App.Contracts.OrderPlaced")
        + "payload = \"int\"\n";
    let tree = csharp_tree("App.Contracts.OrderPlaced", true, true, &spec);
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0), "{}", stdout(&output));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:")
            && err.contains("unknown key(s) \"payload\"")
            && err.contains("in [async_depends] #1")
            && err.contains("supported keys are \"event\", \"from\", \"to\""),
        "the unknown-key refusal must name the key, the entry and the shape:\n{err}"
    );
}

// ---------------------------------------------------------------------------
// the exact-duplicate law (GitHub issue #56)
// ---------------------------------------------------------------------------

/// The #56 class, pinned: a byte-identical `[[async_depends]]` duplicate
/// parses green, renders ONE arrow (the renderers deduplicate rows) while
/// the entry-level counting lists the same event twice as entry #N and
/// entry #M — a silent census inflation. The parse now refuses the
/// verbatim re-declaration by name, naming both entries; entries differing
/// on ANY field (the legal multi-event collision class) stay accepted.
#[test]
fn exact_duplicate_entry_is_refused_naming_both_entry_numbers() {
    // Adjacent duplicate: entry #2 repeats entry #1 verbatim.
    let adjacent = shared_contracts_spec("App.Contracts.OrderPlaced")
        + "[[async_depends]]\nfrom = \"App.Api\"\nto = \"App.Worker\"\nevent = \"App.Contracts.OrderPlaced\"\n";
    let tree = csharp_tree("App.Contracts.OrderPlaced", true, true, &adjacent);
    assert_invalid_spec(&tree, "duplicate [[async_depends]] entry #2 duplicates #1");

    // Non-adjacent duplicate: entry #3 repeats entry #1 across a distinct #2.
    let non_adjacent = shared_contracts_spec("App.Contracts.OrderPlaced")
        + "[[async_depends]]\nfrom = \"App.Worker\"\nto = \"App.Api\"\nevent = \"App.Contracts.OrderPlaced\"\n"
        + "[[async_depends]]\nfrom = \"App.Api\"\nto = \"App.Worker\"\nevent = \"App.Contracts.OrderPlaced\"\n";
    let tree = csharp_tree("App.Contracts.OrderPlaced", true, true, &non_adjacent);
    assert_invalid_spec(&tree, "duplicate [[async_depends]] entry #3 duplicates #1");

    // A key-reordered copy is equal on ALL fields — the same accident speaks
    // the same refusal (the equality is field-by-field, not byte-position).
    let reordered = shared_contracts_spec("App.Contracts.OrderPlaced")
        + "[[async_depends]]\nevent = \"App.Contracts.OrderPlaced\"\nto = \"App.Worker\"\nfrom = \"App.Api\"\n";
    let tree = csharp_tree("App.Contracts.OrderPlaced", true, true, &reordered);
    assert_invalid_spec(&tree, "duplicate [[async_depends]] entry #2 duplicates #1");
}

#[test]
fn near_duplicate_pair_over_distinct_events_stays_accepted() {
    // The legal collision class: the same (from, to) pair declaring TWO
    // distinct events is not a duplicate — the parse accepts it and every
    // arm adjudicates both entries green.
    let spec = shared_contracts_spec("App.Contracts.OrderPlaced")
        + "[[async_depends]]\nfrom = \"App.Api\"\nto = \"App.Worker\"\nevent = \"App.Contracts.OrderShipped\"\n";
    let tree = csharp_tree("App.Contracts.OrderPlaced", true, true, &spec);
    tree.write(
        "App.Contracts/EventsExtra.cs",
        "namespace App.Contracts;\npublic record OrderShipped;\n",
    );
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the pair over two distinct events is accepted, never refused:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let report = stdout(&output);
    assert!(report.starts_with("ok: "), "{report}");
    assert!(
        !report.contains("duplicate"),
        "the duplicate class never fires on distinct events:\n{report}"
    );
}

#[test]
fn explain_lists_one_row_per_declared_entry_on_a_duplicate_free_spec() {
    // The #56 regression pin: entry accounting cannot double-count — the
    // explain async arm prints exactly one row per declared entry, numbered
    // once each, on a spec the duplicate law accepts.
    let spec = "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[[async_depends]]\nfrom = \"demo\"\nto = \"demo\"\nevent = \"demo::OrderPlaced\"\n\n[[async_depends]]\nfrom = \"demo\"\nto = \"demo\"\nevent = \"demo::OrderShipped\"\n";
    let tree = rust_tree(spec, true);
    tree.write("src/lib.rs", "pub struct OrderPlaced;\npub struct OrderShipped;\n");
    let output = tree.run(&["explain", "demo", "demo"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "explain never moves an exit code:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("async edges ([[async_depends]]):"),
        "the async arm must print:\n{out}"
    );
    let rows: Vec<&str> = out
        .lines()
        .filter(|line| line.contains("(entry #"))
        .collect();
    assert_eq!(
        rows.len(),
        2,
        "one explain row per declared entry — entry count and row count agree:\n{out}"
    );
    assert!(
        rows.iter().any(|row| row.contains("(entry #1)"))
            && rows.iter().any(|row| row.contains("(entry #2)")),
        "each entry is numbered exactly once:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// the three named arms
// ---------------------------------------------------------------------------

#[test]
fn arm_a_unresolved_event_names_itself_and_only_itself() {
    let tree = csharp_tree("App.Contracts.OrderMissing", true, true, &shared_contracts_spec("App.Contracts.OrderMissing"));
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    let report = stdout(&output);
    assert!(
        report.contains(
            "async event unresolved: App.Contracts.OrderMissing — App.Api -> App.Worker (entry #1)"
        ),
        "arm (a) must name the event and the entry:\n{report}"
    );
    assert!(
        !report.contains("async publish not derived")
            && !report.contains("async subscribe not derived"),
        "participation holds through the namespace usings — the derivation arms stay silent:\n{report}"
    );
}

#[test]
fn arm_b_missing_publish_participation_names_itself_and_only_itself() {
    let tree = csharp_tree("App.Contracts.OrderPlaced", false, true, &shared_contracts_spec("App.Contracts.OrderPlaced"));
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    let report = stdout(&output);
    assert!(
        report.contains(
            "async publish not derived: App.Api -> App.Worker — event App.Contracts.OrderPlaced (entry #1)"
        ),
        "arm (b) must name the from-unit and the event:\n{report}"
    );
    assert!(
        !report.contains("async event unresolved")
            && !report.contains("async subscribe not derived"),
        "the event resolves and the to-unit participates — only arm (b) fires:\n{report}"
    );
}

#[test]
fn arm_c_missing_subscribe_participation_names_itself_and_only_itself() {
    let tree = csharp_tree("App.Contracts.OrderPlaced", true, false, &shared_contracts_spec("App.Contracts.OrderPlaced"));
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    let report = stdout(&output);
    assert!(
        report.contains(
            "async subscribe not derived: App.Api -> App.Worker — event App.Contracts.OrderPlaced (entry #1)"
        ),
        "arm (c) must name the to-unit and the event:\n{report}"
    );
    assert!(
        !report.contains("async event unresolved")
            && !report.contains("async publish not derived"),
        "the event resolves and the from-unit participates — only arm (c) fires:\n{report}"
    );
}

// ---------------------------------------------------------------------------
// arm (c) — simple-name resolution for duplicated event contracts
// (workplan archspec_async_depends, scenario 06; consumer verdict on 0.6.86,
// issue #44: all 18 real subscriptions refused because the to-unit side
// keyed the declared FQN exactly while the census pairs by simple name)
// ---------------------------------------------------------------------------

/// The duplicated-contracts C# tree: `App.Api` and `App.Worker` are standalone
/// projects — no shared contracts project, no csproj reference, no cross-unit
/// `using` — each declaring its OWN namespaced copy of the event record. The
/// publisher owns its copy in `App.Api.Events`; the worker's copy lives under
/// `App.Worker.Events` and its handler implements
/// `IIntegrationEventHandler<OrderPlaced>` with a typed `Handle` parameter —
/// every subscription is real and NO literal `Subscribe` call exists anywhere.
/// `worker_copy` and `worker_handler` toggle the to-unit's two simple-name
/// facts (its own declared-type census, its derived bound shapes); the spec's
/// declared event FQN always names the PUBLISHER's namespace.
fn duplicated_contracts_tree(worker_copy: bool, worker_handler: bool, spec: &str) -> Fixture {
    let fixture = Fixture::new();
    let plain_csproj = concat!(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n",
        "    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write("App.Api/App.Api.csproj", plain_csproj);
    fixture.write(
        "App.Api/Events.cs",
        "namespace App.Api.Events;\npublic record OrderPlaced(Guid OrderId, decimal Total);\n",
    );
    fixture.write(
        "App.Api/OrdersController.cs",
        "using App.Api.Events;\nnamespace App.Api;\npublic class OrdersController\n{\n    \
         public void Publish(OrderPlaced placed)\n    {\n        _ = placed.OrderId;\n    }\n}\n",
    );
    fixture.write("App.Worker/App.Worker.csproj", plain_csproj);
    if worker_copy {
        fixture.write(
            "App.Worker/Events.cs",
            "namespace App.Worker.Events;\npublic record OrderPlaced(Guid OrderId, decimal Total);\n",
        );
    }
    fixture.write(
        "App.Worker/OrderPlacedHandler.cs",
        if worker_handler {
            "using App.Worker.Events;\nnamespace App.Worker;\npublic class OrderPlacedHandler : IIntegrationEventHandler<OrderPlaced>\n{\n    \
             public Task Handle(OrderPlaced integrationEvent, CancellationToken cancellationToken)\n    \
             {\n        var id = integrationEvent.OrderId;\n        return Task.CompletedTask;\n    \
             }\n}\n"
        } else {
            "namespace App.Worker;\npublic class OrdersWorker { }\n"
        },
    );
    fixture.write("architecture.spec.toml", spec);
    fixture
}

/// The claims own both each unit's modules; `worker_modules` lists only the
/// modules the fixture actually declared (the absent variant holds no
/// `App::Worker::Events`).
fn duplicated_spec(worker_modules: &str) -> String {
    format!(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"api\"\nmatches = {{ units = [\"App.Api\"], modules = [\"App::Api\", \"App::Api::Events\"] }}\n\n[[module]]\nname = \"worker\"\nmatches = {{ units = [\"App.Worker\"], modules = {worker_modules} }}\n\n[[async_depends]]\nfrom = \"App.Api\"\nto = \"App.Worker\"\nevent = \"App.Api.Events.OrderPlaced\"\n"
    )
}

/// The simple-name rule of the fixed arm lives in TWO spelling sites: the
/// drift/identity census in `verify::compare::async_depends` (`event_resolves`)
/// and the to-unit predicate in `scan::csharp`
/// (`derives_subscribe_participation`). This cell pins the rule's agreement:
/// the arm accepts exactly what the census pairs on.
#[test]
fn arm_c_resolves_per_unit_copies_by_the_simple_name_rule_spelled_in_async_depends_and_csharp_scan() {
    let tree = duplicated_contracts_tree(
        true,
        true,
        &duplicated_spec("[\"App::Worker\", \"App::Worker::Events\"]"),
    );
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the duplicated-contract subscriptions are real — the run is green:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    assert!(stdout(&output).starts_with("ok: "), "{}", stdout(&output));
    let report = stdout(&output);
    assert!(
        !report.contains("async subscribe not derived"),
        "the to-unit's own namespaced copy + interface-impl handler participate \
         by simple name — zero refusals:\n{report}"
    );
    assert!(
        !report.contains("async publish not derived")
            && !report.contains("async event unresolved")
            && !report.contains("async drift"),
        "arm (b) keeps its exact-FQN green, arm (a) resolves the census name, \
         and the shapes match — no other async line:\n{report}"
    );
}

#[test]
fn arm_c_absent_to_unit_refuses_by_name_the_async_depends_and_csharp_scan_rule_never_goes_silent() {
    // The to-unit declares no type of that simple name and carries no bound
    // shape: the fix must still refuse it BY NAME, exactly once.
    let tree = duplicated_contracts_tree(false, false, &duplicated_spec("[\"App::Worker\"]"));
    let output = tree.run(&["verify", "."]);
    assert_ne!(
        output.status.code(),
        Some(0),
        "a to-unit holding nothing for the event is refused, never silenced:\n{}",
        stdout(&output)
    );
    let report = stdout(&output);
    let refusals: Vec<&str> = report
        .lines()
        .filter(|line| line.contains("async subscribe not derived"))
        .collect();
    assert_eq!(
        refusals.len(),
        1,
        "the shipped refusal names the entry exactly once:\n{report}"
    );
    assert!(
        refusals[0].contains("App.Api -> App.Worker")
            && refusals[0].contains("App.Api.Events.OrderPlaced")
            && refusals[0].contains("(entry #1)"),
        "the refusal names both endpoints, the event identity and the entry: {}",
        refusals[0]
    );
    assert!(
        !report.contains("async publish not derived")
            && !report.contains("async event unresolved"),
        "the from-unit participates (its exact FQN copy) and the census resolves \
         the simple name — only arm (c) fires:\n{report}"
    );
}

// ---------------------------------------------------------------------------
// capability-conditional inert notes
// ---------------------------------------------------------------------------

fn rust_tree(spec: &str, public_item: bool) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write(
        "src/lib.rs",
        if public_item {
            "pub struct OrderPlaced;\n"
        } else {
            "fn hidden() {}\n"
        },
    );
    tree.write("architecture.spec.toml", spec);
    tree
}

const RUST_TABLE_SPEC: &str = "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[[async_depends]]\nfrom = \"demo\"\nto = \"demo\"\nevent = \"demo::OrderPlaced\"\n";

#[test]
fn not_emitted_driver_skips_the_derivation_arms_with_the_capability_note() {
    // rust's event-declaration-derivation row reads not-emitted: the arms
    // skip with the capability-named note, arm (a) still runs over the
    // census (the pub type resolves the identity), the run stays green.
    let tree = rust_tree(RUST_TABLE_SPEC, true);
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the inert arms never gate:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("note: async_depends publish and subscribe arms inert for rust: event-declaration-derivation = not-emitted"),
        "the capability-named skip note must print:\n{out}"
    );
    assert!(
        !out.contains("async_depends event-type arm inert"),
        "the census exists, so the event-type arm ran and states no note:\n{out}"
    );
}

#[test]
fn census_silence_notes_the_event_type_arm_without_guessing() {
    // A tree whose model states no declared-type census facts keeps arm (a)
    // inert with its own note — next to the driver's capability note.
    let tree = rust_tree(RUST_TABLE_SPEC, false);
    let output = tree.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "inert arms never gate:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let out = stdout(&output);
    assert!(
        out.contains("note: async_depends event-type arm inert: the model states no declared-type census facts"),
        "the census-silence note must print:\n{out}"
    );
    assert!(
        out.contains("async_depends publish and subscribe arms inert for rust"),
        "the capability note rides along:\n{out}"
    );
}

#[test]
fn empty_table_keeps_the_run_byte_identical_and_prints_no_notes() {
    // D8: without the table nothing moves — no note channel at all.
    let tree = rust_tree(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n",
        true,
    );
    let output = tree.run(&["verify", "."]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    let out = stdout(&output);
    assert!(out.starts_with("ok: "), "{out}");
    assert!(!out.contains("async_depends"), "{out}");
}

// ---------------------------------------------------------------------------
// forward compatibility (M9)
// ---------------------------------------------------------------------------

#[test]
fn unknown_table_alongside_the_known_one_is_refused_naming_the_table() {
    // The M9 mechanism, pinned live: an archspec that predates the table
    // refuses a spec carrying it because a top-level table outside the
    // accepted set is refused BY NAME. The same arm, spelled against this
    // binary's vocabulary: the singular typo is unknown and must come back
    // loud, naming the table and the top level.
    let spec = "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"demo\"\nmatches = { units = [\"demo\"] }\n\n[[async_depend]]\nfrom = \"demo\"\nto = \"demo\"\nevent = \"demo::OrderPlaced\"\n";
    let tree = rust_tree(spec, true);
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0), "{}", stdout(&output));
    let err = stderr(&output);
    assert!(
        err.contains("invalid spec:")
            && err.contains("unknown key(s) \"async_depend\"")
            && err.contains("at the top level"),
        "the top-level refusal must name the table (the M9 forward-compat class):\n{err}"
    );
}
