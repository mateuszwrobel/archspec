# `explain` — Command Surface (CLI Contract)

## Invocation

```
archspec explain <source> <target> [path] [--constraint <pattern>]
```

`source` and `target` are required positionals and select the pair to trace. `path` is the third positional and is optional.

| Part | Meaning | Default |
|---|---|---|
| `source` | declared module name or model file path — the pair's from-endpoint | required |
| `target` | declared module name or model file path — the pair's to-endpoint | required |
| `path` | project directory to load and extract | current working directory |
| `--constraint <pattern>` | glob limiting the census to constraint rows whose type or declared module names match | all rows |

## Behaviour contract

- explain renders the same load, extract and compare calls `archspec verify` runs, in that order. Report-surface facts (excluded sources, exclusion liveness, driver markers, wildcards) ride stderr exactly as verify prints them.
- The trace is a renderer over the verify path, not an adjudicator: it states facts and never a verdict, an opinion or a suggested spec edit.
- Exit codes carry the usage and operational classes only. A finding — even the finding the pair produced — never moves the exit code.

## Examples

```
archspec explain ui data
archspec explain app src/data/store.rs .
archspec explain app data --constraint "no_cycles"
```
