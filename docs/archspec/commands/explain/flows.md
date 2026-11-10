# `explain` — Flows

## Trace flow (call-for-call the verify path)

```
archspec explain <source> <target> [path]
  └─ cli::parse                     unknown flag → 64, missing pair → 64
  └─ spec::load_verify(path)        missing spec → 70 (verify's own bytes)
  └─ excluded_sources_report        stderr, verbatim as verify prints it
  └─ exclusion_liveness_report      stderr, verbatim
  └─ language::select               no sources → 70
  └─ marker_report / agreement      stderr / 70 finding
  └─ scan::driver_available         driver missing → 70
  └─ scan::extract_checked          the model
  └─ wildcard_report                stderr, verbatim
  └─ compare internals             map_units → augment → effective_module_tier
                                    flatten_claims → ClaimIndex → graph
  └─ compare(...)                   the very diff verify computes
  └─ render                         pair, endpoints, model rows, census, findings
  exit 0                            always, once the trace was produced
```

## Endpoint resolution flow

```
input
  ├─ declared module name?  → module endpoint (units + paths)
  ├─ claim owner or model file? → file endpoint (claim chain innermost-first, module identity)
  └─ otherwise              → unresolved (stated, never guessed)
```

## Findings flow

```
diff.error_fields()  → per field: entries naming both endpoints → "<category>: <entry>"
diff.warnings        → lines naming both endpoints, verbatim
sort + dedup         → the findings section (or the named none statement)
```

"names both" means the line contains one of the source endpoint's identities and one of the target endpoint's. Error-tier entries come from the exit-gate table itself — the categories can never drift from what gates the exit code or `help diagnostics` catalogs.
