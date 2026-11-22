//! Scenario 03 — generic literal-site pattern primitives and the declared
//! pattern-pair face (workplan `events_value_join`, ADR-069 clauses 4.4a(v)
//! and 4.7(iii), decision-check F2/F3/F8/F10). The done-when cells:
//!
//! - (cell A) renaming the fixture call `emitEvent` → `xyzzy` in the source
//!   AND in the declaration (semantics unchanged) moves ZERO verdicts — the
//!   engine matches name EQUALITY against the declared noun and has no
//!   name-inference arm (the F3 proving test);
//! - (cell B) the vocabulary census (4.7(iii), the F2 falsifiable test)
//!   greps the engine matcher source `src/archspec/events/` for framework,
//!   application and vendor name strings with a ZERO baseline — any new hit
//!   names itself (`rust-arch-test-kit/AGENTS.md` §Archspec engine
//!   boundary);
//! - (cell C) the scenario-09 flip: the go and python sides are FUNDED —
//!   each prints its `literal sites` count line (zero stated, never
//!   silent) and the `driver-gap` line stands down; the funded set (ts,
//!   rust, kotlin) keeps its count lines;
//! - (cell D) the not-declared fixture's `scan.json` serializes NO
//!   literal-site bytes and stays sha256-identical to the pinned pre-change
//!   digest (4.2 arm 1 — the declaration gate, not a special case).
//!
//! The row shape is the F8 seam: `{unit, path, language, value, anchor,
//! pattern}` with `language` a ROW FIELD, never a join arm. The join itself
//! (scenario 04) is out of scope: this file stops at literal-site rows in
//! the shared collection. Names are generic; the witness-corpus shape (a
//! call-name emit pattern plus an exported-property-value subscribe pattern)
//! is expressed with SYNTHETIC nouns only — corpus names stay out of
//! `public/**` (F10).

mod common;

use common::{stderr, stdout, Fixture};

