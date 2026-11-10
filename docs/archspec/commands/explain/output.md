# `explain` — Output

The trace is one stdout block in fixed section order. A run that produced the trace exits 0.

## Sections

```
pair: <source> -> <target>
source: <endpoint description>
target: <endpoint description>
model rows:
  module-graph row <source> -> <target>: present|absent
  value tier: <tier statement>            (only when the row is present)
  file-tier rows ...                      (count + listing, or the named zero statement)
constraints:
  [<index>] <type> ... — pair adjudicated: yes|no ...
findings naming the pair:
  <category>: <entry>                     (one line per finding naming both endpoints)
note: explain renders the same load, extract and compare calls `archspec verify` runs — verdicts stay in `archspec verify`
```

## Endpoint descriptions

- declared module — `module "<name>" — declared` with the row's `matches.units` and, when present, `matches.modules` paths.
- model file — `file <path> — claim chain: <innermost> < <parent> …; module identity: <identity>`, or `no claim owns this file` when nothing claims it.
- unresolved — `unresolved: not a declared module name and no model file claims this spelling`.

## Model rows

- the module-graph row states present/absent for the pair — the same graph `compare` adjudicates, value and type tiers combined.
- the value tier distinguishes a value edge (runtime ring class) from a type-assisted row.
- file-tier rows state how many file-grained rows join the endpoints out of the source endpoint's rows, then list them (capped at 20).

## Constraints

One line per census row. A `no_cycles` row states its effective scope (the row's own or the capability posture table's), its territory — which endpoint is in or out — and whether the pair is adjudicated: yes only when both endpoints sit in the row's territory and the graph holds the pair. A row whose check consumes pair adjacency states adjudication the same way. Any other row states that it adjudicates manifest, export or external facts, not module pairs. An empty filtered census prints the named statement `none — the spec declares no constraint row under this filter`.

## Findings

The findings section lists this run's diff lines that name both endpoints: error-tier entries come from the exit-gate table `Diff::error_fields()` rendered as `<category>: <entry>` — categories mean what `archspec help diagnostics` says — and warning-bucket lines ride verbatim. Lines are sorted and deduplicated. An empty section prints `none — this run's diff holds no line naming both endpoints`.

## stderr

The report-surface facts ride stderr exactly as verify prints them: the excluded-sources report, exclusion liveness, the driver marker report and the wildcard report.
