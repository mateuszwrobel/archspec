use crate::archspec::model::{matches_pattern, Model, Role, Unit};
use crate::archspec::viewer;
use crate::archspec::viewer::{ViewerEdge, ViewerNode};
use rust_arch_test_kit::render::{
    dot_attr, dot_edge, dot_node, hand_svg, legend_rows, mermaid_counted_edge, mermaid_edge,
    mermaid_labeled_edge, mermaid_module_edge, mermaid_module_labeled_edge, mermaid_module_node,
    mermaid_node, mermaid_node_marked, mermaid_subgraph, plantuml_component,
    plantuml_component_marked, plantuml_counted_edge, plantuml_edge, plantuml_labeled_edge,
    plantuml_quoted_component, plantuml_quoted_violation_edge, plantuml_violation_edge,
    COLOR_EDGE_ASYNC, COLOR_EDGE_DIRECT,
    COLOR_EDGE_VIOLATION, COLOR_NODE_EXTERNAL, COLOR_NODE_UNIT, COLOR_ROLE_COMPOSITION,
    COLOR_ROLE_FACADE, SvgEdge, SvgNode,
};
use crate::archspec::spec::{Module, Spec, Stereotype};
use crate::archspec::adjudicate::{Adjudicator, ModuleRender, file_grained_rows};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

/// The `--layer` selector lives beside the shared async-row machinery in
/// the depgraph engine module (`LayerSelection`); the diagram renderers and
/// the command seam both reach it through this re-export.
pub use crate::archspec::depgraph::LayerSelection;

/// The pinned mermaid-host ceilings the ceiling rationale states in
/// `archspec diagram --help` (claim `layer-ceiling-rationale`): mermaid
/// hosts pin 500 edges and 50,000 characters, and a graph over either
/// ceiling renders nothing. The rationale ships ARMED at the mermaid emit
/// seam (workplan visibility_phase2 scenario 05's done-when 5 named the
/// facts falsifiable; the improvement sweep HIGH-1 row named the missing
/// runtime arm): every `--format mermaid` render crosses
/// [`mermaid_ceiling_error`] before a byte is emitted, and an
/// over-ceiling render refuses in the layer arms' usage class (exit 64)
/// with nothing written. These are the arithmetic's single code site.
pub const MERMAID_EDGE_CEILING: usize = 500;
/// The character arm of the same pinned ceiling (host `maxTextSize`).
pub const MERMAID_CHAR_CEILING: usize = 50_000;

/// The two ceilings stated exactly as the help rationale and the refusal
/// row state them — the thousands separator is part of the registered
/// wording (claim `layer-ceiling-rationale`). The unit test
/// `the_ceiling_statement_agrees_with_the_arithmetic` welds these bytes to
/// the constants above: a drift between wording and arithmetic fails there.
const MERMAID_CEILING_STATEMENT: &str = "500 edges and 50,000 characters";

/// The edge rows of a rendered mermaid body: one structural arrow (`-->`)
/// or one labelled dashed row (`-.->`) is one edge — the same edge-row
/// count the corpus-scale witness of the rationale measures ("module-edge
/// rows"). Layout links (`~~~`) draw no arrow and count as nothing.
pub fn mermaid_edge_rows(body: &str) -> usize {
    body.lines()
        .filter(|line| line.contains("-->") || line.contains("-.->"))
        .count()
}

/// The ceiling arm at the mermaid emit seam: `None` at and below both
/// ceilings, the refusal row above either one — the measured counts, the
/// ceiling fact in the registered wording, then the next actions that
/// clear the render (the degraded-projection note's named-flag shape;
/// `clear_with` names only flags the calling surface accepts, so the row
/// never points at a flag its own command would refuse). The refusal is
/// the layer arms' grammar: the command wraps the message in
/// `cli::usage_error` (exit 64) and emits nothing on refusal — no
/// truncation, no partial body, no destination touch.
pub fn mermaid_ceiling_error(body: &str, clear_with: &str) -> Option<String> {
    let edges = mermaid_edge_rows(body);
    let chars = body.chars().count();
    if edges <= MERMAID_EDGE_CEILING && chars <= MERMAID_CHAR_CEILING {
        return None;
    }
    Some(format!(
        "{}; clear it with {clear_with}",
        mermaid_ceiling_head(edges, chars)
    ))
}

/// The measured head of every over-ceiling mermaid refusal — the bytes the
/// whole-model seam and the level-fit seam share (the registered wording of
/// claim `mermaid-ceiling-refusal`, minus its `clear it with` tail). The
/// level arm appends the NAME-THE-FITS row instead of a clear-with tail
/// (workplan level_views scenario 02).
fn mermaid_ceiling_head(edges: usize, chars: usize) -> String {
    format!(
        "this render exceeds the mermaid ceilings of {MERMAID_CEILING_STATEMENT} — measured {edges} edges and {chars} characters, and a graph over either ceiling renders nothing"
    )
}

