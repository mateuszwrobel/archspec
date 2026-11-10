//! The no_cycles arm: boundary-tier and file-tier cycle detection over
//! the declared groups, SCC computation, and cycle path rendering.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use super::claims::{Claim, claim_unit_fallback, count_claims};
use super::mapping::file_grained_rows;
use super::claims::claim_owner;
use crate::archspec::{capability, language};
use crate::archspec::model::{Model, Unit};
use crate::archspec::spec::{Constraint, Exempt, Module};

/// `no_cycles` check (#9, severity #12-#14) with the D5 scope selector: the
/// The file-tier cycles one adjudication REPORTS (D5/D6): the file-grained
/// adjacency, kind-collapsed (a type-only edge is an edge in the cycle
/// graph; rust `mod` declaration rows are the one exclusion — composition is
/// not a dependency, see `file_grained_rows`), with same-file edges dropped
/// at projection entry (R2-1), yields
/// one deterministic representative path per SCC whose members share the
/// constraint's territory — suppressed when its claim projection is a cycle
/// already reported at the claim tier (R3-4). Returns `(path, members, kind)`
/// per reported cycle; the members are the deduplicated path nodes, the atoms
/// of the D6 attribution, and the kind is the GitHub issue #22 ring label:
/// `"runtime"` when every leg carries a value edge, `"type-assisted"` when
/// some leg is backed only by type-only rows. `rows` receives the induced
/// adjacency the adjudication actually walked — the census's counted rows
/// (the A3 bound of the no-cycles vacuity decision-check: adjudicated
/// model rows, never projected ones). Shared by `check_cycles` (the finding
/// lines) and the seam's cycle-membership query (the diagram's marks) — one
/// resolver, never two (M13).
pub(super) fn file_tier_report_cycles(
    constraint_modules: &[String],
    model: &Model,
    group: &BTreeSet<String>,
    claim_cycle_edges: &BTreeSet<(String, String)>,
    claims: &[Claim<'_>],
    rows: &mut BTreeSet<(String, String)>,
) -> Vec<(Vec<String>, Vec<String>, &'static str)> {
    let adjacency: BTreeSet<(String, String)> = file_grained_rows(model)
        .into_iter()
        .filter(|(from, to)| from != to)
        .filter(|(from, to)| {
            // Rust `mod` declaration rows are composition, never a
            // dependency: the cycle graph excludes them (the diagram keeps
            // rendering them — they ARE the file tree's structure). A pair
            // backed by at least one non-`mod` row stays; a ring whose
            // closing edge is a bare declaration is the parent-owns-child
            // fact the module tier never edges (the tier-comparability law
            // of the D6 projection dedup, ADR-023).
            language::from_name(&model.language) != Some(language::Language::Rust)
                || model.file_edges.iter().any(|edge| {
                    edge.kind != crate::archspec::model::FileFactKind::Mod
                        && edge.from.as_str() == *from
                        && edge.to.as_str() == *to
                })
        })
        .filter(|(from, to)| {
            // Test-gated files never participate, mirroring the module-tier
            // exclusion (`is_test_gated`) and the go `*_test.go` / ts test-
            // file law: the `mod tests` declaration plus the tests' import
            // of its parent is a declaration pair, not a production ring.
            [from, to].iter().all(|file| {
                !model.is_test_gated(&file_module_identity(model, file))
            })        })
        .map(|(from, to)| (from.to_string(), to.to_string()))
        .collect();
    let files: BTreeSet<String> = adjacency
        .iter()
        .flat_map(|(from, to)| [from.clone(), to.clone()])
        .filter(|file| {
            constraint_modules.is_empty()
                || file_claim_index(model, file, claims)
                    .is_some_and(|index| group.contains(claims[index].name))
        })
        .collect();
    let file_induced: BTreeSet<(String, String)> = adjacency
        .iter()
        .filter(|(from, to)| files.contains(from) && files.contains(to))
        .cloned()
        .collect();
    rows.extend(file_induced.iter().cloned());
    let mut reports = Vec::new();
    // Issue #17: one out-edge index for the whole sweep (see adjudicate_cycles).
    let mut file_induced_out: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (from, to) in &file_induced {
        file_induced_out
            .entry(from.as_str())
            .or_default()
            .push(to.as_str());
    }
    for scc in strongly_connected_components(&files, &file_induced) {
        let Some(path) = find_cycle_indexed(&scc, &file_induced_out) else {
            continue;
        };
        if projection_reports_claim_cycle(model, &scc, &file_induced, claims, claim_cycle_edges) {
            continue;
        }
        // Attribution is over the CYCLE's members (path), not the whole
        // SCC: one cycle one finding, one ancestor of ITS members.
        let cycle_members: Vec<String> = {
            let mut seen = BTreeSet::new();
            path.iter()
                .filter(|node| seen.insert((*node).clone()))
                .cloned()
                .collect()
        };
        // Issue #22: the ring's kind from its legs' backing rows — a leg is
        // a value edge iff at least one row backs it that is not type-only;
        // one type-only leg makes the ring type-assisted.
        let kind = if path
            .windows(2)
            .all(|leg| file_leg_is_value(model, &leg[0], &leg[1]))
        {
            CYCLE_KIND_RUNTIME
        } else {
            CYCLE_KIND_TYPE_ASSISTED
        };
        reports.push((path, cycle_members, kind));
    }
    reports
}

/// effective scope is the constraint's declared `scope`, derived from the
/// posture table when absent (D7). The claim tier always adjudicates — for
/// each such constraint, the extracted module-pair edges induced on the
/// constraint's modules (all declared modules if the list is empty) yield
/// every strongly connected component of size greater than one, one
/// representative canonically smallest cycle path per SCC; those bytes are
/// today's behavior at every scope (the scope truth table). At scope `both`
/// the file tier adjudicates too: the model's file-grained adjacency (the
/// `file-grained-dependency-facts` rows, kind-collapsed — the cycle graph
/// counts every edge regardless of kind, D3, except rust `mod` declaration
/// rows, which are composition and never participate (see
/// `file_grained_rows`) —, restricted to the constraint's
/// territory, contributes one finding per file SCC, attributed to the nearest
/// common claim ancestor of ALL its members over the claims union a synthetic
/// tree root (D6): `cycle detected in: <claim>: <file> -> <file>` when the
/// members share a claiming ancestor, `cycle detected in: <file> -> <file>`
/// when they share none. A files-tier SCC is suppressed iff its members'
/// nearest claiming ancestors form a cycle already reported at the claim
/// tier — one violation one finding, coarsest tier first (D6, R3-4). A cycle
/// found by a constraint whose `severity` is `"warning"` goes to the warning
/// bucket (rendered as `<category>: <detail>`); anything else is an error.
/// A spec declaring NO `no_cycles` constraint still gets the model's cycle
/// facts (GitHub issue #6): the same adjudication runs once over all declared
/// modules at the posture-derived scope, forced into the warning bucket —
/// visibility is constraint-independent, the constraint gates severity, not
/// visibility. `verify` without the constraint therefore stays green on
/// cycles (only `--strict` promotes the advisory warnings like any other
/// warning), and `report` never states a lying zero for a cyclic tree.
/// Every finding line carries its issue #22 ring label — `(runtime)` when
/// all legs are value edges, `(type-assisted)` when any leg is type-only —
/// and a `kinds = [...]` list on the constraint narrows which ring classes
/// it reports (absent = all kinds).
///
/// The `[[exempt]]` ratchet (GitHub issue #27, R1/R2): a finding matches an
/// entry by (tier, ring identity), never by count; `count` bounds how many
/// findings one identity may suppress, `until` past makes the entry exempt
/// NOTHING (R4). The result carries the split — see `CycleCheck`.
pub(super) struct CycleCheck {
    /// Violating error-severity findings, rendered bytes unchanged.
    pub errors: Vec<String>,
    /// Violating warning-severity findings plus the stale/expired entry
    /// warnings (R3/R4), each already `<category>: <detail>`.
    pub warnings: Vec<String>,
    /// The census line: `None` when nothing was adjudicated, `Some` whenever
    /// any ring was — exempt rings included (R5: never a false zero).
    pub census: Option<String>,
    /// Findings suppressed by the ledger — counted, never listed (R7).
    pub exempt: usize,
    /// Violating findings as sorted, deduplicated `(tier, identity)` pairs —
    /// the input of the `--write-exemptions` seed writer (R8).
    pub debts: Vec<(String, String)>,
}

/// The tier labels an `[[exempt]]` entry matches against (issue #27 R2): the
/// claim tier's findings are `module`, the file tier's are `file`.
const TIER_MODULE: &str = "module";
const TIER_FILE: &str = "file";

/// The `no_cycles` adjudication entry (the constraint loop plus the shared
/// advisory tier) and the `[[exempt]]` ratchet pass over its findings.
#[allow(clippy::too_many_arguments)]
pub(super) fn check_cycles(
    module_edges: &BTreeSet<(String, String)>,
    value_pairs: &BTreeSet<(String, String)>,
    model: &Model,
    constraints: &[Constraint],
    modules: &[Module],
    claims: &[Claim<'_>],
    exempt: &[Exempt],
) -> CycleCheck {
    let all_modules: BTreeSet<String> = modules.iter().map(|module| module.name.clone()).collect();
    let posture = language::from_name(&model.language).map(capability::posture);
    let file_facts_granular = language::from_name(&model.language).is_some_and(|language| {
        capability::emission(language, capability::FACT_FILE_GRAINED_DEPENDENCY_FACTS)
            == Some(capability::GRANULAR)
    });
    let mut error_cycles: BTreeSet<String> = BTreeSet::new();
    let mut warning_cycles: BTreeSet<String> = BTreeSet::new();
    // issue #27: the (tier, identity) behind each finding string — the
    // exemption matching reads this, the finding lists stay the shipped bytes.
    let mut finding_meta: BTreeMap<String, (&'static str, String)> = BTreeMap::new();
    let mut census = CycleCensus::default();

    let mut declared = false;
    for constraint in constraints
        .iter()
        .filter(|constraint| constraint.kind == "no_cycles")
    {
        declared = true;
        let scope = constraint
            .scope
            .as_deref()
            .or(posture.map(|(scope, _)| scope))
            .unwrap_or(capability::SCOPE_CLAIMS);
        let group: BTreeSet<String> = if constraint.modules.is_empty() {
            all_modules.clone()
        } else {
            constraint
                .modules
                .iter()
                .filter(|name| all_modules.contains(*name))
                .cloned()
                .collect()
        };
        adjudicate_cycles(
            &CycleContext {
                module_edges,
                value_pairs,
                model,
                claims,
                file_facts_granular,
            },
            &CycleRow {
                group: &group,
                territory: &constraint.modules,
                kinds: &constraint.kinds,
                scope,
                severity_is_warning: constraint.severity == "warning",
            },
            &mut error_cycles,
            &mut warning_cycles,
            &mut finding_meta,
            &mut census,
        );
    }

    if !declared {
        // Advisory visibility tier (GitHub issue #6): no `no_cycles` constraint
        // was declared, so the model's cycles — the same set the seeded default
        // guard would adjudicate — are reported with warning severity. The
        // verdict machinery is untouched: warnings never gate without
        // `--strict`, exactly like a `severity = "warning"` constraint.
        let scope = posture
            .map(|(scope, _)| scope)
            .unwrap_or(capability::SCOPE_CLAIMS);
        adjudicate_cycles(
            &CycleContext {
                module_edges,
                value_pairs,
                model,
                claims,
                file_facts_granular,
            },
            &CycleRow {
                group: &all_modules,
                territory: &[],
                kinds: &[],
                scope,
                severity_is_warning: true,
            },
            &mut error_cycles,
            &mut warning_cycles,
            &mut finding_meta,
            &mut census,
        );
    }

    // A cycle covered by both a warning and an error constraint must fail: an
    // error-level finding wins over the warning. Only then sort for the report.
    let error_findings: Vec<String> = error_cycles.iter().cloned().collect();
    let warning_findings: Vec<String> = warning_cycles.difference(&error_cycles).cloned().collect();

    // The `[[exempt]]` ratchet pass (GitHub issue #27). Matching is by
    // (tier, ring identity) — never by count (R1); tier mismatch never
    // suppresses (R2); a past `until` exempts NOTHING (R4); an entry that
    // matched nothing while not expired warns as stale (R3). Suppressed
    // findings leave the finding lists; the census and the metrics keep them
    // counted (R5/R7).
    let today = crate::archspec::spec::today_iso();
    let mut suppressed: BTreeSet<String> = BTreeSet::new();
    let mut exempt_warnings: Vec<String> = Vec::new();
    for entry in exempt {
        let cycle = entry.cycle.as_deref().unwrap_or_default().trim();
        let tier = entry.tier.as_deref().unwrap_or_default();
        if cycle.is_empty() {
            continue; // validate_exempt refused the spec before this point
        }
        let matches: Vec<&String> = error_findings
            .iter()
            .chain(warning_findings.iter())
            .filter(|finding| {
                finding_meta
                    .get(finding.as_str())
                    .is_some_and(|(finding_tier, identity)| {
                        identity == cycle && entry.matches_tier(finding_tier)
                    })
            })
            .collect();
        if entry.expired(&today) {
            // R4: a past `until` exempts NOTHING — the debt stays in the
            // finding lists and the entry names itself expired instead.
            exempt_warnings.push(format!("exempt expired: {tier} {cycle}"));
            continue;
        }
        if matches.is_empty() {
            // R3: a healthy-but-matched-nothing entry warns, never vanishes.
            exempt_warnings.push(format!("exempt stale: {tier} {cycle}"));
            continue;
        }
        // R1: identity matched; `count` bounds how many of the matches this
        // entry may suppress (shrink-only cap; absent = every match).
        let cap = entry.count.unwrap_or(i64::MAX).max(1) as usize;
        for finding in matches.iter().take(cap) {
            suppressed.insert((*finding).clone());
        }
    }

    let mut errors: Vec<String> = error_findings
        .iter()
        .filter(|finding| !suppressed.contains(finding.as_str()))
        .cloned()
        .collect();
    errors.sort();
    let mut warnings: Vec<String> = warning_findings
        .iter()
        .filter(|finding| !suppressed.contains(finding.as_str()))
        .map(|finding| format!("cycle: {finding}"))
        .collect();
    warnings.append(&mut exempt_warnings);
    warnings.sort();

    // Seed-writer input (R8): violating findings as (tier, identity) pairs,
    // sorted and deduplicated — exempt rings are NOT debts.
    let mut debts: BTreeSet<(String, String)> = BTreeSet::new();
    for finding in error_findings
        .iter()
        .chain(warning_findings.iter())
        .filter(|finding| !suppressed.contains(finding.as_str()))
    {
        if let Some((tier, identity)) = finding_meta.get(finding.as_str()) {
            debts.insert(((*tier).to_string(), identity.clone()));
        }
    }

    let found = error_findings.len() + warning_findings.len();
    let exempt_count = suppressed.len();
    // R5: the census states the adjudicated rings whenever any ring was
    // adjudicated — suppressed ones included, never a false zero.
    let census = if found == 0 {
        None
    } else {
        Some(census.render(found, exempt_count, modules))
    };
    CycleCheck {
        errors,
        warnings,
        census,
        exempt: exempt_count,
        debts: debts.into_iter().collect(),
    }
}

/// The read-only context one cycle adjudication needs: the extracted model,
/// its claim views, the module-pair edges, the value-crossing pair set
/// (`BoundaryGraph::value_pairs` — the issue #22 kind fact at the claim
/// tier), and the language-derived facts. Grouping them keeps
/// `adjudicate_cycles` inside the argument budget (the D5 scope and
/// granular-file-facts flags are computed once for the whole
/// `check_cycles` run).
struct CycleContext<'a> {
    module_edges: &'a BTreeSet<(String, String)>,
    value_pairs: &'a BTreeSet<(String, String)>,
    model: &'a Model,
    claims: &'a [Claim<'a>],
    file_facts_granular: bool,
}

/// The two ring classes of the issue #22 kind label: a ring is `runtime`
/// iff every leg carries a value edge; one type-only leg makes the ring
/// `type-assisted`. The same two words are the `kinds = [...]` selector
/// values on a `no_cycles` row.
const CYCLE_KIND_RUNTIME: &str = "runtime";
const CYCLE_KIND_TYPE_ASSISTED: &str = "type-assisted";

/// True iff the `kinds` selector reports a ring of this class — an absent
/// (empty) list is every kind, the shipped behavior (issue #22: backward
/// compatible).
fn kind_selected(kinds: &[String], kind: &str) -> bool {
    kinds.is_empty() || kinds.iter().any(|candidate| candidate == kind)
}

/// The claim-tier ring kind from the value-crossing pair set (D3 law read
/// for the label, issue #22): a leg is a value edge iff the pair carries at
/// least one value crossing — a unit-tier edge or a non-type-only module row.
fn claim_ring_kind(path: &[String], value_pairs: &BTreeSet<(String, String)>) -> &'static str {
    if path
        .windows(2)
        .all(|leg| value_pairs.contains(&(leg[0].clone(), leg[1].clone())))
    {
        CYCLE_KIND_RUNTIME
    } else {
        CYCLE_KIND_TYPE_ASSISTED
    }
}