/// Compact, self-checked sha256 (the suite's existing dependency-free
/// pattern — `extraction_freeze.rs`, `events_value_join_unitid.rs` carry the
/// same module; here only the digest helper is used).
mod sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];

    fn digest(data: &[u8]) -> [u32; 8] {
        let mut h = [
            0x6a09e667u32, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let mut message = data.to_vec();
        let bit_len = (data.len() as u64).wrapping_mul(8);
        message.push(0x80);
        while message.len() % 64 != 56 {
            message.push(0);
        }
        message.extend_from_slice(&bit_len.to_be_bytes());
        for block in message.chunks(64) {
            let mut w = [0u32; 64];
            for (i, word) in w.iter_mut().take(16).enumerate() {
                *word = u32::from_be_bytes(block[i * 4..i * 4 + 4].try_into().expect("4 bytes"));
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let mut v = h;
            for i in 0..64 {
                let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
                let ch = (v[4] & v[5]) ^ ((!v[4]) & v[6]);
                let t1 = v[7]
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
                let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
                let t2 = s0.wrapping_add(maj);
                v[7] = v[6];
                v[6] = v[5];
                v[5] = v[4];
                v[4] = v[3].wrapping_add(t1);
                v[3] = v[2];
                v[2] = v[1];
                v[1] = v[0];
                v[0] = t1.wrapping_add(t2);
            }
            for (a, b) in h.iter_mut().zip(v) {
                *a = a.wrapping_add(b);
            }
        }
        h
    }

    /// The digest of `data`, lowercase hex. Self-checked against the published
    /// vectors so a broken helper can never vouch for a model digest.
    pub fn sha256_hex(data: &[u8]) -> String {
        assert_eq!(
            digest(b""),
            [
                0xe3b0c442, 0x98fc1c14, 0x9afbf4c8, 0x996fb924, 0x27ae41e4, 0x649b934c, 0xa495991b,
                0x7852b855
            ],
            "sha256 support: empty-string vector"
        );
        digest(data)
            .iter()
            .map(|word| format!("{word:08x}"))
            .collect()
    }
}

use sha256::sha256_hex;

/// The digest pinned by scenario 01 (`events_value_join_unitid.rs`) against
/// the pre-change binary for the not-declaring workspace below: the
/// literal-site collection is skip-when-empty and candidate-feed
/// serialization-skipped, so scenario 03 must leave these bytes UNMOVED —
/// a moved digest here is the 4.2-arm-1 breach, named by this test.
const NON_DECLARING_SCAN_JSON_SHA256: &str =
    "74c57e9ccf7cd407f458ea7f9e572fa71ea6bae98d0455a167de94206b236431";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The pattern-pair declaration of the synthetic demo bus: the emit side is
/// the call-name+argument-position verb on the noun the caller names
/// (`emit_noun`), the subscribe side is the exported-property-value verb on
/// `event`. SYNTHETIC nouns only (F10): this pair is the witness-corpus SHAPE
/// — one call anchor, one exported-property anchor — spelled with names no
/// corpus tree owns.
fn demo_pair_spec(emit_noun: &str) -> String {
    format!(
        "[project]\nlanguage = \"ts\"\n\n[[pattern_pair]]\nid = \"demo-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"ts\"\nrole = \"publish\"\ncall = \"{emit_noun}\"\narg = 0\n\n\
         [pattern_pair.subscribe]\nlanguage = \"ts\"\nrole = \"subscribe\"\nproperty = \"event\"\n"
    )
}

/// One single-package ts tree: a publish call whose first argument is the
/// event string and an exported metadata object whose `event` property
/// carries the same string. The call noun is the caller's, so the rename
/// cell can spell the SAME tree under two nouns.
fn ts_publish_tree(emit_noun: &str) -> Fixture {
    let fixture = Fixture::new();
    fixture.write(
        "package.json",
        "{\n  \"name\": \"demo-app\",\n  \"version\": \"1.0.0\"\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        &demo_pair_spec(emit_noun),
    );
    fixture.write(
        "src/publish.ts",
        &format!(
            "export function announce(id: number) {{\n  {emit_noun}(\"orders.created\", {{ id }});\n}}\n"
        ),
    );
    fixture.write(
        "src/meta.ts",
        "export const metadata = {\n  event: \"orders.created\",\n};\n",
    );
    fixture
}

// ---------------------------------------------------------------------------
// cell A — renaming the call moves zero verdicts (F3 proving test)
// ---------------------------------------------------------------------------

/// The engine matches the declared noun by EQUALITY and carries no
/// name-inference arm: renaming `emitEvent` → `xyzzy` in the source AND in
/// the declaration (the semantics are identical strings at identical
/// positions) moves ZERO verdicts — the model bytes match byte-for-byte,
/// the census lines match, and the literal-site rows keep the DECLARED
/// pattern id, never the call's name.
#[test]
fn renaming_the_declared_call_moves_zero_verdicts() {
    let declared = ts_publish_tree("emitEvent");
    let renamed = ts_publish_tree("xyzzy");
    let first = declared.run(&["scan", "--output", "-"]);
    let second = renamed.run(&["scan", "--output", "-"]);
    assert_eq!(
        first.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        stderr(&first)
    );
    assert_eq!(
        second.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        stderr(&second)
    );
    let first_json = stdout(&first);
    let second_json = stdout(&second);
    assert!(
        first_json.contains("\"literal_sites\":["),
        "a declaring scan states its literal-site rows:\n{first_json}"
    );
    // The F8 seam shape with the tree-root unit id, both anchors, and the
    // DECLARED pattern id — the call name never reaches a row.
    assert!(
        first_json.contains(
            "{\"unit\":\".\",\"path\":\"src/meta.ts\",\"language\":\"ts\",\"value\":\"orders.created\",\"anchor\":\"exported-property-value\",\"pattern\":\"demo-bus\"}"
        ),
        "the exported-property site must carry the seam shape:\n{first_json}"
    );
    assert!(
        first_json.contains(
            "{\"unit\":\".\",\"path\":\"src/publish.ts\",\"language\":\"ts\",\"value\":\"orders.created\",\"anchor\":\"call-name+arg-position\",\"pattern\":\"demo-bus\"}"
        ),
        "the call site must carry the seam shape:\n{first_json}"
    );
    assert_eq!(
        first_json, second_json,
        "renaming the call AND its declaration moved model bytes — an \
         name-inference arm would be the only way (F3)"
    );
    // The census lines (the never-silent surface) are identical too.
    let census: Vec<String> = stderr(&first)
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .map(str::to_string)
        .collect();
    assert_eq!(
        census,
        vec![
            "literal sites: pattern \"demo-bus\" language \"ts\" anchor call-name+arg-position sites=1",
            "literal sites: pattern \"demo-bus\" language \"ts\" anchor exported-property-value sites=1",
        ],
        "the census lines must state one line per declared side in \
         declaration order, identical across the rename (stderr: {})",
        stderr(&first)
    );
    let census_renamed: Vec<String> = stderr(&second)
        .lines()
        .filter(|line| line.starts_with("literal sites:"))
        .map(str::to_string)
        .collect();
    assert_eq!(census, census_renamed, "the census moved with the rename — F3 breach");
}

// ---------------------------------------------------------------------------
// cell B — the vocabulary census with a ZERO baseline (F2 falsifiable test)
// ---------------------------------------------------------------------------

/// The falsifiable form of the engine-boundary law (operator law
/// 2026-11-17, ADR-069 4.7(iii)): the engine matcher source — every module
/// under `src/archspec/events/` — carries ZERO framework, application,
/// ecosystem or vendor name strings. The names below are the families the
/// corpus pairs actually match on (the F2 finding list, genericized); any
/// hit means a noun entered the engine, and the hit names itself. The
/// engine ships the two VERBS and reads the nouns from config ONLY.
#[test]
fn engine_matcher_source_carries_zero_framework_vocabulary() {
    let forbidden: &[&str] = &[
        "emitEvent",
        "EventEmitter",
        "addEventListener",
        "dispatchEvent",
        "removeEventListener",
        "postMessage",
        "EventBus",
        "eventBus",
        "MessageBus",
        "PubSub",
        "pubsub",
        "kafka",
        "Kafka",
        "rabbitmq",
        "RabbitMQ",
        "nats",
        "NATS",
        "tauri",
        "Tauri",
        "redis",
        "Redis",
        "mqtt",
        "MQTT",
        "socket.io",
        "emitter",
        "EventEmitter2",
        "windi",
    ];
    let events_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/archspec/events");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(events_dir)
        .expect("events module dir")
        .map(|entry| entry.expect("dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "the census must read the matcher modules");
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read matcher module");
        for name in forbidden {
            assert!(
                !text.contains(name),
                "engine-boundary breach (ADR-069 4.7(iii)): matcher module {} \
                 carries the framework-vocabulary string {name:?} — nouns belong \
                 in the declared pattern, verbs here",
                file.display()
            );
        }
    }
}

/// The scanner-side half of the vocabulary census (workplan
/// visibility_phase2 scenario 03, A5): the csharp manifest walk derives its
/// edges from EVERY item element's `Include` value, path-resolved — so no
/// item-element NAME is needed anywhere. The strings below are MSBuild item
/// type names (the `Protobuf` law: this name must NEVER appear in engine or
/// scanner code); a future item-name branch names itself red here. The
/// walk's structural schema names (`ProjectReference`, `PackageReference`,
/// `Target`, `ItemGroup`, `Condition`, `Include`) are platform manifest
/// schema, not framework vocabulary, and stay out of this list by design.
/// Baseline: ZERO hits over `src/scan/csharp/**` — verified at landing.
#[test]
fn csharp_scanner_walk_carries_zero_item_name_vocabulary() {
    let forbidden: &[&str] = &[
        "Protobuf",
        "\"Compile\"",
        "\"None\"",
        "\"Content\"",
        "\"Using\"",
        "\"Folder\"",
        "\"EmbeddedResource\"",
        "\"AdditionalFiles\"",
        "\"Analyzer\"",
    ];
    let scanner_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/scan/csharp");
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    let mut stack = vec![std::path::PathBuf::from(scanner_dir)];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("csharp scanner module dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    assert!(!files.is_empty(), "the census must read the csharp scanner modules");
    for file in files {
        let text = std::fs::read_to_string(&file).expect("read csharp scanner module");
        for name in forbidden {
            assert!(
                !text.contains(name),
                "name-free-law breach (workplan visibility_phase2 scenario 03, A5): \
                 scanner module {} carries the item-name string {name:?} — the rule \
                 is every item element's `Include` value, path-resolved; item names \
                 are never read",
                file.display()
            );
        }
    }
}

// ---------------------------------------------------------------------------
// cell C — the funded-drivers flip: sites counts, zero driver-gap rows
// ---------------------------------------------------------------------------

/// The scenario-09 flip of the scenario-03 gap cell: go and python are
/// FUNDED drivers now, so a declared side on each prints its `literal
/// sites` count line — never a `driver-gap` row (the capability rows read
/// granular), never silence, never a red run. The go side's call site joins
/// the model; the python side states its ZERO. The `driver-gap` mechanism
/// itself stays in the report surface for a driver added before its feed
/// ships — unreachable for every driver shipped today.
#[test]
fn funded_go_and_python_sides_print_sites_counts_not_driver_gap() {
    let fixture = Fixture::new();
    fixture.write(
        "go.mod",
        "module demo/shop\n\ngo 1.22\n",
    );
    fixture.write(
        "main.go",
        "package main\n\nfunc main() {\n\tpublish(\"orders.created\")\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[pattern_pair]]\nid = \"demo-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"go\"\nrole = \"publish\"\ncall = \"publish\"\narg = 0\n\n\
         [pattern_pair.subscribe]\nlanguage = \"python\"\nrole = \"subscribe\"\ncall = \"subscribe\"\narg = 0\n",
    );
    let output = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a funded scan must pass (stderr: {})",
        stderr(&output)
    );
    let error_text = stderr(&output);
    let lines: Vec<&str> = error_text
        .lines()
        .filter(|line| line.starts_with("literal sites:") || line.starts_with("driver-gap:"))
        .collect();
    assert_eq!(
        lines,
        vec![
            "literal sites: pattern \"demo-bus\" language \"go\" anchor call-name+arg-position sites=1",
            "literal sites: pattern \"demo-bus\" language \"python\" anchor call-name+arg-position sites=0",
        ],
        "one count line per declared side in declaration order, zero stated \
         on the language whose sources are absent, zero driver-gap rows \
         (stderr: {error_text})"
    );
    let model = stdout(&output);
    assert!(
        model.contains(
            "\"path\":\"main.go\",\"language\":\"go\",\"value\":\"orders.created\",\"anchor\":\"call-name+arg-position\",\"pattern\":\"demo-bus\""
        ),
        "the go call site rides the model with the F8 seam shape:\n{model}"
    );
}

// ---------------------------------------------------------------------------
// cell D — the not-declared arm stays byte-identical (4.2 arm 1)
// ---------------------------------------------------------------------------

/// A tree without a `[[pattern_pair]]` table: no candidate is collected,
/// no row is written, no report line prints, and `scan.json` stays
/// sha256-identical to the pinned pre-change digest — the declaration gate
/// is the byte identity, not a special case (the D8 law, same pin as the
/// scenario-01 cell C).
#[test]
fn not_declared_scan_json_stays_at_the_pinned_digest() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"shop_api\", \"shop_core\"]\n",
    );
    fixture.write(
        "shop_core/Cargo.toml",
        "[package]\nname = \"shop_core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "shop_core/src/lib.rs",
        "pub mod model;\n\npub fn record() -> model::Booked { model::Booked }\n",
    );
    fixture.write("shop_core/src/model.rs", "pub struct Booked;\n");
    fixture.write(
        "shop_api/Cargo.toml",
        "[package]\nname = \"shop_api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nshop_core = { path = \"../shop_core\" }\n",
    );
    fixture.write(
        "shop_api/src/lib.rs",
        "mod glue;\n\npub fn serve() -> shop_core::model::Booked { glue::book() }\n",
    );
    fixture.write(
        "shop_api/src/glue.rs",
        "pub fn book() -> shop_core::model::Booked { shop_core::record() }\n",
    );
    let first = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(
        first.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        stderr(&first)
    );
    let second = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(first.stdout, second.stdout, "scan bytes must be deterministic");
    let text = stdout(&first);
    assert!(
        !text.contains("literal_sites") && !text.contains("literal_candidates"),
        "the not-declared arm serializes no literal-site or candidate bytes:\n{text}"
    );
    assert!(
        !stderr(&first).contains("literal sites:") && !stderr(&first).contains("driver-gap:"),
        "the not-declared arm prints no census or gap lines (stderr: {})",
        stderr(&first)
    );
    assert_eq!(
        sha256_hex(&first.stdout),
        NON_DECLARING_SCAN_JSON_SHA256,
        "the not-declaring scan.json bytes moved — full bytes for the diff:\n{text}"
    );
}

