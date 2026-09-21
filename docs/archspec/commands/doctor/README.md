# archspec `doctor` — Design (Phase 1)

Environment diagnostics for language drivers and toolchains. Part of the archspec design (see `../../../archspec-design.md`).

The command inspects the environment it runs in and reports, per supported language driver, whether that language's toolchain is present on `PATH`, along with version/status detail and an overall summary — a driver/toolchain **diagnostic report**. Its purpose is **readiness**: to make toolchain presence explicit and debuggable. It reports and nothing else — it never installs, modifies, or configures anything.

The PATH probe is informational: every language driver ships inside the archspec binary, so `scan`, `verify`, `update`, and `report` extract code regardless of what `PATH` holds (their driver-unavailable error is reachable only through the `ARCHSPEC_DISABLE_DRIVERS` test seam). `doctor` surfaces the user's own toolchains — Rust users have cargo, C# users have the .NET SDK, Go users have the Go toolchain — as an explicit report instead of an implicit assumption.

## Phase 1 scope

- Three supported languages: Rust, C#/.NET, Go — matching the phase-1 drivers (see `../../../archspec-design.md` §7).
- Zero configuration: no spec file, no flags, no arguments.
- Pure diagnosis: a text report on stdout. No installation or fix actions.
- Deterministic for the same environment; diagnostic text only, no diagrams.

## Quick usage

```bash
# check driver/toolchain availability for all supported languages
archspec doctor
```

## Design documents

| File | Contract |
|---|---|
| `cli.md` | Command surface: invocation, flags, defaults, usage examples |
| `input.md` | Input contract: no arguments, no flags, environment diagnosis |
| `output.md` | Output contract: destination, per-driver report structure, determinism |
| `errors.md` | Failure contract: error conditions, messages, exit behavior |
| `flows.md` | User flow and application flow |
| `acceptance.md` | Behavioral e2e acceptance tests |
