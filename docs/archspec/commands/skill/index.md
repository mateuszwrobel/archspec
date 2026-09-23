# `archspec skill` — contract

Print or install the agent-facing audit skill. The skill is agent procedure
shipped in-binary: the rule-zero extraction protocol, the model's tiers of
trust, the finding classes that work, the report format pointer, and the honest
blind spots. It is documentation as a deliverable, discovered the way help and
capability are discovered — no human setup required.

## Surface

| Invocation | Result |
|---|---|
| `archspec skill` | print the skill document on stdout, exit 0, write nothing |
| `archspec skill install [path]` | write `<path>/.agent/skills/archspec.md` (default path: cwd), creating parents; report state |
| `archspec skill install --force` | additionally replace a locally modified install |
| `archspec skill --help` | usage contract |

## State semantics

- file absent → written, reports `installed <path>`
- file present, bytes identical → untouched, reports `already installed: <path>`
- file present, bytes differ, no `--force` → untouched, exit 1, refusal names
  the file and `--force`
- file present, bytes differ, `--force` → replaced, reports `overwrote <path>`

A locally modified skill is user work; the tool never destroys it silently.
Reinstall with `--force` after upgrading the binary.

## Source of truth

The payload document `docs/archspec/skill.md`, embedded at compile time. The
emitted bytes are pinned against the document by `tests/skill.rs`; a change to
either alone fails. The document carries YAML front matter (`name`,
`description`) so skill-loading harnesses can trigger it automatically, and it
references only generic fixtures — no project-, host- or account-specific
identifiers (publication leak guard).
