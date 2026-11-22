//! The single-file interactive viewer emitter (`archspec diagram --format
//! viewer`, workplan visibility_phase2 scenario 06): ONE self-contained
//! HTML document over the assembled scan graph — hand-written SVG markup
//! plus ONE hand-written static script block.
//!
//! Laws this module carries (the scenario §The design restatements):
//! - zero-vendored-JS: every emitted byte is generated here; no third-party
//!   library bytes enter the payload and the emitted file carries no
//!   external asset reference (no `http(s)://`, no `<link`, no `src=`).
//! - XSS posture: every model-derived string passes `html_escape` exactly
//!   once at its single entry site into the document; the runtime script
//!   owns no model data and writes the DOM only through `textContent` —
//!   the emitted bytes contain `innerHTML` zero times.
//! - drift law: the caller feeds the ONE `build_scan_graph` inventory like
//!   every other format does, so the viewer can drift from layout, never
//!   from data.

use rust_arch_test_kit::render::{
    xml_escape, COLOR_EDGE_ASYNC, COLOR_EDGE_DIRECT, COLOR_EDGE_VIOLATION, COLOR_NODE_EXTERNAL,
    COLOR_NODE_UNIT, COLOR_ROLE_COMPOSITION, COLOR_ROLE_FACADE,
};
use std::fmt::Write;

/// The shared escape site: `render::xml_escape` under the viewer-local name,
/// so every model string enters the document through exactly one spelling
/// and the entry sites are greppable.
pub fn html_escape(text: &str) -> String {
    xml_escape(text)
}

/// One viewer node: identity (the adjacency key, carried as `data-name`),
/// visible label, kind class (`unit` | `module` | `external`), the fill and
/// stroke colors (the type-color vocabulary the legend states) and the role
/// mark name when the roles map marks the node.
pub struct ViewerNode {
    pub id: String,
    pub label: String,
    pub kind: &'static str,
    pub fill: String,
    pub stroke: String,
    pub role: Option<&'static str>,
}

/// One viewer edge: endpoints are node ids of the same call, `layer` is the
/// scenario-05 vocabulary (`direct` | `async` — a violation stays a
/// `direct`-layer row drawn with the violation class) extended by
/// `declared` (the viewer-usability scenario-01 class: a manifest-declared
/// unit -> external leg, drawn as its own dotted class and toggled by the
/// `declared deps` checkbox, never by the layer radios), and `label` carries
/// the async event string when the row is an async one.
pub struct ViewerEdge {
    pub from: String,
    pub to: String,
    pub layer: &'static str,
    pub violates: bool,
    pub label: Option<String>,
}

/// Layout constants: node box size, vertical gap, the three kind columns.
/// Deterministic by construction — same inventory, same bytes.
const NODE_W: usize = 170;
const NODE_H: usize = 26;
const V_GAP: usize = 10;
const Y0: usize = 40;
const COL_X: [usize; 3] = [24, 246, 470];

/// The longest label drawn in full; longer labels truncate at the raw
/// string's char boundary set before escaping, so no entity can be cut.
const LABEL_MAX: usize = 26;

/// The declared-class stroke: thin dotted grey (viewer-usability scenario 01,
/// finding 1). ONE implementation — the stylesheet rule and the legend swatch
/// both read this constant, the same law as the shared color set above.
const COLOR_EDGE_DECLARED: &str = "#999";

/// The emitted-byte ceiling of the single-file viewer document (workplan
/// level_views scenario 05, decision-check §BLOCKER-3 fix + §Round-2
/// MINOR-18): the same law family as the render ceilings — a MEASURED
/// constant, never a restated-by-hand number. Floor: ≥ 1 MB (MINOR-18);
/// the value is the binary 1 MB, so it stands above the floor under both
/// MB readings. The pane-fold witnesses that sized it — the two corpus-
/// scale witness trees emitted through the pane fold at landing — measure
/// 0.02 MB and 0.28 MB, both under the floor (the `__log__` entry of the
/// landing names the byte counts); a document over the ceiling ships
/// nothing: the emit refuses in the usage class naming the largest scope
/// that fits ([`crate::archspec::diagram::viewer_byte_ceiling_error`]).
/// Budget law: panes embed PRECOMPUTED folds, never raw rows, and the
/// refusal rows a document embeds charge the document's own total — the
/// emitted bytes ARE the budget, measured once at the emit seam, so an
/// embedded refusal can never re-trigger the ceiling (no feedback loop).
pub const VIEWER_BYTE_CEILING: usize = 1_048_576;

