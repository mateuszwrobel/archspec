use crate::archspec::model::Model;
use rust_arch_test_kit::render::{
    mermaid_edge, mermaid_node, mermaid_node_marked, mermaid_subgraph, mermaid_subgraph_marked,
    plantuml_quoted_component, plantuml_quoted_component_marked, plantuml_quoted_edge,
};
use std::collections::BTreeSet;
use std::fmt::Write;

/// The label marker a role entry states for one model path (roles US 06):
/// the vocabulary word in brackets, suffixed to the node's visible label.
/// Pure lookup — the renderers attach markers where the MODEL states them
/// and compute no role of their own.
fn label_marker(model: &Model, name: &str) -> Option<String> {
    model
        .roles
        .get(name)
        .map(|role| format!(" [{}]", role.as_str()))
}

/// Renders the scan-phase model as a Mermaid flowchart. Nodes are the module
/// boundaries grouped per unit (one subgraph per unit); edges are the extracted
/// module edges. When `include_unit_edges` is set (the `scanner` view) the
/// cross-unit hard edges are drawn between unit subgraphs too. The `--format`
/// flag selects between this render and `render_model_plantuml`.
///
/// The emission is declaration-first: every id an arrow references was
/// declared above it — by a subgraph header, a soft-tier module line, or a
/// node line this function adds for endpoints the model declares nowhere
/// (audit defect D3). Those extra nodes carry `id["raw model name"]` with
/// the same labelling semantics `diagram --source scan` uses; ids keep the
/// FNV-suffixed sanitized form, because collision-distinctness across
/// hyphen/dot variants is a rendering precondition, not a defect.
///
/// Role facts reach the labels here (roles US 06): a node whose model path
/// carries a role entry gets that role's marker suffixed to its visible
/// label — units in their subgraph header, modules in their node line. The
/// marker is a pure lookup in `model.roles` (the tree computes nothing), and
/// ids stay the unmarked `mermaid_id` form, so arrow lines are untouched and
/// a model without roles renders byte-identical to the pre-marker era.
pub fn render_model(model: &Model, include_unit_edges: bool) -> String {
    let mut out = String::from("graph TD\n");

    // Emit unit subgraphs in canonical name order so node/edge order never
    // depends on the order the caller assembled `model.units`. `scan::extract`
    // already sorts units, so this is a no-op for the inspect path; it pins the
    // invariant for any model whose unit Vec arrives in a different order.
    let mut units = model.units.clone();
    units.sort_by(|left, right| left.name.cmp(&right.name));
    // Raw names already declared (with their labels) by the lines below:
    // subgraph headers and the soft-tier modules within them.
    let mut declared: BTreeSet<&str> = BTreeSet::new();
    for unit in &units {
        declared.insert(unit.name.as_str());
        let header = match label_marker(model, &unit.name) {
            Some(marker) => mermaid_subgraph_marked(&unit.name, &marker),
            None => mermaid_subgraph(&unit.name),
        };
        let _ = writeln!(out, "  {header}");
        if let Some(modules) = model.soft_structure.get(&unit.name) {
            for module in modules {
                declared.insert(module.as_str());
                let line = match label_marker(model, module) {
                    Some(marker) => mermaid_node_marked(module, &marker),
                    None => mermaid_node(module),
                };
                let _ = writeln!(out, "    {line}");
            }
        }
        let _ = writeln!(out, "  end");
    }

    let mut module_edges = model.module_edges.clone();
    module_edges.sort_by(|left, right| {
        (&left.unit, &left.from, &left.to).cmp(&(&right.unit, &right.from, &right.to))
    });

    // Declaration-first: an id an arrow references without a preceding
    // declaration renders detached, unlabelled, disconnected from its
    // subgraphs. Collect every edge endpoint (module tier, plus the unit
    // tier for the scanner view) that the subgraph block above did not
    // declare and give each one a labelled node line BEFORE the arrows.
    // Endpoints reusing a subgraph or module declaration are never repeated.
    let mut undeclared: BTreeSet<&str> = BTreeSet::new();
    for edge in &module_edges {
        for endpoint in [&edge.from, &edge.to] {
            if !declared.contains(endpoint.as_str()) {
                undeclared.insert(endpoint.as_str());
            }
        }
    }
    if include_unit_edges {
        for edge in &model.edges {
            for endpoint in [&edge.from, &edge.to] {
                if !declared.contains(endpoint.as_str()) {
                    undeclared.insert(endpoint.as_str());
                }
            }
        }
    }
    for name in &undeclared {
        let line = match label_marker(model, name) {
            Some(marker) => mermaid_node_marked(name, &marker),
            None => mermaid_node(name),
        };
        let _ = writeln!(out, "  {line}");
    }

    for edge in &module_edges {
        let _ = writeln!(out, "  {}", mermaid_edge(&edge.from, &edge.to));
    }

    if include_unit_edges {
        let mut edges = model.edges.clone();
        edges.sort_by(|left, right| (&left.from, &left.to).cmp(&(&right.from, &right.to)));
        for edge in &edges {
            let _ = writeln!(out, "  {}", mermaid_edge(&edge.from, &edge.to));
        }
    }

    out
}

