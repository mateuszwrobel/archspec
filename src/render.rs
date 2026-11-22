//! Single owner of diagram formatting primitives: escaping and node/edge
//! syntax for mermaid, plantuml, and markdown table output. Every renderer in
//! the `archspec` binary must build its lines from here so escaping cannot
//! drift between entry points.

use std::collections::BTreeMap;
use std::fmt::Write as _;

/// Escape a name for use inside a quoted identifier.
fn quoted_escape(name: &str) -> String {
    name.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Turn a module name into the mermaid-safe character subset: every character
/// mermaid cannot parse in a bare id (`::`, `.`, `/`, `-`, spaces, …) becomes
/// `_`. Sanitization alone is not injective (`a/b.rs` and `a_b.rs` collapse),
/// so diagram ids come from `mermaid_id`, which adds a hash suffix; the raw
/// name stays lossless in the quoted label `mermaid_node` emits.
pub fn mermaid_escape(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// FNV-1a 64-bit hash, the collision-resistance source for mermaid ids.
fn fnv1a64(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Mermaid-safe id with collision resistance: pure `[A-Za-z0-9_]` names keep
/// their bare form (rust-friendly diagrams stay readable); any name that
/// sanitization would alter gets an 8-hex FNV-1a suffix of the raw name, so
/// `a/b.rs` and `a_b.rs` cannot merge into one node, a declared `A.B -> A_B`
/// edge cannot become a self-loop, and sibling subgraphs cannot duplicate.
/// Two distinct raw names collide only on an FNV-1a 64-bit collision.
fn mermaid_id(name: &str) -> String {
    let sanitized = mermaid_escape(name);
    if sanitized == name {
        return sanitized;
    }
    let hex = format!("{:016x}", fnv1a64(name));
    format!("{sanitized}_{}", &hex[..8])
}

/// Escape a value for a markdown table cell.
pub fn markdown_table_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('|', "\\|")
        .replace('`', "\\`")
}

/// Render a mermaid node line: escaped id, plus a `["name"]` label whenever
/// the mermaid-safe id differs from the raw name (label quotes are escaped).
pub fn mermaid_node(name: &str) -> String {
    let id = mermaid_id(name);
    if id != name {
        format!("{id}[\"{}\"]", quoted_escape(name))
    } else {
        id
    }
}

/// Render a mermaid node line whose label carries a marker suffix (e.g. the
/// ` [facade]` role marker a roles map states for this model path). The id
/// stays the bare `mermaid_id` form, so edge references are unaffected; the
/// marker only reaches the visible label text — a node the model leaves
/// unmarked renders exactly what `mermaid_node` emits.
pub fn mermaid_node_marked(name: &str, marker: &str) -> String {
    format!("{}[\"{}{}\"]", mermaid_id(name), quoted_escape(name), marker)
}

/// Render a mermaid subgraph header: escaped id, plus a `["title"]` whenever
/// the mermaid-safe id differs from the raw name, so folder and unit names
/// with `::`, `.` or `/` stay parseable and readable.
pub fn mermaid_subgraph(name: &str) -> String {
    let id = mermaid_id(name);
    if id != name {
        format!("subgraph {id}[\"{}\"]", quoted_escape(name))
    } else {
        format!("subgraph {id}")
    }
}

/// Render a mermaid subgraph header whose title carries a marker suffix (the
/// role marker a roles map states for this unit path). Like
/// `mermaid_node_marked`, the id stays the bare form and the marker reaches
/// only the visible title — unmarked units keep exactly what
/// `mermaid_subgraph` emits, marker bytes included.
pub fn mermaid_subgraph_marked(name: &str, marker: &str) -> String {
    format!(
        "subgraph {}[\"{}{}\"]",
        mermaid_id(name),
        quoted_escape(name),
        marker
    )
}

/// Render a mermaid edge referencing escaped ids only.
pub fn mermaid_edge(from: &str, to: &str) -> String {
    format!("{} --> {}", mermaid_id(from), mermaid_id(to))
}

/// Render a mermaid violation edge with an inline label.
pub fn mermaid_labeled_edge(from: &str, to: &str, label: &str) -> String {
    format!(
        "{} -.->|{label}| {}",
        mermaid_id(from),
        mermaid_id(to)
    )
}

/// Render a solid mermaid edge with an inline label — the level fold's
/// direct-edge arm (workplan level_views scenario 01): an aggregated
/// cross-bucket edge draws as ONE row carrying its raw-row count, so the
/// count rides the same `-->|label|` grammar the async rows use dashed,
/// ONE implementation per grammar beside [`mermaid_labeled_edge`].
pub fn mermaid_counted_edge(from: &str, to: &str, label: &str) -> String {
    format!(
        "{} -->|{label}| {}",
        mermaid_id(from),
        mermaid_id(to)
    )
}

/// Render a mermaid invisible layout link between escaped ids.
pub fn mermaid_layout_link(from: &str, to: &str) -> String {
    format!(
        "{} ~~~ {}",
        mermaid_id(from),
        mermaid_id(to)
    )
}

/// Render a plantuml component declaration (identifiers stay raw).
pub fn plantuml_component(name: &str) -> String {
    format!("component {name}")
}

/// Render a plantuml component declaration with a quoted, escaped name, for
/// names carrying characters plantuml cannot parse as bare identifiers
/// (`::`, `.`, `/`), mirroring the lossless quoting of `plantuml_entity`.
pub fn plantuml_quoted_component(name: &str) -> String {
    format!("component \"{}\"", quoted_escape(name))
}

/// Render a plantuml component declaration carrying a stereotype marker
/// (the role a roles map states for this model path). The quoted name stays
/// the component's identity — edges quoting the raw name keep binding to the
/// declared component — so the marker reaches the rendered stereotype and
/// never rewrites the endpoint grammar.
pub fn plantuml_quoted_component_marked(name: &str, stereotype: &str) -> String {
    format!(
        "component \"{}\" <<{stereotype}>>",
        quoted_escape(name)
    )
}

/// Render a plantuml entity declaration with a quoted, escaped name.
pub fn plantuml_entity(name: &str) -> String {
    format!("entity \"{}\"", quoted_escape(name))
}

/// Render a plantuml edge between raw identifiers.
pub fn plantuml_edge(from: &str, to: &str) -> String {
    format!("{from} --> {to}")
}

/// Render a plantuml edge between quoted, escaped names.
pub fn plantuml_quoted_edge(from: &str, to: &str) -> String {
    format!(
        "\"{}\" --> \"{}\"",
        quoted_escape(from),
        quoted_escape(to)
    )
}

/// Render a plantuml violation edge (red arrow with a trailing label).
pub fn plantuml_violation_edge(from: &str, to: &str, label: &str) -> String {
    format!("{from} -[#red]-> {to} : {label}")
}

/// Render a plantuml dashed edge with a trailing label — the PlantUML parity
/// of `mermaid_labeled_edge` (workplan archspec_async_depends, follow-up 6):
/// the async renderer draws the same event identity through ONE implementation
/// per grammar, so mermaid and plantuml cannot drift on which edges are async.
pub fn plantuml_labeled_edge(from: &str, to: &str, label: &str) -> String {
    format!("{from} ..> {to} : {label}")
}

/// Render a solid plantuml edge with a trailing label — the PlantUML parity
/// of [`mermaid_counted_edge`] (workplan level_views scenario 01): the level
/// fold's aggregated direct edge draws through ONE implementation per
/// grammar, quoting the endpoint names the way [`plantuml_quoted_edge`]
/// does, so bucket spellings with `/` or `::` stay parseable.
pub fn plantuml_counted_edge(from: &str, to: &str, label: &str) -> String {
    format!(
        "\"{}\" --> \"{}\" : {label}",
        quoted_escape(from),
        quoted_escape(to)
    )
}

/// Mermaid-safe id for a module-tier node: the same sanitization as
/// `mermaid_id`, but the hash input is salted with the tier, so a module
/// node whose identity equals a unit's raw name can never share an id with
/// the unit node — the two tiers cannot collapse into one node (scan-diagram
/// module-tier plan, risk mitigation: id disambiguation is an id-construction
/// detail; identity and label grep-ability are unaffected).
pub fn mermaid_module_id(name: &str) -> String {
    let sanitized = mermaid_escape(name);
    let hex = format!("{:016x}", fnv1a64(&format!("modules:{name}")));
    format!("{sanitized}_{}", &hex[..8])
}

/// Render a module-tier mermaid node line: id from `mermaid_module_id`, the
/// visible label carrying identity and path
/// (`api-billing (Sample::Api::Billing)`) for claim-owned endpoints or the
/// raw path for unowned ones.
pub fn mermaid_module_node(identity: &str, label: &str) -> String {
    format!(
        "{}[\"{}\"]",
        mermaid_module_id(identity),
        quoted_escape(label)
    )
}

/// Render a module-tier mermaid edge referencing module ids only.
pub fn mermaid_module_edge(from: &str, to: &str) -> String {
    format!(
        "{} --> {}",
        mermaid_module_id(from),
        mermaid_module_id(to)
    )
}

/// Render a module-tier mermaid violation edge with an inline label.
pub fn mermaid_module_labeled_edge(from: &str, to: &str, label: &str) -> String {
    format!(
        "{} -.->|{label}| {}",
        mermaid_module_id(from),
        mermaid_module_id(to)
    )
}

/// Render a plantuml violation edge between quoted, escaped names (the
/// module tier declares its components quoted, so the violation grammar
/// needs the same quoting to bind to the declared component).
pub fn plantuml_quoted_violation_edge(from: &str, to: &str, label: &str) -> String {
    format!(
        "\"{}\" -[#red]-> \"{}\" : {label}",
        quoted_escape(from),
        quoted_escape(to)
    )
}

/// Render a plantuml component declaration carrying a stereotype marker on a
/// BARE identifier (the role a roles map states for this model path, in the
/// views that declare components bare — `diagram --source scan`). The edges
/// reference the same bare identifier, so the stereotype never rewrites the
/// endpoint grammar: a node the roles map leaves unaddressed keeps exactly
/// what `plantuml_component` emits.
pub fn plantuml_component_marked(name: &str, stereotype: &str) -> String {
    format!("component {name} <<{stereotype}>>")
}

// ---------------------------------------------------------------------------
// graphviz dot and hand-emitted SVG primitives (workplan visibility_phase2
// scenario 05: the `--format svg` emitter). The dot body is graphviz SOURCE —
// the SVG materializes by piping it through `dot -Tsvg` in a container (no
// vendored binary in the payload, no host `dot` assumption); the hand-emitted
// layout covers the unit tier directly. The type-colors below are the ONE
// vocabulary the legend names, shared by the dot body, the hand-emitted SVG,
// and the legend rows: a class rendered anywhere and a legend entry anywhere
// state the same hex.
// ---------------------------------------------------------------------------

/// Fill color of the unit-family node (driver-open units — `project` for
/// csharp, lib/bin for rust). Presentation, not a verdict.
pub const COLOR_NODE_UNIT: &str = "#dbe7f3";
/// Fill color of the external-family node (third-party crates/packages the
/// model draws in the `external` cluster).
pub const COLOR_NODE_EXTERNAL: &str = "#f0f0f0";
/// Stroke color of a direct (structural) edge.
pub const COLOR_EDGE_DIRECT: &str = "#222222";
/// Stroke color of violating and forbidden-marked edges — the red the
/// plantuml `-[#red]->` grammar already draws.
pub const COLOR_EDGE_VIOLATION: &str = "#cc0000";
/// Stroke color of async (event) edges, dashed on every surface.
pub const COLOR_EDGE_ASYNC: &str = "#0066cc";
/// Stroke color of a node a roles map marks `facade`.
pub const COLOR_ROLE_FACADE: &str = "#b58900";
/// Stroke color of a node a roles map marks `composition`.
pub const COLOR_ROLE_COMPOSITION: &str = "#2e7d32";

/// Escape text for a dot quoted string (id, label or attribute value).
pub fn dot_escape(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Render one dot node declaration carrying its visible label and fill
/// color; a `Some(stroke)` marks the node by role (facade/composition) with
/// that stroke color, `None` leaves the node unmarked.
pub fn dot_node(id: &str, label: &str, fill: &str, stroke: Option<&str>) -> String {
    let quoted_id = dot_escape(id);
    let quoted_label = dot_escape(label);
    match stroke {
        Some(stroke) => format!(
            "\"{quoted_id}\" [label=\"{quoted_label}\", fillcolor=\"{fill}\", color=\"{stroke}\"];"
        ),
        None => format!("\"{quoted_id}\" [label=\"{quoted_label}\", fillcolor=\"{fill}\"];"),
    }
}

/// Render one dot edge between quoted ids with the given rendered
/// attribute entries (empty renders the bare edge).
pub fn dot_edge(from: &str, to: &str, attrs: &[String]) -> String {
    let quoted_from = dot_escape(from);
    let quoted_to = dot_escape(to);
    if attrs.is_empty() {
        format!("\"{quoted_from}\" -> \"{quoted_to}\";")
    } else {
        format!("\"{quoted_from}\" -> \"{quoted_to}\" [{}];", attrs.join(", "))
    }
}

/// Render one dot attribute entry `key="value"` with the value escaped.
pub fn dot_attr(key: &str, value: &str) -> String {
    format!("{key}=\"{}\"", dot_escape(value))
}

/// Escape text for XML/SVG content and attribute values. Every model string
/// enters an SVG through this call — the XSS posture the viewer research
/// adopted for its zero-dependency probe applies to emitted files too.
pub fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// The ONE legend section for the dot and SVG emitters: whenever a `both`
/// render draws BOTH edge families, these rows state the vocabulary and the
/// selector. The rows carry no arrow-glyph bytes (`-->`, `-.->`, `..>`): a
/// legend row can never fake an arrow in a byte scan. The counts are facts
/// of the drawn graph — generated, never hand-maintained. The mermaid and
/// plantuml text surfaces carry NO legend: their `both` bytes are the
/// shipped bytes (the byte-identity law for this slice pins them), and a
/// legend row would move them.
pub struct LegendRow {
    /// The swatch color for the row (edge stroke or node fill class).
    pub stroke: String,
    pub dashed: bool,
    pub text: String,
}

/// Build the legend rows from the drawn graph's own counts: `structural`
/// solid arrows, `async_rows` dashed event-labelled arrows, `marks` dashed
/// red violation/forbidden annotations. `None` (no legend) whenever the
/// render draws a single edge family — the `direct`/`indirect` selections
/// and every single-family `both` render — so the legend fires exactly
/// "whenever `both` draws both edge families".
pub fn legend_rows(
    layer_both: bool,
    structural: usize,
    async_rows: usize,
    marks: usize,
) -> Option<Vec<LegendRow>> {
    if !layer_both || structural == 0 || async_rows == 0 {
        return None;
    }
    let mut rows = vec![LegendRow {
        stroke: COLOR_EDGE_DIRECT.to_string(),
        dashed: false,
        text: format!("solid arrow — structural layer ({structural} arrows)"),
    }];
    rows.push(LegendRow {
        stroke: COLOR_EDGE_ASYNC.to_string(),
        dashed: true,
        text: format!(
            "dashed labelled arrow — async layer, model async rows ({async_rows})"
        ),
    });
    if marks > 0 {
        rows.push(LegendRow {
            stroke: COLOR_EDGE_VIOLATION.to_string(),
            dashed: true,
            text: format!("dashed red arrow — violating or forbidden pair ({marks})"),
        });
    }
    rows.push(LegendRow {
        stroke: "#222222".to_string(),
        dashed: false,
        text: "select one layer with --layer direct|indirect|both".to_string(),
    });
    Some(rows)
}

/// One node of the hand-emitted SVG layout. The caller passes the graph's
/// identity, visible label, fill color (node kind) and stroke color (role
/// mark or the default edge-neutral stroke).
pub struct SvgNode {
    pub id: String,
    pub label: String,
    pub fill: String,
    pub stroke: String,
}

/// One edge of the hand-emitted SVG layout: endpoints are node ids of the
/// same call, the stroke states the edge class and `dashed` separates the
/// async and violation grammar the mermaid surfaces draw dashed.
pub struct SvgEdge {
    pub from: String,
    pub to: String,
    pub stroke: String,
    pub dashed: bool,
    pub label: Option<String>,
}

/// Emit a hand-computed deterministic SVG: nodes in input order on a
/// left-to-right, top-to-bottom grid, edges as straight lines between node
/// centers with an arrowhead, and — whenever `legend` is non-empty — one
/// legend group drawn under the grid. The inventory is the input exactly:
/// one `<g class="node">` per node and one `<path class="edge">` per edge —
/// the node/edge parity cells count these groups against the same graph's
/// mermaid render, so the layout is presentation and the inventory is law.
pub fn hand_svg(
    surface: &str,
    nodes: &[SvgNode],
    edges: &[SvgEdge],
    legend: &[LegendRow],
) -> String {
    let cell_w = 260.0f64;
    let cell_h = 110.0f64;
    let box_w = 220.0f64;
    let box_h = 48.0f64;
    let margin = 20.0f64;
    let cols = (nodes.len() as f64).sqrt().ceil().max(1.0);
    let rows = (nodes.len() as f64 / cols).ceil().max(1.0);
    let legend_h = if legend.is_empty() {
        0.0
    } else {
        30.0 + 22.0 * legend.len() as f64
    };
    let width = margin * 2.0 + cols * cell_w;
    let height = margin * 2.0 + rows * cell_h + if legend.is_empty() { 0.0 } else { legend_h + 20.0 };
    let center = |index: usize| -> (f64, f64) {
        let col = index as f64 % cols;
        let row = (index as f64 / cols).floor();
        (
            margin + col * cell_w + box_w / 2.0 + (cell_w - box_w) / 2.0,
            margin + row * cell_h + box_h / 2.0 + (cell_h - box_h) / 2.0,
        )
    };
    let positions: BTreeMap<&str, (f64, f64)> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), center(index)))
        .collect();
    let mut out = String::new();
    let _ = writeln!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
    );
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width:.0}\" height=\"{height:.0}\" viewBox=\"0 0 {width:.0} {height:.0}\">"
    );
    let _ = writeln!(out, "  <title>archspec {surface}</title>");
    let _ = writeln!(out, "  <desc>hand-emitted layout — nodes and edges exactly as assembled by the graph builder</desc>");
    let _ = writeln!(
        out,
        "  <defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\"><path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"context-stroke\"/></marker></defs>"
    );
    for edge in edges {
        let (Some((from_x, from_y)), Some((to_x, to_y))) = (
            positions.get(edge.from.as_str()).copied(),
            positions.get(edge.to.as_str()).copied(),
        ) else {
            continue;
        };
        let dash = if edge.dashed {
            " stroke-dasharray=\"6 4\""
        } else {
            ""
        };
        let _ = writeln!(
            out,
            "  <path class=\"edge\" d=\"M {from_x:.1} {from_y:.1} L {to_x:.1} {to_y:.1}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.4\"{dash} marker-end=\"url(#arrow)\">",
            xml_escape(&edge.stroke)
        );
        let _ = writeln!(out, "    <title>{}</title>", xml_escape(&edge.from));
        let _ = writeln!(out, "  </path>");
        if let Some(label) = &edge.label {
            let label_x = (from_x + to_x) / 2.0;
            let label_y = (from_y + to_y) / 2.0 - 4.0;
            let _ = writeln!(
                out,
                "  <text x=\"{label_x:.1}\" y=\"{label_y:.1}\" font-family=\"Helvetica,Arial,sans-serif\" font-size=\"11\" fill=\"{}\">{}</text>",
                xml_escape(&edge.stroke),
                xml_escape(label)
            );
        }
    }
    for (index, node) in nodes.iter().enumerate() {
        let (x, y) = center(index);
        let _ = writeln!(
            out,
            "  <g class=\"node\" id=\"node-{}\">",
            xml_escape(&node.id)
        );
        let _ = writeln!(
            out,
            "    <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{box_w:.0}\" height=\"{box_h:.0}\" rx=\"6\" fill=\"{}\" stroke=\"{}\"/>",
            x - box_w / 2.0,
            y - box_h / 2.0,
            xml_escape(&node.fill),
            xml_escape(&node.stroke)
        );
        let _ = writeln!(
            out,
            "    <text x=\"{x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"Helvetica,Arial,sans-serif\" font-size=\"12\" fill=\"#222222\">{}</text>",
            y + 4.0,
            xml_escape(&node.label)
        );
        let _ = writeln!(out, "  </g>");
    }
    if !legend.is_empty() {
        let legend_y = margin + rows * cell_h + 20.0;
        let _ = writeln!(out, "  <g class=\"legend\">");
        for (row, entry) in legend.iter().enumerate() {
            let y = legend_y + row as f64 * 22.0;
            let dash = if entry.dashed {
                " stroke-dasharray=\"6 4\""
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "    <line x1=\"{:.1}\" y1=\"{y:.1}\" x2=\"{:.1}\" y2=\"{y:.1}\" stroke=\"{}\" stroke-width=\"1.6\"{dash}/>",
                margin,
                margin + 26.0,
                xml_escape(&entry.stroke)
            );
            let _ = writeln!(
                out,
                "    <text x=\"{:.1}\" y=\"{:.1}\" font-family=\"Helvetica,Arial,sans-serif\" font-size=\"12\" fill=\"#222222\">{}</text>",
                margin + 34.0,
                y + 4.0,
                xml_escape(&entry.text)
            );
        }
        let _ = writeln!(out, "  </g>");
    }
    let _ = writeln!(out, "</svg>");
    out
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mermaid_node_labels_hyphenated_names() {
        assert_eq!(mermaid_node("my-mod"), "my_mod_f59bd035[\"my-mod\"]");
        assert_eq!(mermaid_node("Shared"), "Shared");
    }

    #[test]
    fn mermaid_node_quotes_and_escapes_renderer_hostile_ids() {
        assert_eq!(
            mermaid_node("Shop.Api"),
            "Shop_Api_9243f63f[\"Shop.Api\"]"
        );
        assert_eq!(
            mermaid_node("std::collections::Queue"),
            "std__collections__Queue_916858ff[\"std::collections::Queue\"]"
        );
        assert_eq!(mermaid_node("src/a.rs"), "src_a_rs_d1e1ab14[\"src/a.rs\"]");
        assert_eq!(mermaid_node("my mod"), "my_mod_313f29db[\"my mod\"]");
        assert_eq!(mermaid_node("a\"b"), "a_b_e646a119[\"a\\\"b\"]");
        assert_eq!(mermaid_node("plain_1"), "plain_1");
    }

    #[test]
    fn mermaid_ids_are_collision_resistant_for_distinct_raw_names() {
        assert_ne!(mermaid_node("src/a/b.rs"), mermaid_node("src/a_b.rs"));
        assert_ne!(mermaid_id("A.B"), mermaid_id("A_B"));
        assert_ne!(mermaid_node("a-b"), mermaid_node("a b"));
        assert_ne!(mermaid_subgraph("db/migrations"), mermaid_subgraph("db_migrations"));
        assert_eq!(mermaid_id("A_B"), "A_B");
        assert_eq!(mermaid_id("plain_name"), "plain_name");
    }

    #[test]
    fn mermaid_id_assignment_is_independent_of_emission_order() {
        // Adversarial names that share one sanitized base (`a_b`): a pure name
        // keeps the bare form, every altered name takes a hash suffix. The id a
        // name maps to must never depend on which other name is emitted first —
        // the failure mode a map-order collision resolver would introduce.
        let names = ["a_b", "a-b", "a.b", "a b"];
        let forward: Vec<(String, String)> = names
            .iter()
            .map(|name| (name.to_string(), mermaid_id(name)))
            .collect();
        let reverse: Vec<(String, String)> = names
            .iter()
            .rev()
            .map(|name| (name.to_string(), mermaid_id(name)))
            .collect();
        for (name, id) in &forward {
            let (_, same) = reverse.iter().find(|(other, _)| other == name).unwrap();
            assert_eq!(id, same, "id for {name} must not depend on emission order");
        }
        let mut ids: Vec<&str> = forward.iter().map(|(_, id)| id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), names.len(), "colliding bases must still yield distinct ids");
    }

    #[test]
    fn mermaid_subgraph_escapes_header_and_labels_title() {
        assert_eq!(mermaid_subgraph("src"), "subgraph src");
        assert_eq!(
            mermaid_subgraph("src/archspec"),
            "subgraph src_archspec_0d6af733[\"src/archspec\"]"
        );
    }

    /// Role markers (roles US 06): the marker reaches the quoted label only —
    /// ids and edge grammar are the unmarked render's ids, so every reference
    /// keeps binding and an unmarked declaration's bytes never move.
    #[test]
    fn mermaid_markers_append_the_role_text_to_the_label() {
        assert_eq!(
            mermaid_node_marked("kit", " [facade]"),
            "kit[\"kit [facade]\"]"
        );
        assert_eq!(
            mermaid_node_marked("kit-bin::main", " [composition]"),
            "kit_bin__main_edf44f3e[\"kit-bin::main [composition]\"]"
        );
        assert_eq!(
            mermaid_subgraph_marked("kit", " [facade]"),
            "subgraph kit[\"kit [facade]\"]"
        );
        assert_eq!(
            mermaid_subgraph_marked("example.com/demo", " [composition]"),
            "subgraph example_com_demo_e32e1841[\"example.com/demo [composition]\"]"
        );
    }

    /// The plantuml marker is a stereotype on the declaration: the quoted
    /// name stays the identity edges reference.
    #[test]
    fn plantuml_markers_are_stereotypes_on_the_declaration() {
        assert_eq!(
            plantuml_quoted_component_marked("App::Api", "composition"),
            "component \"App::Api\" <<composition>>"
        );
        assert_eq!(
            plantuml_quoted_component_marked("kit", "facade"),
            "component \"kit\" <<facade>>"
        );
    }

    /// The bare-identifier twin of the quoted marker (roles-views US 01):
    /// views declaring bare names put the stereotype on the same identifier
    /// their edges reference, so identity grammar is unchanged.
    #[test]
    fn plantuml_bare_component_markers_keep_identifier_identity() {
        assert_eq!(
            plantuml_component_marked("Shop.Api", "composition"),
            "component Shop.Api <<composition>>"
        );
        assert_eq!(
            plantuml_component_marked("kit", "facade"),
            "component kit <<facade>>"
        );
    }

    #[test]
    fn mermaid_edges_reference_escaped_ids() {
        assert_eq!(mermaid_edge("my-mod", "core"), "my_mod_f59bd035 --> core");
        assert_eq!(
            mermaid_labeled_edge("my-mod", "core", "forbidden"),
            "my_mod_f59bd035 -.->|forbidden| core"
        );
        assert_eq!(mermaid_layout_link("my-mod", "core"), "my_mod_f59bd035 ~~~ core");
        assert_eq!(
            mermaid_edge("src/a.rs", "Shop.Api"),
            "src_a_rs_d1e1ab14 --> Shop_Api_9243f63f"
        );
        assert_eq!(mermaid_edge("A.B", "A_B"), "A_B_fa73d919 --> A_B");
    }

    #[test]
    fn plantuml_primitives_keep_identifier_styles() {
        assert_eq!(plantuml_component("cli"), "component cli");
        assert_eq!(
            plantuml_quoted_component("a::b"),
            "component \"a::b\""
        );
        assert_eq!(plantuml_entity("src/a.rs"), "entity \"src/a.rs\"");
        assert_eq!(plantuml_edge("a", "b"), "a --> b");
        assert_eq!(plantuml_quoted_edge("a", "b"), "\"a\" --> \"b\"");
        assert_eq!(
            plantuml_violation_edge("a", "b", "forbidden"),
            "a -[#red]-> b : forbidden"
        );
    }
}
