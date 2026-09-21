//! Prose-vs-table guard (capability contract, pass 3): the capability table
//! in the binary is the single source of truth. Every structured emission
//! claim in the printed manual (`[capability <fact> lang="value" ...]`
//! citations) and every capability-named applicability skip in the shared
//! scenario registry must equal the table — read through its own machine
//! projections (`archspec capability matrix` / `granular`), never through a
//! second list. Any contradiction fails the suite; prose gets tightened, the
//! table never loosens.

mod common;
#[allow(dead_code)]
mod shared;

use shared::capability::table;
use shared::scenario::Applies;
use std::collections::BTreeSet;

/// One parsed `[capability ...]` citation: fact + (language, emission) claims.
struct Citation {
    fact: String,
    claims: Vec<(String, String)>,
}

/// Extract every `[capability <fact> lang="value" ...]` citation from `text`.
/// Values are quoted because two table emissions contain separators; the
/// parser is deliberately dumb — fact up to the first whitespace, then
/// `name="value"` pairs until the closing bracket.
fn parse_citations(text: &str) -> Vec<Citation> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("[capability ") {
        let after = &rest[start + "[capability ".len()..];
        let Some(end) = after.find(']') else { break };
        let body = &after[..end];
        let mut words = body.split_whitespace();
        let Some(fact) = words.next() else { break };
        let mut remainder = words.collect::<Vec<_>>().join(" ");
        let mut claims = Vec::new();
        while let Some(eq) = remainder.find('=') {
            let name = remainder[..eq].trim().to_string();
            let Some(value_rest) = remainder[eq + 1..].trim_start().strip_prefix('"') else {
                break;
            };
            let Some(close) = value_rest.find('"') else { break };
            claims.push((name, value_rest[..close].to_string()));
            remainder = value_rest[close + 1..].to_string();
        }
        out.push(Citation {
            fact: fact.to_string(),
            claims,
        });
        rest = &after[end..];
    }
    out
}

/// The printed manual as users see it: every topic that can carry an
/// "emitted by" note.
fn printed_manual() -> String {
    let fixture = common::Fixture::new();
    let mut out = String::new();
    for args in [&[] as &[&str], &["help", "constraints"], &["help", "spec"], &["help", "diagnostics"]] {
        let output = fixture.run(args);
        assert!(
            output.status.success(),
            "help topic {args:?} must print cleanly"
        );
        out.push_str(&String::from_utf8_lossy(&output.stdout));
    }
    out
}

#[test]
fn every_capability_citation_equals_the_table() {
    let citations = parse_citations(&printed_manual());
    assert!(
        !citations.is_empty(),
        "the manual must carry at least one [capability ...] citation — an \
         emitted-by note without a citation cannot be guarded"
    );
    let languages = table().languages();
    for citation in &citations {
        for language in &languages {
            assert!(
                table().emission(language, &citation.fact).is_some(),
                "citation names fact \"{}\", unstated for {language} — prose \
                 cites a removed or renamed fact",
                citation.fact
            );
        }
        let claimed: BTreeSet<&str> = citation.claims.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            claimed,
            languages.iter().map(String::as_str).collect::<BTreeSet<_>>(),
            "citation for \"{}\" must claim every language exactly once, so a \
             silent drop cannot hide drift: {:?}",
            citation.fact,
            citation.claims
        );
        for (language, value) in &citation.claims {
            let expected = table().emission(language, &citation.fact);
            assert_eq!(
                Some(value.as_str()),
                expected,
                "citation for \"{}\" misstates {language} (manual says \
                 {value:?}, table says {expected:?})",
                citation.fact
            );
        }
    }
}

#[test]
fn rule_facts_are_cited_in_the_manual() {
    let citations = parse_citations(&printed_manual());
    for (rule, fact) in table().rules() {
        assert!(
            citations.iter().any(|citation| citation.fact == *fact),
            "rule {rule} requires fact {fact}; the manual must cite that row — \
             an inert-rule fact the prose stopped documenting cannot be guarded"
        );
    }
}