/// Renders the scan-phase model as a PlantUML diagram: the exact content of
/// `render_model` in PlantUML syntax — one package per unit holding its
/// modules as components, the module edges, plus (for the `scanner` view) the
/// cross-unit hard edges. Names stay raw inside quotes rather than sanitized
/// into ids: model names carry `::` and `.` that PlantUML cannot parse as
/// bare identifiers, and quoting keeps every drawn boundary exactly the raw
/// model endpoint pair. Canonical ordering matches `render_model`. Role
/// markers ride the same lookup as the mermaid twin, emitted as `<<role>>`
/// stereotypes on the package/component declarations the model states —
/// the quoted identity edges reference stays unmarked, so this format
/// honors the markers through the existing quoting plumbing without touching
/// edge bytes.
pub fn render_model_plantuml(model: &Model, include_unit_edges: bool) -> String {
    let mut out = String::from("@startuml\n");

    let mut units = model.units.clone();
    units.sort_by(|left, right| left.name.cmp(&right.name));
    for unit in &units {
        match model.roles.get(unit.name.as_str()) {
            Some(role) => {
                let _ = writeln!(out, "package \"{}\" <<{}>> {{", unit.name, role.as_str());
            }
            None => {
                let _ = writeln!(out, "package \"{}\" {{", unit.name);
            }
        }
        if let Some(modules) = model.soft_structure.get(&unit.name) {
            for module in modules {
                let line = match model.roles.get(module.as_str()) {
                    Some(role) => plantuml_quoted_component_marked(module, role.as_str()),
                    None => plantuml_quoted_component(module),
                };
                let _ = writeln!(out, "  {line}");
            }
        }
        let _ = writeln!(out, "}}");
    }

    let mut module_edges = model.module_edges.clone();
    module_edges.sort_by(|left, right| {
        (&left.unit, &left.from, &left.to).cmp(&(&right.unit, &right.from, &right.to))
    });
    for edge in &module_edges {
        let _ = writeln!(out, "{}", plantuml_quoted_edge(&edge.from, &edge.to));
    }

    if include_unit_edges {
        let mut edges = model.edges.clone();
        edges.sort_by(|left, right| (&left.from, &left.to).cmp(&(&right.from, &right.to)));
        for edge in &edges {
            let _ = writeln!(out, "{}", plantuml_quoted_edge(&edge.from, &edge.to));
        }
    }

    let _ = writeln!(out, "@enduml");
    out
}

#[cfg(test)]
mod tests {
    use super::{render_model, render_model_plantuml};
    use crate::archspec::language::Language;
    use crate::archspec::model::{Model, Role};
    use crate::archspec::scan;
    use std::fs;

