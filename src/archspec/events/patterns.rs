// The generic literal-site matcher verbs and the ONE pattern-resolution
// site (workplan events_value_join scenario 03, ADR-069 clause 4.4a(v),
// decision-check F2/F3/F8).
//
// The law this module owns is the verbs-vs-nouns line (F2 adopted form):
// the engine implements the matcher verbs — call-name+argument-position,
// exported-property-value, and (workplan visibility_phase2 scenario 02)
// field-value — and the repo config supplies the nouns.
// A pattern parameter is data, exactly like the excluded-sources name list
// and the `framework` key the spec already carries. This file therefore
// carries ZERO framework, application, ecosystem or vendor string
// literals: any such literal here is a boundary breach
// (`rust-arch-test-kit/AGENTS.md` §Archspec engine boundary), and the
// vocabulary census in `tests/events_value_join_patterns.rs` keeps that
// falsifiable with a ZERO baseline — any new hit names itself (F2 drafting
// law; ADR-069 4.7(iii) rides the module).
//
// There is NO name-inference arm (F3): the match is a name EQUALITY
// against the declared noun. The engine never decides that a name "sounds
// like" an emit or subscribe site; orientation comes only from the declared
// enumerated role, read downstream by the value join (scenario 04) through
// the row's declared-pattern-id. Renaming a matched call in any source
// changes a row only if the declaration is renamed with it — semantics
// unchanged, verdicts unmoved.
//
// The seam (F8): funded drivers (ts, rust, kotlin) append vocabulary-free
// candidates — syntactic sites carrying no declared name — to the model's
// candidate feed; THIS module resolves candidates against the declared
// sides into the shared `literal_sites` collection. Drivers own NO
// interpretation (A6); the join consumes model facts only — no engine→scan
// import (4.7(i)). Unit identity resolves through the ONE canonical site,
// `super::unitid` (4.4a(i), F-W5): a site's unit is the tree-root-canonical
// id of the unit owning its path, and an unresolvable owner is a named
// `UNITID-UNRESOLVED` line, never a guessed id (fail-closed, the #56
// posture).
//
// The header stays plain `//` comments on purpose: the census cell in
// `tests/events_value_join_patterns.rs` reads this exact file's bytes, and
// the sibling law in `unitid.rs` keeps the module weldable.

use crate::archspec::events::unitid::UnitIdIndex;
use crate::archspec::model::{
    DeclaredPattern, DeclaredPatternVerb, LiteralAnchor, LiteralCandidate, LiteralCandidateAnchor,
    LiteralSite,
};
use std::collections::BTreeMap;

/// The outcome of resolving one candidate set: the matched literal-site
/// rows (unsorted — the caller applies `Model::sort_literal_sites`), and
/// the named `UNITID-UNRESOLVED` refusals — candidates whose owning unit
/// resolved nowhere, whose rows were NOT written (fail-closed).
#[derive(Debug, Default)]
pub struct LiteralSitesOutcome {
    pub sites: Vec<LiteralSite>,
    pub refusals: Vec<String>,
}

/// Matcher verb 1 — call-name + argument position (F2: the engine's verb,
/// the config's noun): true exactly when the candidate is a call site whose
/// callee simple name EQUALS the declared `call` and whose string literal
/// sits at the declared 0-based argument `position`. Pure data equality:
/// the verb reads no semantics from the name (F3 — there is no arm that
/// could), and a non-call candidate never matches.
#[must_use]
pub fn matches_call_site(verb: &DeclaredPatternVerb, anchor: &LiteralCandidateAnchor) -> bool {
    match (verb, anchor) {
        (
            DeclaredPatternVerb::CallNameArgPosition { call, arg },
            LiteralCandidateAnchor::Call { callee, position },
        ) => call == callee && arg == position,
        _ => false,
    }
}

/// Matcher verb 2 — exported-property value (F2): true exactly when the
/// candidate is an exported property whose name EQUALS the declared
/// `property`. A non-property candidate never matches, and no property
/// NAME carries orientation — the role lives on the declaration, not here.
#[must_use]
pub fn matches_exported_property_site(
    verb: &DeclaredPatternVerb,
    anchor: &LiteralCandidateAnchor,
) -> bool {
    match (verb, anchor) {
        (
            DeclaredPatternVerb::ExportedPropertyValue { property },
            LiteralCandidateAnchor::ExportedProperty { property: name },
        ) => property == name,
        _ => false,
    }
}

