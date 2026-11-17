//! Async model-surface cells (workplan archspec_async_depends, scenario 02):
//! the async fact collection exists on the model, skips when empty, and sorts
//! by `(from, to, event)`; an async row never enters `edges`, never reddens
//! the edge adjudication for a lawful pub/sub pair, and never reaches the
//! `update` writer; a serialized direct edge carries no `kind` key — absent
//! `kind` means `direct`; a non-declaring repo's `scan.json` bytes are
//! sha256-pinned to the pre-change digest (D8 — the consumer's hard CI
//! constraint); `verify --coverage` gains no line from an async table (N4)
//! and the `init` skeleton bytes stand (N5).

mod common;

use common::{Fixture, stderr, stdout};
use rust_arch_test_kit::model::{AsyncEdge, Edge, EdgeKind, Model};

// ---------------------------------------------------------------------------
// SHA-256 support (self-checking; no host tool, no new dependency)
// ---------------------------------------------------------------------------

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

    fn digest(data: &[u8]) -> String {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
            0x5be0cd19,
        ];
        let bit_len = (data.len() as u64).wrapping_mul(8);
        let mut msg = data.to_vec();
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());
        for block in msg.chunks(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    block[4 * i],
                    block[4 * i + 1],
                    block[4 * i + 2],
                    block[4 * i + 3],
                ]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
                (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let tmp1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let tmp2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(tmp1);
                d = c;
                c = b;
                b = a;
                a = tmp1.wrapping_add(tmp2);
            }
            for (slot, v) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
                *slot = slot.wrapping_add(v);
            }
        }
        h.iter().map(|w| format!("{w:08x}")).collect()
    }

    /// The digest of `data`, lowercase hex. Self-checked against the published
    /// FIPS 180-4 vectors before it answers: a broken implementation must
    /// never mis-pin a byte identity silently.
    pub fn sha256_hex(data: &[u8]) -> String {
        assert_eq!(
            digest(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "sha256 support: empty-string vector"
        );
        assert_eq!(
            digest(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "sha256 support: \"abc\" vector"
        );
        digest(data)
    }
}

use sha256::sha256_hex;

/// The pre-change sha256 of `scan --output -` for the non-declaring fixture
/// below, recorded against the binary at main (0.6.82, commit a9bdd58ca)
/// before the async collection landed (scenario 02's Verification law: pin
/// BEFORE the change). The fixture is `plain_workspace` verbatim; the digest
/// covers the exact stdout bytes (581 bytes, one compact line). If the model
/// bytes ever move for a repo that declares nothing, this cell fails BEFORE
/// landing — that is the consumer's D8 freshness promise.
const NON_DECLARING_SCAN_JSON_SHA256: &str =
    "d80ec43d96f810283fb82bc5ced47c729dd4d2cdee4c60ac6f822ee601a56928";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// A non-declaring rust workspace: two members, one real manifest reference,
/// no `[[async_depends]]` anywhere. The digest fixture — its file bytes are
/// pinned, so the scan.json bytes are a pure function of this recipe.
fn plain_workspace(fixture: &Fixture) {
    fixture.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"api\", \"core\"]\n",
    );
    fixture.write(
        "core/Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "core/src/lib.rs",
        "pub struct OrderPlaced;\n\npub fn publish() -> OrderPlaced { OrderPlaced }\n",
    );
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ncore = { path = \"../core\" }\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "use core::OrderPlaced;\n\npub fn handle(event: OrderPlaced) {\n    let _ = event;\n}\n",
    );
}

