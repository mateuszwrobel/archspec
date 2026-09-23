# ADR 016: Syntax extraction as the sole path

## Status
Accepted (2026-09-22). Supersedes ADR-014 (flag design); amends ADR-015
("defaults stay classic").

## Context
ADR-014 put syntax-tree extraction behind `--backend syntax` with the default
staying `classic` under a byte-freeze contract, and ADR-015 shipped the audit
protocol as an in-binary skill after blind-agent experiments showed agents
staying on the symbol-less default model. The teaching experiment settles the
question the flag was built to defer: whatever the default is, that is what
agents get — a degraded default produces structurally incomplete audits no
matter what documentation exists. Meanwhile the second path keeps paying rent:
two extractors per language, two sets of refusal/help/capability wording, and
freeze plus parity suites guarding a default whose retirement is the whole
point of the flag's planned flip.

The removal is cheap because the surface being removed never reached a
consumer. The `--backend` flag, the classic/syntax split, and the
back-end-qualified capability rows exist only in unreleased payload ahead of
the last release; no release shipped a user-visible choice of back-ends, and
the model shapes classic emitted have no consumer (the rust model is
byte-stable across the change either way, `syn` having always been its
substrate).

## Decision
Remove the classic byte-tokenizer back-end entirely. Syntax-tree extraction is
the sole path for all non-rust drivers; the rust driver is untouched.

- **The flag disappears rather than deprecating**: `--backend` is removed from
  every command; passing any value produces the standard
  `unknown flag: --backend` error, exit 1. A flag accepting one value is
  documentation noise, and erroring helpfully on old values is migration
  theater for consumers that do not exist.
- **The breaking change collapses into unreleased 0.5.0**: model shapes change
  (symbols, type-position crossings appear where classic emitted none), so the
  version bump is honest, and no migration story is documented — deliberately,
  because no release ever carried the surface being removed.
- Backend dispatch is deleted and scan dispatch keys on language only; the
  syntax submodules become the language extractors.
- `capability matrix` loses its `backend ...` rows; the per-language fact rows
  state the values the sole path emits.
- Back-end-adjacent wording (refusal sentences, empty-reason notes, help
  default sentences) collapses to single constants describing the sole path.
- Freeze and parity suites are deleted; goldens are re-baselined to sole-path
  bytes, checked to byte-match the previously recorded `--backend syntax`
  outputs on the same fixtures before the old goldens die.

## Consequences
- One extractor per language; the inspect↔scan agreement invariants hold once
  rather than per back-end, and ADR-014's dual-path obligations end.
- The zero-flag default is the named-crossing model: agents get the audited
  facts with no flag, no skill, and no prior knowledge. What ADR-015 taught
  about the flag becomes true of the default itself; its status carries an
  amendment note, its delivery decision stands.
- Binary size and packaging consequences from ADR-014 stand unchanged — the
  tree-sitter runtime and grammars remain, only the tokenizers leave.
- Consumers value-locking classic-era capability fact rows would need
  re-keying; the check found none outside trees whose model is byte-stable.
- Git history is the rollback lever ADR-014 bought with the flag; the flag's
  own alternatives section already flagged silent replacement as
  unwelcome-while-unaudited, and the audit has since landed.

## Alternatives considered
- **Flip the default, keep the flag** — leaves the dead tokenizers, the parity
  contract web, and both wording sets alive; the removal retires exactly
  those.
- **Keep `--backend classic` as a legacy value** — preserves the degraded
  default for anyone who types it and every guarantee the removal exists to
  retire.
- **Deprecate the flag for one release** — migration theater for a surface no
  release shipped; a clear cutoff in an unreleased version is the honest
  shape.