fn truncate_label(label: &str) -> String {
    let chars: Vec<char> = label.chars().collect();
    if chars.len() <= LABEL_MAX {
        return label.to_string();
    }
    let mut out: String = chars[..LABEL_MAX].iter().collect();
    out.push('…');
    out
}

fn edge_class(edge: &ViewerEdge) -> &'static str {
    if edge.violates {
        "edge violation"
    } else if edge.layer == "async" {
        "edge async"
    } else if edge.layer == "declared" {
        "edge declared"
    } else {
        "edge direct"
    }
}

/// The static CSS: classes carry the edge colors generated from the shared
/// color constants, so one constant set colors the graph and the legend.
fn viewer_css() -> String {
    format!(
        "body{{font-family:sans-serif;margin:0;background:#fafafa}}\n\
         #graph{{display:block;margin:56px 12px 12px 12px}}\n\
         .node rect{{stroke:#333;cursor:pointer}}\n\
         .node text{{font:11px sans-serif;pointer-events:none;fill:#111}}\n\
         .edge{{stroke-width:1.5}}\n\
         .edge.direct{{stroke:{direct}}}\n\
         .edge.async{{stroke:{async_};stroke-dasharray:6}}\n\
         .edge.violation{{stroke:{violation};stroke-dasharray:4}}\n\
         .edge.declared{{stroke:{declared};stroke-width:1;stroke-dasharray:1,3}}\n\
         .edge.lit{{stroke-width:3}}\n\
         .node.dim{{opacity:.12}}\n\
         .node.sel rect{{stroke-width:3;stroke:#d00}}\n\
         .node:focus{{outline:none}}\n\
         .node:focus-visible rect{{stroke:#06c;stroke-width:3}}\n\
         #controls,#legend,#info{{position:fixed;background:#fff;border:1px solid #bbb;padding:8px;font:12px sans-serif;z-index:2}}\n\
         #controls{{top:8px;left:8px}}#legend{{top:8px;right:8px}}#info{{bottom:8px;left:8px;max-width:44vw}}",
        direct = COLOR_EDGE_DIRECT,
        async_ = COLOR_EDGE_ASYNC,
        violation = COLOR_EDGE_VIOLATION,
        declared = COLOR_EDGE_DECLARED,
    )
}

/// The static script: click-to-highlight over the DOM inventory, the layer
/// radios (direct | indirect | both — the scenario-05 selector's words), the
/// independent `declared deps` checkbox (viewer-usability scenario 01,
/// finding 1 — it toggles ONLY the declared class; the radios' structural
/// semantics are unchanged), selection clear on background click and Escape
/// (finding 2), dependency-free wheel-zoom + drag-pan over the SVG viewBox
/// (finding 3 — no pan/zoom state ever enters the emitted bytes), and the
/// info panel built with `textContent` only. The script holds no model data
/// — it reads the escaped data attributes the markup carries.
fn viewer_js() -> &'static str {
    r#"(function () {
