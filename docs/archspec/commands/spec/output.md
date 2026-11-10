# `spec` — Output Contract

## Destination

- **`spec --schema`:** a JSON Schema (draft-07) written to **stdout**, exit code `0`. Every table shape carries `additionalProperties: false`; the accepted keys come from the one shared key table the runtime unknown-key validator reads, so a key the model does not define is refused by the schema exactly as the loader refuses it — naming the key and where it sits. The schema matches the runtime validator (one shared key table, welded by a test).
- **`spec`:** the annotated reference TOML written to **stdout**, exit code `0`. The reference lists every accepted `[project]` key, including `excluded_sources` (the declared source exclusion: `path` + `reason` mandatory per entry, optional `mode` dial `abort` | `warn`, every entry printed in every run's report).
- Stderr is empty on success. Errors go to stderr (see `errors.md`).

## Schema output (`--schema`)

A JSON object (pretty-printed, newline-terminated) with:

| Key | Content |
|---|---|
| `$schema` | `http://json-schema.org/draft-07/schema#` |
| `title` | a human-readable title |
| `type` | `"object"` |
| `additionalProperties` | `false` |
| `properties` | `project`, `module`, `constraint`, `stereotype`, `exempt` |

- `project`: object with `language` **required**, enumerated by the ONE driver-name table the schema renders (the enum is computed, never a hand-maintained copy); optional `framework`, enumerated by the driver-side framework registry the schema computes; optional `excluded_sources`, an array of `{ path, reason, mode }` tables — `path` and `reason` **required** and non-empty, `mode` optional and enumerated as `["abort", "warn"]` (the default is `abort`), no other key accepted (the declared source exclusion, D14).
- `module`: array of objects — `name` **required**; `matches` accepts `names`, `paths`, `units` and `modules` (arrays of strings; `names`/`paths` are the stereotype vocabulary, parsed on a module claim but not resolved there); `contract` accepts the enforced `forbid` and the enforced `expose` (the expose chain law); `allowed` accepts `depend_on`/`forbidden`; `submodules` recurses to the same module shape — with `matches.units` declarable only on top-level modules: a submodule declares the module tier only, so `matches.units` on a submodule is rejected at parse. An entry key outside these is an unknown-key refusal (see `errors.md`).
- `constraint`: array of objects — `type` enumerated over all 7 types; per-type `required` arrays match the shared required-field table exactly (see `../../spec.md`); the entry shape lists every key any type accepts (`severity`, `scope`, `modules`, `kinds`, `allowed`, `from`, `forbid`, `require_publish`, `forbidden_dependencies`, `required_features`, `feature`, `gated_modules`, `allowed_from`, `parent`); the shape is fail-closed, so a key outside it is refused at load naming the key and its location; `severity` is optional and enumerated as `["error", "warning"]`.
- `stereotype`: array of objects with a string `name` and an optional `match` table.
- `exempt`: array of objects — the cycle ratchet ledger: `cycle` and `tier` required (`tier` is `file`, `module` or `both`); optional `count` (positive integer), `until` (`YYYY-MM-DD`) and `reason`. Entry shape is fail-closed like every other table; the loader additionally refuses a non-positive `count`, an invalid `until` date and two entries naming one (tier, cycle).

## Reference output

Annotated example TOML covering every section, preceded by the command's usage lines (the whole printed block is byte-identical to `archspec spec --help` — one shared constant):

```
# architecture.spec.toml — annotated reference
# Boundaries and relations, not tree shape. See docs/archspec/spec.md.

[project]
language = "rust"   # the computed enum — rust | csharp | go | ts | python
# framework = "sveltekit"   # optional; registry computed by the schema (ts: sveltekit)

[[module]]
name = "domain"                                   # required
matches = { units = ["core"], modules = ["Billing::domain"] }
contract = { forbid = ["entity"] }                 # forbid and expose are both enforced (the expose chain law)
[module.allowed]
depend_on = ["domain", "ports"]
forbidden = ["infrastructure"]

[module.submodules]                                 # recurses: same shape as [[module]]
name = "domain::entities"
matches = { modules = ["Billing::domain::entities"] }
# a submodule with [module.allowed] owns its territory's dependency checks
# (most-specific claim wins at any depth); without it the nearest declaring
# ancestor's policy applies; matches.units on a submodule is a parse error

[[constraint]]
type = "no_cycles"                                # one of 7 types
modules = ["domain", "ports", "adapters"]      # optional
severity = "error"                                # error | warning (optional)
...
```

The exact text is not a contract; that all sections (`[project]`, `[[module]]`, `[[constraint]]`, `[[stereotype]]`, `[[exempt]]`) appear and the exit code is `0` is. `spec` and `spec --help` share one embedded constant, so they cannot drift.

## Determinism

The same invocation always produces **byte-identical output**. The schema serializes through `serde_json` with its default map ordering, so object keys are in a stable order across runs and machines. Output is safe to commit and safe to diff in CI.

## Example

All seven constraint types and their required arrays are emitted, e.g. `forbid_external_crates` requires `["from", "forbid"]`, `feature_boundary` requires `["feature", "gated_modules"]`, and `no_cycles`/`manifest_integrity` require nothing.