# archspec `verify` — Design

Extract the architecture model from source, compare it structurally to the declared spec, report the full diff, exit with a verdict. Part of the archspec design (see `../../../archspec-design.md`).

The tool reads `architecture.spec.toml` (see `../../spec.md`), extracts the model from the source tree, and compares the two **structurally** — set-based, order-insensitive. Its purpose is **gating**: does the code still satisfy the architecture it declared? It reports **everything** that differs, not the first hit: a pass prints a short confirmation and exits `0`; a fail prints the full diff report and exits non-zero.

## Scope

- Checks (design §9): component presence, dependency edges (allowed/forbidden), exposed-surface stereotypes, cycle constraints with severity, and declared internal sub-structure.
- The public-API surface check (`public_api_allowlist`, over named exports and root globs) and the structural root-facade rule (an internal module importing through its publication-only crate root) ship on the same machinery as the edge and cycle checks.
- Every reported difference is a distinct check result; all differences are reported in a single pass.
- `--strict` promotes all warnings (e.g. `no_cycles` severity=`warning`) to errors for CI gates.
- Remaining phase-2 checks (API-symbol contract leaks, composition-root placement) extend the same machinery.
- Per-language capability and skip reasons for every check above: [feature-matrix.md](../../../../feature-matrix.md).

## Quick usage

```bash
# from a project root — reads architecture.spec.toml in the current directory
archspec verify

# explicit path
archspec verify ./crates/auth

# strict gate for CI
archspec verify --strict

# strict gate, explicit path
archspec verify ./crates/auth --strict
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, flags, defaults, usage examples |
| `input.md` | Input contract: path semantics, spec discovery, accepted and rejected inputs |
| `output.md` | Output contract: pass/fail output, diff report structure, determinism |
| `errors.md` | Failure contract: operational errors, rule violations, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |
