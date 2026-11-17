//! The `[[async_depends]]` adjudication (workplan archspec_async_depends,
//! D3/D6/D7): named arms over the declared event-identity table, conditional
//! on a non-empty table — an empty or absent table leaves every code path
//! byte-identical to the pre-table binary (D8). Arm (a) resolves the event
//! type in the model's declared-type census (model-wide, not unit-scoped);
//! arms (b) and (c) read the derived publish/subscribe participation of the
//! from- and to-unit. Where the driver's event-declaration-derivation
//! capability row reads `not-emitted`, arms (b) and (c) skip — the skip
//! prints its capability-named inert note from the command layer
//! (`commands::verify`), never from here. A type declared in a THIRD unit
//! (a shared contracts project) resolves arm (a) model-wide and refuses
//! nothing on its own (M4). The territory check (D7) names an entry endpoint
//! claimed by no unit claim — a CHECK, never a claim: the `[[module]]`
//! `matches` machinery gains no member here. The ring census (D6/M3) runs
//! SCC over the UNION graph (unit-tier direct edges ∪ declared async edges)
//! and names each async-containing ring as ONE advisory warning row — the
//! `no_cycles` adjudication stays DIRECT-ONLY in `cycles.rs`, and these
//! findings ride the warnings channel, never the exempt-matching path
//! (they seed no `[[exempt]]` row and enter no `repro:` line, M6/N1).

use std::collections::{BTreeMap, BTreeSet};
use super::claims::{Claim, async_endpoint_claimed};
use super::diff::Diff;
use crate::archspec::capability;
use crate::archspec::language;
use crate::archspec::model::{EventRecordMember, Model};
use crate::archspec::scan::csharp;
use crate::archspec::spec::AsyncDepend;

/// True when the model states any declared-type census fact at all: either
/// public-fact field (`module_public_types` or `root_public_exports`) has at
/// least one entry. An empty pair makes arm (a) inert — the note states the
/// model's silence, never the driver's.
pub(crate) fn census_present(model: &Model) -> bool {
    !model.module_public_types.is_empty() || !model.root_public_exports.is_empty()
}

/// Resolve one event identity in the model-wide declared-type census: the
/// event's type name (the dotted tail) appears among the census values of
/// either public-fact field. Model-wide by design — a type declared in a
/// shared contracts project is a resolved identity for every unit (M4), so
/// no unit scoping here.
fn event_resolves(model: &Model, event: &str) -> bool {
    // The type-name tail: rust spells identities with `::`, csharp with `.`;
    // either separator's last segment is the declared type's name.
    let name = event.rsplit(['.', ':']).next().unwrap_or(event);
    model
        .module_public_types
        .values()
        .chain(model.root_public_exports.values())
        .any(|types| types.iter().any(|declared| declared == name))
}

/// True when the driver of this model's language derives per-unit event
/// participation (capability row `granular`); every other driver leaves the
/// two derivation arms inert.
fn derives_participation(model: &Model) -> bool {
    language::from_name(&model.language).is_some_and(|language| {
        capability::emits_granular(language, capability::FACT_EVENT_DECLARATION_DERIVATION)
    })
}

