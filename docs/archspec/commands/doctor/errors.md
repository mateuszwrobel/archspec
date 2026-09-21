# `doctor` — Failure Contract (Errors)

Every failure produces a clear human message naming the cause and a **non-zero exit**. Messages go to stderr; no partial report is written.

`doctor` is diagnostic: producing a report is success. **A missing toolchain is a finding, not an error** — the report still exits `0`. Scanning availability is not decided here: every language driver ships inside the binary, so the `PATH` probes only describe the environment (the commands' driver-unavailable error is reachable only through the `ARCHSPEC_DISABLE_DRIVERS` test seam). Non-zero exits are reserved for runs that produce no report at all: an invalid invocation, or a failure to inspect the environment itself.

## Error conditions

| Condition | Message must make clear | Exit |
|---|---|---|
| unexpected positional argument | that `doctor` takes no path argument (it diagnoses the environment, not a project) | non-zero |
| unknown flag | the unknown flag | non-zero |
| cannot inspect its own environment — simulated deterministically by the `ARCHSPEC_DOCTOR_FAIL` test seam | the cause of the failure | non-zero |

## Examples

```bash
# message says a path is not accepted
$ archspec doctor ./my/project
error: doctor takes no path argument (diagnoses the environment, not a project)

# message names the unknown flag
$ archspec doctor --verbose
error: unknown flag: --verbose

# message names the cause of the failed inspection
# ARCHSPEC_DOCTOR_FAIL is the documented test seam that simulates it
$ ARCHSPEC_DOCTOR_FAIL=1 archspec doctor
error: failed to inspect environment: cannot probe toolchain availability
```

## Output on error

- stderr carries the message; stdout carries nothing.
- Exit code is non-zero (distinct failure codes are not required in phase 1).
