// The dispatch-blinded-literals projection (workplan events_value_join
// scenario 06, ADR-069 clause 4.8 + decision-check F-W4 option a: a NAMED
// projection, not a deferral). The dispatch law shipped with issue #63:
// a nested directory carrying root markers for more than one language is a
// ZONE, and a rooted scan models it with the run's driver — one
// `AMBIGUOUS-MARKERS` census row names the zone, the marker set and the
// winner. What that row cannot state is the zero-silence residue: every
// literal of a LOSING zone language was never scanned, so the declared
// pattern sides of that language collect nothing from the zone and the
// census shows zeros the reader could mistake for an empty zone. This file
// closes that unnamed window: for each zone carrying a dispatch row, when a
// declared pattern language lost that dispatch, ONE report-only
// `dispatch-blinded-literals zone=<path> langs=<set> claimed=<driver>
// files=<n>` row prints.
//
// The row PROJECTS existing rows only — the dispatch rows emitted by
// `crate::language::marker_report` (this module reads their OUTPUT rows, it
// adds no scanner walk of its own and imports no driver) plus the
// literal-site rows the resolution seam already wrote. `files=<n>` counts
// the DISTINCT literal-site files the census DOES see for the blinded
// languages under that zone; under one-driver dispatch that count is zero,
// and the zero IS the blindness, stated as a census number instead of
// silence. The row never re-decides the dispatch: the authority/refusal
// question stays clause 4.8's (the shipped visibility-only scope, GitHub
// issue #63), the row never enters the machine body and never a verdict.
// A tree with no dispatch row prints zero rows; a run with no declared
// pattern side reaches this seam not at all (the 4.2 arm 1 declaration
// gate stands at the call site).
//
// Generic by law (engine boundary): this file carries no application,
// framework, ecosystem or vendor string — the language spellings arrive as
// DATA, parsed from the dispatch row's own bytes and from the declared
// pattern census, and the vocabulary census of `src/archspec/events/` keeps
// its ZERO baseline through this file. The header stays plain `//`
// comments, the sibling law of the module weld.

use std::collections::BTreeSet;

/// The shipped dispatch row's prefix — the byte family emitted by
/// `crate::language::marker_report` (issue #63), read here as INPUT rows,
/// never re-spelled.
const DISPATCH_PREFIX: &str = "AMBIGUOUS-MARKERS: ";

/// One literal-site row as the projection sees it: the pair of row FIELDS
/// the grouping reads (the repo-relative path and the driver-language
/// field), flattened from `model::LiteralSite` by the caller — the same
/// seam shape `events::join` keeps std-only through its `JoinSite`.
pub struct SiteRef<'a> {
    pub path: &'a str,
    pub language: &'a str,
}

/// One parsed dispatch row: the zone directory (tree-root-relative), the
/// marker languages in the row's own table order, and the driver the run
/// models the zone with. `None` states a line that is not a dispatch row —
/// the foreign-marker siblings of the same report surface ride the same
/// text, and a line that parses nowhere contributes nothing (fail closed,
/// never a guess at a row the scan never printed).
#[derive(Debug, PartialEq, Eq)]
pub struct DispatchRow<'a> {
    pub zone: &'a str,
    pub langs: Vec<&'a str>,
    pub claimed: &'a str,
}

/// Parse one report-surface line as a dispatch row. The grammar is the
/// shipped line's own bytes: `AMBIGUOUS-MARKERS: <dir> — markers: <file>
/// (<language>), …, modeled as: <language>`, with the marker list riding
/// the root-marker table order. Nothing is re-derived: the zone, the
/// languages and the winner are READ off the row, so this projection
/// cannot disagree with the dispatch face it projects.
#[must_use]
pub fn parse_dispatch_line(line: &str) -> Option<DispatchRow<'_>> {
    let rest = line.strip_prefix(DISPATCH_PREFIX)?;
    let (zone, rest) = rest.split_once(" — markers: ")?;
    let (markers, claimed) = rest.split_once(", modeled as: ")?;
    let mut langs = Vec::new();
    for entry in markers.split(", ") {
        let open = entry.rfind('(')?;
        let close = entry.rfind(')')?;
        if close <= open {
            return None;
        }
        langs.push(&entry[open + 1..close]);
    }
    Some(DispatchRow {
        zone,
        langs,
        claimed,
    })
}

/// The projection entry: the dispatch report surface, the declared pattern
/// languages (the pattern census, canonical driver spellings — the CALLER
/// normalizes them, this file compares strings only), and the literal-site
/// rows. One row per zone whose dispatch row names a losing language the
/// declaration carries a side for, in sorted line order (the same
/// deterministic law the dispatched lines themselves sort under).
#[must_use]
pub fn dispatch_blinded_rows(
    dispatch_surface: &str,
    declared_languages: &BTreeSet<String>,
    sites: &[SiteRef<'_>],
) -> Vec<String> {
    let mut rows: Vec<String> = Vec::new();
    for line in dispatch_surface.lines() {
        let Some(row) = parse_dispatch_line(line) else {
            continue;
        };
        // The blinded set: zone languages that lost dispatch AND the tree
        // declares a pattern side for them. A losing language nobody
        // declared feeds no census input — the row group is empty and the
        // surface stays silent-by-zero-input (the row never speculates
        // about literals no declaration matches).
        let blinded: BTreeSet<&str> = row
            .langs
            .iter()
            .copied()
            .filter(|language| *language != row.claimed)
            .filter(|language| declared_languages.contains(&(*language).to_string()))
            .collect();
        if blinded.is_empty() {
            continue;
        }
        // The input census: files the literal-site rows DO carry for the
        // blinded languages inside the zone. Under single-driver dispatch
        // the set is empty — the zero states that dispatch blinded them.
        let files: BTreeSet<&str> = sites
            .iter()
            .filter(|site| within_zone(row.zone, site.path) && blinded.contains(site.language))
            .map(|site| site.path)
            .collect();
        rows.push(format!(
            "dispatch-blinded-literals zone={} langs={} claimed={} files={}",
            row.zone,
            blinded
                .iter()
                .map(|language| format!("\"{language}\""))
                .collect::<Vec<_>>()
                .join(", "),
            row.claimed,
            files.len(),
        ));
    }
    rows.sort();
    rows
}

/// True when a repo-relative site path sits inside the zone directory:
/// path-equal (a marker-named file AT the zone) or below it. The prefix
/// test rides the trailing separator, so the zone `crates/node` holds
/// `crates/node/app.js` and never `crates/nodejs/app.js` — the same
/// component-wise law the exclusion matching applies.
fn within_zone(zone: &str, path: &str) -> bool {
    path == zone || path.starts_with(&format!("{zone}/"))
}
