use crate::common;
use crate::shared::driver::{
    materialize_go_declared_grouping, materialize_go_declared_submodule, Driver, Language,
    LogicalTree,
};
use crate::shared::feature::Feature;
use crate::shared::scenario::Scenario;
use crate::shared::scenarios::{
    boundary_spec, csharp_project_files, dot_stems_nest_units, expect_success, external_dep,
    go_driver, other_dep, scenario, scenario_when, scenario_when_capability, scenario_when_inert,
};

pub fn all() -> Vec<Scenario> {
    vec![
        scenario(
            Feature::VerifyModuleBoundaries,
            "unit_boundary_matching_tree_verifies_clean",
            "a unit-boundary spec declaring every unit with its edges allowed verifies clean",
            unit_boundary_matching_tree_verifies_clean,
        ),
        scenario_when_capability(
            Feature::VerifyModuleBoundaries,
            "module_boundary_with_allowed_dependency_verifies_clean",
            "a module-boundary spec whose depend_on allows the module edge verifies clean",
            "module-tier",
            module_boundary_with_allowed_dependency_verifies_clean,
        ),
        scenario(
            Feature::VerifyModuleBoundaries,
            "undeclared_component_is_reported_missing",
            "a spec component matching no unit is reported as a missing component",
            undeclared_component_is_reported_missing,
        ),
        // The depend_on floor is the other half of boundary semantics, so it
        // maps onto the existing VerifyModuleBoundaries feature — the check
        // lives in driver-agnostic compare logic, so the probe is implemented
        // for every driver and no new Feature variant is warranted.
        scenario(
            Feature::VerifyModuleBoundaries,
            "stale_allowed_dependency_reports_missing_edge",
            "a declared depend_on edge absent from the model fails non-strict verify with the canonical missing edge label, and the same tree minus the stale declaration verifies clean",
            stale_allowed_dependency_reports_missing_edge,
        ),
        scenario(
            Feature::VerifyNoCycles,
            "acyclic_tree_passes_no_cycles_constraint",
            "a no_cycles constraint over an acyclic dependency graph verifies clean",
            acyclic_tree_passes_no_cycles_constraint,
        ),
        scenario(
            Feature::VerifyNoCycles,
            "cyclic_tree_fails_no_cycles_constraint",
            "a no_cycles constraint over a cyclic graph fails and reports the cycle",
            cyclic_tree_fails_no_cycles_constraint,
        ),
        scenario(
            Feature::VerifyForbidExternal,
            "forbidden_external_dependency_is_reported",
            "a module using a forbidden external package is reported as a violation",
            forbidden_external_dependency_is_reported,
        ),
        scenario_when_capability(
            Feature::VerifyForbidExternal,
            "module_outside_forbid_list_passes",
            "a module not listed in the forbid constraint passes even when a sibling violates",
            "module-tier",
            module_outside_forbid_list_passes,
        ),
        scenario_when(
            Feature::VerifyForbidExternal,
            "forbid_by_package_name_fires_on_misaligned_namespace",
            "a forbid rule naming the exact package id fires on a module whose only using maps to that package by longest shared prefix",
            csharp_project_files,
            forbid_by_package_name_fires_on_misaligned_namespace,
        ),
        scenario_when(
            Feature::VerifyForbidExternal,
            "forbid_by_package_name_fires_on_aligned_namespace",
            "a forbid rule naming an aligned package id fires on the module that imports its namespace",
            csharp_project_files,
            forbid_by_package_name_fires_on_aligned_namespace,
        ),
        scenario_when(
            Feature::VerifyForbidExternal,
            "forbid_prefix_sharing_package_does_not_fire_on_longest_match",
            "a forbid rule naming a package that shares only a shorter prefix with the imported namespace does not fire (longest match attributes the real dependency)",
            csharp_project_files,
            forbid_prefix_sharing_package_does_not_fire_on_longest_match,
        ),
        scenario_when(
            Feature::VerifyForbidExternal,
            "unused_reference_absent_from_module_external_but_in_manifests",
            "a referenced package no using surfaces is absent from module_external (no forbid fire) yet present in the unit manifests",
            csharp_project_files,
            unused_reference_absent_from_module_external_but_in_manifests,
        ),
        scenario(
            Feature::VerifyManifestIntegrity,
            "forbidden_dependency_in_manifest_is_reported",
            "a manifest declaring a forbidden dependency fails manifest_integrity",
            forbidden_dependency_in_manifest_is_reported,
        ),
        scenario(
            Feature::VerifyManifestIntegrity,
            "manifest_matching_constraint_verifies_clean",
            "a manifest whose dependencies avoid the forbidden list verifies clean",
            manifest_matching_constraint_verifies_clean,
        ),
        scenario(
            Feature::VerifyRootFacade,
            "import_of_facade_root_reexport_is_violation",
            "an internal module importing an item through a publication-only root is a facade dependency",
            import_of_facade_root_reexport_is_violation,
        ),
        scenario(
            Feature::VerifyRootFacade,
            "canonical_import_stays_clean_under_active_facade",
            "canonical imports never touch the facade root, so verify passes with the rule active",
            canonical_import_stays_clean_under_active_facade,
        ),
        // The inert side of the root-facade row belongs to the verify surface
        // every driver implements (the facade rule itself stays under
        // VerifyRootFacade, whose probe is honest about the inert drivers).
        scenario_when_inert(
            Feature::VerifyModuleBoundaries,
            "facade_rule_note_announces_facade_tierless_driver",
            "on a driver whose table marks root-facade not-emitted verify states the facade rule inert instead of staying silent, with the verdict and exit status unchanged",
            "root-facade",
            facade_rule_note_announces_facade_tierless_driver,
        ),
        scenario(
            Feature::VerifyForbiddenLaundering,
            "forbidden_edge_routed_through_conduit_hop_is_laundered",
            "a ban routed via an undeclared conduit module is reported as a laundered forbidden edge",
            forbidden_edge_routed_through_conduit_hop_is_laundered,
        ),
        scenario_when(
            Feature::VerifyForbiddenLaundering,
            "worktree_free_go_tree_announces_laundering_capability",
            "a single-module go tree (no go.work) announces consistently: a fired rule reports the laundered finding without any tier note, a clean run states no native module tier this run",
            go_driver,
            worktree_free_go_tree_announces_laundering_capability,
        ),
        // Documents the conduit-claim precondition: the check traces routes on
        // the boundary pair graph, so the intermediate's territory must be
        // claimed by a boundary (explicitly or through its unit's fallback)
        // for a bypass to be visible at all. Both halves pinned; no behavior
        // change is asserted beyond the existing rule.
        scenario(
            Feature::VerifyForbiddenLaundering,
            "laundering_traces_only_claimed_conduit_territory",
            "a ban routed through a conduit whose territory a boundary claims is reported as laundered; the same conduit claimed by no boundary leaves the pair graph — no laundered line, only unexpected component (go) or unowned endpoints (soft tier) surfacing",
            laundering_traces_only_claimed_conduit_territory,
        ),
        scenario(
            Feature::VerifySubmoduleContracts,
            "submodule_contract_leak_reported_under_submodule_path",
            "a contract.forbid stereotype surfacing under a submodule boundary leaks under the submodule's path",
            submodule_contract_leak_reported_under_submodule_path,
        ),
        // Documents the go matching trap: a go unit's name is its full import
        // path, the bare-segment concession addresses `::` paths only, and the
        // contract surface is claimed membership — so a bare-name stereotype
        // (or a matches-less parent) reads inert, never enforced.
        scenario_when(
            Feature::VerifySubmoduleContracts,
            "go_submodule_contract_stereotype_needs_full_path_glob",
            "a submodule contract over go packages fires for a stereotype globbing the full import path under a parent that claims its units, and reads inert for a bare-name stereotype that matches no go unit",
            go_driver,
            go_submodule_contract_stereotype_needs_full_path_glob,
        ),
        scenario(
            Feature::VerifyModuleBoundaries,
            "undeclared_dependency_exceeds_allowed_ceiling",
            "an extracted edge no boundary declares fails with the cross-component ceiling label, and an explicit ban reroutes it to the forbidden-edge label",
            undeclared_dependency_exceeds_allowed_ceiling,
        ),
        scenario(
            Feature::VerifyModuleBoundaries,
            "dead_reference_to_source_module_classifies_per_language",
            "an undeclared depend_on target is classified per capability table: granular soft-tier drivers see a source module, a driver without the tier says not verifiable instead of claiming non-existence",
            dead_reference_to_source_module_classifies_per_language,
        ),
        scenario(
            Feature::VerifyModuleBoundaries,
            "undeclared_unit_reports_unexpected_component",
            "a source unit no boundary declares is reported as an unexpected component",
            undeclared_unit_reports_unexpected_component,
        ),
        scenario_when(
            Feature::VerifyModuleBoundaries,
            "dotted_subunit_of_declared_component_is_unassigned",
            "a unit nested under a declared component by name is reported as unassigned",
            dot_stems_nest_units,
            dotted_subunit_of_declared_component_is_unassigned,
        ),
        scenario_when_capability(
            Feature::VerifyModuleBoundaries,
            "two_boundaries_claim_module_at_equal_specificity",
            "two boundaries claiming one module at equal specificity fail with the ambiguous-match report",
            "module-tier",
            two_boundaries_claim_module_at_equal_specificity,
        ),
        scenario_when_capability(
            Feature::VerifyModuleBoundaries,
            "unclaimed_module_edge_endpoint_is_reported_unowned",
            "a module edge endpoint no boundary claims is a warning-level unowned endpoint, promoted by --strict",
            "module-tier",
            unclaimed_module_edge_endpoint_is_reported_unowned,
        ),
        scenario(
            Feature::VerifyModuleBoundaries,
            "top_level_contract_forbid_stereotype_surfaced_by_unit",
            "a boundary contract forbidding a stereotype whose pattern matches the claimed unit name leaks",
            top_level_contract_forbid_stereotype_surfaced_by_unit,
        ),
        scenario_when_capability(
            Feature::VerifyModuleBoundaries,
            "feature_boundary_gated_pattern_without_declaration_fires_where_emitted",
            "a feature_boundary gate naming no declared module fails on drivers whose table row emits root-module-declarations",
            "root-module-declarations",
            feature_boundary_gated_pattern_without_declaration_fires_where_emitted,
        ),
        scenario_when_inert(
            Feature::VerifyModuleBoundaries,
            "feature_boundary_missing_declarations_fact_surfaces_vacuous",
            "on a driver emitting no root-module-declarations fact the constraint surfaces once as a capability-reason vacuous warning, never as per-pattern errors",
            "root-module-declarations",
            feature_boundary_missing_declarations_fact_surfaces_vacuous,
        ),
        scenario_when_capability(
            Feature::VerifySubmoduleContracts,
            "forbidden_submodule_dependency_across_sibling_children",
            "an edge between sibling submodules banned by the parent constraint is reported with the submodule-dependency label",
            "module-tier",
            forbidden_submodule_dependency_across_sibling_children,
        ),
        scenario_when_capability(
            Feature::VerifySubmoduleContracts,
            "forbidden_submodule_dependency_full_path_engages",
            "the same sibling-submodule ban expressed with full module-path from/forbid engages identically on every module-tier driver and is never reported vacuous",
            "module-tier",
            forbidden_submodule_dependency_full_path_engages,
        ),
        scenario_when_capability(
            Feature::VerifyModuleBoundaries,
            "rust_root_public_api_leak_pin",
            "a public root export outside the allowlist is reported as a public api leak",
            "root-module-declarations",
            rust_root_public_api_leak_pin,
        ),
        scenario_when_capability(
            Feature::VerifyModuleBoundaries,
            "rust_unverifiable_glob_export_pin",
            "a root glob re-export that resolves to nothing is reported as unverifiable",
            "root-module-declarations",
            rust_unverifiable_glob_export_pin,
        ),
        scenario_when_capability(
            Feature::VerifyModuleBoundaries,
            "rust_empty_glob_export_pin",
            "a root glob resolving to zero public items fails closed with the empty-glob label",
            "root-module-declarations",
            rust_empty_glob_export_pin,
        ),
        scenario_when_capability(
            Feature::VerifyModuleBoundaries,
            "rust_unresolved_module_file_pin",
            "a module declaration with no source file is a warning-level unresolved module file, promoted by --strict",
            "root-module-declarations",
            rust_unresolved_module_file_pin,
        ),
        scenario(
            Feature::VerifyNoCycles,
            "warning_severity_cycle_routes_to_warning_bucket",
            "a cycle caught by a warning-severity no_cycles constraint passes verify and renders in the warning bucket, promoted by --strict",
            warning_severity_cycle_routes_to_warning_bucket,
        ),
        scenario(
            Feature::VerifyNoCycles,
            "vacuous_cycle_constraint_surfaces_edgeless_group",
            "a no_cycles constraint over a module group with no internal edges reports vacuous and stays green",
            vacuous_cycle_constraint_surfaces_edgeless_group,
        ),
        scenario_when(
            Feature::VerifyNoCycles,
            "cycle_through_a_test_project_is_invisible_to_production_guard",
            "a c# cycle carried only through a test-tier project leaves the production graph a DAG: the seed omits it and verify stays green (rust cfg(test) / go *_test.go parity)",
            csharp_project_files,
            cycle_through_a_test_project_is_invisible_to_production_guard,
        ),
    ]
}

