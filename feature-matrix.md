# Feature Matrix

## scan.units
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **lists_each_declared_unit** — scan reports every materialized logical unit under its concrete unit name
  - rust: pass
  - csharp: pass
  - go: pass
- **records_hard_edge_between_dependent_units** — a declared hard edge between two units is emitted as a model edge on their concrete names
  - rust: pass
  - csharp: pass
  - go: pass
- **keeps_own_member_out_of_external** — a dependency on another unit of the project is an edge, never an external dependency
  - rust: pass
  - csharp: pass
  - go: pass
- **scan_is_deterministic_across_runs** — an unchanged tree scans to byte-identical JSON across runs
  - rust: pass
  - csharp: pass
  - go: pass
- **test_project_absent_from_production_model** — a project whose package set names a test runner contributes no unit, edge, soft tier, or external fact, and the tree minus that project scans to identical JSON
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **test_tier_follows_framework_evidence_not_name** — a no-framework project named LegacyTests stays production while a project named Helpers referencing xunit is test tier
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **test_framework_package_ids_are_the_decisive_evidence** — xunit/nunit-family and Microsoft.NET.Test.Sdk package ids mark the test tier while FluentAssertions-only stays production
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **central_test_reference_marks_tier_version_entries_do_not** — a centrally declared test-runner PackageReference marks the owning project test tier; a version-only pin marks nothing
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **conditional_props_reference_is_not_evidence_or_dependency** — a Condition-bearing central PackageReference is ignored entirely — the production model stays intact and the conditional runner reaches no tier
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **using_dropped_test_project_namespace_stays_out_of_production** — a production using under a dropped test project's namespace produces no edge, module, seed boundary, or verify finding
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)

## scan.soft_structure
**Capability:** rust=implemented, csharp=implemented, go=not-implemented

**Behaviors:**

- **lists_declared_modules_in_soft_structure** — every declared module of a unit appears as a soft_structure path under that unit
  - rust: pass
  - csharp: pass
  - go: skipped (scan.soft_structure not implemented)
- **nests_child_modules_under_parent** — a nested module is listed with its full dotted path, nested under its parent
  - rust: pass
  - csharp: pass
  - go: skipped (scan.soft_structure not implemented)
- **keeps_modules_of_distinct_units_separate** — soft structure is scoped per unit; one unit never lists another unit's modules
  - rust: pass
  - csharp: pass
  - go: skipped (scan.soft_structure not implemented)

## scan.module_edges
**Capability:** rust=implemented, csharp=implemented, go=not-implemented

**Behaviors:**

- **records_module_edge_from_internal_using** — a using/import from one declared module to another records a module edge
  - rust: pass
  - csharp: pass
  - go: skipped (scan.module_edges not implemented)
- **does_not_create_module_edge_to_external_target** — a using/import of an external package produces no module edge
  - rust: pass
  - csharp: pass
  - go: skipped (scan.module_edges not implemented)
- **no_module_edges_for_units_without_soft_reference** — a unit whose modules reference nothing emits no module edges
  - rust: pass
  - csharp: pass
  - go: skipped (scan.module_edges not implemented)

## scan.external
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **lists_manifest_declared_packages_as_external** — manifest-declared packages appear in the external dependency list
  - rust: pass
  - csharp: pass
  - go: pass
- **sorts_and_deduplicates_external_entries** — external dependencies are sorted and deduplicated even when declared twice
  - rust: pass
  - csharp: pass
  - go: pass
- **central_props_package_reaches_external_tier** — a centrally declared PackageReference item reaches the external tier, its module_external attribution, and fires a forbid rule
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **nearest_props_file_overrides_root_for_central_entries** — each project's central entries come from the nearest props file walking upward; a project-level props shadows the root
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **unreferenced_props_entries_stay_out_of_external** — props PackageVersion entries no project references never appear as external dependencies
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)

## scan.module_external
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **attributes_external_dependency_to_its_module** — an external package used by a module is attributed to that module path
  - rust: pass
  - csharp: pass
  - go: pass
