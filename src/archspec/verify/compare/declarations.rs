// The declaration-face verify arms (workplan events_value_join scenario 02,
// ADR-069 clause 4.4a(iv)): the canonical-producer declaration table and the
// shared-constant declaration table, adjudicated against the tree. The law
// these arms own:
//
// - one canonical producer per fact class — one class declared with two
//   different producers in one table is refused by name (4.4a(iv)'s
//   arbitration precondition: the declaration is what lets two eligible
//   producers MERGE instead of compete, so a class with two named producers
//   states no arbitration at all);
// - a declaration's producer is a tree-root-canonical unit id — the spelling
//   must answer the tree-root unit index at the ONE resolution site
//   (`events::unitid`, reused, never re-implemented) AND already equal the
//   owning unit's canonical id: a bare per-driver name that no tree-root
//   unit answers, and a spelling that resolves only through another
//   vocabulary, are each refused by name (4.4a(i): the canonical id is the
//   declaration's identity, not a lookup the engine performs on the
//   author's behalf);
// - the shared-constant class carries no pattern pair (F-W3): the
//   emit/subscribe keys never reach this file because the schema face has no
//   such member and the spec walker refuses them at load
//   (`spec::check_constant_pattern_pair`); nothing here can fold a constant
//   into an async row, and no async row is ever computed from these tables —
//   the zero-async-row law is structural, welded by the scenario's cells.
//
// Non-declaring posture (D8): both tables empty returns before any line
// below — every code path a non-declaring run takes stays byte-identical.
//
// Generic by law (engine boundary): a fact class is an OPAQUE name and a
// producer is an opaque tree-root path; no application, framework,
// ecosystem or vendor vocabulary appears in these arms.

use crate::archspec::events::unitid::{UnitIdIndex, canonical_dir};
use crate::archspec::model::Model;
use crate::archspec::spec::{CanonicalProducer, SharedConstant};

use super::diff::Diff;

/// One entry of either declaration table, folded for the shared arms: the
/// owning table's key name, the 1-indexed entry number the markers render,
/// the mandatory class (the walker guarantees it is present and non-empty
/// before deserialization, so the empty string here can only mean a loader
/// that skipped the walker — it rides the unresolved arm, never a silent
/// accept), and the producer's RESOLVED identity: the tree-root unit id the
/// ONE site answers, or the raw spelling where nothing answers (the conflict
/// arm groups by identity, so two spellings of one unit never speak a false
/// conflict, and an unresolved spelling keeps its own name; the refusal arms
/// render the RAW spelling straight into the finding, so it needs no field).
struct Declaration {
    table: &'static str,
    number: usize,
    class: String,
    resolved: String,
}

/// The declaration gate: every arm returns immediately when neither table
/// declares anything (D8), and accumulates every finding, never fail-fast
/// (the ADR-010 posture shared by the whole compare layer).
pub(super) fn check_declarations(
    model: &Model,
    canonical_producer: &[CanonicalProducer],
    shared_constant: &[SharedConstant],
    diff: &mut Diff,
) {
    if canonical_producer.is_empty() && shared_constant.is_empty() {
        return;
    }
    // The ONE canonical unit-id site (scenario 01): the tree's units enter
    // the index by their directory plus every ecosystem spelling they carry;
    // the declaration is then resolved with NO producer directory anchored —
    // a declaration is spec prose, not a row, so it carries no owning
    // directory for the dot's ownership arm to resolve through. A bare `"."`
    // answers only the tree-root unit's own id and nothing else; every other
    // vocabulary answers through the spelling or path arms.
    let mut index = UnitIdIndex::new();
    for unit in &model.units {
        let spellings = std::iter::once(unit.name.clone()).chain(unit.crate_ids.iter().cloned());
        index.register(&unit.path, spellings);
    }
    // Fold both tables into one list, canonical-producer entries first (the
    // tables' declaration order within each table is preserved, so every
    // marker names the entry the author wrote).
    let mut entries: Vec<Declaration> = Vec::new();
    let mut entry =
        |table: &'static str, index_: usize, class: Option<String>, producer: Option<String>| {
            let class = class.unwrap_or_default();
            let raw = producer.unwrap_or_default();
            let location = format!("([{}] entry #{})", table, index_ + 1);
            let resolved = match index.resolve(&raw, "") {
                Err(_) => {
                    // Arm (a): the producer answers the tree-root unit index, or
                    // the run names the unresolved spelling — the fail-closed
                    // posture of the site itself (an untethered name never earns
                    // a guessed id).
                    diff.declaration_producer_unresolved
                        .push(format!("{class} — {raw} {location}"));
                    raw.clone()
                }
                Ok(id) => {
                    if id != canonical_dir(&raw) {
                        // Arm (b): the spelling resolves through another
                        // vocabulary — an ecosystem name or a path inside the
                        // owning unit — so it is not the tree-root-canonical id
                        // the declaration law demands; the refusal names the
                        // spelling and the id it resolves to.
                        diff.declaration_producer_not_canonical
                            .push(format!("{class} — {raw} resolves to {id} {location}"));
                    }
                    id
                }
            };
            entries.push(Declaration {
                table,
                number: index_ + 1,
                class,
                resolved,
            });
        };
    for (index_, item) in canonical_producer.iter().enumerate() {
        entry(
            "canonical_producer",
            index_,
            item.class.clone(),
            item.producer.clone(),
        );
    }
    for (index_, item) in shared_constant.iter().enumerate() {
        entry(
            "shared_constant",
            index_,
            item.class.clone(),
            item.producer.clone(),
        );
    }

    // Arm (c): one producer per class per table — pure spec arithmetic (the
    // ids above come from the same index every arm reads), the collision
    // voice of the async table's counterpart (issue #48 shape): a class
    // carrying two named producers states no arbitration, so the conflict is
    // refused by name, one row per class, grouped by RESOLVED identity —
    // two spellings of one unit are one producer, named once by that id.
    for table in ["canonical_producer", "shared_constant"] {
        let mut classes: std::collections::BTreeMap<
            &str,
            std::collections::BTreeMap<&str, std::collections::BTreeSet<usize>>,
        > = Default::default();
        for declaration in entries.iter().filter(|e| e.table == table) {
            classes
                .entry(&declaration.class)
                .or_default()
                .entry(&declaration.resolved)
                .or_default()
                .insert(declaration.number);
        }
        for (class, producers) in classes {
            if producers.len() < 2 {
                continue;
            }
            let names: Vec<String> = producers
                .iter()
                .map(|(producer, numbers)| {
                    format!(
                        "{producer} (entries {})",
                        numbers
                            .iter()
                            .map(usize::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
                .collect();
            diff.declaration_producer_conflict
                .push(format!("{class} — {} ([{table}])", names.join(", ")));
        }
    }
}
