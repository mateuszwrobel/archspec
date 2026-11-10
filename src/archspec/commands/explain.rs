use crate::archspec::cli::{self, FlagSpec};
use crate::archspec::language;
use crate::archspec::scan;
use crate::archspec::spec;
use crate::archspec::verify::compare;
use std::path::Path;

pub const HELP: &str = "usage: archspec explain <source> <target> [path] [--constraint <pattern>]\nprint the adjudication trace for one pair: endpoint resolution, claim chain, model rows, territory membership — facts from the verify path, not a verdict\n\n  source, target  declared module name or model file path — the pair to trace\n  path            project directory to load and extract (default: current directory)\n  --constraint <pattern>  limit the census to constraint rows whose type or declared\n                module names match the glob\n\nexplain renders the same load, extract and compare calls `archspec verify` runs and\nprints what that path computed for the pair: the module-graph row and its value tier,\nthe claiming module chain, territory membership per constraint row, and whether this\nrun's findings name the pair. Exit codes carry the usage and operational classes\nonly — a finding never moves the exit code; verdicts stay in `archspec verify`.\n\nthe per-constraint census stays in `archspec verify --coverage`; finding categories\nmean what `archspec help diagnostics` says; explain adds the per-pair trace those\nsurfaces cannot express.\n";

pub fn run(args: &[String]) -> Result<(), String> {
    let parsed = cli::parse(
        args,
        &[FlagSpec {
            name: "constraint",
            takes_value: true,
        }],
    )?;
    if parsed.positionals.len() < 2 {
        return Err(cli::usage_error(
            "usage: archspec explain <source> <target> [path] [--constraint <pattern>]",
        ));
    }
    if parsed.positionals.len() > 3 {
        return Err(cli::usage_error("expected at most one path argument"));
    }
    let source = parsed.positionals[0].as_str();
    let target = parsed.positionals[1].as_str();
    let root = parsed.positionals.get(2).map(String::as_str).unwrap_or(".");
    let filter = parsed.values.get("constraint").map(String::as_str);

    let root_path = Path::new(root);
    if !root_path.exists() {
        return Err(format!("path does not exist: {root}"));
    }
    if !root_path.is_dir() {
        return Err(format!("path is not a directory: {root}"));
    }

    // The preamble mirrors `verify` call for call (GitHub issue #26, bound B1):
    // the same spec load, the same driver selection, the same extraction. The
    // report-surface facts (exclusions, markers, wildcards) ride stderr exactly
    // as verify prints them, so the facts behind a trace can never be quieter
    // here than on the verdict surface.
    let loaded = spec::load_verify(root_path)?;
    eprint!("{}", spec::excluded_sources_report(&loaded));
    eprint!("{}", spec::exclusion_liveness_report(root_path, &loaded));
    let declared_language = crate::archspec::commands::verify::language_from_str(&loaded.project.language)?;
    let language = language::select(root_path, Some(declared_language))?
        .ok_or_else(|| format!("no supported-language sources found under: {root}"))?;
    eprint!("{}", language::marker_report(root_path, language));
    if let Some(finding) = language::agreement_finding(root_path, language, &spec::exclusion_paths(&loaded)) {
        return Err(finding);
    }
    if !scan::driver_available(language) {
        return Err(format!(
            "no {} toolchain found (driver missing); run 'archspec doctor' to diagnose",
            crate::archspec::commands::verify::language_display(language)
        ));
    }
    let model = scan::extract_checked(language, root_path)?;
    eprint!("{}", scan::wildcard_report(&model));

    // The same adjudication inputs `compare` derives internally (bound B1):
    // augment, module-tier projection, flattened claims and the module graph
    // come from the very functions verify's path runs, so every fact printed
    // below is a value that path computes, never a re-derivation.
    let mapping = compare::map_units(&model.units, &loaded.modules);
    let mapping = compare::augment_with_modules(&mapping, &model, &loaded.modules);
    let derived = compare::effective_module_tier(&model, &mapping);
    let model_ref: &crate::archspec::model::Model = derived.as_ref();
    let claims = compare::flatten_claims(&loaded.modules);
    let claims_index = compare::ClaimIndex::new(&claims);
    let composition_sources = compare::composition_sources(model_ref);
    let graph = compare::resolve_module_edges(
        model_ref,
        &mapping,
        &loaded.modules,
        &claims,
        &composition_sources,
        Some(&claims_index),
    );
    // The verdict machinery itself runs unchanged (parity by construction):
    // the diff below is exactly the diff `verify` computes for this tree+spec,
    // and `pair in findings` reads it, never the trace.
    let base_mapping = compare::map_units(&model.units, &loaded.modules);
    let diff = compare::compare(
        &base_mapping,
        &model,
        &loaded.modules,
        &loaded.stereotype,
        &loaded.constraint,
        &loaded.exempt,
    );

    let source_ep = resolve_endpoint(source, model_ref, &loaded.modules, &claims, &claims_index);
    let target_ep = resolve_endpoint(target, model_ref, &loaded.modules, &claims, &claims_index);

    let mut out = String::new();
    out.push_str(&format!("pair: {source} -> {target}\n"));
    out.push_str(&format!("source: {}\n", source_ep.describe()));
    out.push_str(&format!("target: {}\n", target_ep.describe()));

    // Model-row facts: the module-graph pair the adjudication walks, its value
    // tier, and the file-grained rows joining the endpoints.
    let source_ids = source_ep.pair_identities();
    let target_ids = target_ep.pair_identities();
    let pair_present = graph
        .pairs
        .iter()
        .any(|(from, to)| source_ids.iter().any(|id| id == from) && target_ids.iter().any(|id| id == to));
    let pair_runtime = graph
        .value_pairs
        .iter()
        .any(|(from, to)| source_ids.iter().any(|id| id == from) && target_ids.iter().any(|id| id == to));
    out.push_str(&format!(
        "model rows:\n  module-graph row {} -> {}: {}\n",
        source,
        target,
        if pair_present { "present" } else { "absent" }
    ));
    if pair_present {
        out.push_str(&format!(
            "  value tier: {}\n",
            if pair_runtime {
                "value edge present (runtime ring class)"
            } else {
                "no value edge on this pair — a type-assisted row (type-assisted ring class)"
            }
        ));
    }
    let (file_rows, file_row_total) = file_rows_between(&source_ep, &target_ep, model_ref);
    if file_row_total == 0 {
        out.push_str("  file-tier rows: 0 — the model carries no file-grained row between these endpoints\n");
    } else {
        out.push_str(&format!(
            "  file-tier rows {} -> {}: {file_rows} of {file_row_total} source rows\n",
            source, target
        ));
        for (from, to, kind) in file_rows_listing(&source_ep, &target_ep, model_ref) {
            out.push_str(&format!("    {from} -> {to}{}\n", kind.map(|k| format!(" ({k})")).unwrap_or_default()));
        }
    }

    out.push_str("constraints:\n");
    let rows: Vec<&spec::Constraint> = loaded
        .constraint
        .iter()
        .filter(|row| filter.is_none_or(|p| row_matches_filter(row, p)))
        .collect();
    if rows.is_empty() {
        out.push_str("  none — the spec declares no constraint row under this filter\n");
    }
    for (index, row) in loaded.constraint.iter().enumerate() {
        if !filter.is_none_or(|p| row_matches_filter(row, p)) {
            continue;
        }
        out.push_str(&format!("  [{index}] {}", row.kind));
        if row.kind == "no_cycles" {
            let scope = row.scope.clone().unwrap_or_else(|| match language::from_name(&model.language) {
                Some(lang) => capability_scope(lang),
                None => "claims".to_string(),
            });
            out.push_str(&format!(" scope = {scope}"));
            if !row.kinds.is_empty() {
                out.push_str(&format!(" kinds = {:?}", row.kinds));
            }
            if row.modules.is_empty() {
                out.push_str(" — modules absent: every declared module is in territory");
            } else {
                let source_in = row.modules.iter().any(|m| source_ids.iter().any(|id| id == m));
                let target_in = row.modules.iter().any(|m| target_ids.iter().any(|id| id == m));
                out.push_str(&format!(
                    " — territory modules = {:?}: source {}, target {}",
                    row.modules,
                    if source_in { "in" } else { "out" },
                    if target_in { "in" } else { "out" }
                ));
                if pair_present && (!source_in || !target_in) {
                    out.push_str(" — the pair sits on the row's graph but leaves its declared territory, so the row never walks it");
                }
            }
            let adjudicated = source_in_territory(&source_ids, row, &loaded.modules)
                && target_in_territory(&target_ids, row, &loaded.modules)
                && pair_present;
            out.push_str(&format!(" — pair adjudicated: {}\n", if adjudicated { "yes" } else { "no" }));
        } else if PAIR_SURFACE_ROWS.contains(&row.kind.as_str()) {
            out.push_str(&format!(
                " — pair adjudicated: {}\n",
                if pair_present { "yes" } else { "no (no module-graph row between the endpoints)" }
            ));
        } else {
            out.push_str(" — the row adjudicates manifest, export or external facts, not module pairs\n");
        }
    }

    out.push_str("findings naming the pair:\n");
    let named = findings_naming(&diff, &source_ids, &target_ids);
    if named.is_empty() {
        out.push_str("  none — this run's diff holds no line naming both endpoints\n");
    } else {
        for finding in named {
            out.push_str(&format!("  {finding}\n"));
        }
    }
    out.push_str("note: explain renders the same load, extract and compare calls `archspec verify` runs — verdicts stay in `archspec verify`\n");

    print!("{out}");
    Ok(())
}

