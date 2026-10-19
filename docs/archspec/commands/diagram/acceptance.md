# `diagram` — Acceptance Tests (Phase 1)

CLI-level, behavioral only. Each scenario runs the real command against a fixture project and asserts on observable output (exit code, stdout/stderr, written files). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `diagram` on a fixture project renders a byte-stable diagram (Mermaid by default, PlantUML via `--format plantuml`) whose nodes are the model's components (spec mode) or units (scan mode), whose edges are the model's dependencies, which marks forbidden/violating edges dashed/red and clusters external crates as specified — the scan snapshot renders its unit tier by default and its module tier only under `--tiers modules`, additively — and each error condition produces a clear message and a non-zero exit.

## Happy path — declared spec

| # | Given | When | Then |
|---|---|---|---|
| 1 | project with `architecture.spec.toml` declaring modules `Billing`, `Shared` and edge `Billing -> Shared` | run `diagram` on project root | exit 0; diagram has a node per module and edge `Billing -> Shared` |
| 2 | project with spec declaring a forbidden edge `Billing -> Portal` | run `diagram` on project root | exit 0; the forbidden edge is present and visibly marked (dashed/red) |
| 3 | project with spec declaring nested sub-components | run `diagram` | exit 0; nested sub-components appear in the diagram |
| 4 | spec lives in a project directory other than the current directory | run `diagram ./crates/auth` | exit 0; diagram renders the spec from `./crates/auth` |

## Happy path — scan snapshot

| # | Given | When | Then |
|---|---|---|---|
| 5 | scan artefact with units `auth`, `core`, `portal` and deps `auth -> core`, `auth -> portal`; governing spec forbids `auth -> portal` | run `diagram --source scan <artefact>` | exit 0; node per unit; `auth -> portal` edge marked dashed/red |
| 6 | scan artefact with an external dependency (e.g. `serde`) | run `diagram --source scan <artefact>` | exit 0; external crate rendered in a separate cluster |
| 7 | scan artefact, no governing spec present | run `diagram --source scan <artefact>` | exit 0; no edges marked |

## Rendering and determinism

| # | Given | When | Then |
|---|---|---|---|
| 8 | project with a spec | run `diagram --format plantuml` | exit 0; PlantUML output contains nodes and the declared edges |
| 9 | project with a spec | run `diagram --output out.mmd` | exit 0; `out.mmd` contains the diagram; stdout is `wrote out.mmd` |
| 10 | same project, unchanged | run `diagram` twice | both runs byte-identical output |
| 11 | same scan artefact, unchanged | run `diagram --source scan <artefact>` twice | both runs byte-identical output |

## Errors

| # | Given | When | Then |
|---|---|---|---|
| 12 | path does not exist | run `diagram <missing>` | non-zero exit; message identifies the invalid path |
| 13 | path is a regular file | run `diagram <file>` | non-zero exit; message identifies the invalid path |
| 14 | project directory without `architecture.spec.toml` | run `diagram <dir>` | non-zero exit; message says no spec found |
| 15 | spec file with invalid TOML | run `diagram` | non-zero exit; message names the spec file; no partial diagram |
| 16 | scan artefact that does not exist | run `diagram --source scan <missing>` | non-zero exit; message identifies the missing artefact |
| 17 | scan artefact that is not a valid model snapshot | run `diagram --source scan <file>` | non-zero exit; message names the invalid artefact |
| 18 | scan artefact with an empty model | run `diagram --source scan <empty>` | non-zero exit; message says no model content |
| 19 | spec declaring no components | run `diagram` | non-zero exit; message says no model content |
| 20 | unsupported format value | run `diagram --format bmp` | non-zero exit; message names the unsupported value |
| 21 | unsupported source value | run `diagram --source gif` | non-zero exit; message names the unsupported source |
| 22 | `--source scan` without an artefact path | run `diagram --source scan` | non-zero exit; message says an artefact path is required |
| 23 | two positional paths given | run `diagram <a> <b>` | non-zero exit; message says at most one path accepted |

## Freshness (`--check`)

| # | Given | When | Then |
|---|---|---|---|
| 24 | `--output` destination already holds the generated diagram | run `diagram --output <f> --check` | exit 0; file untouched; stdout is `ok: <f> up to date` |
| 25 | `--output` destination differs from the generated diagram | run `diagram --output <f> --check` | non-zero exit; message says it differs and names the regenerate command; file untouched |
| 26 | no file at the `--output` destination | run `diagram --output <f> --check` | non-zero exit; message says it is missing; no file created |
| 27 | no `[output] diagram` configured and no `--output` | run `diagram --check` | non-zero exit; message says `--check` requires an output destination |

