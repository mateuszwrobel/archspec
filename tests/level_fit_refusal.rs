//! Workplan level_views scenario 02: the NAME-THE-FITS refusal on the
//! over-ceiling `archspec diagram --source scan --level <level>` render.
//!
//! A level render whose EMITTED bytes exceed a shipped ceiling emits nothing
//! and refuses at exit 64. The refusal row names the FITS: every fold up to
//! the first OVER one with cross-edges above 0 (BLOCKER-2 — the degenerate
//! fold stays out), each with its self-share percent, measured
//! emit-and-measure on the emitter's own bytes — badge shape included,
//! never a char estimate (MAJOR-7). Fold enumeration is monotone: the walk
//! stops at the first OVER level and never measures a finer fold to prove
//! refusal (MINOR-19). A zero-cross model prints the named empty-fits row
//! `fits: none — no fold fits at these ceilings` instead, never silence
//! (B2/M10). The row names the two-step narrowing
//! `narrow with: archspec scan <bucket-path> --output <artefact>, then
//! archspec diagram --source scan <artefact> [--level <fit>]` — the steps
//! run as printed (MAJOR-20) and every VERB+FLAG pair in them resolves in
//! the named surface's `--help` (§Round-3 MAJOR-20, §Round-4: the `--level`
//! pair is checked against the implemented surface under test, every other
//! token against the resolved binary's help). The ceilings themselves parse
//! from `archspec diagram --help` at run time — never restated; the
//! ceiling-facts cell pins the parse to the shipped arithmetic. The tool
//! never silently auto-picks a coarser level; `archspec scan` is unchanged.

mod common;
#[allow(dead_code)]
mod shared;

use common::{Fixture, stderr, stdout};

/// Run the built binary in the fixture dir and return (code, out, err).
fn run(fixture: &Fixture, args: &[&str]) -> (i32, String, String) {
    let output = fixture.run(args);
    (
        output.status.code().unwrap_or(-1),
        stdout(&output),
        stderr(&output),
    )
}

/// The fits row of a refusal (the clause starting `fits:`, up to the
/// narrowing grammar).
fn fits_row(stderr: &str) -> String {
    let line = stderr
        .lines()
        .find(|line| line.contains("renders nothing; fits:"))
        .unwrap_or_else(|| panic!("the refusal names no fits row:\n{stderr}"));
    let tail = line
        .split("renders nothing; ")
        .nth(1)
        .expect("fits clause after the measured head");
    tail.split("; narrow with:").next().expect("fits clause").to_string()
}

/// The over-ceiling fit fixture: 501 distinct cross-directory pairs. The
/// unit and dir:1 folds aggregate them onto one drawn edge each (FIT with
/// cross-edges above 0), the dir:2 fold draws 501 edge rows — the first
/// OVER fold — and every finer fold goes unmeasured.
fn cross_pairs_artefact() -> Fixture {
    let fixture = Fixture::new();
    let edges: Vec<String> = (0..501)
        .map(|i| {
            format!(
                "{{\"unit\":\"alpha\",\"from\":\"alpha/d{i}/f.rs\",\"to\":\"beta/e{i}.rs\",\"symbols\":[]}}"
            )
        })
        .collect();
    fixture.write(
        "model.json",
        &format!(
            "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{{\"name\":\"alpha\",\"kind\":\"lib\",\"path\":\"alpha\"}},{{\"name\":\"beta\",\"kind\":\"lib\",\"path\":\"beta\"}}],\"edges\":[],\"module_edges\":[{}]}}",
            edges.join(",")
        ),
    )
    // NOTE: the dir spellings must stay consistent for the dir:2 fold to
    // bucket 501 distinct pairs; `to` paths carry one directory segment,
    // so dir:2 on `to` saturates at one segment — the pairs stay distinct
    // through the `from` side.
    ;
    fixture
}

