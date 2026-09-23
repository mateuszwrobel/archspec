use crate::archspec::model::Model;
use std::collections::{BTreeMap, BTreeSet};

/// Render the seed `architecture.spec.toml` snapshotting the extracted model
/// across both tiers. One `[[module]]` per unit (sorted by name) matched via
/// `matches.units`, whose `allowed.depend_on` lists the unit's hard edge targets
/// (deduped and sorted). Then one `[[module]]` per TOP-LEVEL internal module of
/// each unit (the dotted path exactly one segment below the unit, sorted by
/// name), matched via `matches.modules`. Descendant submodules fold into their
/// top-level boundary via subtree matching, so no nested boundary shadows its
/// parent. A module the scan proved test-gated (`#[cfg(test)]` or an equivalent
/// test-only `cfg(all(..., test, ...))`, resolved by ancestry) is scaffolding and
/// seeds no boundary — consistent with `verify`; a module merely *named* `tests`
/// without a test cfg is production and seeds one like any other. Each
/// boundary's `allowed.depend_on` lists the top-level target modules its subtree
/// reaches via soft edges (deduped and sorted). Because boundaries are coarse,
/// `verify` resolves every soft edge to
/// exactly one boundary and the seed verifies clean against itself for acyclic
/// trees. A top-level fold that matches nothing but itself — no targets, no
/// unit prefix on its path, named by no kept entry's `allowed.depend_on`, and
/// the top-level of no module-edge endpoint (the namespace root of a dotted
/// project with edge-free namespaces, e.g. `Shop::Api` for the unit
/// `Shop.Api`) — is no-op noise and seeds no entry. A fold referenced as
/// a dependency target by a kept entry, or claiming an endpoint of a kept
/// module edge, seeds one even with no targets of its own: `verify` resolves
/// `allowed.depend_on` against declared boundary names and owns edge endpoints
/// through `matches.modules`, so dropping such a fold seeds `dead reference`
/// or `unowned module edge endpoint` warnings. No
/// stereotype vocabulary, no observed-shape constraints beyond
/// `feature_boundary` — plus one global `no_cycles` guard (no `modules` list)
/// so seeded projects start cycle-clean. Byte-deterministic for a given model.
///
/// Go exception, routed on the model's module facts plus tree shape: a go.work
/// tree carries the module tier natively (members are modules the way csharp
/// projects are units), so the seed declares ONE module per member —
/// `matches.units` listing the member's package import paths and
/// `allowed.depend_on` from the real cross-member imports — instead of the
/// unit + top-level-module fold, which would name dotted renderings no verify
/// pattern can claim. A single go.mod tree DOES carry
/// module facts (every package reference projects a module edge) without any
/// go.work member, so the seed declares one module per PACKAGES (see
/// `seed_go_packages`); keying on `has_module_tier()` alone would route that
/// tree into the member path and seed an empty module section (US 17 audit,
/// deviation D-1).
pub fn seed_spec(model: &Model) -> String {
    if model.language == "go" {
        // Member facts (go.work) seed members; package-level module facts
        // without members (single go.mod) seed packages; a tree with no
        // package crossing keeps the general shape.
        if model.soft_structure.values().any(|paths| !paths.is_empty()) {
            return seed_go_members(model);
        }
        if !model.module_edges.is_empty() {
            return seed_go_packages(model);
        }
    }
    let mut units = model.units.clone();
    units.sort_by(|left, right| left.name.cmp(&right.name));

    // Hard unit edges: unit name -> set of target unit names.
    let mut hard_outgoing: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for edge in &model.edges {
        hard_outgoing
            .entry(edge.from.as_str())
            .or_default()
            .insert(edge.to.as_str());
    }

    // Coarse module tier. The top-level module of a dotted path is its first two
    // segments (unit plus one), e.g. "app::billing" for "app::billing::sub".
    // Descendants fold into their top-level boundary. A module the scan PROVED
    // test-gated (`#[cfg(test)]`, resolved by ancestry so descendants fold in
    // too) is scaffolding and is excluded outright: its edges seed no boundary
    // and no dependency, matching `verify`. A module merely *named* `tests`
    // without a test cfg is production and seeds a boundary like any other. Each
    // soft edge is aggregated onto its source's top-level boundary; intra-
    // boundary edges are dropped.
    let top_level = |path: &str| -> Option<String> {
        let mut segments = path.splitn(3, "::");
        let unit = segments.next()?;
        let module = segments.next()?;
        Some(format!("{unit}::{module}"))
    };

    let mut module_paths: BTreeSet<String> = BTreeSet::new();
    let mut module_targets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for paths in model.soft_structure.values() {
        for path in paths {
            if model.is_test_gated(path) {
                continue;
            }
            if let Some(top) = top_level(path) {
                module_paths.insert(top);
            }
        }
    }
    // A module edge whose source is the crate root (a bare unit name, e.g. the
    // root `pub use tauri::plugin::tauri_plugin` producing `acme-lib ->
    // acme-lib::tauri::plugin`) has no `top_level` for its source, so it would
    // be dropped. It is a dependency of the UNIT boundary on a top-level module,
    // recorded per unit and folded into that boundary's `allowed.depend_on`.
    let mut root_outgoing: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    // Top-level boundary of every non-test-gated module-edge endpoint. `verify`
    // owns module-edge endpoints through `matches.modules`, so a fold claiming
    // no endpoint of a kept edge would leave that endpoint unowned.
    let mut endpoint_tops: BTreeSet<String> = BTreeSet::new();
    for edge in &model.module_edges {
        // A test-gated module (root-level or nested, resolved by ancestry) is
        // scaffolding, not a boundary. Test-module edges are excluded from the
        // seed so it stays consistent with `verify`, which likewise ignores them:
        // a test module depending on production modules is normal, never an
        // allowed/dependency declaration.
        if model.is_test_gated(&edge.from) || model.is_test_gated(&edge.to) {
            continue;
        }
        for endpoint in [&edge.from, &edge.to] {
            if let Some(top) = top_level(endpoint) {
                endpoint_tops.insert(top);
            }
        }
        if !edge.from.contains("::") {
            // Source is the crate root (a single-segment unit path). Resolve the
            // target to its top-level boundary; that becomes a unit-level
            // outgoing dependency.
            if let Some(to) = top_level(&edge.to) {
                module_paths.insert(to.clone());
                if edge.from != to {
                    root_outgoing
                        .entry(edge.from.as_str())
                        .or_default()
                        .insert(to);
                }
            }
            continue;
        }
        if !edge.to.contains("::") {
            // Target is the crate root (a single-segment unit path). The source
            // boundary depends on the unit boundary.
            if let Some(from) = top_level(&edge.from) {
                module_paths.insert(from.clone());
                if from != edge.to {
                    module_targets.entry(from).or_default().insert(edge.to.clone());
                }
            }
            continue;
        }
        let (Some(from), Some(to)) = (top_level(&edge.from), top_level(&edge.to)) else {
            continue;
        };
        module_paths.insert(from.clone());
        module_paths.insert(to.clone());
        if from != to {
            module_targets.entry(from).or_default().insert(to);
        }
    }

    let mut out = String::new();
    out.push_str("[project]\n");
    out.push_str(&format!(
        "language = \"{}\"\n",
        toml_escape(&model.language)
    ));
    out.push('\n');

    for (index, unit) in units.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str("[[module]]\n");
        out.push_str(&format!("name = \"{}\"\n", toml_escape(&unit.name)));
        out.push_str(&format!(
            "matches = {{ units = [\"{}\"] }}\n",
            toml_escape(&unit.name)
        ));
        let mut allowed: BTreeSet<&str> = BTreeSet::new();
        if let Some(deps) = hard_outgoing.get(unit.name.as_str()) {
            allowed.extend(deps.iter().copied());
        }
        if let Some(root_deps) = root_outgoing.get(unit.name.as_str()) {
            allowed.extend(root_deps.iter().map(String::as_str));
        }
        if !allowed.is_empty() {
            let rendered = allowed
                .iter()
                .map(|dep| format!("\"{}\"", toml_escape(dep)))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("allowed = {{ depend_on = [{rendered}] }}\n"));
        }
    }

    // Sink folds named as dependency targets by entries that will seed a
    // boundary (every entry with targets is kept, so values of `module_targets`
    // are exactly the references of kept module entries). `verify` resolves
    // `allowed.depend_on` against declared boundary names and owns module-edge
    // endpoints through `matches.modules`, so dropping a referenced sink would
    // seed `dead reference` and `unowned module edge endpoint` warnings.
    let referenced_sinks: BTreeSet<&str> =
        module_targets.values().flatten().map(String::as_str).collect();

    for path in &module_paths {
        // A module entry earns its place by matching units, nesting children,
        // being named as a dependency target, or owning a module-edge endpoint.
        // Skip a boundary that lists only itself in `matches.modules`, declares
        // no targets, is referenced by no kept entry, owns no edge endpoint, and
        // whose path nests under no unit: its first segment names no unit (e.g.
        // the namespace root `Shop::Api` of the dotted project
        // `Shop.Api`), so it folds the whole subtree into one self-matching
        // entry that declares no boundary pairs and floats in spec-sourced
        // diagrams.
        let has_targets = module_targets
            .get(path.as_str())
            .is_some_and(|targets| !targets.is_empty());
        let nests_under_unit = units
            .iter()
            .any(|unit| path.strip_prefix(&unit.name).is_some_and(|rest| rest.starts_with("::")));
        let is_referenced_sink = referenced_sinks.contains(path.as_str());
        let owns_edge_endpoint = endpoint_tops.contains(path.as_str());
        if !has_targets && !nests_under_unit && !is_referenced_sink && !owns_edge_endpoint {
            continue;
        }
        out.push('\n');
        out.push_str("[[module]]\n");
        out.push_str(&format!("name = \"{}\"\n", toml_escape(path)));
        out.push_str(&format!(
            "matches = {{ modules = [\"{}\"] }}\n",
            toml_escape(path)
        ));
        if let Some(targets) = module_targets.get(path.as_str()) {
            if !targets.is_empty() {
                let rendered = targets
                    .iter()
                    .map(|target| format!("\"{}\"", toml_escape(target)))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("allowed = {{ depend_on = [{rendered}] }}\n"));
            }
        }
    }

    // `feature_boundary` constraints (acceptance #19): a top-level module
    // declared under `#[cfg(feature = "...")]` yields a constraint naming its
    // gate and the gated module, so the gate is asserted going forward. Only for
    // modules that also seeded a boundary (always true for a top-level module).
    let mut feature_boundaries: Vec<(String, String)> = Vec::new();
    for unit in &units {
        let Some(decls) = model.root_module_declarations.get(&unit.name) else {
            continue;
        };
        for decl in decls {
            let Some(feature) = &decl.feature else {
                continue;
            };
            if !decl.gated {
                continue;
            }
            let module_path = format!("{}::{}", unit.name, decl.name);
            if !module_paths.contains(&module_path) {
                continue;
            }
            feature_boundaries.push((feature.clone(), module_path));
        }
    }
    feature_boundaries.sort();
    feature_boundaries.dedup();

    for (feature, module_path) in feature_boundaries {
        out.push('\n');
        out.push_str("[[constraint]]\n");
        out.push_str("type = \"feature_boundary\"\n");
        out.push_str(&format!("feature = \"{}\"\n", toml_escape(&feature)));
        out.push_str(&format!(
            "gated_modules = [\"{}\"]\n",
            toml_escape(&module_path)
        ));
    }

    // Global cycle guard: the seeded spec always carries a `no_cycles`
    // constraint with no `modules` list, which already means "all declared
    // modules" — seeded projects start cycle-clean without engine defaults or
    // schema additions.
    out.push('\n');
    out.push_str("[[constraint]]\n");
    out.push_str("type = \"no_cycles\"\n");

    out.push('\n');
    out
}

