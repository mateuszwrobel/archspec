use crate::common;
use crate::shared::driver::{Driver, LogicalTree};
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{
    array_contains, array_count, csharp_project_files, expect_success, external_dep, is_sorted,
    other_dep, scenario, scenario_when, scenario_when_capability,
};

/// A canonical two-unit tree with a hard edge app -> shared. Exercises the
/// unit/edge tiers every driver implements.
fn units_with_edge() -> LogicalTree {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into(), "shared".into()];
    tree.hard_edges.push(("app".into(), "shared".into()));
    tree
}

pub fn all() -> Vec<Scenario> {
    vec![
        scenario(
            Feature::ScanUnits,
            "lists_each_declared_unit",
            "scan reports every materialized logical unit under its concrete unit name",
            lists_each_declared_unit,
        ),
        scenario(
            Feature::ScanUnits,
            "records_hard_edge_between_dependent_units",
            "a declared hard edge between two units is emitted as a model edge on their concrete names",
            records_hard_edge_between_dependent_units,
        ),
        scenario(
            Feature::ScanUnits,
            "keeps_own_member_out_of_external",
            "a dependency on another unit of the project is an edge, never an external dependency",
            keeps_own_member_out_of_external,
        ),
        scenario(
            Feature::ScanUnits,
            "scan_is_deterministic_across_runs",
            "an unchanged tree scans to byte-identical JSON across runs",
            scan_is_deterministic_across_runs,
        ),
        scenario_when(
            Feature::ScanUnits,
            "test_project_absent_from_production_model",
            "a project whose package set names a test runner contributes no unit, edge, soft tier, or external fact, and the tree minus that project scans to identical JSON",
            csharp_project_files,
            test_project_absent_from_production_model,
        ),
        scenario_when(
            Feature::ScanUnits,
            "test_tier_follows_framework_evidence_not_name",
            "a no-framework project named LegacyTests stays production while a project named Helpers referencing xunit is test tier",
            csharp_project_files,
            test_tier_follows_framework_evidence_not_name,
        ),
        scenario_when(
            Feature::ScanUnits,
            "test_framework_package_ids_are_the_decisive_evidence",
            "xunit/nunit-family and Microsoft.NET.Test.Sdk package ids mark the test tier while FluentAssertions-only stays production",
            csharp_project_files,
            test_framework_package_ids_are_the_decisive_evidence,
        ),
        scenario_when(
            Feature::ScanUnits,
            "central_test_reference_marks_tier_version_entries_do_not",
            "a centrally declared test-runner PackageReference marks the owning project test tier; a version-only pin marks nothing",
            csharp_project_files,
            central_test_reference_marks_tier_version_entries_do_not,
        ),
        scenario_when(
            Feature::ScanUnits,
            "conditional_props_reference_is_not_evidence_or_dependency",
            "a Condition-bearing central PackageReference is ignored entirely — the production model stays intact and the conditional runner reaches no tier",
            csharp_project_files,
            conditional_props_reference_is_not_evidence_or_dependency,
        ),
        scenario_when(
            Feature::ScanUnits,
            "using_dropped_test_project_namespace_stays_out_of_production",
            "a production using under a dropped test project's namespace produces no edge, module, seed boundary, or verify finding",
            csharp_project_files,
            using_dropped_test_project_namespace_stays_out_of_production,
        ),
        scenario(
            Feature::ScanSoftStructure,
            "lists_declared_modules_in_soft_structure",
            "every declared module of a unit appears as a soft_structure path under that unit",
            lists_declared_modules_in_soft_structure,
        ),
        scenario(
            Feature::ScanSoftStructure,
            "nests_child_modules_under_parent",
            "a nested module is listed with its full dotted path, nested under its parent",
            nests_child_modules_under_parent,
        ),
        scenario(
            Feature::ScanSoftStructure,
            "keeps_modules_of_distinct_units_separate",
            "soft structure is scoped per unit; one unit never lists another unit's modules",
            keeps_modules_of_distinct_units_separate,
        ),
        scenario(
            Feature::ScanModuleEdges,
            "records_module_edge_from_internal_using",
            "a using/import from one declared module to another records a module edge",
            records_module_edge_from_internal_using,
        ),
        scenario(
            Feature::ScanModuleEdges,
            "does_not_create_module_edge_to_external_target",
            "a using/import of an external package produces no module edge",
            does_not_create_module_edge_to_external_target,
        ),
        scenario(
            Feature::ScanModuleEdges,
            "no_module_edges_for_units_without_soft_reference",
            "a unit whose modules reference nothing emits no module edges",
            no_module_edges_for_units_without_soft_reference,
        ),
        scenario(
            Feature::ScanExternal,
            "lists_manifest_declared_packages_as_external",
            "manifest-declared packages appear in the external dependency list",
            lists_manifest_declared_packages_as_external,
        ),
        scenario(
            Feature::ScanExternal,
            "sorts_and_deduplicates_external_entries",
            "external dependencies are sorted and deduplicated even when declared twice",
            sorts_and_deduplicates_external_entries,
        ),
        scenario_when(
            Feature::ScanExternal,
            "central_props_package_reaches_external_tier",
            "a centrally declared PackageReference item reaches the external tier, its module_external attribution, and fires a forbid rule",
            csharp_project_files,
            central_props_package_reaches_external_tier,
        ),
        scenario_when(
            Feature::ScanExternal,
            "nearest_props_file_overrides_root_for_central_entries",
            "each project's central entries come from the nearest props file walking upward; a project-level props shadows the root",
            csharp_project_files,
            nearest_props_file_overrides_root_for_central_entries,
        ),
        scenario_when(
            Feature::ScanExternal,
            "unreferenced_props_entries_stay_out_of_external",
            "props PackageVersion entries no project references never appear as external dependencies",
            csharp_project_files,
            unreferenced_props_entries_stay_out_of_external,
        ),
        scenario(
            Feature::ScanModuleExternal,
            "attributes_external_dependency_to_its_module",
            "an external package used by a module is attributed to that module path",
            attributes_external_dependency_to_its_module,
        ),
        scenario_when_capability(
            Feature::ScanModuleExternal,
            "keeps_external_usage_off_sibling_modules",
            "only the module that uses an external package records it, never its siblings",
            "module-tier",
            keeps_external_usage_off_sibling_modules,
        ),
        scenario_when(
            Feature::ScanModuleExternal,
            "misaligned_package_identity_replaces_namespace",
            "a using whose namespace misaligns with its referenced package id attributes the package (longest shared prefix), never the namespace; siblings sharing only a shorter prefix stay unattributed and namespaces stay in the soft tier",
            csharp_project_files,
            misaligned_package_identity_replaces_namespace,
        ),
        scenario_when(
            Feature::ScanModuleExternal,
            "aligned_package_id_attributed_case_insensitively",
            "an aligned package whose id differs from its namespace only in case attributes the package id as referenced",
            csharp_project_files,
            aligned_package_id_attributed_case_insensitively,
        ),
        scenario_when(
            Feature::ScanModuleExternal,
            "declared_but_unused_package_stays_out_of_module_external",
            "a referenced package no using surfaces stays out of module_external but present in the unit manifests",
            csharp_project_files,
            declared_but_unused_package_stays_out_of_module_external,
        ),
        scenario_when(
            Feature::ScanModuleExternal,
            "uncompilable_namespace_absent_from_external_tiers",
            "a using that maps to no referenced package appears nowhere in the external tiers with no crash or placeholder entry",
            csharp_project_files,
            uncompilable_namespace_absent_from_external_tiers,
        ),
        scenario_when(
            Feature::ScanModuleExternal,
            "family_root_using_ties_family_sibling_packages",
            "a using of a family root namespace sharing its longest prefix with the root package and a sibling package extending it attributes both (documented tie)",
            csharp_project_files,
            family_root_using_ties_family_sibling_packages,
        ),
        scenario(
            Feature::ScanUnitManifests,
            "exposes_dependency_facts_per_unit",
            "unit_manifests surfaces the manifest-declared packages of each unit",
            exposes_dependency_facts_per_unit,
        ),
        scenario(
            Feature::ScanUnitManifests,
            "unit_manifests_cover_every_unit",
            "every materialized unit has an entry in unit_manifests",
            unit_manifests_cover_every_unit,
        ),
        scenario(
            Feature::ScanUnitManifests,
            "unit_without_packages_has_empty_dependencies",
            "a unit with no declared packages reports an empty dependency list",
            unit_without_packages_has_empty_dependencies,
        ),
    ]
}