// ---------------------------------------------------------------------------
// cell E — the vocabulary census extended to the driver field collectors
// (workplan visibility_phase2 scenario 02, A4: the field-value feeds are
// grammar-node knowledge — token KINDS and node KINDS, never macro names,
// never producer names, never string-keyed framework vocab)
// ---------------------------------------------------------------------------

/// The falsifiable form of A4: the rust field-value collector walks macro
/// token trees by KIND (literal, ident, colon, group) — its code carries
/// ZERO string literals and ZERO macro-producer names, so EVERY macro body
/// is walked the same way and none is special-cased. The ts field-value
/// collector names ONLY tree-sitter grammar node kinds. A producer name
/// (`json`, `serde_json`, `quote`, …) appearing in either region means a
/// macro-identity lookup entered the driver — the F2/F6 fence.
#[test]
fn driver_field_collectors_are_grammar_node_only() {
    let forbidden: &[&str] = &[
        "json",
        "serde_json",
        "json_macro",
        "quote",
        "macro_rules",
        "EventEmitter",
        "addEventListener",
        "dispatchEvent",
        "postMessage",
        "kafka",
        "Kafka",
        "tauri",
        "Tauri",
        "EventBus",
        "pubsub",
        "PubSub",
    ];
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    // The rust region: struct-field arm + token-tree walk + key/value
    // readers, bounded by the two function names that open and close it.
    let rust_src = std::fs::read_to_string(manifest.join("src/scan/rust.rs"))
        .expect("rust driver source");
    let rust_region = extract_region(&rust_src, "fn collect_struct_field_sites", "fn record_string_args");
    for line in rust_region.lines() {
        if line.trim_start().starts_with("///") || line.trim_start().starts_with("//") {
            continue; // prose may name families to forbid them
        }
        assert!(
            !line.contains('"'),
            "the rust field-value collector carries a string literal — the \
             walk must read token KINDS only (A4): {line}"
        );
    }
    for name in forbidden {
        assert!(
            !rust_region.contains(name),
            "driver-boundary breach (A4): the rust field-value collector \
             names the producer/macro family {name:?} — macro bodies are \
             walked structurally, never by identity"
        );
    }

    // The ts region: the pair walk, bounded by its own name and the next
    // walk's doc comment. Every string literal must be a grammar node kind.
    let ts_src = std::fs::read_to_string(manifest.join("src/scan/ts/syntax.rs"))
        .expect("ts driver source");
    let ts_region =
        extract_region(&ts_src, "fn collect_field_value_candidates", "/// The call-anchor walk");
    const ALLOWED_KINDS: &[&str] = &[
        "pair",
        "string",
        "array",
        "property_identifier",
        "identifier",
        "key",
        "value",
    ];
    for line in ts_region.lines() {
        if line.trim_start().starts_with("///") || line.trim_start().starts_with("//") {
            continue;
        }
        for name in forbidden {
            assert!(
                !line.contains(name),
                "driver-boundary breach (A4): the ts field-value collector \
                 names the family {name:?} — keys are read as nodes, \
                 never as known names"
            );
        }
        let mut rest = line;
        while let Some(open) = rest.find('"') {
            let after = &rest[open + 1..];
            let Some(close) = after.find('"') else { break };
            let quoted = &after[..close];
            assert!(
                ALLOWED_KINDS.contains(&quoted),
                "the ts field-value collector names {quoted:?} — only \
                 grammar node kinds ({ALLOWED_KINDS:?}) may appear as \
                 strings: {line}"
            );
            rest = &after[close + 1..];
        }
    }
}

