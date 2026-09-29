# rust-arch-test-kit

The `archspec` CLI: multi-language architecture test & diagram tool. It extracts
an architecture model from a project tree, compares it against
`architecture.spec.toml`, and renders diagrams, dependency views and diff
reports. The crate is the CLI binary — no longer an architecture-assertion
library (`src/lib.rs` re-exports only the shared parse/render primitives for
the binary itself).

Guides live in [docs/archspec/](docs/archspec/); the living design is
[docs/archspec-design.md](docs/archspec-design.md).

---

## Commands

| Command | Purpose |
|---|---|
| `init` | Scaffold a minimal base spec |
| `scan` | Extract the architecture model only (no spec needed) |
| `diagram` | Render model (extracted or declared) to an artefact |
| `verify` | Extract + compare vs spec, full diff, exit code |
| `update` | Snapshot current model as a seed spec |
| `report` | Text/markdown diff + metric output |
| `spec` | Print the spec JSON schema or annotated reference |
| `doctor` | Diagnose which language drivers/toolchains are present |
| `inspect` | Zero-config file-level import map (discovery) |
| `depgraph` | Current-state module/submodule/API-usage dependency views |
| `help` | Built-in manual: topics + per-command help |

Run `archspec help` for the topic index and `archspec <command> --help` for flags.

---

## Quick start

```bash
cargo build --release        # binary: target/release/archspec
cd /path/to/project
archspec init                # minimal base spec ([project] language + no_cycles guard)
# declare boundaries in architecture.spec.toml, then:
archspec verify --strict     # extract + compare vs spec, exit non-zero on any diff
archspec depgraph modules    # current-state module dependency graph
```

The spec declares boundaries and relations — nothing about tree shape:

```toml
[project]
language = "rust"

[[module]]
name = "domain"
matches = { modules = ["auth::domain"] }

[module.allowed]
depend_on = ["ports"]
forbidden = ["infrastructure"]

[[constraint]]
type = "no_cycles"
```

`matches` keys, `[module.allowed]` `depend_on`/`forbidden`, and the constraint
types are specified in [docs/archspec/spec.md](docs/archspec/spec.md) — not
duplicated here.

---

## Where to edit

- CLI: `src/archspec/` — `cli.rs`, `commands/`, `config.rs`, `depgraph.rs`,
  `diagram.rs`, `init/`, `inspect/`, `language.rs`, `model.rs`, `report/`,
  `scan/` (rust/csharp/go drivers), `spec.rs`, `update/`, `verify/`
- Shared engine primitives: `src/collector.rs` (Rust parsing), `src/render.rs`
  (rendering owner)
- Behavior tests: `tests/` (per command + `shared/` multi-language scenarios)
- Project policy: `<project>/architecture.spec.toml`

---

## Self-dogfood

This crate checks its own architecture: `architecture.spec.toml` declares the
`commands`, `engines` and `foundations` groups plus the library boundary
`rust-arch-test-kit` — and the `lib-collector` / `lib-render` prefix groups
that let the cycle checks see inside the library unit — and polices itself
with four of the seven constraint types: `no_cycles`, `public_api_allowlist`,
`forbid_external_crates` and `manifest_integrity`. Test-gated modules and
whatever only they import are excluded from the `depend_on` comparison — the
same rule the `depgraph --help` statement spells out for the graph views —
and this crate keeps even its engine tests pointed down-layer rather than
leaning on that exemption. `run-unit-tests.sh` runs `archspec verify --strict`
plus `--check` freshness gates on the committed artefacts
(`scan`/`diagram`/`inspect`/`report`), and `tests/spec_self_verify.rs` keeps
that strict self-verify reachable from a plain `cargo test`, without the git
hook or the runner script.

---

## License

`archspec` is licensed under the Apache License 2.0 — see
[LICENSE](LICENSE). Third-party dependency licenses are listed in
[CREDITS.md](CREDITS.md), generated from the lockfile by
`scripts/gen-credits.sh`.
