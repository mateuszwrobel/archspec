# `depgraph` — Input Contract

## Path

The `path` positional is a directory scanned exactly as `scan` scans it: language is detected from the tree, and the same driver builds the model that `scan`, `inspect tree`, and `verify` use. Defaults to the current working directory.

| Input | Result |
|---|---|
| directory with supported sources | model extracted, view rendered |
| omitted path | current directory used |
| missing path | error (see `errors.md`) |
| regular file | error |
| directory with no supported-language sources | error |

## `--parent` (submodules view)

`--parent <module>` names a **top-level** module to expand into its immediate children. The full module name (`unit::first-segment`, written with `::` or `.`) resolves first; the bare first segment (e.g. `Api` for `Shop.Api`) still resolves as a compatibility fallback. A bare segment that matches the top-level module of several units expands all of them: the union of their children is rendered, in deterministic order — prefer full names to address exactly one module. A name that resolves to no module is an error listing the module names the model knows. It has no meaning for the other views, so supplying it there is an error.

## Zero configuration

No spec file or rules are read. The views depend only on the extracted model (module paths, module edges, and the symbols carried on those edges), so `depgraph` runs on any tree `scan` can scan.
