// The generic cross-unit value join (workplan events_value_join scenario 04,
// ADR-069 clause 4.4a(ii)/(iii)/(v); decision-check A1/A4/A7, F1/F3/F-W3):
// matched literal sites meet BY STRING VALUE in ONE key space and become
// async row candidates. This file is the join's only mechanism site; the
// construction site that writes `Model::async_edges` rides `depgraph`, the
// callers pass model facts in (4.7(i): no engine→scan import — this file
// speaks plain rows, std only, so the weld in
// `tests/events_value_join_rows.rs` compiles this exact text).
//
// The laws this module owns:
// * Key = the matched string VALUE, never the symbol, never the declaring
//   name, NEVER the language (F-W3: `language` is an input FIELD, present
//   for the callers' reports, read by no join arm — not even the role
//   lookup: sides meet sites through the MATCHED NOUN, the candidate's own
//   syntactic name, the same equality the scenario-03 matcher applied).
// * One key space; a convention namespace where kinds collide (4.4a(ii)).
//   The engine detects a collision mechanically: one raw value matched by
//   sites of two different declared pattern pairs is two conventions
//   naming one string. Such a group keys per pattern pair (no cross-
//   convention row) and prints ONE `value-namespace` census row. A repo
//   whose conventions carry their own prefixes (`queue:`, `file:`, an FQN)
//   needs no engine arm: the prefix rides the value bytes, the join is
//   byte-blind (4.4a(ii) — joins are by string value).
// * Orientation comes ONLY from the declared enumerated role (4.4a(v),
//   F3): a site is oriented by the role of the DECLARED SIDE that matched
//   it — sides of the site's pattern whose noun equals the site's matched
//   noun. One role among those sides → the orientation; two conflicting
//   roles (one noun declared twice with different roles) or no role
//   (every matching side states none) → UNORIENTED, one named
//   `ambiguous-leg` row. The engine reads no semantics off any name.
// * Zero silence over matched literals (F1): every input site lands in a
//   joined row or exactly ONE named row among {ambiguous-leg, unoriented,
//   one-sided-leg, shared-constant, unfunded-shape, value-namespace
//   (plus its site's own arm)}.
// * A pattern pair whose every side states no role prints one pattern-
//   level `unoriented` row; its sites still carry their per-site
//   `ambiguous-leg` rows.
// * A site whose pattern-pair id names a declared `[[shared_constant]]`
//   class feeds a constant, not an edge (4.4a(iv): the shared-constant
//   class is NEVER an async row): zero rows joined, one `shared-constant`
//   named row per site.
// * A value whose counterpart side exists in the declaration but whose
//   driver emits no literal sites (`unfunded` input, from the capability
//   table read by the caller) prints `unfunded-shape`, never silence and
//   never a guess at the missing half (F1's unfunded arm).
//
// Generic by law (engine boundary): this file carries no application,
// framework, ecosystem or vendor string — the nouns live in the
// declaration, the verbs live here (F2 drafting law; the vocabulary census
// of `src/archspec/events/` keeps its ZERO baseline through this file).

use std::collections::{BTreeMap, BTreeSet};

/// One matched literal site as the join sees it: the model's `LiteralSite`
/// row flattened to plain fields by the construction site (the row types
/// stay in `model.rs`; this seam keeps the join weldable std-only).
/// `unit` is the tree-root-canonical id (4.4a(i)) — the join never
/// re-derives an id. `language` and `path` are FIELDs for the census rows,
/// never join arms (F-W3, F8). `noun` is the candidate's own syntactic
/// name (the callee / property name the scenario-03 matcher equalled) —
/// the SOLE side selector for orientation, a name equality against data,
/// never a semantics read (F3).
pub struct JoinSite<'a> {
    pub unit: &'a str,
    pub path: &'a str,
    pub language: &'a str,
    pub value: &'a str,
    pub pattern: &'a str,
    pub anchor: &'a str,
    pub noun: &'a str,
}

/// One declared pattern side as the join sees it (flattened
/// `DeclaredPattern`): the pair id, the driver language (FIELD, for the
/// unfunded arm's spelling), the declared noun (the matcher's other half)
/// and the declared enumerated role (the SOLE orientation source,
/// 4.4a(v)).
pub struct JoinSide<'a> {
    pub pattern: &'a str,
    pub language: &'a str,
    pub noun: &'a str,
    pub role: Option<&'a str>,
}

