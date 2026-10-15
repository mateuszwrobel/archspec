# `archspec skill` — contract

Print or install the agent-facing skills. The skills are agent procedure
shipped in-binary: the audit skill carries the rule-zero extraction protocol,
the model's tiers of trust, the finding classes that work, the report format
pointer, and the honest blind spots; the analysis skill carries the
relation-analysis playbook (the dumps, the five heuristics, the
recommendation output contract). They are documentation as a deliverable,
discovered the way help and capability are discovered — no human setup
required.

## Surface

| Invocation | Result |
|---|---|
| `archspec skill` | print the audit skill on stdout, exit 0, write nothing |
| `archspec skill audit` | print the audit skill by name — same bytes |
| `archspec skill analysis` | print the relation-analysis skill on stdout, exit 0, write nothing |
| `archspec skill print` | print the audit skill (same as `archspec skill`) |
| `archspec skill install [path]` | write `<path>/.agent/skills/archspec.md` (default path: cwd), creating parents; report state |
| `archspec skill install --force` | additionally replace a locally modified install |
| `archspec skill --help` | usage contract |

An unknown skill name is refused loudly — selection never falls back to the
audit skill. `install` installs the audit skill; the analysis skill is
addressable on stdout (redirect the bytes where your harness loads them).

## State semantics

- file absent → written, reports `installed <path>`
- file present, bytes identical → untouched, reports `already installed: <path>`
- file present, bytes differ, no `--force` → untouched, exit 1, refusal names
  the file and `--force`
- file present, bytes differ, `--force` → replaced, reports `overwrote <path>`

A locally modified skill is user work; the tool never destroys it silently.
Reinstall with `--force` after upgrading the binary.

## Source of truth

The payload documents `docs/archspec/skill.md` (audit) and
`docs/archspec/skill-analysis.md` (analysis), embedded at compile time. The
emitted bytes are pinned against the documents by `tests/skill.rs`; a change
to either alone fails. The documents carry YAML front matter (`name`,
`description`) so skill-loading harnesses can trigger them automatically, and
they reference only generic fixtures — no project-, host- or account-specific
identifiers (publication leak guard).
