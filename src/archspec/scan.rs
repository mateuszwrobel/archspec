//! The binary's CLI glue over the library scan engine (discovery-extraction
//! rung one). The engine walks through `rust_arch_test_kit::scan`; this file
//! owns the root-spec discovery (D14(4)) the engine no longer performs: the
//! verbs keep the shipped behavior — the root `architecture.spec.toml` is
//! read once, its declared entries are handed to the walk as facts, and the
//! report lines print on the report surface exactly as before.

use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::config;
use crate::archspec::events::derive;
use crate::archspec::events::patterns;
use crate::archspec::events::unitid::{ProducerRow, UnitIdIndex};
use crate::archspec::language::Language;
use crate::archspec::model::Model;
use crate::archspec::spec;
use rust_arch_test_kit::scan::{DeclaredFacts, DeclaredPatternVerb, DeclaredZone};
use std::path::Path;

// The engine surface the verbs consume unchanged: the driver-availability
// gate, the report-surface helpers, and the driver modules the `inspect`
// renderers walk directly.
pub use rust_arch_test_kit::scan::{
    csharp, driver_available, driver_unavailable, go, report_only_report, unresolvable_census_line,
    wildcard_report,
};

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[
            FlagSpec {
                name: "output",
                takes_value: true,
            },
            FlagSpec {
                name: "check",
                takes_value: false,
            },
            FlagSpec {
                name: "pretty",
                takes_value: false,
            },
        ],
    )?;
    cli::exactly_one_positional(&parsed)?;

    let root = parsed
        .positionals
        .first()
        .map(String::as_str)
        .unwrap_or(".");
    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("path does not exist: {root}"));
    }
    if !root_path.is_dir() {
        return Err(format!("path is not a directory: {root}"));
    }

    // Root-only spec discovery (D14(4)): read the spec when it sits at the
    // scanned root — an invalid spec fails the run with verify's class; a
    // missing spec keeps scan spec-free. Active exclusions are named on the
    // report surface (stderr) before the work runs — every run, R3-7.
    let loaded = spec::load_scan(root_path)?;
    if let Some(spec) = &loaded {
        eprint!("{}", spec::excluded_sources_report(spec));
        // A dead entry (zero matches under the mechanical set) is stated on
        // the same surface right after the active-entry lines (issue-8
        // phase 2, decision-check MAJOR-2): once per entry per run.
        eprint!("{}", spec::exclusion_liveness_report(root_path, spec));
    }

    // Root-marker selection (detection record GC): with a spec at the root
    // the pinned language SELECTS the driver (its root marker wins, a foreign
    // marker root answers the mismatch branch); without one exactly one root
    // marker selects, several answer the AMBIGUITY error, none keeps the
    // unchanged chain fallback. The nested-marker report line prints BEFORE
    // the agreement precondition so a red run carries it too.
    let pin = match &loaded {
        Some(spec) => Some(crate::archspec::language::from_name(&spec.project.language).ok_or_else(|| {
            format!("invalid spec: unknown language \"{}\"", spec.project.language)
        })?),
        None => None,
    };
    let language = crate::archspec::language::select(root_path, pin)?
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    eprint!("{}", crate::archspec::language::marker_report(root_path, language));

    // The agreement check is a precondition (D14): with a spec at the root,
    // the declared language is the truth and any un-excluded foreign source
    // fails the run BEFORE any artifact or model body. Without a spec, chain
    // detection stays the whole truth — nothing is checked (D14(2)).
    if let (Some(spec), Some(declared)) = (loaded.as_ref(), pin)
        && let Some(finding) = crate::archspec::language::agreement_finding(
            root_path,
            declared,
            &spec::exclusion_paths(spec),
        )
    {
        return Err(finding);
    }

    if !driver_available(language) {
        return Err(driver_unavailable(language));
    }

    let model = extract_with_spec(language, root_path, loaded.as_ref())?;
    // The wildcard facts are report-only (R2-6): one stderr line per fact,
    // never part of the machine body, never a verdict.
    eprint!("{}", wildcard_report(&model));
    // The `external not installed` lines are report-only the same way
    // (ts-resolution record R7b).
    eprint!("{}", report_only_report(&model));
    // The warn-mode exclusion findings (issue-8 phase 2, BLOCKER-2): one
    // `warning:` line per finding on the report surface — stated, never a
    // verdict, never the machine body (verify fails the same finding under
    // `--strict`; scan carries no verdict).
    for warning in &model.warnings {
        eprintln!("warning: {warning}");
    }
    // The unresolvable-import census (visibility phase 2, scenario 01):
    // the anchored miss lines print byte-identically to the pre-fix
    // finding bytes, then the ONE census line aggregates them. The model
    // landed anyway — a dead edge states itself here, never in the
    // machine body. An empty census prints zero bytes (shipped-bytes
    // guard); the verdict (exit 1 iff non-empty) rides the emit tail.
    for entry in &model.unresolvable_imports {
        eprintln!("{}", entry.line);
    }
    if let Some(census) = unresolvable_census_line(&model.unresolvable_imports) {
        eprintln!("{census}");
    }
    // The literal-site census (workplan events_value_join scenario 03):
    // one line per declared pattern side on the report surface, never the
    // machine body — a declared side with zero sites states ZERO, the
    // never-silent law; a side whose driver collects no literal candidates
    // (the capability's `not-emitted` reading — go and python today)
    // states the named `driver-gap` absence row instead. Both are
    // report-only census lines: they state counts, never verdicts, never
    // red. A run without declared pattern pairs prints nothing here — the
    // declaration gate keeps the report surface byte-identical (4.2 arm 1).
    if let Some(loaded) = loaded.as_ref() {
        eprint!("{}", literal_site_report(loaded, &model));
    }
    // `--pretty` is a serialization switch only: same facts, indented layout
    // for readers and diffs; the default bytes are untouched (blind4 D03).
    let json = if parsed.switches.contains("pretty") {
        format!("{}\n", model.to_json_pretty()?)
    } else {
        format!("{}\n", model.to_json()?)
    };

    let config = config::load(root_path)?;
    // The model JSON is a machine body: a body on stdout carries no status
    // line (wave-3 D02 purity); a destination write echoes the universal
    // `wrote <path>` line like every path write (blind4 D01).
    let route = config::route(
        parsed.values.get("output").map(String::as_str),
        config.output.scan.as_deref(),
        root_path,
        false,
    );
    let check = parsed.switches.contains("check");
    config::emit(&json, route, check, "model", "scan", None)?;
    // A non-empty census forces exit 1 EVEN WHEN the bytes are fresh
    // (scenario 01 exit law): the model landed with dead edges, and a
    // green `--check` must not hide that. The sentinel rides the shipped
    // "verdict already stated" arm (`report` set the precedent) — the
    // census line printed above IS the statement, so dispatch echoes no
    // second error. An empty census reaches neither arm: every byte and
    // the exit code stay exactly as they were (shipped-bytes guard, C2).
    if model.unresolvable_imports.is_empty() {
        Ok(())
    } else {
        Err(RULE_VIOLATION_REPORTED.to_string())
    }
}

