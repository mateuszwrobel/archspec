//! Matcher unification differential agreement cell (splits-batch scenario
//! `matcher_unification/01`, decision-check 2026-11-22 F-b3): the diagram
//! engine used to carry a SECOND spec-facing glob matcher — a char-DP body —
//! while the canonical `model::matches_pattern` owns the shipped pattern
//! semantics. The move deleted the DP copy; this cell is the PROOF that the
//! deletion was a pure move: for a corpus of (pattern, text) pairs the
//! legacy DP verdicts equal the canonical verdicts for EVERY pair.
//!
//! Corpus composition (the scenario's done-when 2):
//!   * the fixture grid below — multi-`*`, empty, adjacent `**`, backtracking
//!     shapes, non-ASCII (byte-vs-char arms of the two implementations must
//!     agree because UTF-8 has no cross-boundary matches), long runs;
//!   * an optional corpus arm via the env seams `MATCHER_DIFF_PATTERNS` and
//!     `MATCHER_DIFF_TEXTS` — newline-separated pattern and text files
//!     harvested READ-ONLY from the witness corpora's real spec and ignore
//!     specs by `internal/matcher-unification-evidence.sh` (corpus names live
//!     OUTSIDE the payload, so the seams carry data, never names). Absent
//!     seams print a named SKIP and the grid arm alone decides — the gate
//!     stays hermetic.
//!
//! A disagreement is NOT a failing assertion to fix: it is a SEMANTIC
//! DIVERGENCE — a behavior change, not a move. The cell prints the full
//! disagreement grid before failing, so a STOP verdict can be recorded from
//! the log alone.

/// The DELETED diagram-local matcher, kept as the reference implementation
/// for the differential. Body verbatim from the pre-move tree —
/// `src/archspec/diagram.rs:1410-1432` at trunk `d88b02189`; the move is
/// commit-scenario `matcher_unification/01`. Do not "simplify" this copy:
/// it must stay the byte-shape the diagram painted with until the move.
fn legacy_diagram_glob_match(pattern: &str, text: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let text: Vec<char> = text.chars().collect();
    let mut row = vec![false; text.len() + 1];
    row[0] = true;
    for &p in &pattern {
        let mut next = vec![false; text.len() + 1];
        if p == '*' {
            next[0] = row[0];
            for j in 0..text.len() {
                next[j + 1] = row[j + 1] || next[j];
            }
        } else {
            for j in 0..text.len() {
                if row[j] && p == text[j] {
                    next[j + 1] = true;
                }
            }
        }
        row = next;
    }
    row[text.len()]
}

/// The canonical matcher, resolved through the same public path every
/// spec-facing consumer uses.
fn canonical(pattern: &str, text: &str) -> bool {
    rust_arch_test_kit::model::matches_pattern(text, pattern)
}

/// A long-literal pattern with one embedded star (long-run arm).
const LONG_PAT_CHUNK: &str = "abcdefghij";
fn long_pattern() -> String {
    format!("{}*{}", LONG_PAT_CHUNK.repeat(15), LONG_PAT_CHUNK.repeat(15))
}

/// Grid patterns: empty, star-only, adjacent stars, anchors, sandwiched
/// middles, backtracking shapes (the greedy-backtracker's pressure points),
/// non-ASCII literals, and realistic spec shapes.
fn grid_patterns() -> Vec<String> {
    let mut p: Vec<String> = vec![
        "",
        "*",
        "**",
        "***",
        "a",
        "ab",
        "a*",
        "*a",
        "*a*",
        "**a",
        "a**",
        "a*b",
        "a*b*c",
        "*a*b*",
        "a*a",
        "a*a*a",
        "ab*bc",
        "a.c",
        "*.ts",
        "packages/*",
        "src/**/x",
        "*/*.rs",
        "a*long*tail*end",
        "é",
        "*é*",
        "a*é*b",
        "*.mjs",
        "contract.forbid",
        "**/domain/**",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    p.push(long_pattern());
    p
}

/// Grid texts: empty, exacts, prefixes/suffixes, star-pressure shapes,
/// multi-byte sequences at the backtrack point, separator shapes, long runs.
fn grid_texts() -> Vec<String> {
    let mut t: Vec<String> = vec![
        "",
        "a",
        "ab",
        "abc",
        "aab",
        "abab",
        "abcabc",
        "xaxbx",
        "axbxc",
        "abcx",
        "xabc",
        "packages/cache/index.ts",
        "src/components/x",
        "src/x",
        "a.c",
        "axc",
        "mod.mjs",
        "contract.forbid",
        "domain/x/domain",
        "domain/",
        "é",
        "aéb",
        "ééé",
        "aé",
        "aëb",
        "ab*bc",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();
    t.push(LONG_PAT_CHUNK.repeat(31)); // 310 chars, no star
    t.push(LONG_PAT_CHUNK.repeat(15) + &"*".repeat(3) + &LONG_PAT_CHUNK.repeat(15));
    t
}

fn read_seam(var: &str) -> Option<Vec<String>> {
    let path = std::env::var(var).ok()?;
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("MATCHER_DIFF: seam {var}={path} unreadable: {e}"));
    Some(
        body.lines()
            .map(str::to_string)
            .filter(|line| line.len() <= 512)
            .collect(),
    )
}

fn diff_pairs(patterns: &[String], texts: &[String]) -> Vec<(String, String, bool, bool)> {
    let mut disagreements = Vec::new();
    for pattern in patterns {
        for text in texts {
            let legacy = legacy_diagram_glob_match(pattern, text);
            let canon = canonical(pattern, text);
            if legacy != canon {
                disagreements.push((pattern.clone(), text.clone(), legacy, canon));
            }
        }
    }
    disagreements
}

#[test]
fn matcher_unification_differential_grid() {
    let patterns = grid_patterns();
    let texts = grid_texts();
    let pairs = patterns.len() * texts.len();
    let disagreements = diff_pairs(&patterns, &texts);
    for (pattern, text, legacy, canon) in &disagreements {
        println!(
            "MATCHDIFF|DISAGREE pattern={pattern:?} text={text:?} dp={legacy} canonical={canon}"
        );
    }
    assert!(
        disagreements.is_empty(),
        "MATCHDIFF: {} of {pairs} grid pairs disagree — the move would be a \
         behavior change, not a pure move",
        disagreements.len()
    );
    println!("MATCHDIFF|GREEN arm=grid pairs={pairs} disagreements=0");
}

#[test]
fn matcher_unification_differential_corpus() {
    let (Some(patterns), Some(texts)) = (
        read_seam("MATCHER_DIFF_PATTERNS"),
        read_seam("MATCHER_DIFF_TEXTS"),
    ) else {
        println!("MATCHDIFF|SKIP corpus-arm: seam unset (run internal/matcher-unification-evidence.sh to arm it)");
        return;
    };
    let pairs = patterns.len() * texts.len();
    let disagreements = diff_pairs(&patterns, &texts);
    for (pattern, text, legacy, canon) in &disagreements {
        println!(
            "MATCHDIFF|DISAGREE pattern={pattern:?} text={text:?} dp={legacy} canonical={canon}"
        );
    }
    assert!(
        disagreements.is_empty(),
        "MATCHDIFF: {} of {pairs} corpus pairs disagree — the move would be a \
         behavior change, not a pure move",
        disagreements.len()
    );
    println!(
        "MATCHDIFF|GREEN arm=corpus patterns={} texts={} pairs={pairs} disagreements=0",
        patterns.len(),
        texts.len()
    );
}