/// Constraint rows whose check consumes module-pair adjacency. Rows outside
/// this set state manifest/export/external facts and state so in the trace.
const PAIR_SURFACE_ROWS: &[&str] = &[
    "no_cycles",
    "feature_boundary",
    "forbid_submodule_dependency",
];

/// The effective `no_cycles` scope for a language when the row states none:
/// the capability posture table's own answer, the same source compare reads.
fn capability_scope(language: language::Language) -> String {
    crate::archspec::capability::posture(language).0.to_string()
}

fn row_matches_filter(row: &spec::Constraint, pattern: &str) -> bool {
    compare::matches_pattern(&row.kind, pattern)
        || row.modules.iter().any(|m| compare::matches_pattern(m, pattern))
}

fn source_in_territory(ids: &[String], row: &spec::Constraint, modules: &[spec::Module]) -> bool {
    territory_holds(ids, row, &declared_module_names(modules))
}

fn target_in_territory(ids: &[String], row: &spec::Constraint, modules: &[spec::Module]) -> bool {
    territory_holds(ids, row, &declared_module_names(modules))
}

fn declared_module_names(modules: &[spec::Module]) -> Vec<String> {
    modules.iter().map(|m| m.name.clone()).collect()
}

fn territory_holds(ids: &[String], row: &spec::Constraint, all: &[String]) -> bool {
    if row.modules.is_empty() {
        return ids.iter().any(|id| all.contains(id));
    }
    ids.iter().any(|id| row.modules.iter().any(|m| m == id))
}