fn unit_boundary_matching_tree_verifies_clean(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let tree = driver.probe_tree();
    driver.materialize(fx, &tree);
    fx.write("architecture.spec.toml", &boundary_spec(driver));
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("matches source model") {
        return Err(format!("verify must confirm the match:\n{stdout}"));
    }
    Ok(())
}

fn module_boundary_with_allowed_dependency_verifies_clean(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    // Only drivers with a soft module tier run this behavior (see `scenario_when`).
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules
        .insert("app".into(), vec!["core".into(), "ui".into()]);
    tree.module_usings
        .push(("app".into(), "core".into(), "ui".into()));
    driver.materialize(fx, &tree);
    let unit = driver.unit_name("app");
    let core = driver.module_path("app", "core");
    let ui = driver.module_path("app", "ui");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"{unit}\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[module]]\nname = \"{core}\"\nmatches = {{ modules = [\"{core}\"] }}\n\
             [module.allowed]\ndepend_on = [\"{ui}\"]\n\n\
             [[module]]\nname = \"{ui}\"\nmatches = {{ modules = [\"{ui}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("matches source model") {
        return Err(format!("verify must confirm the match:\n{stdout}"));
    }
    Ok(())
}

fn undeclared_component_is_reported_missing(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    driver.materialize(fx, &tree);
    let ghost = driver.unit_name("ghost");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"ghost\"\nmatches = {{ units = [\"{ghost}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("verify must fail when a declared component is missing".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("missing component: ghost") {
        return Err(format!(
            "report must name the missing component ghost:\n{stdout}"
        ));
    }
    Ok(())
}