/// The declared-shape drift census (workplan archspec_async_depends, D4/M5/
/// M7, corrected by scenario 07): member-diff the publisher's derived record
/// shape against the consumer unit's derived bound shape for each declared
/// entry. Shape rides NO spec byte — both sides come from the scan's
/// `event_shapes` facts, and the census speaks on exactly the shapes that
/// are DERIVED on BOTH sides:
/// - the publisher member set follows the record's BASE chains, model-wide
///   and across every same-simple-name copy (the duplicated-contract
///   idiom): a field declared on the record, on any base it inherits, or
///   on any same-name copy is DECLARED — identical-name, identical-type
///   fields on both sides of the edge print NOTHING (the base-blind class
///   of 13 false "declares no such field" rows and the both-sides-identical
///   false positives are closed by this resolution rule);
/// - a field the consumer binds that NO copy and NO base declares — ONE
///   warning-class row naming consumer, event identity and field (the only
///   reference shape that speaks, D4);
/// - the publisher member is enum-typed and the consumer binds it as a
///   string (`.ToString()` at any hop depth, `== "<literal>"` through a
///   property hop) — its OWN named projection class, a projection, not an
///   equality failure, printed EXACTLY ONCE per qualifying entry;
/// - the duplicated-contract pairing (scenario 10): the subscriber unit
///   declares its OWN namespaced copy of the record, types the bound field
///   as a string, and binds it, while the publisher's census declares an
///   enum member of the same simple name — the SAME projection class, still
///   EXACTLY ONCE per entry, whichever unit hosts the enum, paired through
///   the scenario-06 resolver's shared simple-name implementation (the
///   binding-syntax arms above cannot see this projection: it rides the
///   pair of declarations, not a `ToString` or literal);
/// - legitimate subset consumption (fewer fields bound, all present) and
///   any un-derivable side (not a record, a dynamic/`JsonDocument`/map
///   bind, no typed handler variable at all) print NOTHING — never a
///   guess (M5).
///
/// Rows ride the diff's WARNINGS channel (the transient-warnings
/// precedent), so the run stays green without `--strict` — and stays
/// green UNDER it: the severity law of consumer-fix round 2 (scenario 09,
/// issue #44) retires the M7 promotion reading. `--strict` arms this
/// census's DETECTION depth only and never raises an async census row's
/// severity (see `diff::is_async_census_row`). They never enter the exempt
/// path: the ratchet matches cycle findings and `cycle: ` warnings only,
/// and this arm writes no cycle finding and no `cycle: `-prefixed entry.
/// The `not-emitted` drivers derive no shape at all, so the census is
/// inert there — the capability-named inert note law already states that
/// silence from the command layer.
fn check_shape_drift(model: &Model, async_depends: &[AsyncDepend], diff: &mut Diff) {
    if async_depends.is_empty() || !derives_participation(model) {
        return;
    }
    for (index, entry) in async_depends.iter().enumerate() {
        let to = entry.to.clone().unwrap_or_default();
        let event = entry.event.clone().unwrap_or_default();
        // The type's simple name addresses both derived shapes (the record
        // census is keyed by declaration name, the bound shape by the type
        // name written at the binding) — the shared tail rule of
        // `csharp::event_type_simple_name`, ONE implementation with the
        // arm (c) resolver (scenario 10).
        let name = csharp::event_type_simple_name(&event);
        // Publisher side: every same-simple-name copy in the model-wide
        // record census plus the copies' base chains (scenario 07 — the
        // base-blind cure: a record inheriting a shared base resolves its
        // base-declared members as PRESENT, the same model-wide name law
        // as arm (a)). No record of the name carries a derived shape →
        // silence.
        let Some(members) = publisher_members(model, name) else {
            continue;
        };
        // Consumer side: the unit's bound shape. Absent or empty → the
        // handler binds nothing knowable (dynamic/map bind) or binds
        // nothing at all → subset-of-nothing silence (M5).
        let Some(bound) = model.event_shapes.bounds.get(&to).and_then(|b| b.get(name)) else {
            continue;
        };
        if bound.fields.is_empty() {
            continue;
        }
        let marker = format!("(entry #{})", index + 1);
        // The projection class speaks EXACTLY ONCE per qualifying entry
        // (scenario 07): the first projected, enum-declared binding names
        // the entry; later qualifying fields ride that one row. The class
        // was DEAD on the verdict tree because the member lookup missed the
        // base-declared enums and fell to the absent arm — with the base-
        // following resolution above, a base member bound as a string now
        // reaches THIS arm and names its own class.
        let mut projection_row_printed = false;
        for field in &bound.fields {
            let declared: Vec<&EventRecordMember> = members
                .iter()
                .filter(|member| &member.name == field)
                .collect();
            if declared.is_empty() {
                diff.warnings.push(format!(
                    "async drift field absent: {to} binds {event}.{field} — the publisher \
                     record declares no such field {marker}"
                ));
            } else if bound.projected.contains(field)
                && !projection_row_printed
                && declared.iter().any(|member| declares_enum(model, &member.field_type))
            {
                projection_row_printed = true;
                diff.warnings.push(format!(
                    "async drift enum projection: {to} binds {event}.{field} as a string \
                     projection of the publisher's enum field {marker}"
                ));
            }
        }
        // Projection candidates bound one hop further (scenario 07): such a
        // binding joins `projected` alone, so the absent arm can never fire
        // on it. The class names it only where the publisher's member census
        // declares an enum member of the same name — everywhere else the
        // M5 silence law stands.
        if !projection_row_printed {
            for field in bound.projected.difference(&bound.fields) {
                if members
                    .iter()
                    .any(|member| &member.name == field && declares_enum(model, &member.field_type))
                {
                    diff.warnings.push(format!(
                        "async drift enum projection: {to} binds {event}.{field} as a string \
                         projection of the publisher's enum field {marker}"
                    ));
                    // The `break` is the once-per-entry law for this arm:
                    // later candidates ride this one row. The flag carries
                    // the law across the arms (scenario 10): the pairing
                    // below stays silent once this entry has spoken.
                    projection_row_printed = true;
                    break;
                }
            }
        }
        // The duplicated-contract pairing (scenario 10, the consumer verdict
        // on 0.6.89 — RESIDUAL 2): the subscriber unit declares its OWN
        // namespaced copy of the record and types the field as a STRING while
        // the publisher's census declares an enum member of the same simple
        // name. The projection rides the pair of DECLARATIONS, not the
        // binding syntax, so `projected` carries nothing for it and the two
        // arms above stay silent — the class printed ZERO rows on the
        // verdict tree. The subscriber-side lookup is the census's
        // simple-name cross-unit rule through the scenario-06 resolver's
        // shared implementation (`csharp::unit_module_census_holds` scopes it
        // to the subscriber's own modules); the publisher-side union walk and
        // `declares_enum` keep their model-wide name law unchanged, which is
        // why the enum may sit in the publisher's unit or in a THIRD one.
        // The once-per-entry law and every silence arm stand: a field the
        // subscriber never binds never joins `bound.fields`, legitimate
        // subset consumption declares no enum/string pair, and an
        // un-derivable side prints NOTHING (M5).
        if !projection_row_printed {
            for field in &bound.fields {
                if members
                    .iter()
                    .any(|member| &member.name == field && declares_enum(model, &member.field_type))
                    && subscriber_declares_string(model, &to, name, field)
                {
                    diff.warnings.push(format!(
                        "async drift enum projection: {to} binds {event}.{field} as a string \
                         projection of the publisher's enum field {marker}"
                    ));
                    // The once-per-entry law: the first qualifying pair names
                    // the entry, later ones ride this row — this arm is the
                    // last reader of the flag, so the `break` carries the law.
                    break;
                }
            }
        }
    }
}