/// Done-when 1 — fit floor. A single-unit fixture whose every coarse fold
/// is all-self stays OUT of the fits row even though those folds sit under
/// both ceilings: only cross-edges above 0 enter. The file fold crosses the
/// edge ceiling, nothing fits, so the named empty-fits row prints instead.
#[test]
fn fit_floor_keeps_the_degenerate_fold_out_of_the_fits_row() {
    let fixture = Fixture::new();
    let edges: Vec<String> = (0..501)
        .map(|i| {
            format!(
                "{{\"unit\":\"alpha\",\"from\":\"a/f{i}.rs\",\"to\":\"a/g{i}.rs\",\"symbols\":[]}}"
            )
        })
        .collect();
    fixture.write(
        "model.json",
        &format!(
            "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{{\"name\":\"alpha\",\"kind\":\"lib\",\"path\":\"a\"}}],\"edges\":[],\"module_edges\":[{}]}}",
            edges.join(",")
        ),
    );
    let (code, out, err) = run(&fixture, &["diagram", "--source", "scan", "model.json", "--level", "file"]);
    assert_eq!(code, 64, "the over-ceiling file fold must refuse — stderr:\n{err}");
    assert!(out.is_empty(), "a refused render emits no body to stdout:\n{out}");
    assert!(
        err.contains("fits: none — no fold fits at these ceilings"),
        "the all-self model prints the named empty-fits row, never silence:\n{err}"
    );
    assert!(
        !err.contains("% self)"),
        "every coarse fold here is all-self (cross-edges 0), so the degenerate \
         fold stays out of the fits row — no entry may print a percent:\n{err}"
    );
    assert!(
        err.contains("narrow with: archspec scan <bucket-path> --output <artefact>"),
        "the empty-fits refusal still names the two-step narrowing:\n{err}"
    );
}

/// Done-when 2 — emit-and-measure. The char-ceiling boundary flips from
/// the EMITTER's own bytes: the badge label of the aggregated edge row
/// ("1 row" vs "1000 rows") moves the verdict while the drawn paths stay
/// byte-identical. The fits-side body lands inside the 49.5–50.5 k
/// boundary band; the over-side refusal measures exactly the badge delta.
#[test]
fn emit_and_measure_char_boundary_flips_from_the_emitter_bytes() {
    // Calibrated: the file-fold body measures 3×PAD_LEN + 107 chars (two
    // fat node rows plus one edge row repeating the path); PAD_LEN 16_630
    // lands the one-raw-row variant at 49,997 — inside the boundary band,
    // under the parsed 50,000-char ceiling. The 1000-raw-row sibling draws
    // the SAME paths; its aggregated row carries the badge label "1000 rows"
    // instead of "1 row" — exactly 4 bytes — landing at 50,001: OVER.
    const PAD_LEN: usize = 16_630;
    let pad = "p".repeat(PAD_LEN);
    let from = format!("{pad}/f.rs");
    let variant = |rows: usize| {
        let fixture = Fixture::new();
        let edges = vec![
            format!(
                "{{\"unit\":\"pad\",\"from\":{:?},\"to\":\"b/f.rs\",\"symbols\":[]}}",
                from
            );
            rows
        ];
        fixture.write(
            "model.json",
            &format!(
                "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{{\"name\":\"pad\",\"kind\":\"lib\",\"path\":{:?}}},{{\"name\":\"b\",\"kind\":\"lib\",\"path\":\"b\"}}],\"edges\":[],\"module_edges\":[{}]}}",
                pad,
                edges.join(",")
            ),
        );
        fixture
    };

    let fits = variant(1);
    let (code, out, err) = run(&fits, &["diagram", "--source", "scan", "model.json", "--level", "file"]);
    assert_eq!(code, 0, "the under-ceiling boundary render emits: {err}");
    let chars = out.chars().count();
    assert!(
        (49_500..=50_000).contains(&chars),
        "the fits-side body must sit inside the 49.5–50.5 k boundary band: {chars}"
    );

    let over = variant(1000);
    let (code, _, err) = run(&over, &["diagram", "--source", "scan", "model.json", "--level", "file"]);
    assert_eq!(
        code, 64,
        "the same paths with a heavier badge label must refuse — the verdict \
         follows the emitter's bytes, not a path estimate"
    );
    let measured: usize = err
        .split("measured ")
        .nth(1)
        .expect("the refusal states the measured counts")
        .split(" characters")
        .next()
        .and_then(|middle| middle.rsplit(' ').next())
        .and_then(|n| n.parse().ok())
        .expect("the measured char count");
    assert!(measured > 50_000, "the over-side measures past the ceiling: {measured}");
    assert_eq!(
        measured,
        chars + 4,
        "the two fixtures draw the SAME paths and the SAME one aggregated \
         edge row; only the badge label differs — \"1000 rows\" over \"1 row\" \
         is exactly 4 bytes, so the emitter's own bytes decide the flip"
    );
    assert_eq!(
        fits_row(&err),
        "fits: unit (0% self), dir:1 (0% self)",
        "the short-bucket folds stay fits while the path-fat file fold goes \
         OVER on its emitted bytes:\n{err}"
    );
}