/// The missing-edge floor (depend_on exact-set, spec.md): a declared top-level
/// target with no extracted edge fails verify without `--strict`, rendered
/// under the one canonical label on every driver; dropping the stale
/// declaration leaves the same tree clean.
fn stale_allowed_dependency_reports_missing_edge(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    driver.materialize(fx, &tree);
    let a = driver.unit_name("a");
    let b = driver.unit_name("b");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\
             [module.allowed]\ndepend_on = [\"b\"]\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err(
            "verify must fail when a declared depend_on edge is absent from the model".into(),
        );
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("missing edge: a -> b") {
        return Err(format!(
            "report must name the finding with the canonical `missing edge:` label:\n{stdout}"
        ));
    }
    if stdout.contains("allowed edge absent") {
        return Err(format!(
            "the retired label must never render:\n{stdout}"
        ));
    }
    // Same tree, stale declaration dropped: the floor has nothing to enforce
    // and the tree verifies clean.
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if stdout.contains("missing edge:") {
        return Err(format!(
            "without the stale declaration no floor rule engages:\n{stdout}"
        ));
    }
    Ok(())
}

fn acyclic_tree_passes_no_cycles_constraint(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    tree.hard_edges.push(("a".into(), "b".into()));
    driver.materialize(fx, &tree);
    write_cycle_spec(driver, fx, false);
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("matches source model") {
        return Err(format!("acyclic tree must verify clean:\n{stdout}"));
    }
    Ok(())
}

fn cyclic_tree_fails_no_cycles_constraint(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    tree.hard_edges = vec![("a".into(), "b".into()), ("b".into(), "a".into())];
    driver.materialize(fx, &tree);
    write_cycle_spec(driver, fx, true);
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("verify must fail on a dependency cycle".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("cycle: ") {
        return Err(format!("report must list the cycle:\n{stdout}"));
    }
    Ok(())
}

/// Workplan csharp_test_projects_tier, scenario "cycle through a test project
/// stays invisible to production guard": App -> App.Tests -> App is a cycle
/// only through the test project (xunit evidence). The scan drops the test
/// project's facts, so the seeded graph has no edges to guard and `verify`
/// stays green — the project-tier mirror of the rust cfg(test) and go
/// `*_test.go` exclusion contracts.
fn cycle_through_a_test_project_is_invisible_to_production_guard(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["App".into(), "App.Tests".into()];
    tree.hard_edges = vec![
        ("App".into(), "App.Tests".into()),
        ("App.Tests".into(), "App".into()),
    ];
    tree.packages
        .insert("App.Tests".into(), vec!["xunit".into()]);
    driver.materialize(fx, &tree);
    expect_success(driver, fx, &["update"])?;
    let spec = fx.read("architecture.spec.toml");
    if spec.contains("App.Tests") {
        return Err(format!(
            "the seed must declare no boundary for the test project:\n{spec}"
        ));
    }
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if stdout.contains("cycle: ") {
        return Err(format!(
            "a cycle only through the test project must report no cycle finding:\n{stdout}"
        ));
    }
    Ok(())
}

/// A spec declaring components a and b plus a `no_cycles` constraint at error
/// severity. `allow_reverse` also permits b -> a (the mutual-cycle shape);
/// without it only a -> b is declared, so an acyclic tree satisfies every
/// required edge. Shared by both no_cycles scenarios.
fn write_cycle_spec(driver: &Driver, fx: &common::Fixture, allow_reverse: bool) {
    let a = driver.unit_name("a");
    let b = driver.unit_name("b");
    let b_allowed = if allow_reverse {
        "\n[module.allowed]\ndepend_on = [\"a\"]"
    } else {
        ""
    };
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\
             [module.allowed]\ndepend_on = [\"b\"]\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}{b_allowed}\n\n\
             [[constraint]]\ntype = \"no_cycles\"\nmodules = [\"a\", \"b\"]\nseverity = \"error\"\n",
            driver.language.as_str()
        ),
    );
}

fn forbidden_external_dependency_is_reported(
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
    let unit = driver.unit_name("app");
    let from = driver.module_path("app", "core");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"{from}\"]\nforbid = [\"{external}\"]\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("a forbidden external dependency must fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("forbidden external crate") {
        return Err(format!("report must list the forbidden external crate:\n{stdout}"));
    }
    Ok(())
}

fn module_outside_forbid_list_passes(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules
        .insert("app".into(), vec!["core".into(), "ui".into()]);
    let external = external_dep(driver);
    let other = other_dep(driver);
    tree.module_usings
        .push(("app".into(), "core".into(), external.into()));
    tree.module_usings
        .push(("app".into(), "ui".into(), other.into()));
    tree.packages
        .insert("app".into(), vec![external.into(), other.into()]);
    driver.materialize(fx, &tree);
    let unit = driver.unit_name("app");
    let from_ui = driver.module_path("app", "ui");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"{from_ui}\"]\nforbid = [\"{external}\"]\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("matches source model") {
        return Err(format!("verify must confirm the match:\n{stdout}"));
    }
    Ok(())
}

/// A misaligned reference (`Microsoft.IdentityModel.JsonWebTokens` package,
/// `Microsoft.IdentityModel.Tokens` namespace) is a dependency fact: a forbid
/// rule naming the exact package id fires on the importing module.
fn forbid_by_package_name_fires_on_misaligned_namespace(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["Core".into()]);
    tree.module_usings
        .push(("app".into(), "Core".into(), "Microsoft.IdentityModel.Tokens".into()));
    tree.packages
        .insert("app".into(), vec!["Microsoft.IdentityModel.JsonWebTokens".into()]);
    driver.materialize(fx, &tree);
    let unit = driver.unit_name("app");
    let from = driver.module_path("app", "Core");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"{from}\"]\nforbid = [\"Microsoft.IdentityModel.JsonWebTokens\"]\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    let stdout = common::stdout(&output);
    if output.status.code() == Some(0) || !stdout.contains("forbidden external crate") {
        return Err(format!(
            "forbid by package id must fire on the misaligned case (exit {:?}, stdout: {stdout})",
            output.status.code()
        ));
    }
    if !stdout.contains("Microsoft.IdentityModel.JsonWebTokens") {
        return Err(format!(
            "finding must name the package identity, not a namespace:\n{stdout}"
        ));
    }
    Ok(())
}

/// The aligned case: forbidding `fluentassertions` catches a module importing
/// `FluentAssertions`. FluentAssertions-only keeps the project in production
/// (no runner evidence), so the ban has a production module to guard.
fn forbid_by_package_name_fires_on_aligned_namespace(
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
    let unit = driver.unit_name("app");
    let from = driver.module_path("app", "Core");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"{from}\"]\nforbid = [\"fluentassertions\"]\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    let stdout = common::stdout(&output);
    if output.status.code() == Some(0)
        || !stdout.contains("forbidden external crate")
        || !stdout.contains("-> fluentassertions")
    {
        return Err(format!(
            "forbid by aligned package id must fire naming the id (exit {:?}, stdout: {stdout})",
            output.status.code()
        ));
    }
    Ok(())
}