/// The file-tier leg kind (issue #22): a file pair is a value edge iff at
/// least one row backing it is not type-only. rust file rows are
/// `mod`/`use`/path — rust carries no lexical type/value split, so every
/// rust leg is a value edge; the go and csharp drivers never adjudicate a
/// file tier; the ts/python/kotlin file tier reads `module_edges` rows keyed
/// by file identity, where the serialized `type_only` fact decides.
fn file_leg_is_value(model: &Model, from: &str, to: &str) -> bool {
    match language::from_name(&model.language) {
        Some(language::Language::Ts)
        | Some(language::Language::Python)
        | Some(language::Language::Kotlin) => model.module_edges.iter().any(|edge| {
            edge.from == from && edge.to == to && !edge.type_only
        }),
        _ => true,
    }
}

/// The adjudicated rows one `check_cycles` run counted, for the census line
/// (GitHub issue #18's surviving half). The A3 bound of the no-cycles
/// vacuity decision-check: only rows the adjudication actually walked —
/// the induced adjacencies — never projected or hypothetical edges.
#[derive(Default)]
struct CycleCensus {
    /// File-tier induced pairs the adjudications walked (finest tier).
    file_rows: BTreeSet<(String, String)>,
    /// Claim-tier induced pairs where no file tier adjudicated (the
    /// claims-only posture — the rows there are module-tier).
    claim_rows: BTreeSet<(String, String)>,
    /// True when any adjudication reached the file tier.
    file_tier: bool,
    /// The union of the territories that adjudicated.
    territory: BTreeSet<String>,
}