- **keeps_external_usage_off_sibling_modules** — only the module that uses an external package records it, never its siblings
  - rust: pass
  - csharp: pass
  - go: skipped (module-tier: go.work tier only (2+ members); declared grouping otherwise)
- **misaligned_package_identity_replaces_namespace** — a using whose namespace misaligns with its referenced package id attributes the package (longest shared prefix), never the namespace; siblings sharing only a shorter prefix stay unattributed and namespaces stay in the soft tier
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **aligned_package_id_attributed_case_insensitively** — an aligned package whose id differs from its namespace only in case attributes the package id as referenced
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **declared_but_unused_package_stays_out_of_module_external** — a referenced package no using surfaces stays out of module_external but present in the unit manifests
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **uncompilable_namespace_absent_from_external_tiers** — a using that maps to no referenced package appears nowhere in the external tiers with no crash or placeholder entry
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **family_root_using_ties_family_sibling_packages** — a using of a family root namespace sharing its longest prefix with the root package and a sibling package extending it attributes both (documented tie)
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)

## scan.unit_manifests
**Capability:** rust=implemented, csharp=implemented, go=not-implemented

**Behaviors:**

- **exposes_dependency_facts_per_unit** — unit_manifests surfaces the manifest-declared packages of each unit
  - rust: pass
  - csharp: pass
  - go: skipped (scan.unit_manifests not implemented)
- **unit_manifests_cover_every_unit** — every materialized unit has an entry in unit_manifests
  - rust: pass
  - csharp: pass
  - go: skipped (scan.unit_manifests not implemented)
- **unit_without_packages_has_empty_dependencies** — a unit with no declared packages reports an empty dependency list
  - rust: pass
  - csharp: pass
  - go: skipped (scan.unit_manifests not implemented)

## verify.module_boundaries
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **unit_boundary_matching_tree_verifies_clean** — a unit-boundary spec declaring every unit with its edges allowed verifies clean
  - rust: pass
  - csharp: pass
  - go: pass
- **module_boundary_with_allowed_dependency_verifies_clean** — a module-boundary spec whose depend_on allows the module edge verifies clean
  - rust: pass
  - csharp: pass
  - go: skipped (module-tier: go.work tier only (2+ members); declared grouping otherwise)
- **undeclared_component_is_reported_missing** — a spec component matching no unit is reported as a missing component
  - rust: pass
  - csharp: pass
  - go: pass
- **stale_allowed_dependency_reports_missing_edge** — a declared depend_on edge absent from the model fails non-strict verify with the canonical missing edge label, and the same tree minus the stale declaration verifies clean
  - rust: pass
  - csharp: pass
  - go: pass
- **facade_rule_note_announces_facade_tierless_driver** — on a driver whose table marks root-facade not-emitted verify states the facade rule inert instead of staying silent, with the verdict and exit status unchanged
  - rust: skipped (root-facade: granular)
  - csharp: pass
  - go: pass
- **undeclared_dependency_exceeds_allowed_ceiling** — an extracted edge no boundary declares fails with the cross-component ceiling label, and an explicit ban reroutes it to the forbidden-edge label
  - rust: pass
  - csharp: pass
  - go: pass
- **dead_reference_to_source_module_classifies_per_language** — an undeclared depend_on target is classified per capability table: granular soft-tier drivers see a source module, a driver without the tier says not verifiable instead of claiming non-existence
  - rust: pass
  - csharp: pass
  - go: pass
- **undeclared_unit_reports_unexpected_component** — a source unit no boundary declares is reported as an unexpected component
  - rust: pass
  - csharp: pass
  - go: pass
- **dotted_subunit_of_declared_component_is_unassigned** — a unit nested under a declared component by name is reported as unassigned
  - rust: pass
  - csharp: pass
  - go: skipped (project shape not applicable)
- **two_boundaries_claim_module_at_equal_specificity** — two boundaries claiming one module at equal specificity fail with the ambiguous-match report
  - rust: pass
  - csharp: pass
  - go: skipped (module-tier: go.work tier only (2+ members); declared grouping otherwise)