/// One joined async row candidate: canonical endpoints over the joined
/// value, with the pattern pair id kept for the census rows. Identity is
/// `(from, to, event)` — `pattern` is census metadata, never an identity
/// field (the frozen three-key law, 4.4a(i)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JoinedRow {
    pub from: String,
    pub to: String,
    pub event: String,
    pub pattern: String,
}

/// The join's whole answer: the joined rows (deduplicated, ordered) and the
/// named zero-silence census lines, in deterministic order.
#[derive(Debug, Default)]
pub struct JoinOutcome {
    pub rows: Vec<JoinedRow>,
    pub named: Vec<String>,
}

/// The join's entry: sites meet sides. `constant_classes` carries the
/// declared `[[shared_constant]]` class names (a pair id naming one feeds
/// constants, never edges); `unfunded` carries the driver languages whose
/// literal-site census reads `not-emitted` (the capability row the caller
/// reads — this module never imports the capability table).
pub fn value_join(
    sites: &[JoinSite<'_>],
    sides: &[JoinSide<'_>],
    constant_classes: &BTreeSet<String>,
    unfunded: &BTreeSet<String>,
) -> JoinOutcome {
    let mut outcome = JoinOutcome::default();

    // Pattern-level orientation: a pair whose every side states no role is
    // the UNORIENTED shape (its sites still answer their per-site arm).
    let mut pair_roles: BTreeMap<&str, Vec<Option<&str>>> = BTreeMap::new();
    for side in sides {
        pair_roles.entry(side.pattern).or_default().push(side.role);
    }
    for (pattern, roles) in &pair_roles {
        if roles.iter().all(|role| role.is_none()) {
            outcome.named.push(format!(
                "unoriented pattern=\"{pattern}\" — no declared side states a role, \
                 sites carry no orientation"
            ));
        }
    }

    // Group by the raw value: ONE key space (4.4a(ii)). `BTreeMap` over the
    // value bytes — no language arm, no symbol arm.
    let mut groups: BTreeMap<&str, Vec<&JoinSite<'_>>> = BTreeMap::new();
    for site in sites {
        groups.entry(site.value).or_default().push(site);
    }

    let mut rows: BTreeSet<JoinedRow> = BTreeSet::new();
    for (value, group) in groups {
        // The shared-constant arm: a pair id naming a declared constant
        // class feeds values, never edges (4.4a(iv)) — named, never joined.
        let (constant, joined_group): (Vec<_>, Vec<_>) = group
            .into_iter()
            .partition(|site| constant_classes.contains(site.pattern));
        for site in &constant {
            outcome.named.push(format!(
                "shared-constant pattern=\"{}\" value=\"{value}\" anchor {} at={} — \
                 declared constant class, never an async row",
                site.pattern, site.anchor, site.path
            ));
        }

        // Kind collisions are detectable as one value claimed by sites of
        // two conventions (pattern pairs): key them per convention and
        // NAME the split (4.4a(ii) — namespace where kinds collide).
        let mut conventions: BTreeSet<&str> = BTreeSet::new();
        for site in &joined_group {
            conventions.insert(site.pattern);
        }
        if conventions.len() > 1 {
            outcome.named.push(format!(
                "value-namespace value=\"{value}\" patterns {} — two declared conventions \
                 match one raw value, the join keys per convention and names both",
                conventions
                    .iter()
                    .map(|pattern| format!("\"{pattern}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Role partition per convention sub-group. A site with no single
        // declared role never joins — one `ambiguous-leg` row names it.
        let mut publishes: BTreeMap<&str, Vec<&JoinSite<'_>>> = BTreeMap::new();
        let mut subscribes: BTreeMap<&str, Vec<&JoinSite<'_>>> = BTreeMap::new();
        for site in &joined_group {
            match site_role(sides, site) {
                Some("publish") => publishes.entry(site.pattern).or_default().push(site),
                Some("subscribe") => subscribes.entry(site.pattern).or_default().push(site),
                _ => outcome.named.push(format!(
                    "ambiguous-leg pattern=\"{}\" value=\"{value}\" anchor {} at={} — \
                     no single declared role for this site, nothing joined",
                    site.pattern, site.anchor, site.path
                )),
            }
        }
        // A convention may carry one role's sites only: the counterpart
        // side is then either absent (one-sided) or unfunded (named arm).
        for (pattern, publish_sites) in &publishes {
            if subscribes.contains_key(*pattern) {
                continue;
            }
            for site in publish_sites {
                if let Some(language) = counterpart_unfunded(sides, pattern, "subscribe", unfunded)
                {
                    outcome.named.push(format!(
                        "unfunded-shape pattern=\"{pattern}\" value=\"{value}\" role subscribe \
                         language=\"{language}\" at={} — the counterpart driver emits no \
                         literal sites, nothing joined",
                        site.path
                    ));
                } else {
                    outcome.named.push(format!(
                        "one-sided-leg pattern=\"{pattern}\" value=\"{value}\" role publish \
                         anchor {} at={} — no subscribe site carries this value, nothing joined",
                        site.anchor, site.path
                    ));
                }
            }
        }
        for (pattern, subscribe_sites) in &subscribes {
            if publishes.contains_key(*pattern) {
                continue;
            }
            for site in subscribe_sites {
                if let Some(language) = counterpart_unfunded(sides, pattern, "publish", unfunded) {
                    outcome.named.push(format!(
                        "unfunded-shape pattern=\"{pattern}\" value=\"{value}\" role publish \
                         language=\"{language}\" at={} — the counterpart driver emits no \
                         literal sites, nothing joined",
                        site.path
                    ));
                } else {
                    outcome.named.push(format!(
                        "one-sided-leg pattern=\"{pattern}\" value=\"{value}\" role subscribe \
                         anchor {} at={} — no publish site carries this value, nothing joined",
                        site.anchor, site.path
                    ));
                }
            }
        }
        // Both roles present: the cross-unit fan (one row per publisher ×
        // subscriber pair; equal pairs dedup through the `BTreeSet`, the
        // dual-const merge of 4.4a(ii) — value-keyed, clean).
        for (pattern, publish_sites) in &publishes {
            let Some(subscribe_sites) = subscribes.get(*pattern) else {
                continue;
            };
            for publish in publish_sites {
                for subscribe in subscribe_sites {
                    rows.insert(JoinedRow {
                        from: publish.unit.to_string(),
                        to: subscribe.unit.to_string(),
                        event: value.to_string(),
                        pattern: pattern.to_string(),
                    });
                }
            }
        }
    }

    outcome.rows = rows.into_iter().collect();
    outcome.rows.sort_by(|a, b| {
        (&a.from, &a.to, &a.event, &a.pattern).cmp(&(&b.from, &b.to, &b.event, &b.pattern))
    });
    outcome.rows.dedup();
    outcome
}

/// The declared role of one site: the roles of the declared sides of the
/// site's pattern whose NOUN equals the site's matched noun — the same
/// name equality the scenario-03 matcher applied (F3: equality against
/// data, no semantics read, no name "sounds like" an arm). One role →
/// the orientation; conflicting roles or none → unoriented (`None`, the
/// ambiguous-leg arm). Language is read by NO arm here (F-W3): the noun
/// equality already selects the side across languages (cell E).
fn site_role<'a>(sides: &[JoinSide<'a>], site: &JoinSite<'_>) -> Option<&'a str> {
    let mut role: Option<&'a str> = None;
    for side in sides {
        if side.pattern != site.pattern || side.noun != site.noun {
            continue;
        }
        let Some(candidate) = side.role else {
            continue; // a role-less matching side states nothing by itself
        };
        if role.is_some_and(|held| held != candidate) {
            return None; // one noun declared under two roles — unoriented
        }
        role = Some(candidate);
    }
    role
}

/// Where the declaration names the counterpart role but the counterpart
/// driver collects no sites, the shape is UNFUNDED — named, never silent
/// (F1's unfunded arm). Returns the counterpart side's language when its
/// driver reads unfunded and the side exists; `None` otherwise (the
/// one-sided-leg arm).
fn counterpart_unfunded(
    sides: &[JoinSide<'_>],
    pattern: &str,
    counterpart_role: &str,
    unfunded: &BTreeSet<String>,
) -> Option<String> {
    sides
        .iter()
        .find(|side| side.pattern == pattern && side.role == Some(counterpart_role))
        .filter(|side| unfunded.contains(&side.language.to_string()))
        .map(|side| side.language.to_string())
}