## Tier selection (`--source scan --tiers`)

| # | Given | When | Then |
|---|---|---|---|
| 31 | scan artefact carrying `module_edges`, flag absent | run `diagram --source scan <artefact>` | exit 0; bytes byte-identical to the pre-`--tiers` render; no `modules` cluster appears |
| 32 | same artefact | run `diagram --source scan <artefact> --tiers modules` | exit 0; a `modules` cluster is appended with one node per resolved endpoint and one arrow per distinct pair; every line outside the cluster keeps its exact bytes (`--tiers modules` equals `--tiers units,modules`) |
| 33 | governing spec whose claims own the module endpoints | run with `--tiers modules` | exit 0; owned nodes render under the owning claim name with the dual label (`api-billing["api-billing (Sample::Api::Billing)"]`); unowned endpoints keep path labels; a specless artefact renders path-named and marks nothing |
| 34 | module edge whose pair `verify` reports | run with `--tiers modules` | exit 0; the edge is dashed `violates spec`; an allowed pair renders solid; a declared rule with no extracted edge draws nothing |
| 35 | unit edge `verify` reports as `disallowed cross-component dependency`, flag absent | run `diagram --source scan <artefact>` | exit 0; the edge is dashed `violates spec` — the mark set equals verify's verdict classes without any flag |
| 36 | spec whose claim name equals a rendered unit name | run with `--tiers modules` | exit 0; the two tiers render two distinct nodes (ids cannot collapse); labels stay grep-able |
| 37 | `--check` with `--tiers modules`, destination holds the flagged render | run `diagram --source scan <artefact> --tiers modules --output <f> --check` | exit 0; the compared bytes are the flagged render's |

## File tier (`--tiers files` — spec source with a territory flag)

