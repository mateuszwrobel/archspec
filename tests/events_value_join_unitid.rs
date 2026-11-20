//! Scenario 01 — tree-root unit canonicalization (workplan
//! `events_value_join`, ADR-069 clause 4.4a(i), decision-check F-W5). The
//! done-when cells:
//!
//! - (cell A) two producer rows naming one node by two spellings — a crate
//!   name and a package path — merge to ONE row with zero 4.4 identity-conflict
//!   refusals (the F-W5 proving test);
//! - (cell B) a bare `"."` producer row canonicalizes to the tree-root id
//!   owning the directory — the driver dot is retired as row identity — plus
//!   the path → owning-unit-id face the resolver exists for;
//! - (cell C) a non-declaring fixture's `scan.json` is sha256-identical to the
//!   digest recorded against the pre-change binary: canonicalization binds NEW
//!   multi-producer fact identity only, never a retroactive rename;
//! - (dispatch face) the shipped `AMBIGUOUS-MARKERS` row (issue #63, clause
//!   4.8's precondition as written) stays byte-identical: the resolver adds no
//!   refusals and no rows to the scan surface.
//!
//! The resolution site is binary-internal (`src/archspec/events/unitid.rs`):
//! no CLI verb reaches it until the fact writers (scenarios 03–04) land, so
//! these cells compile the ONE file into the suite through `include!` — macro
//! text to the lexical scanner, the welded bytes are the single site, never a
//! second implementation. When a writer reaches the resolver over a verb, the
//! cells migrate to driving that verb.

mod common;

#[allow(dead_code)]
mod resolver {
    include!("../src/archspec/events/unitid.rs");
}

use common::{Fixture, stderr, stdout};
use resolver::{canonical_dir, merge_rows, ProducerRow, UnitIdIndex};

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
/// below, recorded against the binary at the slice base (0.6.112) BEFORE the
/// canonical resolver landed — the same pin-before-the-change law as
/// `async_model_surface.rs`. The fixture is `non_declaring_workspace`
/// verbatim; the digest covers the exact stdout bytes (1376 bytes, one
/// compact line, location-independent). If any landing ever renames bytes a
/// single-producer path already serialized, this cell fails BEFORE the rename
/// ships.
const NON_DECLARING_SCAN_JSON_SHA256: &str =
    "74c57e9ccf7cd407f458ea7f9e572fa71ea6bae98d0455a167de94206b236431";

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// One tree with four units, spelled the way real drivers spell them: a tree
/// root, a desktop app reachable by an ecosystem (crate) name and by its
/// package path, a gateway reachable by its package name, and a build root
/// whose driver names its root unit only by the bare dot. Names are generic;
/// the shapes mirror the survey's unit-spelling divergence class without
/// naming any corpus tree (payload law).
fn demo_index() -> UnitIdIndex {
    let mut index = UnitIdIndex::new();
    index.register(".", ["demo_root"]);
    index.register("apps/desktop", ["desktop_lib", "@demo/desktop"]);
    index.register("gateway", ["gateway_core"]);
    index.register("apps/android", ["android_app"]);
    index
}

fn crate_name_row() -> ProducerRow {
    ProducerRow {
        from: "desktop_lib".to_string(),
        to: "gateway_core".to_string(),
        event: "chat-message".to_string(),
        producer_dir: "apps/desktop".to_string(),
    }
}

fn package_path_row() -> ProducerRow {
    ProducerRow {
        from: "apps/desktop".to_string(),
        to: "gateway".to_string(),
        event: "chat-message".to_string(),
        producer_dir: "gateway".to_string(),
    }
}

/// A non-declaring rust workspace: two members, one real manifest reference,
/// no events declaration anywhere. The digest fixture — its file bytes are
/// pinned, so the scan.json bytes are a pure function of this recipe.
fn non_declaring_workspace() -> Fixture {
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
    fixture
}