- **unclaimed_module_edge_endpoint_is_reported_unowned** — a module edge endpoint no boundary claims is a warning-level unowned endpoint, promoted by --strict
  - rust: pass
  - csharp: pass
  - go: skipped (module-tier: go.work tier only (2+ members); declared grouping otherwise)
- **top_level_contract_forbid_stereotype_surfaced_by_unit** — a boundary contract forbidding a stereotype whose pattern matches the claimed unit name leaks
  - rust: pass
  - csharp: pass
  - go: pass
- **feature_boundary_gated_pattern_without_declaration_fires_where_emitted** — a feature_boundary gate naming no declared module fails on drivers whose table row emits root-module-declarations
  - rust: pass
  - csharp: skipped (root-module-declarations: not-emitted)
  - go: skipped (root-module-declarations: not-emitted)
- **feature_boundary_missing_declarations_fact_surfaces_vacuous** — on a driver emitting no root-module-declarations fact the constraint surfaces once as a capability-reason vacuous warning, never as per-pattern errors
  - rust: skipped (root-module-declarations: granular)
  - csharp: pass
  - go: pass
- **rust_root_public_api_leak_pin** — a public root export outside the allowlist is reported as a public api leak
  - rust: pass
  - csharp: skipped (root-module-declarations: not-emitted)
  - go: skipped (root-module-declarations: not-emitted)
- **rust_unverifiable_glob_export_pin** — a root glob re-export that resolves to nothing is reported as unverifiable
  - rust: pass
  - csharp: skipped (root-module-declarations: not-emitted)
  - go: skipped (root-module-declarations: not-emitted)
- **rust_empty_glob_export_pin** — a root glob resolving to zero public items fails closed with the empty-glob label
  - rust: pass
  - csharp: skipped (root-module-declarations: not-emitted)
  - go: skipped (root-module-declarations: not-emitted)
- **rust_unresolved_module_file_pin** — a module declaration with no source file is a warning-level unresolved module file, promoted by --strict
  - rust: pass
  - csharp: skipped (root-module-declarations: not-emitted)
  - go: skipped (root-module-declarations: not-emitted)

## verify.no_cycles
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **acyclic_tree_passes_no_cycles_constraint** — a no_cycles constraint over an acyclic dependency graph verifies clean
  - rust: pass
  - csharp: pass
  - go: pass
- **cyclic_tree_fails_no_cycles_constraint** — a no_cycles constraint over a cyclic graph fails and reports the cycle
  - rust: pass
  - csharp: pass
  - go: pass
- **warning_severity_cycle_routes_to_warning_bucket** — a cycle caught by a warning-severity no_cycles constraint passes verify and renders in the warning bucket, promoted by --strict
  - rust: pass
  - csharp: pass
  - go: pass
- **vacuous_cycle_constraint_surfaces_edgeless_group** — a no_cycles constraint over a module group with no internal edges reports vacuous and stays green
  - rust: pass
  - csharp: pass
  - go: pass
- **cycle_through_a_test_project_is_invisible_to_production_guard** — a c# cycle carried only through a test-tier project leaves the production graph a DAG: the seed omits it and verify stays green (rust cfg(test) / go *_test.go parity)
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)

## verify.forbid_external
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **forbidden_external_dependency_is_reported** — a module using a forbidden external package is reported as a violation
  - rust: pass
  - csharp: pass
  - go: pass
- **module_outside_forbid_list_passes** — a module not listed in the forbid constraint passes even when a sibling violates
  - rust: pass
  - csharp: pass
  - go: skipped (module-tier: go.work tier only (2+ members); declared grouping otherwise)
