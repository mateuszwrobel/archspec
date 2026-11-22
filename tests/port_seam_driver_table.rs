//! Scenario 10 — the port seam: one driver table behind `extract_with`
//! (workplan `events_value_join` scenario 10, ADR-069 Decision 5's ports
//! rung). The done-when cells:
//!
//! - (A) `every_language_variant_resolves_to_its_own_driver` — for each of
//!   the six `Language` variants, `scan::extract_with` returns EXACTLY the
//!   direct `::<driver>::extract` result on the same fixture tree (model
//!   equal, candidates equal, error strings equal) — the single-dispatch-site
//!   law: the seam owns one `Language` match, `scan::driver`.
//! - (B) `the_trait_exposes_extraction_only` — the A3+A4 law, falsifiable by
//!   grep: `src/driver.rs` declares exactly one trait method with today's
//!   extract shape; the words `resolve`/`contribute`/`accept`/`pattern`
//!   appear in no trait surface; every driver impl block contributes exactly
//!   one method; `Box<dyn Driver`/`Vec<Box<dyn` appear nowhere in the crate;
//!   the seam files (`src/driver.rs`, `src/scan/mod.rs`) carry no `OnceLock`,
//!   no `Mutex`, no `register` — the table is compile-time.
//! - (F) the candidate-order pin: `Model::literal_candidates` is
//!   serde-skipped, so `scan.json` bytes never prove candidate order —
//!   `literal_candidate_order_is_pinned_per_driver` asserts the exact
//!   sequence (path → anchor → position order) each driver produces on a
//!   fixed two-file fixture through the library field itself, and
//!   `resolved_rows_are_invariant_under_candidate_push_order` asserts the
//!   resolved `literal_sites` rows and the census counts stay identical when
//!   the push order is permuted — `Model::sort_literal_sites` is the
//!   normalizer, named by the cell.
//!
//! The `ARCHSPEC_DISABLE_DRIVERS` refusal arm (done-when C) rides the
//! existing `extract_checked` cells, unchanged: the availability gate stays
//! OUTSIDE the table (`scan/mod.rs`), so those cells are the pin.
//!
//! Names are synthetic; no corpus, vendor or account string appears in this
//! file (F10).

mod common;

use common::Fixture;
use rust_arch_test_kit::language::Language;
use rust_arch_test_kit::model::{
    DeclaredFacts, DeclaredPattern, DeclaredPatternVerb, LiteralCandidate, LiteralCandidateAnchor,
    Model,
};
use rust_arch_test_kit::scan;

// ---------------------------------------------------------------------------
// The resolution-seam weld (the F2 cell reads the ONE engine-side site, not
// a restatement): `archspec::events::patterns` through `include!`, same
// shape as `events_value_join_rows.rs` welds `join.rs`. The shim module tree
// mirrors the binary's own paths (`crate::archspec::…` resolves to the crate
// root), the model vocabulary rides its single owner.
// ---------------------------------------------------------------------------
#[allow(dead_code, clippy::all)]
mod archspec {
    pub(crate) mod model {
        pub use rust_arch_test_kit::model::*;
    }
    pub(crate) mod events {
        pub(crate) mod unitid {
            include!("../src/archspec/events/unitid.rs");
        }
        pub(crate) mod patterns {
            include!("../src/archspec/events/patterns.rs");
        }
    }
}

/// One declared side naming `language` with the call verb on `emit` — the
/// candidate-feed gate for the five feeding drivers (the drivers gate on
/// language identity only: the feed is vocabulary-free, A6).
fn declared_call_side(language: &str) -> DeclaredFacts {
    DeclaredFacts {
        patterns: vec![DeclaredPattern {
            id: "seam-pin".to_string(),
            language: language.to_string(),
            role: Some("publish".to_string()),
            verb: DeclaredPatternVerb::CallNameArgPosition {
                call: "emit".to_string(),
                arg: 0,
            },
        }],
        ..DeclaredFacts::default()
    }
}

