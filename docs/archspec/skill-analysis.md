---
name: archspec-analysis
description: Use when analyzing a codebase's relation shape — cycles, coupling hubs, orphan modules, wrong layering direction, god-crate candidates — over archspec model dumps, and reporting findings as recommendations with file:line evidence. Not for verdicts: those are archspec command exit codes.
---

# archspec — relation-analysis skill

Audience: an agent asked to analyze a codebase's relations and name the
non-optimal structure — cycles, coupling hubs, dead weight, wrong layering
direction — with evidence, as recommendations. This skill is the procedure.
It owns no behavior claim: every falsifiable statement below cites the
binary surface that owns it — run that surface, cite its bytes. For audit
finding classes beyond graph shape, emit the audit skill: `archspec skill`.

Verdicts versus recommendations: an archspec verdict is a command exit
code — the legend lives in `archspec help diagnostics`. Everything a
heuristic on this skill produces is a recommendation. Label every finding
you report `recommendation — not a verdict`.

## 1. Generate the dumps

Gate first. Run the audit recipe — `archspec help workflow` owns the
ordering — and settle what `archspec verify` printed before reading a
heuristic result. The gate matters most for cycles: the `archspec init`
scaffold carries the `no_cycles` guard, and `verify` fails a cycle in a
guarded spec (the `cycle` category: `archspec help diagnostics`). A cycle
verify refused is already a verdict — do not restate it below as advice.
Verdict first, heuristics second.

Dump 1 — the model JSON (the adjacency list):

```bash
archspec scan --output model.json
```

No spec is needed; `--pretty` indents for readers and diffs (`archspec scan
--help`). Keys the heuristics read (full contract: `commands/scan/output.md`):

| Key | Fact |
|---|---|
| `units` | build units: name, `kind`, `path` |
| `soft_structure` | the module tier — which module paths exist |
| `module_edges` | module-tier import edges: `unit`, `from`, `to`, `symbols` |
| `module_external` | external dependencies per module path |
| `root_module_declarations` | each root module declaration with its `file` — the file-granular provenance field |
| `roles` | model paths the driver recognizes as `facade` or `composition`; `archspec help roles` owns what the views show |

Module-tier note: `scan` always answers, but on a tree whose package
references record nothing to derive a module tier from, the model stays
unit-tier: `soft_structure` and `module_edges` come back empty and every
`depgraph` view refuses with a precondition message naming what it needs.
Run the refusal once, read it, and let the report carry the refusal bytes:
the graph-level heuristics below then have no module input, and an honest
unit-tier report is the correct output.

Dump 2 — the top-level module graph:

```bash
archspec depgraph modules --output modules.mmd
```

Node labels are projections, not addresses — the addresses are
`module_edges` in dump 1. When the projection folds to one node the view
renders one level deeper (fold rules: `commands/depgraph/output.md`). The
renderers are deterministic, so a dump is safe to commit beside the report.

Dump 3 — one module's internals, once per top-level module:

```bash
archspec depgraph submodules --parent <module> --output sub-<module>.mmd
```

Dependencies leaving that subtree are excluded. A `--parent` value matching
no module is an error naming the modules the model knows — read that list,
do not guess.

Dump 4 — the API-usage table:

```bash
archspec depgraph api-usage --output api-usage.md
```

A pair contributes a row only when its edges record at least one symbol.
When the body comes back empty, the body states the reason — always read
it: emptiness here can mean usage recorded at another granularity, not "no
coupling".

Dump 5 (optional) — the file-level import map:

```bash
archspec inspect --output imports.md
```

Node ids are full relative file paths — the strongest file provenance one
command emits. Use it to attach files to any module-level edge.

## 2. The heuristics

Work the heuristics on dump 1: `module_edges` (plus unit `edges`) is the
adjacency list every degree and cycle below is computed from. Express every
degree threshold as a ratio over the graph mean — total edges ÷ node
count — never as an absolute edge count. The thresholds below say 2× the
mean; state the ratio you used in the report.

H1 — dependency cycles. A chain of import edges returning to its start: a
strongly connected component (SCC) with more than one node. Run an SCC pass
(Tarjan works) over `module_edges` and unit `edges`. An SCC whose endpoints
span different `units` is a cross-unit / cross-crate cycle — the strongest
shape; an SCC inside one unit is a module-hygiene finding. If the spec
declares `no_cycles` (the `archspec init` scaffold does), `verify` refused
these cycles already; this heuristic adds the unguarded tree and the
cross-unit shape the spec diff never had to name. Evidence: every edge line
of the cycle (`unit`, `from`, `to`, `symbols`) plus one located import per
hop — from the `file` fields of `root_module_declarations`, locate the
symbol or import path inside that file.

