# `depgraph` — Command Surface (CLI Contract)

## Invocation

```
archspec depgraph <view> [path] [--format <fmt>] [--parent <module>] [--output <path>] [--check]
```

The first positional selects the view and is required. `path` is the second positional and is optional.

| Part | Meaning | Default |
|---|---|---|
| `view` | `modules`, `api-usage`, `submodules`, or `ledger` | required |
| `path` | directory to scan | current working directory |
| `--format <fmt>` | output format | `mermaid` for graphs, `markdown` for `api-usage` and `ledger` |
| `--parent <module>` | parent top-level module to expand | required for `submodules` |
| `--output <path>` | write the body to that file; `-` prints it on stdout and creates no file | stdout |
| `--check` | compare the body to its destination without writing | off |
| `--aggregate <units\|modules>` | force the projection level in the modules view, bypassing the automatic rule | automatic rule |

## Views

- `modules` — top-level module dependency graph; when that projection folds the whole tier onto a single node, the view renders one level deeper (the paths below the fold become the nodes). `--aggregate units` renders the model's units and their edges instead; `--aggregate modules` pins the top level with no descent. An automatic render above 200 nodes adds one stderr note naming the degraded projection — stdout and exit are unchanged. `--format mermaid` (default) or `plantuml`.
- `api-usage` — Markdown table of used APIs grouped by target module. `markdown` only.
- `submodules` — graph of one parent module's immediate children. `--parent <module>` required; `--format mermaid` (default) or `plantuml`.
- `ledger` — the inversion-debt ledger: the two named sections over the model's own facts — the pairs the code imports while the importer's manifest omits the declaration (with evidence locs), and the pairs the manifest declares while the model holds no code evidence. A clean model states the ledger is clean by name; a model without a declaration tier states it cannot answer. `markdown` only.

## Accepted flag values

- `--format`: `mermaid` and `plantuml` for the graph views; `markdown` for `api-usage` and `ledger`. Any other value is an error (see `errors.md`).
- `--parent`: a top-level module name — full (`unit::module`, either separator) or bare first segment (compatibility fallback) — that must resolve in the model (submodules view only). A bare segment matching several top-level modules renders the union of their children (deterministic); prefer full names.
- `--output`: any writable file path.
- `--check`: freshness switch; compares the view body to the `--output` destination without writing. `depgraph` has no configured `[output]` default, so `--check` requires `--output` (see `errors.md`).

`--parent` is only valid with `submodules`; passing it with `modules` or `api-usage` is an error.

## Usage examples

### Top-level module graph

```bash
archspec depgraph modules ./crates/auth
```

Mermaid `graph TD` of the crate's top-level modules and their import edges, to stdout.

### Module graph as PlantUML to a file

```bash
archspec depgraph modules --format plantuml --output docs/modules.puml
```

### API-usage table

```bash
archspec depgraph api-usage ./crates/auth --output docs/api-usage.md
```

Markdown table to `docs/api-usage.md`; stdout stays empty.

### Inversion-debt ledger

```bash
archspec depgraph ledger ./packages
```

The two named sections — `used but undeclared` and `declared but unused` — to stdout; a clean model prints the named clean statement instead.

### One parent's submodules

```bash
archspec depgraph submodules --parent orchestration
```

Mermaid graph of `orchestration`'s child submodules (plus its own `mod` node) to stdout.

### Freshness check (CI gate)

```bash
archspec depgraph modules --output docs/modules.mmd --check
```

Render the view body in memory and compare it byte-for-byte with `docs/modules.mmd`. Nothing is written; a fresh destination exits `0`, a stale or missing one exits non-zero with the regeneration command. `depgraph` has no `[output]` default, so `--check` needs `--output`.