/// Seed for a go model with a native module tier (go.work members): one
/// `[[module]]` per member, named by its module path, claiming its packages
/// through `matches.units` (exact import paths, so every package lands in
/// exactly one boundary and none surfaces unassigned), and allowed to depend
/// on the members it imports across boundaries. A test-gated path is excluded
/// as everywhere in the seed. The result verifies clean against its own tree:
/// the boundary pairs it declares are exactly the real cross-member imports.
fn seed_go_members(model: &Model) -> String {
    let mut packages_by_member: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut member_of_path: BTreeMap<&str, &str> = BTreeMap::new();
    for (member, paths) in &model.soft_structure {
        for path in paths {
            if model.is_test_gated(path) {
                continue;
            }
            packages_by_member
                .entry(member.as_str())
                .or_default()
                .insert(path.replace("::", "/"));
            member_of_path.insert(path.as_str(), member.as_str());
        }
    }

    let mut member_dependencies: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for edge in &model.module_edges {
        if model.is_test_gated(&edge.from) || model.is_test_gated(&edge.to) {
            continue;
        }
        let (Some(from), Some(to)) = (
            member_of_path.get(edge.from.as_str()).copied(),
            member_of_path.get(edge.to.as_str()).copied(),
        ) else {
            continue;
        };
        if from != to {
            member_dependencies.entry(from).or_default().insert(to);
        }
    }

    let mut out = String::new();
    out.push_str("[project]\n");
    out.push_str(&format!(
        "language = \"{}\"\n",
        toml_escape(&model.language)
    ));
    out.push('\n');

    for (index, member) in packages_by_member.keys().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str("[[module]]\n");
        out.push_str(&format!("name = \"{}\"\n", toml_escape(member)));
        let units = packages_by_member
            .get(member)
            .into_iter()
            .flatten()
            .map(|path| format!("\"{}\"", toml_escape(path)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("matches = {{ units = [{units}] }}\n"));
        if let Some(targets) = member_dependencies.get(member) {
            if !targets.is_empty() {
                let rendered = targets
                    .iter()
                    .map(|target| format!("\"{}\"", toml_escape(target)))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("allowed = {{ depend_on = [{rendered}] }}\n"));
            }
        }
    }

    // Same global cycle guard as the general seed.
    out.push('\n');
    out.push_str("[[constraint]]\n");
    out.push_str("type = \"no_cycles\"\n");

    out.push('\n');
    out
}