H2 — fan-in / fan-out outliers. A node whose degree is far above the graph
mean: high fan-in means many modules depend on it (shared kernel or
bottleneck — judge by what it is); high fan-out means it depends on many
(conduit shape). Compute per-node in- and out-degree from `module_edges`;
mean degree = total edges ÷ node count; flag every node whose in- or
out-degree exceeds 2× the mean. Evidence: the node, its degree against the
mean ("fan-in 11, graph mean 2.3"), and a stated sample of located imports
behind the flagged direction.

H3 — orphan modules. A module with fan-in 0 and fan-out 0. Compute: every
node in `soft_structure` that appears in no edge endpoint — minus the entry
points and the dormant ones, before you call it dead: paths the `roles` map
marks `composition` (wiring roots import without being imported, by
design), units whose `kind` names an executable, and declarations with
`gated: true` in `root_module_declarations` (dormant behind a feature, not
dead). Evidence: the declaration entry — module name and its `file` field —
plus the statement "no edge endpoint in dump 1 names this path". Check
dump 4 before reporting: a module some other granularity of usage reaches
is not an orphan.

H4 — layering direction versus declared boundaries. An import flowing
against the direction the boundaries declare. With a spec, compute nothing:
`verify` owns this comparison — its boundary findings (`forbidden edge`,
`disallowed cross-component dependency`, `laundered forbidden edge` — the
catalog: `archspec help diagnostics`) are the verdict; re-deriving them
here would only fork the claim. Without a spec (or with boundaries too
coarse to say anything): condense the graph with the H1 SCC pass, rank
each module by import depth (rank 0 imports nothing internal; rank = 1 +
the maximum rank among the modules it imports), then report as suspicious
only edges that fight a layering convention stated explicitly in the
report — declare the convention first, or report no direction findings at
all. Where no convention exists, the one honest shape signal: a module
whose fan-in exceeds its fan-out reads as a lower layer, so an edge running
from such a module into a module whose fan-out exceeds its fan-in is a
review recommendation, nothing stronger. Evidence: the edge line, both
endpoints' fan figures, and the stated convention the direction violates.

H5 — god-crate candidates. One build unit imported by many AND importing
many at the same time — a unit at the top of both degree lists. Aggregate
each module's degree onto its owning unit (the `unit` field of
`module_edges` and the prefix relation between `units` and `soft_structure`
carry that mapping), then flag units whose fan-in and fan-out both exceed
2× the unit-tier mean. Before reporting, read `roles`: a path marked
`composition` is sanctioned wiring, and a wiring root importing many is a
root doing its job. Cross-check dump 4 for the public names that flow in
both directions. Evidence: the unit name, both degrees against the mean,
and the api-usage rows naming API names crossing both ways.

## 3. Output contract

1. Cite file:line for every finding. The dumps locate facts at module and
   file granularity — the `file` field of `root_module_declarations`,
   `units[].path` and the file node ids of the `inspect` map name the
   file; the line is the located import or symbol inside that file. A
   finding that cannot name a file is a hypothesis — say so, or drop it.
2. Label every finding `recommendation — not a verdict`. Verdicts come
   only from command exit codes — verify's verdict, the refusals, the
   usage and operational classes (the legend: `archspec help
   diagnostics`). No heuristic promotes itself.
3. One report per run, dated in its name: `archspec-analysis-YYYY-MM-DD.md`
   (UTC date). Where the report lives is the caller's choice — this
   procedure is host-agnostic: it presumes no directory layout, no host, no
   account. The analysis report is your prose; it is not `archspec report`
   output.
4. Make the report reproducible: name each dump's exact command and the
   tool version (`archspec --version`) beside the findings they support.
   The renderers are deterministic, so a reviewer who reruns the commands
   can diff the same bytes.

## 4. Where the truth lives

| Question | Owner surface |
|---|---|
| audit ordering | `archspec help workflow` |
| finding categories, exit codes, report format | `archspec help diagnostics` |
| tiers each driver populates, module-tier derivations | `archspec help languages` |
| which views mark roles | `archspec help roles` |
| constraint keys and severity | `archspec help constraints` |
| flags per command | `archspec <command> --help` |
| what each depgraph view projects | `commands/depgraph/output.md` |
| audit finding classes beyond graph shape | `archspec skill` |

If this skill and an owner surface disagree, the surface wins — and the
disagreement is a payload bug: report it the way `docs/archspec/agents.md`
states.