/// One endpoint of the traced pair, resolved against the model and the spec.
/// Resolution is fact-ordered: a declared module name first, then a model
/// file (claim-attributed), then the bare identity space. Every branch states
/// what matched; nothing is guessed.
struct Endpoint {
    kind: EndpointKind,
}

enum EndpointKind {
    /// A declared module, with the claim patterns it spells.
    DeclaredModule {
        name: String,
        units: Vec<String>,
        paths: Vec<String>,
    },
    /// A model file: its claim chain (innermost first) and module identity.
    ModelFile {
        path: String,
        chain: Vec<String>,
        identity: String,
    },
    /// Spelled neither as a declared module nor as a model file.
    Unresolved,
}

impl Endpoint {
    fn describe(&self) -> String {
        match &self.kind {
            EndpointKind::DeclaredModule { name, units, paths } => {
                let mut line = format!("module \"{name}\" — declared");
                if units.is_empty() {
                    line.push_str(", matches.units absent (no unit pattern)");
                } else {
                    line.push_str(&format!(", matches.units = {units:?}"));
                }
                if !paths.is_empty() {
                    line.push_str(&format!(", matches.modules = {paths:?}"));
                }
                line
            }
            EndpointKind::ModelFile {
                path,
                chain,
                identity,
            } => {
                let chain = if chain.is_empty() {
                    "no claim owns this file".to_string()
                } else {
                    format!("claim chain: {}", chain.join(" < "))
                };
                format!("file {path} — {chain}; module identity: {identity}")
            }
            EndpointKind::Unresolved => {
                "unresolved: not a declared module name and no model file claims this spelling"
                    .to_string()
            }
        }
    }

    /// The identity strings this endpoint can answer as in the module-graph
    /// pair space and the territory lists.
    fn pair_identities(&self) -> Vec<String> {
        match &self.kind {
            EndpointKind::DeclaredModule { name, .. } => vec![name.clone()],
            EndpointKind::ModelFile { identity, chain, .. } => {
                let mut ids = vec![identity.clone()];
                for claim in chain {
                    if !ids.contains(claim) {
                        ids.push(claim.clone());
                    }
                }
                ids
            }
            EndpointKind::Unresolved => Vec::new(),
        }
    }
}