/// Done-when 3 — stop-at-first-OVER. The dir:2 fold is the first OVER
/// level; its entries and everything finer go unmeasured: the fits row
/// names unit and dir:1 only, and the finer fold's bytes are never emitted.
#[test]
fn folds_stop_at_the_first_over_level() {
    let fixture = cross_pairs_artefact();
    let (code, out, err) = run(&fixture, &["diagram", "--source", "scan", "model.json", "--level", "file"]);
    assert_eq!(code, 64, "the file fold draws 501 edge rows — over the edge ceiling");
    assert!(out.is_empty(), "the refusal emits nothing");
    assert_eq!(
        fits_row(&err),
        "fits: unit (0% self), dir:1 (0% self)",
        "the fits row lists exactly the folds before the first OVER (dir:2) — \
         no finer fold may appear"
    );
    for finer in ["dir:2 (", "module (", "file ("] {
        assert!(
            !fits_row(&err).contains(finer),
            "fold {finer:?} is at or finer than the first OVER level and must \
             not be enumerated as a fit:\n{err}"
        );
    }
    // The fitting folds stand on their own: dir:1 renders, dir:2 refuses
    // with the same fits row — the first OVER level never silently picks a
    // coarser one.
    let (code, _, _) = run(&fixture, &["diagram", "--source", "scan", "model.json", "--level", "dir:1"]);
    assert_eq!(code, 0, "dir:1 fits and renders");
    let (code, _, err) = run(&fixture, &["diagram", "--source", "scan", "model.json", "--level", "dir:2"]);
    assert_eq!(code, 64, "dir:2 is the first OVER fold");
    assert_eq!(
        fits_row(&err),
        "fits: unit (0% self), dir:1 (0% self)",
        "the refusal never auto-picks: it names the fits and stops"
    );
}

/// Done-when 4 — empty fits. A zero-cross model (every raw row intra-bucket
/// at every fold) reaches the char ceiling on its file-fold node rows: the
/// refusal prints the named empty-fits row and zero fit entries.
#[test]
fn zero_cross_model_prints_the_named_empty_fits_row() {
    let fixture = Fixture::new();
    let edges: Vec<String> = (0..1900)
        .map(|i| {
            format!(
                "{{\"unit\":\"src\",\"from\":\"src/s/f{i:04}.rs\",\"to\":\"src/s/f{i:04}.rs\",\"symbols\":[]}}"
            )
        })
        .collect();
    fixture.write(
        "model.json",
        &format!(
            "{{\"schema_version\":1,\"language\":\"rust\",\"units\":[{{\"name\":\"src\",\"kind\":\"lib\",\"path\":\"src\"}}],\"edges\":[],\"module_edges\":[{}]}}",
            edges.join(",")
        ),
    );
    let (code, out, err) = run(&fixture, &["diagram", "--source", "scan", "model.json", "--level", "file"]);
    assert_eq!(
        code, 64,
        "the file fold's node rows cross the char ceiling on a zero-cross model"
    );
    assert!(out.is_empty(), "the refusal emits nothing");
    assert!(
        err.contains("fits: none — no fold fits at these ceilings"),
        "the zero-cross model prints the named empty-fits row:\n{err}"
    );
    assert!(
        !err.contains("% self)"),
        "the empty-fits row carries zero fit entries:\n{err}"
    );
}