/// The publisher-side member set for one event type name (scenario 07):
/// the members of EVERY same-simple-name copy in the declared record
/// census — the duplicated-contract idiom declares one record per unit,
/// and a field declared on both sides of the edge with identical name and
/// type is declared, never absent — plus the members of every record the
/// copies reach through their base lists (transitive, model-wide, the same
/// simple-name law as the census head; a visited set bounds the walk).
/// `None` when no record of the name carries a derived shape — the M5
/// silence law of the census head is unchanged.
fn publisher_members(
    model: &Model,
    name: &str,
) -> Option<Vec<EventRecordMember>> {
    let mut queue = vec![name.to_string()];
    let mut visited: BTreeSet<String> = BTreeSet::new();
    visited.insert(name.to_string());
    let mut found = false;
    let mut members: Vec<EventRecordMember> = Vec::new();
    while let Some(current) = queue.pop() {
        for records in model.event_shapes.records.values() {
            if let Some(copy) = records.get(current.as_str()) {
                found = true;
                members.extend(copy.iter().cloned());
            }
        }
        for bases in model.event_shapes.bases.values() {
            if let Some(list) = bases.get(current.as_str()) {
                for base in list {
                    if visited.insert(base.clone()) {
                        queue.push(base.clone());
                    }
                }
            }
        }
    }
    found.then(|| {
        members.sort();
        members.dedup();
        members
    })
}

/// True when a record member's declared type resolves (name-wide, the same
/// case-sensitive name law as the declared-type census) to a type the model
/// declares as an enum — the projection class's publisher-side condition.
fn declares_enum(model: &Model, written_type: &str) -> bool {
    let plain = written_type.split('<').next().unwrap_or(written_type);
    let name = plain.rsplit(['.', ':']).next().unwrap_or(plain).trim();
    !name.is_empty()
        && model
            .event_shapes
            .enums
            .values()
            .any(|enums| enums.iter().any(|declared| declared == name))
}