- **forbid_by_package_name_fires_on_misaligned_namespace** — a forbid rule naming the exact package id fires on a module whose only using maps to that package by longest shared prefix
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **forbid_by_package_name_fires_on_aligned_namespace** — a forbid rule naming an aligned package id fires on the module that imports its namespace
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **forbid_prefix_sharing_package_does_not_fire_on_longest_match** — a forbid rule naming a package that shares only a shorter prefix with the imported namespace does not fire (longest match attributes the real dependency)
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)
- **unused_reference_absent_from_module_external_but_in_manifests** — a referenced package no using surfaces is absent from module_external (no forbid fire) yet present in the unit manifests
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)

## verify.manifest_integrity
**Capability:** rust=implemented, csharp=implemented, go=not-implemented

**Behaviors:**

- **forbidden_dependency_in_manifest_is_reported** — a manifest declaring a forbidden dependency fails manifest_integrity
  - rust: pass
  - csharp: pass
  - go: skipped (verify.manifest_integrity not implemented)
- **manifest_matching_constraint_verifies_clean** — a manifest whose dependencies avoid the forbidden list verifies clean
  - rust: pass
  - csharp: pass
  - go: skipped (verify.manifest_integrity not implemented)

## update.seed
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **update_creates_spec_at_project_root** — update seeds an architecture.spec.toml at the project root
  - rust: pass
  - csharp: pass
  - go: pass
- **seed_verifies_clean_against_itself** — verify against the just-generated seed reports no violations
  - rust: pass
  - csharp: pass
  - go: pass
- **seed_omits_dotnet_test_projects_with_runner_evidence** — update omits a dotnet test project proven by runner-package evidence; the packable flag and the name play no part
  - rust: skipped (project shape not applicable)
  - csharp: pass
  - go: skipped (project shape not applicable)

## report.metrics
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **report_emits_metrics_for_clean_project** — report prints metric rows and a clean result for a spec that matches the tree
  - rust: pass
  - csharp: pass
  - go: pass
- **report_counts_units_and_components** — report counts one component per declared unit and every extracted unit
  - rust: pass
  - csharp: pass
  - go: pass
- **report_json_findings_match_text_rendering** — report --format json parses, carries the same finding set as the text report, and exits 1 on violations
  - rust: pass
  - csharp: pass
  - go: pass
- **report_json_clean_project_has_empty_findings** — a clean project reports empty findings and exit 0 in the json format
  - rust: pass
  - csharp: pass
  - go: pass

## inspect.file_level
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **renders_a_node_per_source_file** — inspect renders a node per source file and the resolved file-level edge
  - rust: pass
  - csharp: pass
  - go: pass
- **file_level_diagram_is_deterministic** — an unchanged tree renders a byte-identical file-level diagram across runs
  - rust: pass
  - csharp: pass
  - go: pass
- **production_imports_never_target_test_files** — no production file-level import targets a test-tier file; test files stay nodes and import sources
  - rust: pass
  - csharp: pass
  - go: pass

## inspect.structural_tree
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **tree_view_renders_units_and_module_edges** — inspect tree renders one subgraph per unit with its module nodes and edges
  - rust: pass
  - csharp: pass
  - go: pass

## inspect.structural_scanner
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **scanner_view_draws_unit_edges** — inspect scanner draws the cross-unit hard edges between unit subgraphs
  - rust: pass
  - csharp: pass
  - go: pass

## depgraph
**Capability:** rust=implemented, csharp=implemented, go=not-implemented

**Behaviors:**

- **modules_projects_top_level_graph** — depgraph modules collapses nested module paths to top-level nodes and may draw several (cross product of folded top-nodes)
  - rust: pass
  - csharp: pass
  - go: skipped (depgraph not implemented)
- **api_usage_view_emits_table_contract** — depgraph api-usage emits the fixed-column table or the explicit no-usage statement
  - rust: pass
  - csharp: pass
  - go: skipped (depgraph not implemented)
- **submodules_view_scopes_to_parent** — depgraph submodules expands one parent into its children (plus mod) and the edges between them
  - rust: pass
  - csharp: skipped (project shape not applicable)
  - go: skipped (depgraph not implemented)