/// The pub/sub tree with NO manifest reference at all: `core` publishes
/// `OrderPlaced` (a crate-root export feeding the declared-type census), the
/// consumer `api` binds a shadow record under its own name (the lawful M4
/// layout). The spec carries the two unit claims and the cycle guard; the
/// pair rides `[[async_depends]]` only when `declaring` — with no
/// `allowed.depend_on` (a declared direct dependency the code does not carry
/// is the `missing edge` class, the very adjudication B1 keeps async rows
/// out of).
fn workspace_pubsub(fixture: &Fixture, declaring: bool) {
    fixture.write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"api\", \"core\"]\n",
    );
    fixture.write(
        "core/Cargo.toml",
        "[package]\nname = \"core\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "core/src/lib.rs",
        "pub struct OrderPlaced;\n\npub fn publish() -> OrderPlaced { OrderPlaced }\n",
    );
    fixture.write(
        "api/Cargo.toml",
        "[package]\nname = \"api\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write(
        "api/src/lib.rs",
        "pub struct OrderPlaced;\n\npub fn handle(event: OrderPlaced) {\n    let _ = event;\n}\n",
    );
    let table = if declaring {
        "\n[[async_depends]]\nfrom = \"core\"\nto = \"api\"\nevent = \"core::OrderPlaced\"\n"
    } else {
        ""
    };
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"api\"\nmatches = {{ units = [\"api\"] }}\n\n[[module]]\nname = \"core\"\nmatches = {{ units = [\"core\"] }}\n\n[[constraint]]\ntype = \"no_cycles\"\n{table}"
        ),
    );
}

/// The same workspace plus an optional `[[async_depends]]` table over the
/// declared direct pair: identical constraint rows, one tree, one edge — the
/// only difference between the two coverage runs is the async table.
fn workspace_declared(fixture: &Fixture, declaring: bool) {
    plain_workspace(fixture);
    let table = if declaring {
        "\n[[async_depends]]\nfrom = \"core\"\nto = \"api\"\nevent = \"core::OrderPlaced\"\n"
    } else {
        ""
    };
    fixture.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"api\"\nmatches = {{ units = [\"api\"] }}\nallowed = {{ depend_on = [\"core\"] }}\n\n[[module]]\nname = \"core\"\nmatches = {{ units = [\"core\"] }}\n\n[[constraint]]\ntype = \"no_cycles\"\n{table}"
        ),
    );
}

fn coverage_lines(output: &std::process::Output) -> Vec<String> {
    stdout(output)
        .lines()
        .filter(|line| line.starts_with("coverage:"))
        .map(String::from)
        .collect()
}

// ---------------------------------------------------------------------------
// The collection: scaffold, skip-when-empty, kind law, sort law
// ---------------------------------------------------------------------------

/// The one scaffold site: a scaffolded model states no async facts at all,
/// so its serialized bytes are the empty model of the language — the
/// skip-when-empty collection contributes no key (`roles` convention).
#[test]
fn the_scaffold_carries_no_async_bytes() {
    let model = Model::new("rust");
    assert!(model.async_edges.is_empty(), "scaffolded empty at Model::new");
    let json = serde_json::to_string(&model).expect("scaffold serializes");
    assert!(
        !json.contains("async_edges"),
        "an empty async collection is absent from JSON: {json}"
    );
}

/// A declaring model serializes the async collection, states `kind` ONLY on
/// its async rows, and leaves `Edge` exactly `{from, to}` — no `kind` key on
/// any direct edge (B2: absent `kind` means direct; the shipped `edges`
/// bytes never move).
#[test]
fn a_declaring_model_serializes_the_async_collection_and_no_kind_key_on_direct_edges() {
    let mut model = Model::new("rust");
    model.language = "rust".to_string();
    model.edges = vec![Edge {
        from: "api".to_string(),
        to: "core".to_string(),
    }];
    model.async_edges = vec![AsyncEdge {
        from: "core".to_string(),
        to: "api".to_string(),
        event: "core::OrderPlaced".to_string(),
        kind: EdgeKind::Async,
    }];
    let value: serde_json::Value =
        serde_json::to_value(&model).expect("declaring model serializes");

    // The direct edge stays byte-identical to the shipped shape.
    let edges = value["edges"].as_array().expect("edges array");
    assert_eq!(
        edges[0],
        serde_json::json!({ "from": "api", "to": "core" }),
        "a direct edge serializes as {{from, to}} — no kind key"
    );

    // The async row is present in its own collection and carries the label.
    let async_rows = value["async_edges"]
        .as_array()
        .expect("async collection serializes when non-empty");
    assert_eq!(async_rows.len(), 1);
    assert_eq!(
        async_rows[0]["kind"], "async",
        "the async collection's own row is the only serialized kind carrier"
    );
    assert_eq!(async_rows[0]["event"], "core::OrderPlaced");

    // Round trip: the collection decodes back from its own rows.
    let round: Model = serde_json::from_value(value).expect("model round trips");
    assert_eq!(round.async_edges.len(), 1);
    assert_eq!(round.async_edges[0].kind, EdgeKind::Async);
}