fn extract_region(text: &str, start: &str, end: &str) -> String {
    let start = text
        .find(start)
        .unwrap_or_else(|| panic!("collector region start {start:?} not found"));
    let end = text[start..]
        .find(end)
        .unwrap_or_else(|| panic!("collector region end {end:?} not found"));
    text[start..start + end].to_string()
}

// ---------------------------------------------------------------------------
// cell F — the capability-misreport cure (F6/A3): a declared `field` side
// on a driver whose `field-value-census` reads `not-emitted` names the
// shape gap — never a `sites=0` silence-adjacent zero
// ---------------------------------------------------------------------------

/// Both sides on the go driver (funded for the call verb, NOT funded for
/// the field verb): two named `driver-gap` rows naming the FIELD-VALUE
/// census, zero `literal sites:` rows, exit 0 — the census is a census.
#[test]
fn field_side_on_unfunded_shape_names_gap_never_zero() {
    let fixture = Fixture::new();
    fixture.write("go.mod", "module demo/shop\n\ngo 1.22\n");
    fixture.write(
        "main.go",
        "package main\n\nfunc main() {\n\tmsg := map[string]string{\"topic\": \"orders.created\"}\n\t_ = msg\n}\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n[[pattern_pair]]\nid = \"field-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"go\"\nrole = \"publish\"\nfield = \"topic\"\n\n\
         [pattern_pair.subscribe]\nlanguage = \"go\"\nrole = \"subscribe\"\nfield = \"topic\"\n",
    );
    let output = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "a gap is report-only, never red (stderr: {})",
        stderr(&output)
    );
    let err = stderr(&output);
    assert_eq!(
        err.matches("driver-gap: pattern \"field-bus\"").count(),
        2,
        "both sides name the shape gap (stderr: {err})"
    );
    assert_eq!(
        err.matches("field-value census not-emitted").count(),
        2,
        "the gap row names the FIELD-VALUE shape, not the literal-site one \
         (stderr: {err})"
    );
    assert!(
        !err.contains("literal sites:"),
        "an unfunded shape never prints a sites line, never a sites=0 zero \
         (stderr: {err})"
    );
    let json = stdout(&output);
    assert!(
        !json.contains("\"literal_sites\""),
        "the go driver collects no field candidates: no rows (json: {json})"
    );
}

