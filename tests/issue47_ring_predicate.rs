//! GitHub issue #47 (C# audit, P5): the async ring census attributes each
//! declared entry by the DECLARING UNIT, not by the loose subscriber-
//! compiled-against project relation. The predicate (adopted fix shape,
//! adversarial pass B1): a subscriber→publisher direct back edge is NOT a
//! ring-closure witness when the event's declaring-unit census is
//! publisher-only — empty (no driver-side record census names the contract:
//! the declaration-trust law) or exactly {publisher} (the publisher declares
//! the contract it publishes). Any declarer OUTSIDE the publisher keeps the
//! edge's witness — ambiguity and independent direct justifications keep
//! `(mixed)` — and the multi-declarer collision names ONE row per declarer,
//! never the SCC representative's first-wins single row.
//!
//! Proven cells: (1) the issue's repro shape — a publisher-compiled-against
//! pair loses the false `(mixed)` row, and the two-async-legs + compile-edge
//! round trip prints `(async)` (the label flip, re-pinned beside the rust
//! cell in `tests/async_cycles.rs`); (2) subscriber-compiled-against-only
//! with NO declaration fact anywhere names no ring row; (3) multi-declarer
//! collision — one `(mixed)` row per declarer in deterministic order;
//! (5) a non-declaring tree (no `[[async_depends]]`) stays byte-identical;
//! (6) the pure-async rust round trip and the pure-async go round trip stay
//! byte-identical (the rust byte pins live in `tests/async_cycles.rs`; the
//! go ring census cell is here).

mod common;

use common::{Fixture, stderr, stdout};

fn ring_rows(report: &str) -> Vec<String> {
    report.lines().filter(|line| line.contains("async cycle:")).map(|l| l.to_string()).collect()
}

/// The C# pair of the issue's repro shape: unit `Publisher` DECLARES
/// `OrderPlaced` in its own namespace and publishes it; unit `Sub`
/// subscribes and references the publisher's project — the
/// subscriber-compiled-against-publisher pair whose compile leg closed a
/// false `(mixed)` ring on 0.6.94. With `with_pubtwo` the collision partner
/// joins: unit `PubTwo` declares a SECOND `OrderPlaced` copy, `Sub`
/// compiles against both publishers.
fn publisher_declares_tree(spec: &str, with_pubtwo: bool) -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Publisher/Publisher.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
         <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "Publisher/Events.cs",
        "namespace Publisher.Events;\npublic record OrderPlaced(decimal Total);\n",
    );
    fixture.write(
        "Publisher/Publisher.cs",
        "using Publisher.Events;\nnamespace Publisher;\npublic class OrderService { }\n",
    );
    let mut sub_csproj = String::from(
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
         <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n  \
         <ItemGroup>\n    <ProjectReference Include=\"..\\Publisher\\Publisher.csproj\" />\n",
    );
    if with_pubtwo {
        sub_csproj.push_str(
            "    <ProjectReference Include=\"..\\PubTwo\\PubTwo.csproj\" />\n",
        );
    }
    sub_csproj.push_str("  </ItemGroup>\n</Project>\n");
    fixture.write("Sub/Sub.csproj", &sub_csproj);
    fixture.write(
        "Sub/Handlers.cs",
        "using Publisher.Events;\nnamespace Sub;\npublic class OrderPlacedHandler\n{    \
         public void Handle(OrderPlaced e)\n    {\n        var t = e.Total;\n    }\n}\n",
    );
    if with_pubtwo {
        fixture.write(
            "PubTwo/PubTwo.csproj",
            "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
             <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
        );
        fixture.write(
            "PubTwo/Events.cs",
            "namespace PubTwo;\npublic record OrderPlaced(decimal Total);\n",
        );
        fixture.write(
            "PubTwo/PubTwo.cs",
            "namespace PubTwo;\npublic class StockService { }\n",
        );
        fixture.write(
            "Sub/PubTwoHandler.cs",
            "using PubTwo;\nnamespace Sub;\npublic class PubTwoOrderPlacedHandler\n{    \
             public void Handle(PubTwo.OrderPlaced e)\n    {\n        var t = e.Total;\n    }\n}\n",
        );
    }
    fixture.write("architecture.spec.toml", spec);
    fixture
}