| # | Given | When | Then |
|---|---|---|---|
| 40 | spec whose claim territory holds a two-file cycle, flag set | run `diagram --source spec --tiers files --module <claim>` | exit 0; the claim subgraph holds a `files` cluster with one node per file and one arrow per used_by edge; the cycle's arrows carry the violation mark — verdict-for-verdict equal to the cycle findings `verify` reports for the same tree |
| 41 | same tree, `verify` exits 0 (the files edges are legal under the contract) | run `diagram --source spec --tiers files --module <claim>` and `verify` | both exit 0; the render carries no `violates spec` mark — exit-code parity across the surfaces |
| 42 | territory holds a file and an asset (`.css`); an asset and a file import the file | run `diagram --source spec --tiers files --module <claim>` | exit 0; the asset vertex renders with its incoming edge only and never carries the mark — cycles close between source files only |
| 43 | same tree, warning-severity cycle | run `diagram --source spec --tiers files --module <claim>` while `verify` exits 0 | exit 0; the cycle's arrows render solid — the mark set is the error bucket, the warning bucket stays unmarked |
| 44 | scan artefact | run `diagram --source scan <artefact> --tiers files` | exit `64`; message says the tier requires the spec source (the scan tier is the module tier — the scan leg extracts no file-level edges) |
| 45 | `--tiers files` without `--module` or `--modules-dir` | run `diagram --source spec --tiers files` | exit `64`; message names both territory flags and the size law (one claim's territory per render, the whole tree never) |
| 46 | rust project with a multi-file crate | run `diagram --source spec --tiers files --module <claim>` | exit 0; the `files` cluster carries the serialized `file_edges` rows when the claim's patterns address the files, the module-view bytes stay inside it, and no `note:` line prints — the rust driver emits the fact (a tree whose unit shows no file adjacency renders the module view unchanged, still with no note: an empty render states absence by omission of rows, not by fabrication) |
| 47 | `--modules-dir` batch | run `diagram --source spec --tiers files --modules-dir <dir>` | exit 0; each diagram whose claim holds a files cycle carries the cluster and marks; other diagrams keep their exact default bytes |
| 48 | `--check` with `--tiers files`, destination holds the cluster render | run `diagram --source spec --tiers files --module <claim> --output <f> --check` | exit 0; the compared bytes are the cluster render's |

## Errors — tier selection

| # | Given | When | Then |
|---|---|---|---|
| 38 | `--tiers` value outside `units`/`modules`/`files`, a duplicate entry, or an empty list | run `diagram --source scan <artefact> --tiers <bad>` | exit `64`; message names the offending value and the supported tiers |
| 39 | `--tiers units` or `--tiers modules` with `--source spec` | run `diagram --tiers modules` | exit `64`; message says the flag requires `--source scan` |
| 44/45 | the `--tiers files` refusals | see the file-tier table | exit `64`; messages as stated there |

## Role marks (`--source scan`)

| # | Given | When | Then |
|---|---|---|---|
| 28 | scan artefact whose `roles` map holds a key equal to a rendered node's identity (modulo separator equivalence — `.` and `::` spellings of one path match) | run `diagram --source scan <artefact>` | exit 0; that node's line carries the role's marker in both formats (mermaid `["Name [role]"]` suffix, PlantUML `<<role>>` stereotype on the bare identifier); node ids and every edge line unchanged |
| 29 | node the `roles` map does not address, including every node of the external cluster | run `diagram --source scan <artefact>` | exit 0; those lines byte-identical to the unmarked render; no line grows marker syntax |
| 30 | scan artefact with an empty or absent `roles` map; spec mode whose declared components collide with role-carrying model paths | run `diagram --source scan <artefact>` / run `diagram` | exit 0; output byte-identical to the pre-roles bytes; spec mode reads no roles map |

Pins (numbering is contiguous through row 30; these three rows are roles-views
US 01's append): row 28 — `tests/diagram.rs::diagram_scan_mermaid_marks_the_role_addressed_node`,
`…::diagram_scan_plantuml_marks_the_role_addressed_node` (the `.`-spelled key
arm of the separator equivalence), `…::diagram_scan_marks_csharp_composition_root_end_to_end`
and `…::diagram_scan_role_marks_are_order_independent_and_deterministic`
(determinism arm); row 29 —
`…::diagram_scan_external_cluster_carries_no_marker_syntax`; row 30 —
`…::diagram_scan_empty_roles_map_is_byte_identical_to_pre_plan_bytes` and
`…::diagram_spec_mode_is_byte_identical_to_pre_plan_bytes`. The bidirectional
cross-driver leg (marks exactly where the map addresses, on every driver) is
the shared scenario `diagram / scan_mode_marks_role_carriers`. Tier-plan pins
(workplan_archspec_scan_diagram_module_tier, rows 31–39): row 31 —
`…::default_scan_render_carries_no_module_bytes`; row 32 —
`…::tiers_modules_adds_path_named_cluster_specless`; row 33 —
`…::tiers_modules_names_owned_nodes_by_claim_with_dual_label`; row 34 —
`…::tiers_modules_pair_law_matches_verify_real_tree` and
`…::tiers_module_marks_parity_across_formats`; row 35 —
`…::unit_tier_ceiling_gap_edge_marks_without_flag`; row 36 —
`…::tiers_modules_claim_name_equal_unit_name_does_not_collapse`; row 37 —
`…::tiers_check_gates_flagged_bytes`; rows 38–39 —
`…::tiers_invalid_values_exit_64`. File-tier pins
(workplan_archspec_ts_driver, phase C — rows 40–48, appended with a
numbering gap noted): row 40 —
`tests/diagram_files_tier.rs::files_tier_renders_territory_files_and_file_edges`,
row 41 — `…::files_tier_marks_agree_verdict_for_verdict_with_verify`
(exit-code-parity arms included), row 42 —
`…::asset_vertex_renders_incoming_only_and_never_cycle_marked`, row 43 —
`…::warning_severity_cycle_stays_solid_in_the_files_render` and the
plantuml grammar arm `…::files_tier_renders_plantuml_with_the_violation_grammar`,
row 44 — `…::files_tier_with_scan_source_is_refused`, row 45 —
`…::files_tier_without_territory_scope_is_refused`, row 46 —
`…::files_tier_on_a_rust_spec_states_vacuity_and_never_fabricates`, row 47 —
`…::modules_dir_batch_renders_clusters_and_check_gates_them`
(`--check` arm is row 48). The unknown-tier vocabulary arm is
`…::unknown_tier_value_names_the_extended_vocabulary` and the byte-identity
arm is `…::module_render_without_files_value_is_byte_unchanged`.