/// The ceiling facts read back out of the diagram help bytes — the plan
/// assumption "the ceiling values parse from `archspec diagram --help` at
/// run time; every fit and refusal decision follows the parse" (workplan
/// level_views). The parse anchors on the ceiling rationale sentence
/// ("ceilings of 500 edges and 50,000 characters"); `None` when the help
/// carries no such sentence, and the caller falls back to the constants the
/// unit test `ceiling_facts_parse_from_the_help_bytes` welds to the help.
/// The fit verdicts ride the parsed pair, never a restated literal.
pub fn mermaid_ceilings_from_help(help: &str) -> Option<(usize, usize)> {
    let anchor = help.find("ceilings of ")? + "ceilings of ".len();
    let digits: String = help[anchor..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    let edges: usize = digits.parse().ok()?;
    let rest = &help[anchor + digits.len()..];
    let rest = rest.strip_prefix(" edges and ")?;
    let chars_at = rest.find(" characters")?;
    let digits: String = rest[..chars_at]
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    let chars: usize = digits.replace(',', "").parse().ok()?;
    Some((edges, chars))
}

/// The fold enumeration order (PoC §2 table): `unit`, then `dir:1` up to the
/// deepest directory the model's endpoints carry, then `module`, then `file`.
/// `dir:0` never enters — the fit-set floor excludes the degenerate
/// whole-tree fold (workplan level_views §CLI partial).
fn level_folds(model: &Model) -> Vec<LevelSelection> {
    let mut max_dirs = 1usize;
    for edge in &model.module_edges {
        for path in [&edge.from, &edge.to] {
            if is_root_file(model, path) {
                continue;
            }
            let parts = if path.contains('/') {
                path.split('/').count()
            } else if path.contains("::") {
                path.split("::").count()
            } else {
                1
            };
            // The last segment names the addressed file or module, so the
            // directory parts are the segments before it — a single-segment
            // spelling IS the directory at the tree root (the same reading
            // `level_bucket` gives).
            let dirs = parts.saturating_sub(1).max(1);
            max_dirs = max_dirs.max(dirs);
        }
    }
    let mut folds = vec![LevelSelection::Unit];
    for k in 1..=max_dirs {
        folds.push(LevelSelection::Dir(k));
    }
    folds.push(LevelSelection::Module);
    folds.push(LevelSelection::File);
    folds
}

/// The NAME-THE-FITS enumeration on the EMITTER'S OWN BYTES (decision-check
/// MAJOR-7): every candidate fold is drawn through the ONE fold site and
/// measured as the exact bytes the emit would write — badge shape included,
/// never a char estimate. The walk is monotone (proven on both witnesses):
/// it STOPS at the first OVER fold and measures no finer fold — a finer
/// fold never fits when a coarser one is OVER (MINOR-19). A fold enters the
/// fit list only with cross-edges above 0 (BLOCKER-2: the degenerate fold
/// is excluded); every entry carries its self-share percent.
pub fn level_fit_entries(
    model: &Model,
    layer: LayerSelection,
    edge_ceiling: usize,
    char_ceiling: usize,
) -> Vec<(LevelSelection, usize)> {
    let mut entries = Vec::new();
    for level in level_folds(model) {
        let projection = level_projection(model, level, None);
        let body = render_level_mermaid(&projection, layer);
        if mermaid_edge_rows(&body) > edge_ceiling || body.chars().count() > char_ceiling {
            break;
        }
        let cross: usize = projection.cross.values().sum();
        if cross == 0 {
            continue;
        }
        let self_share = (projection.self_total * 100)
            .checked_div(projection.raw_rows)
            .unwrap_or(0);
        entries.push((level, self_share));
    }
    entries
}

/// The two-step narrowing grammar the refusal prints (MAJOR-20). The
/// placeholders stay placeholders — `archspec scan` gains no new semantics
/// from this row, and the guard cell instantiates the steps and runs them
/// as printed against a fixture subtree.
const LEVEL_NARROWING_GRAMMAR: &str = "narrow with: archspec scan <bucket-path> --output <artefact>, then archspec diagram --source scan <artefact> [--level <fit>]";

/// The NAME-THE-FITS row: every fitting fold with its self share, or the
/// named empty-fits row when no fold keeps cross-edges above 0 (B2/M10 —
/// never silence), then the narrowing grammar.
pub fn level_fit_refusal_row(
    model: &Model,
    layer: LayerSelection,
    edge_ceiling: usize,
    char_ceiling: usize,
) -> String {
    let entries = level_fit_entries(model, layer, edge_ceiling, char_ceiling);
    let fits = if entries.is_empty() {
        "fits: none — no fold fits at these ceilings".to_string()
    } else {
        let rows: Vec<String> = entries
            .iter()
            .map(|(level, share)| format!("{} ({share}% self)", level.as_str()))
            .collect();
        format!("fits: {}", rows.join(", "))
    };
    format!("{fits}; {LEVEL_NARROWING_GRAMMAR}")
}

/// The ceiling arm at the level render seam: same measured head as the
/// whole-model seam, but the level path names the fits and the two-step
/// narrowing instead of a clear-with tail — the verdict follows the ceilings
/// parsed from the help bytes, and the emit stays silent: an over-ceiling
/// level render emits nothing and never silently auto-picks a coarser level.
pub fn level_mermaid_ceiling_error(
    body: &str,
    model: &Model,
    layer: LayerSelection,
    edge_ceiling: usize,
    char_ceiling: usize,
) -> Option<String> {
    let edges = mermaid_edge_rows(body);
    let chars = body.chars().count();
    if edges <= edge_ceiling && chars <= char_ceiling {
        return None;
    }
    Some(format!(
        "{}; {}",
        mermaid_ceiling_head(edges, chars),
        level_fit_refusal_row(model, layer, edge_ceiling, char_ceiling)
    ))
}

/// The emitted-byte ceiling arm at the viewer emit seam (workplan
/// level_views scenario 05, decision-check §BLOCKER-3 fix): `None` when the
/// emitted document stands at or under [`viewer::VIEWER_BYTE_CEILING`]; the
/// named byte-ceiling refusal above it. The refusal names the LARGEST SCOPE
/// THAT FITS — the FINEST fold whose FULL emitted document measures under
/// the ceiling, decided EMIT-AND-MEASURE on the emitter's own bytes
/// (MAJOR-7 law: pane-embedded folds make the byte mass non-monotone in
/// the fold, so any byte assumption would be an invention — every candidate
/// is measured, embedded pane refusal rows and census rows included, which
/// is the parent-budget charge by construction; an embedded refusal never
/// re-triggers this gate, the refusal rows are plain precomputed bytes).
/// The walk is the MINOR-19 fits-set shape on the byte budget: coarse→fine,
/// recording every fold that fits and STOPPING at the first fold whose own
/// emitted bytes pass the ceiling — a finer fold beyond the stop is never
/// measured (it would only burn bytes to prove refusal). The named scope
/// is the FINEST fold that fits contiguously from the coarsest — the
/// richest render that ships. The emit refuses LOUD: nothing is written,
/// the
/// refusal rides the usage class, and the row stays actionable — the
/// fitting scope or the named empty row plus the two-step narrowing
/// grammar (MAJOR-20 bytes), never silence.
pub fn viewer_byte_ceiling_error(
    doc: &str,
    model: &Model,
    layer: LayerSelection,
    fold: Option<LevelSelection>,
    ceilings: (usize, usize),
    gap_census: Option<String>,
) -> Option<String> {
    if doc.len() <= viewer::VIEWER_BYTE_CEILING {
        return None;
    }
    let folds = level_folds(model);
    // The candidate set: every fold from the coarsest through the
    // REQUESTED one (a scope finer than the requested fold is not what the
    // user asked to draw); MINOR-19 trims it at the first fold whose own
    // cross-edges pass the edge ceiling.
    let upto = match fold {
        Some(level) => folds
            .iter()
            .position(|candidate| *candidate == level)
            .map_or(folds.len(), |at| at + 1),
        None => folds.len(),
    };
    let mut largest: Option<(LevelSelection, usize)> = None;
    for level in folds.into_iter().take(upto) {
        let emitted = render_level_viewer(
            model,
            &level_projection(model, level, gap_census.clone()),
            layer,
            ceilings,
        );
        if emitted.len() > viewer::VIEWER_BYTE_CEILING {
            // MINOR-19 stop at the first OVER: a finer fold is never
            // measured to prove refusal, and the scope already recorded
            // — the FINEST fold that fit before the stop — is the largest
            // scope that fits.
            break;
        }
        largest = Some((level, emitted.len()));
    }
    Some(viewer_byte_ceiling_refusal(doc.len(), largest))
}

/// The named byte-ceiling refusal: the measured head stating the ceiling
/// constant's bytes, then the largest scope that fits with its measured
/// bytes — or the named empty-scope row when no fold fits (B2/M10 law: the
/// empty set is NAMED, never silent) — then the two-step narrowing grammar
/// (MAJOR-20 bytes, the same registered row sibling 02 ships).
fn viewer_byte_ceiling_refusal(measured: usize, fits: Option<(LevelSelection, usize)>) -> String {
    let scope = match fits {
        Some((level, bytes)) => format!(
            "largest scope that fits: level {} — measured {bytes} bytes",
            level.as_str()
        ),
        None => {
            "largest scope that fits: none — no scope fits under the emitted-byte ceiling"
                .to_string()
        }
    };
    format!(
        "this viewer document exceeds the emitted-byte ceiling of {} bytes — measured {measured} bytes, and a document over the ceiling ships nothing; {scope}; {LEVEL_NARROWING_GRAMMAR}",
        viewer::VIEWER_BYTE_CEILING
    )
}

/// The pane fold list (workplan level_views scenario 04, TERMINATION LAW):
/// the shipped fit walk of sibling 02 over the bucket-relative submodel,
/// MINUS the `unit` entry. A one-bucket submodel has no unit fold — with no
/// units to attribute, its `unit` fold would spell every endpoint by its
/// own path, putting the FINEST fold first and breaking the coarse→fine
/// order the MINOR-19 monotone stop rests on. The relative walk starts at
/// `dir:1`.
fn level_pane_folds(model: &Model) -> Vec<LevelSelection> {
    level_folds(model)
        .into_iter()
        .filter(|level| *level != LevelSelection::Unit)
        .collect()
}

/// The pane fit walk (workplan level_views scenario 04): the SAME fit
/// machinery as the CLI refusal — the ONE fold site ([`level_projection`]),
/// the emitter's own bytes, the monotone stop at the first OVER fold — run
/// over the bucket-relative fold list. Returns the folds that FIT (in
/// coarse→fine order) and the first OVER fold: the pane draws the LAST
/// fitting fold — the FINEST that fits — and a refusal names the OVER
/// fold's granularity.
fn level_pane_walk(
    model: &Model,
    layer: LayerSelection,
    edge_ceiling: usize,
    char_ceiling: usize,
) -> (Vec<LevelSelection>, Option<LevelSelection>) {
    let mut fits = Vec::new();
    let mut first_over = None;
    for level in level_pane_folds(model) {
        let projection = level_projection(model, level, None);
        let body = render_level_mermaid(&projection, layer);
        if mermaid_edge_rows(&body) > edge_ceiling || body.chars().count() > char_ceiling {
            first_over = Some(level);
            break;
        }
        let cross: usize = projection.cross.values().sum();
        if cross == 0 {
            continue;
        }
        fits.push(level);
    }
    (fits, first_over)
}

/// The directory parts of one endpoint path — the same spelling rule
/// [`level_bucket`] reads (slash paths fold on `/`, `::` spellings on
/// `::`): the last segment names the addressed file or module, so the
/// directory parts are the segments before it, and a single-segment
/// spelling IS the directory at the fold root.
fn pane_dir_parts(path: &str) -> Vec<&str> {
    let parts: Vec<&str> = if path.contains('/') {
        path.split('/').collect()
    } else if path.contains("::") {
        path.split("::").collect()
    } else {
        vec![path]
    };
    if parts.len() > 1 {
        parts[..parts.len() - 1].to_vec()
    } else {
        parts
    }
}

/// The bucket-relative submodel one pane re-projects (workplan level_views
/// scenario 04, the FOLD-ROOT LAW): the bucket's OWN structural rows — the
/// `module_edges` rows whose two buckets both equal the bucket — with every
/// endpoint spelled RELATIVE TO THE BUCKET ROOT, the longest common
/// directory prefix of the bucket's rows. Global directory depth is the
/// forbidden shape; only the input rows and the fold root move, the fold
/// site itself stays the one [`level_projection`]. `None` when the bucket
/// carries no structural internal rows: the pane region then draws nothing
/// at all ("never an empty pane"), the rows stay counted in the
/// whole-document total row, and an async row folded into the bucket rides
/// the ledger, never a pane of its own.
fn pane_submodel(model: &Model, projection: &LevelProjection, bucket: &str) -> Option<Model> {
    let module_node = crate::archspec::depgraph::projection_node(model);
    let in_bucket = |path: &str| level_bucket(model, path, projection.level, module_node) == bucket;
    let internal: Vec<&crate::archspec::model::ModuleEdge> = model
        .module_edges
        .iter()
        .filter(|edge| in_bucket(&edge.from) && in_bucket(&edge.to))
        .collect();
    if internal.is_empty() {
        return None;
    }
    let mut common: Option<Vec<&str>> = None;
    for edge in &internal {
        for path in [&edge.from, &edge.to] {
            let dirs = pane_dir_parts(path);
            common = Some(match common {
                None => dirs,
                Some(prefix) => prefix
                    .into_iter()
                    .zip(dirs)
                    .take_while(|(a, b)| a == b)
                    .map(|(a, _)| a)
                    .collect(),
            });
        }
    }
    let root = common.unwrap_or_default().join("/");
    let relative = |path: &str| -> String {
        if root.is_empty() {
            return path.to_string();
        }
        path.strip_prefix(&format!("{root}/"))
            .map_or_else(|| path.to_string(), |rest| rest.to_string())
    };
    Some(Model {
        module_edges: internal
            .into_iter()
            .map(|edge| crate::archspec::model::ModuleEdge {
                from: relative(&edge.from),
                to: relative(&edge.to),
                ..edge.clone()
            })
            .collect(),
        ..Model::new(&model.language)
    })
}

/// The named in-pane refusal row (workplan level_views scenario 04,
/// TERMINATION LAW): a pane over ceiling at its finest relative
/// granularity speaks this row — never an empty pane, never a raw dump —
/// naming the OVER granularity and the real bucket path in the shipped
/// two-step narrowing shape. The refusal pane restates the completeness
/// census row (M6 residue). Claim `level-pane-refusal`.
fn pane_refusal_row(bucket: &str, over: LevelSelection) -> String {
    format!(
        "pane over ceiling at {} — narrow: archspec scan {bucket} --output <artefact>, then archspec diagram --source scan <artefact> [--level <fit>]",
        over.as_str()
    )
}

struct SpecGraph {
    nodes: BTreeSet<String>,
    clusters: Vec<(String, Vec<String>)>,
    allowed_edges: BTreeSet<(String, String)>,
    forbidden_edges: BTreeSet<(String, String)>,
    /// The declared `[[async_depends]]` rows, `(from, to, label)` — endpoints
    /// are the declared UNIT names, the label the short `from::SimpleName`
    /// form (scenario 08), and the set order IS the `(from, to, event)` law
    /// (N6). Empty for every spec without the table, and the renderers draw
    /// it only when non-empty: a non-declaring spec's diagram bytes stand
    /// unchanged (workplan archspec_async_depends, N2/D8). An entry with an
    /// unresolved key contributes nothing — a half-declared row is the
    /// validator's refusal, never the renderer's guess.
    async_edges: BTreeSet<(String, String, String)>,
    /// Async endpoint names that no declared module names — neither as a
    /// node nor as a cluster member (issue #51 residual, N4). A declared
    /// module claims its units through `matches.units`, so an endpoint the
    /// spec claims renders under the CLAIMING MODULE's name — the byte-identical
    /// endpoint the solid edges use. A name no module claims stays its own
    /// name and rides here, so the renderers declare it as a labelled node
    /// instead of letting Mermaid draw the raw salted id as box text.
    undeclared_async_nodes: BTreeSet<String>,
}

fn build_graph(spec: &Spec) -> SpecGraph {
    let mut nodes = BTreeSet::new();
    let mut clusters = Vec::new();
    let mut allowed_edges = BTreeSet::new();
    let mut forbidden_edges = BTreeSet::new();
    let mut descendants: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    fn collect(
        module: &Module,
        depth: usize,
        nodes: &mut BTreeSet<String>,
        clusters: &mut Vec<(String, Vec<String>)>,
        allowed_edges: &mut BTreeSet<(String, String)>,
        forbidden_edges: &mut BTreeSet<(String, String)>,
        descendants: &mut BTreeMap<String, BTreeSet<String>>,
    ) -> BTreeSet<String> {
        let mut subtree = BTreeSet::new();
        let mut subs: Vec<String> = Vec::new();
        for submodule in &module.submodules {
            subs.push(submodule.name.clone());
            let sub_subtree = collect(
                submodule,
                depth + 1,
                nodes,
                clusters,
                allowed_edges,
                forbidden_edges,
                descendants,
            );
            subtree.extend(sub_subtree);
        }
        if depth == 0 {
            nodes.insert(module.name.clone());
        }
        if !subs.is_empty() {
            subs.sort();
            clusters.push((module.name.clone(), subs));
        }
        for entry in &module.allowed.depend_on {
            allowed_edges.insert((module.name.clone(), entry.target.clone()));
        }
        for entry in &module.allowed.forbidden {
            forbidden_edges.insert((module.name.clone(), entry.target.clone()));
        }
        descendants.insert(module.name.clone(), subtree.clone());
        subtree.insert(module.name.clone());
        subtree
    }

    for module in &spec.modules {
        collect(
            module,
            0,
            &mut nodes,
            &mut clusters,
            &mut allowed_edges,
            &mut forbidden_edges,
            &mut descendants,
        );
    }
    let contained = |from: &str, to: &str| -> bool {
        descendants
            .get(from)
            .is_some_and(|subtree| subtree.contains(to))
            || descendants
                .get(to)
                .is_some_and(|subtree| subtree.contains(from))
    };
    allowed_edges.retain(|(from, to)| !contained(from, to));
    forbidden_edges.retain(|(from, to)| !contained(from, to));
    clusters.sort_by(|a, b| a.0.cmp(&b.0));

    // The declared async rows draw intent, like the declared allowed edges
    // do — no containment fold, no validation (N2: the renderer states the
    // table, `verify` adjudicates it). One declared entry is one row with
    // unit endpoints (scenario 08); an entry whose endpoints name one unit
    // draws nothing (the self-edge law). GitHub issue #48 moves the KEY:
    // the rows are assembled as `(from, to, full event identity)` and the
    // labels assigned through the ONE shared implementation
    // (`depgraph::async_render_rows`) — two entries over one pair whose
    // events share a simple name keep TWO arrows with distinguishable
    // labels, never one collapsed line.
    let mut async_identities: Vec<(String, String, String)> = Vec::new();
    for entry in &spec.async_depends {
        if let (Some(from), Some(to), Some(event)) =
            (entry.from.as_deref(), entry.to.as_deref(), entry.event.as_deref())
            && !from.is_empty()
            && !to.is_empty()
            && !event.is_empty()
            && from != to
        {
            async_identities.push((from.to_string(), to.to_string(), event.to_string()));
        }
    }
    let async_edges_raw: BTreeSet<(String, String, String)> =
        crate::archspec::depgraph::async_render_rows(async_identities)
            .into_iter()
            .collect();

    // Issue #51 residual (N4): the declared rows name UNITS, the graph draws
    // MODULES. An endpoint one declared module claims (`matches.units` — the
    // only claim grammar submodules reject at parse) renders under that
    // module's name, so the dashed arrow binds to the node the solid edges
    // already declare — no raw salted id ever reaches the box text. An
    // endpoint several modules claim, or none, keeps its own name: the
    // renderer declares it as its own labelled node (never an invented
    // placement, the same one-arrow-honest-vocabulary stance as the folded
    // depgraph views). Names already in the graph's own vocabulary (a
    // module named like a unit — the shape every shipped fixture and the
    // reference consumer's module-per-project spec carry) resolve to
    // themselves, so their bytes stand unchanged (D8).
    let mut vocabulary: BTreeSet<String> = nodes.clone();
    for (_, subs) in &clusters {
        vocabulary.extend(subs.iter().cloned());
    }
    let claimant = |endpoint: &str| -> Option<String> {
        let claimed_by: BTreeSet<String> = spec
            .modules
            .iter()
            .filter(|module| module.matches.units.iter().any(|unit| unit == endpoint))
            .map(|module| module.name.clone())
            .collect();
        if claimed_by.len() == 1 {
            claimed_by.into_iter().next()
        } else {
            None
        }
    };
    let mut undeclared_async_nodes = BTreeSet::new();
    let async_edges: BTreeSet<(String, String, String)> = async_edges_raw
        .into_iter()
        .map(|(from, to, event)| {
            let mut placed = |name: &str| -> String {
                if vocabulary.contains(name) {
                    return name.to_string();
                }
                match claimant(name) {
                    Some(module) => module,
                    None => {
                        undeclared_async_nodes.insert(name.to_string());
                        name.to_string()
                    }
                }
            };
            (placed(&from), placed(&to), event)
        })
        .collect();

    SpecGraph {
        nodes,
        clusters,
        allowed_edges,
        forbidden_edges,
        async_edges,
        undeclared_async_nodes,
    }
}

pub fn render_spec_mermaid(spec: &Spec, layer: LayerSelection) -> String {
    let graph = build_graph(spec);
    let mut out = String::from("graph TD\n");
    for node in &graph.nodes {
        let _ = writeln!(out, "  {}", mermaid_node(node));
    }
    for (parent, subs) in &graph.clusters {
        let _ = writeln!(out, "  {}", mermaid_subgraph(parent));
        for sub in subs {
            let _ = writeln!(out, "    {}", mermaid_node(sub));
        }
        let _ = writeln!(out, "  end");
    }
    // Async endpoints no declared module claims (issue #51 residual, N4):
    // declared as labelled nodes before any edge references them, so the
    // box text reads the name, never the raw salted id. An empty set —
    // every non-declaring or fully claimed render — adds no bytes. Under
    // `--layer direct` nothing references them, so they stay undeclared
    // (workplan visibility_phase2 scenario 05: the selector at the loops).
    if layer.draws_async() {
        for node in &graph.undeclared_async_nodes {
            let _ = writeln!(out, "  {}", mermaid_node(node));
        }
    }
    if layer.draws_structural() {
        for (from, to) in &graph.allowed_edges {
            let _ = writeln!(out, "  {}", mermaid_edge(from, to));
        }
        for (from, to) in &graph.forbidden_edges {
            let _ = writeln!(out, "  {}", mermaid_labeled_edge(from, to, "forbidden"));
        }
    }
    // The declared async rows render dashed and event-labelled after every
    // direct arrow (workplan archspec_async_depends, N2/D8/follow-up 6;
    // scenario 08: unit endpoints, short label, one line per entry); an
    // empty set renders no bytes. The `--layer` selector stands this loop
    // down for `direct`; `both` — the default — renders unchanged bytes.
    if layer.draws_async() {
        for (from, to, event) in &graph.async_edges {
            let _ = writeln!(out, "  {}", mermaid_labeled_edge(from, to, event));
        }
    }
    out
}

pub fn render_spec_plantuml(spec: &Spec, layer: LayerSelection) -> String {
    let graph = build_graph(spec);
    let mut out = String::from("@startuml\n");
    for node in &graph.nodes {
        let _ = writeln!(out, "{}", plantuml_component(node));
    }
    for (parent, subs) in &graph.clusters {
        let _ = writeln!(out, "package {parent} {{");
        for sub in subs {
            let _ = writeln!(out, "  {}", plantuml_component(sub));
        }
        let _ = writeln!(out, "}}");
    }
    // PlantUML parity of the alias arm (issue #51 residual, N4): the same
    // unclaimed endpoint names, declared with the same component grammar the
    // module nodes use; empty renders no bytes; `--layer direct` draws no
    // async, so they stay undeclared (scenario 05 parity).
    if layer.draws_async() {
        for node in &graph.undeclared_async_nodes {
            let _ = writeln!(out, "{}", plantuml_component(node));
        }
    }
    if layer.draws_structural() {
        for (from, to) in &graph.allowed_edges {
            let _ = writeln!(out, "{}", plantuml_edge(from, to));
        }
        for (from, to) in &graph.forbidden_edges {
            let _ = writeln!(out, "{}", plantuml_violation_edge(from, to, "forbidden"));
        }
    }
    // PlantUML parity with the mermaid arm (follow-up 6): same rows, same
    // order, dashed labelled grammar; empty renders no bytes. The `--layer`
    // selector stands this loop down for `direct` (scenario 05).
    if layer.draws_async() {
        for (from, to, event) in &graph.async_edges {
            let _ = writeln!(out, "{}", plantuml_labeled_edge(from, to, event));
        }
    }
    let _ = writeln!(out, "@enduml");
    out
}

/// Render the declared spec as graphviz `dot` source (`--format dot`,
/// workplan visibility_phase2 scenario 05 stage 2): the same
/// `build_graph` assembly the text renderers consume — one assembly site,
/// every format — with the shared type-color vocabulary drawn as per-class
/// attributes and the GENERATED legend cluster whenever the `both` layer
/// selection draws both edge families. The SVG materializes by piping this
/// body through `dot -Tsvg` in a container; the payload vendors no binary.
pub fn render_spec_dot(spec: &Spec, layer: LayerSelection) -> String {
    let graph = build_graph(spec);
    let structural = if layer.draws_structural() {
        graph.allowed_edges.len()
    } else {
        0
    };
    let marks = if layer.draws_structural() {
        graph.forbidden_edges.len()
    } else {
        0
    };
    let async_drawn = if layer.draws_async() {
        graph.async_edges.len()
    } else {
        0
    };
    let legend = legend_rows(layer == LayerSelection::Both, structural, async_drawn, marks);
    let mut out = String::new();
    let _ = writeln!(out, "// archspec diagram — layer: {}", layer.as_str());
    let _ = writeln!(out, "digraph archspec {{");
    let _ = writeln!(out, "  graph [rankdir=TB, splines=true, overlap=false];");
    let _ = writeln!(out, "  node [shape=box, style=filled, fontname=\"Helvetica\"];");
    for node in &graph.nodes {
        let _ = writeln!(out, "  {}", dot_node(node, node, COLOR_NODE_UNIT, None));
    }
    for (index, (parent, subs)) in graph.clusters.iter().enumerate() {
        let _ = writeln!(out, "  subgraph cluster_{index} {{");
        let _ = writeln!(out, "    label=\"{}\";", rust_arch_test_kit::render::dot_escape(parent));
        for sub in subs {
            let _ = writeln!(out, "    {}", dot_node(sub, sub, COLOR_NODE_UNIT, None));
        }
        let _ = writeln!(out, "  }}");
    }
    if layer.draws_async() {
        for node in &graph.undeclared_async_nodes {
            let _ = writeln!(out, "  {}", dot_node(node, node, COLOR_NODE_UNIT, None));
        }
    }
    if layer.draws_structural() {
        for (from, to) in &graph.allowed_edges {
            let _ = writeln!(
                out,
                "  {}",
                dot_edge(from, to, &[dot_attr("color", COLOR_EDGE_DIRECT)])
            );
        }
        for (from, to) in &graph.forbidden_edges {
            let _ = writeln!(
                out,
                "  {}",
                dot_edge(
                    from,
                    to,
                    &[
                        dot_attr("color", COLOR_EDGE_VIOLATION),
                        dot_attr("style", "dashed"),
                        dot_attr("label", "forbidden"),
                    ]
                )
            );
        }
    }
    if layer.draws_async() {
        for (from, to, event) in &graph.async_edges {
            let _ = writeln!(
                out,
                "  {}",
                dot_edge(
                    from,
                    to,
                    &[
                        dot_attr("color", COLOR_EDGE_ASYNC),
                        dot_attr("style", "dashed"),
                        dot_attr("label", event),
                    ]
                )
            );
        }
    }
    crate::archspec::depgraph::write_dot_legend(&mut out, legend.as_deref());
    let _ = writeln!(out, "}}");
    out
}

/// Render the declared spec as a hand-emitted SVG (`--format svg`): the
/// same `build_graph` inventory as every other format, one `<g class=
/// "node">` per node and one `<path class="edge">` per drawn arrow, plus
/// the generated legend group the dot arm draws.
pub fn render_spec_svg(spec: &Spec, layer: LayerSelection) -> String {
    let graph = build_graph(spec);
    let mut nodes: Vec<SvgNode> = graph
        .nodes
        .iter()
        .map(|node| svg_module_node(node))
        .collect();
    for (_, subs) in &graph.clusters {
        nodes.extend(subs.iter().map(|sub| svg_module_node(sub)));
    }
    if layer.draws_async() {
        nodes.extend(
            graph
                .undeclared_async_nodes
                .iter()
                .map(|node| svg_module_node(node)),
        );
    }
    let mut edges: Vec<SvgEdge> = Vec::new();
    let (mut structural, mut marks) = (0usize, 0usize);
    if layer.draws_structural() {
        structural = graph.allowed_edges.len();
        marks = graph.forbidden_edges.len();
        edges.extend(graph.allowed_edges.iter().map(|(from, to)| SvgEdge {
            from: from.clone(),
            to: to.clone(),
            stroke: COLOR_EDGE_DIRECT.to_string(),
            dashed: false,
            label: None,
        }));
        edges.extend(graph.forbidden_edges.iter().map(|(from, to)| SvgEdge {
            from: from.clone(),
            to: to.clone(),
            stroke: COLOR_EDGE_VIOLATION.to_string(),
            dashed: true,
            label: Some("forbidden".to_string()),
        }));
    }
    let mut async_drawn = 0usize;
    if layer.draws_async() {
        async_drawn = graph.async_edges.len();
        edges.extend(graph.async_edges.iter().map(|(from, to, event)| SvgEdge {
            from: from.clone(),
            to: to.clone(),
            stroke: COLOR_EDGE_ASYNC.to_string(),
            dashed: true,
            label: Some(event.clone()),
        }));
    }
    let legend = legend_rows(layer == LayerSelection::Both, structural, async_drawn, marks);
    hand_svg("diagram", &nodes, &edges, legend.as_deref().unwrap_or(&[]))
}

fn svg_module_node(name: &str) -> SvgNode {
    SvgNode {
        id: name.to_string(),
        label: name.to_string(),
        fill: COLOR_NODE_UNIT.to_string(),
        stroke: COLOR_EDGE_DIRECT.to_string(),
    }
}

struct ScanGraph {
    project_nodes: BTreeSet<String>,
    external_nodes: BTreeSet<String>,
    edges: BTreeSet<(String, String)>,
    violating: BTreeSet<(String, String)>,
    external_edges: BTreeSet<(String, String)>,
    /// Module-tier nodes: identity -> visible label (claim name + raw path,
    /// or the raw path alone when no claim owns the endpoint). Present only
    /// under `--tiers modules`; BTreeMap iteration is the canonical
    /// node order (by name).
    module_nodes: BTreeMap<String, String>,
    /// Module-tier arrows: (from identity, to identity, violates). One entry
    /// per distinct resolved endpoint pair; symbols multiplicity collapses.
    module_arrows: BTreeSet<(String, String, bool)>,
    /// The model's derived async rows, `(from, to, label)` — unit endpoints,
    /// the short `from::SimpleName` label (scenario 08), and the set order IS
    /// the `(from, to, event)` law (N6). The scan source draws the DERIVED
    /// facts (N2); an artefact without async rows — every artefact a
    /// no-derivation driver writes, and every artefact shipped before this
    /// feature — carries an empty set and renders its exact shipped bytes
    /// (D8). Renderers draw these at the unit tier; the module tier's
    /// territory math is untouched.
    async_arrows: BTreeSet<(String, String, String)>,
}

fn build_scan_graph(model: &Model, governing: Option<&Spec>, tiers: &TierSelection) -> ScanGraph {
    let mut project_nodes: BTreeSet<String> = model.units.iter().map(|u| u.name.clone()).collect();
    let external_nodes: BTreeSet<String> = model.external.iter().cloned().collect();
    project_nodes.retain(|name| !external_nodes.contains(name));

    // Marks and module names come from the shared adjudication seam — the
    // same pipeline `verify` runs — never a diagram-side re-derivation: a
    // unit edge is dashed exactly when `verify` reports its pair, and a
    // specless artefact (empty modules) marks nothing.
    let modules: &[Module] = governing.map(|spec| spec.modules.as_slice()).unwrap_or(&[]);
    let stereotypes: &[Stereotype] = governing.map(|spec| spec.stereotype.as_slice()).unwrap_or(&[]);
    let adjudicator = Adjudicator::new(model, modules, stereotypes);
    let constraints = governing.map(|spec| build_constraints(spec, model));

    let mut edges = BTreeSet::new();
    let mut violating = BTreeSet::new();
    let mut external_edges = BTreeSet::new();
    for edge in &model.edges {
        let touches_external =
            external_nodes.contains(&edge.from) || external_nodes.contains(&edge.to);
        if touches_external {
            external_edges.insert((edge.from.clone(), edge.to.clone()));
        } else if constraints
            .as_ref()
            .is_some_and(|constraints| edge_violates(constraints, model, &edge.from, &edge.to))
            || adjudicator.unit_edge_violates(&edge.from, &edge.to)
        {
            violating.insert((edge.from.clone(), edge.to.clone()));
        } else {
            edges.insert((edge.from.clone(), edge.to.clone()));
        }
    }

    let mut module_nodes: BTreeMap<String, String> = BTreeMap::new();
    let mut module_arrows: BTreeSet<(String, String, bool)> = BTreeSet::new();
    if tiers.modules {
        for edge in &model.module_edges {
            let placement = adjudicator.module_edge_placement(edge);
            let (from, from_path, to, to_path, draws_arrow, violates) = match &placement {
                ModuleRender::Arrow {
                    from,
                    from_path,
                    to,
                    to_path,
                    violates,
                    api_mark,
                } => (
                    *from,
                    *from_path,
                    *to,
                    *to_path,
                    true,
                    *violates || *api_mark,
                ),
                ModuleRender::Nesting {
                    from,
                    from_path,
                    to,
                    to_path,
                } => (*from, *from_path, *to, *to_path, false, false),
            };
            register_module_node(&mut module_nodes, &adjudicator, from, from_path);
            register_module_node(&mut module_nodes, &adjudicator, to, to_path);
            if draws_arrow && from != to {
                module_arrows.insert((from.to_string(), to.to_string(), violates));
            }
        }
    }

    ScanGraph {
        project_nodes,
        external_nodes,
        edges,
        violating,
        external_edges,
        module_nodes,
        module_arrows,
        async_arrows: crate::archspec::depgraph::async_render_rows(
            crate::archspec::depgraph::derived_render_rows(model)
                .into_iter()
                // the self-edge law (scenario 08) filters BEFORE the shared
                // identity-and-label assembly of issue #48
                .filter(|(from, to, _)| from != to),
        )
        .into_iter()
        .collect(),
    }
}

/// Record one module-tier endpoint node: identity is the owning claim name
/// where a claim owns the path, else the raw path; the label keeps identity
/// and path visible together so verify findings grep into the diagram. The
/// first registered label for an identity wins (deterministic under the
/// canonical `module_edges` iteration).
fn register_module_node(
    nodes: &mut BTreeMap<String, String>,
    adjudicator: &Adjudicator<'_>,
    identity: &str,
    path: &str,
) {
    let label = if identity == path || adjudicator.claim_naming(path).is_none() {
        path.to_string()
    } else {
        format!("{identity} ({path})")
    };
    nodes.entry(identity.to_string()).or_insert(label);
}

/// The per-module subgraph of one named claim (workplan
/// rust-arch-test-kit/worklog/workplan_archspec_module_public_api_diagrams.md
/// D11): the claim's territory nodes (own files union descendant paths, unit
/// roots excluded per D10), the nested-submodule nesting of its subclaims,
/// and its first-party boundary edges. Marks — violating and public-API —
/// read the shared adjudication seam, never a re-derivation; a governing
/// spec that declares no expose anywhere carries no API marks, so the view
/// never claims a green public API it cannot see.
/// A rendered module view: nodes (id → label) and arrows (from, to, marked).
type ModuleRenderGraph = (BTreeMap<String, String>, BTreeSet<(String, String, bool)>);

fn module_subgraph(
    model: &Model,
    spec: &Spec,
    claim: &str,
) -> ModuleRenderGraph {
    let adjudicator = Adjudicator::new(model, &spec.modules, &spec.stereotype);
    let mut nodes: BTreeMap<String, String> = BTreeMap::new();
    register_module_node(&mut nodes, &adjudicator, claim, claim);
    for path in adjudicator.territory_paths(claim) {
        let identity = adjudicator.claim_naming(&path).unwrap_or(path.as_str());
        register_module_node(&mut nodes, &adjudicator, identity, &path);
    }
    let mut arrows: BTreeSet<(String, String, bool)> = BTreeSet::new();
    let territory = adjudicator.territory_paths(claim);
    for edge in &model.module_edges {
        match adjudicator.module_edge_placement(edge) {
            ModuleRender::Arrow {
                from,
                from_path,
                to,
                to_path,
                violates,
                api_mark,
            } => {
                let touches = from == claim
                    || to == claim
                    || territory.contains(from_path)
                    || territory.contains(to_path);
                if touches {
                    register_module_node(&mut nodes, &adjudicator, from, from_path);
                    register_module_node(&mut nodes, &adjudicator, to, to_path);
                    if from != to {
                        arrows.insert((from.to_string(), to.to_string(), violates || api_mark));
                    }
                }
            }
            ModuleRender::Nesting {
                from,
                from_path,
                to,
                to_path,
            } => {
                if from == claim || to == claim {
                    register_module_node(&mut nodes, &adjudicator, from, from_path);
                    register_module_node(&mut nodes, &adjudicator, to, to_path);
                }
            }
        }
    }
    for sub in adjudicator.subclaims_of(claim) {
        register_module_node(&mut nodes, &adjudicator, sub, sub);
    }
    (nodes, arrows)
}

/// The files tier's cluster for one claim (design record D8, M13): the
/// vertex set is the model's file-grained facts under the findings' own
/// attribution (`file_claim_of`, D6/D10) — every file the claim owns that
/// the model shows — and the first-party file edges touching the territory.
/// An arrow is dashed `violates spec` exactly when the seam's
/// cycle-membership query marks its pair: the render never re-derives an
/// SCC or any other verdict (ADR-021 law). Assets arrive as endpoints of
/// their incoming arrows and mark nothing, because no finding ever names
/// them in a cycle (R2-6, R3-3 — a graph fact, not a render rule).
fn files_subgraph(
    model: &Model,
    spec: &Spec,
    claim: &str,
    adjudicator: &Adjudicator<'_>,
) -> (BTreeSet<String>, BTreeSet<(String, String, bool)>) {
    let marks = adjudicator.file_cycle_marks(&spec.constraint);
    let mut nodes = BTreeSet::new();
    let mut arrows = BTreeSet::new();
    for (from, to) in file_grained_rows(model) {
        let touches =
            adjudicator.file_claim_of(from) == Some(claim)
                || adjudicator.file_claim_of(to) == Some(claim);
        if !touches || from == to {
            continue;
        }
        nodes.insert(from.to_string());
        nodes.insert(to.to_string());
        arrows.insert((
            from.to_string(),
            to.to_string(),
            marks.contains(&(from.to_string(), to.to_string())),
        ));
    }
    for path in model.module_public_types.keys() {
        if adjudicator.file_claim_of(path) == Some(claim) {
            nodes.insert(path.clone());
        }
    }
    (nodes, arrows)
}

/// Render one claim's module subgraph (mermaid) — the `--module` body.
/// With the files tier selected the claim subgraph additionally carries a
/// nested `files` cluster; without it the bytes are the shipped render.
pub fn render_module_mermaid(model: &Model, spec: &Spec, claim: &str, tiers: &TierSelection) -> String {
    let (nodes, arrows) = module_subgraph(model, spec, claim);
    let mut out = String::from("graph TD\n");
    let _ = writeln!(out, "  {}", mermaid_subgraph(claim));
    for (identity, label) in &nodes {
        let _ = writeln!(out, "    {}", mermaid_module_node(identity, label));
    }
    for (from, to, violates) in &arrows {
        if *violates {
            let _ = writeln!(
                out,
                "    {}",
                mermaid_module_labeled_edge(from, to, "violates spec")
            );
        } else {
            let _ = writeln!(out, "    {}", mermaid_module_edge(from, to));
        }
    }
    if tiers.files {
        let adjudicator = Adjudicator::new(model, &spec.modules, &spec.stereotype);
        let (file_nodes, file_arrows) = files_subgraph(model, spec, claim, &adjudicator);
        if !file_nodes.is_empty() {
            let _ = writeln!(out, "    subgraph files");
            for path in &file_nodes {
                let _ = writeln!(out, "      {}", mermaid_module_node(path, path));
            }
            for (from, to, violates) in &file_arrows {
                if *violates {
                    let _ = writeln!(
                        out,
                        "      {}",
                        mermaid_module_labeled_edge(from, to, "violates spec")
                    );
                } else {
                    let _ = writeln!(out, "      {}", mermaid_module_edge(from, to));
                }
            }
            let _ = writeln!(out, "    end");
        }
    }
    let _ = writeln!(out, "  end");
    out
}

/// Render one claim's module subgraph (plantuml) — the `--module` body.
/// With the files tier selected the claim package additionally carries a
/// nested `files` package; without it the bytes are the shipped render.
pub fn render_module_plantuml(model: &Model, spec: &Spec, claim: &str, tiers: &TierSelection) -> String {
    let (nodes, arrows) = module_subgraph(model, spec, claim);
    let mut out = String::from("@startuml\n");
    let _ = writeln!(out, "package {claim} {{");
    for label in nodes.values() {
        let _ = writeln!(out, "  {}", plantuml_quoted_component(label));
    }
    for (from, to, violates) in &arrows {
        let from_label = nodes.get(from).map(String::as_str).unwrap_or(from.as_str());
        let to_label = nodes.get(to).map(String::as_str).unwrap_or(to.as_str());
        if *violates {
            let _ = writeln!(
                out,
                "{}",
                plantuml_quoted_violation_edge(from_label, to_label, "violates spec")
            );
        } else {
            let _ = writeln!(out, r#""{from_label}" --> "{to_label}""#);
        }
    }
    if tiers.files {
        let adjudicator = Adjudicator::new(model, &spec.modules, &spec.stereotype);
        let (file_nodes, file_arrows) = files_subgraph(model, spec, claim, &adjudicator);
        if !file_nodes.is_empty() {
            let _ = writeln!(out, "  package files {{");
            for path in &file_nodes {
                let _ = writeln!(out, "    {}", plantuml_quoted_component(path));
            }
            for (from, to, violates) in &file_arrows {
                if *violates {
                    let _ = writeln!(
                        out,
                        "    {}",
                        plantuml_quoted_violation_edge(from, to, "violates spec")
                    );
                } else {
                    let _ = writeln!(out, r#"    "{from}" --> "{to}""#);
                }
            }
            let _ = writeln!(out, "  }}");
        }
    }
    let _ = writeln!(out, "}}");
    let _ = writeln!(out, "@enduml");
    out
}

/// The batch filename escape (D12): split the claim name on `.`, percent-
/// escape every byte outside `[A-Za-z0-9._-]` (lowercase hex) and every `%`
/// inside a segment, encode an empty segment as `%2e`, and rejoin with
/// single dots. `.` and `..` are unrepresentable, the map is injective (`%`
/// escapes itself), and decoding recovers exactly one claim name.
pub fn claim_filename(claim: &str) -> String {
    claim
        .split('.')
        .map(|segment| {
            if segment.is_empty() {
                "%2e".to_string()
            } else {
                segment
                    .bytes()
                    .map(|byte| match byte {
                        b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-' => {
                            (byte as char).to_string()
                        }
                        b'.' => "%2e".to_string(),
                        other => format!("%{other:02x}"),
                    })
                    .collect::<String>()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Inverse of `claim_filename`.
pub fn claim_from_filename(name: &str) -> String {
    name.split('.')
        .map(|segment| {
            if segment == "%2e" {
                String::new()
            } else {
                let bytes = segment.as_bytes();
                let mut out = Vec::with_capacity(bytes.len());
                let mut i = 0;
                while i < bytes.len() {
                    if bytes[i] == b'%'
                        && i + 2 < bytes.len()
                        && let Ok(byte) =
                            u8::from_str_radix(&segment[i + 1..i + 3], 16)
                    {
                        out.push(byte);
                        i += 3;
                        continue;
                    }
                    out.push(bytes[i]);
                    i += 1;
                }
                String::from_utf8_lossy(&out).into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Resolve which unit names a governing spec forbids for each matching module.
struct ScanConstraints {
    /// unit name -> names of modules that match the unit
    modules_by_unit: BTreeMap<String, Vec<String>>,
    /// module name -> unit names the module forbids depending on
    forbidden: BTreeMap<String, BTreeSet<String>>,
}
fn build_constraints(spec: &Spec, model: &Model) -> ScanConstraints {
    let mut modules_by_unit: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for module in &spec.modules {
        for unit in &model.units {
            if unit_matches_module(unit, module) {
                modules_by_unit
                    .entry(unit.name.clone())
                    .or_default()
                    .push(module.name.clone());
            }
        }
    }
    for modules in modules_by_unit.values_mut() {
        modules.sort();
        modules.dedup();
    }

    let forbidden = spec
        .modules
        .iter()
        .map(|module| (module.name.clone(), forbidden_units(module, model, spec)))
        .collect();

    ScanConstraints {
        modules_by_unit,
        forbidden,
    }
}

fn edge_violates(constraints: &ScanConstraints, model: &Model, from: &str, to: &str) -> bool {
    if model.units.iter().all(|u| u.name != from) {
        return false;
    }
    let Some(modules) = constraints.modules_by_unit.get(from) else {
        return false;
    };
    modules.iter().any(|module| {
        constraints
            .forbidden
            .get(module)
            .map(|forbidden| forbidden.contains(to))
            .unwrap_or(false)
    })
}

/// Unit names a module may not depend on: its `allowed.forbidden` entries
/// (module names and direct name/path globs) plus `contract.forbid`
/// stereotypes matched by the target unit.
fn forbidden_units(module: &Module, model: &Model, spec: &Spec) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in &module.allowed.forbidden {
        let entry = entry.target.as_str();
        let mut resolved_by_module = false;
        for candidate in &spec.modules {
            if matches_pattern(&candidate.name, entry) {
                resolved_by_module = true;
                for unit in &model.units {
                    if unit_matches_module(unit, candidate) {
                        out.insert(unit.name.clone());
                    }
                }
            }
        }
        if !resolved_by_module {
            for unit in &model.units {
                if matches_pattern(&unit.name, entry) || matches_pattern(&unit.path, entry) {
                    out.insert(unit.name.clone());
                }
            }
        }
    }
    for stereotype in &module.contract.forbid {
        for definition in &spec.stereotype {
            if matches_pattern(&definition.name, stereotype) {
                for unit in &model.units {
                    if stereotype_matches(unit, definition) {
                        out.insert(unit.name.clone());
                    }
                }
            }
        }
    }
    out
}

fn unit_matches_module(unit: &Unit, module: &Module) -> bool {
    module
        .matches
        .units
        .iter()
        .any(|rule| matches_pattern(&unit.name, rule))
        || module
            .matches
            .names
            .iter()
            .any(|rule| matches_pattern(&unit.name, rule))
        || module
            .matches
            .paths
            .iter()
            .any(|rule| matches_pattern(&unit.path, rule))
        || module
            .submodules
            .iter()
            .any(|sub| unit_matches_module(unit, sub))
}

fn stereotype_matches(unit: &Unit, stereotype: &Stereotype) -> bool {
    stereotype
        .match_set
        .units
        .iter()
        .any(|rule| matches_pattern(&unit.name, rule))
        || stereotype
            .match_set
            .names
            .iter()
            .any(|rule| matches_pattern(&unit.name, rule))
        || stereotype
            .match_set
            .paths
            .iter()
            .any(|rule| matches_pattern(&unit.path, rule))
}

/// The role a scan-sourced node carries (roles-views US 01): the node is
/// marked iff the model's `roles` map holds a key equal to the node's name
/// under the `.`/`::` separator equivalence `depgraph::parent_matches` uses
/// (rust keys at `kit`, c# at `Shop::Api`, the node stands for unit
/// `Shop.Api`). Exact identity — no subtree containment, no fold — so a node
/// the map does not address keeps its exact unmarked bytes, and presentation
/// stays a pure lookup like `inspect structural`'s marker.
fn scan_role_marker(model: &Model, name: &str) -> Option<Role> {
    let normalized = name.replace("::", ".");
    model
        .roles
        .iter()
        .find(|(key, _)| key.replace("::", ".") == normalized)
        .map(|(_, role)| *role)
}

/// Which tiers a scan render draws (`--tiers`). The selection is a render
/// selector only — no spec, model or config field stores it — and the
/// default (flag absent) is units-only, whose bytes are byte-identical to
/// the historical scan surface (workplan D1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierSelection {
    pub units: bool,
    pub modules: bool,
    /// The file-granular tier (design record D8): renders in SPEC source
    /// mode only, under `--module`/`--modules-dir` territory scope.
    pub files: bool,
    /// The tier names the flag literally listed. The source-law gate reads
    /// this, because the unit tier renders implicitly and `files` joins the
    /// vocabulary as a spec-source value beside the scan-source `units`
    /// and `modules`.
    pub named: BTreeSet<String>,
}

impl TierSelection {
    /// The supported tiers, stated in every unknown- or duplicate-value
    /// `--tiers` usage error (D7, D8).
    const SUPPORTED: &'static str = "units, modules, files";

    /// The default selection: the unit render alone.
    pub fn units_only() -> Self {
        Self {
            units: true,
            modules: false,
            files: false,
            named: BTreeSet::new(),
        }
    }

    /// Parse a comma-separated tier list: every entry names a supported
    /// tier, duplicates are rejected, an empty or blank list is rejected;
    /// every error names the offending value and the supported tiers (D7).
    pub fn parse(spec: &str) -> Result<Self, String> {
        let mut units = false;
        let mut modules = false;
        let mut files = false;
        let mut named = BTreeSet::new();
        for entry in spec.split(',') {
            match entry {
                "units" | "modules" | "files" => {}
                other => {
                    return Err(format!(
                        "invalid value {other:?} for --tiers — supported tiers: {}",
                        Self::SUPPORTED
                    ));
                }
            }
            if (entry == "units" && units) || (entry == "modules" && modules) || (entry == "files" && files)
            {
                return Err(format!(
                    "duplicate value {entry:?} for --tiers — supported tiers: {}",
                    Self::SUPPORTED
                ));
            }
            match entry {
                "units" => units = true,
                "modules" => modules = true,
                _ => files = true,
            }
            named.insert(entry.to_string());
        }
        // `modules` renders on top of the unchanged unit render: the unit
        // tier is never dropped, `--tiers modules` equals `--tiers
        // units,modules` (D1).
        Ok(Self {
            units: true,
            modules,
            files,
            named,
        })
    }
}

/// The `--level` selector (workplan level_views scenario 01): the render
/// granularity of the level fold on the scan surface. A level is a read of
/// existing model facts — the `module_edges` rows, the unit paths and the
/// file paths — never a stored object: the fold projects every row onto a
/// bucket at the render seam and the accounting block reads the same rows
/// ([`LevelProjection`]). Structural derivation only (engine boundary):
/// levels read paths, no vocabulary joins the fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelSelection {
    /// The owning unit — [`crate::archspec::depgraph::unit_of_path`], the
    /// shipped directory-ownership rule read backwards from `units`.
    Unit,
    /// Directory-depth fold: the first `k` path segments (k ≥ 1) relative
    /// to the TREE root (the top-level fold-root law). `dir:0` is not
    /// selectable: the fit-set floor excludes the degenerate whole-tree
    /// fold (plan CLI partial).
    Dir(usize),
    /// The module tier through the shipped fold —
    /// [`crate::archspec::depgraph::projection_node`] + `fold_named`, the
    /// same projection `depgraph modules` renders.
    Module,
    /// The raw module-edge endpoints, aggregated on the distinct ordered
    /// bucket pair.
    File,
}

impl LevelSelection {
    /// The supported values, stated in every unknown-value `--level` usage
    /// error — mirroring the `--tiers` refusal shape.
    const SUPPORTED: &'static str = "unit, dir:<k>, module, file";

    pub fn parse(spec: &str) -> Result<Self, String> {
        if let Some(rest) = spec.strip_prefix("dir:") {
            return match rest.parse::<usize>() {
                Ok(k) if k >= 1 => Ok(Self::Dir(k)),
                _ => Err(format!(
                    "invalid value {spec:?} for --level — dir:<k> needs an integer k of at least 1"
                )),
            };
        }
        match spec {
            "unit" => Ok(Self::Unit),
            "module" => Ok(Self::Module),
            "file" => Ok(Self::File),
            other => Err(format!(
                "invalid value {other:?} for --level — supported levels: {}",
                Self::SUPPORTED
            )),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            Self::Unit => "unit".to_string(),
            Self::Dir(k) => format!("dir:{k}"),
            Self::Module => "module".to_string(),
            Self::File => "file".to_string(),
        }
    }
}

/// The named bucket class for files at the tree root (decision-check
/// MINOR-9): ONE class, identical at every level, spelled with the same
/// word depgraph's fold states for a path that folds below any named unit
/// (its `ROOT_NODE`) — never `.` — so it cannot collide with a root unit's
/// path label.
const LEVEL_ROOT_BUCKET: &str = "root";

/// Whether an endpoint spells a file at the tree root: no `/` separator
/// and no unit attributes it (`unit_of_path` answers none — a bare unit
/// name does answer, so unit spellings are never root files). The law
/// fires at every level (MINOR-9's identity law).
fn is_root_file(model: &Model, path: &str) -> bool {
    !path.contains('/') && crate::archspec::depgraph::unit_of_path(model, path).is_none()
}

/// The bucket one endpoint addresses at one level — the ONE bucket rule
/// every fold consumer reads. `module_node` is the precomputed
/// [`crate::archspec::depgraph::projection_node`] fold (computed once per
/// projection, never per row).
fn level_bucket(
    model: &Model,
    path: &str,
    level: LevelSelection,
    module_node: fn(&str) -> String,
) -> String {
    if is_root_file(model, path) {
        return LEVEL_ROOT_BUCKET.to_string();
    }
    match level {
        LevelSelection::File => path.to_string(),
        LevelSelection::Module => {
            crate::archspec::depgraph::fold_named(model, module_node, path)
        }
        LevelSelection::Unit => crate::archspec::depgraph::unit_of_path(model, path)
            .unwrap_or_else(|| path.to_string()),
        LevelSelection::Dir(k) => {
            // The fold reads the endpoint's own spelling: slash paths fold
            // on `/` segments; a `::` spelling folds on its `::` segments.
            // The last segment names the addressed file or module, so the
            // directory parts are the segments before it; a single-segment
            // spelling IS the directory at the tree root (a unit-root
            // module), not a file in it.
            let parts: Vec<&str> = if path.contains('/') {
                path.split('/').collect()
            } else if path.contains("::") {
                path.split("::").collect()
            } else {
                vec![path]
            };
            let dirs: &[&str] = if parts.len() > 1 {
                &parts[..parts.len() - 1]
            } else {
                &parts[..1]
            };
            let take = k.min(dirs.len());
            dirs[..take].join("/")
        }
    }
}

/// The level projection: the ONE fold site feeding every format arm (the
/// one-assembly-site law of visibility phase 2 scenario 05). It aggregates
/// the model's `module_edges` rows onto buckets (an aggregated edge is a
/// distinct ordered bucket pair, a raw row whose two buckets coincide is a
/// SELF row — PoC §2 rules) and folds the derived async rows through the
/// shipped [`crate::archspec::depgraph::project_async_rows`] machinery, so
/// folded-away rows stay counted (decision-check MAJOR-4) and the note
/// keeps its shipped bytes.
/// The declared-vs-projected unit delta carried by a level projection
/// (workplan level_views scenario 03): `(model_only, projection_only)`
/// distinct ordered unit pairs — the named type keeps the struct field
/// readable (the clippy type-complexity law).
pub type UnitDelta = (BTreeSet<(String, String)>, BTreeSet<(String, String)>);

pub struct LevelProjection {
    pub level: LevelSelection,
    /// Every bucket the fold drew or placed — the render's node set.
    pub nodes: BTreeSet<String>,
    /// Aggregated cross-bucket edges with their raw-row counts.
    pub cross: BTreeMap<(String, String), usize>,
    /// Intra-bucket (SELF) raw rows per bucket.
    pub self_rows: BTreeMap<String, usize>,
    /// Cross-bucket async rows placed by the projection, drawn with their
    /// event labels; self-folded and unplaced rows are not drawn — the
    /// ledger counts them.
    pub async_drawn: Vec<(String, String, String)>,
    /// The shipped fold-away note for the async fold, if any (bytes owned
    /// by `depgraph::async_fold_note`).
    pub async_note: Option<String>,
    pub raw_rows: usize,
    pub drawn_rows: usize,
    pub self_total: usize,
    /// The completeness census row this projection restates (MAJOR-6):
    /// `Some` exactly when the artefact's gap census is non-empty.
    pub gap_census: Option<String>,
    /// The declared-vs-projected unit census (workplan level_views scenario
    /// 03, decision-check MAJOR-5): `Some` exactly at level `unit`, holding
    /// the distinct ordered pairs the model declares (its `edges`) that the
    /// fold draws nowhere (model-only) and the pairs the fold draws that the
    /// model does not declare (projection-only). The projected unit graph
    /// and the model's declared unit-edge set are TWO graphs; this holds
    /// their delta, named. UX-ONLY advisory data — never a model edge,
    /// never a render-body byte, never an rc effect (plan §Relations;
    /// MAJOR-13 — the exit-1 gap law stays SCAN's own). Structural
    /// derivation only: the delta compares unit names, no vocabulary
    /// joins it.
    pub unit_delta: Option<UnitDelta>,
}

impl LevelProjection {
    /// The raw rows per drawn edge label: one aggregated edge row carries
    /// its raw-row count (scenario line "each aggregated cross-bucket edge
    /// draws as one row with its raw-row count").
    pub fn row_count_label(count: usize) -> String {
        if count == 1 {
            "1 row".to_string()
        } else {
            format!("{count} rows")
        }
    }
}

pub fn level_projection(
    model: &Model,
    level: LevelSelection,
    gap_census: Option<String>,
) -> LevelProjection {
    let module_node = crate::archspec::depgraph::projection_node(model);
    let bucket = |path: &str| level_bucket(model, path, level, module_node);

    let mut nodes: BTreeSet<String> = BTreeSet::new();
    let mut cross: BTreeMap<(String, String), usize> = BTreeMap::new();
    let mut self_rows: BTreeMap<String, usize> = BTreeMap::new();
    for edge in &model.module_edges {
        let from = bucket(&edge.from);
        let to = bucket(&edge.to);
        nodes.insert(from.clone());
        nodes.insert(to.clone());
        if from == to {
            *self_rows.entry(from).or_insert(0) += 1;
        } else {
            *cross.entry((from, to)).or_insert(0) += 1;
        }
    }

    // The async fold rides the shipped projection. Its placement scan
    // consults the fold-input paths (the `soft_structure` membership plus
    // the module-edge endpoints — the same set `project_async_rows`
    // gathers), so the graph it places against carries every bucket those
    // paths fold to, exactly as a depgraph view's node set does.
    let mut placement_nodes = nodes.clone();
    for unit_paths in model.soft_structure.values() {
        for path in unit_paths {
            placement_nodes.insert(bucket(path));
        }
    }
    let placement_graph = crate::archspec::depgraph::ModuleGraph {
        nodes: placement_nodes,
        ..Default::default()
    };
    let identities: Vec<(String, String, String)> =
        crate::archspec::depgraph::async_render_rows(
            crate::archspec::depgraph::derived_render_rows(model),
        )
        .into_iter()
        .collect();
    let async_raw = identities.len();
    let async_projection = crate::archspec::depgraph::project_async_rows(
        model,
        &placement_graph,
        |path| Some(bucket(path)),
        identities,
    );
    let mut async_drawn: Vec<(String, String, String)> = Vec::new();
    for (from, to, label) in &async_projection.rows {
        if from != to {
            nodes.insert(from.clone());
            nodes.insert(to.clone());
            async_drawn.push((from.clone(), to.clone(), label.clone()));
        } else {
            // A pair that folded to one bucket is an intra-bucket raw row:
            // it draws no arrow (the shipped self-edge law) and rides the
            // SELF tally of the accounting block (MAJOR-4 — folded rows
            // never vanish from the ledger).
            nodes.insert(from.clone());
            *self_rows.entry(from.clone()).or_insert(0) += 1;
        }
    }

    let drawn_rows = cross.values().sum::<usize>() + async_drawn.len();
    let raw_rows = model.module_edges.len() + async_raw;
    // The declared-vs-projected unit census (workplan level_views scenario
    // 03, MAJOR-5): the fold's projected unit pairs and the model's declared
    // unit edges are two graphs — read together, never silently merged. The
    // projected side is the DRAWN unit graph (every aggregated cross-bucket
    // pair plus every placed async pair; a self-folded row draws no arrow,
    // so it is a pair neither graph draws). The declared side is the
    // distinct ordered pairs of `edges` — a declared self-pair included: the
    // render draws no self-arrows, so the census names it model-only rather
    // than dropping it. Advisory data on the projection: the ledger above
    // stays exact untouched (drawn + self = raw).
    let unit_delta = (level == LevelSelection::Unit).then(|| {
        let projected: BTreeSet<(String, String)> = cross
            .keys()
            .cloned()
            .chain(
                async_drawn
                    .iter()
                    .map(|(from, to, _)| (from.clone(), to.clone())),
            )
            .collect();
        let declared: BTreeSet<(String, String)> = model
            .edges
            .iter()
            .map(|edge| (edge.from.clone(), edge.to.clone()))
            .collect();
        (
            declared.difference(&projected).cloned().collect(),
            projected.difference(&declared).cloned().collect(),
        )
    });
    LevelProjection {
        level,
        nodes,
        cross,
        self_rows,
        async_note: crate::archspec::depgraph::async_fold_note(&async_projection),
        async_drawn,
        raw_rows,
        drawn_rows,
        self_total: raw_rows - drawn_rows,
        gap_census,
        unit_delta,
    }
}

/// One class of the declared-vs-projected unit census (workplan level_views
/// scenario 03, decision-check MAJOR-5): the count first, then every pair
/// spelled `from→to` in sorted order — or the word `none` when the class is
/// empty, the `self rows: none` voice. ADVISORY-LOUD: the row rides STDERR
/// like every shipped descriptive row, names its class exactly, and never
/// enters the render body or changes rc (MAJOR-13 — a descriptive delta
/// never reddens a healthy render; the exit-1 law stays SCAN's own).
fn unit_delta_line(class: &str, pairs: &BTreeSet<(String, String)>) -> String {
    if pairs.is_empty() {
        return format!("unit delta {class}: none");
    }
    let rows: Vec<String> = pairs
        .iter()
        .map(|(from, to)| format!("{from}→{to}"))
        .collect();
    format!("unit delta {class}: {} — {}", pairs.len(), rows.join(", "))
}

/// The accounting block the level render owes its run (STDERR, advisory —
/// descriptive rows never enter the render body): the descriptive-only row
/// (MINOR-11), the per-bucket SELF counts, the exact ledger row
/// `drawn + self = raw` or its zero-row replacement (MINOR-10), the
/// completeness restatement when the gap census is non-empty (MAJOR-6),
/// and the shipped async fold-away note when the fold placed one fewer
/// row than it received (MAJOR-4 — the folded rows ride the `self` term,
/// never a dropped count).
pub fn accounting_lines(projection: &LevelProjection) -> Vec<String> {
    let level = projection.level.as_str();
    let mut lines = vec![
        "note: descriptive only — verify gates the module tier".to_string(),
    ];
    if let Some(note) = &projection.async_note {
        lines.push(note.clone());
    }
    if projection.raw_rows == 0 {
        lines.push(format!("empty model: 0 raw rows at level {level}"));
    } else {
        if projection.self_rows.is_empty() {
            lines.push("self rows: none".to_string());
        } else {
            let rows: Vec<String> = projection
                .self_rows
                .iter()
                .map(|(bucket, count)| format!("{bucket} ×{count}"))
                .collect();
            lines.push(format!("self rows: {}", rows.join(", ")));
        }
        lines.push(format!(
            "accounting exact: {} drawn + {} self = {} raw rows at level {level}",
            projection.drawn_rows, projection.self_total, projection.raw_rows
        ));
    }
    // The declared-vs-projected unit census (workplan level_views scenario
    // 03, MAJOR-5): at level `unit` the render owes its run the delta
    // between the two unit graphs it reads — the model's declared unit edges
    // and the projected unit pairs. ADVISORY-LOUD on STDERR, rc unchanged
    // (MAJOR-13): a healthy rc=0 render stays rc=0 with the delta named.
    // The row never enters the model, never the render body, never an exit
    // code; the exit-1 gap law stays on SCAN.
    if let Some((model_only, projection_only)) = &projection.unit_delta {
        lines.push(unit_delta_line("model-only", model_only));
        lines.push(unit_delta_line("projection-only", projection_only));
    }
    if let Some(census) = &projection.gap_census {
        lines.push(format!("completeness restated: {census}"));
    }
    lines
}

/// The level render arms: one flat graph over the projection's buckets —
/// one node row per bucket, one labelled edge row per aggregated
/// cross-bucket pair carrying its raw-row count, and the placed async
/// rows dashed and event-labelled under the same `--layer` selection as
/// every shipped arm. The body shape is each format's own grammar; the
/// inventory comes from the ONE fold site ([`level_projection`]).
fn render_level_mermaid(projection: &LevelProjection, layer: LayerSelection) -> String {
    let mut out = String::from("graph TD\n");
    for node in &projection.nodes {
        let _ = writeln!(out, "  {}", mermaid_node(node));
    }
    if layer.draws_structural() {
        for ((from, to), count) in &projection.cross {
            let _ = writeln!(
                out,
                "  {}",
                mermaid_counted_edge(from, to, &LevelProjection::row_count_label(*count))
            );
        }
    }
    if layer.draws_async() {
        for (from, to, event) in &projection.async_drawn {
            let _ = writeln!(out, "  {}", mermaid_labeled_edge(from, to, event));
        }
    }
    out
}

fn render_level_plantuml(projection: &LevelProjection, layer: LayerSelection) -> String {
    let mut out = String::from("@startuml\n");
    for node in &projection.nodes {
        let _ = writeln!(out, "  {}", plantuml_quoted_component(node));
    }
    if layer.draws_structural() {
        for ((from, to), count) in &projection.cross {
            let _ = writeln!(
                out,
                "{}",
                plantuml_counted_edge(from, to, &LevelProjection::row_count_label(*count))
            );
        }
    }
    if layer.draws_async() {
        for (from, to, event) in &projection.async_drawn {
            let _ = writeln!(out, "{}", plantuml_labeled_edge(from, to, event));
        }
    }
    let _ = writeln!(out, "@enduml");
    out
}

fn render_level_dot(projection: &LevelProjection, layer: LayerSelection) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "// archspec diagram — layer: {}", layer.as_str());
    let _ = writeln!(out, "// archspec diagram — level: {}", projection.level.as_str());
    let _ = writeln!(out, "digraph archspec {{");
    let _ = writeln!(out, "  graph [rankdir=TB, splines=true, overlap=false];");
    let _ = writeln!(out, "  node [shape=box, style=filled, fontname=\"Helvetica\"];");
    for node in &projection.nodes {
        let _ = writeln!(out, "  {}", dot_node(node, node, COLOR_NODE_UNIT, None));
    }
    if layer.draws_structural() {
        for ((from, to), count) in &projection.cross {
            let _ = writeln!(
                out,
                "  {}",
                dot_edge(
                    from,
                    to,
                    &[
                        dot_attr("color", COLOR_EDGE_DIRECT),
                        dot_attr("label", &LevelProjection::row_count_label(*count)),
                    ]
                )
            );
        }
    }
    if layer.draws_async() {
        for (from, to, event) in &projection.async_drawn {
            let _ = writeln!(
                out,
                "  {}",
                dot_edge(
                    from,
                    to,
                    &[
                        dot_attr("color", COLOR_EDGE_ASYNC),
                        dot_attr("style", "dashed"),
                        dot_attr("label", event),
                    ]
                )
            );
        }
    }
    let _ = writeln!(out, "}}");
    out
}

fn render_level_svg(projection: &LevelProjection, layer: LayerSelection) -> String {
    let nodes: Vec<SvgNode> = projection
        .nodes
        .iter()
        .map(|node| SvgNode {
            id: node.clone(),
            label: node.clone(),
            fill: COLOR_NODE_UNIT.to_string(),
            stroke: COLOR_EDGE_DIRECT.to_string(),
        })
        .collect();
    let mut edges: Vec<SvgEdge> = Vec::new();
    if layer.draws_structural() {
        for ((from, to), count) in &projection.cross {
            edges.push(SvgEdge {
                from: from.clone(),
                to: to.clone(),
                stroke: COLOR_EDGE_DIRECT.to_string(),
                dashed: false,
                label: Some(LevelProjection::row_count_label(*count)),
            });
        }
    }
    if layer.draws_async() {
        for (from, to, event) in &projection.async_drawn {
            edges.push(SvgEdge {
                from: from.clone(),
                to: to.clone(),
                stroke: COLOR_EDGE_ASYNC.to_string(),
                dashed: true,
                label: Some(event.clone()),
            });
        }
    }
    hand_svg("diagram", &nodes, &edges, &[])
}

/// The level viewer document (workplan level_views scenario 04): the bucket
/// graph plus ONE drill-down pane per bucket that carries structural
/// internal rows. A pane re-projects its rows BUCKET-RELATIVE through the
/// ONE fold site and draws the FINEST relative fold that fits the ceilings
/// — global directory depth is the forbidden shape (the FOLD-ROOT LAW).
/// A pane over ceiling at its finest relative granularity speaks the named
/// in-pane refusal row and restates the completeness census row; it never
/// draws an empty pane and never dumps raw rows (TERMINATION LAW). Pane
/// node ids are PANE-SCOPED — the bucket path plus the relative label, the
/// global path — so identical relative labels in distinct panes stay
/// distinct nodes (MINOR-16). The document states one whole-document total
/// row exactly once per emit: top-cross + Σ pane-internal = raw (MINOR-17).
/// A pane nests under its bucket node as SCOPE, never ownership. The
/// ceilings are the arm's own seam: sibling 05 re-sizes what a viewer
/// document may embed through the same pair, and the pane walk answers to
/// whatever it is passed.
fn render_level_viewer(
    model: &Model,
    projection: &LevelProjection,
    layer: LayerSelection,
    ceilings: (usize, usize),
) -> String {
    let nodes: Vec<ViewerNode> = projection
        .nodes
        .iter()
        .map(|node| ViewerNode {
            id: node.clone(),
            label: node.clone(),
            kind: "module",
            fill: COLOR_NODE_UNIT.to_string(),
            stroke: "#333".to_string(),
            role: None,
        })
        .collect();
    let mut edges: Vec<ViewerEdge> = Vec::new();
    if layer.draws_structural() {
        for ((from, to), count) in &projection.cross {
            edges.push(ViewerEdge {
                from: from.clone(),
                to: to.clone(),
                layer: "direct",
                violates: false,
                label: Some(LevelProjection::row_count_label(*count)),
            });
        }
    }
    if layer.draws_async() {
        for (from, to, event) in &projection.async_drawn {
            edges.push(ViewerEdge {
                from: from.clone(),
                to: to.clone(),
                layer: "async",
                violates: false,
                label: Some(event.clone()),
            });
        }
    }
    // The panes: one drill-down per bucket with structural internal rows,
    // in the projection's own bucket order (a BTreeMap walk — deterministic
    // by construction).
    let mut panes: Vec<viewer::ViewerPane> = Vec::new();
    for bucket in projection.self_rows.keys() {
        let Some(sub) = pane_submodel(model, projection, bucket) else {
            continue;
        };
        let (fits, first_over) = level_pane_walk(&sub, layer, ceilings.0, ceilings.1);
        let refusal = if fits.is_empty() {
            first_over.map(|over| pane_refusal_row(bucket, over))
        } else {
            None
        };
        // The drawn level: the FINEST fitting relative fold. A bucket whose
        // internal rows are all intra-file (every fold keeps cross at 0
        // without any fold crossing a ceiling) draws the file fold — one
        // node per relative path, no arrows — never an empty pane.
        let drawn = if refusal.is_some() {
            None
        } else {
            Some(fits.last().copied().unwrap_or(LevelSelection::File))
        };
        let mut pane_nodes: Vec<ViewerNode> = Vec::new();
        let mut pane_edges: Vec<ViewerEdge> = Vec::new();
        if let Some(level) = drawn {
            let pane_projection = level_projection(&sub, level, None);
            for label in &pane_projection.nodes {
                pane_nodes.push(ViewerNode {
                    id: format!("{bucket}/{label}"),
                    label: label.clone(),
                    kind: "module",
                    fill: COLOR_NODE_UNIT.to_string(),
                    stroke: "#333".to_string(),
                    role: None,
                });
            }
            if layer.draws_structural() {
                for ((from, to), count) in &pane_projection.cross {
                    pane_edges.push(ViewerEdge {
                        from: format!("{bucket}/{from}"),
                        to: format!("{bucket}/{to}"),
                        layer: "direct",
                        violates: false,
                        label: Some(LevelProjection::row_count_label(*count)),
                    });
                }
            }
        }
        let census = match &refusal {
            Some(_) => projection
                .gap_census
                .as_ref()
                .map(|census| format!("completeness restated: {census}")),
            None => None,
        };
        panes.push(viewer::ViewerPane {
            bucket: bucket.clone(),
            level: drawn.map(|level| level.as_str()),
            nodes: pane_nodes,
            edges: pane_edges,
            refusal,
            census,
        });
    }
    // The whole-document total row (MINOR-17): the top graph's drawn rows
    // plus the buckets' internal rows equal the raw rows — the same ledger
    // identity [`accounting_lines`] states on stderr, stated once inside
    // the document. Σ pane-internal = `self_total` by construction: the
    // panes re-project exactly the rows the fold counted as self rows.
    let total = format!(
        "document total: {} top-cross + {} pane-internal = {} raw rows",
        projection.drawn_rows, projection.self_total, projection.raw_rows
    );
    viewer::viewer_document_with_panes(&nodes, &edges, layer.as_str(), &panes, Some(&total))
}

pub fn render_scan_mermaid(
    model: &Model,
    governing: Option<&Spec>,
    tiers: &TierSelection,
    layer: LayerSelection,
    level: Option<&LevelProjection>,
) -> String {
    if let Some(projection) = level {
        return render_level_mermaid(projection, layer);
    }
    let graph = build_scan_graph(model, governing, tiers);
    let mut out = String::from("graph TD\n");
    if !graph.project_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph project");
        for node in &graph.project_nodes {
            let line = match scan_role_marker(model, node) {
                Some(role) => mermaid_node_marked(node, &format!(" [{}]", role.as_str())),
                None => mermaid_node(node),
            };
            let _ = writeln!(out, "    {line}");
        }
        if layer.draws_structural() {
            for (from, to) in &graph.edges {
                let _ = writeln!(out, "    {}", mermaid_edge(from, to));
            }
            for (from, to) in &graph.violating {
                let _ = writeln!(
                    out,
                    "    {}",
                    mermaid_labeled_edge(from, to, "violates spec")
                );
            }
        }
        // The derived async rows render dashed and event-labelled after the
        // direct and violating arrows (workplan archspec_async_depends,
        // N2/D8/follow-up 6; scenario 08: short label); an empty set renders
        // no bytes. The `--layer` selector stands this loop down for
        // `direct` (workplan visibility_phase2 scenario 05).
        if layer.draws_async() {
            for (from, to, event) in &graph.async_arrows {
                let _ = writeln!(out, "    {}", mermaid_labeled_edge(from, to, event));
            }
        }
        let _ = writeln!(out, "  end");
    }
    if !graph.external_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph external");
        for node in &graph.external_nodes {
            let line = match scan_role_marker(model, node) {
                Some(role) => mermaid_node_marked(node, &format!(" [{}]", role.as_str())),
                None => mermaid_node(node),
            };
            let _ = writeln!(out, "    {line}");
        }
        let _ = writeln!(out, "  end");
        if layer.draws_structural() {
            for (from, to) in &graph.external_edges {
                let _ = writeln!(out, "  {}", mermaid_edge(from, to));
            }
        }
    }
    if !graph.module_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph modules");
        for (identity, label) in &graph.module_nodes {
            let _ = writeln!(out, "    {}", mermaid_module_node(identity, label));
        }
        if layer.draws_structural() {
            for (from, to, violates) in &graph.module_arrows {
                if *violates {
                    let _ = writeln!(
                        out,
                        "    {}",
                        mermaid_module_labeled_edge(from, to, "violates spec")
                    );
                } else {
                    let _ = writeln!(out, "    {}", mermaid_module_edge(from, to));
                }
            }
        }
        let _ = writeln!(out, "  end");
    }
    out
}

pub fn render_scan_plantuml(
    model: &Model,
    governing: Option<&Spec>,
    tiers: &TierSelection,
    layer: LayerSelection,
    level: Option<&LevelProjection>,
) -> String {
    if let Some(projection) = level {
        return render_level_plantuml(projection, layer);
    }
    let graph = build_scan_graph(model, governing, tiers);
    let mut out = String::from("@startuml\n");
    if !graph.project_nodes.is_empty() {
        let _ = writeln!(out, "package project {{");
        for node in &graph.project_nodes {
            let line = match scan_role_marker(model, node) {
                Some(role) => plantuml_component_marked(node, role.as_str()),
                None => plantuml_component(node),
            };
            let _ = writeln!(out, "  {line}");
        }
        if layer.draws_structural() {
            for (from, to) in &graph.edges {
                let _ = writeln!(out, "{}", plantuml_edge(from, to));
            }
            for (from, to) in &graph.violating {
                let _ = writeln!(
                    out,
                    "{}",
                    plantuml_violation_edge(from, to, "violates spec")
                );
            }
        }
        // PlantUML parity with the mermaid scan arm (follow-up 6): the same
        // derived async rows, the same order, dashed labelled grammar. The
        // `--layer` selector stands this loop down for `direct` (scenario 05).
        if layer.draws_async() {
            for (from, to, event) in &graph.async_arrows {
                let _ = writeln!(out, "{}", plantuml_labeled_edge(from, to, event));
            }
        }
        let _ = writeln!(out, "}}");
    }
    if !graph.external_nodes.is_empty() {
        let _ = writeln!(out, "package external {{");
        for node in &graph.external_nodes {
            let line = match scan_role_marker(model, node) {
                Some(role) => plantuml_component_marked(node, role.as_str()),
                None => plantuml_component(node),
            };
            let _ = writeln!(out, "  {line}");
        }
        let _ = writeln!(out, "}}");
        if layer.draws_structural() {
            for (from, to) in &graph.external_edges {
                let _ = writeln!(out, "{}", plantuml_edge(from, to));
            }
        }
    }
    if !graph.module_nodes.is_empty() {
        let _ = writeln!(out, "package modules {{");
        for label in graph.module_nodes.values() {
            let _ = writeln!(out, "  {}", plantuml_quoted_component(label));
        }
        if layer.draws_structural() {
            for (from, to, violates) in &graph.module_arrows {
                let from_label = graph
                    .module_nodes
                    .get(from)
                    .map(String::as_str)
                    .unwrap_or(from.as_str());
                let to_label = graph
                    .module_nodes
                    .get(to)
                    .map(String::as_str)
                    .unwrap_or(to.as_str());
                if *violates {
                    let _ = writeln!(
                        out,
                        "{}",
                        plantuml_quoted_violation_edge(from_label, to_label, "violates spec")
                    );
                } else {
                    let _ = writeln!(out, r#""{from_label}" --> "{to_label}""#);
                }
            }
        }
        let _ = writeln!(out, "}}");
    }
    let _ = writeln!(out, "@enduml");
    out
}

/// Render the scan artefact as graphviz `dot` source (`--format dot`,
/// scenario 05 stage 2): the same `build_scan_graph` assembly the text
/// renderers consume — one assembly site, every format — with the shared
/// type-color vocabulary drawn as per-class attributes (node fills by
/// node-kind, role strokes, solid/dashed edge strokes) and the GENERATED
/// legend cluster whenever the `both` selection draws both edge families.
pub fn render_scan_dot(
    model: &Model,
    governing: Option<&Spec>,
    tiers: &TierSelection,
    layer: LayerSelection,
    level: Option<&LevelProjection>,
) -> String {
    if let Some(projection) = level {
        return render_level_dot(projection, layer);
    }
    let graph = build_scan_graph(model, governing, tiers);
    let (structural, marks, async_drawn) = scan_layer_counts(&graph, layer);
    let legend = legend_rows(layer == LayerSelection::Both, structural, async_drawn, marks);
    let mut out = String::new();
    let _ = writeln!(out, "// archspec diagram — layer: {}", layer.as_str());
    let _ = writeln!(out, "digraph archspec {{");
    let _ = writeln!(out, "  graph [rankdir=TB, splines=true, overlap=false];");
    let _ = writeln!(out, "  node [shape=box, style=filled, fontname=\"Helvetica\"];");
    if !graph.project_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph cluster_project {{");
        let _ = writeln!(out, "    label=\"project\";");
        for node in &graph.project_nodes {
            let _ = writeln!(
                out,
                "    {}",
                dot_node(node, node, COLOR_NODE_UNIT, scan_dot_stroke(model, node))
            );
        }
        let _ = writeln!(out, "  }}");
    }
    if !graph.external_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph cluster_external {{");
        let _ = writeln!(out, "    label=\"external\";");
        for node in &graph.external_nodes {
            let _ = writeln!(
                out,
                "    {}",
                dot_node(node, node, COLOR_NODE_EXTERNAL, scan_dot_stroke(model, node))
            );
        }
        let _ = writeln!(out, "  }}");
    }
    if !graph.module_nodes.is_empty() {
        let _ = writeln!(out, "  subgraph cluster_modules {{");
        let _ = writeln!(out, "    label=\"modules\";");
        for (identity, label) in &graph.module_nodes {
            let _ = writeln!(
                out,
                "    {}",
                dot_node(identity, label, COLOR_NODE_UNIT, None)
            );
        }
        let _ = writeln!(out, "  }}");
    }
    if layer.draws_structural() {
        for (from, to) in graph.edges.iter().chain(graph.external_edges.iter()) {
            let _ = writeln!(
                out,
                "  {}",
                dot_edge(from, to, &[dot_attr("color", COLOR_EDGE_DIRECT)])
            );
        }
        for (from, to) in &graph.violating {
            let _ = writeln!(
                out,
                "  {}",
                dot_edge(
                    from,
                    to,
                    &[
                        dot_attr("color", COLOR_EDGE_VIOLATION),
                        dot_attr("style", "dashed"),
                        dot_attr("label", "violates spec"),
                    ]
                )
            );
        }
        for (from, to, violates) in &graph.module_arrows {
            let mut attrs = vec![dot_attr(
                "color",
                if *violates {
                    COLOR_EDGE_VIOLATION
                } else {
                    COLOR_EDGE_DIRECT
                },
            )];
            if *violates {
                attrs.push(dot_attr("style", "dashed"));
                attrs.push(dot_attr("label", "violates spec"));
            }
            let _ = writeln!(out, "  {}", dot_edge(from, to, &attrs));
        }
    }
    if layer.draws_async() {
        for (from, to, event) in &graph.async_arrows {
            let _ = writeln!(
                out,
                "  {}",
                dot_edge(
                    from,
                    to,
                    &[
                        dot_attr("color", COLOR_EDGE_ASYNC),
                        dot_attr("style", "dashed"),
                        dot_attr("label", event),
                    ]
                )
            );
        }
    }
    crate::archspec::depgraph::write_dot_legend(&mut out, legend.as_deref());
    let _ = writeln!(out, "}}");
    out
}

/// Render the scan artefact as a hand-emitted SVG (`--format svg`): the
/// same `build_scan_graph` inventory as every other format — one `<g class=
/// "node">` per assembled node, one `<path class="edge">` per drawn arrow —
/// plus the generated legend group the dot arm draws.
pub fn render_scan_svg(
    model: &Model,
    governing: Option<&Spec>,
    tiers: &TierSelection,
    layer: LayerSelection,
    level: Option<&LevelProjection>,
) -> String {
    if let Some(projection) = level {
        return render_level_svg(projection, layer);
    }
    let graph = build_scan_graph(model, governing, tiers);
    let (structural, marks, async_drawn) = scan_layer_counts(&graph, layer);
    let legend = legend_rows(layer == LayerSelection::Both, structural, async_drawn, marks);
    let mut nodes: Vec<SvgNode> = Vec::new();
    for node in &graph.project_nodes {
        nodes.push(SvgNode {
            id: node.clone(),
            label: node.clone(),
            fill: COLOR_NODE_UNIT.to_string(),
            stroke: scan_dot_stroke(model, node)
                .unwrap_or(COLOR_EDGE_DIRECT)
                .to_string(),
        });
    }
    for node in &graph.external_nodes {
        nodes.push(SvgNode {
            id: node.clone(),
            label: node.clone(),
            fill: COLOR_NODE_EXTERNAL.to_string(),
            stroke: scan_dot_stroke(model, node)
                .unwrap_or(COLOR_EDGE_DIRECT)
                .to_string(),
        });
    }
    for (identity, label) in &graph.module_nodes {
        nodes.push(SvgNode {
            id: identity.clone(),
            label: label.clone(),
            fill: COLOR_NODE_UNIT.to_string(),
            stroke: COLOR_EDGE_DIRECT.to_string(),
        });
    }
    let mut edges: Vec<SvgEdge> = Vec::new();
    if layer.draws_structural() {
        for (from, to) in graph.edges.iter().chain(graph.external_edges.iter()) {
            edges.push(SvgEdge {
                from: from.clone(),
                to: to.clone(),
                stroke: COLOR_EDGE_DIRECT.to_string(),
                dashed: false,
                label: None,
            });
        }
        for (from, to) in &graph.violating {
            edges.push(SvgEdge {
                from: from.clone(),
                to: to.clone(),
                stroke: COLOR_EDGE_VIOLATION.to_string(),
                dashed: true,
                label: Some("violates spec".to_string()),
            });
        }
        for (from, to, violates) in &graph.module_arrows {
            edges.push(SvgEdge {
                from: from.clone(),
                to: to.clone(),
                stroke: if *violates {
                    COLOR_EDGE_VIOLATION.to_string()
                } else {
                    COLOR_EDGE_DIRECT.to_string()
                },
                dashed: *violates,
                label: violates.then(|| "violates spec".to_string()),
            });
        }
    }
    if layer.draws_async() {
        for (from, to, event) in &graph.async_arrows {
            edges.push(SvgEdge {
                from: from.clone(),
                to: to.clone(),
                stroke: COLOR_EDGE_ASYNC.to_string(),
                dashed: true,
                label: Some(event.clone()),
            });
        }
    }
    hand_svg("diagram", &nodes, &edges, legend.as_deref().unwrap_or(&[]))
}

/// Render the scan artefact as the single-file interactive viewer
/// (`--format viewer`, workplan visibility_phase2 scenario 06): the same
/// `build_scan_graph` inventory as every other format — one `ViewerNode`
/// per assembled node, one `ViewerEdge` per drawn arrow, the scenario-05
/// layer selection applied at the same loops — handed to the ONE document
/// assembler in `viewer`. The emitted file is self-contained: hand-written
/// SVG plus one static script block, zero vendored bytes, zero external
/// references.
pub fn render_scan_viewer(
    model: &Model,
    governing: Option<&Spec>,
    tiers: &TierSelection,
    layer: LayerSelection,
    level: Option<&LevelProjection>,
    ceilings: (usize, usize),
) -> String {
    if let Some(projection) = level {
        return render_level_viewer(model, projection, layer, ceilings);
    }
    let graph = build_scan_graph(model, governing, tiers);
    let mut nodes: Vec<ViewerNode> = Vec::new();
    for node in &graph.project_nodes {
        nodes.push(ViewerNode {
            id: node.clone(),
            label: node.clone(),
            kind: "unit",
            fill: COLOR_NODE_UNIT.to_string(),
            stroke: scan_dot_stroke(model, node)
                .unwrap_or("#333")
                .to_string(),
            role: scan_role_marker(model, node).map(|role| role.as_str()),
        });
    }
    for (identity, label) in &graph.module_nodes {
        nodes.push(ViewerNode {
            id: identity.clone(),
            label: label.clone(),
            kind: "module",
            fill: COLOR_NODE_UNIT.to_string(),
            stroke: "#333".to_string(),
            role: None,
        });
    }
    for node in &graph.external_nodes {
        nodes.push(ViewerNode {
            id: node.clone(),
            label: node.clone(),
            kind: "external",
            fill: COLOR_NODE_EXTERNAL.to_string(),
            stroke: "#333".to_string(),
            role: None,
        });
    }
    let mut edges: Vec<ViewerEdge> = Vec::new();
    if layer.draws_structural() {
        for (from, to) in graph.edges.iter().chain(graph.external_edges.iter()) {
            edges.push(ViewerEdge {
                from: from.clone(),
                to: to.clone(),
                layer: "direct",
                violates: false,
                label: None,
            });
        }
        for (from, to) in &graph.violating {
            edges.push(ViewerEdge {
                from: from.clone(),
                to: to.clone(),
                layer: "direct",
                violates: true,
                label: None,
            });
        }
        for (from, to, violates) in &graph.module_arrows {
            edges.push(ViewerEdge {
                from: from.clone(),
                to: to.clone(),
                layer: "direct",
                violates: *violates,
                label: None,
            });
        }
    }
    if layer.draws_async() {
        for (from, to, event) in &graph.async_arrows {
            edges.push(ViewerEdge {
                from: from.clone(),
                to: to.clone(),
                layer: "async",
                violates: false,
                label: Some(event.clone()),
            });
        }
    }
    // Declared-dependency arrows (viewer-usability scenario 01, finding 1):
    // one `declared` row per `(unit, external)` pair the unit's own manifest
    // DECLARES — generic manifest-declaration fact, no vocabulary (engine
    // boundary law). Name resolution reuses the ONE crate-name law the
    // ledger already uses (`depgraph::ledger_name_key`, the same dash/
    // underscore equivalence the rust scan applies when it resolves a
    // dependency name to a member); the join compares BOTH sides under the
    // key, so a hyphen spelling in a manifest and an underscore spelling in
    // the `external` list meet without a second normalization. A dependency
    // naming a project unit is already a structural row (the rust driver
    // folds it into `edges`); a dependency naming no node draws nothing;
    // self-pairs draw nothing. Viewer-only: no other format arm, no model
    // tier and no scan byte moves. The class rides no `--layer` selection —
    // declaration is neither a direct code arrow nor an async row — and the
    // emitted document's `declared deps` checkbox toggles it client-side.
    let mut declared: BTreeSet<(String, String)> = BTreeSet::new();
    for (unit, info) in &model.unit_manifests {
        if !graph.project_nodes.contains(unit) {
            continue;
        }
        for dependency in &info.dependencies {
            let key = crate::archspec::depgraph::ledger_name_key(dependency);
            let Some(target) = graph
                .external_nodes
                .iter()
                .find(|external| crate::archspec::depgraph::ledger_name_key(external) == key)
            else {
                continue;
            };
            if target != unit {
                declared.insert((unit.clone(), target.clone()));
            }
        }
    }
    for (from, to) in declared {
        edges.push(ViewerEdge {
            from,
            to,
            layer: "declared",
            violates: false,
            label: None,
        });
    }
    viewer::viewer_document(&nodes, &edges, layer.as_str())
}

/// The drawn-family counts behind the legend for every scan-source format
/// arm: solid structural arrows, dashed red marks, dashed async rows —
/// each zero when its layer is selected off.
fn scan_layer_counts(
    graph: &ScanGraph,
    layer: LayerSelection,
) -> (usize, usize, usize) {
    let (structural, marks) = if layer.draws_structural() {
        let marks = graph.violating.len()
            + graph
                .module_arrows
                .iter()
                .filter(|(_, _, violates)| *violates)
                .count();
        let structural = graph.edges.len()
            + graph.external_edges.len()
            + graph
                .module_arrows
                .iter()
                .filter(|(_, _, violates)| !violates)
                .count();
        (structural, marks)
    } else {
        (0, 0)
    };
    let async_drawn = if layer.draws_async() {
        graph.async_arrows.len()
    } else {
        0
    };
    (structural, marks, async_drawn)
}

/// The role stroke a roles-marked node carries in the dot/SVG arms: the
/// same marker `scan_role_marker` states for the mermaid/plantuml label
/// suffix; `None` leaves the node unmarked.
fn scan_dot_stroke(model: &Model, name: &str) -> Option<&'static str> {
    match scan_role_marker(model, name) {
        Some(Role::Facade) => Some(COLOR_ROLE_FACADE),
        Some(Role::Composition) => Some(COLOR_ROLE_COMPOSITION),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mermaid body carrying exactly `edges` arrow rows over at least
    /// `min_chars` characters: comment padding adds no arrows, so the two
    /// ceiling arms stay independently addressable.
    fn body_with(edges: usize, min_chars: usize) -> String {
        let mut body = String::from("graph TD\n");
        for i in 0..edges {
            let _ = writeln!(body, "  u{i} --> v{i}");
        }
        while body.chars().count() < min_chars {
            let _ = writeln!(body, "  %% padding to the requested size for the character arm");
        }
        body
    }

    #[test]
    fn the_ceiling_statement_agrees_with_the_arithmetic() {
        let statement = MERMAID_CEILING_STATEMENT.replace(',', "");
        let mut words = statement.split_whitespace();
        let edges = words
            .next()
            .expect("the statement opens with the edge count")
            .parse::<usize>()
            .expect("edge count");
        // The char count is the word beside `characters`.
        let chars = words
            .next_back()
            .and_then(|last| {
                assert_eq!(last, "characters");
                words.next_back().map(str::to_string)
            })
            .expect("the statement names the char count beside the word characters");
        assert_eq!(edges, MERMAID_EDGE_CEILING);
        assert_eq!(chars.parse::<usize>().expect("char count"), MERMAID_CHAR_CEILING);
    }

    #[test]
    fn the_edge_ceiling_refuses_exactly_above_five_hundred_rows() {
        assert!(mermaid_ceiling_error(&body_with(500, 0), "--format dot").is_none());
        let refusal = mermaid_ceiling_error(&body_with(501, 0), "--format dot")
            .expect("one edge row over the ceiling refuses");
        assert!(refusal.contains("measured 501 edges and "), "{refusal}");
        assert!(
            refusal.contains(MERMAID_CEILING_STATEMENT),
            "the refusal states the ceilings in the registered wording: {refusal}"
        );
        assert!(refusal.ends_with("--format dot"), "the named clear-arm echoes: {refusal}");
    }

    #[test]
    fn the_char_ceiling_refuses_exactly_above_fifty_thousand_characters() {
        // Zero edge rows: only the character arm can fire here.
        let at = body_with(0, MERMAID_CHAR_CEILING);
        let at = &at[..at.char_indices().nth(MERMAID_CHAR_CEILING - 1).unwrap().0 + 1];
        assert_eq!(at.chars().count(), MERMAID_CHAR_CEILING);
        assert!(mermaid_ceiling_error(at, "--format dot").is_none());
        let refusal = mermaid_ceiling_error(&format!("{at}\n"), "--format dot")
            .expect("one character over the ceiling refuses");
        assert!(refusal.contains("measured 0 edges and 50001 characters"), "{refusal}");
    }

    #[test]
    fn dashed_labelled_rows_count_as_edges() {
        // The labelled async arm renders `-.->`, never a plain `-->`: an
        // async-heavy render must be counted, not measured edge-blind.
        let mut body = String::from("graph TD\n");
        for i in 0..MERMAID_EDGE_CEILING {
            let _ = writeln!(body, "  a{i} -.->|evt{i}| b{i}");
        }
        assert!(mermaid_ceiling_error(&body, "--format dot").is_none());
        let _ = writeln!(body, "  a -.->|evt| b");
        assert!(mermaid_ceiling_error(body.as_str(), "--format dot").is_some());
    }

    /// Done-when for the boundary arm (scenario 05): the gate is inclusive
    /// AT the ceiling — a document of exactly the constant's bytes stands
    /// and emits, one byte over speaks the named refusal. The at-boundary
    /// emit is proven here on the gate seam itself (a byte-exact document
    /// is not constructible from model rows), beside the fixture-scale
    /// cells of `tests/level_viewer_bytes.rs`. The scope search is owned
    /// by the CLI cells — a padded string measures, it does not emit.
    #[test]
    fn the_byte_ceiling_refuses_exactly_above_the_measured_constant() {
        let model = Model::new("rust");
        let at = "x".repeat(viewer::VIEWER_BYTE_CEILING);
        assert!(
            viewer_byte_ceiling_error(&at, &model, LayerSelection::Both, None, (500, 50_000), None)
                .is_none(),
            "a document of exactly the ceiling's bytes stands at the ceiling and emits"
        );
        let over = format!("{at}x");
        let refusal = viewer_byte_ceiling_error(
            &over,
            &model,
            LayerSelection::Both,
            None,
            (500, 50_000),
            None,
        )
        .expect("one byte over the ceiling refuses");
        assert!(
            refusal.contains(&format!(
                "exceeds the emitted-byte ceiling of {} bytes — measured {} bytes",
                viewer::VIEWER_BYTE_CEILING,
                viewer::VIEWER_BYTE_CEILING + 1
            )),
            "the refusal states the ceiling and the measured bytes: {refusal}"
        );
        assert!(
            refusal.contains("narrow with: archspec scan <bucket-path> --output <artefact>"),
            "the refusal carries the two-step narrowing grammar: {refusal}"
        );
    }
}
