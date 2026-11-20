//! The fact-events layer (ADR-069): the machinery every multi-producer fact
//! row shares. The canonical unit-id resolution site lives here and nowhere
//! else — clause 4.4a(i) makes ONE tree-root-canonical id law for every unit
//! id appearing in any multi-producer fact identity, so the resolution rules
//! live in exactly one module, consumed by every fact writer, never
//! re-derived per driver (F-W5: a per-driver name reaching a row fires the
//! 4.4 identity-conflict refusal on every scan — a permanently red tree).
//!
//! The layer is generic by law: the resolution rules speak only of tree-root
//! relative directories and opaque spellings — no application, framework or
//! vendor vocabulary reaches this module (engine boundary, operator law
//! 2026-11-17).

/// The ONE canonical unit-id resolution site: tree-root-relative directory
/// identity for every unit id in any multi-producer fact (ADR-069 4.4a(i)).
/// The resolution law is stated in full at the top of the file itself.
pub mod unitid;

/// The generic literal-site matcher verbs and the ONE pattern-resolution
/// site (workplan events_value_join scenario 03): the engine ships the two
/// verbs — call-name+arg-position and exported-property-value — and the
/// repo config supplies every noun. The module carries ZERO framework
/// string literals (engine-boundary law, ADR-069 4.7(iii)); the vocabulary
/// census cell in `tests/events_value_join_patterns.rs` keeps that
/// falsifiable with a ZERO baseline.
pub mod patterns;

/// The generic cross-unit value join (scenario 04): matched sites meet by
/// string VALUE with the declared role as the sole orientation source and
/// the F1 named-row set. Read-only over the scenario-03 rows.
pub mod join;

/// The FIRST `Model::async_edges` construction site (workplan
/// events_value_join scenario 04, issue #48/E6): the model-row adapter that
/// feeds `join` and writes the derived rows. It rides the events layer, not
/// the depgraph engine, because the module map keeps `scan-core -> engines`
/// a cycle — `scan-core` reaches this site lawfully, `engines` stays the
/// render projection consumed by the commands.
pub mod derive;

/// The ambiguous-publish attribution census (workplan events_value_join
/// scenario 05, ADR-069 4.4a(b)): the comparison of a hand-declared row's
/// canonical `from` against the canonical `from` of the dispatch-side
/// derived facts — one named census line per mismatching declared row,
/// advisory tier, never a verdict, never a re-key (the M-4 class made
/// visible). std-only and weldable, like `join`.
pub mod attribution;
/// The dispatch-blinded-literals projection (workplan events_value_join
/// scenario 06, decision-check F-W4 option a): the report-only census row
/// that names, per zone carrying a shipped `AMBIGUOUS-MARKERS` dispatch
/// row, the declared pattern languages that lost dispatch there. It reads
/// the dispatch row's OWN output bytes plus the literal-site rows — no
/// scanner walk, no driver import, no second dispatch law (the authority
/// decision stays ADR-069 clause 4.8's).
pub mod blindspots;

/// The ONE declaration-gate early exit (workplan events_value_join
/// scenario 08, decision-check F-W1/F6): the whole events face — pattern
/// resolution, the value join, the ambiguous-publish attribution census and
/// the dispatch-blinded projection — is armed ONLY while the `[[pattern_pair]]`
/// table declares at least one side. One check, this function, consulted at
/// exactly one call site (`archspec::scan`, the same seam the drivers'
/// candidate feeds already drain into); a non-declaring run exits here
/// mechanically, so every serialized surface stays byte-identical to the
/// previous release (ADR-069 4.2 arm 1 — the gate is the byte identity, not
/// a per-verb special case). The drivers' per-language candidate-feed checks
/// are the FEED seam of this same table (scenario 03, decision-check F8):
/// an empty table makes them vacuously false, never a second gate arm.
#[must_use]
pub fn declarations_active(patterns: &[crate::archspec::model::DeclaredPattern]) -> bool {
    !patterns.is_empty()
}