fn lists_each_declared_unit(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let tree = units_with_edge();
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let units = model["units"]
        .as_array()
        .ok_or("units must be an array")?;
    let names: Vec<String> = units
        .iter()
        .filter_map(|unit| unit["name"].as_str().map(String::from))
        .collect();
    for logical in ["app", "shared"] {
        let expected = driver.unit_name(logical);
        if !names.contains(&expected) {
            return Err(format!("unit {expected} missing from scan: {names:?}"));
        }
    }
    Ok(())
}

fn records_hard_edge_between_dependent_units(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let tree = units_with_edge();
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let from = driver.unit_name("app");
    let to = driver.unit_name("shared");
    let edges = model["edges"].as_array().ok_or("edges must be an array")?;
    let found = edges.iter().any(|edge| {
        edge["from"].as_str() == Some(from.as_str()) && edge["to"].as_str() == Some(to.as_str())
    });
    if !found {
        return Err(format!("edge {from} -> {to} missing: {edges:?}"));
    }
    Ok(())
}

fn keeps_own_member_out_of_external(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let tree = units_with_edge();
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let member = driver.unit_name("shared");
    if array_contains(&model["external"], &member) {
        return Err(format!("own member {member} must not be external"));
    }
    Ok(())
}

fn scan_is_deterministic_across_runs(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    let first = expect_success(driver, fx, &["scan"])?;
    let second = expect_success(driver, fx, &["scan"])?;
    let first_stdout = common::stdout(&first);
    let second_stdout = common::stdout(&second);
    if first_stdout != second_stdout {
        return Err("scan output differs across runs".into());
    }
    Ok(())
}

