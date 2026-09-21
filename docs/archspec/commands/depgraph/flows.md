# `depgraph` — Flows

## User flow

1. Point `depgraph` at a project directory (or run from its root).
2. Choose a view: `modules` for the top-level picture, `api-usage` for which APIs each module uses, `submodules --parent <m>` to expand one module.
3. Read the body from stdout, or pass `--output` to write it straight into a doc.

## Application flow

1. Parse flags; validate the view, format, and `--parent` combination.
2. Detect the language and confirm its driver is available, then extract the model — the same model `scan` produces.
3. Project the model for the chosen view (`modules`, `submodules`, or `api-usage`), rendering through the shared `render` primitives so escaping matches the other diagram commands.
4. Emit the body to stdout, or to `--output`.

No spec is read and no rules are checked: `depgraph` reports current state only, like `scan` and `inspect`.
