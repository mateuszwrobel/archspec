# ADR 024: TS resolution delegates to a pinned resolver crate behind one scan-layer seam

## Status
Accepted (2026-10-17). The flip condition is met: the pinned dependency is
in the manifest and the consuming code landed — the dependency entered the
manifest only with the code that uses it, as the flip law required.

## Context
The ts driver (ADR-022) resolves specifiers through extension-and-index
inference. A workspace-shaped tree declares its structure in three places
the driver cannot read: tsconfig `compilerOptions.paths` behind an
`extends` chain, the package manifest's exports and imports maps behind a
condition set, and the node_modules symlinks a workspace linker leaves. The
measured consequences: aliased imports land in the unresolvable class as
false reds; a package cycle with file-level return edges landing on leaf
files forms NO file-level SCC, so a files-only engine misses a real cycle;
with symlink resolution off, a workspace import classifies as external.
A proof of run (recorded outside the payload, `/var/tmp/poc-tsres/`)
exercised the Rust crate `oxc_resolver` 11.24.3 against a three-package
workspace fixture: all ten planted cases resolved as specified with no
Node.js invoked. The probe settled the API law — automatic tsconfig
discovery (nearest config upward, `extends` followed) is honored by the
file-based entry point ONLY; the directory-based entry ignores it silently.
It settled the condition law — exports keys match in DECLARATION order
among active conditions, so a `types` condition declared first lands
package imports on declaration files; a graph builder must choose its
condition set deliberately. It settled the cycle law — package-level
projection catches what the file-level Tarjan misses. The resolution
algorithm itself is a maintained body of behavior (Node resolution, tsc
paths, exports conditions, ecosystem conditions); an in-house copy turns
every upstream addition into a downstream blind spot.

## Decision
Add `oxc_resolver` as a dependency under exact-pin discipline — the
probed version is pinned verbatim and the combination moves only as a
probed unit, the law the tree-sitter grammar pair already carries. The
dependency is confined to the ts arm of the scan layer; the foundation
modules' forbidden-external list gains the crate, so a resolver import
outside the scan layer reddens archspec's own verify. Every ts leg
resolves through the file-based entry with automatic tsconfig discovery
(extends chains included, discovery bounded by the scan root — configs
above the scan root are never read and never reddened), symlink
resolution ON, and a condition set of
`node` + `import` + `require` + `svelte` — WITHOUT `types`, so
implementation files win and no resolution ever lands on a declaration
file; `require` is in the set so CJS-only exports maps (`types` +
`require`) resolve to their implementation file instead of faulting. The
crate's error taxonomy maps onto named archspec diagnostics: dead alias
target (naming the target from the alias map archspec loaded for the
importing file — the crate's `NotFound` payload is specifier-only, while
the alias-match and imports-map variants carry their own payloads),
declared-but-absent external (a report-only not-installed line, not an
import error), tsconfig not-found (naming the extends target the resolver
attempted — the joined path for a relative extends, the specifier
verbatim for an npm specifier) and tsconfig load-failed (named errors,
never silent — and
backed by an archspec-side pre-read bounded by the scan root: each scanned
file's own tsconfig chain up to the root plus its extends closure at any
file name, since the crate silently skips a config it cannot read),
tsconfig cycle — extends or project-reference (naming the paths verbatim),
package-path-not-exported riding
the unresolvable-import class (naming the package path, the subpath and
the active condition set), package-manifest read or parse failure as a
named invalid-manifest error, and honest not-found for links the package
manager denied. The mapping is exhaustive by law: the seam matches every
resolver error variant, and its catch-all fails loud naming the variant —
an unclassified resolver error, never silence. Resolution-vs-name
precedence is the realpath: a
resolution landing inside a member dir is first-party; a resolution
landing elsewhere rides its resolved classification — with a report-only
workspace-link-missing line when the importing manifest declares the
member — and name-membership adjudicates when
the manifest declares the member and no resolution exists, or where the
round-4 linker-family gate admits it: a bare member-name miss with no
resolution under a hoisting-family lockfile at the registry root rides a
first-party membership edge with its own report-only line (owned by the
`help diagnostics` catalog), while the same miss under pnpm-strict or no
lockfile keeps the honest red. The
workspace registry, file enumeration, package
ownership classification, package-tier cycle projection, and every verdict
stay archspec-side: the resolver answers exactly one question — which file
does this specifier name, from this file.

## Consequences
- Resolution behavior becomes the upstream algorithm's, tested by a
  committed conformance fixture; tsc-fidelity drift surfaces as a fixture
  red plus an upstream issue, not an archspec backlog.
- The dependency surface grows by one crate plus its transitive deps;
  build and packaging legs carry it under the exact pin; supply-chain
  review rides the pin and the probed-unit upgrade law.
- The scan layer becomes the single language-aware seam: model, spec, cli,
  config, language, capability and helptext stay resolver-free by a spec
  constraint, not by convention.
- The linked-npm-workspace blind spot of ADR-022 retires: workspace
  resolution, workspace-package classification, and package-level cycle
  adjudication become enforced behavior with committed fixture evidence.
- Bundler-config aliases remain a named blind spot — no config execution
  ships here; such specifiers resolve to the honest unresolvable class.
- Node builtin specifiers (`node:`-prefixed or bare builtins) classify as
  external facts through the seam's builtins law — an archspec builtin-name
  list pre-empting delegation, or the resolver's own Builtin variant
  classified external; the seam covers either shape. They never resolve
  through delegation and never land in the unresolvable class: with the
  probed option shape the resolver alone answers not-found for
  `node:path`, so the law is what keeps every stdlib import from
  false-reddening.

## Alternatives considered
| Solution | Verdict |
|---|---|
| Pinned `oxc_resolver` behind a scan-layer seam (chosen) | Measured green on every planted class, no Node.js; one question per call; the maintenance lives with the upstream that owns the spec. |
| Hand-rolled resolver inside archspec | Rejected: the POC measured the hard classes — extends chains, exports-key ordering, imports maps, symlink realpaths, extension probing — as real behavior with edge cases; each unimplemented mode becomes a false red in a customer tree, the silent-blindness class this crate has outlawed, re-badged as an owned algorithm. |
| `tsrs` as a dependency | Rejected: tsrs is a Rust port of the TypeScript 7 compiler and type checker (declaration-file generation from Rust types is the DIFFERENT tool ts-rs); a whole type checker is far beyond a resolution seam — archspec counts import edges syntactically and needs no checker. |
| Node delegation, dependency-cruiser style | Rejected: requires a Node.js binary on every host running archspec, and a wrapped tool's config is a second rule vocabulary under ADR-006's standing no-wrap constraint; the POC proved resolution runs with no Node.js invoked. |
| Keep extending the in-house extension-and-index law | Rejected: the class it cannot express is the class the tree declares — tsconfig paths behind extends chains and conditional exports; every aliased import stays a false red forever. |