impl CycleCensus {
    /// The census line: `<N> cycles among <M> file facts; territory: <x>/<y>
    /// modules`. The row count names its tier honestly: `file facts` when a
    /// file tier adjudicated, `module facts` on a claims-only tree. With the
    /// exempt ratchet active (GitHub issue #27, R5) the line splits the
    /// count — `<N> cycles among <M> file facts; exempt: <E>, violating: <V>;
    /// territory: <x>/<y> modules` — the segment absent at zero exempt, so
    /// pre-ratchet bytes stay byte-identical.
    fn render(&self, cycles: usize, exempt: usize, modules: &[Module]) -> String {
        let (rows, noun) = if self.file_tier {
            (self.file_rows.len(), "file fact")
        } else {
            (self.claim_rows.len(), "module fact")
        };
        let split = if exempt > 0 {
            format!("; exempt: {exempt}, violating: {}", cycles - exempt)
        } else {
            String::new()
        };
        format!(
            "{} among {}{split}; territory: {}/{}",
            plural(cycles, "cycle"),
            plural(rows, noun),
            self.territory.len(),
            plural(count_claims(modules), "module"),
        )
    }
}

/// `N cycle` / `N cycles` — the census line's plurals (the report summary's
/// `plural` helper lives behind the verdict surface).
fn plural(count: usize, singular: &str) -> String {
    if count == 1 {
        format!("{count} {singular}")
    } else {
        format!("{count} {singular}s")
    }
}

