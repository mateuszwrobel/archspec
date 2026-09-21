# `init` — Output Contract

## Destination

- Two artefacts are written into the target project directory (the positional `path`, or the current working directory):
  - `architecture.spec.toml` — the minimal base spec.
  - `archspec.toml` — the output config with standard `[output]` defaults (see `../../config.md`).
- There is **no `--output` flag**: the files' locations are fixed by the project directory and their names are fixed by the spec/config contracts.
- **stdout** carries a short summary; **exit code** `0`.
- Both target files are existence-checked before the first write, so every validation error writes nothing and modifies nothing; only a config write that fails after the spec was created leaves that newly created spec on disk (see `errors.md`).

## Summary on stdout

On success `init` prints a short summary naming the created files, the detected language, and the next step. Content is deterministic for a given project.

```
created architecture.spec.toml
created archspec.toml
language: rust
next step: run archspec update to seed boundaries from the tree, then edit, then verify
```

## Spec structure

The generated `architecture.spec.toml` is a **minimal base spec**: a `[project]` section setting the detected language, plus one global cycle guard — a `[[constraint]]` of `type = "no_cycles"` with **no** `modules` list. An absent `modules` list already means "all declared modules", so the guard activates as soon as the spec declares modules: **new projects start cycle-clean**. It declares no stereotypes, modules, or other constraints — the user fills in their actual boundaries.

```toml
[project]
language = "rust"            # rust | csharp | go

[[constraint]]
type = "no_cycles"           # global: all declared modules
```

There is **no profile selection and no profile content**. `init` gives a skeleton to build from; `update` (see `../../commands/update/`) is the command that seeds boundaries from the real tree. Until modules are declared the scaffold guard contributes no violations of its own — its check is vacuous and `verify` reports it as a vacuous warning (exit 0). It does not follow that the run is violation-free: units the scaffold leaves uncovered can still fail as unexpected components. The full shape of a spec is defined in `../../spec.md`.

## Output config structure

The generated `archspec.toml` carries the standard `[output]` defaults so `inspect`, `diagram`, `report`, and `scan` write their artefacts under `docs/archspec/` without passing `--output`:

```toml
[output]
inspect = "docs/archspec/inspect.mmd"
diagram = "docs/archspec/diagram.mmd"
report  = "docs/archspec/report.md"
scan    = "docs/archspec/scan.json"
```

The config is opt-in tooling config (`../../config.md`): it does not affect the spec, and every command still falls back to stdout when the destination is not set.

## Determinism

The same project always produces **byte-identical** spec and config content. Nothing in the generated files reflects timestamps, host names, or traversal order. Output is safe to commit to version control.

## Example

Given a Rust project (`Cargo.toml` present):

```bash
$ archspec init ./rust-workspace
created architecture.spec.toml
created archspec.toml
language: rust
next step: run archspec update to seed boundaries from the tree, then edit, then verify
```

The file `./rust-workspace/architecture.spec.toml` holds:

```toml
[project]
language = "rust"

[[constraint]]
type = "no_cycles"
```

And `./rust-workspace/archspec.toml` holds the standard `[output]` defaults shown above.
