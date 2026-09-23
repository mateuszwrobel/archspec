# `diagram` — Acceptance Tests (Phase 1)

CLI-level, behavioral only. Each scenario runs the real command against a fixture project and asserts on observable output (exit code, stdout/stderr, written files). Test tooling is owned by archspec itself — no shared harness.

## Done when

Running `diagram` on a fixture project renders a byte-stable diagram (Mermaid by default, PlantUML via `--format plantuml`) whose nodes are the model's components (spec mode) or units (scan mode), whose edges are the model's dependencies, which marks forbidden/violating edges dashed/red and clusters external crates as specified — and each error condition produces a clear message and a non-zero exit.

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
the shared scenario `diagram / scan_mode_marks_role_carriers`.
