//! The `[[exempt]]` ratchet + `--write-exemptions` (GitHub issue #27), bounded
//! by the recorded decision-check ruling (R1–R8):
//! identity-keyed in-spec entries (R1/R2), stale and expired entries speaking
//! through the warning channel (R3/R4), one honest census on every surface
//! (R5), the schema law pair (R6), verdict-only suppression (R7) and the
//! seed-only writer (R8).

mod common;

use common::{Fixture, stderr, stdout};

/// A crate whose two modules import and reference each other: the model
/// carries the module-tier ring `a -> b -> a (runtime)`, both tiers have
/// adjudicated rows (no vacuity silences the census), and the pair is
/// declared `depend_on` so no edge finding rides along.
fn cycle_fixture_with(exempt_entries: &str) -> Fixture {
    let fixture = Fixture::new();
    write_demo_crate(&fixture, &[("a", "b")]);
    fixture.write(
        "architecture.spec.toml",
        &spec_with(&base_spec(&[("a", "b")]), exempt_entries),
    );
    fixture
}

fn cycle_fixture() -> Fixture {
    cycle_fixture_with("")
}

/// The same tree with a SECOND disjoint ring `c -> d -> c`: two cycles, so a
/// partial ratchet has a violating finding to stand beside the exempt one.
fn two_cycle_fixture(exempt_entries: &str) -> Fixture {
    let fixture = Fixture::new();
    write_demo_crate(&fixture, &[("a", "b"), ("c", "d")]);
    fixture.write(
        "architecture.spec.toml",
        &spec_with(&base_spec(&[("a", "b"), ("c", "d")]), exempt_entries),
    );
    fixture
}

/// The demo crate with one file per name, each file importing and
/// referencing its partner: `a -> b -> a` becomes the module-tier ring.
fn write_demo_crate(fixture: &Fixture, pairs: &[(&str, &str)]) {
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    let mut names: Vec<&str> = pairs.iter().flat_map(|(a, b)| [*a, *b]).collect();
    names.sort_unstable();
    names.dedup();
    fixture.write(
        "src/lib.rs",
        &names
            .iter()
            .map(|name| format!("pub mod {name};\n"))
            .collect::<String>(),
    );
    for name in names {
        let partners: Vec<&str> = pairs
            .iter()
            .filter(|(a, b)| *a != *b && (*a == name || *b == name))
            .map(|(a, b)| if *a == name { *b } else { *a })
            .collect();
        let cap = |partner: &str| {
            let mut chars = partner.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        };
        let mut source = String::new();
        for partner in &partners {
            source.push_str(&format!("use crate::{partner}::{};\n", cap(partner)));
        }
        source.push_str(&format!("pub struct {};\n", cap(name)));
        for partner in &partners {
            source.push_str(&format!("fn f() {{ let _ = {}; }}\n", cap(partner)));
        }
        fixture.write(&format!("src/{name}.rs"), &source);
    }
}

/// A single-package crate whose two files import each other: the module tier
/// sees no ring (both files attribute to one module), the FILE tier reports
/// the ring — the M1 asymmetric shape a tier-blind exemption would fudge.
/// The helper module carries a declared one-way edge so the module tier
/// stays non-vacuous: the census line must ride, never be silenced by a
/// vacuity warning.
fn file_cycle_fixture(exempt_entries: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\npub mod h;\npub use a::a;\npub use b::b;\n");
    fixture.write(
        "src/a.rs",
        "use crate::b::b;\nuse crate::h::H;\npub fn a() { let _ = b(); let _ = H; }\n",
    );
    fixture.write("src/b.rs", "use crate::a::a;\npub fn b() { let _ = a(); }\n");
    fixture.write("src/h.rs", "pub struct H;\n");
    fixture.write("architecture.spec.toml", &spec_with(FILE_SHAPE, exempt_entries));
    fixture
}

/// The file-tier shape: the whole unit under module `demo`, a one-way edge
/// to module `helper` (kept declared so no edge finding rides along), plus
/// the error-severity `no_cycles` row.
const FILE_SHAPE: &str = "\
[project]
language = \"rust\"

