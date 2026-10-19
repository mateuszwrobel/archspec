//! C6 capability cells of the ts-driver design record: the matrix answers
//! the new `file-grained-dependency-facts` row (ts granular; rust granular
//! since the file-facts serialization change; go and csharp not-emitted),
//! the ts public-api-surface row is granular (D9), and every table fact is
//! stated for the fourth language. All cells read the binary surface, never
//! a second list.

mod common;

use common::{Fixture, stderr, stdout};

#[test]
fn file_grained_dependency_facts_row_answers_per_the_record() {
    // Record scenario: `capability granular` on the file-grained row —
    // ts granular (the driver's edges are file-grained by construction),
    // rust granular (the driver serializes `file_edges` rows), go and
    // csharp not-emitted.
    let tree = Fixture::new();
    for (language, granular) in [
        ("ts", true),
        ("rust", true),
        ("go", false),
        ("csharp", false),
    ] {
        let out = tree.run(&["capability", "granular", language, "file-grained-dependency-facts"]);
        if granular {
            assert_eq!(
                out.status.code(),
                Some(0),
                "{language} must answer granular: {}",
                stderr(&out)
            );
        } else {
            assert_ne!(
                out.status.code(),
                Some(0),
                "{language} must answer not-granular"
            );
            assert!(
                stderr(&out).contains("not-granular"),
                "stderr: {}",
                stderr(&out)
            );
        }
    }
}

#[test]
fn ts_public_api_row_is_granular() {
    // D9: the ts driver emits real named/default-export public-API facts.
    let tree = Fixture::new();
    let out = tree.run(&["capability", "granular", "ts", "public-api-surface"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "the ts public-api-surface row must read granular: {}",
        stderr(&out)
    );
}

#[test]
fn matrix_states_every_table_fact_for_the_fourth_language() {
    // The table is the single truth; the matrix print must carry a ts row
    // for every fact it carries for any other language.
    let tree = Fixture::new();
    let out = tree.run(&["capability", "matrix"]);
    assert!(out.status.success(), "matrix failed: {}", stderr(&out));
    let body = stdout(&out);
    let mut facts: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for line in body.lines().filter(|line| line.starts_with("capability ")) {
        let mut words = line.split_whitespace();
        let (Some("capability"), Some(language), Some(fact)) =
            (words.next(), words.next(), words.next())
        else {
            continue;
        };
        if language != "ts" {
            facts.insert(fact);
        }
    }
    for fact in facts {
        assert!(
            body.lines()
                .any(|line| line.starts_with(&format!("capability ts {fact} "))),
            "the matrix must state the ts row for fact {fact:?} — a fourth \
             language cannot be silently absent"
        );
    }
    assert!(
        body.contains("capability ts file-grained-dependency-facts granular"),
        "the file-grained row must be ts-granular in the matrix print:\n{body}"
    );
}
