//! Single owner of diagram formatting primitives: escaping and node/edge
//! syntax for mermaid, plantuml, and markdown table output. Every renderer in
//! the `archspec` binary must build its lines from here so escaping cannot
//! drift between entry points.

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

/// Render a plantuml component declaration carrying a stereotype marker on a
/// BARE identifier (the role a roles map states for this model path, in the
/// views that declare components bare — `diagram --source scan`). The edges
/// reference the same bare identifier, so the stereotype never rewrites the
/// endpoint grammar: a node the roles map leaves unaddressed keeps exactly
/// what `plantuml_component` emits.
pub fn plantuml_component_marked(name: &str, stereotype: &str) -> String {
    format!("component {name} <<{stereotype}>>")
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