[[module]]
name = \"demo\"
matches = { units = [\"demo\"] }

[module.allowed]
depend_on = [\"helper\"]

[[module]]
name = \"helper\"
matches = { modules = [\"demo::h\"] }

[[constraint]]
type = \"no_cycles\"
severity = \"error\"
";

/// One module per name (mapped to `demo::<name>`, mutual pairs declared
/// `depend_on` so the edge checks stay quiet), plus the error-severity
/// `no_cycles` row.
fn base_spec(pairs: &[(&str, &str)]) -> String {
    let mut out = String::from("[project]\nlanguage = \"rust\"\n\n");
    let mut names: Vec<&str> = pairs.iter().flat_map(|(a, b)| [*a, *b]).collect();
    names.sort_unstable();
    names.dedup();
    for name in names {
        let partners: Vec<&str> = pairs
            .iter()
            .filter(|(a, b)| *a != *b && (*a == name || *b == name))
            .map(|(a, b)| if *a == name { *b } else { *a })
            .collect();
        out.push_str(&format!(
            "[[module]]\nname = \"{name}\"\nmatches = {{ modules = [\"demo::{name}\"] }}\n\n[module.allowed]\ndepend_on = [{}]\n\n",
            partners
                .iter()
                .map(|partner| format!("\"{partner}\""))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    out.push_str("[[constraint]]\ntype = \"no_cycles\"\nseverity = \"error\"\n");
    out
}

fn spec_with(base: &str, exempt_entries: &str) -> String {
    let mut out = base.to_string();
    if !exempt_entries.is_empty() {
        out.push('\n');
        out.push_str(exempt_entries);
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn entry(cycle: &str, tier: &str, extras: &str) -> String {
    format!("[[exempt]]\ncycle = \"{cycle}\"\ntier = \"{tier}\"\n{extras}")
}

// ---------------------------------------------------------------------------
// R1 — identity-keyed suppression
// ---------------------------------------------------------------------------

/// The happy path: a matching entry suppresses the verdict, the finding is
/// gone from the report body, and the census still states the found ring as
/// exempt, never a silent zero (R5 on the finding surface).
#[test]
fn matching_entry_greens_the_run_and_the_census_keeps_the_ring_visible() {
    let fixture = cycle_fixture_with(&entry("a -> b -> a", "module", ""));
    let output = fixture.run(&["verify", "."]);
    assert_eq!(output.status.code(), Some(0), "green expected:\n{}", stdout(&output));
    let out = stdout(&output);
    assert!(out.starts_with("ok:"), "pass confirmation expected:\n{out}");
    assert!(
        out.contains("exempt: 1, violating: 0"),
        "the census must state the exemption (R5):\n{out}"
    );
    assert!(!out.contains("cycle detected"), "the suppressed finding must not print:\n{out}");
    // `--strict` keeps the healthy exemption green: nothing warns.
    let strict = fixture.run(&["verify", ".", "--strict"]);
    assert_eq!(strict.status.code(), Some(0), "a healthy exemption is strict-green:\n{}", stdout(&strict));
}

/// The identity-churn falsification (A3): an entry exempts ring A by identity;
/// a DIFFERENT ring stays red naming its own identity — the entry never
/// whitens debt it never named.
#[test]
fn entry_never_suppresses_a_ring_it_did_not_name() {
    let fixture = two_cycle_fixture(&entry("a -> b -> a", "module", ""));
    let output = fixture.run(&["verify", "."]);
    assert_eq!(output.status.code(), Some(1), "the un-named ring must stay red:\n{}", stdout(&output));
    let out = stdout(&output);
    assert!(out.contains("cycle: c -> d -> c (runtime)"), "the new ring must be named:\n{out}");
    assert!(!out.contains("cycle: a -> b -> a"), "the exempt ring must not print:\n{out}");
    assert!(out.contains("exempt: 1, violating: 1"), "the census splits the two rings (R5):\n{out}");
}

// ---------------------------------------------------------------------------
// R2 — the tier is named, tier mismatch never suppresses
// ---------------------------------------------------------------------------

/// A file-tier entry never suppresses a module-tier finding, and a module-tier
/// entry never suppresses a file-tier finding — the M1 asymmetry law.
#[test]
fn tier_mismatch_never_suppresses() {
    let wrong_tier_on_module_finding =
        cycle_fixture_with(&entry("a -> b -> a", "file", ""));
    let output = wrong_tier_on_module_finding.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a file-tier entry must not suppress a module ring:\n{}",
        stdout(&output)
    );

    let wrong_tier_on_file_finding =
        file_cycle_fixture(&entry("src/a.rs -> src/b.rs -> src/a.rs", "module", ""));
    let output = wrong_tier_on_file_finding.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "a module-tier entry must not suppress a file ring:\n{}",
        stdout(&output)
    );

    let right_tier_on_file_finding =
        file_cycle_fixture(&entry("src/a.rs -> src/b.rs -> src/a.rs", "file", ""));
    let output = right_tier_on_file_finding.run(&["verify", "."]);
    assert_eq!(output.status.code(), Some(0), "the matching file-tier entry greens:\n{}", stdout(&output));
    assert!(stdout(&output).contains("exempt: 1, violating: 0"), "census states the file exemption:\n{}", stdout(&output));
}

// ---------------------------------------------------------------------------
// R3 — stale entries warn
// ---------------------------------------------------------------------------

/// An entry matching zero current findings stays green but speaks through the
/// warning channel, naming tier and identity.
#[test]
fn stale_entry_warns_and_strict_promotes() {
    let fixture = one_unit_tree(&entry(
        "x -> y -> x",
        "module",
        "reason = \"fixed in #1234\"\n",
    ));
    let output = fixture.run(&["verify", "."]);
    assert_eq!(output.status.code(), Some(0), "a stale entry is warning-severity:\n{}", stdout(&output));
    assert!(
        stdout(&output).contains("warning: exempt stale: module x -> y -> x"),
        "the stale warning must name tier and identity:\n{}",
        stdout(&output)
    );
    let strict = fixture.run(&["verify", ".", "--strict"]);
    assert_eq!(strict.status.code(), Some(1), "--strict promotes the stale warning:\n{}", stdout(&strict));
    assert!(
        stdout(&strict).contains("  exempt stale: module x -> y -> x"),
        "the promoted line drops the warning prefix:\n{}",
        stdout(&strict)
    );
}

// ---------------------------------------------------------------------------
// R4 — expired entries warn and the debt returns
// ---------------------------------------------------------------------------

/// A past `until` exempts NOTHING — the violation reports red — and the entry
/// names itself as expired through the warning channel instead of silently
/// whitening or silently vanishing.
#[test]
fn expired_entry_warns_and_the_violation_returns() {
    let fixture = cycle_fixture_with(&entry("a -> b -> a", "module", "until = \"2000-01-01\"\n"));
    let output = fixture.run(&["verify", "."]);
    assert_eq!(output.status.code(), Some(1), "an expired entry must not exempt:\n{}", stdout(&output));
    let out = stdout(&output);
    assert!(out.contains("cycle: a -> b -> a (runtime)"), "the debt reports:\n{out}");
    assert!(out.contains("warning: exempt expired: module a -> b -> a"), "the entry names itself expired:\n{out}");
    assert!(!out.contains("exempt stale"), "an expired entry never doubles as stale:\n{out}");
}

/// A future `until` keeps the exemption.
#[test]
fn future_until_still_exempts() {
    let fixture = cycle_fixture_with(&entry("a -> b -> a", "module", "until = \"2999-12-31\"\n"));
    let output = fixture.run(&["verify", "."]);
    assert_eq!(output.status.code(), Some(0), "a live expiry exempts:\n{}", stdout(&output));
}

/// An expired entry whose debt is already fixed warns (retention notice),
/// green — the entry is a fact the run states, not a lie it swallows.
#[test]
fn expired_entry_on_a_clean_tree_warns_green() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub fn a() {}\n");
    fixture.write(
        "architecture.spec.toml",
        &spec_with(
            "\
[project]
language = \"rust\"

[[module]]
name = \"app\"
matches = { units = [\"app\"] }

[[constraint]]
type = \"no_cycles\"
severity = \"error\"
",
            &entry("gone -> away -> gone", "module", "until = \"2000-01-01\"\n"),
        ),
    );
    let output = fixture.run(&["verify", "."]);
    assert_eq!(output.status.code(), Some(0), "a past expiry on fixed debt is a warning:\n{}", stdout(&output));
    assert!(stdout(&output).contains("warning: exempt expired: module gone -> away -> gone"), "{}", stdout(&output));
}

// ---------------------------------------------------------------------------
// Advisory tier — exemption suppresses warning-severity cycles too
// ---------------------------------------------------------------------------

/// A spec with NO `no_cycles` row reports cycles as advisory warnings; an
/// entry still suppresses the verdict effect and the census stays honest.
#[test]
fn entry_suppresses_advisory_cycles_too() {
    let fixture = Fixture::new();
    write_demo_crate(&fixture, &[("a", "b")]);
    let base = "\
[project]
language = \"rust\"

[[module]]
name = \"a\"
matches = { modules = [\"demo::a\"] }

[module.allowed]
depend_on = [\"b\"]

[[module]]
name = \"b\"
matches = { modules = [\"demo::b\"] }

[module.allowed]
depend_on = [\"a\"]
";
    fixture.write("architecture.spec.toml", &spec_with(base, ""));
    let without = fixture.run(&["verify", "."]);
    assert_eq!(without.status.code(), Some(0));
    assert!(stdout(&without).contains("warning: cycle: a -> b -> a"), "advisory baseline:\n{}", stdout(&without));

    fixture.write("architecture.spec.toml", &spec_with(base, &entry("a -> b -> a", "module", "")));
    let with = fixture.run(&["verify", "."]);
    assert_eq!(with.status.code(), Some(0), "the entry suppresses the advisory finding:\n{}", stdout(&with));
    assert!(!stdout(&with).contains("cycle: a -> b -> a"), "the suppressed advisory never prints:\n{}", stdout(&with));
    assert!(stdout(&with).contains("exempt: 1, violating: 0"), "the census keeps it counted:\n{}", stdout(&with));
}

// ---------------------------------------------------------------------------
// R5 — census honesty on metrics and coverage
// ---------------------------------------------------------------------------

/// Metrics state found / exempt / violating separately: `cycles` counts every
/// adjudicated ring including the exempted ones; `exempt_cycles` rides only
/// when it is non-zero (existing JSON bytes stay stable).
#[test]
fn json_metrics_state_found_exempt_violating() {
    let fixture = two_cycle_fixture(&entry("a -> b -> a", "module", ""));
    let output = fixture.run(&["verify", ".", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_str(&stdout(&output)).expect("json verdict");
    assert_eq!(json["metrics"]["cycles"].as_u64(), Some(2), "found counts every ring");
    assert_eq!(json["metrics"]["exempt_cycles"].as_u64(), Some(1));
    let cycle_findings = json["findings"]
        .as_array()
        .expect("findings array")
        .iter()
        .filter(|f| f["category"] == "cycle detected in")
        .count();
    assert_eq!(cycle_findings, 1, "only the violating ring is a finding");

    let clean = cycle_fixture();
    let output = clean.run(&["verify", ".", "--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(
        !stdout(&output).contains("exempt_cycles"),
        "an unexempted run keeps today's json bytes:\n{}",
        stdout(&output)
    );
}

/// `--coverage` states the same split when exemptions are active.
#[test]
fn coverage_census_states_the_exempt_split() {
    let fixture = two_cycle_fixture(&entry("a -> b -> a", "module", ""));
    let output = fixture.run(&["verify", ".", "--coverage"]);
    assert_eq!(output.status.code(), Some(1));
    let out = stdout(&output);
    assert!(
        out.contains("coverage: cycles: 2, exempt: 1, violating: 1"),
        "the coverage census states found/exempt/violating (R5):\n{out}"
    );

    let plain = cycle_fixture();
    let output = plain.run(&["verify", ".", "--coverage"]);
    assert!(
        !stdout(&output).contains("coverage: cycles:"),
        "no exemptions — today's coverage bytes:\n{}",
        stdout(&output)
    );
}

// ---------------------------------------------------------------------------
// R8 — `--write-exemptions` is seed-only
// ---------------------------------------------------------------------------

/// The seeding run writes one entry per current violation, echoes the write,
/// and greens; a second seeding run writes nothing (never auto-grows).
#[test]
fn write_exemptions_seeds_then_stays_still() {
    let fixture = two_cycle_fixture("");
    let output = fixture.run(&["verify", ".", "--write-exemptions"]);
    assert_eq!(output.status.code(), Some(0), "the seeding run goes green:\n{}", stdout(&output));
    assert!(stdout(&output).contains("wrote "), "the write echoes (blind4 D01):\n{}", stdout(&output));
    let spec = fixture.read("architecture.spec.toml");
    assert!(spec.contains("cycle = \"a -> b -> a\"") && spec.contains("tier = \"module\""), "entry for ring A:\n{spec}");
    assert!(spec.contains("cycle = \"c -> d -> c\"") && spec.contains("tier = \"module\""), "entry for ring C:\n{spec}");

    let second = fixture.run(&["verify", ".", "--write-exemptions"]);
    assert_eq!(second.status.code(), Some(0));
    assert!(!stdout(&second).contains("wrote "), "idempotence: nothing new to seed:\n{}", stdout(&second));
    assert_eq!(fixture.read("architecture.spec.toml"), spec, "the second run never rewrites the ledger");

    let plain = fixture.run(&["verify", "."]);
    assert_eq!(plain.status.code(), Some(0), "the seeded ledger greens plain verify:\n{}", stdout(&plain));
    assert!(stdout(&plain).contains("exempt: 2, violating: 0"), "both rings stay visible:\n{}", stdout(&plain));
}

/// The writer never edits or removes an existing entry — the human-owned
/// fields (count, until, reason) survive byte-identical while a NEW violation
/// gets its own fresh entry appended.
#[test]
fn write_exemptions_never_touches_existing_entries() {
    let fixture = two_cycle_fixture(&entry(
        "a -> b -> a",
        "module",
        "count = 1\nuntil = \"2999-12-31\"\nreason = \"named debt\"\n",
    ));
    let before = fixture.read("architecture.spec.toml");
    let output = fixture.run(&["verify", ".", "--write-exemptions"]);
    assert_eq!(output.status.code(), Some(0), "the new ring seeds, the run greens:\n{}", stdout(&output));
    let after = fixture.read("architecture.spec.toml");
    assert!(after.contains("count = 1"), "the human cap survives:\n{after}");
    assert!(after.contains("until = \"2999-12-31\""), "the human expiry survives:\n{after}");
    assert!(after.contains("reason = \"named debt\""), "the human reason survives:\n{after}");
    assert!(after.contains("cycle = \"c -> d -> c\""), "the new violation is seeded:\n{after}");
    assert!(after.starts_with(before.trim_end()), "existing bytes are preserved verbatim, entries only append:\n{after}");
}

/// Stale entries are NOT silently pruned by the writer — they keep warning
/// (R3) until a human removes them.
#[test]
fn write_exemptions_keeps_stale_entries() {
    let fixture = cycle_fixture_with(&entry("gone -> away -> gone", "module", ""));
    let output = fixture.run(&["verify", ".", "--write-exemptions"]);
    assert_eq!(output.status.code(), Some(0), "the live ring seeds, the stale entry warns:\n{}", stdout(&output));
    assert!(stdout(&output).contains("warning: exempt stale: module gone -> away -> gone"), "the stale warning still rides:\n{}", stdout(&output));
    let spec = fixture.read("architecture.spec.toml");
    assert!(spec.contains("gone -> away -> gone"), "the stale entry is kept for the human:\n{spec}");
}

/// `--json` + `--write-exemptions` is a usage error: a machine consumer must
/// never trigger a write.
#[test]
fn write_exemptions_with_json_is_a_usage_error() {
    let fixture = cycle_fixture();
    let output = fixture.run(&["verify", ".", "--json", "--write-exemptions"]);
    assert_eq!(output.status.code(), Some(64), "the combination is refused at the usage class");
    assert!(
        stderr(&output).contains("--json and --write-exemptions cannot be combined"),
        "the refusal names the combination (frozen bytes):\n{}",
        stderr(&output)
    );
    assert!(!fixture.read("architecture.spec.toml").contains("[[exempt]]"), "no write happened");
}

// ---------------------------------------------------------------------------
// R6 — the schema law
// ---------------------------------------------------------------------------

#[test]
fn exempt_shape_refusals_name_the_key_and_location() {
    // Unknown key — the shipped class.
    let tree = one_unit_tree(&entry("a -> b -> a", "module", "typo = \"x\"\n"));
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("unknown key(s) \"typo\"") && stderr(&output).contains("[exempt]"),
        "unknown exempt keys ride the shipped class:\n{}",
        stderr(&output)
    );

    // Missing tier — R2 refuses the ambiguous entry at load.
    let tree = one_unit_tree("[[exempt]]\ncycle = \"a -> b -> a\"\n");
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("missing mandatory key \"tier\"") && stderr(&output).contains("[exempt]"),
        "the missing tier must be named:\n{}",
        stderr(&output)
    );

    // Missing cycle.
    let tree = one_unit_tree("[[exempt]]\ntier = \"module\"\n");
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("missing mandatory key \"cycle\""), "{}", stderr(&output));

    // Tier value domain.
    let tree = one_unit_tree(&entry("a -> b -> a", "pair", ""));
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    assert!(
        stderr(&output).contains("supported values are \"file\", \"module\" and \"both\""),
        "the tier domain is named:\n{}",
        stderr(&output)
    );

    // A non-ring cycle value.
    let tree = one_unit_tree(&entry("a -> b", "module", ""));
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("invalid spec:"), "{}", stderr(&output));

    // count = 0 is refused (cap semantics: at least one match).
    let tree = one_unit_tree(&entry("a -> b -> a", "module", "count = 0\n"));
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("count must be a positive integer"), "{}", stderr(&output));

    // A malformed expiry.
    let tree = one_unit_tree(&entry("a -> b -> a", "module", "until = \"next-tuesday\"\n"));
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("YYYY-MM-DD"), "{}", stderr(&output));

    // Duplicate (tier, cycle) — one identity one entry.
    let tree = one_unit_tree(&format!(
        "{}\n{}",
        entry("a -> b -> a", "module", ""),
        entry("a -> b -> a", "module", "")
    ));
    let output = tree.run(&["verify", "."]);
    assert_ne!(output.status.code(), Some(0));
    assert!(stderr(&output).contains("duplicate exempt entry"), "{}", stderr(&output));
}

fn one_unit_tree(exempt_entries: &str) -> Fixture {
    let tree = Fixture::new();
    tree.write(
        "Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    tree.write("src/lib.rs", "pub fn f() {}\n");
    tree.write(
        "architecture.spec.toml",
        &spec_with(
            "\
[project]
language = \"rust\"

[[module]]
name = \"demo\"
matches = { units = [\"demo\"] }
",
            exempt_entries,
        ),
    );
    tree
}

// ---------------------------------------------------------------------------
// The documentation surfaces (R6 fan-out)
// ---------------------------------------------------------------------------

#[test]
fn help_diagnostics_catalogs_the_new_categories() {
    let tree = one_unit_tree("");
    let output = tree.run(&["help", "diagnostics"]);
    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert!(out.contains("exempt stale"), "the stale category is cataloged:\n{}", first_lines(&out));
    assert!(out.contains("exempt expired"), "the expired category is cataloged:\n{}", first_lines(&out));
    assert!(out.contains("[[exempt]]"), "the entry table itself is explained:\n{}", first_lines(&out));
}

#[test]
fn spec_reference_and_schema_document_exempt() {
    let tree = one_unit_tree("");
    let reference = tree.run(&["spec"]);
    assert_eq!(reference.status.code(), Some(0));
    assert!(stdout(&reference).contains("[[exempt]]"), "the annotated reference carries the block");

    let schema = tree.run(&["spec", "--schema"]);
    assert_eq!(schema.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_str(&stdout(&schema)).expect("schema json");
    assert!(json["properties"]["exempt"].is_object(), "the schema declares the exempt array");
}

#[test]
fn verify_help_names_the_flag() {
    let tree = one_unit_tree("");
    let output = tree.run(&["verify", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("--write-exemptions"), "the flag is in the usage block");
}

// ---------------------------------------------------------------------------
// R7 — suppression never weakens adjudication: the pair stays counted
// ---------------------------------------------------------------------------

/// With every cycle exempt the report still counts the ring: the metrics state
/// `cycles: 1`, the census states exempt 1 — never the false zero an omitted
/// constraint would print.
#[test]
fn exemption_keeps_pair_counted_and_adjudicated() {
    let fixture = cycle_fixture_with(&entry("a -> b -> a", "module", ""));
    let output = fixture.run(&["report"]);
    assert_eq!(output.status.code(), Some(0), "report on an exempt tree:\n{}", stdout(&output));
    let out = stdout(&output);
    assert!(out.contains("exempt"), "the report states the counted ring:\n{out}");

    let json = fixture.run(&["report", "--format", "json"]);
    let json: serde_json::Value = serde_json::from_str(&stdout(&json)).expect("report json");
    assert_eq!(json["metrics"]["cycles"].as_u64(), Some(1), "found counts the exempt ring");
    assert_eq!(json["metrics"]["exempt_cycles"].as_u64(), Some(1));
    assert_eq!(json["metrics"]["violations_cycles"].as_u64(), Some(0), "violating is zero");
}

fn first_lines(text: &str) -> String {
    text.lines().take(20).collect::<Vec<_>>().join("\n")
}