/// True when the subscriber unit's OWN record copy — the same-simple-name
/// declaration under one of the unit's modules, the duplicated-contract
/// idiom — declares `field` with a string type: the consumer-side condition
/// of the duplicated-contract projection pairing (scenario 10). The unit
/// scoping is the scenario-06 resolver's shared simple-name implementation
/// (`csharp::unit_module_census_holds`), not a re-mirrored loop; a unit
/// declaring no copy of the name answers false and the M5 silence law
/// stands.
fn subscriber_declares_string(model: &Model, unit: &str, name: &str, field: &str) -> bool {
    csharp::unit_module_census_holds(model, unit, &model.event_shapes.records, |by_name| {
        by_name.get(name).is_some_and(|copy| {
            copy.iter()
                .any(|member| member.name == field && is_string_type(&member.field_type))
        })
    })
}

/// True when a record member's declared type is spelled as C#'s string —
/// `string`, its `System.String` form, either with a nullable `?` mark: the
/// subscriber-side half of the duplicated-contract projection pairing, the
/// spelling twin of `declares_enum`'s name resolution (scenario 10).
fn is_string_type(written_type: &str) -> bool {
    let plain = written_type.split('<').next().unwrap_or(written_type);
    let name = plain
        .trim()
        .trim_end_matches('?')
        .rsplit(['.', ':'])
        .next()
        .unwrap_or(plain)
        .trim();
    name.eq_ignore_ascii_case("string")
}