/// The sentinel naming "a verdict already stated itself": dispatch exits 1
/// without echoing the message again (the verify/report arm). The census
/// exit rides it, so the line lives HERE — `scan-core` claims this module
/// and never depends on the commands group (the spec's boundary law);
/// `commands::verify` re-exports this same owner for its own verdicts.
pub const RULE_VIOLATION_REPORTED: &str = "\u{1}archspec-rule-violation-reported";

/// The verbs' entry: load the root spec the way the drivers used to
/// (silently — an invalid spec failed the verb earlier where the verb
/// loads it; a missing spec answers the spec-free walk) and extract with
/// the declared facts. Byte-identical to the pre-extraction
/// `scan::extract(language, root)`.
pub fn extract(language: Language, root: &Path) -> Result<Model, String> {
    let loaded = spec::load_scan(&canonical(root)).ok().flatten();
    extract_with_spec(language, root, loaded.as_ref())
}

/// Extract with the driver-availability gate applied and the root-spec
/// facts loaded (the shipped `extract_checked` contract, preserved).
pub fn extract_checked(language: Language, root: &Path) -> Result<Model, String> {
    if !driver_available(language) {
        return Err(driver_unavailable(language));
    }
    extract(language, root)
}

fn extract_with_spec(
    language: Language,
    root: &Path,
    loaded: Option<&spec::Spec>,
) -> Result<Model, String> {
    let declared = loaded.map(declared_facts).unwrap_or_default();
    let mut model = rust_arch_test_kit::scan::extract_with(language, root, &declared)?;
    // The pattern-resolution site (workplan events_value_join scenario 03,
    // decision-check F8): declared pattern sides meet the drivers'
    // vocabulary-free candidates HERE — one site, engine-side, so drivers
    // own no interpretation (A6) and the join downstream consumes model
    // facts only (4.7(i): no engine→scan import). An unresolvable owning
    // unit prints its named `UNITID-UNRESOLVED` line and writes no row
    // (fail-closed, the #56 posture, through the ONE canonical site
    // `events::unitid`, 4.4a(i)). A non-declaring run resolves nothing:
    // no candidates exist and no byte moves (4.2 arm 1) — the early exit is
    // the ONE named gate arm, `events::declarations_active` (scenario 08,
    // F-W1): consulted here once, never re-derived per verb or per arm below.
    if crate::archspec::events::declarations_active(&declared.patterns) {
        let mut index = UnitIdIndex::new();
        for unit in &model.units {
            let spellings = std::iter::once(unit.name.clone()).chain(unit.crate_ids.iter().cloned());
            index.register(&unit.path, spellings);
        }
        let outcome = patterns::resolve_literal_sites(&index, &declared.patterns, &model.literal_candidates);
        for refusal in &outcome.refusals {
            eprintln!("{refusal}");
        }
        model.literal_sites = outcome.sites;
        model.sort_literal_sites();
        // The value join (workplan events_value_join scenario 04): the
        // FIRST `Model::async_edges` construction site, at the same
        // declaration gate — derived machine facts ride the model only
        // where a pattern side is declared (4.2 arm 1). Orientation came
        // from the declared role; endpoints are the canonical ids the site
        // rows carry (4.4a(i)); every matched literal lands joined or in
        // one named row (F1's zero-silence bar), stated here on the report
        // surface, never a verdict. `constant_classes` moves the
        // shared-constant arm (a pair id naming a `[[shared_constant]]`
        // class feeds values, never edges, 4.4a(iv)); `unfunded` carries
        // the languages whose literal-site census reads `not-emitted` —
        // the capability table read HERE, never inside the join (4.7(i)).
        let constant_classes: std::collections::BTreeSet<String> = loaded
            .map(|loaded| {
                loaded
                    .shared_constant
                    .iter()
                    .filter_map(|entry| entry.class.clone())
                    .collect()
            })
            .unwrap_or_default();
        let unfunded: std::collections::BTreeSet<String> = declared
            .patterns
            .iter()
            .filter(|side| {
                crate::archspec::language::from_name(&side.language)
                    .and_then(|language| {
                        // Per-side SHAPE fact (workplan visibility_phase2
                        // scenario 02, A3): a field side is unfunded when
                        // the driver's `field-value-census` reads
                        // `not-emitted`; the other verbs keep reading the
                        // `literal-site-census` row unchanged. The join's
                        // `unfunded` input stays the shipped language-
                        // keyed set (join.rs unchanged): a language
                        // enters it through ANY of its sides' shape facts,
                        // and the arm consults it only for the counterpart
                        // side of the site's own pattern.
                        crate::archspec::capability::emission(
                            language,
                            match &side.verb {
                                DeclaredPatternVerb::FieldValue { .. } => {
                                    crate::archspec::capability::FACT_FIELD_VALUE_CENSUS
                                }
                                _ => crate::archspec::capability::FACT_LITERAL_SITE_CENSUS,
                            },
                        )
                    })
                    == Some(crate::archspec::capability::NOT_EMITTED)
            })
            .map(|side| side.language.clone())
            .collect();
        for line in derive::build_derived_async_edges(
            &mut model,
            &declared.patterns,
            &constant_classes,
            &unfunded,
        ) {
            eprintln!("{line}");
        }
        // The dedupe-vs-declared law (issue #48 follow-up, ADR-069 4.4):
        // the derived rows and the declared `[[async_depends]]` entries
        // meet at the ONE canonicalization site, `unitid::merge_rows`.
        // Identity-equal rows merge to one row with exactly one
        // `PLUGIN-OVERLAP` line — a derived row never suppresses a
        // declared arrow (the render set at the command seams is the
        // UNION of both spellings, so a declared arrow renders whatever
        // the merge says); an unresolvable endpoint spelling prints its
        // named `UNITID-UNRESOLVED` refusal and the row stays unmerged
        // (fail-closed, the #56 posture). A tree with no derived rows is
        // not fed: the shipped declared-only bytes are untouched (D8).
        if !model.async_edges.is_empty() {
            let mut producers: Vec<ProducerRow> = model
                .async_edges
                .iter()
                .map(|row| ProducerRow {
                    from: row.from.clone(),
                    to: row.to.clone(),
                    event: row.event.clone(),
                    producer_dir: row.from.clone(),
                })
                .collect();
            if let Some(loaded) = loaded {
                for entry in &loaded.async_depends {
                    if let (Some(from), Some(to), Some(event)) = (
                        entry.from.as_deref(),
                        entry.to.as_deref(),
                        entry.event.as_deref(),
                    ) && !from.is_empty()
                        && !to.is_empty()
                        && !event.is_empty()
                    {
                        producers.push(ProducerRow {
                            from: from.to_string(),
                            to: to.to_string(),
                            event: event.to_string(),
                            producer_dir: String::new(),
                        });
                    }
                }
            }
            let merged = crate::archspec::events::unitid::merge_rows(&index, &producers);
            for overlap in &merged.overlaps {
                eprintln!("{overlap}");
            }
            for refusal in &merged.refusals {
                eprintln!("{refusal}");
            }
            // The derived marker (scenario 04, F4): a derived row no
            // declared entry carries renders one arrow WITH this marker —
            // machine fact, stated, never a verdict and never a second
            // arrow over a merged identity.
            let declared_ids: std::collections::BTreeSet<
                crate::archspec::events::unitid::CanonicalRow,
            > = producers[model.async_edges.len()..]
                .iter()
                .filter_map(|row| crate::archspec::events::unitid::canonicalize_row(&index, row).ok())
                .collect();
            for row in &model.async_edges {
                let derived = ProducerRow {
                    from: row.from.clone(),
                    to: row.to.clone(),
                    event: row.event.clone(),
                    producer_dir: row.from.clone(),
                };
                let claimed = crate::archspec::events::unitid::canonicalize_row(&index, &derived)
                    .is_ok_and(|id| declared_ids.contains(&id));
                if !claimed {
                    eprintln!(
                        "async-row derived row=({}, {}, {}) — derived machine fact, \
                         no declared entry",
                        row.from, row.to, row.event
                    );
                }
            }
            // The attribution-ambiguity census (workplan events_value_join
            // scenario 05, ADR-069 4.4a(b) — the M-4 class made VISIBLE): a
            // hand-declared row that passes the verify arms while naming
            // the DECLARING side instead of the DISPATCHING side prints
            // exactly ONE named census line per mismatching declared
            // identity on the REPORT-ONLY channel — `model.report_only_lines`
            // is serde-skipped, so the census never reaches `scan.json`
            // (the report-only channel law), never a verdict, and never a
            // silent re-key of the row. The dispatch-side facts are the
            // derived rows: their `from` is the unit the declared role
            // orients as the publisher. A declaring side equal to the
            // dispatching side — or no dispatch fact sharing the declared
            // `(to, event)` pair — yields zero lines, the shipped bytes.
            // Unresolvable identities are skipped here, never re-named: the
            // merge above already stated their `UNITID-UNRESOLVED` refusals.
            if producers.len() > model.async_edges.len() {
                let to_fact = |row: &ProducerRow| {
                    crate::archspec::events::unitid::canonicalize_row(&index, row)
                        .ok()
                        .map(|id| (id.from, id.to, id.event))
                };
                let dispatch_facts: Vec<(String, String, String)> =
                    producers[..model.async_edges.len()]
                        .iter()
                        .filter_map(to_fact)
                        .collect();
                let declared_facts: Vec<(String, String, String)> =
                    producers[model.async_edges.len()..]
                        .iter()
                        .filter_map(to_fact)
                        .collect();
                let declared_rows: Vec<crate::archspec::events::attribution::FactRow<'_>> =
                    declared_facts
                        .iter()
                        .map(|(from, to, event)| crate::archspec::events::attribution::FactRow {
                            from,
                            to,
                            event,
                        })
                        .collect();
                let dispatch_rows: Vec<crate::archspec::events::attribution::FactRow<'_>> =
                    dispatch_facts
                        .iter()
                        .map(|(from, to, event)| crate::archspec::events::attribution::FactRow {
                            from,
                            to,
                            event,
                        })
                        .collect();
                model.report_only_lines.extend(
                    crate::archspec::events::attribution::attribution_rows(
                        &declared_rows,
                        &dispatch_rows,
                    ),
                );
            }
        }
        // The dispatch-blinded projection (workplan events_value_join
        // scenario 06, F-W4 option a): per zone carrying the shipped
        // `AMBIGUOUS-MARKERS` dispatch row, ONE report-only census row when
        // a declared pattern language lost that dispatch — the zero-silence
        // face over the 4.8 dispatch law's losers. The inputs are existing
        // rows only: the dispatch surface is the shipped `marker_report`
        // output RE-READ (the same pure call the verb made before the
        // agreement precondition, so its lines are byte-identical — this
        // seam prints none of them), and the file census reads the
        // model's resolved literal-site rows. No new scanner walk, no new
        // dispatch law: the authority decision stays clause 4.8's, the
        // lines are census bytes on the report surface — never the machine
        // body, never a verdict, and a run without declared pattern sides
        // never reaches them (4.2 arm 1).
        let declared_languages: std::collections::BTreeSet<String> = declared
            .patterns
            .iter()
            .map(|side| {
                crate::archspec::language::from_name(&side.language).map_or_else(
                    || side.language.clone(),
                    |language| language.as_str().to_string(),
                )
            })
            .collect();
        let site_refs: Vec<crate::archspec::events::blindspots::SiteRef<'_>> = model
            .literal_sites
            .iter()
            .map(|site| crate::archspec::events::blindspots::SiteRef {
                path: site.path.as_str(),
                language: site.language.as_str(),
            })
            .collect();
        for line in crate::archspec::events::blindspots::dispatch_blinded_rows(
            &crate::archspec::language::marker_report(root, language),
            &declared_languages,
            &site_refs,
        ) {
            eprintln!("{line}");
        }
    }
    Ok(model)
}