    /// A two-unit package: a hyphenated lib (`weird-kit`, so its mermaid id is
    /// FNV-suffixed) and a bin (`weirdkit`, pure) whose root references the lib
    /// through the underscore crate name `weird_kit::feature::run`. That
    /// reference is a cross-unit edge, so it belongs to the `scanner` view only.
    fn cross_unit_fixture() -> (tempfile::TempDir, Model) {
        let temp = tempfile::TempDir::new().expect("temp dir");
        let root = temp.path();
        let write = |rel: &str, body: &str| {
            let path = root.join(rel);
            fs::create_dir_all(path.parent().unwrap()).expect("dir");
            fs::write(path, body).expect("write");
        };
        write(
            "Cargo.toml",
            "[package]\nname = \"weird-kit\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"weirdkit\"\npath = \"src/main.rs\"\n",
        );
        write("src/lib.rs", "mod feature;\n");
        write("src/feature.rs", "pub fn run() {}\n");
        write("src/main.rs", "fn main() {\n    let _ = weird_kit::feature::run;\n}\n");
        let model = scan::extract(
            Language::Rust,
            root,
            
        )
        .expect("extract");
        (temp, model)
    }

    fn unit_edge_line(bin: &str, lib: &str) -> String {
        // The bin unit id is pure; the lib unit id is the FNV-suffixed form of
        // its hyphenated name. Reconstruct the exact edge the renderer would
        // emit for the cross-unit reference so the assertion is order-proof.
        let lib_id = {
            let sanitized: String = lib
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' })
                .collect();
            if sanitized == lib {
                sanitized
            } else {
                let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
                for b in lib.as_bytes() {
                    hash ^= *b as u64;
                    hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
                }
                format!("{sanitized}_{:08x}", (hash >> 32) as u32)
            }
        };
        format!("{bin} --> {lib_id}")
    }

    #[test]
    fn repeated_in_process_renders_are_byte_identical() {
        let (_keep, model) = cross_unit_fixture();
        let first = render_model(&model, false);
        assert!(first.contains("graph TD"));
        for _ in 0..9 {
            assert_eq!(
                first,
                render_model(&model, false),
                "tree render must be byte-identical across repeated in-process renders"
            );
        }
    }

    #[test]
    fn node_order_is_independent_of_unit_vec_order() {
        let (_keep, model) = cross_unit_fixture();
        let canonical = render_model(&model, false);
        let mut scrambled = model.clone();
        scrambled.units.reverse();
        assert_eq!(
            canonical,
            render_model(&scrambled, false),
            "subgraph/node emission must not depend on the order units arrive in"
        );
    }

    #[test]
    fn tree_view_never_emits_the_cross_unit_edge_scanner_view_does() {
        let (_keep, model) = cross_unit_fixture();
        let phantom = unit_edge_line("weirdkit", "weird-kit");
        let tree = render_model(&model, false);
        let scanner = render_model(&model, true);
        assert!(
            !tree.contains(&phantom),
            "tree view must not leak the cross-unit edge {phantom}:\n{tree}"
        );
        assert!(
            scanner.contains(&phantom),
            "scanner view must draw the cross-unit edge {phantom}:\n{scanner}"
        );
        // The hyphenated lib unit keeps a stable, collision-proof id in both.
        let lib_id = phantom.rsplit(" --> ").next().unwrap();
        assert!(tree.contains(lib_id), "lib unit id {lib_id} missing from tree:\n{tree}");
    }

