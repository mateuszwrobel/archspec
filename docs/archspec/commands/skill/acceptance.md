# `archspec skill` — acceptance

Pinned by `tests/skill.rs`:

1. **print emits the shipped document and writes nothing** — stdout equals
   `docs/archspec/skill.md` byte-for-byte (drift guard), no files created.
2. **install writes the document into the skill-loading path** —
   `.agent/skills/archspec.md` appears, identical bytes, path reported.
3. **install honours the positional path regardless of cwd**.
4. **install is idempotent** — identical content is never rewritten (mtime
   stable), state reported as `already installed`.
5. **local edits are kept; force is required to replace them** — refusal
   exits 1 keeping content and naming `--force`; `--force` restores the
   shipped bytes and reports `overwrote`.
6. **bad invocations are usage errors** — exact texts per `errors.md`.
7. **the skill is self-discoverable** — top-level `--help` lists `skill`,
   `skill --help` prints the usage contract, `help workflow` points at
   `archspec skill install` outside the numbered recipe sequence.