/// The absent-means-direct law: the kind vocabulary defaults to `direct` —
/// the state every shipped `edges` row states by carrying no key at all —
/// and the closed vocabulary words match the serialized bytes one-to-one.
#[test]
fn absent_kind_means_direct_and_the_label_words_are_the_closed_vocabulary() {
    assert_eq!(EdgeKind::default(), EdgeKind::Direct);
    assert_eq!(
        serde_json::from_str::<EdgeKind>("\"direct\"").expect("direct decodes"),
        EdgeKind::Direct
    );
    assert_eq!(
        serde_json::from_str::<EdgeKind>("\"async\"").expect("async decodes"),
        EdgeKind::Async
    );
    assert_eq!(EdgeKind::Direct.as_str(), "direct");
    assert_eq!(EdgeKind::Async.as_str(), "async");
    // A shipped-shape edge row decodes unchanged — `Edge` has no kind field,
    // so absence structurally means direct (B2).
    let edge: Edge = serde_json::from_str("{\"from\":\"a\",\"to\":\"b\"}").expect("shipped edge");
    assert_eq!(edge.from, "a");
    assert_eq!(edge.to, "b");
}

/// The N6 determinism law: rows order by `(from, to, event)` whatever order
/// the producer built them in — the `FileEdge` sort precedent.
#[test]
fn async_rows_sort_by_from_to_event_before_any_consumer() {
    let mut model = Model::new("rust");
    model.async_edges = vec![
        AsyncEdge {
            from: "worker".to_string(),
            to: "audit".to_string(),
            event: "e::B".to_string(),
            kind: EdgeKind::Async,
        },
        AsyncEdge {
            from: "core".to_string(),
            to: "worker".to_string(),
            event: "e::Z".to_string(),
            kind: EdgeKind::Async,
        },
        AsyncEdge {
            from: "core".to_string(),
            to: "worker".to_string(),
            event: "e::A".to_string(),
            kind: EdgeKind::Async,
        },
        AsyncEdge {
            from: "core".to_string(),
            to: "api".to_string(),
            event: "e::C".to_string(),
            kind: EdgeKind::Async,
        },
    ];
    model.sort_async_edges();
    let order: Vec<(&str, &str, &str)> = model
        .async_edges
        .iter()
        .map(|row| (row.from.as_str(), row.to.as_str(), row.event.as_str()))
        .collect();
    assert_eq!(
        order,
        vec![
            ("core", "api", "e::C"),
            ("core", "worker", "e::A"),
            ("core", "worker", "e::Z"),
            ("worker", "audit", "e::B"),
        ],
        "rows sort by (from, to, event) — the event breaks the same-pair tie"
    );
}

// ---------------------------------------------------------------------------
// Byte identity: the pinned scan.json digest (D8)
// ---------------------------------------------------------------------------

/// A repo that declares no `[[async_depends]]` scans to model bytes whose
/// sha256 equals the pre-change pin: the collection is skip-when-empty and
/// `Edge` is unmoved, so no committed `scan.json` (and therefore no emitted
/// portfolio artifact, whose only input these bytes are) can churn.
#[test]
fn non_declaring_scan_json_digest_equals_the_pinned_sha256() {
    let fixture = Fixture::new();
    plain_workspace(&fixture);
    let first = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(
        first.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        stderr(&first)
    );
    let second = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(
        first.stdout, second.stdout,
        "scan model bytes must be deterministic across runs"
    );
    let bytes = &first.stdout;
    let text = String::from_utf8_lossy(bytes);
    assert!(
        !text.contains("async_edges"),
        "a non-declaring model states no async collection bytes: {text}"
    );
    // The `kind` key rides ONLY async rows — the shipped `edges` entries stay
    // exactly `{from, to}` (the unit rows' `kind` is the pre-existing unit
    // vocabulary, untouched).
    let value: serde_json::Value = serde_json::from_slice(bytes).expect("scan model JSON");
    for edge in value["edges"].as_array().expect("edges array") {
        let keys: Vec<&str> = edge
            .as_object()
            .expect("edge object")
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            vec!["from", "to"],
            "a direct edge row carries no kind key: {edge}"
        );
    }
    let observed = sha256_hex(bytes);
    assert_eq!(
        observed, NON_DECLARING_SCAN_JSON_SHA256,
        "the non-declaring scan.json bytes moved — full bytes for the diff:\n{text}"
    );
}