/// Two referenced packages share the `Microsoft` root; the imported namespace
/// matches `Microsoft.IdentityModel.JsonWebTokens` longest, so a rule naming
/// the prefix-sharing `Microsoft.Extensions.Logging` stays clean.
fn forbid_prefix_sharing_package_does_not_fire_on_longest_match(
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
    let unit = driver.unit_name("app");
    let from = driver.module_path("app", "Core");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"{from}\"]\nforbid = [\"Microsoft.Extensions.Logging\"]\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("matches source model") {
        return Err(format!("verify must confirm the match:\n{stdout}"));
    }
    Ok(())
}

/// Declared-but-unused: forbid-by-package on a reference no using surfaces
/// passes (no usage attribution) while the manifests still declare it.
fn unused_reference_absent_from_module_external_but_in_manifests(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert("app".into(), vec!["Core".into()]);
    tree.module_usings
        .push(("app".into(), "Core".into(), "Newtonsoft.Json".into()));
    tree.packages
        .insert("app".into(), vec!["Serilog".into(), "Newtonsoft.Json".into()]);
    driver.materialize(fx, &tree);
    let model = driver.scan(fx);
    let unit = driver.unit_name("app");
    let deps = model["unit_manifests"]
        .get(&unit)
        .and_then(|facts| facts.get("dependencies"))
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("dependencies for {unit} missing"))?;
    if !deps.iter().any(|d| d.as_str() == Some("Serilog")) {
        return Err(format!("Serilog must stay declared in manifests: {deps:?}"));
    }
    let attributed: Vec<&str> = model["module_external"]
        .as_object()
        .map(|map| {
            map.values()
                .filter_map(|list| list.as_array())
                .flatten()
                .filter_map(|name| name.as_str())
                .collect()
        })
        .unwrap_or_default();
    if attributed.contains(&"Serilog") {
        return Err(format!(
            "unused reference must not reach module_external: {attributed:?}"
        ));
    }
    let from = driver.module_path("app", "Core");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_external_crates\"\nfrom = [\"{from}\"]\nforbid = [\"Serilog\"]\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("matches source model") {
        return Err(format!("unused-reference forbid must stay clean:\n{stdout}"));
    }
    Ok(())
}

fn forbidden_dependency_in_manifest_is_reported(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    let external = external_dep(driver);
    tree.packages.insert("app".into(), vec![external.into()]);
    driver.materialize(fx, &tree);
    let unit = driver.unit_name("app");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"{external}\"]\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("a forbidden manifest dependency must fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("forbidden dependency") {
        return Err(format!(
            "report must list the forbidden manifest dependency:\n{stdout}"
        ));
    }
    Ok(())
}

fn manifest_matching_constraint_verifies_clean(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    let external = external_dep(driver);
    let other = other_dep(driver);
    tree.packages.insert("app".into(), vec![external.into()]);
    driver.materialize(fx, &tree);
    let unit = driver.unit_name("app");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{unit}\"] }}\n\n\
             [[constraint]]\ntype = \"manifest_integrity\"\nforbidden_dependencies = [\"{other}\"]\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("matches source model") {
        return Err(format!("verify must confirm the match:\n{stdout}"));
    }
    Ok(())
}

fn import_of_facade_root_reexport_is_violation(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    fx.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write("src/lib.rs", "mod engine;\npub use engine::Thing;\n");
    fx.write(
        "src/engine.rs",
        "pub struct Thing;\nuse crate::Thing;\npub fn make() -> Thing { Thing }\n",
    );
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"umbrella\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"engine\"]\n\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[module.allowed]\ndepend_on = [\"umbrella\"]\n",
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("an internal import through a facade root must fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("facade dependency: app::engine -> app") {
        return Err(format!("report must name the facade dependency:\n{stdout}"));
    }
    if stdout.contains("disallowed cross-component dependency") {
        return Err(format!(
            "the declared depend_on keeps the pair legal for the boundary check:\n{stdout}"
        ));
    }
    Ok(())
}

fn canonical_import_stays_clean_under_active_facade(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    fx.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write(
        "src/lib.rs",
        "mod engine;\nmod client;\npub use engine::Thing;\npub use client::run;\n",
    );
    fx.write("src/engine.rs", "pub struct Thing;\n");
    fx.write(
        "src/client.rs",
        "use crate::engine::Thing;\npub fn run() -> Thing { Thing }\n",
    );
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"umbrella\"\nmatches = { units = [\"app\"] }\n\n[module.allowed]\ndepend_on = [\"engine\", \"client\"]\n\n[[module]]\nname = \"engine\"\nmatches = { modules = [\"app::engine\"] }\n\n[[module]]\nname = \"client\"\nmatches = { modules = [\"app::client\"] }\n\n[module.allowed]\ndepend_on = [\"engine\"]\n",
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if stdout.contains("facade") {
        return Err(format!("canonical imports never touch the facade root:\n{stdout}"));
    }
    Ok(())
}

/// The inert side of the `root-facade` row: on every driver the table marks
/// not-emitted (csharp, go) verify must SAY the rule is inert instead of
/// silently passing it. The note is output only: the verdict and the exit
/// status stay what they were without it. The firing side of the same rule is
/// `import_of_facade_root_reexport_is_violation` (the one driver emitting the
/// fact at full granularity).
fn facade_rule_note_announces_facade_tierless_driver(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    driver.materialize(fx, &driver.probe_tree());
    fx.write("architecture.spec.toml", &boundary_spec(driver));
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    let note = format!(
        "note: facade dependency rule inert for {}: driver emits no root-facade fact",
        driver.language.as_str()
    );
    if !stdout.contains(&note) {
        return Err(format!("run must carry the inert-rule note {note}:\n{stdout}"));
    }
    if !stdout.contains("matches source model") || stdout.contains("facade dependency:") {
        return Err(format!(
            "the note must not enter the verdict or become a finding:\n{stdout}"
        ));
    }
    Ok(())
}

