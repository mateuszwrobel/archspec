# `capability` — Failure Contract (Errors)

Every failure prints `error: <message>` on stderr, nothing on stdout, and
exits `1` (distinct failure codes are not part of the contract). One entry in
the list is not a fault but a query answer: `not-granular` from the
`granular` subcommand means the table row states something other than full
granularity — exit `1` is the "no" answer, and guards treat it as such.

## Error conditions

| Condition | Message | Exit |
|---|---|---|
| no subcommand / unknown subcommand | `usage: archspec capability matrix \| granular <language> <fact>` | 1 |
| extra positional after `matrix` | `capability matrix takes no arguments` | 1 |
| `granular` with fewer than two arguments | `usage: archspec capability granular <language> <fact>` | 1 |
| `granular` with more than two arguments | `capability granular takes two arguments` | 1 |
| language not one of `rust`, `csharp`, `go` | `unknown language: <name>` | 1 |
| fact not stated for that language in the table | `unknown fact for <language>: <fact>` | 1 |
| row exists but is not full granularity | `not-granular` (query answer, not a fault) | 1 |
| unknown flag | `unknown flag: --<name>` | 1 |

## Examples

```bash
# query answer: the row states not-emitted, not a fault
$ archspec capability granular csharp root-facade
error: not-granular
$ echo $?
1

# conditional rows answer the same way (granularity is not full)
$ archspec capability granular go module-tier
error: not-granular

# genuine invocation errors
$ archspec capability
error: usage: archspec capability matrix | granular <language> <fact>

$ archspec capability granular kotlin module-tier
error: unknown language: kotlin

$ archspec capability granular rust no-such-fact
error: unknown fact for rust: no-such-fact
```

## Stability

The message texts above are part of the diagnostic surface, not a stable
API: they may change as the table evolves. Consumers that branch on the
`granular` result must branch on the **exit code**, never on the message.
