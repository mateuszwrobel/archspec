# ADR 015: Agent procedure ships as an installable in-binary skill

## Status
Accepted

**Amended by ADR-016** (2026-09-22): the `--backend` flag is removed and syntax
extraction is the sole path — "defaults are unchanged; teaching is the
mechanism" no longer holds, and what the skill taught agents to reach by flag
is now what the default emits. The delivery decision below stands.

## Context
The syntax back-ends (ADR-014) put richer facts behind `--backend syntax` with
the default staying `classic` (byte-freeze contract). An unguided-agent
experiment measured the consequence: given a C# application and no
instructions, a capable agent never discovered the flag, stayed on the
symbol-less model, and reported a structurally degraded audit — spec hygiene
and surface imports only. The same model with a one-line hint produced layering
violations with line evidence, stale-reference findings and a dead-seam
finding. The knowledge, not the software, was the bottleneck; documentation in
a repository the agent never reads does not close it.

## Decision
Ship the audit protocol as a **skill deliverable produced by the binary
itself**: `archspec skill` prints it, `archspec skill install [path]` writes
`<path>/.agent/skills/archspec.md`, the skill-loading convention of this
repository family. The content lives once in the payload docs
(`docs/archspec/skill.md`), is embedded at compile time, and emission is
byte-pinned against it. The skill teaches the `--backend syntax` extraction
rule, the tiers of trust, the finding classes proven to work, and the honest
blind spots. Discovery chains from surfaces agents already consult: the
top-level command list, `skill --help`, and the workflow manual topic.
Defaults are unchanged; teaching is the mechanism.

## Consequences
- Agent knowledge becomes a tested, versioned deliverable alongside help and
  capability — same culture (in-binary, deterministic, drift-guarded), new
  class.
- Skill prose carries a maintenance obligation: it must stay a pointer layer
  over surfaces that are already pinned, never a second source of truth.
- Stale installs after upgrades are possible; the skill is short, installation
  is idempotent, and `--force` refreshes it.
- Multi-harness install targets were rejected: the tool takes one positional
  path and otherwise stays blind to other tools' directory layouts.

## Alternatives considered
- **Flip or remove `classic` defaults** — rejected for now: the freeze
  contract protects existing consumers and the evidence supports teaching at
  least as well; a later flip can adopt what this skill already teaches.
- **Repo README / agents guide only** — rejected: mid-task agents do not clone
  documentation trees, and the experiment showed prose out of reach changes
  nothing.
- **A capability-output pointer** — rejected: the capability grammar carries
  driver facts consumed by value-lock guards; a deliverable row is not a
  driver fact and would pollute a machine-parsed surface.
- **AGENTS.md snippets emitted for merging** — rejected: mutating or seeding a
  file with the project's own agent configuration oversteps the tool's place.
