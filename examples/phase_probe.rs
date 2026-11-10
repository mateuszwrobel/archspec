//! Dev-only phase-budget probe (GitHub issue #34) — never shipped, never
//! published: the `[[example]]` entry carries `required-features`, so this
//! target only builds under `--features profile-phases`, and that feature
//! names nothing but dev-dependency externs (the graph proof lives in
//! `tests/profile_gate.rs`).
//!
//! It runs the REAL verify pipeline against the committed synthetic corpus
//! (`scripts/bench/gen_corpus.py`) under a hotpath guard. The pipeline tree
//! is included with `#[path]` — the same source the binary compiles — so
//! every phase boundary declared in `src/archspec/profile.rs` is live here.
//! The per-phase spans come from the recorder; the hotpath guard times the
//! whole run and writes its own report. The measured table prints on
//! stderr, so a run can throw the pipeline's stdout away (`... >/dev/null`).
//!
//! usage: cargo run --release --features profile-phases --example phase-probe
//!        -- <corpus-dir> [runs=5] [hotpath-report=hotpath_phase_report.json]

#[path = "../src/archspec/mod.rs"]
mod archspec;

use hotpath::{Format, HotpathGuardBuilder};
use std::collections::BTreeMap;
use std::path::Path;

/// The top-level phase labels, in pipeline order. `scan-facts` nests inside
/// `scan`; `cycle-detect` nests inside `compare` — the table states both
/// shares explicitly instead of double-counting them into the total.
const TOP_PHASES: [&str; 5] = ["spec-load", "scan", "mapping", "compare", "report"];

/// Seed the spec the corpus verifies against: one module per package unit,
/// no constraint rows. Written fresh every run so the probe is reproducible
/// from a bare generated tree.
fn seed_spec(root: &Path, packages: usize) -> Result<(), String> {
    let mut toml = String::from("[project]\nlanguage = \"ts\"\n\n");
    for pkg in 0..packages {
        toml.push_str(&format!(
            "[[module]]\nname = \"pkg{pkg}\"\nmatches = {{ units = [\"@syn/pkg{pkg}\"] }}\n\n"
        ));
    }
    std::fs::write(root.join("architecture.spec.toml"), toml)
        .map_err(|err| format!("cannot seed spec under {}: {err}", root.display()))
}

/// Corpus sanity before timing: the guard law (bench_sweep.py) owns the
/// byte hash; here the probe only refuses a tree that cannot carry the
/// declared package count at all.
fn corpus_present(root: &Path, packages: usize) -> Result<(), String> {
    for pkg in [0usize, packages - 1] {
        let dir = root.join("packages").join(format!("pkg{pkg}")).join("src");
        if !dir.is_dir() {
            return Err(format!(
                "corpus incomplete: {} absent — run scripts/bench/gen_corpus.py first",
                dir.display()
            ));
        }
    }
    Ok(())
}

fn median(values: &mut Vec<u128>) -> u128 {
    values.sort_unstable();
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        (values[mid - 1] + values[mid]) / 2
    } else {
        values[mid]
    }
}

fn main() -> Result<(), String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some(tree) = argv.first().map(std::path::PathBuf::from) else {
        return Err(
            "usage: phase-probe <corpus-dir> [runs=5] [hotpath-report=json]".to_string(),
        );
    };
    let runs: usize = argv
        .get(1)
        .map(|s| s.parse().map_err(|_| format!("bad runs value: {s}")))
        .transpose()?
        .unwrap_or(5);
    let report = argv
        .get(2)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("hotpath_phase_report.json"));

    let packages = 800usize;
    corpus_present(&tree, packages)?;
    seed_spec(&tree, packages)?;

    // The real verb, exactly the surface `archspec verify --json` speaks.
    // Cycles are expected findings on this corpus (the file-tier rings are
    // its point) — the run exits 1 and the timings stand.
    let dispatch_args = vec![
        "verify".to_string(),
        tree.display().to_string(),
        "--json".to_string(),
    ];
    let warmup = archspec::dispatch(&dispatch_args);
    eprintln!(
        "phase-probe: warmup run exit={warmup} (1 = rule violations reported, expected on the corpus)"
    );

    let mut per_label: BTreeMap<&'static str, Vec<u128>> = BTreeMap::new();
    {
        let guard = HotpathGuardBuilder::new("phase-probe")
            .output_path(&report)
            .format(Format::Json)
            .build();
        for _ in 0..runs {
            archspec::profile::reset();
            hotpath::measure_block!("verify-total", {
                archspec::dispatch(&dispatch_args);
            });
            for (label, nanos) in archspec::profile::snapshot() {
                per_label.entry(label).or_default().push(nanos);
            }
        }
        drop(guard); // writes the hotpath report
    }

    let total_top: u128 = TOP_PHASES
        .iter()
        .filter_map(|label| per_label.get_mut(label).map(median))
        .sum();
    eprintln!("phase-probe: runs={runs} median-of-runs; total(top phases) = {} ms", total_top as f64 / 1e6);
    eprintln!("phase\tmedian_ms\tmean_ms\tshare_pct");
    for (label, values) in &per_label {
        let mut sorted = values.clone();
        let med = median(&mut sorted);
        let mean = values.iter().sum::<u128>() / values.len() as u128;
        let share_of = match *label {
            "scan-facts" => per_label.get("scan").map(Vec::as_slice).map(median_slice),
            "cycle-detect" => per_label.get("compare").map(Vec::as_slice).map(median_slice),
            other => per_label
                .get(other)
                .filter(|_| TOP_PHASES.contains(label))
                .map(|_| total_top),
        };
        let share = share_of.filter(|t| *t > 0).map_or(0.0, |t| 100.0 * med as f64 / t as f64);
        eprintln!(
            "{label}\t{:.1}\t{:.1}\t{share:.1}",
            med as f64 / 1e6,
            mean as f64 / 1e6
        );
    }
    eprintln!(
        "phase-probe: hotpath report at {} — share_pct: top phases over their total; scan-facts over scan; cycle-detect over compare",
        report.display()
    );
    Ok(())
}

fn median_slice(values: &[u128]) -> u128 {
    median(&mut values.to_vec())
}