"use strict";
var graph = document.getElementById("graph");
var info = document.getElementById("info");
var INFO_DEFAULT = "click a node to highlight its connections";
var nodes = graph.querySelectorAll("g.node");
var edges = graph.querySelectorAll("line.edge");
var radios = document.getElementsByName("layer");
var declaredBox = document.getElementById("declared-deps");
var selected = null;
function layerValue() {
    for (var i = 0; i < radios.length; i++) { if (radios[i].checked) { return radios[i].value; } }
    return "both";
}
function edgeInLayer(edge, mode) {
    var asyncRow = edge.getAttribute("data-layer") === "async";
    if (mode === "direct") { return !asyncRow; }
    if (mode === "indirect") { return asyncRow; }
    return true;
}
function isDeclared(edge) { return edge.getAttribute("data-layer") === "declared"; }
function declaredOn() { return declaredBox === null || declaredBox.checked; }
function edgeShown(edge, mode) {
    if (isDeclared(edge)) { return declaredOn(); }
    return edgeInLayer(edge, mode);
}
function touches(a, b) {
    for (var i = 0; i < edges.length; i++) {
        var e = edges[i];
        if ((e.getAttribute("data-from") === a && e.getAttribute("data-to") === b) ||
            (e.getAttribute("data-from") === b && e.getAttribute("data-to") === a)) { return true; }
    }
    return false;
}
function apply() {
    var mode = layerValue();
    for (var i = 0; i < edges.length; i++) {
        var e = edges[i];
        var on = edgeShown(e, mode);
        if (selected !== null) {
            on = on && (e.getAttribute("data-from") === selected || e.getAttribute("data-to") === selected);
        }
        e.style.display = on ? "" : "none";
        e.classList.toggle("lit", on && selected !== null);
    }
    for (var j = 0; j < nodes.length; j++) {
        var g = nodes[j];
        var name = g.getAttribute("data-name");
        var adj = selected === null || name === selected || touches(selected, name);
        g.classList.toggle("dim", !adj);
        g.classList.toggle("sel", selected === name);
    }
}
function showInfo(name) {
    while (info.firstChild) { info.removeChild(info.firstChild); }
    var head = document.createElement("b");
    head.textContent = name;
    info.append("node ", head, document.createElement("br"));
    var mode = layerValue();
    for (var i = 0; i < edges.length; i++) {
        var e = edges[i];
        var from = e.getAttribute("data-from");
        var to = e.getAttribute("data-to");
        if (from !== name && to !== name) { continue; }
        if (!edgeShown(e, mode)) { continue; }
        var row = document.createElement("p");
        var text = from + " -> " + to;
        var event = e.getAttribute("data-event");
        if (event !== null) { text = text + " (" + event + ")"; }
        if (isDeclared(e)) { text = text + " (declared)"; }
        row.textContent = text;
        info.append(row);
    }
}
function selectNode(name) {
    selected = name;
    apply();
    showInfo(selected);
}
function clearSelection() {
    selected = null;
    apply();
    while (info.firstChild) { info.removeChild(info.firstChild); }
    info.append(INFO_DEFAULT);
}
for (var i = 0; i < nodes.length; i++) {
    nodes[i].addEventListener("click", function (event) {
        event.stopPropagation();
        selectNode(this.getAttribute("data-name"));
    });
    nodes[i].addEventListener("keydown", function (event) {
        if (event.key === "Enter" || event.key === " ") {
            event.preventDefault();
            event.stopPropagation();
            selectNode(this.getAttribute("data-name"));
        }
    });
}
graph.addEventListener("click", function () { clearSelection(); });
document.addEventListener("keydown", function (event) {
    if (event.key === "Escape") { clearSelection(); }
});
for (var r = 0; r < radios.length; r++) {
    radios[r].addEventListener("change", function () {
        apply();
        if (selected !== null) { showInfo(selected); }
    });
}
if (declaredBox !== null) {
    declaredBox.addEventListener("change", function () {
        apply();
        if (selected !== null) { showInfo(selected); }
    });
}
var vb = graph.getAttribute("viewBox").split(/[\s,]+/).map(Number);
var base = vb.slice(0);
var MIN_ZOOM = 0.2;
var MAX_ZOOM = 8;
function applyView() {
    graph.setAttribute("viewBox", vb[0] + " " + vb[1] + " " + vb[2] + " " + vb[3]);
}
function viewPoint(clientX, clientY) {
    var rect = graph.getBoundingClientRect();
    return [vb[0] + (clientX - rect.left) * vb[2] / rect.width,
            vb[1] + (clientY - rect.top) * vb[3] / rect.height];
}
function zoomAt(clientX, clientY, factor) {
    var width = vb[2] * factor;
    if (width < base[2] * MIN_ZOOM || width > base[2] * MAX_ZOOM) { return; }
    var p = viewPoint(clientX, clientY);
    vb[0] = p[0] - (p[0] - vb[0]) * factor;
    vb[1] = p[1] - (p[1] - vb[1]) * factor;
    vb[2] = width;
    vb[3] = vb[3] * factor;
    applyView();
}
graph.addEventListener("wheel", function (event) {
    event.preventDefault();
    zoomAt(event.clientX, event.clientY, event.deltaY < 0 ? 0.9 : 1.1);
}, { passive: false });
var drag = null;
graph.addEventListener("pointerdown", function (event) {
    drag = [event.clientX, event.clientY];
});
graph.addEventListener("pointermove", function (event) {
    if (drag === null) { return; }
    var rect = graph.getBoundingClientRect();
    vb[0] = vb[0] - (event.clientX - drag[0]) * vb[2] / rect.width;
    vb[1] = vb[1] - (event.clientY - drag[1]) * vb[3] / rect.height;
    drag = [event.clientX, event.clientY];
    applyView();
});
graph.addEventListener("pointerup", function () { drag = null; });
graph.addEventListener("pointercancel", function () { drag = null; });
apply();
})();"#
}

