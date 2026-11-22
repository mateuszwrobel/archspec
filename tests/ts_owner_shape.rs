//! The owner-lookup shape guard (perf-hotspots record 2026-11-20, H1): the
//! module-edges assembly must not scale with `rows × member dirs`.
//!
//! The witness is DIFFERENTIAL, so a busy host cannot fake either verdict. Two
//! synthetic trees carry the SAME file count and the SAME number of module-edge
//! rows; they differ only in how many workspace member dirs those rows spread
//! over. A linear per-row scan over every member dir pays `rows × members` and
//! must show up as a large RATIO between the two trees' walls. An ancestor
//! lookup pays one walk of the file's own path, so its ratio stays at the
//! per-member housekeeping a workspace scan legitimately does — the member
//! manifest reads and the unit rows. The RATIO (wide wall over narrow wall) is
//! the committed witness, not an absolute millisecond delta: sibling fleet load
//! inflates every wall, so an absolute ceiling flakes while the indexed shape
//! stays (the named push-gate red of 2026-11-20, receipt in
//! `__log__/2026-11-20-file-unit-lookup-fix.md`). The pre-fix linear scan lands
//! far above the ratio ceiling, so the O(rows × members) shape cannot return
//! quietly.
//!
//! Every byte is generated in a temp dir from index-derived text: generic
//! `pkg<N>` dirs and `@syn/pkg<N>` names, so no corpus bytes and no
//! application vocabulary enter the payload.

mod common;

use common::Fixture;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

/// Sources per tree — identical on both shapes, so the walk, the parse leg and
/// the resolution leg cost the same on each.
const FILES: usize = 4_096;
/// Distinct import targets per source. Every import resolves to an existing
/// file, so each contributes exactly one module-edge row: rows per tree =
/// `FILES * IMPORTS`, identical on both shapes.
const IMPORTS: usize = 5;
/// Member dirs of the two shapes: the same rows spread over `2_048` members
/// (wide) or `64` members (narrow). The product a linear owner scan pays is
/// `rows × members`, so only the wide tree pays it.
const WIDE_MEMBERS: usize = 2_048;
const NARROW_MEMBERS: usize = 64;
/// Ceiling on the wide-over-narrow wall ratio, in percent: the indexed shape
/// measured 159 %–196 % (idle and fleet-load receipts ride
/// `__log__/2026-11-20-file-unit-lookup-fix.md`); the pre-fix linear scan
/// measured 1 648 % at load 29. The ceiling sits ~3x above the indexed maximum
/// and ~2.7x below the regression floor, so the shape cannot regress quietly.
const WIDE_OVER_NARROW_PCT_MAX: u128 = 600;
/// Timed run pairs per tree, interleaved wide-then-narrow in every round so
/// each pair sees the same host conditions; the MIN per tree stands, because a
/// sibling fleet inflates a run and never shrinks one.
const REPS: usize = 3;

/// One generated tree: `members` packages, `FILES / members` sources each,
/// `IMPORTS` distinct resolvable imports per source. The bytes are a pure
/// function of (member index, file index, shape size), so both shapes carry
/// the same file count, the same row count and the same row shape.
fn generate(root: &Path, members: usize) {
    fs::create_dir_all(root.join("packages")).expect("packages dir");
    fs::write(
        root.join("package.json"),
        "{\n  \"name\": \"synth-owner-bench\",\n  \"private\": true,\n  \
         \"workspaces\": [\"packages/*\"]\n}\n",
    )
    .expect("root manifest");
    let per = FILES / members;
    for member in 0..members {
        let dir = root.join("packages").join(format!("pkg{member}"));
        fs::create_dir_all(dir.join("src")).expect("member src dir");
        fs::write(
            dir.join("package.json"),
            format!(
                "{{\n  \"name\": \"@syn/pkg{member}\",\n  \"version\": \"0.0.0\",\n  \
                 \"main\": \"src/f0.ts\"\n}}\n"
            ),
        )
        .expect("member manifest");
        for index in 0..per {
            let mut source = format!("// synthetic owner-bench file pkg{member} f{index}\n");
            source.push_str(&format!("import {{ S }} from \"./f{}\";\n", (index + 1) % per));
            for hop in 1..IMPORTS {
                source.push_str(&format!(
                    "import {{ X{hop} }} from \"../../pkg{}/src/f{}\";\n",
                    (member + hop) % members,
                    (index + hop) % per,
                ));
            }
            source.push_str(&format!("\nexport const V{member}_{index} = {{ ok: true }};\n"));
            fs::write(dir.join("src").join(format!("f{index}.ts")), source).expect("source file");
        }
    }
}