fn lists_declared_modules_in_soft_structure(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules
        .insert("app".into(), vec!["core".into(), "ui".into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let unit = driver.unit_name("app");
    let paths = model["soft_structure"]
        .get(&unit)
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("soft_structure for {unit} must be an array"))?;
    let paths: Vec<&str> = paths.iter().filter_map(|path| path.as_str()).collect();
    for module in ["core", "ui"] {
        let expected = driver.module_path("app", module);
        if !paths.contains(&expected.as_str()) {
            return Err(format!("module {expected} missing from soft_structure: {paths:?}"));
        }
    }
    Ok(())
}

fn nests_child_modules_under_parent(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules
        .insert("app".into(), vec!["core".into(), "core.storage".into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let unit = driver.unit_name("app");
    let paths = model["soft_structure"]
        .get(&unit)
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("soft_structure for {unit} must be an array"))?;
    let paths: Vec<&str> = paths.iter().filter_map(|path| path.as_str()).collect();
    let parent = driver.module_path("app", "core");
    let child = driver.module_path("app", "core.storage");
    if !paths.contains(&parent.as_str()) {
        return Err(format!("parent module {parent} missing: {paths:?}"));
    }
    if !paths.contains(&child.as_str()) {
        return Err(format!("nested module {child} missing: {paths:?}"));
    }
    Ok(())
}

fn keeps_modules_of_distinct_units_separate(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into(), "shared".into()];
    tree.modules.insert("app".into(), vec!["core".into()]);
    tree.modules
        .insert("shared".into(), vec!["models".into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let app_paths = module_paths(&model, driver, "app")?;
    let shared_paths = module_paths(&model, driver, "shared")?;
    let app_core = driver.module_path("app", "core");
    let shared_models = driver.module_path("shared", "models");
    if !app_paths.contains(&app_core.as_str()) {
        return Err(format!("app must list {app_core}: {app_paths:?}"));
    }
    if !shared_paths.contains(&shared_models.as_str()) {
        return Err(format!("shared must list {shared_models}: {shared_paths:?}"));
    }
    if app_paths.contains(&shared_models.as_str()) {
        return Err(format!("app must not list {shared_models}: {app_paths:?}"));
    }
    if shared_paths.contains(&app_core.as_str()) {
        return Err(format!("shared must not list {app_core}: {shared_paths:?}"));
    }
    Ok(())
}

fn records_module_edge_from_internal_using(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules
        .insert("app".into(), vec!["core".into(), "ui".into()]);
    tree.module_usings
        .push(("app".into(), "core".into(), "ui".into()));
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let unit = driver.unit_name("app");
    let from = driver.module_path("app", "core");
    let to = driver.module_path("app", "ui");
    let edges = model["module_edges"]
        .as_array()
        .ok_or("module_edges must be an array")?;
    let found = edges.iter().any(|edge| {
        edge["unit"].as_str() == Some(unit.as_str())
            && edge["from"].as_str() == Some(from.as_str())
            && edge["to"].as_str() == Some(to.as_str())
    });
    if !found {
        return Err(format!("module edge {from} -> {to} in {unit} missing: {edges:?}"));
    }
    Ok(())
}

fn does_not_create_module_edge_to_external_target(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["core".into()]);
    let external = external_dep(driver);
    tree.module_usings
        .push(("app".into(), "core".into(), external.into()));
    tree.packages
        .insert("app".into(), vec![external.into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let edges = model["module_edges"]
        .as_array()
        .ok_or("module_edges must be an array")?;
    if !edges.is_empty() {
        return Err(format!(
            "using an external package must not create a module edge: {edges:?}"
        ));
    }
    Ok(())
}

fn no_module_edges_for_units_without_soft_reference(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into(), "shared".into()];
    tree.modules
        .insert("app".into(), vec!["core".into(), "ui".into()]);
    tree.modules
        .insert("shared".into(), vec!["models".into()]);
    tree.module_usings
        .push(("app".into(), "core".into(), "ui".into()));
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let shared = driver.unit_name("shared");
    let edges = model["module_edges"]
        .as_array()
        .ok_or("module_edges must be an array")?;
    let leaked = edges
        .iter()
        .any(|edge| edge["unit"].as_str() == Some(shared.as_str()));
    if leaked {
        return Err(format!("unit {shared} must have no module edges: {edges:?}"));
    }
    Ok(())
}

fn lists_manifest_declared_packages_as_external(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    let external = external_dep(driver);
    tree.packages.insert("app".into(), vec![external.into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    if !array_contains(&model["external"], external) {
        return Err(format!(
            "external must contain the manifest package {external}: {:?}",
            model["external"]
        ));
    }
    Ok(())
}

fn sorts_and_deduplicates_external_entries(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["core".into()]);
    let external = external_dep(driver);
    let other = other_dep(driver);
    tree.packages
        .insert("app".into(), vec![external.into(), other.into()]);
    tree.module_usings
        .push(("app".into(), "core".into(), external.into()));
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let list = &model["external"];
    if !array_contains(list, external) {
        return Err(format!("external must contain {external}: {list:?}"));
    }
    if !array_contains(list, other) {
        return Err(format!("external must contain {other}: {list:?}"));
    }
    if array_count(list, external) != 1 {
        return Err(format!(
            "external must deduplicate {external} across manifest and using: {list:?}"
        ));
    }
    if !is_sorted(list) {
        return Err(format!("external must be sorted: {list:?}"));
    }
    Ok(())
}

/// A csproj with no version info for Polly at all: the reference is declared
/// centrally in the root Directory.Packages.props. Central entries must reach
/// the external tier, the module_external attribution, and fire a
/// forbid_external_crates rule naming Polly.
fn central_props_package_reaches_external_tier(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["Core".into()]);
    tree.module_usings
        .push(("app".into(), "Core".into(), "Polly".into()));
    tree.central_packages
        .push((".".into(), "Polly".into()));
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    if !array_contains(&model["external"], "Polly") {
        return Err(format!(
            "centrally referenced Polly missing from external tier: {:?}",
            model["external"]
        ));
    }
    let unit = driver.unit_name("app");
    let deps = model["unit_manifests"]
        .get(&unit)
        .and_then(|facts| facts.get("dependencies"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("dependencies for {unit} missing"))?;
    if !deps.iter().any(|name| name.as_str() == Some("Polly")) {
        return Err(format!(
            "central reference Polly missing from {unit} dependencies: {deps:?}"
        ));
    }
    let module = driver.module_path("app", "Core");
    let attributed = model["module_external"]
        .get(&module)
        .and_then(|value| value.as_array())
        .map(|crates| crates.iter().any(|name| name.as_str() == Some("Polly")))
        .unwrap_or(false);
    if !attributed {
        return Err(format!(
            "Polly not attributed to {module}: {:?}",
            model["module_external"]
        ));
    }
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"{module}\"]\nforbid = [\"Polly\"]\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    let text = common::stdout(&output);
    if output.status.code() == Some(0) || !text.contains("forbidden external crate") {
        return Err(format!(
            "forbid_external_crates rule naming Polly must fire (exit {:?}, stdout: {text})",
            output.status.code()
        ));
    }
    Ok(())
}

/// Root props centralizes Root.Pkg; app carries a closer props file
/// centralizing Project.Pkg. Nearest props wins: app sees only its own file's
/// central entries, shared (no closer file) still sees the root's.
fn nearest_props_file_overrides_root_for_central_entries(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into(), "shared".into()];
    tree.modules
        .insert("app".into(), vec!["Core".into()]);
    tree.modules
        .insert("shared".into(), vec!["Core".into()]);
    tree.module_usings
        .push(("app".into(), "Core".into(), "Project.Pkg".into()));
    tree.module_usings
        .push(("shared".into(), "Core".into(), "Root.Pkg".into()));
    tree.central_packages
        .push((".".into(), "Root.Pkg".into()));
    tree.central_packages
        .push(("app".into(), "Project.Pkg".into()));
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    for package in ["Project.Pkg", "Root.Pkg"] {
        if !array_contains(&model["external"], package) {
            return Err(format!(
                "central package {package} missing from external tier: {:?}",
                model["external"]
            ));
        }
    }
    let app = driver.unit_name("app");
    let shared = driver.unit_name("shared");
    let app_deps = model["unit_manifests"]
        .get(&app)
        .and_then(|facts| facts.get("dependencies"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("dependencies for {app} missing"))?;
    let shared_deps = model["unit_manifests"]
        .get(&shared)
        .and_then(|facts| facts.get("dependencies"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("dependencies for {shared} missing"))?;
    if !deps_contain(app_deps, "Project.Pkg") {
        return Err(format!("app must use its nearest props file: {app_deps:?}"));
    }
    if deps_contain(app_deps, "Root.Pkg") {
        return Err(format!(
            "project-level props must shadow the root for app: {app_deps:?}"
        ));
    }
    if !deps_contain(shared_deps, "Root.Pkg") {
        return Err(format!("shared must fall through to the root props: {shared_deps:?}"));
    }
    if deps_contain(shared_deps, "Project.Pkg") {
        return Err(format!(
            "app's props must not leak into shared: {shared_deps:?}"
        ));
    }
    Ok(())
}

/// Versions do not imply usage: a props file pinning a package no project
/// references must not contribute the name to the external tier.
fn unreferenced_props_entries_stay_out_of_external(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    let external = external_dep(driver);
    tree.packages.insert("app".into(), vec![external.into()]);
    tree.central_version_entries
        .push((".".into(), "Unused.Transitive.Pin".into()));
    driver.materialize(fx, &tree);
    if !fx.path("Directory.Packages.props").is_file() {
        return Err("fixture must carry a Directory.Packages.props".into());
    }
    let model = driver.scan(fx);
    if !array_contains(&model["external"], external) {
        return Err(format!(
            "csproj PackageReference {external} must stay in external: {:?}",
            model["external"]
        ));
    }
    if array_contains(&model["external"], "Unused.Transitive.Pin") {
        return Err(format!(
            "unreferenced props entry must not be external: {:?}",
            model["external"]
        ));
    }
    let unit = driver.unit_name("app");
    let deps = model["unit_manifests"]
        .get(&unit)
        .and_then(|facts| facts.get("dependencies"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("dependencies for {unit} missing"))?;
    if deps_contain(deps, "Unused.Transitive.Pin") {
        return Err(format!(
            "unreferenced props entry must not reach {unit} dependencies: {deps:?}"
        ));
    }
    Ok(())
}

fn deps_contain(deps: &[serde_json::Value], package: &str) -> bool {
    deps.iter().any(|name| name.as_str() == Some(package))
}

fn attributes_external_dependency_to_its_module(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["core".into()]);
    let external = external_dep(driver);
    tree.module_usings
        .push(("app".into(), "core".into(), external.into()));
    tree.packages
        .insert("app".into(), vec![external.into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let module = driver.module_path("app", "core");
    let crates = model["module_external"]
        .get(&module)
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("module_external for {module} must be an array"))?;
    if !crates.iter().any(|name| name.as_str() == Some(external)) {
        return Err(format!(
            "external dep {external} not attributed to {module}: {crates:?}"
        ));
    }
    Ok(())
}

fn keeps_external_usage_off_sibling_modules(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules
        .insert("app".into(), vec!["core".into(), "ui".into()]);
    let external = external_dep(driver);
    tree.module_usings
        .push(("app".into(), "core".into(), external.into()));
    tree.packages
        .insert("app".into(), vec![external.into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let core = driver.module_path("app", "core");
    let ui = driver.module_path("app", "ui");
    let crates = model["module_external"]
        .get(&core)
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("module_external for {core} must be an array"))?;
    if !crates.iter().any(|name| name.as_str() == Some(external)) {
        return Err(format!("external dep {external} not on {core}: {crates:?}"));
    }
    if model["module_external"].get(&ui).is_some() {
        return Err(format!(
            "sibling module {ui} must not record the external dep"
        ));
    }
    Ok(())
}

/// Every value list of the `module_external` map flattened.
fn all_module_external(model: &serde_json::Value) -> Vec<String> {
    model["module_external"]
        .as_object()
        .map(|map| {
            map.values()
                .filter_map(|list| list.as_array())
                .flatten()
                .filter_map(|name| name.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn module_external_at<'a>(
    model: &'a serde_json::Value,
    module: &str,
) -> Result<Vec<&'a str>, String> {
    model["module_external"]
        .get(module)
        .and_then(|value| value.as_array())
        .map(|list| list.iter().filter_map(|name| name.as_str()).collect())
        .ok_or_else(|| format!("module_external for {module} must be an array"))
}

/// Package identity wins over namespace shape: the using references the
/// misaligned package via a two-segment shared prefix, the unrelated package
/// sharing only `Microsoft` cannot win the longest match, and no namespace
/// string reaches the external tier. Namespaces stay in the soft tier.
fn misaligned_package_identity_replaces_namespace(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["Core".into()]);
    tree.module_usings
        .push(("app".into(), "Core".into(), "Microsoft.IdentityModel.Tokens".into()));
    tree.packages.insert(
        "app".into(),
        vec![
            "Microsoft.IdentityModel.JsonWebTokens".into(),
            "Microsoft.Extensions.Logging".into(),
        ],
    );
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let module = driver.module_path("app", "Core");
    let attributed = module_external_at(&model, &module)?;
    if !attributed.contains(&"Microsoft.IdentityModel.JsonWebTokens") {
        return Err(format!(
            "referenced package with longest shared prefix must be attributed to {module}: {attributed:?}"
        ));
    }
    if attributed.contains(&"Microsoft.Extensions.Logging") {
        return Err(format!(
            "package sharing only a shorter prefix must not be attributed: {attributed:?}"
        ));
    }
    let values = all_module_external(&model);
    if values.iter().any(|v| v == "Microsoft.IdentityModel.Tokens") {
        return Err(format!(
            "namespace string must never reach module_external values: {values:?}"
        ));
    }
    let unit = driver.unit_name("app");
    let soft: Vec<&str> = model["soft_structure"]
        .get(&unit)
        .and_then(|list| list.as_array())
        .map(|list| list.iter().filter_map(|m| m.as_str()).collect())
        .unwrap_or_default();
    let soft_module = driver.module_path("app", "Core");
    if !soft.contains(&soft_module.as_str()) {
        return Err(format!(
            "namespaces must stay in the soft tier: {soft:?}"
        ));
    }
    Ok(())
}

/// `fluentassertions` ↔ `using FluentAssertions;` — an aligned pair modulo id
/// casing attributes the referenced package id, not the namespace spelling.
/// FluentAssertions carries no runner, so the project stays production (the
/// test-tier evidence is the runner SDK, not an assertion library).
fn aligned_package_id_attributed_case_insensitively(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["Core".into()]);
    tree.module_usings
        .push(("app".into(), "Core".into(), "FluentAssertions".into()));
    tree.packages
        .insert("app".into(), vec!["fluentassertions".into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let module = driver.module_path("app", "Core");
    let attributed = module_external_at(&model, &module)?;
    if !attributed.contains(&"fluentassertions") {
        return Err(format!(
            "case-insensitive aligned id must be attributed: {attributed:?}"
        ));
    }
    let values = all_module_external(&model);
    if values.iter().any(|v| v == "FluentAssertions") {
        return Err(format!(
            "namespace spelling must not be attributed: {values:?}"
        ));
    }
    Ok(())
}

/// Declared-but-unused follows the rust precedent: manifests carry it, the
/// usage-attributed tier does not.
fn declared_but_unused_package_stays_out_of_module_external(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["Core".into()]);
    tree.module_usings
        .push(("app".into(), "Core".into(), "Newtonsoft.Json".into()));
    tree.packages.insert(
        "app".into(),
        vec!["Newtonsoft.Json".into(), "Serilog".into()],
    );
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let module = driver.module_path("app", "Core");
    let attributed = module_external_at(&model, &module)?;
    if !attributed.contains(&"Newtonsoft.Json") {
        return Err(format!("used package missing from {module}: {attributed:?}"));
    }
    let values = all_module_external(&model);
    if values.iter().any(|v| v == "Serilog") {
        return Err(format!(
            "referenced-but-never-used package must stay out of module_external: {values:?}"
        ));
    }
    let unit = driver.unit_name("app");
    let deps = model["unit_manifests"]
        .get(&unit)
        .and_then(|facts| facts.get("dependencies"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("dependencies for {unit} missing"))?;
    if !deps_contain(deps, "Serilog") {
        return Err(format!(
            "declared package must stay visible in {unit} manifests: {deps:?}"
        ));
    }
    Ok(())
}

/// A namespace that maps to no referenced package (typo, missing reference) is
/// not a dependency fact: it vanishes from the external tiers without a crash
/// or a placeholder entry.
fn uncompilable_namespace_absent_from_external_tiers(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["Core".into()]);
    tree.module_usings
        .push(("app".into(), "Core".into(), "Newtonsoft.Json".into()));
    tree.module_usings
        .push(("app".into(), "Core".into(), "Microsoft.IdentityModel.Tokens".into()));
    tree.packages.insert("app".into(), vec!["Newtonsoft.Json".into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let module = driver.module_path("app", "Core");
    let attributed = module_external_at(&model, &module)?;
    if !attributed.contains(&"Newtonsoft.Json") {
        return Err(format!(
            "the referenced package must still be attributed: {attributed:?}"
        ));
    }
    if attributed.contains(&"Microsoft.IdentityModel.Tokens") {
        return Err(format!(
            "namespace mapping to no referenced package must drop, not attribute: {attributed:?}"
        ));
    }
    let values = all_module_external(&model);
    if values.iter().any(|v| v == "Microsoft.IdentityModel.Tokens") {
        return Err(format!(
            "uncompilable namespace must not reach any module_external entry: {values:?}"
        ));
    }
    if array_contains(&model["external"], "Microsoft.IdentityModel.Tokens") {
        return Err(format!(
            "uncompilable namespace must not reach the external tier: {:?}",
            model["external"]
        ));
    }
    if let Some(map) = model["module_external"].as_object() {
        for (key, list) in map {
            if list.as_array().map(Vec::is_empty).unwrap_or(true) {
                return Err(format!("placeholder (empty) module_external entry at {key}"));
            }
        }
    }
    Ok(())
}

/// The documented tie pinned as behavior: a `using` of a family root namespace
/// shares the same longest dotted prefix with the root package AND a sibling
/// package extending it, so the longest-prefix rule attributes both.
fn family_root_using_ties_family_sibling_packages(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["Core".into()]);
    tree.module_usings
        .push(("app".into(), "Core".into(), "Microsoft.Extensions.Options".into()));
    tree.packages.insert(
        "app".into(),
        vec![
            "Microsoft.Extensions.Options".into(),
            "Microsoft.Extensions.Options.Abstractions".into(),
        ],
    );
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let module = driver.module_path("app", "Core");
    let attributed = module_external_at(&model, &module)?;
    if !attributed.contains(&"Microsoft.Extensions.Options") {
        return Err(format!(
            "root package of the tied family must be attributed to {module}: {attributed:?}"
        ));
    }
    if !attributed.contains(&"Microsoft.Extensions.Options.Abstractions") {
        return Err(format!(
            "sibling package tied at the same longest prefix must be attributed: {attributed:?}"
        ));
    }
    Ok(())
}

fn exposes_dependency_facts_per_unit(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    let external = external_dep(driver);
    tree.packages.insert("app".into(), vec![external.into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let unit = driver.unit_name("app");
    let deps = model["unit_manifests"]
        .get(&unit)
        .and_then(|facts| facts.get("dependencies"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("unit_manifests.dependencies for {unit} missing"))?;
    if !deps.iter().any(|name| name.as_str() == Some(external)) {
        return Err(format!("dependencies for {unit} must contain {external}: {deps:?}"));
    }
    Ok(())
}

fn unit_manifests_cover_every_unit(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into(), "shared".into()];
    let external = external_dep(driver);
    tree.packages.insert("app".into(), vec![external.into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let manifests = model["unit_manifests"]
        .as_object()
        .ok_or("unit_manifests must be an object")?;
    for logical in ["app", "shared"] {
        let unit = driver.unit_name(logical);
        if !manifests.contains_key(&unit) {
            return Err(format!("unit_manifests must cover {unit}"));
        }
    }
    Ok(())
}

fn unit_without_packages_has_empty_dependencies(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into(), "shared".into()];
    let external = external_dep(driver);
    tree.packages.insert("app".into(), vec![external.into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let shared = driver.unit_name("shared");
    let deps = model["unit_manifests"]
        .get(&shared)
        .and_then(|facts| facts.get("dependencies"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("dependencies for {shared} missing"))?;
    if !deps.is_empty() {
        return Err(format!("dependencies for {shared} must be empty: {deps:?}"));
    }
    Ok(())
}

/// The sorted soft-structure paths of one logical unit, as `&str`s.
fn module_paths<'a>(
    model: &'a serde_json::Value,
    driver: &Driver,
    logical_unit: &str,
) -> Result<Vec<&'a str>, String> {
    let unit = driver.unit_name(logical_unit);
    model["soft_structure"]
        .get(&unit)
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("soft_structure for {unit} must be an array"))?
        .iter()
        .map(|path| path.as_str().ok_or_else(|| format!("soft path in {unit} not a string")))
        .collect()
}
/// Unit names of a scanned model.
fn scanned_unit_names(model: &serde_json::Value) -> Vec<String> {
    model["units"]
        .as_array()
        .map(|units| {
            units
                .iter()
                .filter_map(|unit| unit["name"].as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// Workplan csharp_test_projects_tier, scenario "test project excluded from
/// production model": a project whose package set names a test runner is test
/// tier — its unit, edges, soft tier and packages contribute no production
/// fact, and the same tree minus the test project scans to identical JSON.
fn test_project_absent_from_production_model(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["App".into(), "App.Tests".into()];
    tree.modules.insert("App".into(), vec!["Core".into()]);
    tree.modules.insert("App.Tests".into(), vec!["Specs".into()]);
    tree.hard_edges
        .push(("App.Tests".into(), "App".into()));
    tree.module_usings
        .push(("App.Tests".into(), "Specs".into(), "Xunit".into()));
    tree.packages
        .insert("App.Tests".into(), vec!["xunit".into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);

    let names = scanned_unit_names(&model);
    if !names.contains(&"App".to_string()) {
        return Err(format!("production unit App missing from scan: {names:?}"));
    }
    if names.contains(&"App.Tests".to_string()) {
        return Err(format!(
            "test project App.Tests must be absent from the production model: {names:?}"
        ));
    }
    if array_contains(&model["external"], "xunit") {
        return Err(format!(
            "test-only package xunit must not reach the production external tier: {:?}",
            model["external"]
        ));
    }
    let attributed = all_module_external(&model);
    if attributed.iter().any(|package| package == "xunit") {
        return Err(format!(
            "test-only package xunit must not be attributed in module_external: {attributed:?}"
        ));
    }
    let edges = model["edges"].as_array().ok_or("edges must be an array")?;
    if edges.iter().any(|edge| {
        edge["from"].as_str() == Some("App.Tests") || edge["to"].as_str() == Some("App.Tests")
    }) {
        return Err(format!(
            "edges through the test project must not exist: {edges:?}"
        ));
    }

    let mut production = LogicalTree::new();
    production.units = vec!["App".into()];
    production.modules.insert("App".into(), vec!["Core".into()]);
    let without = common::Fixture::new();
    driver.materialize(&without, &production);
    let with_tests = expect_success(driver, fx, &["scan"])?;
    let without_tests = expect_success(driver, &without, &["scan"])?;
    if common::stdout(&with_tests) != common::stdout(&without_tests) {
        return Err(format!(
            "model of the tree with the test project must equal the same tree minus it:\n{}\nvs\n{}",
            common::stdout(&with_tests),
            common::stdout(&without_tests)
        ));
    }
    Ok(())
}

/// Workplan csharp_test_projects_tier, scenario "detection is dependency
/// evidence, not name lore": project names play no part — a real production
/// tool named `LegacyTests` (no runner package) stays a production unit, while
/// a project named `Helpers` referencing xunit is test tier.
fn test_tier_follows_framework_evidence_not_name(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["App".into(), "LegacyTests".into(), "Helpers".into()];
    tree.packages
        .insert("LegacyTests".into(), vec!["Serilog".into()]);
    tree.packages
        .insert("Helpers".into(), vec!["xunit".into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);

    let names = scanned_unit_names(&model);
    if !names.contains(&"LegacyTests".to_string()) {
        return Err(format!(
            "a no-framework project named LegacyTests is production and must stay a unit: {names:?}"
        ));
    }
    if names.contains(&"Helpers".to_string()) {
        return Err(format!(
            "a project named Helpers referencing xunit must be test tier: {names:?}"
        ));
    }
    if array_contains(&model["external"], "xunit") {
        return Err(format!(
            "the excluded project's runner package must not reach the external tier: {:?}",
            model["external"]
        ));
    }
    if !array_contains(&model["external"], "Serilog") {
        return Err(format!(
            "the production LegacyTests project must keep its external tier: {:?}",
            model["external"]
        ));
    }
    Ok(())
}

/// Workplan csharp_test_projects_tier, scenario "framework detection needs the
/// actual frameworks": the pinned decisive list — xunit-family, nunit-family
/// and Microsoft.NET.Test.Sdk ids (case-insensitively) — marks the test tier;
/// FluentAssertions alone (no runner) is production tooling, and the MSTest
/// packages OUTSIDE the SDK family (`MSTest.TestFramework`,
/// `MSTest.TestAdapter`) are pinned as the documented residual: an MSTest
/// project without `Microsoft.NET.Test.Sdk` stays production.
fn test_framework_package_ids_are_the_decisive_evidence(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let runner_projects = [
        ("u_xunit", "xunit"),
        ("u_xunit_runner", "Xunit.Runner.VisualStudio"),
        ("u_nunit", "NUnit"),
        ("u_nunit_adapter", "NUnit3TestAdapter"),
        ("u_test_sdk", "Microsoft.NET.Test.Sdk"),
        ("u_test_sdk_lowercase", "microsoft.net.test.sdk"),
    ];
    let mstest_residual = [
        ("u_mstest_framework", "MSTest.TestFramework"),
        ("u_mstest_adapter", "MSTest.TestAdapter"),
    ];
    let mut tree = LogicalTree::new();
    tree.units = vec!["u_fluent".into(), "u_plain".into()];
    for (unit, package) in runner_projects {
        tree.units.push(unit.into());
        tree.packages.insert(unit.into(), vec![package.into()]);
    }
    for (unit, package) in mstest_residual {
        tree.units.push(unit.into());
        tree.packages.insert(unit.into(), vec![package.into()]);
    }
    tree.packages
        .insert("u_fluent".into(), vec!["FluentAssertions".into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);

    let names = scanned_unit_names(&model);
    for (unit, package) in runner_projects {
        if names.contains(&unit.to_string()) {
            return Err(format!(
                "project {unit} referencing runner package {package} must be test tier: {names:?}"
            ));
        }
        if array_contains(&model["external"], package) {
            return Err(format!(
                "runner package {package} must not reach the production external tier: {:?}",
                model["external"]
            ));
        }
    }
    for production in ["u_fluent", "u_plain"] {
        if !names.contains(&production.to_string()) {
            return Err(format!(
                "{production} carries no runner evidence and must stay a production unit: {names:?}"
            ));
        }
    }
    for (unit, package) in mstest_residual {
        if !names.contains(&unit.to_string()) {
            return Err(format!(
                "MSTest packages outside Microsoft.NET.Test.Sdk are the documented residual — \
                 {unit} referencing {package} must stay a production unit: {names:?}"
            ));
        }
        if !array_contains(&model["external"], package) {
            return Err(format!(
                "the residual keeps its package in the external tier: {:?}",
                model["external"]
            ));
        }
    }
    if !array_contains(&model["external"], "FluentAssertions") {
        return Err(format!(
            "FluentAssertions-only stays production, so its package stays in the external tier: {:?}",
            model["external"]
        ));
    }
    Ok(())
}

/// Workplan csharp_test_projects_tier (central packages, H1): detection reads
/// the same resolved package set the external tier uses — a centrally declared
/// runner `PackageReference` marks the owning project test tier, while a
/// version-only `PackageVersion` pin marks nothing anywhere.
fn central_test_reference_marks_tier_version_entries_do_not(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["App".into(), "App.Tests".into()];
    tree.central_packages
        .push(("App.Tests".into(), "xunit".into()));
    tree.central_version_entries
        .push((".".into(), "xunit".into()));
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);

    let names = scanned_unit_names(&model);
    if !names.contains(&"App".to_string()) {
        return Err(format!(
            "a version-only central xunit pin is not usage — App must stay production: {names:?}"
        ));
    }
    if names.contains(&"App.Tests".to_string()) {
        return Err(format!(
            "the centrally referenced runner marks App.Tests test tier: {names:?}"
        ));
    }
    if array_contains(&model["external"], "xunit") {
        return Err(format!(
            "the excluded project's central runner reference must not reach the external tier: {:?}",
            model["external"]
        ));
    }
    Ok(())
}

/// Review F1: an MSBuild condition is unevaluable without MSBuild, so a
/// `Condition`-bearing central `PackageReference` is ignored entirely — as
/// tier evidence AND as an external dependency. A root props file wiring a
/// runner behind `Condition="'$(IsTestProject)'=='true'"` next to
/// unconditional production references must leave the production model intact
/// (no project erased) and the runner absent from every tier, while the
/// unconditional central runner of the central-tier scenario still marks the
/// tier there.
fn conditional_props_reference_is_not_evidence_or_dependency(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["App".into(), "App.Tests".into()];
    tree.modules.insert("App".into(), vec!["Core".into()]);
    tree.central_packages
        .push((".".into(), "Newtonsoft.Json".into()));
    tree.central_conditional_packages
        .push((".".into(), "xunit".into()));
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);

    let names = scanned_unit_names(&model);
    for unit in ["App", "App.Tests"] {
        if !names.contains(&unit.to_string()) {
            return Err(format!(
                "a conditional central runner reference must not erase the production model — \
                 {unit} must stay a unit: {names:?}"
            ));
        }
    }
    if !array_contains(&model["external"], "Newtonsoft.Json") {
        return Err(format!(
            "the unconditional central production reference must stay in the external tier: {:?}",
            model["external"]
        ));
    }
    if model.to_string().contains("xunit") {
        return Err(format!(
            "the conditional runner reference is not modeled — it must reach no tier: {}",
            model
        ));
    }
    Ok(())
}

/// Review F2: dropping a test project at the scan must not let prefix-owner
/// namespace resolution resurrect it. `App` references (and its sources use)
/// the dropped test project's namespace `App.Tests.Specs`; prefix resolution
/// used to attribute that using to the production unit's own `App` prefix — a
/// phantom production edge, a bogus seeded module, a verify finding. The
/// dropped prefixes are retained, so the using resolves to nothing anywhere.
fn using_dropped_test_project_namespace_stays_out_of_production(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["App".into(), "App.Tests".into()];
    tree.modules.insert("App".into(), vec!["Core".into()]);
    tree.modules.insert("App.Tests".into(), vec!["Specs".into()]);
    tree.hard_edges.push(("App".into(), "App.Tests".into()));
    tree.packages
        .insert("App.Tests".into(), vec!["xunit".into()]);
    tree.module_usings
        .push(("App".into(), "Core".into(), "App.Tests.Specs".into()));
    driver.materialize(fx, &tree);
    // A production file in the unit's ROOT namespace — the shared prefix that
    // makes prefix-owner resolution fire: `App.Tests.Specs` resolves through
    // its `App` prefix back into the referencing unit.
    fx.write(
        "App/App.cs",
        "using App.Tests.Specs;\nnamespace App;\npublic class AppRoot { }\n",
    );
    let model = driver.scan(fx);

    let names = scanned_unit_names(&model);
    if names != vec!["App".to_string()] {
        return Err(format!(
            "the runner-referencing project must be dropped from the model units: {names:?}"
        ));
    }
    if model.to_string().contains("Tests") {
        return Err(format!(
            "the dropped test project's namespaces must not resurrect as production modules or \
             edges anywhere in the model: {}",
            model
        ));
    }
    expect_success(driver, fx, &["update"])?;
    let spec = fx.read("architecture.spec.toml");
    if spec.contains("Tests") {
        return Err(format!(
            "the seed must declare no module or boundary for the dropped test project:\n{spec}"
        ));
    }
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if stdout.contains("missing") || stdout.contains("cycle:") || stdout.contains("error") {
        return Err(format!("verify must report no findings:\n{stdout}"));
    }
    Ok(())
}