    #[test]
    fn plantuml_views_carry_the_same_content_as_their_mermaid_twins() {
        let (_keep, model) = cross_unit_fixture();
        let tree = render_model_plantuml(&model, false);
        let scanner = render_model_plantuml(&model, true);
        for diagram in [&tree, &scanner] {
            assert!(
                diagram.starts_with("@startuml\n"),
                "missing @startuml header:\n{diagram}"
            );
            assert!(
                diagram.ends_with("@enduml\n"),
                "missing @enduml footer:\n{diagram}"
            );
            assert!(
                !diagram.contains("graph TD"),
                "mermaid syntax leaked into the plantuml render:\n{diagram}"
            );
        }
        // Quoted raw unit names: the hyphenated lib stays itself, no id
        // sanitization is involved — its role marker is the stereotype, not
        // a rewrite of the name (the lib root is declaration-only, so the
        // model states `facade` and the declaration carries it).
        assert!(
            tree.contains("package \"weird-kit\" <<facade>> {"),
            "hyphenated unit package missing:\n{tree}"
        );
        // The same tree/scanner split the mermaid views draw: the cross-unit
        // edge appears only in the scanner view, named by its raw endpoints.
        let phantom = "\"weirdkit\" --> \"weird-kit\"";
        assert!(
            !tree.contains(phantom),
            "tree view must not leak the cross-unit edge {phantom}:\n{tree}"
        );
        assert!(
            scanner.contains(phantom),
            "scanner view must draw the cross-unit edge {phantom}:\n{scanner}"
        );
    }

    /// Role markers (roles US 06) attach to the nodes the MODEL states: this
    /// fixture's scan derives `facade` at the declaration-only lib unit
    /// `weird-kit` (marked), and no composition entry for the cross-unit
    /// wiring bin (the derivation keys on the unit's own edges — absence
    /// means the node renders exactly its unmarked bytes).
    #[test]
    fn role_entries_from_the_model_mark_their_declarations() {
        let (_keep, model) = cross_unit_fixture();
        assert_eq!(
            model.roles.keys().collect::<Vec<_>>(),
            vec!["weird-kit"],
            "fixture states exactly the lib-unit facade"
        );
        let tree = render_model(&model, false);
        assert!(
            tree.contains("[\"weird-kit [facade]\"]"),
            "facade unit subgraph missing its marker:\n{tree}"
        );
        assert!(
            tree.contains("weirdkit__main_0b886f03[\"weirdkit::main\"]"),
            "a node the model leaves unmarked keeps its exact bytes:\n{tree}"
        );
        // Arrow lines reference bare ids only — markers never enter an edge.
        let phantom = unit_edge_line("weirdkit", "weird-kit");
        let scanner = render_model(&model, true);
        assert!(
            scanner.contains(&phantom),
            "unit edge line must stay unmarked:\n{scanner}"
        );

        let plantuml = render_model_plantuml(&model, false);
        assert!(
            plantuml.contains("package \"weird-kit\" <<facade>> {"),
            "facade package missing its stereotype:\n{plantuml}"
        );
        assert!(
            plantuml.contains("component \"weirdkit::main\""),
            "unmarked component stays a bare quoted declaration:\n{plantuml}"
        );
    }

    /// Absence, not noise: clear the roles map and no marker syntax survives
    /// in either format (the iff side of the marker contract).
    #[test]
    fn cleared_roles_emit_no_marker_syntax_anywhere() {
        let (_keep, model) = cross_unit_fixture();
        let mut plain = model.clone();
        plain.roles.clear();
        for diagram in [
            render_model(&plain, false),
            render_model(&plain, true),
            render_model_plantuml(&plain, false),
            render_model_plantuml(&plain, true),
        ] {
            for marker in [" [facade]", " [composition]", "<<facade>>", "<<composition>>"] {
                assert!(
                    !diagram.contains(marker),
                    "marker {marker:?} leaked without role facts:\n{diagram}"
                );
            }
        }
    }

    /// Marker placement follows declaration order, not the order role entries
    /// were inserted or units arrived in (determinism pin of US 06): the same
    /// model always renders the same marked bytes.
    #[test]
    fn markers_are_independent_of_role_insertion_and_unit_order() {
        let (_keep, model) = cross_unit_fixture();
        let canonical = render_model(&model, false);
        let mut reordered = model.clone();
        reordered.units.reverse();
        let mut rescrambled: Vec<(String, Role)> =
            reordered.roles.iter().map(|(k, v)| (k.clone(), *v)).collect();
        rescrambled.reverse();
        reordered.roles = rescrambled.into_iter().collect();
        assert_eq!(
            canonical,
            render_model(&reordered, false),
            "markers must not depend on unit or role arrival order"
        );
    }
}
