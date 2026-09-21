# Example Specs — Illustrative Only

These are **examples** of what a spec can look like for common architectural shapes. They carry **no runtime meaning**: the runtime has no profile catalog and no recognition engine (ADR-013). They are starting points you copy, adapt to your actual boundaries, and edit — never presets that `init` applies.

The examples are grouped by the *shape* of the project (how many units, whether they nest modules), but note that the spec format itself is shape-agnostic — these are only conventions people often want.

## Single crate, internally modular (Rust)

One unit; boundaries are its internal modules.

```toml
[project]
language = "rust"

[[module]]
name = "commands"
matches = { modules = ["commands"] }

[[module]]
name = "orchestration"
matches = { modules = ["orchestration"] }

[[module]]
name = "config"
matches = { modules = ["config"] }

[module.allowed]
depend_on = ["config"]
forbidden = ["orchestration"]
```

## Multi-crate workspace (Rust)

Units are crates; boundaries are crates.

```toml
[project]
language = "rust"

[[module]]
name = "auth"
matches = { units = ["auth"] }

[[module]]
name = "payments"
matches = { units = ["payments"] }

[[module]]
name = "shared"
matches = { units = ["shared"] }

[module.allowed]
depend_on = ["shared"]
forbidden = ["auth"]
```

## Hybrid — crates with internal modules (C#/.NET)

Units are projects; some boundaries are internal namespaces/services within a project.

```toml
[project]
language = "csharp"

[[module]]
name = "Billing.Domain"
matches = { modules = ["Billing::Domain"] }

[[module]]
name = "Billing.Api"
matches = { units = ["Billing.Api"], modules = ["Billing::Api"] }

[[module]]
name = "Payments"
matches = { units = ["Payments", "Payments.Abstractions"] }

[module.allowed]
depend_on = ["Billing.Domain"]
forbidden = ["Billing.Api"]
```