fn worktree_free_go_tree_announces_laundering_capability(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    // A single go.mod tree (no go.work): the scan emits no module tier, so
    // the rule can fire only from declared grouping. The note and the
    // finding must never pair: a run where the rule fired announces through
    // the finding (a fired rule is not inert), a run that reported nothing
    // announces through the tier note.
    materialize_go_declared_grouping(fx);
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n\
         [[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\
         [module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"store\"]\n\n\
         [[module]]\nname = \"shell\"\nmatches = { units = [\"example.com/demo/shell\"] }\n\
         [module.allowed]\ndepend_on = [\"store\"]\n\n\
         [[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n",
    );
    let finding = "laundered forbidden edge: a -> store via shell";
    let note = "note: laundered forbidden edge rule: no native module tier this run \
                (grouping derived from spec declarations)";
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains(finding) {
        return Err(format!("the fired run must report the finding {finding}:\n{stdout}"));
    }
    if stdout.contains(note) || stdout.contains("laundered forbidden edge rule inert") {
        return Err(format!(
            "a fired rule must announce through its finding, not a note:\n{stdout}"
        ));
    }
    // The same tree declared clean: the rule runs, reports nothing, and the
    // run then states the tier absence instead.
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n\
         [[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\
         [module.allowed]\ndepend_on = [\"shell\"]\n\n\
         [[module]]\nname = \"shell\"\nmatches = { units = [\"example.com/demo/shell\"] }\n\
         [module.allowed]\ndepend_on = [\"store\"]\n\n\
         [[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n",
    );
    let clean = expect_success(driver, fx, &["verify"])?;
    let clean_stdout = common::stdout(&clean);
    if clean_stdout.contains(finding) {
        return Err(format!("declared-clean must report no finding:\n{clean_stdout}"));
    }
    if !clean_stdout.contains(note) {
        return Err(format!(
            "a run reporting nothing must carry the tier note {note}:\n{clean_stdout}"
        ));
    }
    Ok(())
}

fn forbidden_edge_routed_through_conduit_hop_is_laundered(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    if driver.language == Language::Go {
        // Go has no soft module tier in the logical-tree shape, so the same
        // conduit runs through the declared-grouping derivation instead:
        // packages a -> shell -> store, ban a -> store, shell claimed only by
        // `matches.units` — the same conduit the rust probe reports.
        materialize_go_declared_grouping(fx);
        fx.write(
            "architecture.spec.toml",
            "[project]\nlanguage = \"go\"\n\n\
             [[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\
             [module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"store\"]\n\n\
             [[module]]\nname = \"shell\"\nmatches = { units = [\"example.com/demo/shell\"] }\n\
             [module.allowed]\ndepend_on = [\"store\"]\n\n\
             [[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n",
        );
        let finding = "laundered forbidden edge: a -> store via shell";
        let output = driver.run(fx, &["verify"]);
        if output.status.code() != Some(0) {
            return Err(format!(
                "a warning-level finding is tolerated without --strict (stderr: {})",
                common::stderr(&output)
            ));
        }
        if !common::stdout(&output).contains(finding) {
            return Err(format!(
                "warning must name the laundered route:\n{}",
                common::stdout(&output)
            ));
        }
        let strict = driver.run(fx, &["verify", "--strict"]);
        if strict.status.code() == Some(0) {
            return Err("--strict must promote the laundering warning".into());
        }
        if !common::stdout(&strict).contains(finding) {
            return Err(format!(
                "--strict report must keep the finding without the prefix:\n{}",
                common::stdout(&strict)
            ));
        }
        return Ok(());
    }
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert(
        "app".into(),
        vec!["a".into(), "b".into(), "hidden".into(), "shell".into()],
    );
    tree.module_usings
        .push(("app".into(), "a".into(), "hidden".into()));
    tree.module_usings
        .push(("app".into(), "hidden".into(), "b".into()));
    driver.materialize(fx, &tree);
    let app = driver.unit_name("app");
    let a = driver.module_path("app", "a");
    let b = driver.module_path("app", "b");
    let shell = driver.module_path("app", "shell");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ modules = [\"{a}\"] }}\n\
             [module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"b\"]\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ modules = [\"{b}\"] }}\n\n\
             [[module]]\nname = \"shell\"\nmatches = {{ modules = [\"{shell}\"], units = [\"{app}\"] }}\n\
             [module.allowed]\ndepend_on = [\"b\"]\n",
            driver.language.as_str()
        ),
    );
    let finding = "laundered forbidden edge: a -> b via shell";
    let output = driver.run(fx, &["verify"]);
    if output.status.code() != Some(0) {
        return Err(format!(
            "a warning-level finding is tolerated without --strict (stderr: {})",
            common::stderr(&output)
        ));
    }
    let stdout = common::stdout(&output);
    if !stdout.contains(finding) {
        return Err(format!("warning must name the laundered route:\n{stdout}"));
    }
    let strict = driver.run(fx, &["verify", "--strict"]);
    if strict.status.code() == Some(0) {
        return Err("--strict must promote the laundering warning".into());
    }
    if !common::stdout(&strict).contains(finding) {
        return Err(format!(
            "--strict report must keep the finding without the prefix:\n{}",
            common::stdout(&strict)
        ));
    }
    Ok(())
}

/// The laundering check traces banned routes on the boundary pair graph
/// only, so the intermediate's territory must be claimed by a boundary —
/// explicitly via `matches.modules` or through its unit's `matches.units`
/// fallback — for the bypass to be visible. Claimed conduit: the route is on
/// the graph and the ban launders. Unclaimed conduit: the intermediate's
/// edges have no owner, leave the graph (unowned-endpoint warnings; on go the
/// package itself is an unexpected component), and the check never sees the
/// bypass. Pins the documented precondition; both halves are current behavior.
fn laundering_traces_only_claimed_conduit_territory(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    if driver.language == Language::Go {
        materialize_go_declared_grouping(fx);
        // Claimed conduit: every package on the a -> shell -> store route is
        // claimed by a declared module, so the route exists on the pair graph
        // and the ban a -> store launders through it.
        fx.write(
            "architecture.spec.toml",
            "[project]\nlanguage = \"go\"\n\n\
             [[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\
             [module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"store\"]\n\n\
             [[module]]\nname = \"shell\"\nmatches = { units = [\"example.com/demo/shell\"] }\n\
             [module.allowed]\ndepend_on = [\"store\"]\n\n\
             [[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n",
        );
        let finding = "laundered forbidden edge: a -> store via shell";
        let output = expect_success(driver, fx, &["verify"])?;
        if !common::stdout(&output).contains(finding) {
            return Err(format!(
                "a claimed conduit must launder the ban {finding}:\n{}",
                common::stdout(&output)
            ));
        }
        // Unclaimed conduit: dropping the shell declaration leaves the
        // intermediate claimed by nothing — its edges never reach the pair
        // graph, so the same ban produces no laundered line and the package
        // surfaces as an unexpected component instead.
        fx.write(
            "architecture.spec.toml",
            "[project]\nlanguage = \"go\"\n\n\
             [[module]]\nname = \"a\"\nmatches = { units = [\"example.com/demo/a\"] }\n\
             [module.allowed]\nforbidden = [\"store\"]\n\n\
             [[module]]\nname = \"store\"\nmatches = { units = [\"example.com/demo/store\"] }\n",
        );
        let output = driver.run(fx, &["verify"]);
        if output.status.code() == Some(0) {
            return Err(
                "the unclaimed conduit package must surface as an unexpected component".into(),
            );
        }
        let stdout = common::stdout(&output);
        if !stdout.contains("unexpected component: example.com/demo/shell") {
            return Err(format!(
                "the bypass must surface through the unclaimed package:\n{stdout}"
            ));
        }
        if stdout.contains("laundered forbidden edge:") {
            return Err(format!(
                "edges outside the pair graph must not be traced as a laundered route:\n{stdout}"
            ));
        }
        return Ok(());
    }
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert(
        "app".into(),
        vec!["a".into(), "b".into(), "hidden".into(), "shell".into()],
    );
    tree.module_usings
        .push(("app".into(), "a".into(), "hidden".into()));
    tree.module_usings
        .push(("app".into(), "hidden".into(), "b".into()));
    driver.materialize(fx, &tree);
    let app = driver.unit_name("app");
    let a = driver.module_path("app", "a");
    let b = driver.module_path("app", "b");
    let shell = driver.module_path("app", "shell");
    // Claimed conduit: `shell` claims the unit, so the unclaimed module
    // `hidden` rides the unit fallback, its hops land on the pair graph, and
    // the ban a -> b launders through them.
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ modules = [\"{a}\"] }}\n\
             [module.allowed]\ndepend_on = [\"shell\"]\nforbidden = [\"b\"]\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ modules = [\"{b}\"] }}\n\n\
             [[module]]\nname = \"shell\"\nmatches = {{ modules = [\"{shell}\"], units = [\"{app}\"] }}\n\
             [module.allowed]\ndepend_on = [\"b\"]\n",
            driver.language.as_str()
        ),
    );
    let finding = "laundered forbidden edge: a -> b via shell";
    let output = expect_success(driver, fx, &["verify"])?;
    if !common::stdout(&output).contains(finding) {
        return Err(format!(
            "a claimed conduit must launder the ban {finding}:\n{}",
            common::stdout(&output)
        ));
    }
    // Unclaimed conduit: with no boundary claiming the unit or the conduit's
    // module path, the intermediate's edges have no owner and leave the pair
    // graph — the ban passes with no laundered line, surfaced only as
    // unowned-endpoint warnings (the unit itself is accounted for through
    // `a`'s module claim).
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ modules = [\"{a}\"] }}\n\
             [module.allowed]\nforbidden = [\"b\"]\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ modules = [\"{b}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if stdout.contains("laundered forbidden edge:") {
        return Err(format!(
            "an unclaimed conduit must not produce a laundered line:\n{stdout}"
        ));
    }
    if !stdout.contains("unowned module edge endpoint") {
        return Err(format!(
            "the conduit's ownerless edges must surface as warnings:\n{stdout}"
        ));
    }
    Ok(())
}

fn submodule_contract_leak_reported_under_submodule_path(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    if driver.language == Language::Go {
        // No soft tier in the logical-tree shape for go, so the submodule
        // boundary claims the entity package by import path — the declared
        // grouping derivation gives the contract its surface.
        materialize_go_declared_submodule(fx);
        fx.write(
            "architecture.spec.toml",
            "[project]\nlanguage = \"go\"\n\n\
             [[stereotype]]\nname = \"entity\"\nmatch = { paths = [\"example.com/demo/auth/entity\"] }\n\n\
             [[module]]\nname = \"auth\"\nmatches = { units = [\"example.com/demo/auth\", \"example.com/demo/auth/entity\"] }\n\n\
             [[module.submodules]]\nname = \"auth::entity\"\nmatches = { modules = [\"example.com/demo/auth/entity\"] }\n\
             contract = { forbid = [\"entity\"] }\n",
        );
        let output = driver.run(fx, &["verify"]);
        if output.status.code() == Some(0) {
            return Err(
                "a contract.forbid stereotype surfacing under the submodule boundary must fail verify".into(),
            );
        }
        let expected = "contract leak: auth::entity exposes entity (forbidden)";
        if !common::stdout(&output).contains(expected) {
            return Err(format!(
                "leak must be reported under the submodule path auth::entity:\n{}",
                common::stdout(&output)
            ));
        }
        return Ok(());
    }
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into()];
    tree.modules
        .insert("a".into(), vec!["b".into(), "b.c".into()]);
    driver.materialize(fx, &tree);
    let a = driver.unit_name("a");
    let ab = driver.module_path("a", "b");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\n\
             [[stereotype]]\nname = \"c\"\nmatch = {{ names = [\"c\"] }}\n\n\
             [[module.submodules]]\nname = \"{ab}\"\nmatches = {{ modules = [\"{ab}\"] }}\n\
             contract = {{ forbid = [\"c\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("a contract.forbid stereotype surfacing under the submodule boundary must fail verify".into());
    }
    let stdout = common::stdout(&output);
    let expected = format!("contract leak: {ab} exposes c (forbidden)");
    if !stdout.contains(&expected) {
        return Err(format!("leak must be reported under the submodule path {ab}:\n{stdout}"));
    }
    Ok(())
}