/// The literal-site census and driver-gap report lines (workplan
/// events_value_join scenario 03, F8's named funded driver set): one line
/// per declared pattern side, in declaration order (`emit` before
/// `subscribe`). A funded side prints its count — zero included, the
/// never-silent law; a side whose driver's literal-site census reads
/// `not-emitted` in the capability table prints the named `driver-gap`
/// absence row, never silence and never a guess. Report-only, event-
/// ordered, byte-deterministic.
fn literal_site_report(loaded: &spec::Spec, model: &Model) -> String {
    let census = patterns::literal_site_census(&model.literal_sites);
    let mut out = String::new();
    for pair in &loaded.pattern_pair {
        for side in spec::pattern_sides(pair) {
            // Per-side SHAPE fact (workplan visibility_phase2 scenario 02,
            // A3/F6): a declared `field` side is funded by the driver's
            // `field-value-census` row, not by the `literal-site-census`
            // row — a driver that collects call/property candidates but no
            // field-value candidates names the gap, never a `sites=0`
            // silence-adjacent zero.
            let (fact, census_name, site_name) = match &side.verb {
                DeclaredPatternVerb::FieldValue { .. } => (
                    crate::archspec::capability::FACT_FIELD_VALUE_CENSUS,
                    "field-value",
                    "field-value",
                ),
                _ => (
                    crate::archspec::capability::FACT_LITERAL_SITE_CENSUS,
                    "literal-site",
                    "literal",
                ),
            };
            let gap = crate::archspec::language::from_name(&side.language)
                .and_then(|language| crate::archspec::capability::emission(language, fact))
                == Some(crate::archspec::capability::NOT_EMITTED);
            if gap {
                out.push_str(&format!(
                    "driver-gap: pattern \"{}\" language \"{}\" — {census_name} census \
                     not-emitted for this driver: the declared side collects no {site_name} \
                     sites this release\n",
                    side.id, side.language,
                ));
                continue;
            }
            let anchor = match &side.verb {
                DeclaredPatternVerb::CallNameArgPosition { .. } => {
                    rust_arch_test_kit::model::LiteralAnchor::CallNameArgPosition
                }
                DeclaredPatternVerb::ExportedPropertyValue { .. } => {
                    rust_arch_test_kit::model::LiteralAnchor::ExportedPropertyValue
                }
                DeclaredPatternVerb::FieldValue { .. } => {
                    rust_arch_test_kit::model::LiteralAnchor::FieldValue
                }
            };
            let sites = census
                .get(&(
                    side.id.clone(),
                    side.language.clone(),
                    anchor.as_str().to_string(),
                ))
                .copied()
                .unwrap_or(0);
            out.push_str(&format!(
                "literal sites: pattern \"{}\" language \"{}\" anchor {} sites={sites}\n",
                side.id,
                side.language,
                anchor.as_str(),
            ));
        }
    }
    out
}