/// The identity arms, the territory check and the ring census over the
/// declared table, writing findings into the diff fields. Each failing side
/// names itself with its own category (diff.rs), one line per failing side,
/// entries 1-indexed as the spec walker numbers them. Conditional on a
/// non-empty table: an empty table runs NONE of this (D8 — the shipped bytes
/// of a non-declaring repo stand unchanged).
pub(super) fn check_async_depends(
    model: &Model,
    claims: &[Claim<'_>],
    async_depends: &[AsyncDepend],
    diff: &mut Diff,
) {
    if async_depends.is_empty() {
        return;
    }
    let derives = derives_participation(model);
    // The derived drift census rides the same non-empty-table gate as the
    // identity arms (D8: an absent or empty table leaves every path
    // byte-identical); it reads the scan's shape facts, never a spec byte.
    check_shape_drift(model, async_depends, diff);
    for (index, entry) in async_depends.iter().enumerate() {
        let from = entry.from.clone().unwrap_or_default();
        let to = entry.to.clone().unwrap_or_default();
        let event = entry.event.clone().unwrap_or_default();
        let marker = format!("(entry #{})", index + 1);
        // Arm (a): the event type resolves in the declared-type census —
        // wherever the census exists; an empty census keeps the arm inert
        // (the command layer states that silence as a note).
        if census_present(model) && !event_resolves(model, &event) {
            diff.async_event_unresolved
                .push(format!("{event} — {from} -> {to} {marker}"));
        }
        // The territory check (D7): an endpoint claimed by no unit claim is
        // refused by name — a CHECK, never a claim. One line per failing
        // side, the same voice as the identity arms.
        if !async_endpoint_claimed(&from, claims) {
            diff.async_endpoint_unclaimed
                .push(format!("{from} — {from} -> {to} {marker}"));
        }
        if !async_endpoint_claimed(&to, claims) {
            diff.async_endpoint_unclaimed
                .push(format!("{to} — {from} -> {to} {marker}"));
        }
        if derives {
            // Arm (b): the from-unit's publish side keeps the exact-FQN
            // namespace-ownership law byte-for-byte.
            if !csharp::derives_event_participation(model, &from, &event) {
                diff.async_publish_not_derived
                    .push(format!("{from} -> {to} — event {event} {marker}"));
            }
            // Arm (c): the to-unit's subscribe side additionally resolves the
            // event by the census's simple-name rule (scenario 06): a unit
            // owning its own namespaced copy, or carrying a bound shape for
            // the type (interface-impl handler), participates exactly as a
            // shared-contracts unit does.
            if !csharp::derives_subscribe_participation(model, &to, &event) {
                diff.async_subscribe_not_derived
                    .push(format!("{from} -> {to} — event {event} {marker}"));
            }
        }
    }
    // The ring census (D6/M3): SCC over the union graph, async-containing
    // rings named as advisory warnings — green without `--strict` AND green
    // under it (the scenario 09 severity law supersedes the M7 promotion
    // reading: `--strict` arms census DETECTION depth, never this row's
    // severity — see `diff::is_async_census_row`), never a `cycle detected`
    // finding, never an exempt-ratchet or `repro:` input (the direct
    // adjudicator in `cycles.rs` never sees an async edge). One row per
    // reported ring, off the exempt path by construction: this pushes into
    // `warnings`, which exempt matching never reads.
    for (path, mixed) in async_rings(model, async_depends) {
        let kind = if mixed { "mixed" } else { "async" };
        diff.warnings
            .push(format!("async cycle: {} ({kind})", path.join(" -> ")));
    }
}

/// One union adjacency leg: `(to, is_direct)`. A pair carrying both kinds
/// enters as two legs — the census classifies each ring by the kinds its
/// representative path's legs actually carry.
type Leg = (String, bool);

/// The census's ring report: `(path, mixed)` per named ring, `path` rotated
/// to its lexicographically smallest node and closed (the direct
/// adjudicator's rendering law), `mixed` true when at least one leg is a
/// direct edge. Pure-direct cycles are NOT returned — the direct-only
/// `no_cycles` adjudication owns them (D6), and naming them twice would
/// move the shipped cycle bytes (B2/M3).
fn async_rings(model: &Model, async_depends: &[AsyncDepend]) -> Vec<(Vec<String>, bool)> {
    // Direct side: the unit-tier dependency rows of the model — the same
    // pairs the edge adjudication walks, never a re-derivation. Self-pairs
    // are not rings (the direct side drops them at projection entry).
    let direct: BTreeSet<(String, String)> = model
        .edges
        .iter()
        .filter(|edge| edge.from != edge.to)
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect();
    // Async side: one directed leg per declared entry, publisher to
    // subscriber (the ⇢ arrow of the design: the event flows `from` to
    // `to`). A ring of such legs is the pub→sub→pub round trip the consumer
    // pin names (D6) — one entry alone is never a ring (the green
    // shared-contracts shape).
    let async_edges: BTreeSet<(String, String)> = async_depends
        .iter()
        .filter_map(|entry| {
            let (from, to) = (entry.from.clone().unwrap_or_default(), entry.to.clone().unwrap_or_default());
            (from != to).then_some((from, to))
        })
        .collect();
    let mut adjacency: BTreeMap<String, BTreeSet<Leg>> = BTreeMap::new();
    for (from, to) in direct.iter().chain(async_edges.iter()) {
        let is_direct = direct.contains(&(from.clone(), to.clone()));
        adjacency
            .entry(from.clone())
            .or_default()
            .insert((to.clone(), is_direct));
        adjacency.entry(to.clone()).or_default();
    }
    let nodes: Vec<String> = adjacency.keys().cloned().collect();
    let mut reported = Vec::new();
    for scc in strongly_connected(&nodes, &adjacency) {
        // One deterministic representative per SCC (the direct adjudicator's
        // shape): the DFS walks nodes and legs in sorted order, so the path
        // is a function of the graph, never of iteration luck.
        let Some(path) = find_ring(&scc, &adjacency) else {
            continue;
        };
        // Classify per leg. No async leg → the cycle is direct-only: the
        // `no_cycles` adjudicator owns it (D6), the census stays silent.
        let (has_direct, has_async) = leg_kinds(&path, &direct, &async_edges);
        if !has_async {
            continue;
        }
        reported.push((rotate_to_smallest(&path), has_direct));
    }
    reported.sort();
    reported.dedup();
    reported
}

/// The kind split of a cycle path's legs: `path` closes on its first node,
/// a leg is counted for every kind that backs it.
fn leg_kinds(
    path: &[String],
    direct: &BTreeSet<(String, String)>,
    async_edges: &BTreeSet<(String, String)>,
) -> (bool, bool) {
    let mut has_direct = false;
    let mut has_async = false;
    for window in path.windows(2) {
        let leg = (window[0].clone(), window[1].clone());
        has_direct |= direct.contains(&leg);
        has_async |= async_edges.contains(&leg);
    }
    (has_direct, has_async)
}

/// Rotate a closed cycle path so it starts (and re-states) its
/// lexicographically smallest node — the shipped cycle-path rendering law,
/// applied here to the union graph's representatives.
fn rotate_to_smallest(path: &[String]) -> Vec<String> {
    let Some(closing) = path.last().cloned() else {
        return path.to_vec();
    };
    let body = &path[..path.len().saturating_sub(1)];
    let Some(start) = body.iter().position(|node| *node == closing) else {
        return path.to_vec();
    };
    let mut rotated: Vec<String> = body[start..].to_vec();
    rotated.extend(body[..start].iter().cloned());
    rotated.push(closing);
    rotated
}

/// Tarjan SCC over the union adjacency, iteration ascending by node name
/// and by leg target so the SCC list and each member set are deterministic;
/// depth is bounded by the number of units, and both sides drop self-pairs,
/// so no self-leg component can arise here.
fn strongly_connected<'a>(
    nodes: &'a [String],
    adjacency: &'a BTreeMap<String, BTreeSet<Leg>>,
) -> Vec<Vec<String>> {
    let mut state = Tarjan {
        adjacency,
        index: BTreeMap::new(),
        low: BTreeMap::new(),
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        counter: 0,
        sccs: Vec::new(),
    };
    for node in nodes {
        if !state.index.contains_key(node.as_str()) {
            tarjan_visit(node, &mut state);
        }
    }
    state.sccs.sort();
    state.sccs
}