/// Go submodule contracts match units through glob patterns against FULL
/// import paths: a go unit's name is the whole import path, the bare-segment
/// concession of the path-matching rule addresses `::`-separated module paths
/// (which go paths never contain), and the contract surface is claimed
/// membership — so the parent module must claim its packages. Pins both
/// halves of the documented trap: the working shape (full-path glob +
/// claimed parent) fires, the bare-name stereotype matches no go unit and the
/// contract reads inert.
fn go_submodule_contract_stereotype_needs_full_path_glob(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    materialize_go_declared_submodule(fx);
    // Working shape: the stereotype pattern globs the full import path, the
    // parent claims both packages, the submodule boundary addresses the
    // entity package — the contract engages and leaks.
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n\
         [[stereotype]]\nname = \"entity\"\nmatch = { paths = [\"example.com/demo/**/entity\"] }\n\n\
         [[module]]\nname = \"auth\"\nmatches = { units = [\"example.com/demo/auth\", \"example.com/demo/auth/entity\"] }\n\n\
         [[module.submodules]]\nname = \"auth::entity\"\nmatches = { modules = [\"example.com/demo/auth/entity\"] }\n\
         contract = { forbid = [\"entity\"] }\n",
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err(
            "a full-path glob stereotype under a claimed parent must fire the contract leak".into(),
        );
    }
    let expected = "contract leak: auth::entity exposes entity (forbidden)";
    if !common::stdout(&output).contains(expected) {
        return Err(format!("leak must be reported under the submodule path:\n{}", common::stdout(&output)));
    }
    // The trap: the identical spec with a bare-name stereotype. The pattern
    // matches no go unit (no `::` to strip a bare segment from), the contract
    // enforces over nothing, and verify passes — a silent no-op, not a leak.
    fx.write(
        "architecture.spec.toml",
        "[project]\nlanguage = \"go\"\n\n\
         [[stereotype]]\nname = \"entity\"\nmatch = { names = [\"entity\"] }\n\n\
         [[module]]\nname = \"auth\"\nmatches = { units = [\"example.com/demo/auth\", \"example.com/demo/auth/entity\"] }\n\n\
         [[module.submodules]]\nname = \"auth::entity\"\nmatches = { modules = [\"example.com/demo/auth/entity\"] }\n\
         contract = { forbid = [\"entity\"] }\n",
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if stdout.contains("contract leak") {
        return Err(format!(
            "a bare-name stereotype matches no go unit and must not leak:\n{stdout}"
        ));
    }
    Ok(())
}
/// The ceiling half of boundary semantics (spec.md: allowed.depend_on is the
/// exact set): an extracted edge with no declaration fails with the
/// cross-component label; declaring the ban instead reroutes the same edge to
/// the forbidden-edge label without the ceiling complaint.
fn undeclared_dependency_exceeds_allowed_ceiling(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    tree.hard_edges.push(("a".into(), "b".into()));
    driver.materialize(fx, &tree);
    let a = driver.unit_name("a");
    let b = driver.unit_name("b");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("verify must fail when an extracted edge exceeds every declaration".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("disallowed cross-component dependency: a -> b") {
        return Err(format!(
            "report must name the ceiling finding:\n{stdout}"
        ));
    }
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\
             [module.allowed]\nforbidden = [\"b\"]\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("a declared ban on an extracted edge must still fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("forbidden edge: a -> b") {
        return Err(format!("report must name the forbidden edge:\n{stdout}"));
    }
    if stdout.contains("disallowed cross-component dependency") {
        return Err(format!(
            "the declared ban supersedes the ceiling complaint:\n{stdout}"
        ));
    }
    Ok(())
}

/// One undeclared `depend_on` target whose name exists as a module in the
/// source for granular soft-tier drivers but is invisible to go (whose
/// capability row carries no module-tier fact on this tree shape): the same
/// spec line classifies as a source module rust/c# can point at versus a
/// reference go cannot verify from source — the table, not language lore,
/// decides whether non-existence may be claimed.
fn dead_reference_to_source_module_classifies_per_language(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    tree.hard_edges.push(("a".into(), "b".into()));
    tree.modules.insert("a".into(), vec!["core".into()]);
    driver.materialize(fx, &tree);
    let a = driver.unit_name("a");
    let b = driver.unit_name("b");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\
             [module.allowed]\ndepend_on = [\"b\", \"core\"]\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let variant = match driver.language {
        Language::Go => "not verifiable from source",
        Language::Rust | Language::Csharp => "exists in source but undeclared",
    };
    let output = driver.run(fx, &["verify"]);
    if output.status.code() != Some(0) {
        return Err(format!(
            "a warning-level dead reference is tolerated without --strict (stdout: {})",
            common::stdout(&output)
        ));
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("warning: dead reference:") || !stdout.contains(variant) {
        return Err(format!(
            "report must warn with the {variant} classification:\n{stdout}"
        ));
    }
    let strict = driver.run(fx, &["verify", "--strict"]);
    if strict.status.code() == Some(0) {
        return Err("--strict must promote the dead reference".into());
    }
    let stdout = common::stdout(&strict);
    if !stdout.contains("dead reference:") || stdout.contains("warning: dead reference:") {
        return Err(format!(
            "--strict report must keep the finding without the warning prefix:\n{stdout}"
        ));
    }
    Ok(())
}

fn undeclared_unit_reports_unexpected_component(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    driver.materialize(fx, &tree);
    let a = driver.unit_name("a");
    let b = driver.unit_name("b");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("verify must fail on an undeclared source unit".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains(&format!("unexpected component: {b}")) {
        return Err(format!("report must name the unexpected component {b}:\n{stdout}"));
    }
    Ok(())
}

/// A unit whose name nests under a declared component (`a.Tests` under `a`) is
/// reported unassigned.
///
/// contract-plan: go scopes units by directory path, so a dotted stem is an
/// unrelated package there and lands in the unexpected-component bucket
/// instead; unit-path nesting will align when go adopts hierarchical units.
fn dotted_subunit_of_declared_component_is_unassigned(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "a.Tests".into()];
    driver.materialize(fx, &tree);
    let a = driver.unit_name("a");
    let tests = driver.unit_name("a.Tests");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("verify must fail on an unassigned nested unit".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains(&format!("unassigned unit: {tests}")) {
        return Err(format!("report must name the unassigned unit {tests}:\n{stdout}"));
    }
    Ok(())
}

fn two_boundaries_claim_module_at_equal_specificity(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules
        .insert("app".into(), vec!["Core".into()]);
    driver.materialize(fx, &tree);
    let app = driver.unit_name("app");
    let core = driver.module_path("app", "Core");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"one\"\nmatches = {{ modules = [\"{core}\"], units = [\"{app}\"] }}\n\n\
             [[module]]\nname = \"two\"\nmatches = {{ modules = [\"{core}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("equal-specificity double claims must fail verify".into());
    }
    let stdout = common::stdout(&output);
    let expected = format!("ambiguous module match: {core} claimed by both");
    if !stdout.contains(&expected) {
        return Err(format!("report must name the ambiguous claim on {core}:\n{stdout}"));
    }
    Ok(())
}

fn unclaimed_module_edge_endpoint_is_reported_unowned(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert(
        "app".into(),
        vec!["a".into(), "hidden".into(), "shell".into()],
    );
    tree.module_usings
        .push(("app".into(), "a".into(), "hidden".into()));
    driver.materialize(fx, &tree);
    let a = driver.module_path("app", "a");
    let hidden = driver.module_path("app", "hidden");
    let shell = driver.module_path("app", "shell");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ modules = [\"{a}\"] }}\n\n\
             [[module]]\nname = \"shell\"\nmatches = {{ modules = [\"{shell}\"] }}\n",
            driver.language.as_str()
        ),
    );
    let finding = format!("unowned module edge endpoint: {hidden}");
    let output = driver.run(fx, &["verify"]);
    if output.status.code() != Some(0) {
        return Err(format!(
            "a warning-level unowned endpoint is tolerated without --strict (stdout: {})",
            common::stdout(&output)
        ));
    }
    let stdout = common::stdout(&output);
    if !stdout.contains(&format!("warning: {finding}")) {
        return Err(format!("warning must name the unowned endpoint:\n{stdout}"));
    }
    let strict = driver.run(fx, &["verify", "--strict"]);
    if strict.status.code() == Some(0) {
        return Err("--strict must promote the unowned endpoint warning".into());
    }
    let stdout = common::stdout(&strict);
    if !stdout.contains(&finding) || stdout.contains(&format!("warning: {finding}")) {
        return Err(format!(
            "--strict report must keep the finding without the warning prefix:\n{stdout}"
        ));
    }
    Ok(())
}

fn top_level_contract_forbid_stereotype_surfaced_by_unit(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["vaultc".into()];
    driver.materialize(fx, &tree);
    let vaultc = driver.unit_name("vaultc");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"vault\"\nmatches = {{ units = [\"{vaultc}\"] }}\n\
             [module.contract]\nforbid = [\"ledger\"]\n\n\
             [[stereotype]]\nname = \"ledger\"\nmatch = {{ names = [\"*vaultc\"] }}\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("a forbidden stereotype surfaced by the claimed unit name must fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("contract leak: vault exposes ledger (forbidden)") {
        return Err(format!("report must name the top-level contract leak:\n{stdout}"));
    }
    Ok(())
}

/// The #29 per-pattern finding on the drivers that can read declarations
/// (rust): a gate naming no declared module is a real spec error there.
fn feature_boundary_gated_pattern_without_declaration_fires_where_emitted(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    driver.materialize(fx, &tree);
    let app = driver.unit_name("app");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{app}\"] }}\n\n\
             [[constraint]]\ntype = \"feature_boundary\"\nfeature = \"experimental\"\ngated_modules = [\"ghost\"]\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("a gate naming no declared module must fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("feature boundary: gated module pattern 'ghost' matches no declared module") {
        return Err(format!("report must name the unmatchable gate:\n{stdout}"));
    }
    Ok(())
}

/// The inert side of the same constraint: where the capability table says the
/// driver emits no root-module-declarations fact, verify must not invent the
/// per-pattern (#29) error from an empty declaration map — the constraint
/// surfaces once as a capability-reason vacuous warning (exit 0, --strict
/// promotes) and no `feature boundary:` line appears at all.
fn feature_boundary_missing_declarations_fact_surfaces_vacuous(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    driver.materialize(fx, &tree);
    let app = driver.unit_name("app");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{app}\"] }}\n\n\
             [[constraint]]\ntype = \"feature_boundary\"\nfeature = \"experimental\"\ngated_modules = [\"ghost\"]\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() != Some(0) {
        return Err(format!(
            "a constraint that cannot engage is vacuous, not an error (stdout: {})",
            common::stdout(&output)
        ));
    }
    let stdout = common::stdout(&output);
    let expected = "vacuous constraint: [constraint #1] feature_boundary: driver emits no root-module-declarations fact (capability not-emitted): the constraint cannot engage";
    if !stdout.contains(expected) {
        return Err(format!("report must state the capability vacuity once:\n{stdout}"));
    }
    if stdout.contains("feature boundary:") {
        return Err(format!("no per-pattern #29 finding may appear:\n{stdout}"));
    }
    let strict = driver.run(fx, &["verify", "--strict"]);
    if strict.status.code() == Some(0) {
        return Err("--strict must promote the vacuous constraint".into());
    }
    let stdout = common::stdout(&strict);
    if !stdout.contains(expected) || stdout.contains("warning: vacuous constraint:") {
        return Err(format!(
            "--strict report must keep the vacuous finding without the warning prefix:\n{stdout}"
        ));
    }
    Ok(())
}

fn forbidden_submodule_dependency_across_sibling_children(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert(
        "app".into(),
        vec!["parent".into(), "parent.x".into(), "parent.y".into()],
    );
    tree.module_usings
        .push(("app".into(), "parent.x".into(), "parent.y".into()));
    driver.materialize(fx, &tree);
    let app = driver.unit_name("app");
    let parent = driver.module_path("app", "parent");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{app}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_submodule_dependency\"\nparent = \"{parent}\"\nfrom = [\"x\"]\nforbid = [\"y\"]\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("a banned sibling-submodule edge must fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("forbidden submodule dependency: parent::x -> parent::y") {
        return Err(format!(
            "report must name the banned sibling edge under the parent:\n{stdout}"
        ));
    }
    Ok(())
}

fn forbidden_submodule_dependency_full_path_engages(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["app".into()];
    tree.modules.insert(
        "app".into(),
        vec!["parent".into(), "parent.x".into(), "parent.y".into()],
    );
    tree.module_usings
        .push(("app".into(), "parent.x".into(), "parent.y".into()));
    driver.materialize(fx, &tree);
    let app = driver.unit_name("app");
    let parent = driver.module_path("app", "parent");
    let from = driver.module_path("app", "parent.x");
    let forbid = driver.module_path("app", "parent.y");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"app\"\nmatches = {{ units = [\"{app}\"] }}\n\n\
             [[constraint]]\ntype = \"forbid_submodule_dependency\"\nparent = \"{parent}\"\nfrom = [\"{from}\"]\nforbid = [\"{forbid}\"]\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err(
            "a full module-path from/forbid must engage, not read as a silently inert rule".into(),
        );
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("forbidden submodule dependency: parent::x -> parent::y") {
        return Err(format!(
            "full-path form must report the same banned sibling edge as the bare form:\n{stdout}"
        ));
    }
    if stdout.contains("vacuous constraint") {
        return Err(format!(
            "an engaged full-path rule must not read vacuous:\n{stdout}"
        ));
    }
    Ok(())
}

