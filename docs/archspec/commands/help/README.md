# archspec `help` — Design

Built-in manual for the model-driven workflow: `help` lists the seven topics (`commands`, `glob`, `spec`, `constraints`, `languages`, `workflow`, `diagnostics`), aliases `help <command>` to that command's `--help`, and prints the `architecture.spec.toml` reference, the glob rules, the constraint types, the language-tier matrix, the recommended `scan → report → diagram → verify` recipe, and the diagnostics catalog that teaches an agent to read every `verify`/`report` finding. Part of the archspec design (see `../../../archspec-design.md`).

The command never reads a project tree. It is a pure output command: all topic content is embedded in the binary as constants, and the per-command blocks reuse each command's own `--help` text so nothing drifts.

## Scope

- Topic index — a usage line, all 7 topics, and every command (the same index `archspec help`, `archspec help topics`, and `archspec help --help` print; bare `archspec` / `archspec --help` print the tool help — a different layout that carries the same command list).
- `commands` — one block per command: name, purpose, and its full `--help` text (flags included), straight from the command registry.
- `glob` — the glob matching rules for unit names and dotted module paths, including which keys are globs and which are exact declared-name matches.
- `spec` — the `architecture.spec.toml` annotated reference, byte-identical to `archspec spec` (one shared constant).
- `constraints` — all seven constraint types with their keys and the severity contract (`error` default, `warning`, `--strict` promotion).
- `languages` — the model-tier matrix: which of the nine model tiers each language scanner populates, embedded as `LANGUAGE_TIERS` and guarded by a consistency test against a real scan.
- `workflow` — the `scan → report → diagram → verify` recipe, `--strict` as the CI gate, and the supporting `doctor`/`init`/`update` commands.
- `diagnostics` — the catalog of every `verify`/`report` finding category: verbatim message pattern, meaning, origin (code/spec/intent), severity + `--strict` behaviour, follow-up command, decision rule, the exit-code contract, and the required agent report format.
- `help <command>` — that command's `--help`, byte-identical (looked up from the registry).

## Quick usage

```bash
archspec help            # topic index
archspec help glob       # glob matching rules
archspec help spec       # the spec reference (same as `archspec spec`)
archspec help verify     # same as `archspec verify --help`
archspec help --help     # the topic index (help's own help)
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, topics, usage examples |
| `input.md` | Input contract: what the command accepts and rejects |
| `output.md` | Output contract: topic output, byte-identity, determinism |
| `errors.md` | Failure contract: unknown topics, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests (G1–G17) |