/// The issue #63 witness shape: a nested directory carrying root markers for
/// two languages inside a root the other driver claims.
fn polyglot_tree() -> Fixture {
    let fixture = Fixture::new();
    fixture.write("package.json", "{\"name\": \"parent-probe\", \"version\": \"0.1.0\"}\n");
    fixture.write(
        "pkg/Cargo.toml",
        "[package]\nname = \"polyglot-probe\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("pkg/package.json", "{\"name\": \"polyglot-probe\", \"version\": \"0.1.0\"}\n");
    fixture.write("pkg/src/lib.rs", "pub fn local_helper() -> u32 { 1 }\n");
    fixture.write("pkg/src/index.js", "export function localHelper() { return 1; }\n");
    fixture
}

// ---------------------------------------------------------------------------
// cell A — one node, two spellings: one merged row, zero 4.4 refusals
// ---------------------------------------------------------------------------

/// The F-W5 proving test: two producer rows naming one node by two spellings
/// merge to one row with zero identity-conflict refusals. Each producer spells
/// BOTH endpoints in its own ecosystem vocabulary (a crate name from the
/// desktop build, a package path from the gateway package); after the ONE
/// resolution site the identities are byte-equal, so the merge arm fires —
/// one `PLUGIN-OVERLAP` census line per merged identity — and the fail-closed
/// refusal arm stays silent.
#[test]
fn two_spellings_of_one_node_merge_to_one_row_with_zero_refusals() {
    let index = demo_index();
    let outcome = merge_rows(&index, &[crate_name_row(), package_path_row()]);
    assert!(
        outcome.refusals.is_empty(),
        "identity-equal producers must not reach the 4.4 refusal: {:?}",
        outcome.refusals
    );
    assert_eq!(
        outcome.rows.len(),
        1,
        "two spellings of one node must draw one row: {:?}",
        outcome.rows
    );
    let row = &outcome.rows[0];
    assert_eq!(row.from, "apps/desktop", "the crate-name spelling resolves to its unit's tree-root id");
    assert_eq!(row.to, "gateway", "the package-name spelling resolves to its unit's tree-root id");
    assert_eq!(row.event, "chat-message", "the join-key value rides the identity untouched");
    assert_eq!(
        outcome.overlaps.len(),
        1,
        "one merge draws exactly one census line: {:?}",
        outcome.overlaps
    );
    assert_eq!(
        outcome.overlaps[0],
        "PLUGIN-OVERLAP row=(apps/desktop, gateway, chat-message) producers=2 — identities equal after canonicalization, merged to one row"
    );
    // The merged identity is never a driver dot: both endpoints carry
    // tree-root ids on both sides of the merge.
    assert_ne!(row.from, ".");
    assert_ne!(row.to, ".");
}

// ---------------------------------------------------------------------------
// cell B — the bare dot is retired as row identity
// ---------------------------------------------------------------------------

/// A driver's bare `"."` unit name is retired as row identity: it canonicalizes
/// through the producer's own directory to the tree-root id OWNING that
/// directory — a gradle build root at `apps/android` names its root unit `"."`,
/// and the canonical id is `apps/android`, never the ambiguous dot. The dot is
/// never looked up as a spelling either: even a unit that (wrongly) registers
/// the dot among its spellings resolves the dot through ownership, so one
/// spelling can never name two nodes.
#[test]
fn bare_dot_canonicalizes_to_the_owning_tree_root_unit() {
    let index = demo_index();
    let row = ProducerRow {
        from: ".".to_string(),
        to: "gateway_core".to_string(),
        event: "connect".to_string(),
        producer_dir: "apps/android".to_string(),
    };
    let canonical = resolver::canonicalize_row(&index, &row)
        .expect("the endpoints resolve");
    assert_eq!(canonical.from, "apps/android", "the build-root dot names the owning unit");
    assert_eq!(canonical.to, "gateway");

    // A file path below the build root is owned by the same unit (the D-Module
    // path → tree-root unit id face).
    assert_eq!(
        index.path_owner("apps/android/app/src/main/main.kt").as_deref(),
        Ok("apps/android")
    );
    // Depth below a registered unit folds to that unit, and the deepest
    // registered directory wins over the root.
    assert_eq!(index.path_owner("apps/desktop").as_deref(), Ok("apps/desktop"));
    // A producer rooted AT the tree root canonicalizes the dot to the root
    // unit's own id — a resolved root id, not the retired driver dot.
    assert_eq!(index.resolve(".", ".").as_deref(), Ok("."));

    // The retired dot never keys the spelling table: a stray registration of
    // the dot is dropped, so the dot always answers by ownership.
    let mut dotted = UnitIdIndex::new();
    dotted.register("apps/android", [".", "android_app"]);
    assert_eq!(dotted.resolve(".", "apps/android").as_deref(), Ok("apps/android"));
    assert!(
        dotted.resolve(".", ".").is_err(),
        "with no registered unit owning the tree root the dot stays fail-closed"
    );

    // The normalization rule itself: collapse, up-step, root spelling.
    assert_eq!(canonical_dir("./apps//desktop/"), "apps/desktop");
    assert_eq!(canonical_dir("apps/../gateway"), "gateway");
    assert_eq!(canonical_dir(""), ".");
    assert_eq!(canonical_dir("../locked"), "../locked", "never resolves above the root spelling");
}

/// The fail-closed arm the merge law needs named: a spelling the tree cannot
/// answer refuses by name and the row is NOT merged — the #56 posture, now
/// the only way a producer row reaches the 4.4 refusal after canonicalization.
#[test]
fn unresolved_spelling_refuses_by_name_and_the_row_does_not_merge() {
    let index = demo_index();
    let row = ProducerRow {
        from: "ghost_pkg".to_string(),
        to: "gateway_core".to_string(),
        event: "chat-message".to_string(),
        producer_dir: "apps/desktop".to_string(),
    };
    let outcome = merge_rows(&index, &[row]);
    assert!(outcome.rows.is_empty(), "a refused row is never merged: {:?}", outcome.rows);
    assert_eq!(outcome.overlaps, Vec::<String>::new());
    assert_eq!(outcome.refusals.len(), 1);
    assert!(
        outcome.refusals[0].starts_with("UNITID-UNRESOLVED unit=ghost_pkg")
            && outcome.refusals[0].contains("row not merged"),
        "the refusal names the spelling and the outcome: {}",
        outcome.refusals[0]
    );
}

// ---------------------------------------------------------------------------
// cell C — existing serialized bytes are never renamed
// ---------------------------------------------------------------------------

/// The byte-identity law: a repo that declares nothing scans to model bytes
/// whose sha256 equals the pre-change pin. The canonical resolver is a NEW
/// site with NO call edge into any serialized path (scenarios 03–04 wire the
/// writers), so a single-producer scan is a pure function of the fixture —
/// two runs equal, and equal to the bytes the base binary emitted.
#[test]
fn non_declaring_scan_json_digest_equals_the_pinned_sha256() {
    let fixture = non_declaring_workspace();
    let first = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(
        first.status.code(),
        Some(0),
        "scan must pass (stderr: {})",
        stderr(&first)
    );
    let second = fixture.run(&["scan", "--output", "-"]);
    assert_eq!(first.stdout, second.stdout, "scan model bytes must be deterministic across runs");
    let text = String::from_utf8_lossy(&first.stdout).into_owned();
    assert!(
        !text.contains("PLUGIN-OVERLAP") && !text.contains("UNITID-UNRESOLVED"),
        "the canonicalizer emits no bytes into a single-producer model: {text}"
    );
    let observed = sha256_hex(&first.stdout);
    assert_eq!(
        observed, NON_DECLARING_SCAN_JSON_SHA256,
        "the non-declaring scan.json bytes moved — full bytes for the diff:\n{text}"
    );
}

/// The 4.8 dispatch law stays the stated precondition: the shipped
/// `AMBIGUOUS-MARKERS` row answers for a nested dual-marker directory
/// byte-identically to the resolvable base binary, and the resolver adds no
/// refusal line of its own to any scan surface. A base binary resolves as
/// `ARCHSPEC` -> `archspec` on PATH -> named SKIP (the global-install
/// resolution rule); the corpus battery leg repeats the same diff on the
/// witness tree.
#[test]
fn the_shipped_dispatch_face_stays_byte_identical_and_refusal_free() {
    let fixture = polyglot_tree();
    let fresh = fixture.run(&["scan", "."]);
    assert!(fresh.status.success(), "rooted scan failed: {}", stderr(&fresh));
    let fresh_err = stderr(&fresh);
    assert_eq!(
        fresh_err.matches("AMBIGUOUS-MARKERS").count(),
        1,
        "the dispatch face still fires exactly one row:\n{fresh_err}"
    );
    assert!(
        !fresh_err.contains("UNITID-UNRESOLVED"),
        "the canonical resolver adds no refusal to the scan face:\n{fresh_err}"
    );
    let Some(base) = base_binary() else {
        eprintln!("dispatch-parity: SKIP — no base binary (ARCHSPEC unset, none on PATH)");
        return;
    };
    let base_out = std::process::Command::new(&base)
        .args(["scan", "."])
        .current_dir(&fixture.root)
        .output()
        .expect("run base binary");
    let base_err = stderr(&base_out);
    let rows = |err: &str| -> Vec<String> {
        err.lines()
            .filter(|line| line.starts_with("AMBIGUOUS-MARKERS"))
            .map(str::to_string)
            .collect()
    };
    assert_eq!(
        rows(&fresh_err),
        rows(&base_err),
        "the AMBIGUOUS-MARKERS lines moved — fresh:\n{fresh_err}\nbase:\n{base_err}"
    );
    assert!(
        !base_err.is_empty() && !stdout(&fresh).is_empty(),
        "both binaries produced a scan surface"
    );
}

/// The base binary for the dispatch-parity weld: `ARCHSPEC` -> PATH, empty
/// when nothing resolves (the cell then states its SKIP).
fn base_binary() -> Option<String> {
    if let Some(path) = std::env::var("ARCHSPEC").ok().filter(|p| !p.is_empty()) {
        return Some(path);
    }
    let probe = std::process::Command::new("archspec")
        .arg("--version")
        .output()
        .ok()?;
    probe.status.success().then(|| "archspec".to_string())
}
