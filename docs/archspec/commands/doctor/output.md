# `doctor` — Output Contract

## Destination

- Report written to **stdout**, exit code `0`. There is no `--output` flag in phase 1.
- The report is plain diagnostic text. No diagrams.

## Report structure

The report always contains, in a canonical fixed order (`rust`, then `csharp`, then `go`):

1. **One block per supported driver** — driver name (`rust`, `csharp`, `go`) and two INDEPENDENT facts, each on its own line:
   - `driver:` the in-binary capability state — `present (in-binary, parse-only)` when the language driver can run (matching the capability table: drivers parse sources without any toolchain), `absent` only when the driver is disabled in this environment;
   - `toolchain:` availability on PATH — `present (<version string>)` when the probe answered, `absent` otherwise. An absent toolchain is a diagnostic detail, never a scanning gate; the block then carries a `note:` line stating that scanning does not require the toolchain.
2. **Guidance for absent drivers** — only a driver that cannot run is called out with a next step; a merely absent toolchain gets the note, not install pressure.
3. **Summary** — the line `Scannable languages: <list>`, the languages whose DRIVER is present (`none` when every driver is disabled) — the toolchain probes never change this list.

The probe is a diagnostic, not a scanning gate: every language driver ships inside the archspec binary, so the `toolchain:` line describes the user's toolchain environment, while the `driver:` line and the summary state whether the language can actually be scanned.

## Determinism

The same environment always produces **byte-identical output**. Block order and content are canonical and independent of run order or the invocation directory. Output is safe to commit to CI logs and diff across runs.

## Example

On a machine with Rust and C#/.NET toolchains present and no Go toolchain, the report is (representative):

```
rust
  driver: present (in-binary, parse-only)
  toolchain: present (rustc 1.80.1 (cargo 1.80.1))

csharp
  driver: present (in-binary, parse-only)
  toolchain: present (8.0.201)

go
  driver: present (in-binary, parse-only)
  toolchain: absent
  note: scanning go does not require the toolchain; the driver parses sources in-binary

Scannable languages: rust, csharp, go
```

Driver names, block order, the two facts, and the summary line are fixed. The exact wording of the version detail depends on what each toolchain reports.