/// The kotlin feed arm is the exported-property one (scenario 06/09): the
/// side names a property, the gate names the language.
fn declared_property_side(language: &str, property: &str) -> DeclaredFacts {
    DeclaredFacts {
        patterns: vec![DeclaredPattern {
            id: "seam-pin".to_string(),
            language: language.to_string(),
            role: Some("publish".to_string()),
            verb: DeclaredPatternVerb::ExportedPropertyValue {
                property: property.to_string(),
            },
        }],
        ..DeclaredFacts::default()
    }
}

// ---------------------------------------------------------------------------
// One fixture per driver (the same tree serves cells A and F: the seam must
// return the direct call's result and carry that driver's candidate feed)
// ---------------------------------------------------------------------------

fn rust_tree(fixture: &Fixture) {
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"seam\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("src/lib.rs", "pub mod a;\npub mod b;\n");
    fixture.write(
        "src/a.rs",
        "pub fn one() {\n    emit(\"seam-a\");\n}\nfn emit(_e: &str) {}\n",
    );
    fixture.write(
        "src/b.rs",
        "pub fn two() {\n    emit(\"seam-b\");\n}\nfn emit(_e: &str) {}\n",
    );
}

fn csharp_tree(fixture: &Fixture) {
    fixture.write(
        "App/App.csproj",
        "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    <TargetFramework>net8.0</TargetFramework>\n  </PropertyGroup>\n</Project>\n",
    );
    fixture.write(
        "App/First.cs",
        "namespace App;\npublic class First { public void Go() { App.Emit(\"seam-a\"); } }\nstatic class Emitter { public static void Emit(string _e) {} }\n",
    );
    fixture.write(
        "App/Second.cs",
        "namespace App;\npublic class Second { public void Go() { App.Emit(\"seam-b\"); } }\n",
    );
}

fn go_tree(fixture: &Fixture) {
    fixture.write("go.mod", "module demo/shop\n\ngo 1.22\n");
    fixture.write(
        "a.go",
        "package main\n\nfunc a() {\n\temit(\"seam-a\")\n}\n\nfunc emit(_e string) {}\n",
    );
    fixture.write(
        "b.go",
        "package main\n\nfunc b() {\n\temit(\"seam-b\")\n}\n",
    );
}

fn kotlin_tree(fixture: &Fixture) {
    // No gradle settings: the documented `no_settings` arm answers one unit
    // at the scan root (K-P2), stated on the report-only face.
    fixture.write("a.kt", "const val SEAM_A = \"seam-a\"\n");
    fixture.write("b.kt", "const val SEAM_B = \"seam-b\"\n");
}

fn python_tree(fixture: &Fixture) {
    fixture.write("a.py", "def a():\n    emit(\"seam-a\")\n\n\ndef emit(_e):\n    pass\n");
    fixture.write("b.py", "def b():\n    emit(\"seam-b\")\n");
}

fn ts_tree(fixture: &Fixture) {
    fixture.write("package.json", "{\"name\": \"seam\", \"version\": \"0.0.0\"}\n");
    fixture.write(
        "a.ts",
        "export function a() {\n    emit(\"seam-a\");\n}\nfunction emit(_e: string) {}\n",
    );
    fixture.write(
        "b.ts",
        "export function b() {\n    emit(\"seam-b\");\n}\n",
    );
}

/// The declared side each feed gate reads for its driver.
fn declared_for(language: Language) -> DeclaredFacts {
    match language {
        Language::Kotlin => declared_property_side("kotlin", "SEAM_A"),
        other => declared_call_side(other.as_str()),
    }
}

/// The direct driver call — the seam's comparison arm for cell (A). The
/// module paths spell the drivers exactly as the table does.
fn direct_extract(
    language: Language,
    root: &std::path::Path,
    declared: &DeclaredFacts,
) -> Result<Model, String> {
    match language {
        Language::Rust => scan::rust::extract(root, declared),
        Language::Csharp => scan::csharp::extract(root, declared),
        Language::Go => scan::go::extract(root, declared),
        Language::Kotlin => scan::kotlin::extract(root, declared),
        Language::Python => scan::python::extract(root, declared),
        Language::Ts => scan::ts::extract(root, declared),
    }
}