/// One `no_cycles` adjudication (the constraint loop body and the shared
/// advisory tier — one resolver, never two): the claim tier yields one
/// representative path per SCC of the module edges induced on `group`, and at
/// scope `both` (with granular file facts) the file tier adds its deduped
/// findings; every path goes to the error or the warning bucket per
/// `severity_is_warning`. `territory` is the constraint's `modules` list —
/// empty means all files (`file_tier_report_cycles`); the advisory tier
/// passes an empty territory over all declared modules. `kinds` is the
/// issue #22 ring-class selector: a ring whose class is absent from the
/// (non-empty) list is NOT reported — and, per the R3-7 law, its edges do
/// not enter `claim_cycle_edges` either, so it can never suppress a file
/// finding it never printed. `census` accumulates the adjudicated rows.
/// The per-row adjudication inputs a `no_cycles` constraint (or the advisory
/// tier) contributes: the module group to walk, the constraint's declared
/// territory, the issue #22 `kinds` selector, the tier scope and the severity
/// flip. Bundled per the module's small-argument-list idiom.
#[derive(Clone, Copy)]
struct CycleRow<'a> {
    group: &'a BTreeSet<String>,
    territory: &'a [String],
    kinds: &'a [String],
    scope: &'a str,
    severity_is_warning: bool,
}

