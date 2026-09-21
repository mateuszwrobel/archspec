//! The shared view of the driver capability table for the test harness:
//! parsed once from `archspec capability matrix` (the table's own machine
//! projection), so every capability skip in the matrix is decided by the
//! table itself — never by a per-language list in test code. The
//! `granular` decision comes from `archspec capability granular`, the same
//! accessor the binary's rules use, keeping even the word "granular" out of
//! the harness.

use crate::common;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::{Mutex, OnceLock};

/// One parsed table: (language, fact) -> emission, plus the (rule, fact)
/// consumers.
pub(crate) struct Table {
    rows: BTreeMap<(String, String), String>,
    rules: Vec<(String, String)>,
    granular: Mutex<Option<HashMap<(String, String), bool>>>,
}

static TABLE: OnceLock<Table> = OnceLock::new();

pub(crate) fn table() -> &'static Table {
    TABLE.get_or_init(|| {
        let fixture = common::Fixture::new();
        let output = fixture.run(&["capability", "matrix"]);
        assert!(
            output.status.success(),
            "`capability matrix` must dump the table: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let mut rows = BTreeMap::new();
        let mut rules = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let mut words = line.split_whitespace();
            match words.next() {
                Some("capability") => {
                    let (Some(lang), Some(fact), Some(emission)) =
                        (words.next(), words.next(), words.next())
                    else {
                        panic!("malformed matrix row: {line}");
                    };
                    let emission = line
                        .splitn(4, ' ')
                        .nth(3)
                        .unwrap_or(emission)
                        .to_string();
                    rows.insert((lang.to_string(), fact.to_string()), emission);
                }
                Some("rule") => {
                    let Some((rule, fact)) = line["rule ".len()..].rsplit_once(' ') else {
                        panic!("malformed rule row: {line}")
                    };
                    rules.push((rule.to_string(), fact.to_string()));
                }
                other => panic!("unknown matrix line: {other:?} in {line:?}"),
            }
        }
        assert!(
            !rows.is_empty(),
            "capability matrix parsed no rows — is the command broken?"
        );
        Table {
            rows,
            rules,
            granular: Mutex::new(None),
        }
    })
}

impl Table {
    pub(crate) fn emission(&self, language: &str, fact: &str) -> Option<&str> {
        self.rows
            .get(&(language.to_string(), fact.to_string()))
            .map(String::as_str)
    }

    pub(crate) fn languages(&self) -> BTreeSet<String> {
        self.rows.keys().map(|(language, _)| language.clone()).collect()
    }

    pub(crate) fn facts(&self) -> BTreeSet<String> {
        self.rows.keys().map(|(_, fact)| fact.clone()).collect()
    }

    pub(crate) fn rules(&self) -> &[(String, String)] {
        &self.rules
    }

    /// True when the binary's own accessor reports the pair as granular.
    /// Cached; the accessor — not this file — defines the word.
    pub(crate) fn granular(&self, language: &str, fact: &str) -> bool {
        let mut cache = self.granular.lock().expect("table cache lock");
        let map = cache.get_or_insert_with(|| {
            let fixture = common::Fixture::new();
            self.rows
                .keys()
                .map(|(lang, name)| {
                    let output =
                        fixture.run(&["capability", "granular", lang, name]);
                    ((lang.clone(), name.clone()), output.status.success())
                })
                .collect()
        });
        let key = (language.to_string(), fact.to_string());
        map.get(&key)
            .copied()
            .unwrap_or_else(|| panic!("unknown capability pair: {language} {fact}"))
    }
}