/// Done-when 5 — executable-as-printed. On a real scanned tree the
/// refusal's two narrowing steps run as printed once instantiated: the
/// `scan` step obeys the existing gap law (a clean tree exits 0) and the
/// `diagram --source scan` step exits 0. The tree is one crate whose fat
/// `x` modules use the `y` modules — rust scan records module edges for
/// `use crate::` crossings, so the folds ride the derived module edges.
#[test]
fn narrowing_steps_run_as_printed_against_a_scanned_subtree() {
    let fixture = Fixture::new();
    fixture.write("Cargo.toml", "[workspace]\nmembers = [\"auth\"]\n");
    fixture.write(
        "auth/Cargo.toml",
        "[package]\nname = \"auth\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fixture.write("auth/src/lib.rs", "mod x;\nmod y;\n");
    // Fat module keys (~200 chars, under the NAME_MAX file-name cap) make
    // the module fold cross the char ceiling on its node rows; the dir:3
    // fold aggregates to one small row.
    let names: Vec<String> = (1..=125)
        .map(|i| format!("{}{i:03}", "a".repeat(197)))
        .collect();
    let names_b: Vec<String> = (1..=125)
        .map(|i| format!("{}{i:03}", "b".repeat(197)))
        .collect();
    fixture.write(
        "auth/src/x.rs",
        &names.iter().map(|n| format!("pub mod {n};\n")).collect::<String>(),
    );
    fixture.write(
        "auth/src/y.rs",
        &names_b
            .iter()
            .map(|n| format!("pub mod {n};\n"))
            .collect::<String>(),
    );
    for (i, n) in names.iter().enumerate() {
        fixture.write(
            &format!("auth/src/x/{n}.rs"),
            &format!("use crate::y::{};\npub fn f() {{ {}::g(); }}\n", names_b[i], names_b[i]),
        );
    }
    for n in &names_b {
        fixture.write(&format!("auth/src/y/{n}.rs"), "pub fn g() {}\n");
    }

    // Step shape first: the scan step follows the existing gap law — a
    // clean tree (every import resolves inside the crate) exits 0.
    let (code, _, err) = run(&fixture, &["scan", ".", "--output", "artefact.json"]);
    assert_eq!(code, 0, "a clean scan exits 0 under the existing gap law: {err}");

    let (code, _, err) =
        run(&fixture, &["diagram", "--source", "scan", "artefact.json", "--level", "file"]);
    assert_eq!(
        code, 64,
        "the file fold crosses the char ceiling on its fat module keys — stderr:\n{err}"
    );
    let fits = fits_row(&err);
    assert_eq!(
        fits,
        "fits: dir:2 (0% self), module (0% self)",
        "the unit and dir:1 folds fold every module edge to itself (cross-edges \
         0) so they stay out (dir:2 is the first fold where a module edge \
         crosses buckets); the module fold itself lands under the ceilings, so \
         the file fold is the first OVER level: nothing finer is enumerated"
    );
    assert!(
        err.contains("narrow with: archspec scan <bucket-path> --output <artefact>, then archspec diagram --source scan <artefact> [--level <fit>]"),
        "the refusal names the two-step narrowing grammar:\n{err}"
    );

    // Instantiate the printed grammar exactly: <bucket-path> = the scanned
    // subtree, <artefact> = the artefact file, <fit> = the first fit.
    let (code, _, err) = run(&fixture, &["scan", ".", "--output", "narrow.json"]);
    assert_eq!(code, 0, "narrowing step 1 (scan) as printed: {err}");
    let first_fit = fits
        .trim_start_matches("fits: ")
        .split(" (")
        .next()
        .expect("a fit entry")
        .to_string();
    let (code, _, err) = run(
        &fixture,
        &["diagram", "--source", "scan", "narrow.json", "--level", &first_fit],
    );
    assert_eq!(
        code, 0,
        "narrowing step 2 (diagram --source scan --level {first_fit}) as printed: {err}"
    );
}

/// Done-when 6 — help parity. Every VERB+FLAG pair in the refusal's
/// narrowing row resolves in the named surface's `--help`. Per record
/// §Round-4 the `--level` pair is checked against the BUILT surface under
/// test (the flag is new here); every other token resolves in the resolved
/// (installed-when-set) binary's help.
#[test]
fn every_refusal_verb_flag_pair_resolves_in_help() {
    let fixture = cross_pairs_artefact();
    let (code, _, err) = run(&fixture, &["diagram", "--source", "scan", "model.json", "--level", "file"]);
    assert_eq!(code, 64);
    let grammar = err
        .split("narrow with: ")
        .nth(1)
        .expect("the narrowing grammar rides the refusal row")
        .trim()
        .to_string();

    // Pairs as printed: a step starts at the `archspec` verb-name; the next
    // token is the verb; every --flag token until the next `archspec` rides
    // that verb. Brackets are grammar punctuation, not bytes to pass.
    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut verb = String::new();
    let mut after_archspec = false;
    for token in grammar.split_whitespace() {
        if token == "archspec" {
            after_archspec = true;
            continue;
        }
        if after_archspec {
            verb = token.to_string();
            after_archspec = false;
            continue;
        }
        if token.trim_start_matches('[').starts_with("--") {
            let flag: String = token
                .chars()
                .filter(|c| c.is_ascii_lowercase() || *c == '-' || *c == '[' || *c == ']')
                .collect();
            let flag = flag.trim_matches(['-', '[', ']']);
            let flag = format!("--{}", flag.trim_start_matches('-'));
            pairs.push((verb.clone(), flag));
        }
    }
    assert!(
        pairs.contains(&("scan".to_string(), "--output".to_string()))
            && pairs.contains(&("diagram".to_string(), "--source".to_string()))
            && pairs.contains(&("diagram".to_string(), "--level".to_string())),
        "the narrowing grammar must carry the scan --output, diagram --source \
         and diagram --level pairs, got {pairs:?}"
    );

    for (verb, flag) in pairs {
        // §Round-4: the --level pair names the post-scenario diagram
        // surface — check it against the built binary under test; every
        // other pair resolves in the resolved binary's help.
        let binary = if flag == "--level" {
            std::path::PathBuf::from(env!("CARGO_BIN_EXE_archspec"))
        } else {
            shared::matrix::resolve_binary()
        };
        let output = std::process::Command::new(&binary)
            .args([verb.as_str(), "--help"])
            .output()
            .expect("the help surface must be runnable");
        assert_eq!(output.status.code(), Some(0), "{verb} --help must exit 0");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            text.contains(&flag),
            "refusal pair `{verb} {flag}` never resolves in `{verb} --help` of {}:\n{text}",
            binary.display()
        );
    }
}

/// The ceiling-facts pin: the fit machinery parses the two ceilings from
/// the `archspec diagram --help` bytes at run time — this cell pins the
/// parse to the shipped arithmetic, so a help-bytes move reddens here
/// before any fit verdict shifts silently.
#[test]
fn ceiling_facts_parse_from_the_help_bytes() {
    let fixture = Fixture::new();
    let (code, out, err) = run(&fixture, &["diagram", "--help"]);
    assert_eq!(code, 0);
    let text = format!("{out}{err}");
    let anchor = text
        .find("ceilings of ")
        .expect("the help states the ceiling pair")
        + "ceilings of ".len();
    let edges: usize = text[anchor..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .expect("the edge ceiling parses");
    let rest = text[anchor + edges.to_string().len()..]
        .strip_prefix(" edges and ")
        .expect("the edge ceiling is followed by the char ceiling")
        .to_string();
    let chars_at = rest.find(" characters").expect("the char ceiling follows");
    let chars: usize = rest[..chars_at]
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect::<String>()
        .replace(',', "")
        .parse()
        .expect("the char ceiling parses");
    assert_eq!(
        (edges, chars),
        (500, 50_000),
        "the help bytes drifted from the shipped arithmetic"
    );
}
