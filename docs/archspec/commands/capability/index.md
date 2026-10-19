# archspec `capability` — Diagnostic Surface

Machine-readable projection of the driver capability table (the single source
of truth in `capability.rs`). **This is a diagnostic surface, not a stable
API**: the command exists so guards and humans can read what each driver
emits without maintaining a second list, and its output shape may change
while the table itself evolves. The contract that matters is the table — one
row per `(language, fact)` stating the emission granularity — not the exact
line format printed here.

The command appears in the tool's command listing but is deliberately absent
from the user-facing command set in the README's table: no flags, no spec
file, no output destinations, and no promise of format stability.

## Subcommands

### `archspec capability matrix`

Prints the table verbatim, one row per line, deterministic and ordered by
fact then language, followed by the rules that consume a fact and the
deep-cycle posture rows (ADR-023):

```text
capability <language> <fact> <emission>
...
rule <rule> <fact>
posture <language> <scope> <severity>
```

`<language>` is the driver name (`rust`, `csharp`, `go`, `ts`), `<fact>` one of the
stated facts (`role-facade`, `role-composition`, `module-tier`, `symbols`,
`root-module-declarations`, `test-tier`, `external-packages`,
`public-api-surface`, `file-grained-dependency-facts`, `orphan-census`), and `<emission>`
the granularity: `granular`, `not-emitted`, or a conditional string (e.g.
`file tier only (*_test.go excluded at scan)`). Parsing these
rows is equivalent to reading the table — this is what the prose-drift guards
and the shared scenario registry consult. Each `posture` row states the
`(scope, severity)` a `no_cycles` row with no `scope` derives from —
`claims` or `both` with `error` (red is the posture); the weld test binds
every `both` row to the granular `file-grained-dependency-facts` row.

### `archspec capability granular <language> <fact>`

The machine query behind the guards: exit `0` when the row states
full-granularity emission, exit `1` otherwise. Conditionals
(`WORKTIER_ONLY`, `FILE_TIER_ONLY`) and `not-emitted` rows answer `1` — on a
plain fixture tree the fact is only there when the table says the driver
always puts it there. Prints nothing on success; on a non-granular row the
stderr message is `not-granular`, which is the query's answer, not a fault.
An unknown language or fact is a genuine invocation error (see
[errors.md](errors.md)).

## Where the same table is read

- `verify` — inert-rule notes: a check whose required fact is `not-emitted`
  (or `WORKTIER_ONLY` with no module tier in this run's model) is reported as
  inert for the language instead of silently passing.
- `doctor` — driver capability is reported apart from toolchain availability.
- `archspec help` manual topics — `[capability ...]` citations; the
  prose-vs-table guard fails the suite when a citation contradicts the table.

## Related contracts

- [acceptance.md](acceptance.md) — behavioural rows for the matrix output.
- [errors.md](errors.md) — failure contract.
- `feature-matrix.md` (crate root) — per-language capability and skip reasons.