fn fixture_for(language: Language) -> Fixture {
    let fixture = Fixture::new();
    match language {
        Language::Rust => rust_tree(&fixture),
        Language::Csharp => csharp_tree(&fixture),
        Language::Go => go_tree(&fixture),
        Language::Kotlin => kotlin_tree(&fixture),
        Language::Python => python_tree(&fixture),
        Language::Ts => ts_tree(&fixture),
    }
    fixture
}

// ---------------------------------------------------------------------------
// cell A — one dispatch site: the table equals the direct driver call
// ---------------------------------------------------------------------------

#[test]
fn every_language_variant_resolves_to_its_own_driver() {
    for language in [
        Language::Rust,
        Language::Csharp,
        Language::Go,
        Language::Kotlin,
        Language::Python,
        Language::Ts,
    ] {
        let fixture = fixture_for(language);
        let declared = declared_for(language);

        // Ok arm: the model compares member for member — `PartialEq` spans
        // every field, the serde-skipped `literal_candidates` included, so
        // the drivers' contribution paths are compared, not just the JSON.
        let via_seam = scan::extract_with(language, &fixture.root, &declared);
        let direct = direct_extract(language, &fixture.root, &declared);
        match (&via_seam, &direct) {
            (Ok(seam), Ok(straight)) => assert_eq!(
                seam, straight,
                "{language:?}: extract_with must equal the direct driver call on the same tree"
            ),
            (Err(seam), Err(straight)) => assert_eq!(
                seam, straight,
                "{language:?}: the seam must carry the driver's error string verbatim"
            ),
            _ => panic!(
                "{language:?}: the seam and the direct driver call disagree — seam={via_seam:?} direct={direct:?}"
            ),
        }

        // Err arm (the same call site, a tree that is not there): the exact
        // driver error string rides the seam, byte for byte.
        let absent = fixture.path("no-such-tree");
        let seam_absent = scan::extract_with(language, &absent, &DeclaredFacts::default());
        let direct_absent = direct_extract(language, &absent, &DeclaredFacts::default());
        match (&seam_absent, &direct_absent) {
            (Err(seam), Err(straight)) => assert_eq!(
                seam, straight,
                "{language:?}: the seam and the direct call state one error string on an absent root"
            ),
            (Ok(_), Ok(_)) => {}
            _ => panic!("{language:?}: absent-root results disagree"),
        }
    }
}

// ---------------------------------------------------------------------------
// cell B — the trait exposes extraction only; no registration machinery
// ---------------------------------------------------------------------------