/// Seed for a single go.mod tree whose model carries package-level module
/// facts (the scan projects every package reference onto the module
/// tier without any go.work member — US 17 audit, deviation D-1): one
/// `[[module]]` per package (unit), named by its import path and claiming
/// exactly itself through `matches.units`,
/// with `allowed.depend_on` the union of the
/// package's hard unit edges and its module edges (`::` restored to import
/// paths, a test-gated endpoint skipped as everywhere in the seed). Because
/// the module tier projects the very imports the unit tier already
/// carries, the union states the same crossings once at module granularity,
/// and the update -> report
/// round-trip is clean by construction: every package lands in exactly one
/// boundary, every module edge endpoint is claimed through the unit match,
/// and every observed boundary pair is allowed.
fn seed_go_packages(model: &Model) -> String {
    let mut dependencies: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in &model.edges {
        if edge.from != edge.to {
            dependencies
                .entry(edge.from.clone())
                .or_default()
                .insert(edge.to.clone());
        }
    }
    for edge in &model.module_edges {
        if model.is_test_gated(&edge.from) || model.is_test_gated(&edge.to) {
            continue;
        }
        let from = edge.from.replace("::", "/");
        let to = edge.to.replace("::", "/");
        if from != to {
            dependencies.entry(from).or_default().insert(to);
        }
    }

    let mut units: Vec<&str> = model.units.iter().map(|unit| unit.name.as_str()).collect();
    units.sort_unstable();

    let mut out = String::new();
    out.push_str("[project]\n");
    out.push_str(&format!(
        "language = \"{}\"\n",
        toml_escape(&model.language)
    ));
    out.push('\n');

    for (index, unit) in units.iter().enumerate() {
        if index > 0 {
            out.push('\n');
        }
        out.push_str("[[module]]\n");
        out.push_str(&format!("name = \"{}\"\n", toml_escape(unit)));
        out.push_str(&format!("matches = {{ units = [\"{}\"] }}\n", toml_escape(unit)));
        if let Some(targets) = dependencies.get(*unit) {
            if !targets.is_empty() {
                let rendered = targets
                    .iter()
                    .map(|target| format!("\"{}\"", toml_escape(target)))
                    .collect::<Vec<_>>()
                    .join(", ");
                out.push_str(&format!("allowed = {{ depend_on = [{rendered}] }}\n"));
            }
        }
    }

    // Same global cycle guard as the general seed.
    out.push('\n');
    out.push_str("[[constraint]]\n");
    out.push_str("type = \"no_cycles\"\n");

    out.push('\n');
    out
}

/// Escape a value for use inside a TOML basic string literal.
fn toml_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}