#[test]
fn scenario_capability_skips_follow_the_table() {
    // The matrix's capability skip reasons are citations too: each names a
    // table fact by name, and the run/skip decision must be recomputable from
    // the table's own granular accessor alone.
    let languages = table().languages();
    for scenario in shared::scenarios::all() {
        let Some(applies) = &scenario.applies else {
            continue;
        };
        let is_inert = matches!(applies, Applies::Inert(_));
        let fact = match applies {
            Applies::Capability(fact) | Applies::Inert(fact) => *fact,
            Applies::Custom(_) => continue,
        };
        for language in &languages {
            assert!(
                table().emission(language, fact).is_some(),
                "scenario {} cites fact {fact}, unstated for {language}",
                scenario.name
            );
            let should_run = table().granular(language, fact) != is_inert;
            let driver = shared::driver::Driver::all()
                .into_iter()
                .find(|driver| driver.language.as_str() == *language)
                .unwrap_or_else(|| panic!("no driver harness for {language}"));
            let applies_here = shared::scenarios::applies_here(&driver, applies);
            assert_eq!(
                applies_here, should_run,
                "scenario {} on {language} disagrees with the table's granular \
                 accessor for {fact}",
                scenario.name
            );
        }
    }
}

#[test]
fn granular_accessor_agrees_with_the_matrix_rows() {
    // The two machine projections of the table must agree: every pair the
    // granular accessor accepts prints one and the same emission string, and
    // every pair it rejects prints a different one. The harness's skips lean
    // entirely on this agreement, and no emission vocabulary is hardcoded.
    let mut granular_rows: BTreeSet<&str> = BTreeSet::new();
    let mut other_rows: BTreeSet<&str> = BTreeSet::new();
    for language in table().languages() {
        for fact in table().facts() {
            let row = table()
                .emission(&language, &fact)
                .expect("languages x facts are stated");
            if table().granular(&language, &fact) {
                granular_rows.insert(row);
            } else {
                other_rows.insert(row);
            }
        }
    }
    assert_eq!(
        granular_rows.len(),
        1,
        "the granular accessor must accept rows printing exactly one emission \
         string, saw {granular_rows:?}"
    );
    let granular_value = granular_rows
        .iter()
        .next()
        .expect("table states at least one granular row");
    assert!(
        !other_rows.contains(granular_value),
        "a rejected row prints the granular emission string {granular_value:?}"
    );
}

#[test]
fn matrix_skip_reasons_quote_the_table() {
    // A "capability: <row>" skip reason in the committed report is a citation
    // too: the fact must be in the table and the quoted emission must equal
    // the row for that language, nothing else.
    for json_path in matrix_paths() {
        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(json_path).expect("feature-matrix.json"),
        )
        .expect("matrix report json");
        let languages = json["languages"]
            .as_array()
            .expect("languages array")
            .iter()
            .map(|v| v.as_str().expect("language string").to_string())
            .collect::<Vec<_>>();
        let mut reasons: Vec<String> = Vec::new();
        for behavior in json["features"]
            .as_array()
            .expect("features array")
            .iter()
            .flat_map(|feature| feature["behaviors"].as_array().expect("behaviors").iter())
        {
            let Some(map) = behavior["skip_reasons"].as_object() else {
                continue;
            };
            reasons.extend(map.values().filter_map(|value| value.as_str()).map(str::to_string));
        }
        for reason in reasons {
            let Some((fact, quoted)) = reason.split_once(": ") else {
                continue; // non-capability reasons cite no row
            };
            assert!(
                languages.iter().any(|language| {
                    table().emission(language, fact) == Some(quoted)
                }),
                "skip reason {reason:?} quotes no table row for any language"
            );
            assert!(
                table().facts().contains(fact),
                "skip reason {reason:?} cites fact {fact:?}, absent from the table"
            );
        }
    }
}

fn matrix_paths() -> Vec<std::path::PathBuf> {
    vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("feature-matrix.json")]
}

#[test]
fn table_languages_match_the_matrix_languages() {
    // The matrix report enumerates drivers on disk too; it must name exactly
    // the drivers the capability table states rows for.
    let json_path = matrix_paths()
        .into_iter()
        .next()
        .expect("matrix path");
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(json_path).expect("feature-matrix.json"))
            .expect("matrix report json");
    let languages = json["languages"]
        .as_array()
        .expect("languages array")
        .iter()
        .map(|value| value.as_str().expect("language string"))
        .collect::<BTreeSet<_>>();
    let table_languages = table().languages();
    assert_eq!(
        languages,
        table_languages.iter().map(String::as_str).collect::<BTreeSet<_>>(),
        "matrix languages and the capability table must enumerate the same drivers"
    );
}