fn read_source(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// The code lines only: comment and doc lines dropped. The forbidden shapes
/// are MACHINERY — a doc comment may NAME a rejected shape (this file's own
/// docs record the A3 rejection) without shipping it.
fn code_only(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The brace-balanced text of the block that starts on `offset`'s line
/// (the trait block / an impl block), counted from the first `{` at or
/// after the offset.
fn block_at(source: &str, needle: &str) -> String {
    let start = source.find(needle).unwrap_or_else(|| panic!("no {needle:?} in source"));
    let bytes: Vec<char> = source[start..].chars().collect();
    let mut depth = 0usize;
    let mut opened = false;
    let mut out = String::new();
    for c in bytes {
        out.push(c);
        if c == '{' {
            depth += 1;
            opened = true;
        } else if c == '}' {
            depth -= 1;
            if opened && depth == 0 {
                return out;
            }
        }
    }
    panic!("unbalanced block at {needle:?}");
}

#[test]
fn the_trait_exposes_extraction_only() {
    let port = read_source("driver.rs");

    // (A4) The trait declares EXACTLY ONE method — today's extract shape.
    let trait_block = block_at(&port, "pub(crate) trait Driver");
    let fns: Vec<&str> = trait_block
        .lines()
        .filter(|line| line.trim_start().starts_with("fn "))
        .collect();
    assert_eq!(fns.len(), 1, "the trait declares exactly one method: {fns:?}");
    assert!(
        fns[0].contains("fn extract(")
            && fns[0].contains("&self")
            && fns[0].contains("root: &Path")
            && fns[0].contains("declared: &DeclaredFacts")
            && fns[0].contains("Result<Model, String>"),
        "the one method is today's extract shape: {}",
        fns[0]
    );
    // (A4) No matcher surface grows on the port: the four words name the
    // join machinery, which stays at the ONE engine-side site. The audit
    // reads code lines — docs may name the rejected machinery, code may
    // not grow it.
    let port_code = code_only(&port);
    for word in ["resolve", "contribute", "accept", "pattern"] {
        assert!(
            !port_code.to_lowercase().contains(word),
            "the word {word:?} must not appear on the port surface — the join and the matcher verbs stay engine-side (A4)"
        );
    }

    // (A4) Each driver implements the port with exactly one method and
    // contributes nothing else to it.
    for file in [
        "scan/rust.rs",
        "scan/csharp/mod.rs",
        "scan/go/mod.rs",
        "scan/kotlin/mod.rs",
        "scan/python/mod.rs",
        "scan/ts/mod.rs",
    ] {
        let source = read_source(file);
        let occurrences = source.matches("impl crate::driver::Driver for").count();
        assert_eq!(occurrences, 1, "{file}: exactly one driver-port impl");
        let block = block_at(&source, "impl crate::driver::Driver for");
        let fns: Vec<&str> = block
            .lines()
            .filter(|line| line.trim_start().starts_with("fn "))
            .collect();
        assert_eq!(
            fns.len(),
            1,
            "{file}: the port impl declares one method: {fns:?}"
        );
        assert!(fns[0].contains("fn extract("), "{file}: {}", fns[0]);
        let impl_code = code_only(&block);
        for word in ["resolve", "contribute", "accept", "pattern"] {
            assert!(
                !impl_code.to_lowercase().contains(word),
                "{file}: the word {word:?} must not appear on the port impl surface (A4)"
            );
        }
    }

    // (A3) No runtime registration anywhere: the boxed driver vector and
    // the `dyn Driver` box are the shapes the record rejects, and they
    // appear in NO file of the crate. (`Box<dyn Driver` covers
    // `Vec<Box<dyn Driver>>` structurally.)
    let mut offenders = Vec::new();
    for path in walkdir::WalkDir::new(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("rs"))
    {
        let text = std::fs::read_to_string(path.path()).expect("read src file");
        let code = code_only(&text);
        for shape in ["Box<dyn Driver", "Vec<Box<dyn"] {
            if code.contains(shape) {
                offenders.push(format!("{}: {shape}", path.path().display()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "no runtime registration may ship (A3): {offenders:?}"
    );

    // (A3) The seam files — the port and its table — carry no lock-guarded
    // lookup and no register verb: the table is a compile-time match.
    // Named exemptions OUTSIDE the seam files, by their owners: the
    // parallel fact leg (`scan/leg.rs`) owns a `Vec<OnceLock<…>>` of PER-FILE
    // result slots (not a driver table) and `events::unitid` owns an index
    // `register` (not a driver registration) — both predate the seam and
    // neither is a lookup inside a walk loop.
    for file in ["driver.rs", "scan/mod.rs"] {
        let text = code_only(&read_source(file));
        for shape in ["OnceLock", "Mutex", "Box<dyn", "register"] {
            assert!(
                !text.contains(shape),
                "{file}: the seam must be compile-time — {shape:?} is forbidden (A3)"
            );
        }
    }

    // The table itself: the ONE exhaustive match, six unit-struct entries.
    let seam = read_source("scan/mod.rs");
    let table = block_at(&seam, "fn driver(language: Language) -> &'static dyn Driver");
    assert_eq!(
        table.matches("=> &").count(),
        6,
        "the table dispatches the six Language variants: {table}"
    );
    for entry in [
        "&rust::RustDriver",
        "&csharp::CsharpDriver",
        "&go::GoDriver",
        "&kotlin::KotlinDriver",
        "&python::PythonDriver",
        "&ts::TsDriver",
    ] {
        assert!(table.contains(entry), "the table names {entry}");
    }
}

// ---------------------------------------------------------------------------
// cell F — the candidate-order pin (`scan.json` never proves it)
// ---------------------------------------------------------------------------

fn candidate_shape(candidates: &[LiteralCandidate]) -> Vec<(String, String, String)> {
    candidates
        .iter()
        .map(|candidate| {
            let anchor = match &candidate.anchor {
                LiteralCandidateAnchor::Call { callee, position } => {
                    format!("call({callee}@{position})")
                }
                LiteralCandidateAnchor::ExportedProperty { property } => {
                    format!("property({property})")
                }
                LiteralCandidateAnchor::Field { field } => format!("field({field})"),
            };
            (candidate.path.clone(), anchor, candidate.value.clone())
        })
        .collect()
}

/// The path → anchor → position order each driver produces on its fixed
/// two-file fixture, read through the library field itself. The drivers
/// sort their file lists by path (`files.sort()` / the go package BTreeMap),
/// and the within-file order is source position — so the exact sequence is
/// pinned, and the refactored table must carry every driver's contribution
/// path byte-identically.
#[test]
fn literal_candidate_order_is_pinned_per_driver() {
    // rust / go / python / ts: one `emit` call per file, files sorted.
    for (language, expected) in [
        (
            Language::Rust,
            vec![
                ("src/a.rs".to_string(), "call(emit@0)".to_string(), "seam-a".to_string()),
                ("src/b.rs".to_string(), "call(emit@0)".to_string(), "seam-b".to_string()),
            ],
        ),
        (
            Language::Go,
            vec![
                ("a.go".to_string(), "call(emit@0)".to_string(), "seam-a".to_string()),
                ("b.go".to_string(), "call(emit@0)".to_string(), "seam-b".to_string()),
            ],
        ),
        (
            Language::Python,
            vec![
                ("a.py".to_string(), "call(emit@0)".to_string(), "seam-a".to_string()),
                ("b.py".to_string(), "call(emit@0)".to_string(), "seam-b".to_string()),
            ],
        ),
        (
            Language::Ts,
            vec![
                ("a.ts".to_string(), "call(emit@0)".to_string(), "seam-a".to_string()),
                ("b.ts".to_string(), "call(emit@0)".to_string(), "seam-b".to_string()),
            ],
        ),
    ] {
        let fixture = fixture_for(language);
        let model = scan::extract_with(language, &fixture.root, &declared_for(language))
            .expect("fixture scans");
        assert_eq!(
            candidate_shape(&model.literal_candidates),
            expected,
            "{language:?}: the candidate sequence the driver pushes through the seam is pinned"
        );
    }

    // kotlin feeds the exported-property arm (scenario 06/09): the two
    // top-level consts, files sorted, property anchors.
    let fixture = fixture_for(Language::Kotlin);
    let model = scan::extract_with(Language::Kotlin, &fixture.root, &declared_for(Language::Kotlin))
        .expect("kotlin fixture scans");
    assert_eq!(
        candidate_shape(&model.literal_candidates),
        vec![
            ("a.kt".to_string(), "property(SEAM_A)".to_string(), "seam-a".to_string()),
            ("b.kt".to_string(), "property(SEAM_B)".to_string(), "seam-b".to_string()),
        ],
        "kotlin: the property arm keeps its order through the seam"
    );

    // csharp feeds the call arm (workplan visibility_phase2 scenario 03,
    // the sixth driver): the two call sites under walk order — sorted in
    // the pin, WalkDir file order being unpinned — with call anchors.
    let fixture = fixture_for(Language::Csharp);
    let model = scan::extract_with(Language::Csharp, &fixture.root, &declared_for(Language::Csharp))
        .expect("csharp fixture scans");
    let mut shape = candidate_shape(&model.literal_candidates);
    shape.sort();
    assert_eq!(
        shape,
        vec![
            ("App/First.cs".to_string(), "call(Emit@0)".to_string(), "seam-a".to_string()),
            ("App/Second.cs".to_string(), "call(Emit@0)".to_string(), "seam-b".to_string()),
        ],
        "csharp contributes the call candidates through the seam: {:?}",
        model.literal_candidates
    );
}

/// The resolution side of the order law: `Model::sort_literal_sites` is the
/// normalizer — the engine seam calls it after building the rows — so the
/// resolved `literal_sites` rows and the census counts are invariant under
/// a permutation of the candidate PUSH order. The cell welds the ONE
/// resolution site (`events::patterns::resolve_literal_sites`) and names
/// the normalizer by calling it on both arms.
#[test]
fn resolved_rows_are_invariant_under_candidate_push_order() {
    use archspec::events::patterns::{literal_site_census, resolve_literal_sites};
    use archspec::events::unitid::UnitIdIndex;

    // A two-member rust workspace: `api` publishes, `worker` subscribes.
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nmembers = [\"api\", \"worker\"]\nresolver = \"2\"\n",
    );
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "pub fn announce() {\n    emit(\"orders.created\");\n}\n\nfn emit(_e: &str) {}\n",
    );
    fixture.write(
        "worker/Cargo.toml",
        "[package]\nname = \"worker\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "worker/src/lib.rs",
        "pub fn init() {\n    listen(\"orders.created\");\n}\n\nfn listen(_e: &str) {}\n",
    );
    let declared = DeclaredFacts {
        patterns: vec![
            DeclaredPattern {
                id: "demo-bus".to_string(),
                language: "rust".to_string(),
                role: Some("publish".to_string()),
                verb: DeclaredPatternVerb::CallNameArgPosition {
                    call: "emit".to_string(),
                    arg: 0,
                },
            },
            DeclaredPattern {
                id: "demo-bus".to_string(),
                language: "rust".to_string(),
                role: Some("subscribe".to_string()),
                verb: DeclaredPatternVerb::CallNameArgPosition {
                    call: "listen".to_string(),
                    arg: 0,
                },
            },
        ],
        ..DeclaredFacts::default()
    };

    let model = scan::extract_with(Language::Rust, &fixture.root, &declared).expect("scan");
    assert!(
        model.literal_candidates.len() >= 2,
        "both sides contribute (the feed is vocabulary-free)"
    );

    // The index the engine seam builds (the same construction as
    // `archspec::scan::extract_with_spec`).
    let mut index = UnitIdIndex::new();
    for unit in &model.units {
        let spellings = std::iter::once(unit.name.clone()).chain(unit.crate_ids.iter().cloned());
        index.register(&unit.path, spellings);
    }

    let resolve_sorted = |candidates: Vec<LiteralCandidate>| -> (Model, Vec<LiteralCandidate>) {
        let outcome = resolve_literal_sites(&index, &declared.patterns, &candidates);
        assert!(outcome.refusals.is_empty(), "{:?}", outcome.refusals);
        // The normalizer the cell names: the production seam applies
        // `Model::sort_literal_sites` after building the rows.
        let mut container = Model::new("rust");
        container.literal_sites = outcome.sites;
        container.sort_literal_sites();
        (container, candidates)
    };

    let (straight, _) = resolve_sorted(model.literal_candidates.clone());
    let mut permuted = model.literal_candidates.clone();
    permuted.reverse(); // the named permutation of the push order
    assert_ne!(
        candidate_shape(&model.literal_candidates),
        candidate_shape(&permuted),
        "the permutation must actually move the push order"
    );
    let (reversed, _) = resolve_sorted(permuted);

    assert_eq!(
        straight.literal_sites, reversed.literal_sites,
        "the resolved literal_sites rows are invariant under the push-order permutation (sort_literal_sites normalizes)"
    );
    assert_eq!(
        literal_site_census(&straight.literal_sites),
        literal_site_census(&reversed.literal_sites),
        "the census counts are invariant under the push-order permutation"
    );
}
