// The FIRST `Model::async_edges` construction site (workplan
// events_value_join scenario 04 — the hole of issue #48/E6, closed).
//
// WHY HERE, not in `depgraph` (the projection engine the scenario sketch
// first named): the module map of `architecture.spec.toml` keeps the
// acyclic component graph as law — `scan-core` (the extraction glue that
// owns the pattern-resolution seam) may reach `events`, and `engines` may
// reach `scan-core`, but a `scan-core -> engines` import closes the cycle
// the self-verify gate polices (`no_cycles`). The construction therefore
// rides the `events` layer beside the join itself: this file adapts MODEL
// rows to the join's plain inputs, writes the derived rows, and returns
// the named zero-silence census lines; `events::join` keeps the pure
// mechanism (std-only, weldable in `tests/events_value_join_rows.rs`);
// the render seam stays in `depgraph` (consumed by the commands, which
// may lawfully import engines).
//
// Generic by law (engine boundary): no application, framework, ecosystem
// or vendor string reaches this file — the nouns live in the declaration.

use std::collections::BTreeSet;

use crate::archspec::events::join;
use crate::archspec::model::{AsyncEdge, DeclaredPattern, EdgeKind, Model};

/// The construction call: the generic value join over the model's resolved
/// literal sites (model facts in, no scan import) writes the derived async
/// rows into the model and returns the named zero-silence census lines for
/// the report surface. Every endpoint is the tree-root-canonical unit id
/// the site row already carries (4.4a(i) — this site never re-derives an
/// id); every event is the joined string VALUE (4.4a(ii)); orientation
/// came from the declared role only (4.4a(v)). The call site
/// (`extract_with_spec`) runs this ONLY when pattern sides are declared: a
/// non-declaring model keeps an empty collection and serializes no async
/// bytes (4.2 arm 1). Rows order by `(from, to, event)` through the shipped
/// `sort_async_edges` law (E4/N6). The dedupe against the declared
/// `[[async_depends]]` table does NOT happen here: the render set stays the
/// union of both sources at the command seams (a derived row never
/// suppresses a declared arrow), and identity-equal pairs merge through
/// `events::unitid::merge_rows` — one arrow, one `PLUGIN-OVERLAP` row — at
/// the model-build seam, where both spellings are in scope.
pub fn build_derived_async_edges(
    model: &mut Model,
    sides: &[DeclaredPattern],
    constant_classes: &BTreeSet<String>,
    unfunded_languages: &BTreeSet<String>,
) -> Vec<String> {
    // The matched noun per site (F3's mechanical selector). The site rows
    // carry no noun (scenario 03's frozen row face), but each row WAS
    // produced by one (candidate, side) match in `resolve_literal_sites`,
    // and that side declares the noun the matcher equalled. Re-derive the
    // match set HERE from the transient candidate feed (still on the model
    // at this seam): every candidate × side pair that the scenario-03
    // verbs accept. Each site row then consumes ONE still-unconsumed pair
    // of its own pattern over the same (path, language, value, anchor
    // shape) — the greedy bijection that keeps two same-spelling rows from
    // two different nouns (two conventions on one raw string in one file)
    // from claiming the same noun. Orientation rides the matched SIDE,
    // never a language arm (F-W3).
    let mut pairs: Vec<(usize, &DeclaredPattern, bool)> = Vec::new();
    for (ci, candidate) in model.literal_candidates.iter().enumerate() {
        for side in sides
            .iter()
            .filter(|side| side.language == candidate.language)
        {
            let matched = match &candidate.anchor {
                crate::archspec::model::LiteralCandidateAnchor::Call { .. } => {
                    crate::archspec::events::patterns::matches_call_site(
                        &side.verb,
                        &candidate.anchor,
                    )
                }
                crate::archspec::model::LiteralCandidateAnchor::ExportedProperty { .. } => {
                    crate::archspec::events::patterns::matches_exported_property_site(
                        &side.verb,
                        &candidate.anchor,
                    )
                }
                crate::archspec::model::LiteralCandidateAnchor::Field { .. } => {
                    crate::archspec::events::patterns::matches_field_site(
                        &side.verb,
                        &candidate.anchor,
                    )
                }
            };
            if matched {
                pairs.push((ci, side, false));
            }
        }
    }
    let mut nouns: Vec<String> = Vec::with_capacity(model.literal_sites.len());
    for site in &model.literal_sites {
        let hit = pairs.iter().position(|(ci, side, taken)| {
            !*taken
                && side.id == site.pattern
                && model.literal_candidates[*ci].path == site.path
                && model.literal_candidates[*ci].language == site.language
                && model.literal_candidates[*ci].value == site.value
                && matches!(
                    (&model.literal_candidates[*ci].anchor, &site.anchor),
                    (
                        crate::archspec::model::LiteralCandidateAnchor::Call { .. },
                        crate::archspec::model::LiteralAnchor::CallNameArgPosition,
                    ) | (
                        crate::archspec::model::LiteralCandidateAnchor::ExportedProperty { .. },
                        crate::archspec::model::LiteralAnchor::ExportedPropertyValue,
                    ) | (
                        crate::archspec::model::LiteralCandidateAnchor::Field { .. },
                        crate::archspec::model::LiteralAnchor::FieldValue,
                    )
                )
        });
        nouns.push(hit.map_or_else(String::new, |i| {
            pairs[i].2 = true;
            match &pairs[i].1.verb {
                crate::archspec::model::DeclaredPatternVerb::CallNameArgPosition {
                    call,
                    ..
                } => call.clone(),
                crate::archspec::model::DeclaredPatternVerb::ExportedPropertyValue {
                    property,
                } => property.clone(),
                crate::archspec::model::DeclaredPatternVerb::FieldValue { field } => {
                    field.clone()
                }
            }
        }));
    }
    let sites: Vec<join::JoinSite<'_>> = model
        .literal_sites
        .iter()
        .zip(&nouns)
        .map(|(site, noun)| join::JoinSite {
            unit: &site.unit,
            path: &site.path,
            language: &site.language,
            value: &site.value,
            pattern: &site.pattern,
            anchor: site.anchor.as_str(),
            noun,
        })
        .collect();
    let side_refs: Vec<join::JoinSide<'_>> = sides
        .iter()
        .map(|side| join::JoinSide {
            pattern: &side.id,
            language: &side.language,
            noun: match &side.verb {
                crate::archspec::model::DeclaredPatternVerb::CallNameArgPosition {
                    call,
                    ..
                } => call,
                crate::archspec::model::DeclaredPatternVerb::ExportedPropertyValue {
                    property,
                } => property,
                crate::archspec::model::DeclaredPatternVerb::FieldValue { field } => field,
            },
            role: side.role.as_deref(),
        })
        .collect();
    let outcome = join::value_join(&sites, &side_refs, constant_classes, unfunded_languages);
    model.async_edges = outcome
        .rows
        .into_iter()
        .map(|row| AsyncEdge {
            from: row.from,
            to: row.to,
            event: row.event,
            kind: EdgeKind::Async,
        })
        .collect();
    model.sort_async_edges();
    outcome.named
}