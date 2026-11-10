# `verify` — Command Surface (CLI Contract)

## Invocation

```
archspec verify [path] [--strict] [--json] [--coverage] [--only <class>] [--write-exemptions]
```

`path` is optional and positional. `--strict`, `--json`, `--coverage`, `--only` and `--write-exemptions` are optional.

| Part | Meaning | Default |
|---|---|---|
| `path` | project directory to extract and compare | current working directory |
| `--strict` | promote warnings to errors (CI gate) | off |
| `--json` | render the verdict in the `report --format json` machine schema | off |
| `--coverage` | print the coverage census after the verdict | off |
| `--only <class>` | hide one diagnostic class from the report-only notices; the verdict and the environment census are unchanged | off |
| `--write-exemptions` | seed the `[[exempt]]` cycle ledger: append one entry per currently violating ring, never edit or remove an existing entry, then report green for what the run named | off |

## Accepted flag values

- `--strict`: boolean, takes no value.
- `--json`: boolean, takes no value.
- `--coverage`: boolean, takes no value.
- `--only <class>`: takes exactly one value, the diagnostic class to exclude from the report-only notices — the prefix before the first `": "` of a notice line. An unknown or empty class excludes nothing and still echoes. Any other flag is an error (see `errors.md`).
- `--write-exemptions`: boolean, takes no value. Refused in combination with `--json` (usage class 64): a machine consumer must never trigger a write.

## Usage examples

### Default run

```bash
archspec verify
```

Extract the model from the current directory, compare it against `architecture.spec.toml` in the same directory, print the result.

### Explicit path

```bash
archspec verify ./crates/auth
```

Extract `./crates/auth`, compare it against `./crates/auth/architecture.spec.toml`, print the result.

### Strict gate

```bash
archspec verify --strict
```

Promote every warning to an error so any divergence fails the run — the CI gate.

### Machine verdict

```bash
archspec verify --json
```

Render the verdict as the `archspec report --format json` machine body on stdout — the same findings, the same verdict, the same exit codes. The schema is owned by `report` (see `../report/output.md`); `verify --json` restates nothing.

### Coverage census

```bash
archspec verify --coverage
```

Prints the census after the verdict — one line per constraint plus a summary:

```
coverage census — facts adjudicated per constraint versus the model totals
coverage: no_cycles[0] modules=[wide] — 0/3 files
coverage: no_cycles[1] modules=[app] — 2/3 files
coverage summary — 3 files adjudicated by 2 constraints
```

With the `[[exempt]]` ratchet active one further line states the split —
found, exempt and violating rings, never a silent zero:
`coverage: cycles: <found>, exempt: <E>, violating: <V>`.

### Seed the exemption ledger

```bash
archspec verify --write-exemptions
```

Appends one `[[exempt]]` entry per currently violating cycle to
`architecture.spec.toml` — identity and tier only; `count`, `until` and
`reason` are human-owned and stay absent until a human adds them. The run
never edits or removes an existing entry and never auto-grows a covered
one; a run with nothing new to seed writes nothing and echoes nothing.
On a write the run echoes `wrote <path>` and then reports the verdict
over the ledger as it now stands — the exemptions it just named. Refused
with `--json` (usage class 64, see `errors.md`).

### Diagnostic class filter

```bash
archspec verify --only "<class>"
```

Hides one diagnostic class from the report-only notices. Findings, the verdict and the environment census are unchanged; the run echoes `only: excluding class "<class>" — verdict unchanged` even when the class matches nothing.

### Combine anywhere

```bash
archspec verify --strict ./crates/auth
```

Flags and path can be given in any order.

## Behavior notes

- Exactly one positional argument. Extra positional arguments are an error.
- Exit `0` when no error-level failure; non-zero otherwise (see `errors.md`).
- A pass prints a short confirmation to stdout; a fail prints the full diff report to stdout. Warning findings are listed inside that stdout diff report (as `warning:` lines unless `--strict`); **stderr carries operational errors only** (see `errors.md`).
- `verify` is single-shot: it extracts, compares, reports, and exits. No daemon, no persistent state.
- Every verdict declares the environment it reports on: the line `adjudicated against BUILT tree`, or `adjudicated against UNBUILT tree — N specifiers resolved to src instead of dist` when the scan resolved declared build targets to source. It prints after the verdict body, before a `--coverage` census; an `Environment lines by class` census table follows the header wherever the scan emitted report-only lines. `verify --json` states the same fact in the `environment` field (see `../report/output.md`).

## Help

`archspec verify --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
