use crate::archspec::model::Model;
use rust_arch_test_kit::render::{mermaid_edge, mermaid_node, mermaid_subgraph};
use std::fmt::Write;

/// Renders the scan-phase model as a Mermaid flowchart. Nodes are the module
/// boundaries grouped per unit (one subgraph per unit); edges are the extracted
/// module edges. When `include_unit_edges` is set (the `scanner` view) the
/// cross-unit hard edges are drawn between unit subgraphs too.
pub fn render_model(model: &Model, include_unit_edges: bool) -> String {
    let mut out = String::from("graph TD\n");

    // Emit unit subgraphs in canonical name order so node/edge order never
    // depends on the order the caller assembled `model.units`. `scan::extract`
    // already sorts units, so this is a no-op for the inspect path; it pins the
    // invariant for any model whose unit Vec arrives in a different order.
    let mut units = model.units.clone();
    units.sort_by(|left, right| left.name.cmp(&right.name));
    for unit in &units {
        let _ = writeln!(out, "  {}", mermaid_subgraph(&unit.name));
        if let Some(modules) = model.soft_structure.get(&unit.name) {
            for module in modules {
                let _ = writeln!(out, "    {}", mermaid_node(module));
            }
        }
        let _ = writeln!(out, "  end");
    }

    let mut module_edges = model.module_edges.clone();
    module_edges.sort_by(|left, right| {
        (&left.unit, &left.from, &left.to).cmp(&(&right.unit, &right.from, &right.to))
    });
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

#[cfg(test)]
mod tests {
    use super::render_model;
    use crate::archspec::language::Language;
    use crate::archspec::model::Model;
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
        let model = scan::extract(Language::Rust, root).expect("extract");
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
}
