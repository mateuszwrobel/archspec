# `doctor` — Command Surface (CLI Contract)

## Invocation

```
archspec doctor
```

`doctor` takes **no arguments and no flags** in phase 1. There is no positional path argument: `doctor` diagnoses the environment it runs in, not a project.

## Accepted flags and arguments

None in phase 1. No positional path argument — `doctor` diagnoses the environment, not a project. Any flag or positional argument is an error (see `errors.md`).

## Usage examples

### Default run

```bash
archspec doctor
```

Inspect the environment, print the per-driver report to stdout, exit `0`.

## Behavior notes

- Zero positional arguments. Any positional argument is an error.
- Zero flags. Any flag is an error.
- A valid diagnosis exits `0` — always. A diagnosis is a report, not a pass/fail result; missing toolchains are findings, not failures.
- The only non-zero exits are invalid invocations and the case where `doctor` cannot produce a report at all (e.g. it cannot inspect its own environment).
- `doctor` is single-shot: it inspects, reports, and exits. No daemon, no persistent state.

## Help

`archspec doctor --help` prints this command's usage and flags to stdout and exits `0`; it runs nothing else. See `../../help.md` for the tool-wide contract.