fn resolve_endpoint(
    input: &str,
    model: &crate::archspec::model::Model,
    modules: &[spec::Module],
    claims: &[compare::Claim<'_>],
    claims_index: &compare::ClaimIndex,
) -> Endpoint {
    if let Some(module) = modules.iter().find(|m| m.name == input) {
        return Endpoint {
            kind: EndpointKind::DeclaredModule {
                name: module.name.clone(),
                units: module.matches.units.clone(),
                paths: module.matches.modules.clone(),
            },
        };
    }
    let owns = claims_index.claim_owner(claims, input);
    let claimed = compare::file_claim_index(model, input, claims);
    if owns.is_some() || claimed.is_some() || model_looks_at_path(model, input) {
        let chain = claimed
            .or(owns)
            .map(|index| claim_chain(claims, index))
            .unwrap_or_default();
        return Endpoint {
            kind: EndpointKind::ModelFile {
                path: input.to_string(),
                chain,
                identity: compare::file_module_identity(model, input),
            },
        };
    }
    Endpoint {
        kind: EndpointKind::Unresolved,
    }
}

/// The claiming module chain: the owning claim, then its ancestors, innermost
/// first — the chain the adjudication walks for jurisdiction.
fn claim_chain(claims: &[compare::Claim<'_>], mut index: usize) -> Vec<String> {
    let mut chain = Vec::new();
    loop {
        chain.push(claims[index].name.to_string());
        match claims[index].parent {
            Some(parent) => index = parent,
            None => break,
        }
    }
    chain
}

/// True when the spelling appears as an endpoint of any file-grained row or
/// unit path — a model file even when no claim owns it.
fn model_looks_at_path(model: &crate::archspec::model::Model, path: &str) -> bool {
    compare::file_grained_rows(model)
        .into_iter()
        .any(|(from, to)| from == path || to == path)
}

/// (rows joining the endpoints, total rows out of the source endpoint).
fn file_rows_between(
    source: &Endpoint,
    target: &Endpoint,
    model: &crate::archspec::model::Model,
) -> (usize, usize) {
    let source_ids = source.pair_identities();
    let target_ids = target.pair_identities();
    let rows = compare::file_grained_rows(model);
    let source_rows = rows
        .iter()
        .filter(|(from, _)| endpoint_matches_file(&source_ids, from))
        .count();
    let joining = rows
        .iter()
        .filter(|(from, to)| {
            endpoint_matches_file(&source_ids, from) && endpoint_matches_file(&target_ids, to)
        })
        .count();
    (joining, source_rows)
}

fn file_rows_listing(
    source: &Endpoint,
    target: &Endpoint,
    model: &crate::archspec::model::Model,
) -> Vec<(String, String, Option<String>)> {
    let source_ids = source.pair_identities();
    let target_ids = target.pair_identities();
    model
        .file_edges
        .iter()
        .filter(|edge| {
            endpoint_matches_file(&source_ids, &edge.from)
                && endpoint_matches_file(&target_ids, &edge.to)
        })
        .map(|edge| {
            (
                edge.from.clone(),
                edge.to.clone(),
                Some(serde_json::to_value(edge.kind).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_else(|| "row".to_string())),
            )
        })
        .take(20)
        .collect()
}

/// A file path belongs to the endpoint when it IS the file endpoint's path,
/// or when its claiming chain names one of the endpoint's identities.
fn endpoint_matches_file(ids: &[String], path: &str) -> bool {
    ids.iter().any(|id| id == path)
}

fn findings_naming(diff: &compare::Diff, source_ids: &[String], target_ids: &[String]) -> Vec<String> {
    // The error-tier fields come from `Diff::error_fields` — the same table
    // that gates the exit code and that `help diagnostics` catalogs, so the
    // categories named here can never drift from the verdict machinery.
    // The warning bucket entries are pre-rendered (`<category>: <detail>`),
    // so they are scanned verbatim.
    let mut named = Vec::new();
    for (label, entries) in diff.error_fields() {
        for entry in entries {
            if names_both(entry, source_ids, target_ids) {
                named.push(format!("{label}: {entry}"));
            }
        }
    }
    for line in &diff.warnings {
        if names_both(line, source_ids, target_ids) {
            named.push(line.clone());
        }
    }
    named.sort();
    named.dedup();
    named
}

fn names_both(line: &str, source_ids: &[String], target_ids: &[String]) -> bool {
    source_ids.iter().any(|id| line.contains(id.as_str()))
        && target_ids.iter().any(|id| line.contains(id.as_str()))
}