fn rust_api_spec(body: &str) -> String {
    format!(
        "[project]\nlanguage = \"rust\"\n\n[[module]]\nname = \"app\"\nmatches = {{ units = [\"app\"] }}\n\n{body}"
    )
}

fn rust_root_fixture(fx: &common::Fixture, lib: &str) {
    fx.write(
        "Cargo.toml",
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    fx.write("src/lib.rs", lib);
}

fn rust_root_public_api_leak_pin(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    rust_root_fixture(fx, "pub fn serve() {}\npub mod auth { pub fn x() {} }\n");
    fx.write(
        "architecture.spec.toml",
        &rust_api_spec("[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"serve\"]\n"),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("an unallowlisted public root export must fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("public api leak: app exposes auth (not allowlisted)") {
        return Err(format!("report must name the leaking root export:\n{stdout}"));
    }
    Ok(())
}

fn rust_unverifiable_glob_export_pin(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    rust_root_fixture(fx, "pub fn serve() {}\npub use ghost::*;\n");
    fx.write(
        "architecture.spec.toml",
        &rust_api_spec("[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"serve\"]\n"),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("an unresolvable root glob must fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("unverifiable glob export: app exposes ghost::*") {
        return Err(format!("report must name the unverifiable glob:\n{stdout}"));
    }
    Ok(())
}

fn rust_empty_glob_export_pin(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    rust_root_fixture(fx, "mod empty_mod {}\npub use empty_mod::*;\n");
    fx.write(
        "architecture.spec.toml",
        &rust_api_spec(
            "[[constraint]]\ntype = \"public_api_allowlist\"\nallowed = [\"empty_mod\"]\n",
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() == Some(0) {
        return Err("a resolvable glob exporting zero public items must fail verify".into());
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("empty glob export: app exposes empty_mod::*") {
        return Err(format!("report must name the empty glob:\n{stdout}"));
    }
    Ok(())
}

fn rust_unresolved_module_file_pin(driver: &Driver, fx: &common::Fixture) -> Result<(), String> {
    rust_root_fixture(fx, "mod ghost;\n");
    fx.write("architecture.spec.toml", &rust_api_spec(""));
    let output = driver.run(fx, &["verify"]);
    if output.status.code() != Some(0) {
        return Err(format!(
            "a warning-level unresolved module file is tolerated without --strict (stdout: {})",
            common::stdout(&output)
        ));
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("warning: unresolved module file: app::ghost") {
        return Err(format!("warning must name the unresolved module file:\n{stdout}"));
    }
    let strict = driver.run(fx, &["verify", "--strict"]);
    if strict.status.code() == Some(0) {
        return Err("--strict must promote the unresolved module file".into());
    }
    let stdout = common::stdout(&strict);
    if !stdout.contains("unresolved module file: app::ghost")
        || stdout.contains("warning: unresolved module file:")
    {
        return Err(format!(
            "--strict report must keep the finding without the warning prefix:\n{stdout}"
        ));
    }
    Ok(())
}

fn warning_severity_cycle_routes_to_warning_bucket(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    tree.hard_edges = vec![("a".into(), "b".into()), ("b".into(), "a".into())];
    driver.materialize(fx, &tree);
    let a = driver.unit_name("a");
    let b = driver.unit_name("b");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\
             [module.allowed]\ndepend_on = [\"b\"]\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}\n\
             [module.allowed]\ndepend_on = [\"a\"]\n\n\
             [[constraint]]\ntype = \"no_cycles\"\nmodules = [\"a\", \"b\"]\nseverity = \"warning\"\n",
            driver.language.as_str()
        ),
    );
    let output = driver.run(fx, &["verify"]);
    if output.status.code() != Some(0) {
        return Err(format!(
            "a warning-severity cycle must not fail plain verify (stdout: {})",
            common::stdout(&output)
        ));
    }
    let stdout = common::stdout(&output);
    if !stdout.contains("warning: cycle:") {
        return Err(format!("report must warn with the cycle:\n{stdout}"));
    }
    let strict = driver.run(fx, &["verify", "--strict"]);
    if strict.status.code() == Some(0) {
        return Err("--strict must promote the warning-severity cycle".into());
    }
    let stdout = common::stdout(&strict);
    if !stdout.contains("cycle:") || stdout.contains("warning: cycle:") {
        return Err(format!(
            "--strict report must keep the cycle without the warning prefix:\n{stdout}"
        ));
    }
    Ok(())
}

fn vacuous_cycle_constraint_surfaces_edgeless_group(
    driver: &Driver,
    fx: &common::Fixture,
) -> Result<(), String> {
    let mut tree = LogicalTree::new();
    tree.units = vec!["a".into(), "b".into()];
    driver.materialize(fx, &tree);
    let a = driver.unit_name("a");
    let b = driver.unit_name("b");
    fx.write(
        "architecture.spec.toml",
        &format!(
            "[project]\nlanguage = \"{}\"\n\n\
             [[module]]\nname = \"a\"\nmatches = {{ units = [\"{a}\"] }}\n\n\
             [[module]]\nname = \"b\"\nmatches = {{ units = [\"{b}\"] }}\n\n\
             [[constraint]]\ntype = \"no_cycles\"\nmodules = [\"a\", \"b\"]\n",
            driver.language.as_str()
        ),
    );
    let output = expect_success(driver, fx, &["verify"])?;
    let stdout = common::stdout(&output);
    if !stdout.contains("vacuous constraint:") {
        return Err(format!(
            "an edgeless cycle-check group must surface as a vacuous constraint:\n{stdout}"
        ));
    }
    Ok(())
}