/// One scan of one tree, writing the model to `out`.
fn scan(root: &Path, out: &Path) {
    let run = Command::new(env!("CARGO_BIN_EXE_archspec"))
        .args(["scan", ".", "--output", &out.display().to_string()])
        .current_dir(root)
        .output()
        .expect("run archspec scan");
    assert!(
        run.status.success(),
        "the owner-shape tree must scan green: {}",
        String::from_utf8_lossy(&run.stderr)
    );
}

/// Model rows of a tree — the parity check that makes the differential mean
/// anything: the two shapes must carry the SAME number of module-edge rows.
fn row_count(root: &Path) -> usize {
    let out = root.join("model.json");
    scan(root, &out);
    let text = fs::read_to_string(&out).expect("model json");
    let model: Value = serde_json::from_str(&text).expect("model json parses");
    model["module_edges"]
        .as_array()
        .map(Vec::len)
        .unwrap_or_default()
}

/// The guard: assembling module edges must cost the same per row whichever
/// member dirs the rows sit under. A wide-over-narrow ratio over the ceiling
/// says the owner lookup went back to scanning every member dir per row.
/// The [`row_count`] warm scans above already warm both trees' caches.
#[test]
fn module_edges_assembly_does_not_scale_with_rows_times_members() {
    let wide = Fixture::new();
    let narrow = Fixture::new();
    generate(&wide.root, WIDE_MEMBERS);
    generate(&narrow.root, NARROW_MEMBERS);

    let wide_rows = row_count(&wide.root);
    let narrow_rows = row_count(&narrow.root);
    assert_eq!(
        wide_rows, narrow_rows,
        "the two shapes must carry the SAME module-edge rows ({WIDE_MEMBERS} members x \
         {} files vs {NARROW_MEMBERS} members x {} files)",
        FILES / WIDE_MEMBERS,
        FILES / NARROW_MEMBERS
    );
    assert_eq!(
        wide_rows,
        FILES * IMPORTS,
        "every planted import must land as one module-edge row — a tree that \
         resolves nothing times nothing"
    );

    let wide_out = wide.root.join("model.json");
    let narrow_out = narrow.root.join("model.json");
    let mut wide_ms = u128::MAX;
    let mut narrow_ms = u128::MAX;
    for _ in 0..REPS {
        let started = Instant::now();
        scan(&wide.root, &wide_out);
        wide_ms = wide_ms.min(started.elapsed().as_millis());
        let started = Instant::now();
        scan(&narrow.root, &narrow_out);
        narrow_ms = narrow_ms.min(started.elapsed().as_millis());
    }
    let delta_ms = wide_ms.saturating_sub(narrow_ms);
    let ratio_pct = wide_ms.saturating_mul(100) / narrow_ms.max(1);
    eprintln!(
        "module-edges-assembly owner-shape: wide({WIDE_MEMBERS} members) {wide_ms} ms, \
         narrow({NARROW_MEMBERS} members) {narrow_ms} ms, delta {delta_ms} ms (report-only), \
         ratio {ratio_pct} %, ceiling {WIDE_OVER_NARROW_PCT_MAX} %"
    );
    assert!(
        ratio_pct <= WIDE_OVER_NARROW_PCT_MAX,
        "module-edges-assembly scales with the member count: the same {} rows over \
         {WIDE_MEMBERS} member dirs cost {ratio_pct} % of the {NARROW_MEMBERS}-member wall \
         ({wide_ms} ms vs {narrow_ms} ms, ceiling {WIDE_OVER_NARROW_PCT_MAX} %) — the per-row \
         owner lookup is linear in the members again (perf-hotspots record 2026-11-20 H1)",
        wide_rows
    );
}
