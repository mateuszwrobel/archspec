# `help` — Output Contract

## Destination

- All topics print to **stdout**, exit code `0`. Stderr is empty on success.
  Errors go to stderr (see `errors.md`).

## Topic index (bare `help`, `help topics`, `help --help`)

- A usage line: `usage: archspec help [topic]`.
- A one-line description of the manual.
- The 7 topics, one line each with a one-line description.
- Every command, one line each (name, padded, then its purpose).
- A trailing hint pointing at `archspec help <topic>`.

The three invocations print the **same byte sequence** (one shared constant).

## `help commands`

One block per command (registry order):

```
scan  extract the architecture model only (no spec needed)
  usage: archspec scan [path] [--output <path>]
  ...
```

The block is the command's name + summary followed by its full `--help` text,
so the printed flags are the same single source the commands themselves print.
Every command in the tool appears.

## `help glob`

A prose block stating:

- `*` / `**` match any run of characters, **including separators** `.`, `/`,
  `::`, `-`.
- Matching is case-sensitive and literal; `*` is the only special character
  (no regex, no character classes, no anchors).
- Unit globs match full unit names.
- Module globs match the full dotted path, the bare last segment, or the path
  with its unit prefix stripped.
- `matches.modules` matches the named module **and its descendants** (subtree).
- `from` / `forbid` / `gated_modules` / `allowed_from` / `parent` share the
  module-path glob semantics.
- `allowed.depend_on` / `allowed.forbidden` targets and `no_cycles.modules`
  are **exact declared-module-name** matches — not globs.

## `help spec`

Byte-identical to `archspec spec` (and `archspec spec --help`): the command's
usage block followed by the annotated `architecture.spec.toml` reference TOML
covering `[project]`, `[[module]]`, `[module.allowed]`, `[[constraint]]`, and
`[[stereotype]]`. One shared constant
(`commands::spec::HELP`), so it cannot drift.

## `help constraints`

One block per constraint type naming its keys, plus the severity contract:
default `error`, `warning` tolerated without `--strict`, and `--strict`
promotion as the CI gate.

| Type | Keys |
|---|---|
| `no_cycles` | `modules`, `severity` |
| `public_api_allowlist` | `allowed` |
| `forbid_external_crates` | `from`, `forbid` |
| `manifest_integrity` | `require_publish`, `forbidden_dependencies`, `required_features` |
| `feature_boundary` | `feature`, `gated_modules`, `allowed_from` |
| `forbid_submodule_dependency` | `parent`, `from`, `forbid` |
| `external_free` | `from` |

## `help languages`

A header naming the nine model tiers, then one block per language (rust,
csharp, go) listing which tiers that scanner populates. The matrix is
`LANGUAGE_TIERS` and is guarded by a unit test against real scan output, so
the manual cannot drift from what the scanners actually fill. go has no
toolchain requirement at scan time (in-binary parser), so the matrix reflects
the parser's behavior.

## `help workflow`

The recipe `scan → report → diagram → verify` with each command's purpose and
when to use it; `--strict` presented as the CI gate; and the supporting
`doctor`, `init`, `update` commands mentioned.

## `help diagnostics`

The catalog of every `verify`/`report` finding category. It opens with the
origin legend (`code` → code-fix, `spec` → spec-fix, `intent` →
architecture-rework) and the severity + exit-code contract (`error` → exit 1,
`warning` → exit 0 unless `--strict` promotes it), then the required REPORT
FORMAT (finding / evidence / resolution / follow-up). Each category block names
the verbatim message pattern emitted by `verify/compare.rs` (`render_report`,
its `format!` bodies, and the `report::diff_items` labels), its meaning, origin,
severity + `--strict` behaviour, the first follow-up command (`scan` /
`report` / `depgraph` / `inspect` keys), and the decision rule. Split into
error-level then warning-level groups; `root_public_exports` / `root_glob_exports`
are noted as model tiers that surface through the public-API categories rather
than standalone lines. Adding a finding category to `verify/compare.rs` requires
a new block here **and** a new entry in the `DIAGNOSTIC_CATEGORIES` list in
`tests/help.rs` (the completeness guard).

## `help <command>`

Byte-identical to `archspec <command> --help`, looked up from the command
registry.

## Determinism

The same invocation always produces **byte-identical output**: every topic is
an embedded constant, registry blocks iterate a fixed ordered array, and the
language matrix renders from a fixed constant. Output is safe to commit and
safe to diff in CI. The topic blocks are newline-terminated like all help
output.

## Example

```text
usage: archspec help [topic]
archspec built-in manual: ...
topics: ...
commands: ...
run 'archspec help <topic>' for a topic; 'archspec help <command>' for a command
```