/// Column index per kind: units left, modules middle, externals right.
fn kind_column(kind: &str) -> usize {
    match kind {
        "module" => 1,
        "external" => 2,
        _ => 0,
    }
}

/// Edge endpoints at the NEAREST box borders (viewer-usability scenario 01,
/// finding 4 — one implementation for the top-level grid and the pane grid):
/// different columns join the facing vertical borders at the boxes' vertical
/// mid-lines, byte-identical to the shipped cross-column geometry; same-
/// column boxes join the facing horizontal borders at the column centre
/// line. Deterministic by construction — same coordinates, same bytes.
fn border_endpoints(from: (usize, usize), to: (usize, usize)) -> (usize, usize, usize, usize) {
    let (fx, fy) = from;
    let (tx, ty) = to;
    if fx == tx {
        let cx = fx + NODE_W / 2;
        if ty >= fy {
            (cx, fy + NODE_H, cx, ty)
        } else {
            (cx, fy, cx, ty + NODE_H)
        }
    } else if tx > fx {
        (fx + NODE_W, fy + NODE_H / 2, tx, ty + NODE_H / 2)
    } else {
        (fx, fy + NODE_H / 2, tx + NODE_W, ty + NODE_H / 2)
    }
}

/// One legend row: a swatch (filled rect for node kinds, stroked line for
/// edge classes) and the generated count text. The rows state the emitted
/// inventory only — the numbers are computed from the vectors passed in.
fn legend_swatch_rect(fill: &str, stroke: &str) -> String {
    format!(
        "<svg width=\"14\" height=\"12\"><rect width=\"14\" height=\"12\" fill=\"{fill}\" stroke=\"{stroke}\"></rect></svg>"
    )
}

fn legend_swatch_line(stroke: &str, dashed: bool) -> String {
    let dash = if dashed { " stroke-dasharray=\"4\"" } else { "" };
    format!(
        "<svg width=\"14\" height=\"12\"><line x1=\"0\" y1=\"6\" x2=\"14\" y2=\"6\" stroke=\"{stroke}\" stroke-width=\"2\"{dash}></line></svg>"
    )
}

/// One drill-down pane (workplan level_views scenario 04): the recursive
/// region nested under one bucket node. `bucket` is the top-level bucket
/// label the pane states SCOPE for — never ownership (plan §Relations).
/// A DRAWN pane carries `level` (the finest fitting bucket-relative fold)
/// plus its pane-scoped nodes and edges; a REFUSAL pane carries `refusal`
/// (the named in-pane refusal row) and the restated completeness `census`
/// row instead of rows — never an empty pane, never a raw dump. The two
/// arms are mutually exclusive by construction.
pub struct ViewerPane {
    pub bucket: String,
    pub level: Option<String>,
    pub nodes: Vec<ViewerNode>,
    pub edges: Vec<ViewerEdge>,
    pub refusal: Option<String>,
    pub census: Option<String>,
}

/// Assemble the ONE self-contained document. `layer_label` is the
/// build-time selection name (`both` | `direct` | `indirect`) whose radio
/// ships checked; the emitted edge set already carries that selection.
pub fn viewer_document(nodes: &[ViewerNode], edges: &[ViewerEdge], layer_label: &str) -> String {
    viewer_document_with_panes(nodes, edges, layer_label, &[], None)
}

