# `scan` — Input Contract

## Accepted input

Exactly one optional positional argument: a directory.

- If omitted, the **current working directory** is scanned.
- The directory must exist and must let one supported language be detected from its manifests or sources, discovered recursively.
- The scan covers the whole directory tree beneath the given path.

```bash
# scans the current directory
archspec scan

# scans the given directory tree
archspec scan ./crates/auth

# scans a Go module
archspec scan ./services/checkout
```

## Language detection

`scan` determines the project language from the tree's manifests and sources, and uses the matching language driver (Rust, C#, Go). Representative detection signals:

| Language | Typical signal in the tree |
|---|---|
| Rust | `Cargo.toml` workspace/member manifests, or `.rs` sources |
| C# / .NET | `.csproj` / `.sln` project files, or `.cs` sources |
| Go | `go.mod` module file, or `.go` sources |

A tree must present exactly one supported language. A tree with no detectable supported language is rejected; mixed-language trees are not a phase-1 target (see Notes).

For a Go tree a root `go.work` is also a read input (not a detection signal): its `use` entries — a pure source-level parse, no Go toolchain required — name the member modules whose packages form the model's module tier; see `../../spec.md` § Go modularity.

## Excluded directories

Each driver skips build, vendored, and tooling-output directories so they never contribute units, modules, or edges. The set is language-specific:

| Language | Directories skipped (recursively) |
|---|---|
| Rust | `target`, `vendor`, and any directory starting with `.` |
| C# | `bin`, `obj`, `node_modules`, and any directory starting with `.` |
| Go | `vendor`, `testdata`, and any directory starting with `.` |

So a Rust build artifact under `target/debug/build/<crate>/out/`, a vendored crate under `vendor/`, or a `.cargo/` cache directory is invisible to `scan`. `inspect` uses the same per-language rules through one shared predicate per language, so both commands see the same source tree.

## Excluded files

The Go driver additionally skips `*_test.go` files (both the `package foo` and `package foo_test` forms). Like a Rust `#[cfg(test)]` module, a Go test file compiles only for the test build, so its imports feed no production unit, edge, or external tier; the exclusion is a scan-level fact with no entry in the serialized model (mirroring the rust driver's `#[serde(skip)]`ed test-gating marker).

The C# driver skips whole **projects** the same way: a project whose csproj carries test-tier evidence — a resolved package set (csproj `PackageReference` items plus the nearest central props) naming a test runner (an `xunit`- or `nunit`-family id, `Microsoft.NET.Test.Sdk`, a `Microsoft.Testing.Platform`- or `MSTest`-family id) or a root `Sdk` attribute from the `MSTest.Sdk` family (the MSTest 3 project shape, versioned spellings included, carrying no runner package at all) — is C#'s test unit, so it enters no production unit, edge, soft tier, `module_external` attribution, or unit manifest. The evidence is dependency- and SDK-based, never the name: a runner-free project named `LegacyTests` stays production, a project named `Helpers` referencing xunit is test tier, and an assertion library with no runner (`FluentAssertions`) marks nothing. The family match is a prefix on purpose — satellites such as `xunit.analyzers` or `NUnit.Analyzers` appear only in test projects and count as evidence by design. MSBuild `Condition` attributes on the references the driver reads — on the item, its `ItemGroup`, or inside a `Choose`/`When`/`Otherwise` subtree — are decided by the shipped condition evaluator; the evaluated subset, the verdicts and their consequences, and the named non-goals are stated once by the `test marker under unevaluated condition` entry of `archspec help diagnostics`. The suppression is never silent for a test marker: a suppressed `PackageReference` whose `Include` names a test package prints one `test marker under unevaluated condition: <file>:<line>` line on the report surface — report-only, exit 0, never the machine body. Markers declared only in `Directory.Build.targets` or props brought in by `<Import>` are not walked and stay unmodeled — the named non-goal stands — and every such file a run encounters states itself on the report surface through the `condition face not walked` entry of `archspec help diagnostics`. The exclusion is never silent: `scan` states every dropped project on the report surface with one `test-gated unit: <path> — <marker>` line (the project directory, the matched marker), and the `verify` and `report` surfaces add the census note `test-gated units: <n>` — report-only lines that never join the machine body. Dropped test projects also keep their namespace prefixes dead: a production `using App.Tests.Specs;` under a dropped prefix resolves to nothing — no owner, no edge, no module. As with Go, the exclusion happens at the scan and nothing test-tier joins the serialized model.

## Rejected input

| Input | Why rejected |
|---|---|
| path that does not exist | nothing to scan |
| path that is a regular file | only directories are scannable |
| directory with no supported-language sources or manifests (recursively) | no language to drive extraction |
| more than one positional argument | ambiguous target |

Each rejection produces a clear message naming the cause and a non-zero exit (see `errors.md`).

## Examples

```bash
# rejected: no such directory
archspec scan /no/such/dir

# rejected: it is a file, not a directory
archspec scan Cargo.toml

# rejected: no supported-language sources
archspec scan ./docs

# rejected: two paths given
archspec scan ./src ./tests
```

## Notes

- No spec file (`architecture.spec.toml`, see `../../spec.md`) is read or required. `scan` is the extraction-only stage of the pipeline.
- No verification and no diagram: the model is emitted as-is.
- Mixed-language trees (e.g. a Rust crate nested in a Go module) are not a phase-1 target; `scan` targets a single project tree per run.
