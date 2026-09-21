# `spec` — Output Contract

## Destination

- **`spec --schema`:** a JSON Schema (draft-07) written to **stdout**, exit code `0`.
- **`spec`:** the annotated reference TOML written to **stdout**, exit code `0`.
- Stderr is empty on success. Errors go to stderr (see `errors.md`).

## Schema output (`--schema`)

A JSON object (pretty-printed, newline-terminated) with:

| Key | Content |
|---|---|
| `$schema` | `http://json-schema.org/draft-07/schema#` |
| `title` | a human-readable title |
| `type` | `"object"` |
| `properties` | `project`, `module`, `constraint`, `stereotype` |

- `project`: object with `language` **required**, enumerated as `["rust", "csharp", "go"]`.
- `module`: array of objects — `name` **required**; `matches` accepts `units` and `modules` (arrays of strings); `contract` accepts the enforced `forbid`, while `expose` is reserved and rejected on presence (`"not": { "required": ["expose"] }` — an empty list is rejected too); `allowed` accepts `depend_on`/`forbidden`; `submodules` recurses to the same module shape.
- `constraint`: array of objects — `type` enumerated over all 7 types; per-type `required` arrays match the shared required-field table exactly (see `../../spec.md`); `severity` is optional and enumerated as `["error", "warning"]`; `modules` is an optional array of strings.
- `stereotype`: array of objects with a string `name`.

## Reference output

Annotated example TOML covering every section, preceded by the command's usage lines (the whole printed block is byte-identical to `archspec spec --help` — one shared constant):

```
# architecture.spec.toml — annotated reference
# Boundaries and relations, not tree shape. See docs/archspec/spec.md.

[project]
language = "rust"   # rust | csharp | go

[[module]]
name = "domain"                                   # required
matches = { units = ["core"], modules = ["Billing::domain"] }
contract = { forbid = ["entity"] }                 # expose is reserved: declaring it is a schema error
[module.allowed]
depend_on = ["domain", "ports"]
forbidden = ["infrastructure"]

[module.submodules]                                 # recurses: same shape as [[module]]
name = "domain::entities"
matches = { modules = ["Billing::domain::entities"] }

[[constraint]]
type = "no_cycles"                                # one of 7 types
modules = ["domain", "ports", "adapters"]      # optional
severity = "error"                                # error | warning (optional)
...
```

The exact text is not a contract; that all sections (`[project]`, `[[module]]`, `[[constraint]]`, `[[stereotype]]`) appear and the exit code is `0` is. `spec` and `spec --help` share one embedded constant, so they cannot drift.

## Determinism

The same invocation always produces **byte-identical output**. The schema serializes through `serde_json` with its default map ordering, so object keys are in a stable order across runs and machines. Output is safe to commit and safe to diff in CI.

## Example

All seven constraint types and their required arrays are emitted, e.g. `forbid_external_crates` requires `["from", "forbid"]`, `feature_boundary` requires `["feature", "gated_modules"]`, and `no_cycles`/`manifest_integrity` require nothing.