/// The pane bands' geometry (deterministic by construction): pane nodes
/// stand in the two right-hand columns, two per row, below the top graph;
/// a refusal pane is one text row plus the optional census restatement.
const PANE_COLS: [usize; 2] = [COL_X[1], COL_X[2]];
const PANE_GAP: usize = 24;

fn pane_height(pane: &ViewerPane) -> usize {
    let rows = if pane.refusal.is_some() {
        1 + usize::from(pane.census.is_some())
    } else {
        pane.nodes.len().div_ceil(2)
    };
    rows * (NODE_H + V_GAP) + V_GAP
}

/// Assemble the document with recursive panes (workplan level_views
/// scenario 04). Each pane nests INSIDE its bucket node's `<g>` — DOM
/// nesting states scope, never ownership. `total_row` is the
/// whole-document total, emitted exactly once as `#total` when present;
/// the no-pane, no-total call (`viewer_document`) stays byte-identical to
/// the shipped document.
pub fn viewer_document_with_panes(
    nodes: &[ViewerNode],
    edges: &[ViewerEdge],
    layer_label: &str,
    panes: &[ViewerPane],
    total_row: Option<&str>,
) -> String {
    let mut coords: Vec<(usize, usize)> = Vec::with_capacity(nodes.len());
    let mut heights = [0usize; 3];
    for node in nodes {
        let col = kind_column(node.kind);
        coords.push((COL_X[col], Y0 + heights[col] * (NODE_H + V_GAP)));
        heights[col] += 1;
    }
    let top_h = heights.iter().max().copied().unwrap_or(1) * (NODE_H + V_GAP) + Y0 + 20;
    let pane_h: usize = panes.iter().map(pane_height).sum();
    let canvas_h = if pane_h > 0 {
        top_h + PANE_GAP + pane_h
    } else {
        top_h
    };

    let mut out = String::new();
    out.push_str("<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    out.push_str("<title>archspec viewer</title>\n<style>\n");
    out.push_str(&viewer_css());
    out.push_str("\n</style>\n</head>\n<body>\n");

    // Controls: the layer toggle, riding the scenario-05 selector words,
    // plus the ONE independent `declared deps` checkbox (viewer-usability
    // scenario 01, finding 1) — default ON, it toggles only the declared
    // class and never touches the radios' structural semantics.
    out.push_str("<div id=\"controls\"><b>layers</b><br>");
    for option in ["both", "direct", "indirect"] {
        let checked = if option == layer_label { " checked" } else { "" };
        let _ = writeln!(
            out,
            "<label><input type=\"radio\" name=\"layer\" value=\"{option}\"{checked}> {option}</label><br>"
        );
    }
    out.push_str("<label><input type=\"checkbox\" id=\"declared-deps\" checked> declared deps</label><br>");
    out.push_str("</div>\n");

    // The whole-document total row (workplan level_views scenario 04,
    // MINOR-17): ONE row per level-viewer emit, stating the document's
    // completeness census — top-cross + Σ pane-internal = raw. Absent (no
    // byte) on every non-level document.
    if let Some(total) = total_row {
        out.push_str("<div id=\"total\" style=\"position:fixed;bottom:8px;right:8px;background:#fff;border:1px solid #bbb;padding:8px;font:12px sans-serif;z-index:2\">");
        out.push_str(&html_escape(total));
        out.push_str("</div>\n");
    }

    // Legend: the type-color vocabulary generated from the emitted inventory.
    out.push_str("<div id=\"legend\"><b>legend</b><br>\n");
    let mut counts: Vec<(&'static str, usize)> = Vec::new();
    for (kind, caption) in [
        ("unit", "unit — project unit tier"),
        ("module", "module — module tier"),
        ("external", "external — dependency outside the project"),
    ] {
        let count = nodes.iter().filter(|n| n.kind == kind).count();
        if count > 0 {
            counts.push((kind, count));
            let fill = if kind == "external" {
                COLOR_NODE_EXTERNAL
            } else {
                COLOR_NODE_UNIT
            };
            let _ = writeln!(
                out,
                "{} {} ({count})<br>",
                legend_swatch_rect(fill, "#333"),
                caption
            );
        }
    }
    for (role, color) in [
        ("facade", COLOR_ROLE_FACADE),
        ("composition", COLOR_ROLE_COMPOSITION),
    ] {
        let count = nodes
            .iter()
            .filter(|n| n.role == Some(role))
            .count();
        if count > 0 {
            let _ = writeln!(
                out,
                "{} role mark: {role} — stroke on roles-marked nodes ({count})<br>",
                legend_swatch_line(color, false)
            );
        }
    }
    let direct = edges
        .iter()
        .filter(|e| e.layer == "direct" && !e.violates)
        .count();
    let async_rows = edges.iter().filter(|e| e.layer == "async").count();
    let violating = edges.iter().filter(|e| e.violates).count();
    if direct > 0 {
        let _ = writeln!(
            out,
            "{} direct — solid structural arrow ({direct})<br>",
            legend_swatch_line(COLOR_EDGE_DIRECT, false)
        );
    }
    if async_rows > 0 {
        let _ = writeln!(
            out,
            "{} indirect — dashed event-labelled async row ({async_rows})<br>",
            legend_swatch_line(COLOR_EDGE_ASYNC, true)
        );
    }
    if violating > 0 {
        let _ = writeln!(
            out,
            "{} violation — dashed red arrow, the pair `verify` reports ({violating})<br>",
            legend_swatch_line(COLOR_EDGE_VIOLATION, true)
        );
    }
    let declared_rows = edges.iter().filter(|e| e.layer == "declared").count();
    if declared_rows > 0 {
        let _ = writeln!(
            out,
            "{} declared — thin dotted manifest-declared dependency arrow ({declared_rows})<br>",
            legend_swatch_line(COLOR_EDGE_DECLARED, true)
        );
    }
    out.push_str("click a node to highlight its connections\n</div>\n");

    // The graph: one <line> per drawn edge, one <g> per node — the DOM
    // inventory the cells count.
    let _ = writeln!(
        out,
        "<svg id=\"graph\" viewBox=\"0 0 {} {canvas_h}\">",
        COL_X[2] + NODE_W + 24
    );
    for edge in edges {
        let (Some(from), Some(to)) = (
            nodes.iter().position(|n| n.id == edge.from),
            nodes.iter().position(|n| n.id == edge.to),
        ) else {
            continue;
        };
        let (fx, fy) = coords[from];
        let (tx, ty) = coords[to];
        let (x1, y1, x2, y2) = border_endpoints((fx, fy), (tx, ty));
        let event = match &edge.label {
            Some(event) => format!(" data-event=\"{}\"", html_escape(event)),
            None => String::new(),
        };
        let _ = writeln!(
            out,
            "<line class=\"{cls}\" data-layer=\"{layer}\" data-from=\"{from}\" data-to=\"{to}\"{event} x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\"></line>",
            cls = edge_class(edge),
            layer = edge.layer,
            from = html_escape(&edge.from),
            to = html_escape(&edge.to),
        );
    }
    let mut pane_y = top_h + PANE_GAP;
    for (index, node) in nodes.iter().enumerate() {
        let (x, y) = coords[index];
        let role_attr = match node.role {
            Some(role) => format!(" data-role=\"{role}\""),
            None => String::new(),
        };
        let has_pane = panes.iter().any(|pane| pane.bucket == node.id);
        // The bucket node's `<g>` stays OPEN while its panes emit, so the
        // pane regions nest INSIDE the bucket node — DOM nesting states
        // scope, never ownership (plan §Relations). Without a pane the
        // emitted bytes are the shipped document's, byte for byte.
        let close = if has_pane { "" } else { "</g>" };
        let _ = writeln!(
            out,
            "<g class=\"node {kind}\" data-name=\"{name}\"{role_attr} tabindex=\"0\" role=\"button\" aria-label=\"{label_attr}\"><rect x=\"{x}\" y=\"{y}\" width=\"{NODE_W}\" height=\"{NODE_H}\" rx=\"4\" fill=\"{fill}\" stroke=\"{stroke}\"></rect><text x=\"{}\" y=\"{}\">{label}</text>{close}",
            x + 6,
            y + NODE_H / 2 + 4,
            kind = node.kind,
            name = html_escape(&node.id),
            label_attr = html_escape(&node.id),
            stroke = node.stroke,
            fill = node.fill,
            label = html_escape(&truncate_label(&node.label)),
        );
        for pane in panes.iter().filter(|pane| pane.bucket == node.id) {
            emit_pane(&mut out, pane, pane_y);
            pane_y += pane_height(pane);
        }
        if has_pane {
            out.push_str("</g>\n");
        }
    }
    out.push_str("</svg>\n");
    out.push_str(
        "<div id=\"info\">click a node to highlight its connections</div>\n<script>\n",
    );
    out.push_str(viewer_js());
    out.push_str("\n</script>\n</body>\n</html>\n");
    out
}

/// The pane-local grid: two columns, two nodes per row, deterministic by
/// construction — same pane inventory, same bytes.
fn pane_coord(index: usize, y: usize) -> (usize, usize) {
    (PANE_COLS[index % 2], y + V_GAP + (index / 2) * (NODE_H + V_GAP))
}

/// Emit one pane region, nested where the caller left its bucket node's
/// `<g>` open. A DRAWN pane states its bucket-relative fold level
/// (`data-level`) and draws its pane-scoped nodes and arrows; a REFUSAL
/// pane states the named in-pane refusal row and restates the completeness
/// census row (M6 residue) — never an empty pane, never raw rows. Every
/// model string enters through `html_escape` at its single entry site.
fn emit_pane(out: &mut String, pane: &ViewerPane, y: usize) {
    let (class, level_attr) = match &pane.level {
        Some(level) => (
            "pane",
            format!(" data-level=\"{}\"", html_escape(level)),
        ),
        None => ("pane refusal", String::new()),
    };
    let _ = writeln!(
        out,
        "<g class=\"{class}\" data-bucket=\"{bucket}\"{level_attr}>",
        bucket = html_escape(&pane.bucket),
    );
    if let Some(refusal) = &pane.refusal {
        let _ = writeln!(
            out,
            "<text class=\"pane-refusal\" x=\"{}\" y=\"{}\">{}</text>",
            COL_X[0],
            y + NODE_H,
            html_escape(refusal)
        );
        if let Some(census) = &pane.census {
            let _ = writeln!(
                out,
                "<text class=\"pane-census\" x=\"{}\" y=\"{}\">{}</text>",
                COL_X[0],
                y + NODE_H + NODE_H / 2 + 4,
                html_escape(census)
            );
        }
        out.push_str("</g>\n");
        return;
    }
    for (index, node) in pane.nodes.iter().enumerate() {
        let (x, ny) = pane_coord(index, y);
        let _ = writeln!(
            out,
            "<g class=\"node {kind}\" data-name=\"{name}\" tabindex=\"0\" role=\"button\" aria-label=\"{label_attr}\"><rect x=\"{x}\" y=\"{ny}\" width=\"{NODE_W}\" height=\"{NODE_H}\" rx=\"4\" fill=\"{fill}\" stroke=\"{stroke}\"></rect><text x=\"{}\" y=\"{}\">{label}</text></g>",
            x + 6,
            ny + NODE_H / 2 + 4,
            kind = node.kind,
            name = html_escape(&node.id),
            label_attr = html_escape(&node.id),
            stroke = node.stroke,
            fill = node.fill,
            label = html_escape(&truncate_label(&node.label)),
        );
    }
    for edge in &pane.edges {
        let (Some(from), Some(to)) = (
            pane.nodes.iter().position(|n| n.id == edge.from),
            pane.nodes.iter().position(|n| n.id == edge.to),
        ) else {
            continue;
        };
        let (fx, fy) = pane_coord(from, y);
        let (tx, ty) = pane_coord(to, y);
        let (x1, y1, x2, y2) = border_endpoints((fx, fy), (tx, ty));
        let event = match &edge.label {
            Some(event) => format!(" data-event=\"{}\"", html_escape(event)),
            None => String::new(),
        };
        let _ = writeln!(
            out,
            "<line class=\"{cls}\" data-layer=\"{layer}\" data-from=\"{from}\" data-to=\"{to}\"{event} x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\"></line>",
            cls = edge_class(edge),
            layer = edge.layer,
            from = html_escape(&edge.from),
            to = html_escape(&edge.to),
        );
    }
    out.push_str("</g>\n");
}