## diagram
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **renders_a_node_per_declared_module_and_edge** — diagram renders a node per declared module and the declared dependency edge
  - rust: pass
  - csharp: pass
  - go: pass
- **marks_forbidden_edge_dashed** — diagram renders a forbidden edge dashed and keeps allowed edges solid
  - rust: pass
  - csharp: pass
  - go: pass

## init
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **init_creates_spec_declaring_language** — init writes an architecture.spec.toml that declares the detected language
  - rust: pass
  - csharp: pass
  - go: pass
- **init_refuses_to_overwrite_existing_spec** — init refuses to overwrite an existing architecture.spec.toml
  - rust: pass
  - csharp: pass
  - go: pass

## help.diagnostics
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **prints_catalog_with_facade_and_laundered_entries** — `help diagnostics` exits 0 and quotes the facade and laundered-edge catalog entries
  - rust: pass
  - csharp: pass
  - go: pass

## verify.root_facade
**Capability:** rust=implemented, csharp=not-implemented, go=not-implemented

**Behaviors:**

- **import_of_facade_root_reexport_is_violation** — an internal module importing an item through a publication-only root is a facade dependency
  - rust: pass
  - csharp: skipped (verify.root_facade not implemented)
  - go: skipped (verify.root_facade not implemented)
- **canonical_import_stays_clean_under_active_facade** — canonical imports never touch the facade root, so verify passes with the rule active
  - rust: pass
  - csharp: skipped (verify.root_facade not implemented)
  - go: skipped (verify.root_facade not implemented)

## verify.forbidden_laundering
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **forbidden_edge_routed_through_conduit_hop_is_laundered** — a ban routed via an undeclared conduit module is reported as a laundered forbidden edge
  - rust: pass
  - csharp: pass
  - go: pass
- **worktree_free_go_tree_announces_laundering_capability** — a single-module go tree (no go.work) announces consistently: a fired rule reports the laundered finding without any tier note, a clean run states no native module tier this run
  - rust: skipped (project shape not applicable)
  - csharp: skipped (project shape not applicable)
  - go: pass
- **laundering_traces_only_claimed_conduit_territory** — a ban routed through a conduit whose territory a boundary claims is reported as laundered; the same conduit claimed by no boundary leaves the pair graph — no laundered line, only unexpected component (go) or unowned endpoints (soft tier) surfacing
  - rust: pass
  - csharp: pass
  - go: pass

## verify.submodule_contracts
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **submodule_contract_leak_reported_under_submodule_path** — a contract.forbid stereotype surfacing under a submodule boundary leaks under the submodule's path
  - rust: pass
  - csharp: pass
  - go: pass
- **go_submodule_contract_stereotype_needs_full_path_glob** — a submodule contract over go packages fires for a stereotype globbing the full import path under a parent that claims its units, and reads inert for a bare-name stereotype that matches no go unit
  - rust: skipped (project shape not applicable)
  - csharp: skipped (project shape not applicable)
  - go: pass
- **forbidden_submodule_dependency_across_sibling_children** — an edge between sibling submodules banned by the parent constraint is reported with the submodule-dependency label
  - rust: pass
  - csharp: pass
  - go: skipped (module-tier: go.work tier only (2+ members); declared grouping otherwise)
- **forbidden_submodule_dependency_full_path_engages** — the same sibling-submodule ban expressed with full module-path from/forbid engages identically on every module-tier driver and is never reported vacuous
  - rust: pass
  - csharp: pass
  - go: skipped (module-tier: go.work tier only (2+ members); declared grouping otherwise)

## cli.artefact_freshness
**Capability:** rust=implemented, csharp=implemented, go=implemented

**Behaviors:**

- **scan_check_passes_on_fresh_artefact** — `scan --output` then `scan --output --check` exits 0 on the untouched file
  - rust: pass
  - csharp: pass
  - go: pass
- **scan_check_fails_on_stale_artefact** — a modified artefact makes `scan --check` exit non-zero with the out-of-date finding
  - rust: pass
  - csharp: pass
  - go: pass

