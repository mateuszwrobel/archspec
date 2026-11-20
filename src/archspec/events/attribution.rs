// The ambiguous-publish attribution census (workplan events_value_join
// scenario 05, ADR-069 clause 4.4a(b) — the ATTRIBUTION-AMBIGUITY CENSUS
// FACE; decision-check F6: the face ships in the one-cycle set because a
// value join without it lands green-but-wrong rows — the M-4 class: an
// assembly-scan registration means no declared producer→subscriber pair
// exists in source, so the visibility derivation lets a hand-declared row
// name the DECLARING unit while the DISPATCH site lives elsewhere, all
// verify arms passing under declaration-trust).
//
// The laws this module owns:
// * The comparison inputs are canonical fact identities (4.4a(i)): the
//   canonicalization rides the ONE site (`events::unitid`), never a second
//   spelling of the resolution law. The declared row's `from` and the
//   dispatch-side fact's `from` meet as tree-root-canonical unit ids.
// * The dispatch-side facts are the derived rows: their `from` is the unit
//   of the site whose declared role orients it as the publisher — the
//   DISPATCHING side by the declaration's own role (4.4a(v)).
// * A declared identity that some dispatch fact carries VERBATIM is
//   attributed to the dispatching side: zero rows (declaring side ==
//   dispatching side, the never-false-positive arm).
// * A declared row whose dispatch facts for the same `(to, event)` pair all
//   name a different `from` prints exactly ONE named census row — the
//   sorted set of the units the dispatch-side facts name. One row per
//   mismatching declared row, deterministic order, duplicates merged.
// * No dispatch fact shares the declared `(to, event)` pair: the face has
//   nothing to census against — zero rows, the shipped bytes stand (a
//   non-declaring tree never reaches this module at all).
// * The face NEVER re-keys the declared row and NEVER reddens a verdict:
//   it makes green-but-wrong VISIBLE (4.4a(b); the roadmap's §2
//   commitment — a silent zero output is forbidden where the mismatch
//   fires). The routing is the caller's: report-only channel, never a
//   finding, never the machine body.
//
// Generic by law (engine boundary): this file carries no application,
// framework, ecosystem or vendor string — "declaring" and "dispatching"
// here name the two sides of the comparison, not a vocabulary the engine
// matches against; the side facts come from the declaration and the model.
// std-only and weldable (the `include!` cell of
// `tests/events_value_join_attribution.rs` compiles this exact text), so
// there is never a second implementation of the comparison law.

use std::collections::{BTreeMap, BTreeSet};

/// One fact identity as the attribution comparison sees it: both endpoints
/// already canonical (4.4a(i) — the caller resolves every spelling through
/// the ONE site before feeding this file), the event is the join-key VALUE
/// (4.4a(ii) — an opaque string here).
pub struct FactRow<'a> {
    pub from: &'a str,
    pub to: &'a str,
    pub event: &'a str,
}

/// The attribution comparison: declared hand rows against the derived
/// dispatch-side facts. One named census line per mismatching declared
/// identity, in `(from, to, event)` order; zero lines when every declared
/// row the dispatch facts speak of names the same `from` the facts name.
pub fn attribution_rows(declared: &[FactRow<'_>], dispatch_facts: &[FactRow<'_>]) -> Vec<String> {
    // Dispatch facts indexed by `(to, event)`: the pair the declared row
    // also claims, with the set of units the facts name as the `from` side.
    let mut facts: BTreeMap<(&str, &str), BTreeSet<&str>> = BTreeMap::new();
    for fact in dispatch_facts {
        facts
            .entry((fact.to, fact.event))
            .or_default()
            .insert(fact.from);
    }
    // Declared identities deduplicated over the `BTreeSet`: one census row
    // per distinct mismatching identity, never one per table duplicate.
    let mut identities: BTreeSet<(&str, &str, &str)> = BTreeSet::new();
    for row in declared {
        identities.insert((row.from, row.to, row.event));
    }
    let mut lines = Vec::new();
    for (from, to, event) in identities {
        let Some(units) = facts.get(&(to, event)) else {
            continue; // no dispatch fact for this pair — nothing to census against
        };
        if units.contains(from) {
            continue; // declaring side == dispatching side — the row is attributed right
        }
        let units = units
            .iter()
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "ambiguous-publish-attribution: row=({from}, {to}, {event}) dispatching {units} — \
             the dispatch-side facts for this value and target name {units}, not the declared \
             from; the declared row stands unchanged"
        ));
    }
    lines
}