struct Tarjan<'a> {
    adjacency: &'a BTreeMap<String, BTreeSet<Leg>>,
    index: BTreeMap<&'a str, usize>,
    low: BTreeMap<&'a str, usize>,
    stack: Vec<&'a str>,
    on_stack: BTreeSet<&'a str>,
    counter: usize,
    sccs: Vec<Vec<String>>,
}

fn tarjan_visit<'a>(node: &'a str, state: &mut Tarjan<'a>) {
    state.index.insert(node, state.counter);
    state.low.insert(node, state.counter);
    state.counter += 1;
    state.stack.push(node);
    state.on_stack.insert(node);
    if let Some(legs) = state.adjacency.get(node) {
        for (next, _) in legs {
            let next = next.as_str();
            if !state.index.contains_key(next) {
                tarjan_visit(next, state);
                let next_low = state.low[next];
                let here = state.low.get_mut(node).expect("node on low");
                *here = (*here).min(next_low);
            } else if state.on_stack.contains(next) {
                let next_index = state.index[next];
                let here = state.low.get_mut(node).expect("node on low");
                *here = (*here).min(next_index);
            }
        }
    }
    if state.low[node] == state.index[node] {
        let mut members = Vec::new();
        while let Some(popped) = state.stack.pop() {
            state.on_stack.remove(popped);
            members.push(popped.to_string());
            if popped == node {
                break;
            }
        }
        members.sort();
        state.sccs.push(members);
    }
}

/// The deterministic representative cycle of one SCC: DFS from the
/// lexicographically smallest member, walking only legs inside the member
/// set and its legs in sorted order, first closing back-edge wins (the
/// direct adjudicator's find-cycle shape). The returned path closes on its
/// start node.
fn find_ring(scc: &[String], adjacency: &BTreeMap<String, BTreeSet<Leg>>) -> Option<Vec<String>> {
    let members: BTreeSet<&str> = scc.iter().map(String::as_str).collect();
    let start: &str = scc.first()?.as_str();
    let ring = dfs_ring(start, start, adjacency, &members, &mut Vec::new(), &mut BTreeSet::new())?;
    Some(ring.iter().map(|node| (*node).to_string()).collect())
}

fn dfs_ring<'a>(
    node: &'a str,
    start: &'a str,
    adjacency: &'a BTreeMap<String, BTreeSet<Leg>>,
    members: &BTreeSet<&'a str>,
    path: &mut Vec<&'a str>,
    on_path: &mut BTreeSet<&'a str>,
) -> Option<Vec<&'a str>> {
    path.push(node);
    on_path.insert(node);
    let mut found = None;
    if let Some(legs) = adjacency.get(node) {
        for (next, _) in legs {
            let next = next.as_str();
            if !members.contains(next) {
                continue; // legs outside the component close no cycle in it
            }
            if next == start && path.len() > 1 {
                let mut ring = path.clone();
                ring.push(start);
                found = Some(ring);
                break;
            }
            if on_path.contains(next) {
                continue;
            }
            if let Some(ring) = dfs_ring(next, start, adjacency, members, path, on_path) {
                found = Some(ring);
                break;
            }
        }
    }
    if found.is_none() {
        path.pop();
        on_path.remove(node);
    }
    found
}