// ---------------------------------------------------------------------------
// Adjudication surfaces the collection must not touch (B1, N4, N5)
// ---------------------------------------------------------------------------

/// The B1 adversarial shape at the model surface: a declaring pub/sub pair
/// whose consumer carries NO manifest reference to the publisher verifies
/// GREEN at error class — the async row lives in its own collection, never
/// in `edges`, so the missing-edge and used-but-undeclared adjudication never
/// sees it.
#[test]
fn declaring_pub_sub_without_manifest_reference_verifies_green_at_error_class() {
    let fixture = Fixture::new();
    workspace_pubsub(&fixture, true);
    let output = fixture.run(&["verify", "."]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "verify must pass at error class:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
    let report = stdout(&output);
    assert!(
        !report.contains("used but undeclared")
            && !report.contains("missing edge")
            && !report.contains("disallowed cross-component")
            && !report.contains("async event unresolved")
            && !report.contains("async publish not derived")
            && !report.contains("async subscribe not derived"),
        "an async-only pair reddens no edge class and no async arm: {report}"
    );
}

/// N4: the coverage census prints one line per CONSTRAINT row; an
/// `[[async_depends]]` entry is not a constraint, so the declaring run's
/// census equals the non-declaring run's census line-for-line and no census
/// line mentions async.
#[test]
fn verify_coverage_gains_no_line_from_the_async_table() {
    let plain = Fixture::new();
    workspace_declared(&plain, false);
    let declaring = Fixture::new();
    workspace_declared(&declaring, true);
    let plain_run = plain.run(&["verify", ".", "--coverage"]);
    let declaring_run = declaring.run(&["verify", ".", "--coverage"]);
    assert_eq!(
        plain_run.status.code(),
        Some(0),
        "non-declaring verify must pass: {}",
        stdout(&plain_run)
    );
    assert_eq!(
        declaring_run.status.code(),
        Some(0),
        "declaring verify must pass: {}",
        stdout(&declaring_run)
    );
    let plain_lines = coverage_lines(&plain_run);
    let declaring_lines = coverage_lines(&declaring_run);
    assert!(
        !plain_lines.is_empty(),
        "the census carries its constraint row: {}",
        stdout(&plain_run)
    );
    assert_eq!(
        plain_lines, declaring_lines,
        "the coverage census must not gain a line from the async table"
    );
    assert!(
        declaring_lines.iter().all(|line| !line.contains("async")),
        "no coverage line states async facts: {declaring_lines:?}"
    );
}

/// N5: `archspec init` writes the existing skeleton — the async table is
/// optional spec prose that the scaffold never gains bytes from (the async
/// collection is a model fact, not a skeleton line).
#[test]
fn init_skeleton_bytes_stand() {
    let fixture = Fixture::new();
    fixture.write(
        "demo/Cargo.toml",
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    );
    let output = fixture.run(&["init", "demo"]);
    assert_eq!(
        output.status.code(),
        Some(0),
        "init must pass (stderr: {})",
        stderr(&output)
    );
    let spec = fixture.read("demo/architecture.spec.toml");
    assert_eq!(
        spec,
        "[project]\nlanguage = \"rust\"\n\n[[constraint]]\ntype = \"no_cycles\"\n",
        "the init skeleton bytes must stand unchanged: {spec}"
    );
    assert!(
        !spec.contains("async") && !spec.contains("kind"),
        "the skeleton states no async bytes: {spec}"
    );
}