/// The shared spec: two publisher-shaped claims over the units, the
/// subscriber allowed to compile against them, and the caller's
/// `[[async_depends]]` entries verbatim. The `pubtwo` claim (and the second
/// compile reference) joins only the collision cell.
fn pair_spec(entries: &str, with_pubtwo: bool) -> String {
    let allowed = if with_pubtwo { "[\"pub\", \"pubtwo\"]" } else { "[\"pub\"]" };
    let pubtwo = if with_pubtwo {
        "\n[[module]]\nname = \"pubtwo\"\nmatches = { units = [\"PubTwo\"] }\n"
    } else {
        ""
    };
    format!(
        "[project]\nlanguage = \"csharp\"\n\n[[module]]\nname = \"pub\"\n\
         matches = {{ units = [\"Publisher\"] }}\n\n[[module]]\nname = \"sub\"\n\
         matches = {{ units = [\"Sub\"] }}\n\n[module.allowed]\ndepend_on = {allowed}\n\
         {pubtwo}{entries}"
    )
}

/// Cell 1 (the issue's repro shape): one entry over a
/// publisher-compiled-against pair declares nothing outside the publisher,
/// so the compile leg is no ring-closure witness — the false `(mixed)` row
/// of 0.6.94 is GONE and the run is green.
#[test]
fn publisher_compiled_against_pair_names_no_false_mixed_ring() {
    let entry = "\n[[async_depends]]\nfrom = \"Publisher\"\nto = \"Sub\"\n\
                 event = \"Publisher.Events.OrderPlaced\"\n";
    let tree = publisher_declares_tree(&pair_spec(entry, false), false);
    let output = tree.run(&["verify", "."]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "the pair verifies green:\n{out}\n--- stderr ---\n{err}");
    assert!(
        !out.contains("async cycle:"),
        "the contract-compile back edge closes no ring — no `(mixed)` row:\n{out}"
    );
}

/// Cell 1 + 4 (the label flip, csharp arm): the two-async-legs round trip
/// with the subscriber compiled against the publisher — both events declared
/// by their publishers — is a RING the event legs close, and it now prints
/// `(async)`: the `(mixed)` class disappears for the
/// publisher-compiled-against pair while the ring keeps its honest voice.
#[test]
fn publisher_declared_round_trip_with_compile_edge_labels_async() {
    let entries = "\n[[async_depends]]\nfrom = \"Publisher\"\nto = \"Sub\"\n\
                   event = \"Publisher.Events.OrderPlaced\"\n\n\
                   [[async_depends]]\nfrom = \"Sub\"\nto = \"Publisher\"\n\
                   event = \"Sub.StockReserved\"\n";
    let tree = publisher_declares_tree(&pair_spec(entries, false), false);
    // The subscriber publishes its OWN declared contract back.
    tree.write(
        "Sub/Stock.cs",
        "namespace Sub;\npublic record StockReserved(int Quantity);\n",
    );
    tree.write(
        "Sub/Publisher.cs",
        "namespace Sub;\npublic class StockPublisher\n{    \
         public void Publish(StockReserved e)\n    {\n        var q = e.Quantity;\n    }\n}\n",
    );
    tree.write(
        "Publisher/StockHandler.cs",
        "namespace Publisher;\npublic class StockReservedHandler\n{    \
         public void Handle(Sub.StockReserved e)\n    {\n        var q = e.Quantity;\n    }\n}\n",
    );
    let output = tree.run(&["verify", "."]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "the async-only ring is green:\n{out}\n--- stderr ---\n{err}");
    let rings = ring_rows(&out);
    assert_eq!(rings.len(), 1, "exactly one ring row for the round trip:\n{out}");
    assert!(
        rings[0].contains("async cycle: Publisher -> Sub -> Publisher (async)"),
        "the ring is keyed on the declaring units and labels (async):\n{out}\n--- stderr ---\n{err}"
    );
    assert!(!out.contains("(mixed)"), "the compile-against leg names no mixed class:\n{out}");
}

/// Cell 2 (declaration trust, the `not-emitted` arm): the rust driver states
/// NO record-census declaration fact at all, so the subscriber-compiled-
/// against back edge names nothing and closes no ring — a pair whose ONLY
/// ring witness was the compile leg prints no ring row.
#[test]
fn compiled_against_pair_without_declaration_fact_names_no_ring() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"api\", \"core\"]\nresolver = \"2\"\n");
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "pub struct OrderPlaced;\n\npub fn publish() -> OrderPlaced { OrderPlaced }\n",
    );
    fixture.write(
        "core/Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
         [dependencies]\napi = { path = \"../api\" }\n",
    );
    fixture.write(
        "core/src/lib.rs",
        "use api::OrderPlaced;\n\npub fn handle(event: OrderPlaced) {\n    let _ = event;\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"api\"\n\
         matches = { units = [\"api\"] }\n\n[[module]]\nname = \"core\"\n\
         matches = { units = [\"core\"] }\n\n[module.allowed]\ndepend_on = [\"api\"]\n\n\
         [[async_depends]]\nfrom = \"api\"\nto = \"core\"\nevent = \"api::OrderPlaced\"\n",
    );
    let output = fixture.run(&["verify"]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "the census rows stay advisory:\n{out}\n--- stderr ---\n{err}");
    assert!(
        !out.contains("async cycle:"),
        "no declaration fact anywhere — the declaration-trust law closes no ring:\n{out}"
    );
}