fn adjudicate_cycles(
    context: &CycleContext<'_>,
    row: &CycleRow<'_>,
    error_cycles: &mut BTreeSet<String>,
    warning_cycles: &mut BTreeSet<String>,
    finding_meta: &mut BTreeMap<String, (&'static str, String)>,
    census: &mut CycleCensus,
) {
    let CycleRow {
        group,
        territory,
        kinds,
        scope,
        severity_is_warning,
    } = *row;
    census.territory.extend(group.iter().cloned());
    let induced: BTreeSet<(String, String)> = context
        .module_edges
        .iter()
        .filter(|(from, to)| group.contains(from) && group.contains(to))
        .cloned()
        .collect();
    if scope != capability::SCOPE_BOTH {
        census.claim_rows.extend(induced.iter().cloned());
    }
    // The edges of the cycles this adjudication actually REPORTS at the
    // claim tier — the only story the files tier may be deduped against
    // (R3-4/R3-7: an adjacency the claim report never printed cannot
    // stand in for a file cycle, even when it exists).
    let mut claim_cycle_edges: BTreeSet<(String, String)> = BTreeSet::new();
    // Issue #17: one out-edge index for the whole sweep; the per-component
    // walk in `find_cycle` used to rescan `induced` per step.
    let mut induced_out: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (from, to) in &induced {
        induced_out.entry(from.as_str()).or_default().push(to.as_str());
    }

    for scc in strongly_connected_components(group, &induced) {
        if let Some(path) = find_cycle_indexed(&scc, &induced_out) {
            let kind = claim_ring_kind(&path, context.value_pairs);
            if !kind_selected(kinds, kind) {
                continue;
            }
            for pair in path.windows(2) {
                claim_cycle_edges.insert((pair[0].clone(), pair[1].clone()));
            }
            let finding = format!("{} ({kind})", path.join(" -> "));
            finding_meta.insert(
                finding.clone(),
                (TIER_MODULE, path.join(" -> ")),
            );
            if severity_is_warning {
                warning_cycles.insert(finding);
            } else {
                error_cycles.insert(finding);
            }
        }
    }

    if scope != capability::SCOPE_BOTH {
        return;
    }
    // The file tier (D5/D6): the shared extractor `file_tier_report_cycles`
    // — the same cycle set the diagram's files tier marks from (M13).
    if !context.file_facts_granular {
        return;
    }
    census.file_tier = true;
    for (path, members, kind) in file_tier_report_cycles(
        territory,
        context.model,
        group,
        &claim_cycle_edges,
        context.claims,
        &mut census.file_rows,
    ) {
        if !kind_selected(kinds, kind) {
            continue;
        }
        let mut finding = file_cycle_finding(context.model, &members, &path, context.claims);
        finding.push_str(&format!(" ({kind})"));
        finding_meta.insert(finding.clone(), (TIER_FILE, path.join(" -> ")));
        if severity_is_warning {
            warning_cycles.insert(finding);
        } else {
            error_cycles.insert(finding);
        }
    }
}

/// The module-path spelling of a file identity (territory spelling, ts-driver
/// design record): a rust `.rs` file projects onto the module the scanner
/// spells for its backing module — `<unit>::<dir>::<stem>`, a `mod.rs`
/// folding into its directory and a unit root file folding into the unit
/// path (the lexical law `internal/cycle-count-probe.py` measures with; a
/// file attributed by `#[path]` projects lexically like any other). A dotted
/// path or a non-rust file identity is already module-spelled: identity.
/// This is the projection the file tier adjudicates claims against, so a
/// dotted-module claim covers the files backing its module — never a
/// silently-vacuous file tier (the vacuity carrier states facts, not
/// spelling accidents).
pub(crate) fn file_module_identity(model: &Model, path: &str) -> String {
    if language::from_name(&model.language) != Some(language::Language::Rust)
        || path.contains("::")
        || !path.ends_with(".rs")
    {
        return path.to_string();
    }
    // The unit whose `src/` root the file sits under (longest prefix wins).
    // A tie at one prefix length — a single manifest spelling several units
    // (`path = "."`, the bin+lib shape of GitHub issue #29) — is decided by
    // the row facts, never by `model.units` order: the unit a `file_edges`
    // row naming this file carries (the row-own-unit law of the no-cycles
    // vacuity decision-check). A file every tied unit spells in its own
    // rows is the mirrored lib/bin class the probe de-mirrors by file set:
    // either identity states the same file there. A file no tied unit's
    // rows name keeps the first tied unit (the probe's fallback law).
    let mut tied: Vec<(&Unit, usize)> = Vec::new();
    for unit in &model.units {
        let src = if unit.path == "." {
            "src/".to_string()
        } else {
            format!("{}/src/", unit.path.trim_end_matches('/'))
        };
        if !path.starts_with(&src) {
            continue;
        }
        match tied.first() {
            Some(&(_, len)) if src.len() > len => {
                tied.clear();
                tied.push((unit, src.len()));
            }
            Some(&(_, len)) if src.len() == len => tied.push((unit, src.len())),
            Some(_) => {}
            None => tied.push((unit, src.len())),
        }
    }
    if tied.is_empty() {
        // No unit source root matched: project relative to a plain `src/`
        // under the first unit — the probe's fallback law. A path under no
        // source root at all stays identity and claims through nothing else.
        let (unit, rel) = match (model.units.first(), path.strip_prefix("src/")) {
            (Some(unit), Some(rel)) => (unit, rel),
            _ => return path.to_string(),
        };
        return project_rust_stem(unit, rel);
    }
    let (unit, prefix) = if tied.len() == 1 {
        tied[0]
    } else {
        let decided = tied
            .iter()
            .find(|(unit, _)| {
                model
                    .file_edges
                    .iter()
                    .any(|edge| (edge.from == path || edge.to == path) && edge.unit == unit.name)
            })
            .unwrap_or(&tied[0]);
        *decided
    };
    project_rust_stem(unit, &path[prefix..])
}

/// Fold a `src/`-relative rust file into its module path: `mod.rs` folds
/// into its directory, a unit root file (`lib.rs`, `main.rs`, `bin/x.rs`)
/// folds into the unit path itself.
fn project_rust_stem(unit: &Unit, rel: &str) -> String {
    if rel == unit.root {
        return unit.name.clone();
    }
    let stem = rel.strip_suffix(".rs").unwrap_or(rel);
    let mut segs: Vec<&str> = stem.split('/').collect();
    if segs.last().copied() == Some("mod") {
        segs.pop();
    }
    if segs.is_empty() {
        return unit.name.clone();
    }
    let mut dotted = String::from(unit.name.as_str());
    for seg in segs {
        dotted.push_str("::");
        dotted.push_str(seg);
    }
    dotted
}

/// The nearest common ancestor of the cycle members over the claims union a
/// synthetic tree root (D6): the deepest claim on every member's
/// self-to-root parent chain. `None` is the synthetic root — members under
/// disjoint top-level claims, or a member no claim covers — where the emitted
/// line carries no claiming segment and names every member instead.
pub(super) fn common_claim_ancestor<'a>(
    model: &Model,
    members: &[String],
    claims: &[Claim<'a>],
) -> Option<&'a str> {
    let mut chains: Vec<Vec<usize>> = Vec::with_capacity(members.len());
    for member in members {
        let index = file_claim_index(model, member, claims)?;
        let mut chain = Vec::new();
        let mut current = Some(index);
        while let Some(step) = current {
            chain.push(step);
            current = claims[step].parent;
        }
        chains.push(chain);
    }
    chains
        .iter()
        .min_by_key(|chain| chain.len())?
        .iter()
        .find(|index| {
            chains
                .iter()
                .all(|chain| chain.iter().any(|candidate| *candidate == **index))
        })
        .map(|index| claims[*index].name)
}