/// Map the loaded spec onto the engine's declared facts: the entries the
/// adjudicating drivers consume, in the same order and values the drivers
/// derived from the spec before the extraction.
fn declared_facts(loaded: &spec::Spec) -> DeclaredFacts {
    DeclaredFacts {
        exclusions: spec::exclusion_entries(loaded),
        zones: loaded
            .modules
            .iter()
            .map(|zone| DeclaredZone {
                name: zone.name.clone(),
                units: zone.matches.units.clone(),
                paths: zone.matches.paths.clone(),
                modules: zone.matches.modules.clone(),
            })
            .collect(),
        framework: loaded.project.framework.clone(),
        // The `[[pattern_pair]]` table flattened to per-side facts: entry
        // order, `emit` side before `subscribe` side (the declaration's own
        // order — the same pair, seen twice by the candidate match, is two
        // provenances, and their roles settle at the join, not here).
        patterns: loaded
            .pattern_pair
            .iter()
            .flat_map(spec::pattern_sides)
            .collect(),
    }
}

/// The canonicalization every spec-reading driver applied to its root
/// before the spec load; the glue reproduces it at the same point.
fn canonical(root: &Path) -> std::path::PathBuf {
    root.canonicalize().unwrap_or_else(|_| root.to_path_buf())
}