/// The honest face: a funded rust publish half plus an unfunded csharp
/// subscribe half — the csharp side names the shape gap and the rust site
/// lands in the join's `unfunded-shape` row (the counterpart driver is
/// read per SIDE SHAPE, so the language enters the unfunded set through
/// its field row): zero silence with no guessing at the missing half.
#[test]
fn funded_publish_half_names_unfunded_field_counterpart_not_silence() {
    let fixture = Fixture::new();
    fixture.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[pattern_pair]]\nid = \"field-bus\"\n\n\
         [pattern_pair.emit]\nlanguage = \"rust\"\nrole = \"publish\"\nfield = \"topic\"\n\n\
         [pattern_pair.subscribe]\nlanguage = \"csharp\"\nrole = \"subscribe\"\nfield = \"handles\"\n",
    );
    fixture.write(
        "src/lib.rs",
        "struct Frame {\n    topic: &'static str,\n}\n\npub fn frame() -> Frame {\n    Frame { topic: \"orders.created\" }\n}\n",
    );
    let output = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "the honest face is report-only (stderr: {})",
        stderr(&output)
    );
    let err = stderr(&output);
    assert!(
        err.contains("driver-gap: pattern \"field-bus\" language \"csharp\" — field-value census not-emitted"),
        "the csharp side names the field-value gap (stderr: {err})"
    );
    assert!(
        err.contains("unfunded-shape pattern=\"field-bus\" value=\"orders.created\" role subscribe language=\"csharp\""),
        "the rust publish site names the unfunded counterpart — zero \
         silence, the row is join.rs's own (stderr: {err})"
    );
    assert!(
        !err.contains("one-sided-leg"),
        "an UNFUNDED counterpart is not a silence-adjacent missing half — \
         it names the shape (stderr: {err})"
    );
}