/// The `::` address of a ts file endpoint: directory separators become
/// segments and the `.ts` suffix is noise. This is the identity the shared
/// scenario dialect (`driver.module_path` on a ts driver) addresses files by,
/// so `common_ancestor`, `rel_after` and the bare-name concession all read
/// ts trees in the same space as rust module paths.
pub(super) fn ts_submodule_identity(path: &str) -> String {
    submodule_identity("ts", path)
}

/// The `::` address of a file-identity endpoint on the ts, python or kotlin
/// driver: directory separators become segments and the source suffix
/// (`.ts`, `.py`, `.kt`) is noise. Rust and go never route here — their
/// module tier is stated in native module paths already.
pub(super) fn submodule_identity(language: &str, path: &str) -> String {
    let stem = match language {
        "python" => path.strip_suffix(".py").unwrap_or(path),
        "kotlin" => path.strip_suffix(".kt").unwrap_or(path),
        _ => path.strip_suffix(".ts").unwrap_or(path),
    };
    stem.replace('/', "::")
}

/// The claim index claiming one file identity — the D6 nearest claiming
/// ancestor over the claims, the same ownership the edge placement applies
/// to module-edge endpoints (`matches.modules` patterns first, the unit-tier
/// fallback beside it). File identities are projected to their module paths
/// first (territory spelling): claims address modules, never raw paths.
/// `None` states the synthetic tree root: no claim covers the file, and the
/// root is where the attribution lands (R2-5).
pub(crate) fn file_claim_index(model: &Model, file: &str, claims: &[Claim<'_>]) -> Option<usize> {
    let identity = file_module_identity(model, file);
    claim_owner(&identity, claims).or_else(|| claim_unit_fallback(&identity, claims))
}

/// The files-tier finding line (D6, pinned by the design record's emitted
/// strings table): with a common claiming ancestor the line carries the
/// claiming segment `cycle detected in: <claim>: <file> -> <file> -> <file>`;
/// attributed to the synthetic root it carries none and names every member.
fn file_cycle_finding(
    model: &Model,
    members: &[String],
    path: &[String],
    claims: &[Claim<'_>],
) -> String {
    match common_claim_ancestor(model, members, claims) {
        Some(claim) => format!("{claim}: {}", path.join(" -> ")),
        None => path.join(" -> "),
    }
}

/// Cross-tier dedup by claim PROJECTION (D6, R3-4/R3-7): the file SCC is
/// suppressed iff every adjacency WITNESSED INSIDE the SCC (both endpoints
/// members) projects onto a claim edge that a cycle REPORTED by this very
/// constraint printed, and the projections form a cycle themselves. A
/// projection onto edges the claim report never showed — an unreported
/// adjacency, a containment hop, an off-territory endpoint — is not the
/// claim tier's story: the file finding prints beside it.
fn projection_reports_claim_cycle(
    model: &Model,
    members: &[String],
    file_edges: &BTreeSet<(String, String)>,
    claims: &[Claim<'_>],
    claim_cycle_edges: &BTreeSet<(String, String)>,
) -> bool {
    let mut ancestors: BTreeSet<String> = BTreeSet::new();
    let mut projected: BTreeSet<(String, String)> = BTreeSet::new();
    for (from, to) in file_edges {
        if !members.contains(from) || !members.contains(to) {
            continue;
        }
        let (Some(from_index), Some(to_index)) = (
            file_claim_index(model, from, claims),
            file_claim_index(model, to, claims),
        ) else {
            continue;
        };
        let (from_claim, to_claim) = (claims[from_index].name, claims[to_index].name);
        if from_claim == to_claim {
            continue;
        }
        if !claim_cycle_edges.contains(&(from_claim.to_string(), to_claim.to_string())) {
            // The claim tier never printed this edge — its report cannot
            // stand in for this file cycle (D6), the finding stands alone.
            return false;
        }
        ancestors.insert(from_claim.to_string());
        ancestors.insert(to_claim.to_string());
        projected.insert((from_claim.to_string(), to_claim.to_string()));
    }
    if ancestors.len() < 2 {
        return false;
    }
    strongly_connected_components(&ancestors, &projected)
        .into_iter()
        .any(|scc| scc.len() > 1)
}

/// Tarjan's strongly connected components over the induced graph, each SCC
/// sorted for deterministic iteration.
pub(super) fn strongly_connected_components(
    group: &BTreeSet<String>,
    edges: &BTreeSet<(String, String)>,
) -> Vec<Vec<String>> {
    struct Tarjan<'a> {
        edges: &'a BTreeSet<(String, String)>,
        group: &'a BTreeSet<String>,
        index: usize,
        indices: BTreeMap<String, usize>,
        lowlink: BTreeMap<String, usize>,
        on_stack: BTreeSet<String>,
        stack: Vec<String>,
        components: Vec<Vec<String>>,
    }

    impl<'a> Tarjan<'a> {
        fn visit(&mut self, node: &str) {
            self.indices.insert(node.to_string(), self.index);
            self.lowlink.insert(node.to_string(), self.index);
            self.index += 1;
            self.stack.push(node.to_string());
            self.on_stack.insert(node.to_string());

            for (from, to) in self.edges {
                if from != node || !self.group.contains(to) {
                    continue;
                }
                if !self.indices.contains_key(to) {
                    self.visit(to);
                    let candidate = self.lowlink[to];
                    let current = self.lowlink[node];
                    self.lowlink
                        .insert(node.to_string(), current.min(candidate));
                } else if self.on_stack.contains(to) {
                    let candidate = self.indices[to];
                    let current = self.lowlink[node];
                    self.lowlink
                        .insert(node.to_string(), current.min(candidate));
                }
            }

            if self.lowlink[node] == self.indices[node] {
                let mut component = Vec::new();
                loop {
                    // The closing node was pushed at entry and is only popped
                    // here, so the stack is non-empty at every pop of this loop
                    // (self-audit F5).
                    let popped = self
                        .stack
                        .pop()
                        .expect("SCC root is still on the Tarjan stack");
                    self.on_stack.remove(&popped);
                    if popped == node {
                        component.push(popped);
                        break;
                    }
                    component.push(popped);
                }
                component.sort();
                self.components.push(component);
            }
        }
    }

    let mut tarjan = Tarjan {
        edges,
        group,
        index: 0,
        indices: BTreeMap::new(),
        lowlink: BTreeMap::new(),
        on_stack: BTreeSet::new(),
        stack: Vec::new(),
        components: Vec::new(),
    };

    for node in group {
        if !tarjan.indices.contains_key(node) {
            tarjan.visit(node);
        }
    }
    tarjan.components
}

/// Deterministic representative cycle for a strongly connected component: start
/// at the lexicographically smallest node and greedily follow the smallest
/// outgoing target until a node on the path repeats; the repeated segment is
/// the cycle. A singleton without a self-loop yields no cycle.
pub(super) fn find_cycle(nodes: &[String], edges: &BTreeSet<(String, String)>) -> Option<Vec<String>> {
    // Issue #17: the walk used to rescan the whole edge set for every path
    // step, making one call O(path x edges). Indexing the out-edges once per
    // call keeps the successor rule identical — the smallest `to` inside
    // `nodes` from the current node — because iterating the set already
    // yields pairs in (from, to) order, so the smallest candidate inside
    // each list is its first member that lies in `nodes`.
    let mut out: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (from, to) in edges {
        out.entry(from.as_str()).or_default().push(to.as_str());
    }
    find_cycle_indexed(nodes, &out)
}

/// The walk of `find_cycle` against a prebuilt out-edge index, so callers
/// that sweep many strongly connected components pay for the index once
/// (issue #17).
fn find_cycle_indexed(nodes: &[String], out: &BTreeMap<&str, Vec<&str>>) -> Option<Vec<String>> {
    let start = nodes.iter().map(String::as_str).next()?;
    let inside: HashSet<&str> = nodes.iter().map(String::as_str).collect();
    let mut path: Vec<&str> = vec![start];
    let mut position: BTreeMap<&str, usize> = BTreeMap::new();
    position.insert(start, 0);
    let mut current = start;

    loop {
        let next = out
            .get(current)
            .and_then(|successors| successors.iter().copied().find(|to| inside.contains(to)));
        let next = next?;
        if let Some(&index) = position.get(next) {
            let mut cycle: Vec<String> = path[index..].iter().map(|node| node.to_string()).collect();
            cycle.push(next.to_string());
            return Some(cycle);
        }
        position.insert(next, path.len());
        path.push(next);
        current = next;
    }
}