/// Cell 3 (ALL declarers, never first-wins): `OrderPlaced` is declared by
/// TWO units (the duplicated-contract idiom), each publishing it to the
/// subscriber, which compiles against both. Ambiguity keeps its voice: the
/// compile legs stay witnesses and the census prints ONE `(mixed)` row per
/// declarer, in deterministic order — no arbitrary single pick.
#[test]
fn multi_declarer_collision_names_one_row_per_declarer() {
    let entries = "\n[[async_depends]]\nfrom = \"Publisher\"\nto = \"Sub\"\n\
                   event = \"Publisher.Events.OrderPlaced\"\n\n\
                   [[async_depends]]\nfrom = \"PubTwo\"\nto = \"Sub\"\n\
                   event = \"PubTwo.OrderPlaced\"\n";
    let tree = publisher_declares_tree(&pair_spec(entries, true), true);
    let output = tree.run(&["verify", "."]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "the collision rings are green:\n{out}\n--- stderr ---\n{err}");
    let rings = ring_rows(&out);
    assert_eq!(rings.len(), 2, "one row per declarer — never first-wins:\n{out}");
    assert!(
        rings[0].contains("PubTwo -> Sub -> PubTwo (mixed)"),
        "the first declarer's ring keeps the mixed voice:\n{rings:?}"
    );
    assert!(
        rings[1].contains("Publisher -> Sub -> Publisher (mixed)"),
        "the second declarer's ring is named too:\n{rings:?}"
    );
    // Deterministic order: rotating each pair to its smallest node and
    // sorting the rows puts `PubTwo` before `Publisher` (`T` < `l`).
    assert!(
        out.find("async cycle: PubTwo") < out.find("async cycle: Publisher"),
        "rows ride the deterministic sorted order:\n{out}"
    );
}

/// Cell 5 (D8, the csharp arm): a tree with NO `[[async_depends]]` section
/// reaches NONE of the census code — no ring row, no label, green.
#[test]
fn csharp_tree_without_async_table_stays_byte_silent() {
    let tree = publisher_declares_tree(&pair_spec("", false), false);
    let output = tree.run(&["verify", "."]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "the non-declaring tree stays green:\n{out}\n--- stderr ---\n{err}");
    assert!(
        !out.contains("async cycle:") && !out.contains("(mixed)") && !out.contains("(async)"),
        "the ring census is inert without a table:\n{out}"
    );
}

/// Cell 6 (byte-identity, the go arm): the pure pub→sub→pub round trip with
/// no manifest dependency between the units — every ring leg is a declared
/// event hop — prints `(async)` byte-identically to the pre-issue binary.
#[test]
fn go_round_trip_stays_an_async_ring() {
    let fixture = Fixture::new();
    fixture.write("go.work", "go 1.21\n\nuse (\n\t./api\n\t./store\n)\n");
    fixture.write("api/go.mod", "module example.com/api\ngo 1.21\n");
    fixture.write(
        "api/api.go",
        "package api\n\ntype OrderPlaced struct{ Total int }\n\nfunc Publish() OrderPlaced \
         {\n\treturn OrderPlaced{}\n}\n",
    );
    fixture.write("store/go.mod", "module example.com/store\ngo 1.21\n");
    fixture.write(
        "store/store.go",
        "package store\n\ntype StockReserved struct{ Quantity int }\n\nfunc Publish() \
         StockReserved {\n\treturn StockReserved{}\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[module]]\nname = \"api\"\n\
         matches = { units = [\"example.com/api\"] }\n\n[[module]]\nname = \"store\"\n\
         matches = { units = [\"example.com/store\"] }\n\n[[async_depends]]\n\
         from = \"example.com/api\"\nto = \"example.com/store\"\n\
         event = \"example.com/api.OrderPlaced\"\n\n[[async_depends]]\n\
         from = \"example.com/store\"\nto = \"example.com/api\"\n\
         event = \"example.com/store.StockReserved\"\n",
    );
    let output = fixture.run(&["verify"]);
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "the go round trip stays green:\n{out}\n--- stderr ---\n{err}");
    let rings = ring_rows(&out);
    assert_eq!(rings.len(), 1, "one advisory row for the round trip:\n{out}");
    assert!(
        rings[0].contains("async cycle: example.com/api -> example.com/store -> example.com/api (async)"),
        "the pure-async go ring labels (async) unchanged:\n{rings:?}"
    );
}