/// Matcher verb 3 — field value (workplan visibility_phase2 scenario 02,
/// decision-check F2/A1: the engine's verb, the config's noun): true
/// exactly when the candidate is a keyed pair inside an object-like
/// construction whose key EQUALS the declared `field`. Pure data equality
/// through the same channel as verbs 1 and 2 (F3: no name-inference arm —
/// the verb reads no semantics off the field name), and a non-field
/// candidate never matches.
#[must_use]
pub fn matches_field_site(verb: &DeclaredPatternVerb, anchor: &LiteralCandidateAnchor) -> bool {
    match (verb, anchor) {
        (
            DeclaredPatternVerb::FieldValue { field },
            LiteralCandidateAnchor::Field { field: name },
        ) => field == name,
        _ => false,
    }
}

/// Resolve one candidate set against the declared pattern sides: every
/// candidate is tested against every side whose `language` equals the
/// candidate's language, through exactly the matcher verbs above. A matched
/// candidate earns ONE row per matching side (one row per site per matched
/// declaration — two pairs naming one call are two provenances, and their
/// roles settle at the join, never here). The row's `unit` is the
/// tree-root-canonical id of the unit owning the site's path, resolved at
/// the ONE site through `super::unitid` (4.4a(i)); an unresolvable owner
/// contributes its named refusal and no row.
#[must_use]
pub fn resolve_literal_sites(
    index: &UnitIdIndex,
    patterns: &[DeclaredPattern],
    candidates: &[LiteralCandidate],
) -> LiteralSitesOutcome {
    let mut outcome = LiteralSitesOutcome::default();
    for candidate in candidates {
        let matches: Vec<&DeclaredPattern> = patterns
            .iter()
            .filter(|pattern| {
                pattern.language == candidate.language
                    && match verb_for(&candidate.anchor) {
                        Some(matches_candidate) => {
                            matches_candidate(&pattern.verb, &candidate.anchor)
                        }
                        None => false,
                    }
            })
            .collect();
        if matches.is_empty() {
            continue;
        }
        let unit = match index.path_owner(&candidate.path) {
            Ok(unit) => unit,
            Err(unresolved) => {
                outcome.refusals.push(unresolved.to_string());
                continue;
            }
        };
        for pattern in matches {
            outcome.sites.push(LiteralSite {
                unit: unit.clone(),
                path: candidate.path.clone(),
                language: candidate.language.clone(),
                value: candidate.value.clone(),
                anchor: match &candidate.anchor {
                    LiteralCandidateAnchor::Call { .. } => LiteralAnchor::CallNameArgPosition,
                    LiteralCandidateAnchor::ExportedProperty { .. } => {
                        LiteralAnchor::ExportedPropertyValue
                    }
                    LiteralCandidateAnchor::Field { .. } => LiteralAnchor::FieldValue,
                },
                pattern: pattern.id.clone(),
            });
        }
    }
    outcome
}

/// The ONE routing of a candidate shape to its matcher verb: a call
/// candidate reaches verb 1, an exported-property candidate reaches verb
/// 2, a field-value candidate reaches verb 3. `None` states a candidate
/// shape no verb can read — collectable only by a driver bug, and it
/// matches nothing (fail closed, never a guess).
fn verb_for(
    anchor: &LiteralCandidateAnchor,
) -> Option<fn(&DeclaredPatternVerb, &LiteralCandidateAnchor) -> bool> {
    match anchor {
        LiteralCandidateAnchor::Call { .. } => Some(matches_call_site),
        LiteralCandidateAnchor::ExportedProperty { .. } => Some(matches_exported_property_site),
        LiteralCandidateAnchor::Field { .. } => Some(matches_field_site),
    }
}

/// Per-(pattern, language, anchor) match counts of one resolved set — the
/// shape the report surface prints as the literal site census, in
/// deterministic key order. A declared side with zero sites counts zero:
/// the census states the number, it never goes silent on a declaration.
#[must_use]
pub fn literal_site_census(sites: &[LiteralSite]) -> BTreeMap<(String, String, String), usize> {
    let mut census = BTreeMap::new();
    for site in sites {
        *census
            .entry((
                site.pattern.clone(),
                site.language.clone(),
                site.anchor.as_str().to_string(),
            ))
            .or_insert(0usize) += 1;
    }
    census
}
